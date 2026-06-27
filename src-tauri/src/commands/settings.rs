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
    /// Which LLM to use for cleanup. "local" uses the bundled llama.cpp;
    /// "deepseek" / "qwen" call the respective online APIs.
    #[serde(default = "default_llm_provider")]
    pub llm_provider: LlmProvider,
    /// API key for the cloud LLM provider. Empty when using local.
    #[serde(default)]
    pub llm_api_key: String,
    /// Model name to use with the chosen cloud provider.
    #[serde(default = "default_llm_model")]
    pub llm_model: String,
}

fn default_llm_provider() -> LlmProvider { LlmProvider::Local }
fn default_llm_model() -> String { "deepseek-chat".into() }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    Local,
    DeepSeek,
    QwenDashScope,
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
            llm_provider: LlmProvider::Local,
            llm_api_key: String::new(),
            llm_model: "deepseek-chat".into(),
        }
    }
}

impl Settings {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let txt = std::fs::read_to_string(path)?;
        // Older settings files may be missing the new fields; serde will
        // use the defaults above thanks to `#[serde(default = ...)]`.
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

/// Returns the recommended (provider, model) pair for the chosen provider.
/// Useful when the user just toggled a new provider and we want to
/// pre-fill the model field with a sensible default.
pub fn default_model_for(provider: &LlmProvider) -> &'static str {
    match provider {
        LlmProvider::DeepSeek => "deepseek-chat",
        LlmProvider::QwenDashScope => "qwen-turbo",
        LlmProvider::Local => "local",
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
        s.llm_provider = LlmProvider::DeepSeek;
        s.llm_api_key = "sk-test".into();
        s.save(&path).unwrap();

        let loaded = Settings::load_or_default(&path).unwrap();
        assert_eq!(loaded.hotkey, "Ctrl+Alt+V");
        assert_eq!(loaded.llm_provider, LlmProvider::DeepSeek);
        assert_eq!(loaded.llm_api_key, "sk-test");
    }

    #[test]
    fn defaults_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let s = Settings::load_or_default(&path).unwrap();
        assert_eq!(s.hotkey, "Ctrl+Shift+Space");
        assert_eq!(s.auto_stop_seconds, 30);
        assert_eq!(s.model_variant, ModelVariant::Balanced);
        assert_eq!(s.llm_provider, LlmProvider::Local);
    }

    #[test]
    fn corrupted_file_falls_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, "{ this is not valid json").unwrap();
        let s = Settings::load_or_default(&path).unwrap();
        assert_eq!(s.hotkey, "Ctrl+Shift+Space");
    }

    #[test]
    fn legacy_settings_without_llm_fields_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        // Simulate an older settings file written before LLM fields existed.
        std::fs::write(&path, r#"{
            "hotkey": "Ctrl+Alt+V",
            "mic_device_id": null,
            "auto_stop_seconds": 20,
            "model_variant": "balanced",
            "cleanup_intensity": "normal",
            "auto_launch": false,
            "overlay_follow_cursor": true
        }"#).unwrap();
        let s = Settings::load_or_default(&path).unwrap();
        assert_eq!(s.hotkey, "Ctrl+Alt+V");
        assert_eq!(s.llm_provider, LlmProvider::Local);
        assert_eq!(s.llm_api_key, "");
        assert_eq!(s.llm_model, "deepseek-chat");
    }

    #[test]
    fn default_model_for_provider() {
        assert_eq!(default_model_for(&LlmProvider::DeepSeek), "deepseek-chat");
        assert_eq!(default_model_for(&LlmProvider::QwenDashScope), "qwen-turbo");
    }
}