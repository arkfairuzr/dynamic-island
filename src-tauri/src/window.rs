/// Windows only: Tauri keeps WS_CAPTION on "undecorated" windows and merely
/// hides it. Windows refuses to shrink a captioned window below ~136x39 and
/// draws a title bar at that size, so turn the window into a plain popup.
/// Returns the SetWindowPos flags needed to apply the change.
#[cfg(target_os = "windows")]
fn make_plain_popup(
    hwnd: windows::Win32::Foundation::HWND,
) -> windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
        WS_THICKFRAME,
    };

    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let plain = (style
            & !(WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0))
            | WS_POPUP.0;
        let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
        if plain != style {
            eprintln!("[island] window style {style:#x} -> {plain:#x}");
            SetWindowLongPtrW(hwnd, GWL_STYLE, plain as isize);
            flags |= SWP_FRAMECHANGED;
        }
        flags
    }
}

#[cfg(target_os = "windows")]
fn hwnd_of(window: &tauri::WebviewWindow) -> Result<windows::Win32::Foundation::HWND, String> {
    use std::ffi::c_void;
    let raw = window.hwnd().map_err(|e| e.to_string())?;
    Ok(windows::Win32::Foundation::HWND(raw.0 as isize as *mut c_void))
}

/// Called once at startup, before the frontend has loaded.
pub fn prepare(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOMOVE, SWP_NOSIZE};
        match hwnd_of(window) {
            Ok(hwnd) => unsafe {
                let flags = make_plain_popup(hwnd) | SWP_NOMOVE | SWP_NOSIZE;
                if let Err(e) = SetWindowPos(hwnd, None, 0, 0, 0, 0, flags) {
                    eprintln!("[island] prepare: SetWindowPos failed: {e}");
                }
            },
            Err(e) => eprintln!("[island] prepare: no hwnd: {e}"),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
    }
}

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
        use windows::Win32::UI::WindowsAndMessaging::SetWindowPos;

        let scale = window.scale_factor().map_err(|e| e.to_string())?;
        let hwnd = hwnd_of(&window)?;
        let (px, py, pw, ph) = (
            (x * scale).round() as i32,
            (y * scale).round() as i32,
            (w * scale).round() as i32,
            (h * scale).round() as i32,
        );
        eprintln!("[island] set_window_rect {px},{py} {pw}x{ph} (scale {scale})");
        unsafe {
            let flags = make_plain_popup(hwnd);
            SetWindowPos(hwnd, None, px, py, pw, ph, flags)
        }
        .map_err(|e| {
            eprintln!("[island] set_window_rect failed: {e}");
            e.to_string()
        })
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
