pub mod cpal_util;

#[cfg(target_os = "linux")]
pub mod linux_pulse;
#[cfg(windows)]
pub mod windows_wasapi;
#[cfg(target_os = "macos")]
pub mod macos_input;

/// A running audio capture that can be suspended/resumed.
/// Dropping the implementation must fully stop capture and release the device.
pub trait AudioCapture: Send {
    fn pause(&mut self);
    fn resume(&mut self);
}

/// Kind of a listed device, used to group entries in the tray and dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceKind {
    /// Monitor/loopback of an output sink (desktop audio).
    Monitor,
    /// Output device captured via loopback (Windows WASAPI).
    OutputLoopback,
    /// Physical or virtual input (microphone, BlackHole, ...).
    Input,
}

impl DeviceKind {
    /// Playback devices capture desktop audio; everything else is a microphone.
    pub fn is_playback(self) -> bool {
        !matches!(self, DeviceKind::Input)
    }
}

/// One entry of the audio device list.
///
/// `id` is the stable identifier that gets persisted in `settings.json` (PulseAudio
/// source name, WASAPI endpoint id, CoreAudio device UID, ...). `name` is the
/// human friendly name reported by the OS. `label` is what the user sees in the
/// tray menu and in the dashboard, formatted Windows-style as `Name (ID)`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub label: String,
    pub kind: DeviceKind,
    pub is_default: bool,
}

impl DeviceInfo {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        kind: DeviceKind,
        is_default: bool,
    ) -> Self {
        let id = id.into();
        let name = name.into();
        let label = format_label(&name, &id);
        Self { id, name, label, kind, is_default }
    }
}

/// Windows-style display format: `Name (ID)`.
///
/// Skips the parenthetical when there is nothing to add — i.e. when the id is
/// empty, identical to the name, or already contained in the name (Windows
/// reports "Speakers (Realtek(R) Audio)" as a single friendly name).
pub fn format_label(name: &str, id: &str) -> String {
    let n = name.trim();
    let i = id.trim();
    if i.is_empty() || i.eq_ignore_ascii_case(n) || n.contains(i) {
        n.to_string()
    } else {
        format!("{} ({})", n, i)
    }
}

/// True when the stored setting means "use the system default device".
pub fn is_default_request(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || n.eq_ignore_ascii_case("default")
        || n == "@DEFAULT_MONITOR@"
        || n == "@DEFAULT_AUDIO_SINK@"
        || n == "@DEFAULT_SOURCE@"
}

/// Resolve a stored device setting against the live device list.
///
/// Accepts, in order of confidence: the exact stable id, the displayed label,
/// the friendly name, then substring matches on id/name so that settings saved
/// by older releases (friendly names only) keep working.
pub fn find_device<'a>(want: &str, devices: &'a [DeviceInfo]) -> Option<&'a DeviceInfo> {
    let w = want.trim();
    if w.is_empty() {
        return None;
    }
    if is_default_request(w) {
        return None;
    }

    let lower = w.to_lowercase();
    devices
        .iter()
        .find(|d| d.id.eq_ignore_ascii_case(w))
        .or_else(|| devices.iter().find(|d| d.label.eq_ignore_ascii_case(w)))
        .or_else(|| devices.iter().find(|d| d.name.eq_ignore_ascii_case(w)))
        .or_else(|| devices.iter().find(|d| d.id.to_lowercase().contains(&lower)))
        .or_else(|| devices.iter().find(|d| d.name.to_lowercase().contains(&lower)))
}

/// A successfully opened capture.
pub struct OpenedCapture {
    pub capture: Box<dyn AudioCapture>,
    pub sample_rate: u32,
    pub channels: u16,
    pub bound_name: String,
    /// Id of the device that was actually opened (what gets shown in logs).
    pub bound_id: String,
}

type SampleTx = std::sync::mpsc::SyncSender<Vec<f32>>;

#[cfg(target_os = "linux")]
pub fn open_capture(device_name: &str, tx: SampleTx) -> Result<OpenedCapture, String> {
    match linux_pulse::open_capture(device_name, tx.clone()) {
        Ok(o) => Ok(o),
        Err(e) => {
            crate::info!(
                "[Visualizer] PulseAudio backend unavailable ({}), falling back to ALSA/cpal input.",
                e
            );
            cpal_util::open_capture(device_name, cpal_util::DefaultPolicy::Input, None, tx)
        }
    }
}

#[cfg(target_os = "linux")]
pub fn list_devices() -> Vec<DeviceInfo> {
    match linux_pulse::list_devices() {
        Some(list) if !list.is_empty() => list,
        _ => cpal_util::list_devices(None),
    }
}

#[cfg(windows)]
pub fn open_capture(device_name: &str, tx: SampleTx) -> Result<OpenedCapture, String> {
    windows_wasapi::open_capture(device_name, tx)
}

#[cfg(windows)]
pub fn list_devices() -> Vec<DeviceInfo> {
    windows_wasapi::list_devices()
}

#[cfg(target_os = "macos")]
pub fn open_capture(device_name: &str, tx: SampleTx) -> Result<OpenedCapture, String> {
    macos_input::open_capture(device_name, tx)
}

#[cfg(target_os = "macos")]
pub fn list_devices() -> Vec<DeviceInfo> {
    macos_input::list_devices()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<DeviceInfo> {
        vec![
            DeviceInfo::new(
                "{0.0.0.00000000}.{abc-123}",
                "Speakers (Realtek(R) Audio)",
                DeviceKind::OutputLoopback,
                true,
            ),
            DeviceInfo::new(
                "alsa_input.usb-mic",
                "USB Microphone",
                DeviceKind::Input,
                false,
            ),
            DeviceInfo::new(
                "alsa_output.pci-0000.analog-stereo.monitor",
                "Built-in Audio Analog Stereo",
                DeviceKind::Monitor,
                true,
            ),
        ]
    }

    #[test]
    fn label_is_windows_style_name_and_id() {
        assert_eq!(
            format_label("Speakers (Realtek(R) Audio)", "{0.0.0.00000000}.{abc}"),
            "Speakers (Realtek(R) Audio) ({0.0.0.00000000}.{abc})"
        );
    }

    #[test]
    fn label_skips_redundant_id() {
        // Id equal to the name, or already inside it -> just the name.
        assert_eq!(format_label("HDMI", "HDMI"), "HDMI");
        assert_eq!(format_label("Speakers (Realtek)", "Realtek"), "Speakers (Realtek)");
        assert_eq!(format_label("Mic", ""), "Mic");
    }

    #[test]
    fn find_device_prefers_exact_id_then_falls_back_to_names() {
        let devices = sample();
        let by_id = find_device("{0.0.0.00000000}.{abc-123}", &devices).unwrap();
        assert_eq!(by_id.kind, DeviceKind::OutputLoopback);

        let by_label = find_device("USB Microphone (alsa_input.usb-mic)", &devices).unwrap();
        assert_eq!(by_id.id, "{0.0.0.00000000}.{abc-123}");
        assert_eq!(by_label.id, "alsa_input.usb-mic");

        // Legacy settings stored the plain friendly name.
        let legacy = find_device("Speakers (Realtek(R) Audio)", &devices).unwrap();
        assert_eq!(legacy.id, "{0.0.0.00000000}.{abc-123}");

        // And a bare substring still resolves.
        let fuzzy = find_device("usb-mic", &devices).unwrap();
        assert_eq!(fuzzy.id, "alsa_input.usb-mic");
    }

    #[test]
    fn default_requests_are_recognized() {
        for req in ["", "default", "@DEFAULT_MONITOR@", "@DEFAULT_SOURCE@"] {
            assert!(is_default_request(req), "{req:?} should mean default");
        }
        assert!(!is_default_request("alsa_output.pci.analog-stereo.monitor"));
        assert!(find_device("default", &sample()).is_none());
    }

    #[test]
    fn playback_split_matches_sound_panel() {
        let devices = sample();
        let playback: Vec<_> = devices.iter().filter(|d| d.kind.is_playback()).collect();
        let capture: Vec<_> = devices.iter().filter(|d| !d.kind.is_playback()).collect();
        assert_eq!(playback.len(), 2);
        assert_eq!(capture.len(), 1);
        assert_eq!(capture[0].kind, DeviceKind::Input);
    }
}

/// Manual diagnostic: `cargo test dump_devices -- --ignored --nocapture`
#[test]
#[ignore]
fn dump_devices() {
    let devices = list_devices();
    println!("{} device(s):", devices.len());
    for d in devices {
        println!("  [{:?}] {:?}  id={:?} default={}", d.kind, d.label, d.id, d.is_default);
    }
    let (tx, _rx) = std::sync::mpsc::sync_channel(4);
    match open_capture("default", tx) {
        Ok(o) => println!(
            "open_capture(default) ok: {} Hz, {} ch, bound={}",
            o.sample_rate, o.channels, format_label(&o.bound_name, &o.bound_id)
        ),
        Err(e) => println!("open_capture(default) failed: {}", e),
    }
}
