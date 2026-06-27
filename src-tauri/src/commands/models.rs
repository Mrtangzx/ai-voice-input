#[tauri::command]
pub async fn status() -> Result<String, String> { Ok("not-installed".into()) }

#[tauri::command]
pub async fn download() -> Result<(), String> { Ok(()) }
