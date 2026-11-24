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
    audio_item: MenuItem,
    /// (checked item, stable device id, base label) for every selectable audio device.
    device_items: Vec<(CheckMenuItem, String, String)>,
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
    last_audio: String,
    last_status_poll: Instant,
    devices: Vec<crate::visualizer::backend::DeviceInfo>,
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

        // Visualizer only captures desktop audio / playback devices. Never microphones.
        let devices: Vec<crate::visualizer::backend::DeviceInfo> = crate::visualizer::backend::list_devices()
            .into_iter()
            .filter(|d| d.kind.is_playback())
            .collect();
        let (handles, current_audio) = build_menu(state, &devices);
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
            last_audio: current_audio,
            last_status_poll: Instant::now(),
            devices,
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

    /// Refreshes the WS/RPC/Audio status rows. Main thread only.
    fn update_status(&mut self, state: &AppState) {
        let (ws, rpc, audio_dev) = {
            let s = state.blocking_read();
            let dev = s
                .settings
                .as_ref()
                .map(|st| st.overlay.visualizer.audio_device.clone())
                .unwrap_or_else(|| "default".to_string());
            (s.ws_status.clone(), s.rpc_status.clone(), dev)
        };
        if ws != self.last_ws {
            self.handles.ws_item.set_text(format!("WS: {}", ws));
            self.last_ws = ws;
        }
        if rpc != self.last_rpc {
            self.handles.rpc_item.set_text(format!("RPC: {}", rpc));
            self.last_rpc = rpc;
        }
        if audio_dev != self.last_audio {
            let label = device_display_name(&audio_dev, &self.devices);
            self.handles.audio_item.set_text(format!("Audio: {}", label));
            update_device_items(&self.handles.device_items, &audio_dev, &self.devices);
            self.last_audio = audio_dev;
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
            .find(|(item, _, _)| *item.id() == *id)
            .map(|(_, dev_id, _)| dev_id.clone());

        if let Some(device_id) = selected {
            update_device_items(&self.handles.device_items, &device_id, &self.devices);

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

            let label = device_display_name(&device_id, &self.devices);
            self.handles.audio_item.set_text(format!("Audio: {}", label));
            self.last_audio = device_id;
        }
    }
}

/// Prepends a visible checkmark so the selected item is unmistakable across
/// all desktop environments (GNOME AppIndicator, KDE, Windows, macOS).
fn format_item_text(base_label: &str, is_selected: bool) -> String {
    if is_selected {
        format!("✓  {}", base_label)
    } else {
        format!("    {}", base_label)
    }
}

/// Updates both the CheckMenuItem state and the visible text checkmark.
fn update_device_items(
    items: &[(CheckMenuItem, String, String)],
    current_device: &str,
    devices: &[crate::visualizer::backend::DeviceInfo],
) {
    let want_default = crate::visualizer::backend::is_default_request(current_device);
    for (item, dev_id, base_label) in items {
        let is_selected = if dev_id == "default" {
            want_default
        } else {
            !want_default
                && (dev_id == current_device
                    || devices
                        .iter()
                        .any(|d| &d.id == dev_id && matches_current(d, current_device)))
        };
        item.set_checked(is_selected);
        item.set_text(format_item_text(base_label, is_selected));
    }
}

/// Formats a readable display name for the tray status row.
pub(crate) fn device_display_name(
    current: &str,
    devices: &[crate::visualizer::backend::DeviceInfo],
) -> String {
    if crate::visualizer::backend::is_default_request(current) {
        return "System Default".to_string();
    }
    if let Some(d) = devices.iter().find(|d| matches_current(d, current)) {
        return d.name.clone();
    }
    let trimmed = current.trim();
    if trimmed.chars().count() > 30 {
        let end = trimmed.char_indices().nth(27).map(|(i, _)| i).unwrap_or(27);
        format!("{}...", &trimmed[..end])
    } else {
        trimmed.to_string()
    }
}

/// Whether a listed device is the one selected in settings. Older versions
/// stored the plain device name, so accept that too when showing checkmarks.
fn matches_current(d: &crate::visualizer::backend::DeviceInfo, current: &str) -> bool {
    d.id.eq_ignore_ascii_case(current) || d.name == current
}

fn build_menu(
    state: &AppState,
    devices: &[crate::visualizer::backend::DeviceInfo],
) -> (MenuHandles, String) {
    let menu = Menu::new();
    let version_item = MenuItem::new(
        format!("T_Music_Bot RPC v{}", env!("CARGO_PKG_VERSION")),
        false,
        None,
    );
    let _ = menu.append(&version_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

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

    let ws_item = MenuItemBuilder::new().text("WS: Disconnected").enabled(false).build();
    let rpc_item = MenuItemBuilder::new().text("RPC: Disconnected").enabled(false).build();
    let audio_display = device_display_name(&current_device, devices);
    let audio_item = MenuItemBuilder::new()
        .text(format!("Audio: {}", audio_display))
        .enabled(false)
        .build();
    let _ = menu.append(&ws_item);
    let _ = menu.append(&rpc_item);
    let _ = menu.append(&audio_item);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let open_settings = MenuItemBuilder::new().text("Open Settings").enabled(true).build();
    let open_logs = MenuItemBuilder::new().text("Open Logs").enabled(true).build();
    let quit = MenuItemBuilder::new().text("Quit").enabled(true).build();

    let mut device_items: Vec<(CheckMenuItem, String, String)> = Vec::new();

    let default_base = "System Default".to_string();
    let default_item = CheckMenuItemBuilder::new()
        .text(format_item_text(&default_base, want_default))
        .enabled(true)
        .checked(want_default)
        .build();
    device_items.push((default_item, "default".to_string(), default_base));

    let audio_menu = SubmenuBuilder::new()
        .text("Audio Device")
        .enabled(true)
        .build()
        .expect("audio submenu");
    let _ = audio_menu.append(&device_items[0].0);

    if !devices.is_empty() {
        let _ = audio_menu.append(&PredefinedMenuItem::separator());
        for d in devices {
            let is_selected = !want_default && matches_current(d, &current_device);
            let base_label = d.label.clone();
            let item = CheckMenuItemBuilder::new()
                .text(format_item_text(&base_label, is_selected))
                .enabled(true)
                .checked(is_selected)
                .build();
            let _ = audio_menu.append(&item);
            device_items.push((item, d.id.clone(), base_label));
        }
    }
    let _ = menu.append(&audio_menu);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&open_settings);
    let _ = menu.append(&open_logs);
    let _ = menu.append(&quit);

    (
        MenuHandles {
            ws_item,
            rpc_item,
            audio_item,
            menu,
            device_items,
            quit_id: quit.id().clone(),
            logs_id: open_logs.id().clone(),
            settings_id: open_settings.id().clone(),
        },
        current_device,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualizer::backend::{DeviceInfo, DeviceKind};

    #[test]
    fn test_device_display_name() {
        let devices = vec![
            DeviceInfo::new(
                "{0.0.0.00000000}.{123}",
                "Speakers (Realtek Audio)",
                DeviceKind::OutputLoopback,
                true,
            ),
            DeviceInfo::new(
                "alsa_input.mic",
                "USB Microphone",
                DeviceKind::Input,
                false,
            ),
        ];

        assert_eq!(device_display_name("default", &devices), "System Default");
        assert_eq!(device_display_name("", &devices), "System Default");
        assert_eq!(
            device_display_name("{0.0.0.00000000}.{123}", &devices),
            "Speakers (Realtek Audio)"
        );
        assert_eq!(
            device_display_name("alsa_input.mic", &devices),
            "USB Microphone"
        );
        assert_eq!(
            device_display_name("Speakers (Realtek Audio)", &devices),
            "Speakers (Realtek Audio)"
        );
    }
}
