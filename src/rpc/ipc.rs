use serde_json::{json, Value};
use crate::AppState;

#[cfg(windows)]
type IpcSocket = std::fs::File;

#[cfg(unix)]
type IpcSocket = std::os::unix::net::UnixStream;

pub struct IpcClient {
    socket: Option<IpcSocket>,
}

#[cfg(windows)]
impl IpcClient {
    pub fn new(pipe: i32) -> Option<Self> {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;
        
        let pipes: Vec<i32> = if pipe == -1 { (0..10).collect() } else { vec![pipe] };
        for i in pipes {
            let path = format!(r"\\?\pipe\discord-ipc-{}", i);
            if let Ok(file) = OpenOptions::new().access_mode(0x3).open(&path) {
                return Some(Self { socket: Some(file) });
            }
        }
        None
    }
}

#[cfg(unix)]
impl IpcClient {
    pub fn new(pipe: i32) -> Option<Self> {
        use std::os::unix::net::UnixStream;
        
        let pipes: Vec<i32> = if pipe == -1 { (0..10).collect() } else { vec![pipe] };
        
        let temp_dirs = vec![
            std::env::var("XDG_RUNTIME_DIR").ok().map(std::path::PathBuf::from),
            std::env::var("TMPDIR").ok().map(std::path::PathBuf::from),
            std::env::var("TMP").ok().map(std::path::PathBuf::from),
            std::env::var("TEMP").ok().map(std::path::PathBuf::from),
            Some(std::path::PathBuf::from("/tmp")),
        ];

        for i in pipes {
            for dir_opt in &temp_dirs {
                if let Some(dir) = dir_opt {
                    let path = dir.join(format!("discord-ipc-{}", i));
                    if let Ok(stream) = UnixStream::connect(&path) {
                        return Some(Self { socket: Some(stream) });
                    }
                }
            }
        }
        None
    }
}
impl IpcClient {
    pub fn send(&mut self, payload: Value, opcode: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(socket) = &mut self.socket {
            use std::io::Write;
            let json_str = payload.to_string();
            let bytes = json_str.as_bytes();
            let mut buf = Vec::new();
            buf.extend_from_slice(&opcode.to_le_bytes());
            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(bytes);
            socket.write_all(&buf)?;
            socket.flush()?;
            Ok(())
        } else {
            Err("No socket".into())
        }
    }
    
    pub fn recv(&mut self) -> Result<(u32, Value), Box<dyn std::error::Error>> {
        if let Some(socket) = &mut self.socket {
            use std::io::Read;
            let mut header = [0u8; 8];
            socket.read_exact(&mut header)?;
            let opcode = u32::from_le_bytes(header[0..4].try_into().unwrap());
            let len = u32::from_le_bytes(header[4..8].try_into().unwrap());
            let mut buf = vec![0u8; len as usize];
            socket.read_exact(&mut buf)?;
            let json: Value = serde_json::from_slice(&buf)?;
            Ok((opcode, json))
        } else {
            Err("No socket".into())
        }
    }
    
    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.send(json!({}), 2);
        self.socket = None;
        Ok(())
    }
}

pub fn scan_discord_clients() -> Vec<Value> {
    let mut clients = Vec::new();
    for pipe in 0..10 {
        if let Some(mut c) = IpcClient::new(pipe) {
            let _ = c.send(json!({ "v": 1, "client_id": "1045800378228281345" }), 0); 
            if let Ok((_opcode, payload)) = c.recv() {
                #[cfg(debug_assertions)]
                info!("[RPC Debug] Pipe {} raw payload: {}", pipe, payload);

                if let Some(user) = payload.get("data").and_then(|d| d.get("user")) {
                    let id = user.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                    let username = user.get("username").and_then(|u| u.as_str()).unwrap_or("").to_string();
                    let global_name = user.get("global_name").and_then(|g| g.as_str()).unwrap_or("");
                    
                    if !id.is_empty() {
                        let display_name = if id == "1045800378228281345" {
                            "arRPC / WebCord".to_string()
                        } else if !global_name.is_empty() {
                            global_name.to_string()
                        } else {
                            username
                        };

                        clients.push(json!({
                            "pipe": pipe,
                            "id": id,
                            "display_name": display_name
                        }));
                    }
                }
            }
            let _ = c.close();
        }
    }
    clients
}

pub fn attempt_connect(cid: &str, state: &AppState) -> Option<IpcClient> {
    let (pipe_pref, configured_uid) = {
        let s = state.blocking_read();
        let pref = s.settings.as_ref().map(|st| st.rpc.ipc_pipe).unwrap_or(-1);
        let uid = s.settings.as_ref().and_then(|st| st.user_id.clone()).unwrap_or_default();
        (pref, uid)
    };

    let pipes_to_check: Vec<i32> = if pipe_pref == -1 { (0..10).collect() } else { vec![pipe_pref] };
    let mut fallback_pipe = None;
    
    for pipe in pipes_to_check {
        if let Some(mut c) = IpcClient::new(pipe) {
            let _ = c.send(json!({ "v": 1, "client_id": cid }), 0);
            
            if let Ok((_opcode, payload)) = c.recv() {
                if let Some(uid) = payload.get("data").and_then(|d| d.get("user")).and_then(|u| u.get("id")).and_then(|i| i.as_str()) {
                    
                    // 🚀 Smart Auto-Detect: If auto-detecting and a specific user account is configured, try to match it.
                    // BUT, save the first valid pipe as a fallback just in case the configured one is offline.
                    if pipe_pref == -1 && !configured_uid.is_empty() && configured_uid != "1045800378228281345" && uid != configured_uid {
                        if fallback_pipe.is_none() {
                            fallback_pipe = Some((pipe, uid.to_string()));
                        }
                        let _ = c.close();
                        continue; 
                    }

                    return setup_pipe(c, pipe, uid, state);
                }
            }
            let _ = c.close();
        }
    }

    // If we didn't find the exact configured user, but we found *a* valid Discord client during auto-detect, use it.
    if let Some((f_pipe, f_uid)) = fallback_pipe {
        if let Some(mut c) = IpcClient::new(f_pipe) {
            let _ = c.send(json!({ "v": 1, "client_id": cid }), 0);
            if let Ok((_opcode, _payload)) = c.recv() {
                info!("[RPC] Configured User ID not found. Falling back to available Discord client.");
                return setup_pipe(c, f_pipe, &f_uid, state);
            }
        }
    }

    None
}

fn setup_pipe(c: IpcClient, pipe: i32, uid: &str, state: &AppState) -> Option<IpcClient> {
    let mut needs_reconnect = false;

    if uid == "1045800378228281345" {
        let has_saved_id = {
            let s = state.blocking_read();
            s.settings.as_ref().map(|st| st.user_id.is_some() && !st.user_id.as_ref().unwrap().is_empty()).unwrap_or(false)
        };
        
        if has_saved_id {
            info!("[Auth] arRPC detected on Pipe {}. Using previously saved User ID.", pipe);
        } else {
            info!("[Auth] arRPC detected on Pipe {}. Manual User ID required.", pipe);
        }
        
        let mut s = state.blocking_write();
        s.arrpc_detected = true;
    } else {
        info!("[Auth] Identified Discord User: {} on Pipe {}", uid, pipe);
        let mut s = state.blocking_write();
        s.arrpc_detected = false;
        
        if let Some(st) = s.settings.as_mut() {
            if st.user_id.as_deref() != Some(uid) {
                info!("[Auth] System User ID updated to match active Discord client ({}).", uid);
                st.user_id = Some(uid.to_string());
                st.session_token = None; // Wipe the old token since it belongs to the old user
                
                // If we have a saved code for this specific user, load it
                if let Some(saved) = st.saved_codes.get(uid) {
                    st.code = Some(saved.clone());
                } else {
                    st.code = Some(String::new());
                }
                
                crate::config::save_settings(st);
                needs_reconnect = true;
            }
        }
    }

    if needs_reconnect {
        let s = state.blocking_read();
        let _ = s.overlay_tx.send(serde_json::json!({ "type": "settings_update" }).to_string());
    }

    {
        let mut s = state.blocking_write();
        s.rpc_status = "Connected".to_string();
        s.active_pipe = Some(pipe);
    }
    info!("[RPC] Local Pipe Connection Established (Pipe {})", pipe);
    Some(c)
}

pub fn clear_activity(client: &mut IpcClient) -> Result<(), Box<dyn std::error::Error>> {
    client.send(json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": json!(null) },
        "nonce": uuid::Uuid::new_v4().to_string()
    }), 1)
}
