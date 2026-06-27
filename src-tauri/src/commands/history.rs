use crate::storage::Transcript;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn list(state: State<'_, AppState>) -> Result<Vec<Transcript>, String> {
    state.storage.list(100, 0).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.storage.delete(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search(state: State<'_, AppState>, q: String) -> Result<Vec<Transcript>, String> {
    state.storage.search(&q, 100).await.map_err(|e| e.to_string())
}