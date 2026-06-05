use crate::{AppState, TrackUpdate, RpcCommand, utils};
use crate::rpc::ipc::IpcClient;
use serde_json::{json, Value};
use std::time::Duration;

pub const DISCORD_RATELIMIT: Duration = Duration::from_secs(15);

pub async fn apply_new_state(state: &AppState, mut track: TrackUpdate, _is_transition: bool, rpc_raw: Value) {
    let details = track.details.as_deref().unwrap_or("").to_lowercase();
    let state_str = track.state.as_deref().unwrap_or("").to_lowercase();

    let is_resting = details == "idle" || 
                     details == "resting" || 
                     details == "resting..." ||
                     details == "loading next track..." ||
                     details == "loading next track" ||
                     state_str == "preparing to play..." ||
                     state_str == "preparing to play" ||
                     details.is_empty();
                     
    let is_empty = (track.details.is_none() && track.state.is_none()) || is_resting;
    
    let is_paused_by_key = track.small_image_key.as_deref().map(|s| s.contains("pause")).unwrap_or(false)
        || track.large_image_key.as_deref().map(|s| s.contains("pause")).unwrap_or(false)
        || track.paused.unwrap_or(false)
        || track.status == "paused";

    if is_paused_by_key {
        track.paused = Some(true);
        track.status = "paused".to_string();
    } else {
        track.status = if is_empty { "idle".to_string() } else { "playing".to_string() };
    }

    info!("[WS] Track Update: {} - {} (Status: {})", 
        track.details.as_deref().unwrap_or("None"), 
        track.state.as_deref().unwrap_or("None"),
        track.status
    );

    let overlay_fmt = utils::format_overlay_track(track.clone());
    
    let mut final_rpc = rpc_raw.clone();
    if is_empty {
        if let Some(obj) = final_rpc.as_object_mut() {
            obj.insert("status".to_string(), json!("idle"));
        }
    }

    let (is_transition, should_broadcast) = {
        let s = state.read().await;
        if let Some(last) = s.last_track.as_ref() {
            let was_idle = last.status == "idle";
            let now_idle = track.status == "idle";
            let song_changed = !was_idle && !now_idle && last.details != track.details;
            
            // 🚀 STRICT COMPARISON: If ANYTHING changed (paused, position, artist, etc.), broadcast it!
            let any_change = last != &overlay_fmt;
            
            (was_idle != now_idle || song_changed, any_change)
        } else {
            (true, true)
        }
    };

    let rpc_tx = {
        let mut s = state.write().await;
        s.last_track = Some(overlay_fmt.clone());
        s.last_rpc_raw = Some(final_rpc.clone());
        if should_broadcast {
            let _ = s.overlay_tx.send(json!({ "type": "track_update", "data": overlay_fmt }).to_string());
        }
        s.rpc_tx.clone()
    };

    if let Some(tx) = rpc_tx {
        let _ = tx.send(RpcCommand::Update { track, rpc_raw: final_rpc, is_transition });
    }
}

pub fn update_discord_activity_raw(client: &mut IpcClient, rpc_raw: &Value, state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    let status = rpc_raw.get("status").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
    let details_raw = rpc_raw["details"].as_str().unwrap_or("").trim();
    let state_raw = rpc_raw["state"].as_str().unwrap_or("").trim();
    
    // 🚀 CLEAR ACTIVITY: Only if EVERYTHING is empty. Otherwise, let Idle/Resting show up!
    if status.is_empty() && details_raw.is_empty() && state_raw.is_empty() { 
        info!("[RPC] No data provided. Clearing Activity.");
        let _ = client.send(json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": json!(null) },
            "nonce": uuid::Uuid::new_v4().to_string()
        }), 1);
        return Ok(()); 
    }

    let get_ts = |keys: &[&str]| keys.iter()
        .find_map(|&k| rpc_raw.get(k)?.as_f64())
        .map(|s| if s < 10_000_000_000.0 { (s * 1000.0) as i64 } else { s as i64 });

    let get_val = |keys: &[&str]| keys.iter()
        .find_map(|&k| rpc_raw.get(k))
        .cloned();

    let speed = rpc_raw.get("playbackSpeed").or_else(|| rpc_raw.get("playback_speed")).and_then(|v| v.as_f64()).unwrap_or(1.0);
    let mut details = rpc_raw["details"].as_str().unwrap_or("").trim().to_string();
    if (speed - 1.0).abs() > 0.01 {
        details = format!("{} [{:.2}x]", details, speed);
    }
    let state_text = rpc_raw["state"].as_str().unwrap_or("").trim().to_string();

    let swap = {
        let s = state.blocking_read();
        let global_swap = s.settings.as_ref().map(|st| st.rpc.swap_rpc_lines).unwrap_or(false);
        let player_use_details = rpc_raw.get("useDetails").and_then(|v| v.as_bool())
            .or_else(|| {
                rpc_raw.get("statusDisplayType").and_then(|v| {
                    if let Some(s) = v.as_str() {
                        Some(s == "DETAILS" || s == "0")
                    } else if let Some(n) = v.as_i64() {
                        Some(n == 0) // Assuming 0 is DETAILS
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(true);
        global_swap || !player_use_details
    };

    let (mut final_details, final_state) = if swap {
        (state_text.clone(), details.clone())
    } else {
        (details.clone(), state_text.clone())
    };

    // 🚀 Conditional "by " removal: Only if artist is on the top line (details)
    if swap && final_details.to_lowercase().starts_with("by ") {
        final_details = final_details[3..].trim().to_string();
    }

    // 🚀 Ensure lines are never empty to prevent Discord from showing App Name instead
    let final_details = if final_details.is_empty() { " ".to_string() } else { final_details };
    let final_state = if final_state.is_empty() { " ".to_string() } else { final_state };

    let mut assets = serde_json::Map::new();
    if let Some(v) = get_val(&["large_image_key", "largeImageKey", "large_image"]) { assets.insert("large_image".to_string(), v); }
    if let Some(v) = get_val(&["large_image_text", "largeImageText", "large_text"]) { assets.insert("large_text".to_string(), v); }
    if let Some(v) = get_val(&["small_image_key", "smallImageKey", "small_image"]) { assets.insert("small_image".to_string(), v); }
    if let Some(v) = get_val(&["small_image_text", "smallImageText", "small_text"]) { assets.insert("small_text".to_string(), v); }
    if let Some(v) = get_val(&["large_url", "largeUrl"]) { assets.insert("large_url".to_string(), v); }
    if let Some(v) = get_val(&["small_url", "smallUrl"]) { assets.insert("small_url".to_string(), v); }

    let mut buttons = Vec::new();
    buttons.push(json!({ "label": "Try T_Music_Bot RPC! 🎶", "url": "https://github.com/T-Bot-Team/t-music-bot-rpc" }));
    buttons.truncate(1);

    #[cfg(debug_assertions)]
    info!("[RPC] Raw Payload from Server: {}", rpc_raw);

    let header_name = if status == "idle" || status == "resting" || status == "resting..." {
        "T_Music_Bot".to_string()
    } else {
        final_details.clone()
    };

    let mut activity = json!({
        "name": header_name,
        "details": final_details,
        "state": final_state,
        "type": rpc_raw.get("type").unwrap_or(&json!(2)),
        "instance": false
    });

    if let Some(obj) = activity.as_object_mut() {
        let discord_invite = json!("https://discord.gg/FYzyYTX");
        obj.insert("details_url".to_string(), discord_invite.clone());
        obj.insert("detailsUrl".to_string(), discord_invite);
        
        if !assets.is_empty() { obj.insert("assets".to_string(), Value::Object(assets)); }
        if !buttons.is_empty() { obj.insert("buttons".to_string(), json!(buttons)); }
    }
    
    let start = get_ts(&["start_timestamp", "startTimestamp", "start", "startTime"]);
    let end = get_ts(&["end_timestamp", "endTimestamp", "end", "endTime"]);
    if start.is_some() || end.is_some() {
        let mut ts_obj = json!({});
        if let Some(s) = start { ts_obj["start"] = json!(s); }
        if let Some(e) = end { ts_obj["end"] = json!(e); }
        activity["timestamps"] = ts_obj;
    }

    #[cfg(debug_assertions)]
    info!("[RPC] Sending: {}", serde_json::to_string(&activity).unwrap());

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": activity },
        "nonce": uuid::Uuid::new_v4().to_string()
    });

    client.send(payload, 1)?;
    Ok(())
}
