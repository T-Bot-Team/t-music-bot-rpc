//! macOS capture via cpal's CoreAudio backend.
//!
//! macOS has no public loopback API, so desktop audio has to come from a
//! virtual device (BlackHole, Loopback, ...) — those are preferred for
//! "default" and listed as monitor devices. Everything else is a microphone
//! input.
//!
//! Device identity is the CoreAudio device UID (`kAudioDevicePropertyDeviceUID`),
//! which survives renames; cpal only exposes the human readable name, so we
//! build a name -> UID map through CoreAudio directly and pair the two.

use super::cpal_util::{self, DefaultPolicy, IdLookup};
use super::{DeviceInfo, DeviceKind, OpenedCapture};
use core_foundation_sys::string::{CFStringGetCString, CFStringRef};
use coreaudio::sys::{
    kAudioDevicePropertyDeviceNameCFString, kAudioDevicePropertyDeviceUID,
    kAudioDevicePropertyScopeOutput, kAudioHardwareNoError,
    kAudioHardwarePropertyDefaultInputDevice, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMaster, kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
    kCFStringEncodingUTF8, AudioDeviceID, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize,
    AudioObjectPropertyAddress,
};
use std::ffi::CStr;
use std::mem;
use std::os::raw::c_char;
use std::ptr::null;

type SampleTx = std::sync::mpsc::SyncSender<Vec<f32>>;

/// Well-known virtual/aggregate devices that can carry desktop audio.
const VIRTUAL_HINTS: &[&str] = &["blackhole", "loopback", "soundflower", "aggregate"];

fn is_virtual_device(name: &str) -> bool {
    let n = name.to_lowercase();
    VIRTUAL_HINTS.iter().any(|h| n.contains(h))
}

unsafe fn audio_devices() -> Option<Vec<AudioDeviceID>> {
    let addr = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDevices,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMaster,
    };

    let mut data_size: u32 = 0;
    let status = AudioObjectGetPropertyDataSize(
        kAudioObjectSystemObject,
        &addr as *const _,
        0,
        null(),
        &mut data_size as *mut _,
    );
    if status != kAudioHardwareNoError as i32 {
        return None;
    }

    let count = data_size as usize / mem::size_of::<AudioDeviceID>();
    if count == 0 {
        return Some(Vec::new());
    }

    let mut out: Vec<AudioDeviceID> = Vec::with_capacity(count);
    let status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject,
        &addr as *const _,
        0,
        null(),
        &mut data_size as *mut _,
        out.as_mut_ptr() as *mut _,
    );
    if status != kAudioHardwareNoError as i32 {
        return None;
    }
    out.set_len(count);
    Some(out)
}

unsafe fn default_input_device() -> Option<AudioDeviceID> {
    let addr = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyDefaultInputDevice,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMaster,
    };
    let mut dev: AudioDeviceID = 0;
    let mut data_size: u32 = mem::size_of::<AudioDeviceID>() as u32;
    let status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject,
        &addr as *const _,
        0,
        null(),
        &mut data_size as *mut _,
        &mut dev as *mut _ as *mut _,
    );
    if status != kAudioHardwareNoError as i32 || dev == 0 {
        None
    } else {
        Some(dev)
    }
}

/// Reads a CFString property (UID or name) off a device.
unsafe fn cf_string_property(id: AudioDeviceID, selector: u32, scope: u32) -> Option<String> {
    let addr = AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: kAudioObjectPropertyElementMaster,
    };
    let mut value: CFStringRef = null();
    let mut data_size: u32 = mem::size_of::<CFStringRef>() as u32;
    let status = AudioObjectGetPropertyData(
        id,
        &addr as *const _,
        0,
        null(),
        &mut data_size as *mut _,
        &mut value as *mut _ as *mut _,
    );
    if status != kAudioHardwareNoError as i32 || value.is_null() {
        return None;
    }

    let mut buf = [0 as c_char; 1024];
    if CFStringGetCString(value, buf.as_mut_ptr(), buf.len() as isize, kCFStringEncodingUTF8) == 0 {
        return None;
    }
    let s = CStr::from_ptr(buf.as_ptr()).to_string_lossy().trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Stable CoreAudio device UID (e.g. `BuiltInSpeakerDevice`, `AppleHDAEngineOutput:...`).
fn device_uid(id: AudioDeviceID) -> Option<String> {
    unsafe { cf_string_property(id, kAudioDevicePropertyDeviceUID, kAudioObjectPropertyScopeGlobal) }
}

/// Device name exactly as cpal reports it (output scope first, like cpal), so
/// the name <-> UID pairing lines up with cpal's device list.
fn device_name(id: AudioDeviceID) -> Option<String> {
    unsafe {
        cf_string_property(id, kAudioDevicePropertyDeviceNameCFString, kAudioDevicePropertyScopeOutput)
            .or_else(|| {
                cf_string_property(
                    id,
                    kAudioDevicePropertyDeviceNameCFString,
                    kAudioObjectPropertyScopeGlobal,
                )
            })
    }
}

/// name -> (UID, is_default_input) for every CoreAudio device.
fn coreaudio_lookup() -> IdLookup {
    let mut lookup = IdLookup::default();
    let Some(devices) = (unsafe { audio_devices() }) else {
        return lookup;
    };
    let default = unsafe { default_input_device() };

    for id in devices {
        let Some(uid) = device_uid(id) else { continue };
        let name = device_name(id).unwrap_or_else(|| uid.clone());
        let is_default = default == Some(id);
        lookup.insert(DeviceKind::Input, &name, &uid, is_default);
    }
    lookup
}

pub fn open_capture(device_name: &str, tx: SampleTx) -> Result<OpenedCapture, String> {
    let ids = coreaudio_lookup();
    cpal_util::open_capture(device_name, DefaultPolicy::VirtualInput(VIRTUAL_HINTS), Some(&ids), tx)
}

pub fn list_devices() -> Vec<DeviceInfo> {
    let ids = coreaudio_lookup();
    let mut devices = cpal_util::list_devices(Some(&ids));
    for d in &mut devices {
        if is_virtual_device(&d.name) {
            d.kind = DeviceKind::Monitor;
        }
    }
    devices
}
