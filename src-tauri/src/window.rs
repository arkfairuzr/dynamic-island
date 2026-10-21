/// Move and resize the window in one step. Doing position and size as two
/// separate calls leaves a frame where the window is the wrong shape.
/// All values are logical pixels.
#[tauri::command]
pub fn set_window_rect(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        unsafe {
            SetWindowPos(
                HWND(hwnd.0 as isize as *mut c_void),
                None,
                (x * scale).round() as i32,
                (y * scale).round() as i32,
                (w * scale).round() as i32,
                (h * scale).round() as i32,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        }
        .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        window
            .set_position(tauri::LogicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(tauri::LogicalSize::new(w, h))
            .map_err(|e| e.to_string())
    }
}
