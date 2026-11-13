//! Process main loop: tray pumping.
//!
//! The tray must live on the main thread on every platform:
//! * macOS — winit runs `NSApplication`'s run loop (AppKit objects are only
//!   ever touched there), the tray is created after `applicationDidFinishLaunching`.
//! * Windows — winit's message pump dispatches tray-icon/muda messages.
//! * Linux — GTK is pumped manually between winit wakeups.
//!
//! If no graphical session is available (headless systemd service, SSH, ...)
//! we fall back to a plain pump loop so the server still runs.

use crate::tray::Tray;
use crate::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

/// Cleared on Linux when GTK cannot find a display.
static GUI_AVAILABLE: AtomicBool = AtomicBool::new(true);

pub fn set_gui_available(available: bool) {
    GUI_AVAILABLE.store(available, Ordering::Relaxed);
}

/// Blocks until the process exits. Never returns on a working desktop session.
pub fn run(state: AppState) {
    if !GUI_AVAILABLE.load(Ordering::Relaxed) {
        info!("[UI] No graphical session available; running without tray.");
        pump_forever(&state);
        return;
    }

    let mut builder = EventLoop::builder();
    // Tray-only app: no Dock icon, no task switcher entry.
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        builder.with_activation_policy(ActivationPolicy::Accessory);
    }

    match builder.build() {
        Ok(event_loop) => {
            let mut app = Ui {
                state: state.clone(),
                tray: None,
            };
            if let Err(e) = event_loop.run_app(&mut app) {
                info!("[UI] Event loop failed: {}; falling back to a plain pump.", e);
            } else {
                return;
            }
        }
        Err(e) => {
            info!("[UI] No event loop ({}); falling back to a plain pump.", e);
        }
    }

    pump_forever(&state);
}

struct Ui {
    state: AppState,
    tray: Option<Tray>,
}

impl ApplicationHandler for Ui {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // `resumed` fires after NSApplication finished launching on macOS,
        // which is the earliest safe point to create the status item.
        if self.tray.is_none() {
            match Tray::create_in_run_loop(&self.state) {
                Ok(tray) => self.tray = Some(tray),
                Err(e) => info!("[UI] Tray unavailable: {}", e),
            }
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: winit::window::WindowId, _event: winit::event::WindowEvent) {
        // No windows of our own; the tray gets its events from the platform pump.
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(tray) = &mut self.tray {
            tray.pump(&self.state);
        }
        // Wake periodically so tray status text stays fresh.
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(500),
        ));
    }
}

/// Headless / no-event-loop fallback: keep the process alive, pump what we can.
fn pump_forever(state: &AppState) {
    let mut tray = if GUI_AVAILABLE.load(Ordering::Relaxed) {
        Tray::create(state).ok()
    } else {
        None
    };

    #[cfg(windows)]
    {
        // Tray menu messages need a Win32 message pump.
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
        };
        loop {
            unsafe {
                let mut msg = std::mem::zeroed::<MSG>();
                while PeekMessageW(&mut msg, 0, 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if let Some(tray) = &mut tray {
                tray.pump(state);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[cfg(not(windows))]
    loop {
        if let Some(tray) = &mut tray {
            tray.pump(state);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
