#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::utils::log_message(&msg);
    }};
}

pub mod models;
pub mod config;
pub mod utils;
pub mod server;
pub mod rpc;
pub mod visualizer;
pub mod tray;

pub use models::*;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use std::fs;

fn main() {
    {
        // 🚀 TRUNCATE ON STARTUP: Keep one file, but only the latest session.
        let _ = fs::OpenOptions::new().write(true).truncate(true).create(true).open(utils::get_log_path());

        std::panic::set_hook(Box::new(|panic_info| {
            let msg = format!("[CRITICAL PANIC] The application crashed: {}", panic_info);
            crate::utils::log_message(&msg);
        }));
    }

    info!("=======================================================");
    info!("   T_Music_Bot RPC   ");
    info!("=======================================================");

    utils::check_for_updates();

    let settings = config::load_settings();
    utils::check_lock(settings.overlay.port);

    let is_first_run = {
        let code = settings.code.as_deref().unwrap_or("");
        let has_token = settings.session_token.as_ref().map(|t| !t.is_empty()).unwrap_or(false);
        let has_user_id = settings.user_id.as_ref().map(|u| !u.is_empty()).unwrap_or(false);
        !has_user_id && code.len() != 6 && !has_token
    };
    let port = settings.overlay.port;

    let (overlay_tx, mut restart_rx) = broadcast::channel(4096);
    let (viz_tx, _) = broadcast::channel(4096);
    let (rpc_cmd_tx, rpc_cmd_rx) = std::sync::mpsc::channel();

    let state = Arc::new(RwLock::new(GlobalState {
        is_shutting_down: false,
        last_track: None,
        last_rpc_raw: None,
        settings: Some(settings.clone()),
        overlay_tx: overlay_tx.clone(),
        viz_tx: viz_tx.clone(),
        rpc_tx: Some(rpc_cmd_tx),
        ws_status: "Disconnected".to_string(),
        rpc_status: "Disconnected".to_string(),
        arrpc_detected: false,
        active_pipe: None,
        discovery_cache: None,
    }));
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    // 🚀 REACTIVE SERVER HANDLER
    let s_srv = state.clone();
    let mut srv_restart_rx = overlay_tx.subscribe();
    rt.spawn(async move {
        loop {
            let (port, allow_firewall) = {
                let s = s_srv.read().await;
                let settings = s.settings.as_ref().unwrap();
                (settings.overlay.port, settings.allow_firewall)
            };
            
            let s_inst = s_srv.clone();
            let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
            
            let mut server_task = tokio::spawn(async move {
                server::start_server(s_inst, stop_rx).await;
            });

            // Wait for restart signal or server crash
            let current_port = port;
            let current_allow_firewall = allow_firewall;
            loop {
                tokio::select! {
                    msg = srv_restart_rx.recv() => {
                        match msg {
                            Ok(m) => {
                                if m.contains("settings_update") {
                                    let (new_port, new_allow_firewall) = {
                                        let s = s_srv.read().await;
                                        let settings = s.settings.as_ref().unwrap();
                                        (settings.overlay.port, settings.allow_firewall)
                                    };
                                    if new_port != current_port || new_allow_firewall != current_allow_firewall {
                                        info!("[Server] Settings change detected (Port: {} -> {}, LAN: {} -> {}). Restarting...", current_port, new_port, current_allow_firewall, new_allow_firewall);
                                        let _ = stop_tx.send(());
                                        let _ = server_task.await;
                                        break; 
                                    }
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(_) => break,
                        }
                    }
                    _ = &mut server_task => {
                        info!("[Server] Stopped unexpectedly.");
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    if is_first_run {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = open::that(format!("http://127.0.0.1:{}/setup", port));
        });
    }

    let s2 = state.clone();
    rt.spawn(async move { rpc::start_rpc_handler(s2, rpc_cmd_rx).await; });

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
                let current_config = config.clone();
                
                // Wait for restart signal
                loop {
                    match restart_rx.recv().await {
                        Ok(msg) => {
                            if msg.contains("settings_update") {
                                let new_config = {
                                    let s = s3.read().await;
                                    s.settings.as_ref().unwrap().overlay.visualizer.clone()
                                };

                                // 🚀 SELECTIVE RESTART: Only restart if a Visualizer setting actually changed
                                if serde_json::to_string(&new_config).unwrap() != serde_json::to_string(&current_config).unwrap() {
                                    info!("[Visualizer] Visualizer settings updated. Re-initializing...");
                                    if let Some(h) = abort_handle { h.abort(); let _ = h.await; }
                                    break; 
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            } else {
                let _ = restart_rx.recv().await;
            }
        }
    });

    let shutdown_state = state.clone();
    let shutdown_tx = overlay_tx.clone();
    ctrlc::set_handler(move || {
        let s_state = shutdown_state.clone();
        let s_tx = shutdown_tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                { let mut s = s_state.write().await; s.is_shutting_down = true; }
                let _ = s_tx.send(serde_json::json!({ "type": "program_shutdown" }).to_string());
            });
            std::thread::sleep(std::time::Duration::from_millis(500));
            std::process::exit(0);
        });
    }).expect("Error setting Ctrl-C handler");

    #[cfg(target_os = "linux")]
    {
        gtk::init().unwrap();
    }

    tray::create_tray(state, rt);
}