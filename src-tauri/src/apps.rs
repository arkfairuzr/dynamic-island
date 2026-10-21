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

#[tauri::command]
pub fn list_open_windows() -> Vec<OpenWindow> {
    #[cfg(target_os = "windows")]
    {
        win::list()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[tauri::command]
pub fn focus_window(id: i64) -> bool {
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
