/// Lets the web page write to the terminal, so problems there are visible.
#[tauri::command]
pub fn frontend_log(msg: String) {
    eprintln!("[island:js] {msg}");
}
