use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct OpenWindow {
    pub id: i64,
    pub title: String,
    pub app: String,
    pub active: bool,
}

#[cfg(target_os = "windows")]
#[path = "apps_win.rs"]
mod win;

fn list_blocking() -> Vec<OpenWindow> {
    #[cfg(target_os = "windows")]
    {
        win::list()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

fn focus_blocking(id: i64) -> bool {
    #[cfg(target_os = "windows")]
    {
        win::focus(id)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = id;
        false
    }
}

#[tauri::command]
pub async fn list_open_windows() -> Vec<OpenWindow> {
    tokio::task::spawn_blocking(list_blocking)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn focus_window(id: i64) -> bool {
    tokio::task::spawn_blocking(move || focus_blocking(id))
        .await
        .unwrap_or(false)
}
