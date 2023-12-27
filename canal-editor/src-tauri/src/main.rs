// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn message(name: &str) -> String {
    format!("Message in Rust from {}", name)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![message])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
