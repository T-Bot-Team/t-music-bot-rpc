use crate::{AppState, Settings};
use crate::server::ServerHandle;
use crate::server::templates::get_html;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::{Html, IntoResponse},
};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::json;

pub async fn overlay_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let s = state.read().await;
    let settings = s.settings.as_ref().unwrap();
    let layout_override = params.get("layout").cloned();
    let html = get_html(
        &settings.overlay,
        s.last_track.as_ref(),
        layout_override,
    );
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CACHE_CONTROL, "no-store, must-revalidate"),
            (axum::http::header::PRAGMA, "no-cache"),
            (axum::http::header::EXPIRES, "0"),
        ],
        Html(html)
    ).into_response()
}

pub async fn settings_gui_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let s = state.read().await;
    let settings = s.settings.as_ref().unwrap();
    let code = settings.code.clone().unwrap_or_default();
    let has_token = settings.session_token.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    if !has_token && (code.is_empty() || code.len() != 6) {
        return axum::response::Redirect::to("/setup").into_response();
    }
    
    let html = include_str!("../ui/gui.html");
    let settings_json = serde_json::to_string(settings).unwrap_or_default();
    let injected = html
        .replace("window.__SETTINGS__ = null;", &format!("window.__SETTINGS__ = {};", settings_json))
        .replace("{app_version}", env!("CARGO_PKG_VERSION"));
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CACHE_CONTROL, "no-store, must-revalidate"),
            (axum::http::header::PRAGMA, "no-cache"),
            (axum::http::header::EXPIRES, "0"),
        ],
        Html(injected)
    ).into_response()
}

pub async fn setup_gui_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let s = state.read().await;
    let settings = s.settings.as_ref().unwrap();
    let code = settings.code.clone().unwrap_or_default();
    let has_token = settings.session_token.as_ref().map(|t| !t.is_empty()).unwrap_or(false);

    if has_token || (!code.is_empty() && code.len() == 6) {
        return axum::response::Redirect::to("/settings").into_response();
    }
    Html(include_str!("../ui/setup.html")).into_response()
}

pub async fn get_devices_handler() -> impl IntoResponse {
    axum::Json(crate::utils::get_audio_devices())
}

pub async fn get_settings_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    axum::Json(state.read().await.settings.clone())
}

pub async fn get_defaults_handler() -> impl IntoResponse {
    axum::Json(crate::models::Settings::default())
}

pub async fn rpc_sync_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let s = state.read().await;
    if let (Some(tx), Some(track), Some(rpc_raw)) = (&s.rpc_tx, &s.last_track, &s.last_rpc_raw) {
        info!("[Server] Manual RPC Sync triggered");
        let _ = tx.send(crate::RpcCommand::Update { 
            track: track.clone(), 
            rpc_raw: rpc_raw.clone(),
            is_transition: true 
        });
        json!({ "status": "success" }).to_string().into_response()
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "RPC or Track data unavailable").into_response()
    }
}

pub async fn get_discord_clients_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let now = std::time::Instant::now();
    let cache_duration = std::time::Duration::from_secs(30);

    // 1. Try to serve from cache first
    {
        let s = state.read().await;
        if let Some((ts, clients)) = &s.discovery_cache {
            if now.duration_since(*ts) < cache_duration {
                return axum::Json(clients.clone());
            }
        }
    }

    // 2. Cache expired or missing, perform a fresh scan
    let active_pipe = state.read().await.active_pipe;
    let mut clients = tokio::task::spawn_blocking(move || {
        crate::rpc::scan_discord_clients()
    }).await.unwrap_or_default();

    // 🚀 INJECT ACTIVE PIPE: If we are currently connected to a pipe, it will be busy and skip the scan.
    // We must manually add it back to the list using the user_id from settings.
    if let Some(pipe) = active_pipe {
        let is_in_list = clients.iter().any(|c| c["pipe"].as_i64() == Some(pipe as i64));
        if !is_in_list {
            let s = state.read().await;
            let uid = s.settings.as_ref().and_then(|st| st.user_id.clone()).unwrap_or_default();
            if !uid.is_empty() {
                clients.push(json!({
                    "pipe": pipe,
                    "id": uid,
                    "display_name": format!("Connected Account ({})", uid)
                }));
            }
        }
    }

    // 3. Update cache
    {
        let mut s = state.write().await;
        s.discovery_cache = Some((now, clients.clone()));
    }

    axum::Json(clients)
}

pub async fn update_settings_handler(
    State((state, handle)): State<(AppState, Arc<ServerHandle>)>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut s = state.write().await;
    let prev_audio = s.settings.as_ref().map(|st| st.audio.clone());

    // 1. Get current settings as JSON Value
    let mut current_settings_json = match &s.settings {
        Some(set) => serde_json::to_value(set).unwrap(),
        None => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Settings not initialized").into_response(),
    };
    let original_settings_json = current_settings_json.clone();

    // 2. Extract potential actions and inner settings payload
    let actual_payload = if payload.get("settings").is_some() {
        if let Some(actions) = payload.get("actions") {
            if actions.get("allow_firewall").and_then(|v| v.as_bool()).unwrap_or(false) {
                let port = payload.get("settings")
                    .and_then(|s| s.get("overlay"))
                    .and_then(|o| o.get("port"))
                    .and_then(|p| p.as_u64())
                    .unwrap_or(3000) as u16;
                crate::utils::firewall::add_firewall_rule(port);
            }
            if actions.get("desktop_shortcut").and_then(|v| v.as_bool()).unwrap_or(false) {
                crate::utils::create_desktop_shortcut();
            }
            if actions.get("start_menu_shortcut").and_then(|v| v.as_bool()).unwrap_or(false) {
                crate::utils::create_start_menu_shortcut();
            }
        }
        payload.get("settings").unwrap()
    } else {
        &payload
    };

    // 3. Deep Merge the partial update
    merge_json(&mut current_settings_json, actual_payload);

    let changed = current_settings_json != original_settings_json;

    // 4. Convert back to Settings struct and validate
    match serde_json::from_value::<Settings>(current_settings_json) {
        Ok(new_settings) => {
            if changed {
                s.settings = Some(new_settings.clone());
                crate::config::save_settings(&new_settings);

                if let Some(tx) = &s.rpc_tx {
                    let _ = tx.send(crate::RpcCommand::Refresh);
                }

                info!("[Server] Settings updated (Partial). Triggering reload...");
                let _ = handle.tx.send(json!({ "type": "settings_update" }).to_string());

                // Audio routing is owned by us now: create/refresh or tear down
                // the virtual bus whenever those specific settings change.
                if prev_audio.as_ref() != Some(&new_settings.audio) {
                    let audio = new_settings.audio.clone();
                    tokio::task::spawn_blocking(move || {
                        match crate::audio_bus::reconcile(audio.manage_bus, audio.listen_volume.min(150) as u8) {
                            Ok(st) => info!(
                                "[AudioBus] Reconcile -> active={} loopback={} listen={}%",
                                st.active, st.loopback, st.listen_volume
                            ),
                            Err(e) => info!("[AudioBus] Reconcile failed: {}", e),
                        }
                    });
                }
            }
            json!({ "status": "success" }).to_string().into_response()
        }
        Err(e) => {
            info!("[Server] Settings Merge Failed: {}", e);
            (axum::http::StatusCode::BAD_REQUEST, format!("Invalid Settings: {}", e)).into_response()
        }
    }
}

fn merge_json(a: &mut serde_json::Value, b: &serde_json::Value) {
    match (a, b) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            for (k, v) in b {
                merge_json(a.entry(k).or_insert(serde_json::Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State((state, handle)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        info!("[Server] WebSocket connection established");
        handle_socket(socket, state, handle)
    })
}

pub async fn handle_socket(mut socket: WebSocket, state: AppState, _handle: Arc<ServerHandle>) {
    let (mut track_rx, mut viz_rx) = {
        let s = state.read().await;
        (s.overlay_tx.subscribe(), s.viz_tx.subscribe())
    };

    {
        let s = state.read().await;
        let track = s.last_track.clone().unwrap_or_default();
        let _ = socket.send(Message::Text(json!({ "type": "track_update", "data": track }).to_string().into())).await;
    }
    
    loop {
        tokio::select! {
            res = track_rx.recv() => {
                match res {
                    Ok(msg) => { if socket.send(Message::Text(msg.into())).await.is_err() { break; } }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            res = viz_rx.recv() => {
                match res {
                    Ok(msg) => { if socket.send(Message::Text(msg.into())).await.is_err() { break; } }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => { 
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => continue,
                }
            }
        }
    }
    info!("[Server] WebSocket connection closed");
}

pub async fn get_status_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
) -> impl IntoResponse {
    let s = state.read().await;
    let is_linked = s.settings.as_ref().map(|st| st.session_token.is_some()).unwrap_or(false);
    axum::Json(json!({
        "arrpcDetected": s.arrpc_detected,
        "wsStatus": s.ws_status,
        "rpcStatus": s.rpc_status,
        "isLinked": is_linked,
    }))
}

pub async fn open_settings_file_handler() -> impl IntoResponse {
    let path = crate::utils::get_app_dir().join("settings.json");
    if open::that(&path).is_ok() {
        axum::response::Html("Success").into_response()
    } else {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to open file").into_response()
    }
}

/// Live state of the client-managed virtual audio bus.
pub async fn get_audio_bus_handler() -> impl IntoResponse {
    let status = tokio::task::spawn_blocking(crate::audio_bus::status).await.unwrap_or_default();
    axum::Json(status)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBusRequest {
    #[serde(default)]
    pub action: String,
    pub volume: Option<u8>,
}

/// One-shot control for the audio bus: create it, drop it, change the listen
/// level, or retire the old background service. Used by the live volume slider
/// so dragging it takes effect immediately (Deploy still persists the value).
pub async fn audio_bus_action_handler(
    State((state, _)): State<(AppState, Arc<ServerHandle>)>,
    axum::extract::Json(req): axum::extract::Json<AudioBusRequest>,
) -> impl IntoResponse {
    let saved = state
        .read()
        .await
        .settings
        .as_ref()
        .map(|s| s.audio.listen_volume)
        .unwrap_or(75);
    let action = req.action.clone();
    // Clamp before the u32->u8 cast: a hand-edited settings.json with e.g. 300
    // would otherwise wrap to 44 and silently drop the volume.
    let volume = req.volume.unwrap_or(saved.min(150) as u8).min(150);

    let result = tokio::task::spawn_blocking(move || match action.as_str() {
        "enable" | "volume" => crate::audio_bus::ensure(volume).map(|_| ()),
        "disable" => crate::audio_bus::remove(),
        "disableLegacy" => crate::audio_bus::disable_legacy_service(),
        other => Err(format!("unknown audio bus action '{}'", other)),
    })
    .await;

    match result {
        Ok(Ok(())) => {
            let status = tokio::task::spawn_blocking(crate::audio_bus::status).await.unwrap_or_default();
            axum::Json(json!({ "ok": true, "status": status })).into_response()
        }
        Ok(Err(e)) => {
            info!("[AudioBus] action failed: {}", e);
            (axum::http::StatusCode::BAD_REQUEST, axum::Json(json!({ "ok": false, "error": e })))
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}
