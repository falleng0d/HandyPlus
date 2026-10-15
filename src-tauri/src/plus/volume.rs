//! Lowers the default output device's volume while recording and restores
//! it afterwards. Windows only; other platforms leave the volume alone.

use log::debug;
use std::sync::Mutex;

/// Volume before `lower_to` changed it, or `None` when nothing was changed.
static SAVED_VOLUME: Mutex<Option<f32>> = Mutex::new(None);

/// Lowers the volume to `target` (0.0 to 1.0) if it is currently louder.
pub fn lower_to(target: f32) {
    let mut saved = SAVED_VOLUME.lock().unwrap();
    if saved.is_some() {
        return;
    }
    let Some(current) = platform::get_volume() else {
        return;
    };
    let target = target.clamp(0.0, 1.0);
    if current <= target {
        return;
    }
    if platform::set_volume(target) {
        *saved = Some(current);
        debug!("Volume lowered from {current:.2} to {target:.2}");
    }
}

/// Puts back the volume saved by `lower_to`.
pub fn restore() {
    if let Some(previous) = SAVED_VOLUME.lock().unwrap().take() {
        platform::set_volume(previous);
        debug!("Volume restored to {previous:.2}");
    }
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    fn endpoint_volume() -> Option<IAudioEndpointVolume> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let device = devices.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .ok()
        }
    }

    pub fn get_volume() -> Option<f32> {
        unsafe { endpoint_volume()?.GetMasterVolumeLevelScalar().ok() }
    }

    pub fn set_volume(level: f32) -> bool {
        endpoint_volume().is_some_and(|volume| unsafe {
            volume
                .SetMasterVolumeLevelScalar(level, std::ptr::null())
                .is_ok()
        })
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn get_volume() -> Option<f32> {
        None
    }

    pub fn set_volume(_level: f32) -> bool {
        false
    }
}
