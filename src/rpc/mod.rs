pub mod ipc;
pub mod client;
pub mod logic;

use crate::{AppState, TrackUpdate, RpcCommand, APP_VERSION};
use serde_json::{json, Value};
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};
use base64::{Engine as _, engine::general_purpose};
use tokio::sync::mpsc;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const WS_URL_B64: &str = "d3NzOi8vcnBjLnRlaGNyYWZ0Lnh5ei93cw==";

pub use ipc::scan_discord_clients;

pub async fn start_rpc_handler(state: AppState, rpc_rx: std_mpsc::Receiver<RpcCommand>) {
    let ws_url = String::from_utf8(general_purpose::STANDARD.decode(WS_URL_B64).unwrap()).unwrap();
    
    let state_worker = state.clone();
    std::thread::spawn(move || { run_discord_worker(state_worker, rpc_rx); });

    let s_watch = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            let current = {
                let s = s_watch.read().await;
                s.last_track.clone()
            };

            if let Some(track) = current {
                if track.status == "playing" {
                    if let Some(end) = track.end_timestamp {
                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
                        if now > end + 1.0 {
                            info!("[Server] Track expired (Watchdog). Transitioning to Idle.");
                            logic::apply_new_state(&s_watch, TrackUpdate::default(), true, json!({"status": "idle"})).await;
                        }
                    }
                }
            }
        }
    });

    loop {
        if state.read().await.is_shutting_down { break; }
        
        info!("[WS] Connecting...");
        { 
            let mut s = state.write().await; 
            s.ws_status = "Connecting...".to_string(); 
            s.last_track = None;
        }
        
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
                
                let mut settings_rx = state.read().await.overlay_tx.subscribe();

                let hb_tx = msg_tx.clone();
                let hb_state = state.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(10));
                    let mut last_activity = Instant::now();
                    loop {
                        interval.tick().await;
                        if hb_state.read().await.is_shutting_down { break; }
                        
                        if last_activity.elapsed() >= Duration::from_secs(90) {
                            let ping = json!({ "type": "ping", "time": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() });
                            if hb_tx.send(Message::Text(ping.to_string().into())).await.is_err() { break; }
                            last_activity = Instant::now();
                        }
                    }
                });

                let current_creds = {
                    let s = state.read().await;
                    s.settings.as_ref().map(|st| (st.code.clone(), st.user_id.clone())).unwrap_or_default()
                };

                let mut auth_task: Option<tokio::task::JoinHandle<()>> = None;

                loop {
                    if state.read().await.is_shutting_down { break; }
                    
                    tokio::select! {
                        res = settings_rx.recv() => {
                            if let Ok(msg) = res {
                                if let Ok(m) = serde_json::from_str::<Value>(&msg) {
                                    if m["type"] == "settings_update" {
                                        // 🚀 Check if auth credentials actually changed. If not, ignore the disconnect!
                                        let new_creds = {
                                            let s = state.read().await;
                                            s.settings.as_ref().map(|st| (st.code.clone(), st.user_id.clone())).unwrap_or_default()
                                        };
                                        
                                        if new_creds != current_creds {
                                            info!("[WS] Auth credentials updated. Reconnecting to bridge...");
                                            break;
                                        } else {
                                            // Visualizer/Overlay settings updated. No need to drop the bridge connection.
                                        }
                                    }
                                }
                            }
                        }
                        out_msg = msg_rx.recv() => {
                            if let Some(m) = out_msg { 
                                if ws_stream.send(m).await.is_err() { break; } 
                            }
                        }
                        in_msg = ws_stream.next() => {
                            match in_msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(m) = serde_json::from_str::<Value>(&text) {
                                        match m["type"].as_str() {
                                            Some("ping") => { let _ = msg_tx.send(Message::Text(json!({"type": "pong", "time": m["time"]}).to_string().into())).await; }
                                            Some("connect") => {
                                                let cid = m["clientId"].as_str().unwrap_or("").to_string();
                                                let rpc_tx = state.read().await.rpc_tx.clone();
                                                if let Some(tx) = rpc_tx { let _ = tx.send(RpcCommand::Connect(cid.clone())); }
                                                
                                                // 🚀 ABORT PREVIOUS AUTH: Ensure only one auth attempt is active!
                                                if let Some(task) = auth_task.take() { task.abort(); }
                                                
                                                let st = state.clone(); let tx = msg_tx.clone();
                                                auth_task = Some(tokio::spawn(async move { client::authenticate(tx, st, cid).await; }));
                                            }
                                            Some("rpc_update") => {
                                                let raw_rpc = m["data"].clone();
                                                if let Ok(track) = serde_json::from_value::<TrackUpdate>(raw_rpc.clone()) {
                                                    logic::apply_new_state(&state, track, false, raw_rpc).await;
                                                }
                                            }
                                            Some("authenticated") => {
                                                info!("[Auth] Bridge Connection OK");
                                                
                                                // 🚀 TOKEN EXCHANGE: If the server provided a session token, save it for future auto-reconnects
                                                if let Some(token) = m.get("token").and_then(|t| t.as_str()) {
                                                    let mut s = state.write().await;
                                                    if let Some(st) = s.settings.as_mut() {
                                                        st.session_token = Some(token.to_string());
                                                        st.code = None; // Clear the code after successful link!
                                                        crate::config::save_settings(st);
                                                        
                                                        // 🚀 SYNC GUI: Notify the overlay/GUI that settings changed (code cleared)
                                                        let _ = s.overlay_tx.send(json!({ "type": "settings_update" }).to_string());
                                                        
                                                        info!("[Auth] Secure Session Token received and saved. Pairing code cleared and GUI notified.");
                                                    }
                                                }

                                                let st_clone = state.clone();
                                                let tx_clone = msg_tx.clone();
                                                tokio::spawn(async move {
                                                    for _ in 0..3 {
                                                        let uid = {
                                                            let s = st_clone.read().await;
                                                            s.settings.as_ref().and_then(|st| st.user_id.clone())
                                                        };
                                                        if let Some(uid) = uid {
                                                            let _ = tx_clone.send(Message::Text(json!({ "type": "request_update", "userId": uid }).to_string().into())).await;
                                                            break;
                                                        }
                                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                                    }
                                                });
                                            }
                                            Some("auth_failed") | Some("error") => {
                                                let is_auth_err = m["type"] == "auth_failed" || m["message"].as_str().map(|msg| msg.contains("auth") || msg.contains("token") || msg.contains("code") || msg.contains("Invalid")).unwrap_or(false);
                                                if is_auth_err {
                                                    let mut s = state.write().await;
                                                    if let Some(st) = s.settings.as_mut() {
                                                        if st.session_token.is_some() || st.code.as_ref().map(|c| !c.is_empty()).unwrap_or(false) {
                                                            info!("[Auth] Session Token rejected by server. Clearing stale credentials...");
                                                            st.session_token = None;
                                                            st.code = None;
                                                            crate::config::save_settings(st);
                                                            
                                                            // 🚀 SYNC GUI: Notify the overlay/GUI that settings changed
                                                            let _ = s.overlay_tx.send(json!({ "type": "settings_update" }).to_string());
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    info!("[WS] Stream Error: {}", e);
                                    break;
                                }
                                _ => break,
                            }
                        }
                    }
                }
                { let mut s = state.write().await; s.ws_status = "Disconnected".to_string(); s.rpc_status = "Disconnected".to_string(); }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                info!("[WS] Connection failed: {}. Retrying in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn run_discord_worker(state: AppState, rpc_rx: std_mpsc::Receiver<RpcCommand>) {
    let mut client: Option<ipc::IpcClient> = None;
    let mut last_update = Instant::now() - Duration::from_secs(60);
    let mut queued: Option<(TrackUpdate, Value, bool)> = None;
    let mut sync_pending = false;
    let mut current_client_id = String::new();
    let mut last_reconnect_attempt = Instant::now();

    loop {
        while let Ok(cmd) = rpc_rx.try_recv() {
            match cmd {
                RpcCommand::Connect(cid) => {
                    current_client_id = cid;
                    if let Some(mut c) = client.take() { 
                        let _ = ipc::clear_activity(&mut c);
                        let _ = c.close(); 
                    }
                    client = ipc::attempt_connect(&current_client_id, &state);
                    if client.is_some() { 
                        sync_pending = true; 
                        // 🚀 Catch-up: Pull the last known state from memory so we sync immediately upon connection
                        let s = state.blocking_read();
                        if let (Some(track), Some(rpc_raw)) = (s.last_track.clone(), s.last_rpc_raw.clone()) {
                            queued = Some((track, rpc_raw, true));
                        }
                    }
                }
                RpcCommand::Update { track, rpc_raw, is_transition } => {
                    queued = Some((track, rpc_raw, is_transition));
                }
                RpcCommand::Refresh => {
                    sync_pending = true;
                    if let Some(mut c) = client.take() { 
                        let _ = ipc::clear_activity(&mut c);
                        let _ = c.close(); 
                    }
                    if queued.is_none() {
                        let s = state.blocking_read();
                        if let (Some(track), Some(rpc_raw)) = (s.last_track.clone(), s.last_rpc_raw.clone()) {
                            queued = Some((track, rpc_raw, true));
                        }
                    }
                    last_reconnect_attempt = Instant::now() - Duration::from_secs(60);
                }
            }
        }

        if client.is_none() && !current_client_id.is_empty() && last_reconnect_attempt.elapsed() >= Duration::from_secs(10) {
            last_reconnect_attempt = Instant::now();
            client = ipc::attempt_connect(&current_client_id, &state);
            if client.is_some() { 
                sync_pending = true; 
                // 🚀 Catch-up: Pull the last known state from memory so we sync immediately upon connection
                let s = state.blocking_read();
                if let (Some(track), Some(rpc_raw)) = (s.last_track.clone(), s.last_rpc_raw.clone()) {
                    queued = Some((track, rpc_raw, true));
                }
            }
        }

        if let (Some(c), Some((track, rpc_raw, is_transition))) = (&mut client, queued.take()) {
            if sync_pending || last_update.elapsed() >= logic::DISCORD_RATELIMIT {
                if let Err(e) = logic::update_discord_activity_raw(c, &rpc_raw, &state) {
                    info!("[RPC] Connection Lost: {}. Searching for Discord...", e);
                    let _ = c.close(); client = None;
                    {
                        let mut s = state.blocking_write();
                        s.rpc_status = "Disconnected".to_string();
                        s.active_pipe = None;
                    }
                    queued = Some((track, rpc_raw, is_transition));
                } else {
                    last_update = Instant::now();
                    sync_pending = false;
                }
            } else {
                queued = Some((track, rpc_raw, is_transition));
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
