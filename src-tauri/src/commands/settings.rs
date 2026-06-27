use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub hotkey: String,
    pub mic_device_id: Option<String>,
    pub auto_stop_seconds: u32,
    pub model_variant: ModelVariant,
    pub cleanup_intensity: CleanupIntensity,
    pub auto_launch: bool,
    pub overlay_follow_cursor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModelVariant { Fast, Balanced, Accurate }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CleanupIntensity { Light, Normal, Aggressive }

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+Space".into(),
            mic_device_id: None,
            auto_stop_seconds: 30,
            model_variant: ModelVariant::Balanced,
            cleanup_intensity: CleanupIntensity::Normal,
            auto_launch: true,
            overlay_follow_cursor: true,
        }
    }
}

impl Settings {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let txt = std::fs::read_to_string(path)?;
        let s: Settings = serde_json::from_str(&txt).unwrap_or_default();
        Ok(s)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let txt = serde_json::to_string_pretty(self)?;
        std::fs::write(path, txt)?;
        Ok(())
    }
}

#[tauri::command]
pub async fn get(app: tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    Settings::load_or_default(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app)?;
    settings.save(&path).map_err(|e| e.to_string())
}

fn settings_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn settings_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let mut s = Settings::load_or_default(&path).unwrap();
        s.hotkey = "Ctrl+Alt+V".into();
        s.save(&path).unwrap();

        let loaded = Settings::load_or_default(&path).unwrap();
        assert_eq!(loaded.hotkey, "Ctrl+Alt+V");
    }

    #[test]
    fn defaults_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let s = Settings::load_or_default(&path).unwrap();
        assert_eq!(s.hotkey, "Ctrl+Shift+Space");
        assert_eq!(s.auto_stop_seconds, 30);
        assert_eq!(s.model_variant, ModelVariant::Balanced);
    }

    #[test]
    fn corrupted_file_falls_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, "{ this is not valid json").unwrap();
        let s = Settings::load_or_default(&path).unwrap();
        assert_eq!(s.hotkey, "Ctrl+Shift+Space");
    }
}