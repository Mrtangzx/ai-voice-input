use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::audio;
use crate::cloud_llm::{CloudConfig, CloudLlm};
use crate::commands::settings::{default_model_for, LlmProvider, Settings};
use crate::insert;
use crate::overlay;
use crate::sidecar::Sidecar;
use crate::storage::{Storage, Transcript};
use chrono::Utc;
use tauri::Manager;

static RECORDING: AtomicBool = AtomicBool::new(false);

pub fn is_recording() -> bool { RECORDING.load(Ordering::SeqCst) }

pub async fn run_once(
    app: AppHandle,
    storage: Arc<Storage>,
    whisper: Arc<Sidecar>,
    llama: Arc<Sidecar>,
    auto_stop: Duration,
) -> Result<()> {
    if RECORDING.swap(true, Ordering::SeqCst) {
        return Err(anyhow!("already recording"));
    }
    let result = inner(app.clone(), storage, whisper, llama, auto_stop).await;
    RECORDING.store(false, Ordering::SeqCst);

    // Always hide overlay when pipeline ends, even on error
    let _ = overlay::hide(&app);
    let phase = if result.is_ok() { "done" } else { "error" };
    let _ = app.emit("pipeline-status", serde_json::json!({
        "phase": phase,
        "error": result.as_ref().err().map(|e| e.to_string()),
    }));
    result
}

async fn inner(
    app: AppHandle,
    storage: Arc<Storage>,
    whisper: Arc<Sidecar>,
    llama: Arc<Sidecar>,
    auto_stop: Duration,
) -> Result<()> {
    let _ = llama; // kept for future use; cleanup now goes via cleanup_with_settings
    let _ = overlay::show(&app, "recording", None);
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"recording"}));

    // Capture audio on a blocking thread (cpal::Stream is !Send)
    let buf = audio::SharedBuffer::default();
    let start = std::time::Instant::now();
    let pcm: Vec<f32> = tokio::task::spawn_blocking({
        let buf = buf.clone();
        move || -> Result<Vec<f32>> {
            let _stream = audio::open_input_stream(None, buf.clone())?;
            while crate::hotkey::should_stop() == false && start.elapsed() < auto_stop {
                std::thread::sleep(Duration::from_millis(50));
            }
            drop(_stream);
            Ok(buf.lock().clone())
        }
    }).await??;

    if pcm.is_empty() {
        return Err(anyhow!("empty recording"));
    }

    // Reject silent recordings before paying for ASR. SenseVoice hallucinates
    // multilingual text on near-silence, so a user who just fat-fingered the
    // hotkey would otherwise get "transfer esperar Sol Kopf открыв caval"
    // pasted into their foreground app.
    // RMS threshold of 0.005 ≈ -46 dBFS - well below normal speech but above
    // pure silence or distant ambient hum.
    let rms: f32 = (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt();
    if rms < 0.005 {
        return Err(anyhow!("recording too quiet (rms={:.4})", rms));
    }
    tracing::info!("recording ok: rms={:.4}, {} samples", rms, pcm.len());

    let _ = overlay::show(&app, "transcribing", None);
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"transcribing"}));
    let sample_rate = 16000;
    let pcm_16k = audio::resample_to_16k(&pcm, sample_rate);
    let wav = audio::encode_wav(&pcm_16k)?;
    let raw_text = whisper.transcribe(wav).await?;
    if raw_text.trim().is_empty() {
        return Err(anyhow!("empty transcription"));
    }

    // Decide which LLM to use based on settings.
    let settings = load_settings(&app).unwrap_or_default();
    let cleanup_phrase = match settings.llm_provider {
        LlmProvider::Local => "本地 LLM 整理中…",
        LlmProvider::DeepSeek => "DeepSeek 整理中…",
        LlmProvider::QwenDashScope => "通义千问整理中…",
    };
    let _ = overlay::show(&app, "cleaning", None);
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"cleaning","step": cleanup_phrase}));

    let clean_text = match cleanup_with_settings(&app, &settings, &raw_text).await {
        Ok(s) if !s.trim().is_empty() => s,
        Ok(_) => raw_text.clone(),     // provider returned empty - fall back to raw
        Err(e) => {
            // Cloud LLM failures (no key, network down, rate-limit) should not
            // break the whole flow - paste the raw transcript and log.
            tracing::warn!("cleanup failed: {e}; using raw text");
            let _ = app.emit("pipeline-status", serde_json::json!({
                "phase":"cleaning","warning": e.to_string()
            }));
            raw_text.clone()
        }
    };

    if !insert::foreground_rejects_text() {
        let _ = insert::atomic_paste(clean_text.clone());
    } else {
        let _ = insert::copy_only(clean_text.clone());
    }

    storage.insert(&Transcript {
        id: None,
        raw_text: raw_text.clone(),
        clean_text: clean_text.clone(),
        duration_ms: start.elapsed().as_millis() as i64,
        created_at: Utc::now(),
        app_name: None,
    }).await?;

    let _ = overlay::show(&app, "done", Some(&clean_text));
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"done","clean":clean_text}));
    Ok(())
}

/// Dispatch the cleanup step based on the user's chosen provider.
async fn cleanup_with_settings(
    app: &AppHandle,
    settings: &Settings,
    raw_text: &str,
) -> Result<String> {
    match settings.llm_provider {
        LlmProvider::Local => {
            // Try the local llama-server; fall back gracefully if not running.
            let llama = app.state::<crate::AppState>().llama.clone();
            llama.cleanup(raw_text).await
        }
        LlmProvider::DeepSeek | LlmProvider::QwenDashScope => {
            let api_key = settings.llm_api_key.trim();
            if api_key.is_empty() {
                return Err(anyhow!("未配置云端 API Key"));
            }
            // Use the user-set model, or the recommended default if blank.
            let model_raw = settings.llm_model.trim();
            let model = if model_raw.is_empty() {
                default_model_for(&settings.llm_provider).to_string()
            } else {
                model_raw.to_string()
            };
            let cfg = CloudConfig {
                provider: settings.llm_provider.clone(),
                api_key: api_key.to_string(),
                model,
            };
            let cloud = CloudLlm::new(cfg);
            cloud.cleanup(raw_text, settings.cleanup_intensity.clone()).await
        }
    }
}

fn load_settings(app: &AppHandle) -> Result<Settings> {
    use tauri::Manager;
    let state = app.state::<crate::AppState>();
    Settings::load_or_default(&state.settings_path).map_err(Into::into)
}