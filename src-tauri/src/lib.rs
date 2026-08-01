pub mod pdf;

use pdf::{unlock_pdf_impl, UnlockError, UnlockOutcome};
use std::path::PathBuf;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn unlock_pdf(input_path: String, password: String) -> Result<UnlockOutcome, UnlockError> {
    unlock_pdf_impl(&PathBuf::from(input_path), &password)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![greet, unlock_pdf])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
