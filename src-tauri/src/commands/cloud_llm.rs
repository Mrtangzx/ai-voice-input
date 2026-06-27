use crate::cloud_llm::{CloudConfig, CloudLlm};
use crate::commands::settings::{default_model_for, LlmProvider, Settings};
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
pub struct LlmTestResult {
    pub ok: bool,
    pub msg: String,
}

/// Verifies the user's cloud LLM credentials are valid by sending a tiny
/// cleanup request. Useful from the Settings UI when the user has just
/// pasted a key.
#[tauri::command]
pub async fn test_llm(app: AppHandle) -> Result<LlmTestResult, String> {
    let state = app.state::<crate::AppState>();
    let settings = Settings::load_or_default(&state.settings_path).map_err(|e| e.to_string())?;

    if matches!(settings.llm_provider, LlmProvider::Local) {
        return Ok(LlmTestResult {
            ok: false,
            msg: "当前为本地模型模式，无法测试云端 API".into(),
        });
    }
    if settings.llm_api_key.trim().is_empty() {
        return Ok(LlmTestResult {
            ok: false,
            msg: "未填写 API Key".into(),
        });
    }

    let model = if settings.llm_model.trim().is_empty() {
        default_model_for(&settings.llm_provider).to_string()
    } else {
        settings.llm_model.trim().to_string()
    };
    let cfg = CloudConfig {
        provider: settings.llm_provider.clone(),
        api_key: settings.llm_api_key.trim().to_string(),
        model,
    };
    let cloud = CloudLlm::new(cfg);
    match cloud.cleanup("测试一下语音输入", settings.cleanup_intensity).await {
        Ok(t) => Ok(LlmTestResult { ok: true, msg: format!("连接成功，回复：{}", t) }),
        Err(e) => Ok(LlmTestResult { ok: false, msg: e.to_string() }),
    }
}