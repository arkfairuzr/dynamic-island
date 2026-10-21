use std::ffi::c_void;

use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow,
    GWL_EXSTYLE, GW_OWNER, SW_RESTORE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

use super::OpenWindow;

pub fn list() -> Vec<OpenWindow> {
    let mut windows: Vec<OpenWindow> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut windows as *mut Vec<OpenWindow> as isize));
    }
    // Stable order (like a taskbar) instead of z-order, which reshuffles on every focus.
    windows.sort_by(|a, b| a.app.to_lowercase().cmp(&b.app.to_lowercase()).then(a.id.cmp(&b.id)));
    windows
}

pub fn focus(id: i64) -> bool {
    let hwnd = HWND(id as isize as *mut c_void);
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd).as_bool()
    }
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<OpenWindow>);
    if let Some(window) = describe(hwnd) {
        list.push(window);
    }
    BOOL(1)
}

/// Returns the window if it would show up on the Windows taskbar.
fn describe(hwnd: HWND) -> Option<OpenWindow> {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return None;
        }

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let app_window = ex_style & WS_EX_APPWINDOW.0 != 0;
        let tool_window = ex_style & WS_EX_TOOLWINDOW.0 != 0;
        let has_owner = GetWindow(hwnd, GW_OWNER)
            .map(|owner| !owner.0.is_null())
            .unwrap_or(false);
        if (tool_window || has_owner) && !app_window {
            return None;
        }

        // Windows keeps hidden UWP windows "visible" but cloaked.
        let mut cloaked: u32 = 0;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return None;
        }

        let mut buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len <= 0 {
            return None;
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        if title == "Program Manager" {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == std::process::id() {
            return None;
        }

        let mut app = exe_name(pid);
        if app.is_empty() || app.eq_ignore_ascii_case("ApplicationFrameHost") {
            app = title.clone();
        }

        Some(OpenWindow {
            id: hwnd.0 as isize as i64,
            title,
            app,
            active: GetForegroundWindow() == hwnd,
        })
    }
}

fn exe_name(pid: u32) -> String {
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
            .is_ok();
        let _ = CloseHandle(process);
        if !ok {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    }
}
