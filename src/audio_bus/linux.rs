//! Linux/PipeWire implementation of the managed audio bus.
//!
//! Talks to the PulseAudio compatibility layer that PipeWire (and plain
//! PulseAudio) exposes, using the same `libpulse-binding` dependency the
//! capture backend already links.

use super::{BusStatus, SINK_DESCRIPTION, SINK_NAME};
use libpulse_binding as pulse;
use pulse::callbacks::ListResult;
use pulse::context::{Context, FlagSet as CtxFlags, State as CtxState};
use pulse::mainloop::threaded::Mainloop;
use pulse::operation::{Operation, State as OpState};
use pulse::volume::{ChannelVolumes, Volume};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const NULL_SINK_MODULE: &str = "module-null-sink";
const LOOPBACK_MODULE: &str = "module-loopback";
/// Matches the loopback module we (or the legacy service) created.
const LOOPBACK_ARG: &str = "source=tmusic_rpc.monitor";
const SINK_ARG: &str = "sink_name=tmusic_rpc";

/// PulseAudio volumes are cubic; the displayed percentage is `volume/NORMAL`.
/// We only ever express user intent here, so a straight linear map is correct.
fn pct_to_volume(pct: u32) -> Volume {
    let p = pct.min(150) as u64;
    Volume(((Volume::NORMAL.0 as u64 * p) / 100) as u32)
}

fn volume_to_pct(v: &ChannelVolumes) -> u8 {
    let first = v.get().first().copied().unwrap_or(Volume::NORMAL);
    ((first.0 as f64 / Volume::NORMAL.0 as f64) * 100.0).round().clamp(0.0, 150.0) as u8
}

/// Polls `probe` until it returns `Some(ok)` or the timeout elapses.
fn wait_for<F>(ml: &mut Mainloop, mut probe: F) -> bool
where
    F: FnMut() -> Option<bool>,
{
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    loop {
        let result = {
            ml.lock();
            let r = probe();
            ml.unlock();
            r
        };
        match result {
            Some(ok) => return ok,
            None if Instant::now() >= deadline => return false,
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

/// Blocks until a pulse operation completes (or times out).
fn wait_op<T: ?Sized>(ml: &mut Mainloop, op: &Operation<T>) {
    let _ = wait_for(ml, || match op.get_state() {
        OpState::Done => Some(true),
        OpState::Cancelled => Some(false),
        _ => None,
    });
}

/// Connects to the audio server, runs `f`, then disconnects. Every operation
/// gets its own short-lived context; they are infrequent (settings changes,
/// volume edits) so this keeps the code stateless and robust across restarts.
fn with_context<T>(f: impl FnOnce(&mut Mainloop, &mut Context) -> Result<T, String>) -> Result<T, String> {
    let mut ml = Mainloop::new().ok_or_else(|| "failed to create PulseAudio mainloop".to_string())?;
    ml.start().map_err(|_| "failed to start PulseAudio mainloop".to_string())?;
    let mut ctx = Context::new(&ml, "T_Music_Bot_RPC-bus")
        .ok_or_else(|| "failed to create PulseAudio context".to_string())?;

    {
        ml.lock();
        let conn = ctx.connect(None, CtxFlags::NOFLAGS, None);
        ml.unlock();
        if let Err(e) = conn {
            ml.stop();
            return Err(format!("cannot connect to audio server: {}", e));
        }
    }

    if !wait_for(&mut ml, || match ctx.get_state() {
        CtxState::Ready => Some(true),
        CtxState::Failed | CtxState::Terminated => Some(false),
        _ => None,
    }) {
        ml.stop();
        return Err("audio server connection failed or timed out".to_string());
    }

    let result = f(&mut ml, &mut ctx);

    ml.lock();
    ctx.disconnect();
    ml.unlock();
    ml.stop();
    result
}

fn load_module(ml: &mut Mainloop, ctx: &mut Context, name: &str, args: &str) -> Result<u32, String> {
    let out: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let op = {
        ml.lock();
        let op = ctx.introspect().load_module(name, args, {
            let out = out.clone();
            move |idx| {
                *out.lock().unwrap() = Some(idx);
            }
        });
        ml.unlock();
        op
    };
    wait_op(ml, &op);
    let idx = *out.lock().unwrap();
    match idx {
        Some(i) if i != u32::MAX => Ok(i),
        _ => Err(format!("failed to load {}", name)),
    }
}

fn unload_module(ml: &mut Mainloop, ctx: &mut Context, index: u32) {
    let op = {
        ml.lock();
        let op = ctx.introspect().unload_module(index, |_| {});
        ml.unlock();
        op
    };
    wait_op(ml, &op);
}

/// Finds the module whose name matches and whose argument contains `needle`.
fn module_index(ml: &mut Mainloop, ctx: &mut Context, module: &str, needle: &str) -> Option<u32> {
    let found: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let module = module.to_string();
    let needle = needle.to_string();
    let op = {
        ml.lock();
        let op = ctx.introspect().get_module_info_list({
            let found = found.clone();
            move |res| {
                if let ListResult::Item(info) = res {
                    let name = info.name.as_deref().unwrap_or("");
                    let arg = info.argument.as_deref().unwrap_or("");
                    if name == module && arg.contains(&needle) {
                        let mut slot = found.lock().unwrap();
                        if slot.is_none() {
                            *slot = Some(info.index);
                        }
                    }
                }
            }
        });
        ml.unlock();
        op
    };
    wait_op(ml, &op);
    let result = *found.lock().unwrap();
    result
}

fn sink_exists(ml: &mut Mainloop, ctx: &mut Context, name: &str) -> bool {
    let found = Arc::new(Mutex::new(false));
    let name = name.to_string();
    let op = {
        ml.lock();
        let op = ctx.introspect().get_sink_info_by_name(&name, {
            let found = found.clone();
            move |res| {
                if let ListResult::Item(_) = res {
                    *found.lock().unwrap() = true;
                }
            }
        });
        ml.unlock();
        op
    };
    wait_op(ml, &op);
    let result = *found.lock().unwrap();
    result
}

fn default_sink(ml: &mut Mainloop, ctx: &mut Context) -> Option<String> {
    let out: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let op = {
        ml.lock();
        let op = ctx.introspect().get_server_info({
            let out = out.clone();
            move |si| {
                *out.lock().unwrap() = si.default_sink_name.as_ref().map(|s| s.to_string());
            }
        });
        ml.unlock();
        op
    };
    wait_op(ml, &op);
    let result = out.lock().unwrap().clone();
    result
}

fn set_sink_volume(ml: &mut Mainloop, ctx: &mut Context, name: &str, pct: u32) -> Result<(), String> {
    let mut cv = ChannelVolumes::default();
    cv.set(2, pct_to_volume(pct));
    let ok: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let name = name.to_string();
    let op = {
        ml.lock();
        let op = ctx.introspect().set_sink_volume_by_name(&name, &cv, Some(Box::new({
            let ok = ok.clone();
            move |success| {
                *ok.lock().unwrap() = Some(success);
            }
        })));
        ml.unlock();
        op
    };
    wait_op(ml, &op);
    let success = *ok.lock().unwrap();
    match success {
        Some(true) => Ok(()),
        _ => Err(format!("failed to set volume of sink '{}'", name)),
    }
}

/// Applies `pct` to the playback stream owned by the loopback module. This is
/// the stream that carries audio to the speakers, so changing it never touches
/// what the visualizer reads from the monitor.
fn apply_loopback_volume(ml: &mut Mainloop, ctx: &mut Context, module_idx: u32, pct: u32) -> Result<(), String> {
    let target: Arc<Mutex<Option<(u32, u8)>>> = Arc::new(Mutex::new(None));
    let op = {
        ml.lock();
        let op = ctx.introspect().get_sink_input_info_list({
            let target = target.clone();
            move |res| {
                if let ListResult::Item(info) = res {
                    if info.owner_module == Some(module_idx) {
                        let channels = info.volume.get().len().max(1) as u8;
                        *target.lock().unwrap() = Some((info.index, channels));
                    }
                }
            }
        });
        ml.unlock();
        op
    };
    wait_op(ml, &op);

    let found = *target.lock().unwrap();
    let (index, channels) = found.ok_or_else(|| "music loopback stream not found".to_string())?;

    let mut cv = ChannelVolumes::default();
    cv.set(channels, pct_to_volume(pct));

    let ok: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
    let op = {
        ml.lock();
        let op = ctx.introspect().set_sink_input_volume(index, &cv, Some(Box::new({
            let ok = ok.clone();
            move |success| {
                *ok.lock().unwrap() = Some(success);
            }
        })));
        ml.unlock();
        op
    };
    wait_op(ml, &op);

    // Make sure a previously muted stream comes back.
    let op = {
        ml.lock();
        let op = ctx.introspect().set_sink_input_mute(index, false, Some(Box::new(|_| {})));
        ml.unlock();
        op
    };
    wait_op(ml, &op);

    let success = *ok.lock().unwrap();
    match success {
        Some(true) => Ok(()),
        _ => Err("failed to set music volume".to_string()),
    }
}

fn loopback_volume(ml: &mut Mainloop, ctx: &mut Context, module_idx: u32) -> Option<u8> {
    let out: Arc<Mutex<Option<u8>>> = Arc::new(Mutex::new(None));
    let op = {
        ml.lock();
        let op = ctx.introspect().get_sink_input_info_list({
            let out = out.clone();
            move |res| {
                if let ListResult::Item(info) = res {
                    if info.owner_module == Some(module_idx) {
                        *out.lock().unwrap() = Some(volume_to_pct(&info.volume));
                    }
                }
            }
        });
        ml.unlock();
        op
    };
    wait_op(ml, &op);
    let result = *out.lock().unwrap();
    result
}

pub fn ensure(listen_volume: u8) -> Result<BusStatus, String> {
    with_context(|ml, ctx| -> Result<BusStatus, String> {
        if !sink_exists(ml, ctx, SINK_NAME) {
            let args = format!(
                "sink_name={} sink_properties=device.description={}",
                SINK_NAME, SINK_DESCRIPTION
            );
            load_module(ml, ctx, NULL_SINK_MODULE, &args)?;
        }

        // The visualizer reads this sink's monitor, so it must stay at unity.
        set_sink_volume(ml, ctx, SINK_NAME, 100)?;

        let loop_idx = match module_index(ml, ctx, LOOPBACK_MODULE, LOOPBACK_ARG) {
            Some(index) => index,
            None => {
                let default = default_sink(ml, ctx).ok_or_else(|| "no default output sink found".to_string())?;
                let args = format!("source={}.monitor sink={}", SINK_NAME, default);
                load_module(ml, ctx, LOOPBACK_MODULE, &args)?
            }
        };

        apply_loopback_volume(ml, ctx, loop_idx, listen_volume as u32)?;

        Ok(BusStatus {
            supported: true,
            active: true,
            loopback: true,
            listen_volume: loopback_volume(ml, ctx, loop_idx).unwrap_or(listen_volume),
            legacy_service: legacy_service_present(),
            detail: String::new(),
        })
    })
}

pub fn set_listen_volume(volume: u8) -> Result<u8, String> {
    with_context(|ml, ctx| -> Result<u8, String> {
        let index = module_index(ml, ctx, LOOPBACK_MODULE, LOOPBACK_ARG)
            .ok_or_else(|| "music loopback is not running".to_string())?;
        apply_loopback_volume(ml, ctx, index, volume as u32)?;
        Ok(volume)
    })
}

pub fn remove() -> Result<(), String> {
    with_context(|ml, ctx| -> Result<(), String> {
        if let Some(index) = module_index(ml, ctx, LOOPBACK_MODULE, LOOPBACK_ARG) {
            unload_module(ml, ctx, index);
        }
        if let Some(index) = module_index(ml, ctx, NULL_SINK_MODULE, SINK_ARG) {
            unload_module(ml, ctx, index);
        }
        Ok(())
    })
}

pub fn status() -> BusStatus {
    let mut status = BusStatus {
        supported: true,
        legacy_service: legacy_service_present(),
        ..BusStatus::default()
    };

    match with_context(|ml, ctx| -> Result<(bool, Option<u32>), String> {
        Ok((sink_exists(ml, ctx, SINK_NAME), module_index(ml, ctx, LOOPBACK_MODULE, LOOPBACK_ARG)))
    }) {
        Ok((active, loop_idx)) => {
            status.active = active;
            if let Some(index) = loop_idx {
                status.loopback = true;
                status.listen_volume = with_context(|ml, ctx| {
                    loopback_volume(ml, ctx, index).ok_or_else(|| "loopback volume unavailable".to_string())
                })
                .unwrap_or(status.listen_volume);
            }
        }
        Err(e) => {
            status.detail = e;
        }
    }
    status
}

pub fn legacy_service_present() -> bool {
    let exists = std::env::var_os("HOME")
        .map(|home| {
            std::path::Path::new(&home)
                .join(".config/systemd/user/tmusic-audio-sink.service")
                .exists()
        })
        .unwrap_or(false);
    if !exists {
        return false;
    }
    // Only a *conflict* if the old service can actually start (it must be
    // enabled). A leftover-but-disabled unit is harmless.
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "--quiet", "tmusic-audio-sink.service"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn disable_legacy_service() -> Result<(), String> {
    let output = std::process::Command::new("systemctl")
        .args(["--user", "disable", "--now", "tmusic-audio-sink.service"])
        .output()
        .map_err(|e| format!("failed to run systemctl: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if err.is_empty() { "systemctl reported failure".to_string() } else { err })
    }
}
