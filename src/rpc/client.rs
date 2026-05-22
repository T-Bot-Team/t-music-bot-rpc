use crate::{info, AppState, APP_VERSION};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;
use std::time::Duration;

pub async fn authenticate(ws_tx: mpsc::Sender<Message>, state: AppState, _client_id: String) {
    let mut last_log_state = 0; // 0 = init, 1 = wait uid, 2 = wait code

    loop {
        if state.read().await.is_shutting_down { break; }
        
        let (uid, code, token) = {
            let s = state.read().await;
            if s.settings.is_none() { return; }
            let st = s.settings.as_ref().unwrap();
            (st.user_id.clone().unwrap_or_default(), st.code.clone().unwrap_or_default(), st.session_token.clone())
        };

        if uid.is_empty() {
            if last_log_state != 1 {
                info!("[Auth] Waiting for User ID. Required for server security.");
                last_log_state = 1;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        if code.len() == 6 {
            let auth_msg = json!({ "type": "authenticate", "userId": uid, "code": code, "version": APP_VERSION });
            if ws_tx.send(Message::Text(auth_msg.to_string().into())).await.is_err() { break; }
            info!("[Auth] Authenticating with Pairing Code. Linking to Discord User: {}", uid);
            break;
        }

        if let Some(t) = token {
            if !t.is_empty() {
                let auth_msg = json!({ "type": "authenticate", "userId": uid, "token": t, "version": APP_VERSION });
                if ws_tx.send(Message::Text(auth_msg.to_string().into())).await.is_err() { break; }
                info!("[Auth] Authenticating with secure Session Token...");
                break;
            }
        }

        if last_log_state != 2 {
            info!("[Auth] Waiting for valid 6-digit Pairing Code...");
            last_log_state = 2;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
