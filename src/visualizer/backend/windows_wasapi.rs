//! Windows capture via cpal's WASAPI backend.
//!
//! "default" (or empty) captures the default output device through WASAPI
//! loopback — i.e. desktop audio. A custom name first matches output devices
//! (loopback), then input devices (microphone) as a fallback.
//!
//! Endpoint identity comes from `IMMDevice::GetId`, which is the stable id
//! Windows itself persists for a device (it survives renames and driver
//! reinstalls). cpal only exposes the friendly name, so we enumerate the
//! endpoint list through COM once per call and pair the two by name.

use super::cpal_util::{self, DefaultPolicy, IdLookup};
use super::{DeviceInfo, DeviceKind, OpenedCapture};
use windows::core::PROPVARIANT;
use windows::Win32::Devices::Properties::DEVPKEY_Device_FriendlyName;
use windows::Win32::Media::Audio as AudioApi;
use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
use windows::Win32::System::Com::{self as Com, STGM_READ};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

type SampleTx = std::sync::mpsc::SyncSender<Vec<f32>>;

/// RAII COM initialization for the calling thread.
///
/// Mirrors cpal's own helper: STA is requested because that is what the rest
/// of the audio stack (and winit's drag & drop) expects; `RPC_E_CHANGED_MODE`
/// means somebody else already initialized the thread, which is fine.
fn com_guard() -> ComGuard {
    unsafe {
        let hr = Com::CoInitializeEx(None, Com::COINIT_APARTMENTTHREADED);
        // `RPC_E_CHANGED_MODE` (and any other failure) means the thread was
        // already initialized elsewhere — never call `CoUninitialize` then.
        ComGuard { initialized: hr.is_ok() }
    }
}

struct ComGuard {
    initialized: bool,
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { Com::CoUninitialize() };
        }
    }
}

/// Reads `DEVPKEY_Device_FriendlyName` from an endpoint's property store.
///
/// Falls back to the endpoint id when the store cannot be read, so a device
/// still shows up in the list.
unsafe fn friendly_name(device: &AudioApi::IMMDevice, fallback: &str) -> String {
    let store = match device.OpenPropertyStore(STGM_READ) {
        Ok(s) => s,
        Err(_) => return fallback.to_string(),
    };

    let mut value: PROPVARIANT =
        match store.GetValue(&DEVPKEY_Device_FriendlyName as *const _ as *const PROPERTYKEY) {
            Ok(v) => v,
            Err(_) => return fallback.to_string(),
        };

    let raw = value.as_raw().Anonymous.Anonymous;
    let out = if raw.vt == VT_LPWSTR.0 {
        let ptr_utf16 = *(&raw.Anonymous as *const _ as *const *const u16);
        if ptr_utf16.is_null() {
            fallback.to_string()
        } else {
            let mut len = 0;
            while *ptr_utf16.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr_utf16, len);
            String::from_utf16_lossy(slice)
        }
    } else {
        fallback.to_string()
    };

    let _ = PropVariantClear(&mut value);
    if out.trim().is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

/// Stable endpoint id (`{0.0.0.00000000}.{guid}`), freed with `CoTaskMemFree`.
unsafe fn endpoint_id(device: &AudioApi::IMMDevice) -> Option<String> {
    let ptr = device.GetId().ok()?;
    let mut len = 0;
    while *ptr.0.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr.0, len);
    let id = String::from_utf16_lossy(slice);
    Com::CoTaskMemFree(Some(ptr.0 as *const _));
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// Enumerates active endpoints for one data flow into the id lookup and
/// records which of them is the current system default.
unsafe fn collect_flow(
    enumerator: &AudioApi::IMMDeviceEnumerator,
    flow: AudioApi::EDataFlow,
    kind: DeviceKind,
    lookup: &mut IdLookup,
) {
    let default_id = enumerator
        .GetDefaultAudioEndpoint(flow, AudioApi::eConsole)
        .ok()
        .and_then(|dev| endpoint_id(&dev));

    let Ok(collection) = enumerator.EnumAudioEndpoints(flow, AudioApi::DEVICE_STATE_ACTIVE) else {
        return;
    };
    let Ok(count) = collection.GetCount() else { return };

    for i in 0..count {
        let Ok(device) = collection.Item(i) else { continue };
        let Some(id) = endpoint_id(&device) else { continue };
        let is_default = default_id.as_deref() == Some(id.as_str());
        let name = friendly_name(&device, &id);
        lookup.insert(kind, &name, &id, is_default);
    }
}

/// Snapshot of every active endpoint: (kind, friendly name) -> (stable id, is_default).
fn endpoint_lookup() -> IdLookup {
    let _guard = com_guard();
    let mut lookup = IdLookup::default();

    let enumerator = unsafe {
        match Com::CoCreateInstance::<_, AudioApi::IMMDeviceEnumerator>(
            &AudioApi::MMDeviceEnumerator,
            None,
            Com::CLSCTX_ALL,
        ) {
            Ok(e) => e,
            Err(_) => return lookup,
        }
    };

    unsafe {
        collect_flow(&enumerator, AudioApi::eRender, DeviceKind::OutputLoopback, &mut lookup);
        collect_flow(&enumerator, AudioApi::eCapture, DeviceKind::Input, &mut lookup);
    }

    lookup
}

pub fn open_capture(device_name: &str, tx: SampleTx) -> Result<OpenedCapture, String> {
    let ids = endpoint_lookup();
    cpal_util::open_capture(device_name, DefaultPolicy::OutputLoopback, Some(&ids), tx)
}

pub fn list_devices() -> Vec<DeviceInfo> {
    let ids = endpoint_lookup();
    cpal_util::list_devices(Some(&ids))
}
