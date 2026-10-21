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
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
            WS_SYSMENU, WS_THICKFRAME,
        };

        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0 as isize as *mut c_void);
        unsafe {
            // Tauri keeps WS_CAPTION on "undecorated" windows and only hides it.
            // Windows refuses to shrink a captioned window below ~136x39 and draws
            // a title bar at that size, so make it a plain popup instead.
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let plain = (style
                & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0))
                | WS_POPUP.0;
            let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
            if plain != style {
                SetWindowLongPtrW(hwnd, GWL_STYLE, plain as isize);
                flags |= SWP_FRAMECHANGED;
            }
            SetWindowPos(
                hwnd,
                None,
                (x * scale).round() as i32,
                (y * scale).round() as i32,
                (w * scale).round() as i32,
                (h * scale).round() as i32,
                flags,
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
