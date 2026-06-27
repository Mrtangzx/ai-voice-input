mod commands;
mod hotkey;
mod audio;
mod insert;
mod sidecar;
mod storage;
mod overlay;
mod pipeline;

use commands::{history, models, recording, settings};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            recording::start,
            recording::stop,
            history::list,
            history::delete,
            history::search,
            settings::get,
            settings::update,
            models::status,
            models::download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
