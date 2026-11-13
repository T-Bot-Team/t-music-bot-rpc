use super::{AudioCapture, DeviceInfo, DeviceKind, OpenedCapture};
use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::{Context, FlagSet as CtxFlags, State as CtxState};
use pulse::mainloop::threaded::Mainloop;
use pulse::sample::{Format, Spec};
use pulse::stream::{FlagSet as StreamFlags, PeekResult, Stream};
use std::sync::mpsc::{Receiver, RecvTimeoutError};

type SampleTx = std::sync::mpsc::SyncSender<Vec<f32>>;

const TARGET_RATE: u32 = 48000;
const TARGET_CHANNELS: u8 = 2;
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(4);
/// How often the reader wakes to pull samples from the server buffer.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

enum Cmd {
    Cork(bool),
    Quit,
}

struct SourceEntry {
    name: String,
    description: String,
    is_monitor: bool,
}

pub struct PulseCapture {
    cmd_tx: std::sync::mpsc::Sender<Cmd>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl PulseCapture {
    fn send(&self, cmd: Cmd) {
        let _ = self.cmd_tx.send(cmd);
    }
}

impl AudioCapture for PulseCapture {
    fn pause(&mut self) {
        self.send(Cmd::Cork(true));
    }
    fn resume(&mut self) {
        self.send(Cmd::Cork(false));
    }
}

impl Drop for PulseCapture {
    fn drop(&mut self) {
        self.send(Cmd::Quit);
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

/// Resolved target of a capture stream, as reported back to the caller.
struct Opened {
    sample_rate: u32,
    channels: u16,
    /// Friendly name (pulse description) shown to the user.
    name: String,
    /// Stable pulse source name persisted in settings.
    id: String,
}

/// Opens a capture stream on a PulseAudio source.
///
/// Works with both PulseAudio and PipeWire (via its PulseAudio compatibility
/// layer, which every modern desktop ships). "default" resolves to the monitor
/// of the default sink, i.e. desktop audio — not the microphone.
pub fn open_capture(device_name: &str, tx: SampleTx) -> Result<OpenedCapture, String> {
    let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<Opened, String>>();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Cmd>();
    let want = device_name.to_string();

    let worker = std::thread::Builder::new()
        .name("pulse-capture".into())
        .spawn(move || {
            run_capture_thread(&want, tx, cmd_rx, init_tx);
        })
        .map_err(|e| format!("failed to spawn capture thread: {}", e))?;

    match init_rx.recv_timeout(CONNECT_TIMEOUT * 3) {
        Ok(Ok(o)) => Ok(OpenedCapture {
            capture: Box::new(PulseCapture { cmd_tx, worker: Some(worker) }),
            sample_rate: o.sample_rate,
            channels: o.channels,
            bound_name: o.name,
            bound_id: o.id,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            let _ = cmd_tx.send(Cmd::Quit);
            Err("timed out connecting to PulseAudio/PipeWire".into())
        }
    }
}

fn run_capture_thread(
    want: &str,
    tx: SampleTx,
    cmd_rx: Receiver<Cmd>,
    init_tx: std::sync::mpsc::Sender<Result<Opened, String>>,
) {
    let Some(mut ml) = Mainloop::new() else {
        let _ = init_tx.send(Err("failed to create mainloop".into()));
        return;
    };
    if ml.start().is_err() {
        let _ = init_tx.send(Err("failed to start mainloop".into()));
        return;
    }
    let Some(mut ctx) = Context::new(&ml, "T_Music_Bot_RPC") else {
        let _ = init_tx.send(Err("failed to create context".into()));
        return;
    };

    ml.lock();
    let conn = ctx.connect(None, CtxFlags::NOFLAGS, None);
    ml.unlock();
    if let Err(e) = conn {
        let _ = init_tx.send(Err(format!("cannot connect to audio server: {}", e)));
        return;
    }

    if !wait_for(&mut ml, || match ctx.get_state() {
        CtxState::Ready => Some(true),
        CtxState::Failed | CtxState::Terminated => Some(false),
        _ => None,
    }) {
        let _ = init_tx.send(Err("audio server connection failed or timed out".into()));
        return;
    }

    // Resolve which source to record from.
    let (sources, default_sink) = {
        ml.lock();
        let introspect = ctx.introspect();
        let sources: std::rc::Rc<std::cell::RefCell<Vec<SourceEntry>>> = Default::default();
        let list_op = introspect.get_source_info_list({
            let sources = sources.clone();
            move |res| {
                if let ListResult::Item(info) = res {
                    let name = info.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                    if name.starts_with("auto_null") {
                        return; // placeholder devices for unplugged hardware
                    }
                    let description =
                        info.description.as_ref().map(|s| s.to_string()).unwrap_or_else(|| name.clone());
                    let is_monitor = name.ends_with(".monitor");
                    sources.borrow_mut().push(SourceEntry { is_monitor, name, description });
                }
            }
        });
        let default_sink: std::rc::Rc<std::cell::RefCell<Option<String>>> = Default::default();
        let server_op = introspect.get_server_info({
            let default_sink = default_sink.clone();
            move |si| {
                *default_sink.borrow_mut() = si.default_sink_name.as_ref().map(|s| s.to_string());
            }
        });
        ml.unlock();

        // The threaded mainloop dispatches callbacks on its own thread, so we
        // must wait for the operations to finish before reading the results.
        wait_for_operation(&mut ml, &list_op);
        wait_for_operation(&mut ml, &server_op);

        let sources = std::rc::Rc::try_unwrap(sources).ok().map(|c| c.into_inner()).unwrap_or_default();
        let default_sink = std::rc::Rc::try_unwrap(default_sink).ok().and_then(|c| c.into_inner());
        (sources, default_sink)
    };

    let Some(target) = resolve_source(want, &sources, default_sink.as_deref()) else {
        let _ = init_tx.send(Err(format!("no source matching '{}' found", want)));
        return;
    };

    let spec = Spec { format: Format::F32le, rate: TARGET_RATE, channels: TARGET_CHANNELS };
    if !spec.is_valid() {
        let _ = init_tx.send(Err("invalid sample spec".into()));
        return;
    }

    let mut stream = {
        ml.lock();
        let mut created = Stream::new(&mut ctx, "visualizer", &spec, None);
        // Small fragments (~10 ms) so the visualizer gets a steady flow of
        // samples instead of one large burst once a huge buffer fills up.
        let attr = pulse::def::BufferAttr {
            maxlength: u32::MAX,
            tlength: 0,
            prebuf: 0,
            minreq: 0,
            fragsize: 4096,
        };
        let connected =
            created.as_mut().map(|st| st.connect_record(Some(&target.name), Some(&attr), StreamFlags::NOFLAGS));
        ml.unlock();
        match (created, connected) {
            (Some(st), Some(Ok(()))) => st,
            (Some(_), Some(Err(e))) => {
                let _ = init_tx.send(Err(format!(
                    "failed to connect record stream to '{}': {}",
                    target.name, e
                )));
                return;
            }
            (None, _) => {
                let _ = init_tx.send(Err("failed to create stream".into()));
                return;
            }
            (_, None) => unreachable!(),
        }
    };

    if !wait_for(&mut ml, || match stream.get_state() {
        pulse::stream::State::Ready => Some(true),
        pulse::stream::State::Failed | pulse::stream::State::Terminated => Some(false),
        _ => None,
    }) {
        let _ = init_tx.send(Err(format!("record stream on '{}' failed or timed out", target.name)));
        return;
    }

    let (actual_sr, actual_ch) = {
        ml.lock();
        let ss = stream.get_sample_spec();
        let out = ss.map(|s| (s.rate, s.channels));
        ml.unlock();
        out.unwrap_or((TARGET_RATE, TARGET_CHANNELS))
    };

    let bound_name =
        if target.description.is_empty() { target.name.clone() } else { target.description.clone() };
    let _ = init_tx.send(Ok(Opened {
        sample_rate: actual_sr,
        channels: actual_ch as u16,
        name: bound_name,
        id: target.name.clone(),
    }));

    // Reader/command loop. The threaded mainloop runs in the background; this
    // thread periodically drains captured samples and services cork/quit
    // requests, always taking the mainloop lock while touching pulse objects.
    loop {
        match cmd_rx.recv_timeout(POLL_INTERVAL) {
            Ok(Cmd::Cork(cork)) => {
                ml.lock();
                if cork {
                    let _op = stream.cork(None);
                } else {
                    let _op = stream.uncork(None);
                }
                ml.unlock();
            }
            Ok(Cmd::Quit) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        // Drain everything currently buffered.
        loop {
            ml.lock();
            let result = stream.peek();
            match result {
                Ok(PeekResult::Data(data)) => {
                    let samples: Vec<f32> = data
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect();
                    let _ = stream.discard();
                    ml.unlock();
                    if !samples.is_empty() {
                        let _ = tx.try_send(samples);
                    }
                }
                Ok(PeekResult::Hole(_)) => {
                    let _ = stream.discard();
                    ml.unlock();
                }
                Ok(PeekResult::Empty) | Err(_) => {
                    ml.unlock();
                    break;
                }
            }
        }
    }

    ml.lock();
    let _ = stream.disconnect();
    ctx.disconnect();
    ml.unlock();
    ml.stop();
}

/// Polls `probe` until it returns `Some(ok)` or the timeout elapses.
/// Must be called from the thread owning the mainloop; each probe takes the lock.
fn wait_for<F>(ml: &mut Mainloop, mut probe: F) -> bool
where
    F: FnMut() -> Option<bool>,
{
    let deadline = std::time::Instant::now() + CONNECT_TIMEOUT;
    loop {
        let result = {
            ml.lock();
            let r = probe();
            ml.unlock();
            r
        };
        match result {
            Some(ok) => return ok,
            None if std::time::Instant::now() >= deadline => return false,
            None => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
}

/// Blocks until a pulse operation completes (or times out). The threaded
/// mainloop executes the operation's callbacks on its own thread, so results
/// written by those callbacks are only safe to read once this returns.
fn wait_for_operation<T: ?Sized>(ml: &mut Mainloop, op: &pulse::operation::Operation<T>) {
    let _ = wait_for(ml, || match op.get_state() {
        pulse::operation::State::Done => Some(true),
        pulse::operation::State::Cancelled => Some(false),
        _ => None,
    });
}

fn resolve_source<'a>(want: &str, sources: &'a [SourceEntry], default_sink: Option<&str>) -> Option<&'a SourceEntry> {
    if super::is_default_request(want) {
        let monitor_name = default_sink.map(|s| format!("{}.monitor", s));
        return sources
            .iter()
            .find(|s| Some(s.name.as_str()) == monitor_name.as_deref())
            .or_else(|| sources.iter().find(|s| s.is_monitor));
    }

    // 1. Exact stable id (the pulse source name) — what settings store.
    if let Some(exact) = sources.iter().find(|s| s.name.eq_ignore_ascii_case(want)) {
        return Some(exact);
    }

    // 2. Substring on the id or the friendly description (legacy settings and
    //    hand-typed values).
    let want_lower = want.to_lowercase();
    let matches: Vec<&SourceEntry> = sources
        .iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&want_lower) || s.description.to_lowercase().contains(&want_lower)
        })
        .collect();

    // Prefer monitors so that selecting a sink name captures its desktop audio,
    // then fall back to raw source names (microphones).
    matches.iter().find(|s| s.is_monitor).copied().or_else(|| matches.first().copied())
}

/// Lists all sources; monitors first (desktop audio), then inputs.
///
/// Each entry carries the stable pulse source name as its id, the pulse
/// description as its friendly name, and marks whichever source is the system
/// default (monitor of the default sink, or the default source).
pub fn list_devices() -> Option<Vec<DeviceInfo>> {
    let mut ml = Mainloop::new()?;
    ml.start().ok()?;
    let mut ctx = Context::new(&ml, "T_Music_Bot_RPC-devices")?;
    {
        ml.lock();
        let conn = ctx.connect(None, CtxFlags::NOFLAGS, None);
        ml.unlock();
        conn.ok()?;
    }

    let ready = wait_for(&mut ml, || match ctx.get_state() {
        CtxState::Ready => Some(true),
        CtxState::Failed | CtxState::Terminated => Some(false),
        _ => None,
    });
    if !ready {
        ml.stop();
        return None;
    }

    let (raw, default_sink, default_source) = {
        ml.lock();
        let introspect = ctx.introspect();
        let raw: std::rc::Rc<std::cell::RefCell<Vec<(String, String, bool)>>> = Default::default();
        let list_op = introspect.get_source_info_list({
            let raw = raw.clone();
            move |res| {
                if let ListResult::Item(info) = res {
                    let name = info.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
                    if name.starts_with("auto_null") {
                        return;
                    }
                    let description =
                        info.description.as_ref().map(|s| s.to_string()).unwrap_or_else(|| name.clone());
                    let is_monitor = name.ends_with(".monitor");
                    raw.borrow_mut().push((name, description, is_monitor));
                }
            }
        });
        let server: std::rc::Rc<std::cell::RefCell<(Option<String>, Option<String>)>> = Default::default();
        let server_op = introspect.get_server_info({
            let server = server.clone();
            move |si| {
                let mut s = server.borrow_mut();
                s.0 = si.default_sink_name.as_ref().map(|v| v.to_string());
                s.1 = si.default_source_name.as_ref().map(|v| v.to_string());
            }
        });
        ml.unlock();

        wait_for_operation(&mut ml, &list_op);
        wait_for_operation(&mut ml, &server_op);

        let raw = std::rc::Rc::try_unwrap(raw).ok().map(|c| c.into_inner()).unwrap_or_default();
        let (sink, source) =
            std::rc::Rc::try_unwrap(server).ok().map(|c| c.into_inner()).unwrap_or_default();
        (raw, sink, source)
    };

    ml.lock();
    ctx.disconnect();
    ml.unlock();
    ml.stop();

    let default_monitor = default_sink.map(|s| format!("{}.monitor", s));

    let mut monitors: Vec<DeviceInfo> = Vec::new();
    let mut inputs: Vec<DeviceInfo> = Vec::new();
    for (name, display, is_monitor) in raw {
        let kind = if is_monitor { DeviceKind::Monitor } else { DeviceKind::Input };
        let is_default = if is_monitor {
            default_monitor.as_deref() == Some(name.as_str())
        } else {
            default_source.as_deref() == Some(name.as_str())
        };
        let entry = DeviceInfo::new(name, display, kind, is_default);
        if is_monitor {
            monitors.push(entry);
        } else {
            inputs.push(entry);
        }
    }
    monitors.extend(inputs);
    Some(monitors)
}
