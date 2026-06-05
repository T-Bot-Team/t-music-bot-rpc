pub mod handlers;
pub mod templates;

use crate::AppState;
use axum::{
    response::{IntoResponse},
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct ServerHandle {
    pub tx: broadcast::Sender<String>,
}

pub async fn start_server(state: AppState, shutdown_rx: tokio::sync::oneshot::Receiver<()>) {
    let (port, allow_firewall, tx) = {
        let s = state.read().await;
        let settings = s.settings.as_ref().unwrap();
        (
            settings.overlay.port,
            settings.allow_firewall,
            s.overlay_tx.clone(),
        )
    };

    let server_handle = Arc::new(ServerHandle { tx });

    let app = Router::new()
        .route("/", get(handlers::overlay_handler))
        .route("/ws", get(handlers::ws_handler))
        .route("/settings", get(handlers::settings_gui_handler))
        .route("/setup", get(handlers::setup_gui_handler))
        .route(
            "/api/settings",
            get(handlers::get_settings_handler).post(handlers::update_settings_handler),
        )
        .route("/api/settings/open", axum::routing::post(handlers::open_settings_file_handler))
        .route("/api/defaults", get(handlers::get_defaults_handler))
        .route("/api/rpc/sync", get(handlers::rpc_sync_handler))
        .route("/api/devices", get(handlers::get_devices_handler))
        .route("/api/status", get(handlers::get_status_handler))
        .route("/api/discord_clients", get(handlers::get_discord_clients_handler))
        .route("/assets/:path", get(asset_handler))
        .route("/ui/assets/:path", get(asset_handler))
        .with_state((state.clone(), server_handle));

    let bind_ip = if allow_firewall { "0.0.0.0" } else { "127.0.0.1" };
    let addr = format!("{}:{}", bind_ip, port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("[Server] Overlay active on http://{}", addr);
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                    info!("[Server] Shutting down listener on port {}...", port);
                })
                .await
                .unwrap();
        }
        Err(e) => {
            let err_msg = format!("CRITICAL: Permission Denied or Port {} is already in use.\n\nError: {}\n\nPlease change the port in settings.json or close the conflicting application.", port, e);
            info!("[Server] {}", err_msg);
            crate::utils::show_error_popup(&err_msg);
        }
    }
}

async fn asset_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    let clean_path = path.trim_start_matches('/');
    let mime = match clean_path.split('.').last() {
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        _ => "application/octet-stream",
    };

    let content: Option<&'static [u8]> = match clean_path {
        "avatar.png" => Some(include_bytes!("../../assets/avatar.png")),
        "music.png" => Some(include_bytes!("../../assets/music.png")),
        "icon.png" => Some(include_bytes!("../../assets/icon.png")),
        "icon.ico" => Some(include_bytes!("../../assets/icon.ico")),
        "inter.woff2" => Some(include_bytes!("../../assets/inter.woff2")),
        "inter-bold.woff2" => Some(include_bytes!("../../assets/inter-bold.woff2")),
        "pickr.min.js" => Some(include_bytes!("../../assets/pickr.min.js")),
        "pickr.min.css" => Some(include_bytes!("../../assets/pickr.min.css")),
        
        "gui.css" => Some(include_bytes!("../ui/assets/gui.css")),
        "gui.js" => Some(include_bytes!("../ui/assets/gui.js")),
        "setup.css" => Some(include_bytes!("../ui/assets/setup.css")),
        "setup.js" => Some(include_bytes!("../ui/assets/setup.js")),
        _ => None,
    };

    if let Some(c) = content {
        ([(axum::http::header::CONTENT_TYPE, mime)], c).into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}
