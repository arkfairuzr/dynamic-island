use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct MediaStatus {
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_art_url: Option<String>,
    pub progress: f64,
    pub duration: f64,
    pub player: String,
}


#[cfg(target_os = "windows")]
mod win {
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    };
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    pub fn current_session() -> Option<Session> {
        // Command threads have no COM apartment yet; ignore "already initialised".
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
        let manager = Manager::RequestAsync().ok()?.get().ok()?;
        manager.GetCurrentSession().ok()
    }

    pub fn status() -> Option<super::MediaStatus> {
        let session = current_session()?;
        let props = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let playing = session
            .GetPlaybackInfo()
            .ok()?
            .PlaybackStatus()
            .ok()?
            == PlaybackStatus::Playing;
        let timeline = session.GetTimelineProperties().ok()?;
        // TimeSpan is in 100ns ticks.
        let position = timeline.Position().ok()?.Duration as f64 / 10_000_000.0;
        let end = timeline.EndTime().ok()?.Duration as f64 / 10_000_000.0;
        let start = timeline.StartTime().ok()?.Duration as f64 / 10_000_000.0;

        Some(super::MediaStatus {
            playing,
            title: props.Title().ok()?.to_string(),
            artist: props.Artist().ok()?.to_string(),
            album: props.AlbumTitle().ok()?.to_string(),
            album_art_url: None,
            progress: (position - start).max(0.0),
            duration: (end - start).max(0.0),
            player: session
                .SourceAppUserModelId()
                .map(|id| id.to_string())
                .unwrap_or_default(),
        })
    }

    pub fn control(action: fn(&Session) -> windows::core::Result<bool>) -> bool {
        current_session()
            .and_then(|s| action(&s).ok())
            .unwrap_or(false)
    }
}

#[tauri::command]
pub fn get_media_status() -> Option<MediaStatus> {
    #[cfg(target_os = "linux")]
    {
        // Try to get MPRIS media info via playerctl
        let output = std::process::Command::new("playerctl")
            .args([
                "metadata",
                "--format",
                "{{status}}\n{{title}}\n{{artist}}\n{{album}}\n{{mpris:artUrl}}\n{{position}}\n{{mpris:length}}\n{{playerName}}",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = text.trim().split('\n').collect();

        if lines.len() < 8 {
            return None;
        }

        let position: f64 = lines[5].parse().unwrap_or(0.0) / 1_000_000.0; // microseconds to seconds
        let length: f64 = lines[6].parse().unwrap_or(0.0) / 1_000_000.0;

        Some(MediaStatus {
            playing: lines[0] == "Playing",
            title: lines[1].to_string(),
            artist: lines[2].to_string(),
            album: lines[3].to_string(),
            album_art_url: if lines[4].is_empty() {
                None
            } else {
                Some(lines[4].to_string())
            },
            progress: position,
            duration: length,
            player: lines[7].to_string(),
        })
    }

    #[cfg(target_os = "windows")]
    {
        win::status()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[tauri::command]
pub fn media_play_pause() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("playerctl")
            .arg("play-pause")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "windows")]
    {
        win::control(|s| s.TryTogglePlayPauseAsync()?.get())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

#[tauri::command]
pub fn media_next() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("playerctl")
            .arg("next")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "windows")]
    {
        win::control(|s| s.TrySkipNextAsync()?.get())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

#[tauri::command]
pub fn media_previous() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("playerctl")
            .arg("previous")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "windows")]
    {
        win::control(|s| s.TrySkipPreviousAsync()?.get())
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}
