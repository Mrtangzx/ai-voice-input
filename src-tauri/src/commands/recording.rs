#[tauri::command]
pub async fn start() -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn stop() -> Result<(), String> { Ok(()) }

pub async fn start_via_handle(_handle: tauri::AppHandle) -> Result<(), String> { Ok(()) }
