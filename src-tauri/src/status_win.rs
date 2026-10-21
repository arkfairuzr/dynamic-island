use windows::Networking::Connectivity::NetworkInformation;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use super::{BatteryStatus, VolumeStatus, WifiStatus};

pub fn battery() -> BatteryStatus {
    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err() {
        return BatteryStatus::default();
    }
    // BatteryFlag 128 = no battery, BatteryLifePercent 255 = unknown.
    let present = status.BatteryFlag & 128 == 0 && status.BatteryLifePercent != 255;
    BatteryStatus {
        present,
        percent: if present { status.BatteryLifePercent as f64 } else { 0.0 },
        charging: status.ACLineStatus == 1,
    }
}

pub fn wifi() -> WifiStatus {
    let Ok(profile) = NetworkInformation::GetInternetConnectionProfile() else {
        return WifiStatus::default();
    };
    let is_wifi = profile.IsWlanConnectionProfile().unwrap_or(false);
    let name = profile.ProfileName().map(|n| n.to_string()).unwrap_or_default();
    let bars = profile
        .GetSignalBars()
        .ok()
        .and_then(|b| b.Value().ok())
        .unwrap_or(0);
    WifiStatus { connected: true, is_wifi, name, bars }
}

fn endpoint() -> Option<IAudioEndpointVolume> {
    unsafe {
        // Command threads have no COM apartment yet; ignore "already initialised".
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()
    }
}

pub fn volume() -> VolumeStatus {
    let Some(volume) = endpoint() else {
        return VolumeStatus::default();
    };
    unsafe {
        VolumeStatus {
            available: true,
            level: volume.GetMasterVolumeLevelScalar().unwrap_or(0.0),
            muted: volume.GetMute().map(|m| m.as_bool()).unwrap_or(false),
        }
    }
}

pub fn set_volume(level: f32) -> bool {
    endpoint()
        .map(|v| unsafe { v.SetMasterVolumeLevelScalar(level, std::ptr::null()).is_ok() })
        .unwrap_or(false)
}

pub fn set_muted(muted: bool) -> bool {
    endpoint()
        .map(|v| unsafe { v.SetMute(muted, std::ptr::null()).is_ok() })
        .unwrap_or(false)
}
