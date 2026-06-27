use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings { pub hotkey: String }

impl Default for Settings {
    fn default() -> Self { Self { hotkey: "Ctrl+Shift+Space".into() } }
}

#[tauri::command]
pub async fn get() -> Result<Settings, String> { Ok(Settings::default()) }

#[tauri::command]
pub async fn update(_s: Settings) -> Result<(), String> { Ok(()) }
