use crate::{AppState, TrackUpdate, info, utils};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use futures_util::{StreamExt, SinkExt};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use base64::{Engine as _, engine::general_purpose};
use std::io::{Read, Write};

const WS_URL_B64: &str = "d3NzOi8vcnBjLnRlaGNyYWZ0Lnh5ei93cw==";
const APP_VERSION: &str = "v1.1.0";
const DISCORD_RATELIMIT: Duration = Duration::from_secs(5); // Lowered to Discord's actual 5s limit

#[derive(Debug, Clone)]
enum RpcCommand {
    Connect(String),
    Update(TrackUpdate),
}

pub async fn start_rpc_handler(state: AppState) {
    let ws_url = String::from_utf8(general_purpose::STANDARD.decode(WS_URL_B64).unwrap()).unwrap();
    let (rpc_tx, rpc_rx) = mpsc::channel::<RpcCommand>(128);
    
    let state_worker = state.clone();
    std::thread::spawn(move || { run_discord_worker(state_worker, rpc_rx); });

    loop {
        info!("[WS] Connecting...");
        { let mut s = state.write().await; s.ws_status = "Connecting...".to_string(); }
        
        let mut request = ws_url.clone().into_client_request().unwrap();
        let headers = request.headers_mut();
        headers.insert("User-Agent", format!("T_Music_Bot-RPC/{} ({})", APP_VERSION, if cfg!(windows) { "win32" } else { "linux" }).parse().unwrap());
        headers.insert("Origin", "https://rpc.tehcraft.xyz".parse().unwrap());

        match connect_async(request).await {
            Ok((mut ws_stream, _)) => {
                info!("[WS] Connected");
                { let mut s = state.write().await; s.ws_status = "Connected".to_string(); }
                
                let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(128);
                let _ = ws_stream.send(Message::Text(json!({"type": "connect"}).to_string().into())).await;

                loop {
                    tokio::select! {
                        out_msg = msg_rx.recv() => {
                            if let Some(m) = out_msg { if ws_stream.send(m).await.is_err() { break; } }
                        }
                        in_msg = ws_stream.next() => {
                            match in_msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(m) = serde_json::from_str::<Value>(&text) {
                                        match m["type"].as_str() {
                                            Some("ping") => { let _ = msg_tx.send(Message::Text(json!({"type": "pong", "time": m["time"]}).to_string().into())).await; }
                                            Some("connect") => {
                                                let cid = m["clientId"].as_str().unwrap_or("").to_string();
                                                let _ = rpc_tx.send(RpcCommand::Connect(cid)).await;
                                                let st = state.clone(); let tx = msg_tx.clone();
                                                tokio::spawn(async move { authenticate(tx, st).await; });
                                            }
                                            Some("rpc_update") => {
                                                if let Ok(raw) = serde_json::from_value::<TrackUpdate>(m["data"].clone()) {
                                                    let overlay_fmt = utils::format_overlay_track(raw.clone());
                                                    
                                                    // 1. UPDATE STATE
                                                    {
                                                        let mut s = state.write().await;
                                                        s.last_track = Some(overlay_fmt.clone());
                                                        // 2. BROADCAST TO ALL WS CLIENTS
                                                        let _ = s.overlay_tx.send(json!({ "type": "track_update", "data": overlay_fmt }).to_string());
                                                    }
                                                    
                                                    // 3. SEND TO DISCORD QUEUE
                                                    let _ = rpc_tx.send(RpcCommand::Update(utils::format_rpc_track(raw))).await;
                                                }
                                            }
                                            Some("authenticated") => {
                                                info!("[Auth] OK");
                                                if let Some(uid) = { state.read().await.settings.as_ref().and_then(|st| st.user_id.clone()) } {
                                                    let _ = msg_tx.send(Message::Text(json!({ "type": "request_update", "userId": uid }).to_string().into())).await;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                _ => break,
                            }
                        }
                    }
                }
                { let mut s = state.write().await; s.ws_status = "Disconnected".to_string(); s.rpc_status = "Disconnected".to_string(); }
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(5)).await,
        }
    }
}

fn run_discord_worker(state: AppState, mut rx: mpsc::Receiver<RpcCommand>) {
    let mut client: Option<DiscordIpcClient> = None;
    let mut last_update = Instant::now() - Duration::from_secs(60);
    let mut queued: Option<TrackUpdate> = None;
    let mut active_client_id: Option<String> = None; // Track the ID to auto-reconnect

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                RpcCommand::Connect(id) => {
                    active_client_id = Some(id.clone());
                    if let Some(uid) = get_user_id_surgical(&id) {
                        let mut s = state.blocking_write();
                        if let Some(st) = s.settings.as_mut() { st.user_id = Some(uid); utils::save_settings(st); }
                    }
                    if let Some(mut old) = client.take() { let _ = old.close(); }
                    
                    // Initial connection attempt
                    if let Ok(mut c) = DiscordIpcClient::new(&id) {
                        if c.connect().is_ok() {
                            info!("[RPC] Connected to Discord");
                            state.blocking_write().rpc_status = "Connected".to_string();
                            client = Some(c);
                        }
                    }
                }
                RpcCommand::Update(track) => { queued = Some(track); }
            }
        }

        // 🚨 THE FIX: AUTO-RECONNECT
        // If we have an ID but client died or failed to connect, try again
        if client.is_none() {
            if let Some(id) = &active_client_id {
                if let Ok(mut c) = DiscordIpcClient::new(id) {
                    if c.connect().is_ok() {
                        info!("[RPC] Reconnected to Discord");
                        state.blocking_write().rpc_status = "Connected".to_string();
                        client = Some(c);
                    }
                }
            }
        }

        if let (Some(c), Some(track)) = (&mut client, queued.take()) {
            if last_update.elapsed() >= DISCORD_RATELIMIT {
                if let Err(e) = update_discord_activity_raw(c, &track) {
                    info!("[RPC] Connection Lost/Error: {}", e);
                    let _ = c.close(); 
                    client = None; // Will trigger auto-reconnect on next loop
                    state.blocking_write().rpc_status = "Disconnected".to_string();
                    queued = Some(track); // Don't drop the data
                } else {
                    last_update = Instant::now();
                }
            } else { queued = Some(track); }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn update_discord_activity_raw(client: &mut DiscordIpcClient, track: &TrackUpdate) -> Result<(), Box<dyn std::error::Error>> {
    if track.details.is_none() && track.state.is_none() { return client.clear_activity(); }
    
    let mut activity = json!({
        "type": 2, // Listening
        "details": track.details.as_deref().unwrap_or(" "), // Prevent empty string crashes
        "state": track.state.as_deref().unwrap_or(" "),     // Prevent empty string crashes
        "assets": {
            "small_image": track.small_image_key.as_deref().unwrap_or(""),
            "small_text": track.small_image_text.as_deref().unwrap_or(""),
        },
    });

    if !track.paused.unwrap_or(false) {
        activity["timestamps"] = json!({
            "start": track.start_timestamp.map(|t| t as i64),
            "end": track.end_timestamp.map(|t| t as i64),
        });
    }

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": std::process::id(),
            "activity": activity
        },
        "nonce": uuid::Uuid::new_v4().to_string()
    });

    client.send(payload, 1)?;
    Ok(())
}

fn get_user_id_surgical(client_id: &str) -> Option<String> {
    let pipe_path = if cfg!(windows) { r"\\.\pipe\discord-ipc-0".to_string() } else {
        let temp = std::env::var("XDG_RUNTIME_DIR").or_else(|_| std::env::var("TMPDIR")).unwrap_or_else(|_| "/tmp".into());
        format!("{}/discord-ipc-0", temp)
    };

    let mut stream = std::fs::OpenOptions::new().read(true).write(true).open(pipe_path).ok()?;
    let payload = json!({"v": 1, "client_id": client_id}).to_string();
    let mut header = [0u8; 8];
    header[0..4].copy_from_slice(&(0u32).to_le_bytes());
    header[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    let _ = stream.write_all(&header);
    let _ = stream.write_all(payload.as_bytes());

    let mut resp_header = [0u8; 8];
    if stream.read_exact(&mut resp_header).is_ok() {
        let len = u32::from_le_bytes([resp_header[4], resp_header[5], resp_header[6], resp_header[7]]) as usize;
        let mut buffer = vec![0u8; len];
        if stream.read_exact(&mut buffer).is_ok() {
            let resp: Value = serde_json::from_slice(&buffer).ok()?;
            return resp["data"]["user"]["id"].as_str().map(|s| s.to_string());
        }
    }
    None
}

async fn authenticate(tx: mpsc::Sender<Message>, state: AppState) {
    let mut is_prompting = false;
    loop {
        let (uid, mut code) = {
            let s = state.read().await;
            let st = s.settings.as_ref().unwrap();
            (st.user_id.clone().unwrap_or_default(), st.code.clone().unwrap_or_default())
        };

        // 🚨 WAIT FOR WEB UI ON FIRST LAUNCH
        if code.is_empty() {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        // 🚨 TRIGGER NATIVE POPUP IF CODE IS INVALID (but not empty)
        if code.len() != 6 && !is_prompting {
            is_prompting = true;

            let new_code = utils::get_pairing_code();
            if new_code == "CANCELLED" {
                info!("[Auth] Pairing cancelled. Exiting.");
                std::process::exit(0);
            }
            code = new_code.clone();
            {
                let mut s = state.write().await;
                if let Some(st) = s.settings.as_mut() { 
                    st.code = Some(new_code); 
                    utils::save_settings(st); 
                }
            }
            is_prompting = false;
        }

        // SUCCESS: We have both!
        if !uid.is_empty() && code.len() == 6 {
            info!("[Auth] Credentials acquired. Sending to server...");
            let _ = tx.send(Message::Text(json!({ "type": "auth", "userId": uid, "code": code }).to_string().into())).await;
            break;
        }

        // UID GRAB WAIT
        if uid.is_empty() {
            tokio::time::sleep(Duration::from_secs(2)).await;
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}