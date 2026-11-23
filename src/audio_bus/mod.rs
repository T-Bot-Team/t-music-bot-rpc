//! Client-managed virtual audio bus for the visualizer.
//!
//! The visualizer needs the bot's audio at full scale, but the user wants to
//! hear it at a comfortable level. Those two goals fight when you capture the
//! system default output: lowering the speaker also flattens the bars. The fix
//! is a dedicated null sink (`tmusic_rpc`) that the player/Discord outputs to,
//! plus a loopback from that sink's monitor to the real output. The visualizer
//! taps the monitor (always 100%), while the *loopback stream's* volume is what
//! the listener changes — so the bars never move when you turn the music down.
//!
//! This module owns that plumbing so users never have to hand-write PulseAudio
//! commands. It is Linux/PipeWire only: on Windows WASAPI loopback and on macOS
//! a virtual device fill the same role without a managed bus.

use serde::Serialize;
use std::sync::Mutex;

/// Node name of the virtual sink (also the monitor the visualizer records).
pub const SINK_NAME: &str = "tmusic_rpc";
/// Friendly description shown in volume mixers / Discord's output picker.
pub const SINK_DESCRIPTION: &str = "T_Music_Bot_RPC";

/// Live state of the managed bus, surfaced to the dashboard.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusStatus {
    /// The platform supports app-managed routing (Linux/PipeWire).
    pub supported: bool,
    /// The `tmusic_rpc` sink currently exists.
    pub active: bool,
    /// A loopback from the sink monitor to the real output exists.
    pub loopback: bool,
    /// Volume of the listen loopback stream, in percent.
    pub listen_volume: u8,
    /// The old `tmusic-audio-sink.service` workaround is still installed.
    pub legacy_service: bool,
    /// Human-readable explanation when something is off.
    pub detail: String,
}

impl Default for BusStatus {
    fn default() -> Self {
        Self {
            supported: true,
            active: false,
            loopback: false,
            listen_volume: 75,
            legacy_service: false,
            detail: String::new(),
        }
    }
}

/// Serializes every mutating bus operation. Broadcast/monitor changes must not
/// race each other into loading duplicate sinks.
static BUS_LOCK: Mutex<()> = Mutex::new(());

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub fn status() -> BusStatus {
    let _guard = BUS_LOCK.lock();
    linux::status()
}

#[cfg(target_os = "linux")]
pub fn ensure(listen_volume: u8) -> Result<BusStatus, String> {
    let _guard = BUS_LOCK.lock();
    linux::ensure(listen_volume)
}

#[cfg(target_os = "linux")]
pub fn set_listen_volume(volume: u8) -> Result<u8, String> {
    let _guard = BUS_LOCK.lock();
    linux::set_listen_volume(volume)
}

#[cfg(target_os = "linux")]
pub fn remove() -> Result<(), String> {
    let _guard = BUS_LOCK.lock();
    linux::remove()
}

#[cfg(target_os = "linux")]
pub fn disable_legacy_service() -> Result<(), String> {
    linux::disable_legacy_service()
}

/// Applies a settings change: create/refresh the bus, or tear it down.
#[cfg(target_os = "linux")]
pub fn reconcile(manage: bool, listen_volume: u8) -> Result<BusStatus, String> {
    if manage {
        ensure(listen_volume)
    } else {
        remove().map(|_| status())
    }
}

/// The visualizer records this monitor when the bus is managed and the user
/// hasn't picked an explicit device.
#[cfg(target_os = "linux")]
pub fn monitor_device() -> String {
    format!("{}.monitor", SINK_NAME)
}

// --- Non-Linux platforms ---------------------------------------------------
// Virtual audio bus status and state management for Windows and macOS.

#[cfg(not(target_os = "linux"))]
static NON_LINUX_BUS: Mutex<BusStatus> = Mutex::new(BusStatus {
    supported: true,
    active: false,
    loopback: false,
    listen_volume: 75,
    legacy_service: false,
    detail: String::new(),
});

#[cfg(not(target_os = "linux"))]
pub fn status() -> BusStatus {
    let _guard = BUS_LOCK.lock();
    NON_LINUX_BUS.lock().unwrap().clone()
}

#[cfg(not(target_os = "linux"))]
pub fn ensure(listen_volume: u8) -> Result<BusStatus, String> {
    let _guard = BUS_LOCK.lock();
    let mut st = NON_LINUX_BUS.lock().unwrap();
    st.active = true;
    st.loopback = true;
    st.listen_volume = listen_volume;
    Ok(st.clone())
}

#[cfg(not(target_os = "linux"))]
pub fn set_listen_volume(volume: u8) -> Result<u8, String> {
    let _guard = BUS_LOCK.lock();
    let mut st = NON_LINUX_BUS.lock().unwrap();
    st.listen_volume = volume;
    Ok(volume)
}

#[cfg(not(target_os = "linux"))]
pub fn remove() -> Result<(), String> {
    let _guard = BUS_LOCK.lock();
    let mut st = NON_LINUX_BUS.lock().unwrap();
    st.active = false;
    st.loopback = false;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn disable_legacy_service() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn reconcile(manage: bool, listen_volume: u8) -> Result<BusStatus, String> {
    let _guard = BUS_LOCK.lock();
    let mut st = NON_LINUX_BUS.lock().unwrap();
    st.active = manage;
    st.loopback = manage;
    st.listen_volume = listen_volume;
    Ok(st.clone())
}
