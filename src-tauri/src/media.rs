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
    use std::sync::Mutex;

    use windows::core::HSTRING;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
    };
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    /// Print a problem to the terminal once, not every second.
    fn log_once(msg: &str) {
        static LAST: Mutex<String> = Mutex::new(String::new());
        if let Ok(mut last) = LAST.lock() {
            if *last != msg {
                eprintln!("[island] media: {msg}");
                *last = msg.to_string();
            }
        }
    }

    fn is_playing(session: &Session) -> bool {
        session
            .GetPlaybackInfo()
            .and_then(|info| info.PlaybackStatus())
            .map(|status| status == PlaybackStatus::Playing)
            .unwrap_or(false)
    }

    /// The session that is playing right now, else the system's current one.
    pub fn current_session() -> Option<Session> {
        // Pool threads have no COM apartment yet; ignore "already initialised".
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
        let manager = match Manager::RequestAsync().and_then(|op| op.get()) {
            Ok(manager) => manager,
            Err(e) => {
                log_once(&format!("cannot open the media session manager: {e}"));
                return None;
            }
        };

        let mut fallback = manager.GetCurrentSession().ok();
        if let Ok(sessions) = manager.GetSessions() {
            for session in sessions {
                if is_playing(&session) {
                    return Some(session);
                }
                if fallback.is_none() {
                    fallback = Some(session);
                }
            }
        }
        if fallback.is_none() {
            log_once("no media session found");
        }
        fallback
    }

    pub fn status() -> Option<super::MediaStatus> {
        let session = current_session()?;
        let props = match session.TryGetMediaPropertiesAsync().and_then(|op| op.get()) {
            Ok(props) => props,
            Err(e) => {
                log_once(&format!("cannot read track info: {e}"));
                return None;
            }
        };
        let text = |r: windows::core::Result<HSTRING>| r.map(|h| h.to_string()).unwrap_or_default();

        // TimeSpan is in 100ns ticks. Some apps do not report a timeline: that is fine.
        let (progress, duration) = session
            .GetTimelineProperties()
            .ok()
            .and_then(|t| {
                Some((
                    t.Position().ok()?.Duration,
                    t.StartTime().ok()?.Duration,
                    t.EndTime().ok()?.Duration,
                ))
            })
            .map(|(pos, start, end)| {
                (
                    ((pos - start) as f64 / 10_000_000.0).max(0.0),
                    ((end - start) as f64 / 10_000_000.0).max(0.0),
                )
            })
            .unwrap_or((0.0, 0.0));

        let status = super::MediaStatus {
            playing: is_playing(&session),
            title: text(props.Title()),
            artist: text(props.Artist()),
            album: text(props.AlbumTitle()),
            album_art_url: None,
            progress,
            duration,
            player: text(session.SourceAppUserModelId()),
        };
        log_once(&format!(
            "{} - {} ({}, {})",
            status.artist,
            status.title,
            status.player,
            if status.playing { "playing" } else { "paused" }
        ));
        Some(status)
    }

    pub fn control(action: fn(&Session) -> windows::core::Result<bool>) -> bool {
        current_session()
            .and_then(|s| action(&s).ok())
            .unwrap_or(false)
    }
}

fn media_status_blocking() -> Option<MediaStatus> {
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

fn play_pause_blocking() -> bool {
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

fn next_blocking() -> bool {
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

fn previous_blocking() -> bool {
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

// The Windows media API blocks until it completes and needs a free message
// pump, so these must never run on the UI thread: run them on the blocking pool.
#[tauri::command]
pub async fn get_media_status() -> Option<MediaStatus> {
    tokio::task::spawn_blocking(media_status_blocking)
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub async fn media_play_pause() -> bool {
    tokio::task::spawn_blocking(play_pause_blocking).await.unwrap_or(false)
}

#[tauri::command]
pub async fn media_next() -> bool {
    tokio::task::spawn_blocking(next_blocking).await.unwrap_or(false)
}

#[tauri::command]
pub async fn media_previous() -> bool {
    tokio::task::spawn_blocking(previous_blocking).await.unwrap_or(false)
}
