use tauri::Manager;

mod apps;
mod clipboard;
mod media;
mod notifications;
mod status;
mod system;
mod timer;
mod weather;
mod window;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Dynamic Island 🏝️", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Get the main window and configure it for dynamic island look
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
                let _ = window.set_always_on_top(true);
                let _ = window.set_skip_taskbar(true);
                // Position and size are managed by the frontend.
                window::prepare(&window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            window::set_window_rect,
            window::frontend_log,
            apps::list_open_windows,
            apps::focus_window,
            status::get_status,
            status::set_volume,
            status::set_muted,
            system::get_system_info,
            system::get_cpu_usage,
            system::get_memory_info,
            system::get_disk_info,
            system::get_network_info,
            system::get_battery_info,
            media::get_media_status,
            media::media_play_pause,
            media::media_next,
            media::media_previous,
            weather::get_weather,
            clipboard::get_clipboard_history,
            clipboard::add_clipboard_entry,
            clipboard::clear_clipboard_history,
            notifications::get_notifications,
            notifications::dismiss_notification,
            notifications::toggle_dnd,
            notifications::get_dnd_status,
            timer::start_timer,
            timer::stop_timer,
            timer::get_timer_status,
            timer::start_stopwatch,
            timer::stop_stopwatch,
            timer::get_stopwatch_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Dynamic Island");
}
