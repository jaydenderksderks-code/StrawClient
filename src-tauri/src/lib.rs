#[tauri::command]
fn platform_info() -> String {
    format!("NATIVE BACKEND ONLINE · {}", std::env::consts::OS.to_uppercase())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![platform_info])
        .run(tauri::generate_context!())
        .expect("error while running StrawClient");
}
