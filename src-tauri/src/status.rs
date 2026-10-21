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

fn status_blocking() -> StatusInfo {
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
fn set_volume_blocking(level: f32) -> bool {
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

fn set_muted_blocking(muted: bool) -> bool {
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

/// Battery only, for the System widget (a single cheap call, no COM/WinRT).
pub fn battery_now() -> BatteryStatus {
    #[cfg(target_os = "windows")]
    {
        win::battery()
    }
    #[cfg(not(target_os = "windows"))]
    {
        BatteryStatus::default()
    }
}

// Windows COM / WinRT calls: keep them off the UI thread.
#[tauri::command]
pub async fn get_status() -> StatusInfo {
    tokio::task::spawn_blocking(status_blocking)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn set_volume(level: f32) -> bool {
    tokio::task::spawn_blocking(move || set_volume_blocking(level))
        .await
        .unwrap_or(false)
}

#[tauri::command]
pub async fn set_muted(muted: bool) -> bool {
    tokio::task::spawn_blocking(move || set_muted_blocking(muted))
        .await
        .unwrap_or(false)
}
