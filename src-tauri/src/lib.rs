pub mod commands;
mod hotkey;
mod audio;
mod insert;
mod sidecar;
pub mod storage;
mod overlay;
mod pipeline;

use std::sync::Arc;
use sidecar::{Sidecar, SidecarKind};
use storage::Storage;

pub struct AppState {
    pub storage: Arc<Storage>,
    pub whisper: Arc<Sidecar>,
    pub llama: Arc<Sidecar>,
    pub settings_path: std::path::PathBuf,
    pub history_db_path: std::path::PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let hotkey_str = std::env::var("AV_HOTKEY").unwrap_or_else(|_| "Ctrl+Shift+Space".into());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            use tauri::Manager;
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("transcripts.db");
            let settings_path = app_data.join("settings.json");

            let storage = tauri::async_runtime::block_on(async {
                Storage::open(db_path.to_str().unwrap()).await
            })?;
            let storage = Arc::new(storage);

            let whisper = Arc::new(Sidecar::new(SidecarKind::Whisper, "http://127.0.0.1:8178"));
            let llama   = Arc::new(Sidecar::new(SidecarKind::Llama,   "http://127.0.0.1:8188"));

            // Best-effort spawn of sidecars (ignore failures - will retry on first use)
            tauri::async_runtime::block_on(async {
                let _ = sidecar::spawn(&app.handle(), SidecarKind::Whisper).await;
                let _ = sidecar::spawn(&app.handle(), SidecarKind::Llama).await;
            });

            app.manage(AppState { storage, whisper, llama, settings_path, history_db_path: db_path });

            // Register global hotkey
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            if let Some(sc) = hotkey::parse_hotkey(&hotkey_str) {
                let handle = app.handle().clone();
                app.global_shortcut().on_shortcut(sc, move |_app, _sc, event| {
                    if event.state == ShortcutState::Pressed {
                        let h = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = commands::recording::start_via_handle(h).await;
                        });
                    } else {
                        hotkey::request_stop();
                    }
                })?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::recording::start,
            commands::recording::stop,
            commands::history::list,
            commands::history::delete,
            commands::history::search,
            commands::settings::get,
            commands::settings::update,
            commands::models::status,
            commands::models::download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}