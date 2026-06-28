pub mod commands;
pub mod cloud_llm;
mod hotkey;
mod audio;
mod insert;
mod sidecar;
pub mod storage;
mod overlay;
mod pipeline;

use std::sync::Arc;
use std::time::Duration;
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

            // Spawn sidecars at startup. We log failures (visible in dev console)
            // and wait briefly for each to come online so the first recording
            // doesn't race the model loader. Whisper loads faster than llama.
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                match sidecar::spawn(&app_handle, SidecarKind::Whisper).await {
                    Ok(_) => {
                        tracing::info!("whisper sidecar spawned");
                        if !whisper.wait_ready(Duration::from_secs(60), Duration::from_millis(500)).await {
                            tracing::warn!("whisper sidecar did not respond healthy within 60s");
                        } else {
                            tracing::info!("whisper sidecar healthy on :8178");
                        }
                    }
                    Err(e) => tracing::error!("failed to spawn whisper sidecar: {e}"),
                }
                match sidecar::spawn(&app_handle, SidecarKind::Llama).await {
                    Ok(_) => {
                        tracing::info!("llama sidecar spawned");
                        if !llama.wait_ready(Duration::from_secs(120), Duration::from_secs(1)).await {
                            tracing::warn!("llama sidecar did not respond healthy within 120s");
                        } else {
                            tracing::info!("llama sidecar healthy on :8188");
                        }
                    }
                    Err(e) => tracing::error!("failed to spawn llama sidecar: {e}"),
                }
            });

            app.manage(AppState { storage, whisper, llama, settings_path, history_db_path: db_path });

            // Overlay window creation disabled for crash debugging.
            // The pipeline still emits "pipeline-status" events to the main
            // window so the user gets feedback in the main UI.
            // TODO: re-enable with simpler config (e.g. no focused(false))
            // after we confirm what's panicking.

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

            // System tray icon
            #[cfg(desktop)]
            {
                use tauri::tray::TrayIconBuilder;
                let _tray = TrayIconBuilder::with_id("main-tray")
                    .tooltip("AI Voice Input")
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&tauri::menu::Menu::with_items(app, &[
                        &tauri::menu::MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?,
                        &tauri::menu::MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
                    ])?)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => { app.exit(0); }
                        _ => {}
                    })
                    .build(app)?;
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
            commands::models::download_sensevoice,
            commands::cloud_llm::test_llm,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}