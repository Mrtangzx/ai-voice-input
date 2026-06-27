use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::audio;
use crate::insert;
use crate::overlay;
use crate::sidecar::{Sidecar, SidecarKind};
use crate::storage::{Storage, Transcript};
use chrono::Utc;

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

    let _ = overlay::show(&app, "transcribing", None);
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"transcribing"}));
    let sample_rate = 16000;
    let pcm_16k = audio::resample_to_16k(&pcm, sample_rate);
    let wav = audio::encode_wav(&pcm_16k)?;
    let raw_text = whisper.transcribe(wav).await?;
    if raw_text.trim().is_empty() {
        return Err(anyhow!("empty transcription"));
    }

    let _ = overlay::show(&app, "cleaning", None);
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"cleaning"}));
    let clean_text = match llama.cleanup(&raw_text).await {
        Ok(s) if !s.trim().is_empty() => s,
        _ => raw_text.clone(),
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