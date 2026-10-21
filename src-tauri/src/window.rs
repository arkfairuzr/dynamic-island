//! Window placement for the island. On Windows everything here talks to the
//! window handle directly and never calls Tauri's window getters from the
//! command thread, which can block the UI thread.

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicIsize, Ordering};

    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
        WS_SYSMENU, WS_THICKFRAME,
    };

    /// The island window's handle, stored at startup.
    static HWND_RAW: AtomicIsize = AtomicIsize::new(0);

    pub fn store(hwnd: HWND) {
        HWND_RAW.store(hwnd.0 as isize, Ordering::SeqCst);
    }

    fn stored() -> Option<HWND> {
        match HWND_RAW.load(Ordering::SeqCst) {
            0 => None,
            raw => Some(HWND(raw as *mut c_void)),
        }
    }

    fn scale(hwnd: HWND) -> f64 {
        match unsafe { GetDpiForWindow(hwnd) } {
            0 => 1.0,
            dpi => dpi as f64 / 96.0,
        }
    }

    /// Tauri keeps WS_CAPTION on "undecorated" windows and merely hides it.
    /// Windows refuses to shrink a captioned window below ~136x39 and draws a
    /// title bar at that size, so turn the window into a plain popup.
    unsafe fn plain_popup_flags(hwnd: HWND) -> windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS {
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

    /// Startup: plain popup, idle-pill size, top centre of the screen.
    pub fn prepare(hwnd: HWND) {
        eprintln!("[island] placement thread started");
        store(hwnd);
        let scale = scale(hwnd);
        let w = (134.0 * scale).round() as i32;
        let h = (42.0 * scale).round() as i32;
        let (mut x, mut y) = (0, 0);
        unsafe {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(monitor, &mut info).as_bool() {
                x = info.rcMonitor.left + (info.rcMonitor.right - info.rcMonitor.left - w) / 2;
                y = info.rcMonitor.top + (2.0 * scale).round() as i32;
            }
            let flags = plain_popup_flags(hwnd);
            match SetWindowPos(hwnd, None, x, y, w, h, flags) {
                Ok(()) => eprintln!("[island] prepare: placed at {x},{y} {w}x{h} (scale {scale})"),
                Err(e) => eprintln!("[island] prepare: SetWindowPos failed: {e}"),
            }
        }
    }

    /// Logical-pixel rectangle; position and size change in one call.
    pub fn set_rect(x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
        let hwnd = stored().ok_or("window not ready")?;
        let scale = scale(hwnd);
        let (px, py, pw, ph) = (
            (x * scale).round() as i32,
            (y * scale).round() as i32,
            (w * scale).round() as i32,
            (h * scale).round() as i32,
        );
        eprintln!("[island] set_window_rect {px},{py} {pw}x{ph} (scale {scale})");
        unsafe {
            let flags = plain_popup_flags(hwnd);
            SetWindowPos(hwnd, None, px, py, pw, ph, flags)
        }
        .map_err(|e| e.to_string())
    }
}

/// Called once from `setup`. Only grabs the window handle there; the window
/// restyling runs on its own thread a moment later, because changing window
/// styles from inside Tauri's own message handling can deadlock the UI thread.
pub fn prepare(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        eprintln!("[island] prepare: getting window handle");
        match window.hwnd() {
            Ok(raw) => {
                let raw = raw.0 as isize;
                eprintln!("[island] prepare: got handle, scheduling placement");
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    win::prepare(windows::Win32::Foundation::HWND(raw as *mut c_void));
                });
            }
            Err(e) => eprintln!("[island] prepare: no hwnd: {e}"),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
    }
}

/// Lets the web page write to the terminal, so problems there are visible.
#[tauri::command]
pub fn frontend_log(msg: String) {
    eprintln!("[island:js] {msg}");
}

/// Move and resize the island window (logical pixels).
#[tauri::command]
pub async fn set_window_rect(x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        win::set_rect(x, y, w, h)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (x, y, w, h);
        Ok(())
    }
}
