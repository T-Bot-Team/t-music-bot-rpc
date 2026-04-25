#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::utils::log_message(&msg);
    }};
}

pub mod config;
pub mod utils;
pub mod server;
pub mod rpc;
pub mod visualizer;
pub mod tray;

pub use config::*;

use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use std::fs;

pub struct GlobalState {
    pub is_shutting_down: bool,
    pub last_track: Option<TrackUpdate>,
    pub settings: Option<Settings>,
    pub overlay_tx: broadcast::Sender<String>,
    pub ws_status: String,
    pub rpc_status: String,
}

pub type AppState = Arc<RwLock<GlobalState>>;

fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    
    {
        let _ = fs::OpenOptions::new().write(true).truncate(true).create(true).open(utils::get_log_path());
        std::panic::set_hook(Box::new(|panic_info| {
            let msg = format!("[CRITICAL PANIC] The application crashed: {}", panic_info);
            crate::utils::log_message(&msg);
        }));
    }
    
    info!("=======================================================");
    info!("   T_Music_Bot RPC (RUST)   ");
    info!("=======================================================");

    let settings = config::load_settings();
    utils::check_lock(settings.overlay.port);
    
    let is_first_run = settings.code.as_deref().unwrap_or("").len() != 6;
    let port = settings.overlay.port;
    
    let (overlay_tx, mut restart_rx) = broadcast::channel(4096);

    let state = Arc::new(RwLock::new(GlobalState {
        is_shutting_down: false,
        last_track: None,
        settings: Some(settings.clone()),
        overlay_tx: overlay_tx.clone(),
        ws_status: "Disconnected".to_string(),
        rpc_status: "Disconnected".to_string(),
    }));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let s1 = state.clone();
    rt.spawn(async move { server::start_server(s1).await; });

    if is_first_run {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = open::that(format!("http://127.0.0.1:{}/setup", port));
        });
    }

    let s2 = state.clone();
    rt.spawn(async move { rpc::start_rpc_handler(s2).await; });

    // 🚀 REACTIVE VISUALIZER HANDLER
    let s3 = state.clone();
    rt.spawn(async move { 
        loop {
            let config = {
                let s = s3.read().await;
                s.settings.as_ref().unwrap().overlay.visualizer.clone()
            };

            if config.enabled {
                info!("[Visualizer] Starting audio capture...");
                let abort_handle = visualizer::start_visualizer(s3.clone()).await;
                
                // Wait for restart signal
                while let Ok(msg) = restart_rx.recv().await {
                    if msg.contains("settings_update") {
                        info!("[Visualizer] Restart signal received. Re-initializing...");
                        if let Some(h) = abort_handle { h.abort(); }
                        break; 
                    }
                }
            } else {
                // If disabled, wait for a signal to maybe enable it
                if let Err(_) = restart_rx.recv().await { break; }
            }
        }
    });

    let shutdown_state = state.clone();
    let shutdown_tx = overlay_tx.clone();
    ctrlc::set_handler(move || {
        let s_state = shutdown_state.clone();
        let s_tx = shutdown_tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                { let mut s = s_state.write().await; s.is_shutting_down = true; }
                let _ = s_tx.send(serde_json::json!({ "type": "program_shutdown" }).to_string());
            });
            std::thread::sleep(std::time::Duration::from_millis(500));
            std::process::exit(0);
        });
    }).expect("Error setting Ctrl-C handler");

    tray::create_tray(state, rt);
}