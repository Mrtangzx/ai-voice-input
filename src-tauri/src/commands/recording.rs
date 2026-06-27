use crate::pipeline;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub async fn start(handle: AppHandle) -> Result<(), String> {
    if pipeline::is_recording() { return Err("already recording".into()); }
    start_via_handle(handle).await
}

#[tauri::command]
pub async fn stop() -> Result<(), String> {
    crate::hotkey::request_stop();
    Ok(())
}

pub async fn start_via_handle(handle: AppHandle) -> Result<(), String> {
    use crate::AppState;
    crate::hotkey::reset_stop();
    let state = handle.state::<AppState>();
    let settings = crate::commands::settings::Settings::load_or_default(&state.settings_path)
        .map_err(|e| e.to_string())?;
    let auto_stop = Duration::from_secs(settings.auto_stop_seconds as u64);
    let app = handle.clone();
    let storage = state.storage.clone();
    let whisper = state.whisper.clone();
    let llama = state.llama.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = pipeline::run_once(app.clone(), storage, whisper, llama, auto_stop).await {
            tracing::error!("pipeline: {e}");
            let _ = app.emit("pipeline-status", serde_json::json!({"phase":"error","error":e.to_string()}));
        }
    });
    Ok(())
}