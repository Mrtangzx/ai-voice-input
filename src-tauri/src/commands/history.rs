#[tauri::command]
pub async fn list() -> Result<Vec<String>, String> { Ok(vec![]) }

#[tauri::command]
pub async fn delete(_id: i64) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn search(_q: String) -> Result<Vec<String>, String> { Ok(vec![]) }
