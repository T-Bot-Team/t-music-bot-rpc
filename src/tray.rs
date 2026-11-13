use std::time::{Duration, Instant};
use tray_icon::menu::{
    CheckMenuItem, CheckMenuItemBuilder, Menu, MenuEvent, MenuItem, MenuItemBuilder,
    PredefinedMenuItem, SubmenuBuilder,
};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::AppState;

/// Everything `build_menu` produced, kept as handles into the live menu.
struct MenuHandles {
    menu: Menu,
    ws_item: MenuItem,
    rpc_item: MenuItem,
    /// (checked item, stable device id) for every selectable audio device.
    device_items: Vec<(CheckMenuItem, String)>,
    quit_id: tray_icon::menu::MenuId,
    logs_id: tray_icon::menu::MenuId,
    settings_id: tray_icon::menu::MenuId,
}

/// Tray icon + menus.
///
/// The tray never owns the process main loop: the caller (see `ui::run`) must
/// call [`Tray::pump`] regularly from the *main* thread. That keeps every
/// AppKit/GTK/Win32 menu mutation on the thread that owns those toolkits —
/// touching the tray from a background thread is what crashed macOS with
/// `Assertion failed: (CGAtomicGet(&is_initialized)), CGSConnectionByID`.
pub struct Tray {
    _icon: TrayIcon,
    events: crossbeam_channel::Receiver<MenuEvent>,
    handles: MenuHandles,
    last_ws: String,
    last_rpc: String,
    last_status_poll: Instant,
}

/// Initializes AppKit before any `NSStatusItem` is created.
///
/// Creating the status item before `NSApplication::sharedApplication()` leaves
/// the Core Graphics session connection uninitialized, which aborts the process
/// on macOS 14/15 (`CGSConnectionByID`, line 422).
///
/// `own_launch` is only true when we build the tray *without* winit: there
/// `-[NSApplication run]` never runs, so we have to finish launching ourselves.
/// Under winit the run loop already launched the app before `Resumed` fires,
/// and calling `finishLaunching` again would re-post the launch notification
/// into winit's delegate.
#[cfg(target_os = "macos")]
fn ensure_appkit(own_launch: bool) {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    use objc2_foundation::MainThreadMarker;

    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        // Tray-only process: no Dock icon, no app menu takeover.
        let _ = app.setActivationPolicy(NSApplicationActivationPolicyAccessory);
        if own_launch {
            app.finishLaunching();
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn ensure_appkit(_own_launch: bool) {}

/// 1x1 transparent fallback icon when the bundled one cannot be decoded.
fn empty_icon() -> tray_icon::Icon {
    tray_icon::Icon::from_rgba(vec![0, 0, 0, 0], 1, 1).expect("1x1 fallback icon")
}

fn load_icon() -> tray_icon::Icon {
    #[cfg(windows)]
    if let Ok(icon) = tray_icon::Icon::from_resource(101, None) {
        return icon;
    }

    image::load_from_memory(include_bytes!("../assets/icon.ico"))
        .ok()
        .and_then(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = (rgba.width(), rgba.height());
            tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).ok()
        })
        .unwrap_or_else(empty_icon)
}

impl Tray {
    /// Builds the tray icon and menus without winit driving NSApplication
    /// (fallback pump). Must run on the main thread.
    pub fn create(state: &AppState) -> Result<Tray, String> {
        Self::create_inner(state, true)
    }

    /// Builds the tray from inside winit's `Resumed` callback: NSApplication
    /// is already running, so we must not finish-launch it again.
    pub fn create_in_run_loop(state: &AppState) -> Result<Tray, String> {
        Self::create_inner(state, false)
    }

    fn create_inner(state: &AppState, own_launch: bool) -> Result<Tray, String> {
        ensure_appkit(own_launch);

        let handles = build_menu(state);
        let _icon = TrayIconBuilder::new()
            .with_menu(Box::new(handles.menu.clone()))
            .with_tooltip("T_Music_Bot RPC")
            .with_icon(load_icon())
            .build()
            .map_err(|e| format!("failed to create tray icon: {}", e))?;

        Ok(Tray {
            _icon,
            events: MenuEvent::receiver().clone(),
            handles,
            last_ws: String::new(),
            last_rpc: String::new(),
            last_status_poll: Instant::now(),
        })
    }

    /// Drains pending menu events, pumps GTK where needed, and refreshes the
    /// status rows. Call from the main thread whenever the event loop wakes up.
    pub fn pump(&mut self, state: &AppState) {
        self.pump_platform();

        while let Ok(event) = self.events.try_recv() {
            self.handle_event(&event.id, state);
        }

        if self.last_status_poll.elapsed() >= Duration::from_millis(500) {
            self.last_status_poll = Instant::now();
            self.update_status(state);
        }
    }

    /// Refreshes the WS/RPC status rows. Main thread only.
    fn update_status(&mut self, state: &AppState) {
        let (ws, rpc) = {
            let s = state.blocking_read();
            (s.ws_status.clone(), s.rpc_status.clone())
        };
        if ws != self.last_ws {
            self.handles.ws_item.set_text(format!("WS: {}", ws));
            self.last_ws = ws;
        }
        if rpc != self.last_rpc {
            self.handles.rpc_item.set_text(format!("RPC: {}", rpc));
            self.last_rpc = rpc;
        }
    }

    #[cfg(target_os = "linux")]
    fn pump_platform(&self) {
        // GTK menus only dispatch from the GTK main loop; pump it without blocking.
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn pump_platform(&self) {}

    fn handle_event(&mut self, id: &tray_icon::menu::MenuId, state: &AppState) {
        if *id == self.handles.quit_id {
            let _ = state.blocking_read().overlay_tx.send(
                serde_json::json!({ "type": "program_shutdown" }).to_string(),
            );
            state.blocking_write().is_shutting_down = true;
            std::thread::sleep(Duration::from_millis(500));
            std::process::exit(0);
        }

        if *id == self.handles.logs_id {
            let _ = open::that(crate::utils::get_log_path());
            return;
        }

        if *id == self.handles.settings_id {
            let port = state
                .blocking_read()
                .settings
                .as_ref()
                .map(|s| s.overlay.port)
                .unwrap_or(3000);
            let _ = open::that(format!("http://localhost:{}/settings", port));
            return;
        }

        // Audio device selection: the stored value is the stable device id.
        let selected = self
            .handles
            .device_items
            .iter()
            .find(|(item, _)| *item.id() == *id)
            .map(|(_, dev_id)| dev_id.clone());

        if let Some(device_id) = selected {
            for (item, dev_id) in &self.handles.device_items {
                item.set_checked(*dev_id == device_id);
            }

            let mut s = state.blocking_write();
            if let Some(st) = s.settings.as_mut() {
                st.overlay.visualizer.audio_device = device_id.clone();
                crate::config::save_settings(st);
                info!("[Tray] Audio device updated: {}", device_id);
                // Restart the visualizer with the new device.
                let _ = s.overlay_tx.send(
                    serde_json::json!({ "type": "settings_update" }).to_string(),
                );
            }
        }
    }
}

/// Whether a listed device is the one selected in settings. Older versions
/// stored the plain device name, so accept that too when showing checkmarks.
fn matches_current(d: &crate::visualizer::backend::DeviceInfo, current: &str) -> bool {
    d.id.eq_ignore_ascii_case(current) || d.name == current
}

fn build_menu(state: &AppState) -> MenuHandles {
    let menu = Menu::new();
    let version_item = MenuItem::new(
        format!("T_Music_Bot RPC v{}", env!("CARGO_PKG_VERSION")),
        false,
        None,
    );
    let _ = menu.append(&version_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let ws_item = MenuItemBuilder::new().text("WS: Disconnected").enabled(false).build();
    let rpc_item = MenuItemBuilder::new().text("RPC: Disconnected").enabled(false).build();
    let _ = menu.append(&ws_item);
    let _ = menu.append(&rpc_item);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let open_settings = MenuItemBuilder::new().text("Open Settings").enabled(true).build();
    let open_logs = MenuItemBuilder::new().text("Open Logs").enabled(true).build();
    let quit = MenuItemBuilder::new().text("Quit").enabled(true).build();

    let (current_device, want_default) = {
        let s = state.blocking_read();
        let dev = s
            .settings
            .as_ref()
            .map(|st| st.overlay.visualizer.audio_device.clone())
            .unwrap_or_else(|| "default".to_string());
        let def = crate::visualizer::backend::is_default_request(&dev);
        (dev, def)
    };

    let devices = crate::visualizer::backend::list_devices();
    let playback: Vec<&crate::visualizer::backend::DeviceInfo> =
        devices.iter().filter(|d| d.kind.is_playback()).collect();
    let capture: Vec<&crate::visualizer::backend::DeviceInfo> =
        devices.iter().filter(|d| !d.kind.is_playback()).collect();

    // Same split as the Windows Sound control panel: playback (desktop audio)
    // and capture (microphones) are separate menus, labels are "Name (ID)".
    let mut device_items: Vec<(CheckMenuItem, String)> = Vec::new();

    let default_item = CheckMenuItemBuilder::new()
        .text("System Default (Desktop Audio)")
        .enabled(true)
        .checked(want_default)
        .build();
    device_items.push((default_item, "default".to_string()));

    let playback_menu = SubmenuBuilder::new().text("Playback Device").enabled(true)
        .build()
        .expect("playback submenu");
    let _ = playback_menu.append(&device_items[0].0);
    for d in &playback {
        let item = CheckMenuItemBuilder::new()
            .text(&d.label)
            .enabled(true)
            .checked(!want_default && matches_current(d, &current_device))
            .build();
        let _ = playback_menu.append(&item);
        device_items.push((item, d.id.clone()));
    }
    let _ = menu.append(&playback_menu);

    if !capture.is_empty() {
        let capture_menu = SubmenuBuilder::new().text("Capture Device").enabled(true)
            .build()
            .expect("capture submenu");
        for d in &capture {
            let item = CheckMenuItemBuilder::new()
                .text(&d.label)
                .enabled(true)
                .checked(!want_default && matches_current(d, &current_device))
                .build();
            let _ = capture_menu.append(&item);
            device_items.push((item, d.id.clone()));
        }
        let _ = menu.append(&capture_menu);
    }

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&open_settings);
    let _ = menu.append(&open_logs);
    let _ = menu.append(&quit);

    MenuHandles {
        ws_item,
        rpc_item,
        menu,
        device_items,
        quit_id: quit.id().clone(),
        logs_id: open_logs.id().clone(),
        settings_id: open_settings.id().clone(),
    }
}
