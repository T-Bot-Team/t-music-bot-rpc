use std::sync::{Arc};
use std::time::{Duration};
use tray_icon::{
    menu::{Menu, MenuItem, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder, MenuEvent, CheckMenuItemBuilder, CheckMenuItem},
    TrayIconBuilder,
};
use crate::AppState;
use tokio::runtime::Runtime;

pub struct TrayHandle {
    pub ws_item: MenuItem,
    pub rpc_item: MenuItem,
}

static mut GLOBAL_TRAY_HANDLE: Option<TrayHandle> = None;

pub fn create_tray(state: AppState, rt: Runtime) {
    let rt_arc = Arc::new(rt);
    let (menu, device_items, quit_id, logs_id, settings_id) = build_menu(&state, &rt_arc);

    let icon_bytes = include_bytes!("../assets/icon.ico");
    #[cfg(windows)]
    let icon = tray_icon::Icon::from_resource(101, None)
        .unwrap_or_else(|_| {
            let img = image::load_from_memory(icon_bytes).unwrap().to_rgba8();
            let (width, height) = img.dimensions();
            tray_icon::Icon::from_rgba(img.into_raw(), width, height).unwrap()
        });
    #[cfg(not(windows))]
    let icon = {
        let img = image::load_from_memory(icon_bytes).unwrap().to_rgba8();
        let (width, height) = img.dimensions();
        tray_icon::Icon::from_rgba(img.into_raw(), width, height).unwrap()
    };

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
        let mut last_ws = String::new();
        let mut last_rpc = String::new();
        loop {
            let (ws, rpc) = loop_rt.block_on(async {
                let s = loop_state.read().await;
                (s.ws_status.clone(), s.rpc_status.clone())
            });

            if ws != last_ws || rpc != last_rpc {
                update_tray_status_direct(ws.clone(), rpc.clone());
                last_ws = ws;
                last_rpc = rpc;
            }

            std::thread::sleep(Duration::from_millis(2000));
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
                                crate::config::save_settings(st);
                                info!("[Tray] Audio device updated: {}", name);
                                
                                // 🚨 CRITICAL: Trigger visualizer restart!
                                let _ = s.overlay_tx.send(serde_json::json!({ "type": "settings_update" }).to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let event_receiver = event_receiver.clone();
        let loop_state_2 = loop_state_2.clone();
        let device_items = device_items.clone();

        gtk::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(event) = event_receiver.try_recv() {
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
                
                // AUDIO DEVICE SELECTION
                for (item, name) in &device_items {
                    if event.id == item.id() {
                        for (other_it, _) in &device_items {
                            if other_it.id() != item.id() { other_it.set_checked(false); }
                        }
                        item.set_checked(true);

                        let mut s = loop_state_2.blocking_write();
                        if let Some(st) = s.settings.as_mut() {
                            st.overlay.visualizer.audio_device = name.clone();
                            crate::config::save_settings(st);
                            info!("[Tray] Audio device updated: {}", name);
                            
                            // Trigger visualizer restart!
                            let _ = s.overlay_tx.send(serde_json::json!({ "type": "settings_update" }).to_string());
                        }
                    }
                }
            }
            gtk::glib::ControlFlow::Continue
        });

        gtk::main();
    }

    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSApplication;
        use objc2_foundation::MainThreadMarker;

        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);

            struct PollContext {
                event_receiver: crossbeam_channel::Receiver<tray_icon::menu::MenuEvent>,
                loop_state_2: crate::AppState,
                device_items: Vec<(tray_icon::menu::CheckMenuItem, String)>,
                quit_id: tray_icon::menu::MenuId,
                logs_id: tray_icon::menu::MenuId,
                settings_id: tray_icon::menu::MenuId,
            }

            unsafe fn get_main_queue() -> *mut std::ffi::c_void {
                extern "C" {
                    static _dispatch_main_q: std::ffi::c_void;
                }
                &_dispatch_main_q as *const std::ffi::c_void as *mut std::ffi::c_void
            }

            extern "C" fn poll_loop(ctx_ptr: *mut std::ffi::c_void) {
                unsafe {
                    let ctx = &mut *(ctx_ptr as *mut PollContext);

                    while let Ok(event) = ctx.event_receiver.try_recv() {
                        if event.id == ctx.quit_id {
                            let _ = ctx.loop_state_2.blocking_read().overlay_tx.send(serde_json::json!({ "type": "program_shutdown" }).to_string());
                            ctx.loop_state_2.blocking_write().is_shutting_down = true;
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            std::process::exit(0);
                        }
                        if event.id == ctx.logs_id { let _ = open::that(crate::utils::get_log_path()); }
                        if event.id == ctx.settings_id {
                            let port = ctx.loop_state_2.blocking_read().settings.as_ref().unwrap().overlay.port;
                            let _ = open::that(format!("http://localhost:{}/settings", port));
                        }
                        
                        // AUDIO DEVICE SELECTION
                        for (item, name) in &ctx.device_items {
                            if event.id == item.id() {
                                for (other_it, _) in &ctx.device_items {
                                    if other_it.id() != item.id() { other_it.set_checked(false); }
                                }
                                item.set_checked(true);

                                let mut s = ctx.loop_state_2.blocking_write();
                                if let Some(st) = s.settings.as_mut() {
                                    st.overlay.visualizer.audio_device = name.clone();
                                    crate::config::save_settings(st);
                                    info!("[Tray] Audio device updated: {}", name);
                                    
                                    // Trigger visualizer restart!
                                    let _ = s.overlay_tx.send(serde_json::json!({ "type": "settings_update" }).to_string());
                                }
                            }
                        }
                    }

                    extern "C" {
                        fn dispatch_time(when: u64, delta: i64) -> u64;
                        fn dispatch_after_f(
                            when: u64,
                            queue: *mut std::ffi::c_void,
                            context: *mut std::ffi::c_void,
                            work: extern "C" fn(*mut std::ffi::c_void),
                        );
                    }

                    let delay_ns = 50 * 1_000_000; // 50ms
                    let when = dispatch_time(0, delay_ns);
                    let queue = get_main_queue();
                    dispatch_after_f(when, queue, ctx_ptr, poll_loop);
                }
            }

            let ctx = Box::new(PollContext {
                event_receiver: event_receiver.clone(),
                loop_state_2: loop_state_2.clone(),
                device_items: device_items.clone(),
                quit_id: quit_id.clone(),
                logs_id: logs_id.clone(),
                settings_id: settings_id.clone(),
            });

            unsafe {
                extern "C" {
                    fn dispatch_async_f(
                        queue: *mut std::ffi::c_void,
                        context: *mut std::ffi::c_void,
                        work: extern "C" fn(*mut std::ffi::c_void),
                    );
                }
                let queue = get_main_queue();
                dispatch_async_f(queue, Box::into_raw(ctx) as *mut std::ffi::c_void, poll_loop);

                app.run();
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

    // AUDIO DEVICE SELECTION
    let devices = crate::utils::get_audio_devices();
    let mut device_items = Vec::new();
    for d in devices {
        let is_selected = d == current_device || (current_device == "default" && d.to_lowercase().contains("default"));
        let item = CheckMenuItemBuilder::new().text(&d).enabled(true).checked(is_selected).build();
        device_items.push((item, d));
    }

    let _ = menu.append(&ws_item);
    let _ = menu.append(&rpc_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let devices_menu = SubmenuBuilder::new().text("Playback Device").enabled(true);
    let dm = devices_menu.build().unwrap();
    for (item, _) in &device_items {
        let _ = dm.append(item);
    }
    let _ = menu.append(&dm);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&open_settings);
    let _ = menu.append(&open_logs);
    let _ = menu.append(&quit);

    unsafe { GLOBAL_TRAY_HANDLE = Some(TrayHandle { ws_item, rpc_item }); }
    (menu, device_items, quit.id().clone(), open_logs.id().clone(), open_settings.id().clone())
}

fn update_tray_status_direct(ws: String, rpc: String) {
    unsafe {
        if let Some(h) = (&raw const GLOBAL_TRAY_HANDLE).as_ref().unwrap() {
            let _ = h.ws_item.set_text(format!("WS: {}", ws));
            let _ = h.rpc_item.set_text(format!("RPC: {}", rpc));
        }
    }
}