use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct BatteryStatus {
    pub present: bool,
    pub percent: f64,
    pub charging: bool,
}

#[derive(Serialize, Clone, Default)]
pub struct WifiStatus {
    pub connected: bool,
    pub is_wifi: bool,
    pub name: String,
    pub bars: u8,
}

#[derive(Serialize, Clone, Default)]
pub struct VolumeStatus {
    pub available: bool,
    pub level: f32,
    pub muted: bool,
}

#[derive(Serialize, Clone, Default)]
pub struct StatusInfo {
    pub battery: BatteryStatus,
    pub wifi: WifiStatus,
    pub volume: VolumeStatus,
}

#[cfg(target_os = "windows")]
#[path = "status_win.rs"]
mod win;

#[tauri::command]
pub fn get_status() -> StatusInfo {
    #[cfg(target_os = "windows")]
    {
        StatusInfo {
            battery: win::battery(),
            wifi: win::wifi(),
            volume: win::volume(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        StatusInfo::default()
    }
}

/// `level` is 0.0..=1.0.
#[tauri::command]
pub fn set_volume(level: f32) -> bool {
    #[cfg(target_os = "windows")]
    {
        win::set_volume(level.clamp(0.0, 1.0))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = level;
        false
    }
}

#[tauri::command]
pub fn set_muted(muted: bool) -> bool {
    #[cfg(target_os = "windows")]
    {
        win::set_muted(muted)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = muted;
        false
    }
}
