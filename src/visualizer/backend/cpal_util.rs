use super::{AudioCapture, DeviceInfo, DeviceKind, OpenedCapture};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::collections::HashMap;

type SampleTx = std::sync::mpsc::SyncSender<Vec<f32>>;

enum Cmd {
    Pause,
    Resume,
    Quit,
}

/// Which device `"default"` resolves to, per platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultPolicy {
    /// Default *output* device captured through WASAPI loopback (desktop audio).
    OutputLoopback,
    /// Prefer one of the well-known virtual loopback inputs (BlackHole & friends)
    /// so desktop audio is captured on platforms without a loopback API;
    /// falls back to the default microphone.
    VirtualInput(&'static [&'static str]),
    /// Default input device (microphone).
    Input,
}

/// Platform-provided stable ids + default flags, keyed by (kind, OS device name).
///
/// Built by the Windows (WASAPI endpoint ids) and macOS (CoreAudio device UIDs)
/// backends; `None` falls back to using the OS-reported name as the id.
#[derive(Default)]
pub struct IdLookup {
    entries: HashMap<(u8, String), (String, bool)>,
}

fn kind_key(kind: DeviceKind) -> u8 {
    match kind {
        DeviceKind::Monitor => 0,
        DeviceKind::OutputLoopback => 1,
        DeviceKind::Input => 2,
    }
}

impl IdLookup {
    pub fn insert(&mut self, kind: DeviceKind, os_name: &str, id: &str, is_default: bool) {
        let key = (kind_key(kind), os_name.to_string());
        self.entries.insert(key, (id.to_string(), is_default));
    }

    /// Stable id + default flag for an OS-reported device name.
    pub fn get(&self, kind: DeviceKind, os_name: &str) -> (String, bool) {
        match self.entries.get(&(kind_key(kind), os_name.to_string())) {
            Some((id, is_default)) => (id.clone(), *is_default),
            None => (os_name.to_string(), false),
        }
    }

    /// Reverse lookup: stored id -> (OS device name, kind).
    pub fn name_for_id(&self, id: &str) -> Option<(String, DeviceKind)> {
        self.entries
            .iter()
            .find(|(_, (stored, _))| stored.eq_ignore_ascii_case(id))
            .map(|((k, name), _)| (name.clone(), un_key(*k)))
    }
}

fn un_key(k: u8) -> DeviceKind {
    match k {
        0 => DeviceKind::Monitor,
        1 => DeviceKind::OutputLoopback,
        _ => DeviceKind::Input,
    }
}

/// Handle to a capture stream living on a dedicated worker thread.
///
/// cpal streams are not `Send` on all platforms, so the stream is created and
/// owned entirely by the worker; this handle only ships commands to it.
pub struct CpalCapture {
    cmd_tx: std::sync::mpsc::Sender<Cmd>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl AudioCapture for CpalCapture {
    fn pause(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Pause);
    }
    fn resume(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Resume);
    }
}

impl Drop for CpalCapture {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(Cmd::Quit);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

/// Opens a capture stream on `device`, running it on a worker thread.
///
/// On Windows, `is_loopback = true` captures the output device via WASAPI
/// loopback (cpal allows building an input stream on an output device).
/// On other platforms only real input devices can be captured.
pub fn open_stream(
    device: Device,
    is_loopback: bool,
    tx: SampleTx,
) -> Result<(CpalCapture, u32, u16), String> {
    let supported = if is_loopback {
        device.default_output_config()
    } else {
        device.default_input_config()
    }
    .map_err(|e| format!("failed to query device config: {}", e))?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let (sr, ch) = (config.sample_rate.0, config.channels);

    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Cmd>();

    let worker = std::thread::Builder::new()
        .name("cpal-capture".into())
        .spawn(move || {
            let err_fn = |err| crate::info!("[Visualizer] Stream error: {}", err);

            macro_rules! build {
                ($ty:ty, $conv:expr) => {
                    device.build_input_stream(
                        &config,
                        move |data: &[$ty], _| {
                            let vec: Vec<f32> = data.iter().map($conv).collect();
                            let _ = tx.try_send(vec);
                        },
                        err_fn,
                        None,
                    )
                };
            }

            let stream_result: Result<Stream, cpal::BuildStreamError> = match sample_format {
                SampleFormat::F32 => build!(f32, |&s| s),
                SampleFormat::I16 => build!(i16, |&s| s as f32 / i16::MAX as f32),
                SampleFormat::U16 => build!(u16, |&s| (s as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)),
                SampleFormat::I8 => build!(i8, |&s| s as f32 / i8::MAX as f32),
                SampleFormat::I32 => build!(i32, |&s| s as f32 / i32::MAX as f32),
                SampleFormat::I64 => build!(i64, |&s| s as f32 / i64::MAX as f32),
                SampleFormat::U8 => build!(u8, |&s| (s as f32 - 128.0) / 128.0),
                SampleFormat::U32 => build!(u32, |&s| (s as f32 - u32::MAX as f32 / 2.0) / (u32::MAX as f32 / 2.0)),
                SampleFormat::U64 => build!(u64, |&s| (s as f64 - u64::MAX as f64 / 2.0) as f32 / (u64::MAX as f64 / 2.0) as f32),
                SampleFormat::F64 => build!(f64, |&s| s as f32),
                other => {
                    let _ = init_tx.send(Err(format!("unsupported sample format: {:?}", other)));
                    return;
                }
            };

            let stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    let _ = init_tx.send(Err(format!("failed to open capture stream: {}", e)));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = init_tx.send(Err(format!("failed to start capture stream: {}", e)));
                return;
            }
            let _ = init_tx.send(Ok(()));

            // Command loop; dropping `stream` on exit releases the device.
            loop {
                match cmd_rx.recv() {
                    Ok(Cmd::Pause) => {
                        if let Err(e) = stream.pause() {
                            crate::info!("[Visualizer] Failed to pause stream: {}", e);
                        }
                    }
                    Ok(Cmd::Resume) => {
                        if let Err(e) = stream.play() {
                            crate::info!("[Visualizer] Failed to resume stream: {}", e);
                        }
                    }
                    Ok(Cmd::Quit) | Err(_) => break,
                }
            }
        })
        .map_err(|e| format!("failed to spawn capture worker: {}", e))?;

    match init_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("timed out opening capture stream".into()),
    }

    Ok((CpalCapture { cmd_tx, worker: Some(worker) }, sr, ch))
}

/// Every device cpal can see, paired with its display info.
///
/// `allow_loopback` lists output devices as capturable (Windows only — on
/// macOS/Linux an output device cannot be opened as an input stream, so
/// advertising them would produce devices that fail to open).
fn collect_devices(host: &cpal::Host, allow_loopback: bool, ids: Option<&IdLookup>) -> Vec<(Device, DeviceInfo)> {
    let mut out: Vec<(Device, DeviceInfo)> = Vec::new();
    let mut seen: std::collections::HashSet<(u8, String)> = std::collections::HashSet::new();

    if allow_loopback {
        if let Ok(devices) = host.output_devices() {
            for d in devices {
                let Ok(name) = d.name() else { continue };
                let kind = DeviceKind::OutputLoopback;
                if !seen.insert((kind_key(kind), name.clone())) {
                    continue;
                }
                let (id, is_default) = ids
                    .map(|l| l.get(kind, &name))
                    .unwrap_or_else(|| (name.clone(), false));
                let display = pretty_device_name(&name);
                out.push((d, DeviceInfo::new(id, display, kind, is_default)));
            }
        }
    }

    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let Ok(name) = d.name() else { continue };
            let kind = DeviceKind::Input;
            if !seen.insert((kind_key(kind), name.clone())) {
                continue;
            }
            let (id, is_default) = ids
                .map(|l| l.get(kind, &name))
                .unwrap_or_else(|| (name.clone(), false));
            let display = pretty_device_name(&name);
            out.push((d, DeviceInfo::new(id, display, kind, is_default)));
        }
    }

    out
}

/// Turns raw host strings (ALSA hw strings on Linux) into readable names.
/// Everywhere else the OS already reports a friendly name and this is a no-op.
#[cfg(target_os = "linux")]
fn pretty_device_name(name: &str) -> String {
    crate::utils::format_alsa_device_name(name)
}

#[cfg(not(target_os = "linux"))]
fn pretty_device_name(name: &str) -> String {
    name.to_string()
}

/// cpal-based capture used on Windows (loopback) and as a fallback elsewhere.
pub fn open_capture(
    device_name: &str,
    policy: DefaultPolicy,
    ids: Option<&IdLookup>,
    tx: SampleTx,
) -> Result<OpenedCapture, String> {
    let host = cpal::default_host();
    let allow_loopback = policy == DefaultPolicy::OutputLoopback;
    let want_default = super::is_default_request(device_name);
    let entries = collect_devices(&host, allow_loopback, ids);

    let selected: (Device, DeviceInfo, bool) = if want_default {
        let pick: Option<(Device, DeviceInfo, bool)> = match policy {
            DefaultPolicy::OutputLoopback => host
                .default_output_device()
                .map(|d| {
                    let name = d.name().unwrap_or_else(|_| "Unknown".into());
                    let (id, is_default) = ids
                        .map(|l| l.get(DeviceKind::OutputLoopback, &name))
                        .unwrap_or_else(|| (name.clone(), true));
                    let display = pretty_device_name(&name);
                    (d, DeviceInfo::new(id, display, DeviceKind::OutputLoopback, is_default), true)
                })
                .or_else(|| first_of_kind(&entries, DeviceKind::Input).map(|(d, i)| (d, i, false))),
            DefaultPolicy::Input => host
                .default_input_device()
                .map(|d| {
                    let name = d.name().unwrap_or_else(|_| "Unknown".into());
                    let (id, is_default) = ids
                        .map(|l| l.get(DeviceKind::Input, &name))
                        .unwrap_or_else(|| (name.clone(), true));
                    let display = pretty_device_name(&name);
                    (d, DeviceInfo::new(id, display, DeviceKind::Input, is_default), true)
                }),
            DefaultPolicy::VirtualInput(hints) => {
                // Prefer a virtual loopback input so desktop audio is captured
                // (macOS has no public loopback API); otherwise the microphone.
                let virtual_entry = entries.iter().find(|(_, info)| {
                    info.kind == DeviceKind::Input
                        && hints
                            .iter()
                            .any(|h| info.name.to_lowercase().contains(&h.to_lowercase()))
                });
                match virtual_entry {
                    Some((d, info)) => Some((d.clone(), info.clone(), true)),
                    None => host.default_input_device().map(|d| {
                        let name = d.name().unwrap_or_else(|_| "Unknown".into());
                        let (id, is_default) = ids
                            .map(|l| l.get(DeviceKind::Input, &name))
                            .unwrap_or_else(|| (name.clone(), true));
                        let display = pretty_device_name(&name);
                        (d, DeviceInfo::new(id, display, DeviceKind::Input, is_default), true)
                    }),
                }
            }
        };
        // Last resort for "default": whatever we could enumerate, so the
        // visualizer still comes up on systems with no default endpoint.
        pick.or_else(|| {
            entries.first().map(|(d, i)| {
                (
                    d.clone(),
                    i.clone(),
                    matches!(i.kind, DeviceKind::OutputLoopback | DeviceKind::Monitor),
                )
            })
        })
        .ok_or_else(|| "no audio devices available".to_string())?
    } else {
        resolve_device(device_name, &entries)?
    };

    let (device, info, is_loopback) = selected;
    let (capture, sr, ch) = open_stream(device, is_loopback, tx)?;
    Ok(OpenedCapture {
        capture: Box::new(capture),
        sample_rate: sr,
        channels: ch,
        bound_name: info.name.clone(),
        bound_id: info.id.clone(),
    })
}

fn first_of_kind(entries: &[(Device, DeviceInfo)], kind: DeviceKind) -> Option<(Device, DeviceInfo)> {
    entries
        .iter()
        .find(|(_, i)| i.kind == kind)
        .map(|(d, i)| (d.clone(), i.clone()))
}

/// Resolve a stored id/label/name (or legacy substring) to a concrete device.
///
/// `DeviceInfo::id` already carries the stable platform id (when the platform
/// provides one), so matching happens against the listed info directly.
fn resolve_device(
    want: &str,
    entries: &[(Device, DeviceInfo)],
) -> Result<(Device, DeviceInfo, bool), String> {
    let infos: Vec<DeviceInfo> = entries.iter().map(|(_, i)| i.clone()).collect();
    let matched = super::find_device(want, &infos)
        .ok_or_else(|| format!("no audio device matching '{}' found", want))?
        .clone();

    let idx = entries
        .iter()
        .position(|(_, info)| info.id.eq_ignore_ascii_case(&matched.id))
        .or_else(|| entries.iter().position(|(_, info)| info.name == matched.name))
        .ok_or_else(|| format!("no audio device matching '{}' found", want))?;

    let (device, info) = entries[idx].clone();
    let is_loopback = matches!(info.kind, DeviceKind::OutputLoopback | DeviceKind::Monitor);
    Ok((device, info, is_loopback))
}

/// Lists the devices that can actually be captured on this host.
pub fn list_devices(ids: Option<&IdLookup>) -> Vec<DeviceInfo> {
    let host = cpal::default_host();
    // Loopback is only a real capability on Windows (WASAPI); everywhere else
    // an output device cannot be opened as an input stream.
    let allow_loopback = cfg!(windows);
    collect_devices(&host, allow_loopback, ids)
        .into_iter()
        .map(|(_, info)| info)
        .collect()
}
