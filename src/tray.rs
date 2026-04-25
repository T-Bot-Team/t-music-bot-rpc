use std::sync::{Arc};
use std::time::{Duration, Instant};
use tray_icon::{
    menu::{Menu, MenuItem, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder, MenuEvent, CheckMenuItemBuilder, CheckMenuItem},
    TrayIconBuilder,
};
use crate::{AppState, info};
use tokio::runtime::Runtime;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct TrayHandle {
    pub ws_item: MenuItem,
    pub rpc_item: MenuItem,
}

static mut GLOBAL_TRAY_HANDLE: Option<TrayHandle> = None;
static LAST_UPDATE_MILLIS: AtomicU64 = AtomicU64::new(0);

pub fn create_tray(state: AppState, rt: Runtime) {
    let rt_arc = Arc::new(rt);
    let (menu, device_items, quit_id, logs_id, settings_id) = build_menu(&state, &rt_arc);

    let icon_bytes = include_bytes!("../assets/icon.ico");
    let icon = tray_icon::Icon::from_resource(101, None)
        .unwrap_or_else(|_| {
            let img = image::load_from_memory(icon_bytes).unwrap().to_rgba8();
            let (width, height) = img.dimensions();
            tray_icon::Icon::from_rgba(img.into_raw(), width, height).unwrap()
        });

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("T_Music_Bot RPC")
        .with_icon(icon)
        .build()
        .unwrap();

    // STATUS UPDATE THREAD
    let loop_state = state.clone();
    let loop_rt = rt_arc.clone();
    std::thread::spawn(move || {
        let start_time = Instant::now();
        loop {
            update_tray_status(loop_state.clone(), &loop_rt, start_time);
            std::thread::sleep(Duration::from_millis(1000));
        }
    });

    // EVENT HANDLING LOOP (Windows Message Loop)
    let event_receiver = MenuEvent::receiver();
    let loop_state_2 = state.clone();

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, DispatchMessageW, TranslateMessage, MSG};
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        unsafe {
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
                
                if let Ok(event) = event_receiver.try_recv() {
                    if event.id == quit_id {
                        let _ = loop_state_2.blocking_read().overlay_tx.send(serde_json::json!({ "type": "program_shutdown" }).to_string());
                        loop_state_2.blocking_write().is_shutting_down = true;
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        std::process::exit(0);
                    }
                    if event.id == logs_id { let _ = open::that(crate::utils::get_log_path()); }
                    if event.id == settings_id {
                        let port = loop_state_2.blocking_read().settings.as_ref().unwrap().overlay.port;
                        let _ = open::that(format!("http://localhost:{}/settings", port));
                    }
                    
                    // 🚀 AUDIO DEVICE SELECTION
                    for (item, name) in &device_items {
                        if event.id == item.id() {
                            for (other_it, _) in &device_items {
                                if other_it.id() != item.id() { other_it.set_checked(false); }
                            }
                            item.set_checked(true);

                            let mut s = loop_state_2.blocking_write();
                            if let Some(st) = s.settings.as_mut() {
                                st.overlay.visualizer.audio_device = name.clone();
                                crate::utils::save_settings(st);
                                info!("[Tray] Audio device updated: {}", name);
                                
                                // 🚨 CRITICAL: Trigger visualizer restart!
                                let _ = s.overlay_tx.send(serde_json::json!({ "type": "settings_update" }).to_string());
                            }
                        }
                    }

                    // LAYOUT SELECTION REMOVED FROM TRAY
                }
            }
        }
    }
}

fn build_menu(state: &AppState, rt: &Arc<Runtime>) -> (Menu, Vec<(CheckMenuItem, String)>, tray_icon::menu::MenuId, tray_icon::menu::MenuId, tray_icon::menu::MenuId) {
    let menu = Menu::new();
    let version_string = format!("T_Music_Bot RPC v{}", env!("CARGO_PKG_VERSION"));
    let version_item = MenuItem::new(version_string, false, None);
    let _ = menu.append(&version_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    
    let ws_item = MenuItemBuilder::new().text("WS: Disconnected").enabled(false).build();
    let rpc_item = MenuItemBuilder::new().text("RPC: Disconnected").enabled(false).build();
    let open_settings = MenuItemBuilder::new().text("Open Settings").enabled(true).build();
    let open_logs = MenuItemBuilder::new().text("Open Logs").enabled(true).build();
    let quit = MenuItemBuilder::new().text("Quit").enabled(true).build();

    let settings = rt.block_on(async { state.read().await.settings.as_ref().unwrap().clone() });
    let current_device = settings.overlay.visualizer.audio_device;

    // AUDIO DEVICE SUBMENU
    let devices = crate::utils::get_audio_devices();
    let devices_menu = SubmenuBuilder::new().text("Playback Device").enabled(true);
    let mut device_items = Vec::new();
    for d in devices {
        let is_selected = d == current_device || (current_device == "default" && d.to_lowercase().contains("default"));
        let item = CheckMenuItemBuilder::new().text(&d).enabled(true).checked(is_selected).build();
        device_items.push((item, d));
    }
    let dm = devices_menu.build().unwrap();
    for (item, _) in &device_items { let _ = dm.append(item); }

    let _ = menu.append_items(&[
        &ws_item, &rpc_item,
        &PredefinedMenuItem::separator(),
        &dm,
        &PredefinedMenuItem::separator(),
        &open_settings, &open_logs, &quit
    ]);

    unsafe { GLOBAL_TRAY_HANDLE = Some(TrayHandle { ws_item, rpc_item }); }
    (menu, device_items, quit.id().clone(), open_logs.id().clone(), open_settings.id().clone())
}

fn update_tray_status(state: AppState, rt: &Runtime, start_time: Instant) {
    let now_millis = Instant::now().duration_since(start_time).as_millis() as u64;
    let last = LAST_UPDATE_MILLIS.load(Ordering::Relaxed);
    if now_millis - last < 1000 { return; }
    LAST_UPDATE_MILLIS.store(now_millis, Ordering::Relaxed);

    let (ws, rpc) = rt.block_on(async {
        let s = state.read().await;
        (s.ws_status.clone(), s.rpc_status.clone())
    });

    unsafe {
        if let Some(h) = (&raw const GLOBAL_TRAY_HANDLE).as_ref().unwrap() {
            let _ = h.ws_item.set_text(format!("WS: {}", ws));
            let _ = h.rpc_item.set_text(format!("RPC: {}", rpc));
        }
    }
}
