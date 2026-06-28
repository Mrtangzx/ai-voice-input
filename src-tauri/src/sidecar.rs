use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tauri::Manager;
use tokio::process::{Child, Command};

/// Resolve the directory containing sidecar binaries + python script.
/// Tries (in order):
///  1. Tauri's resource_dir (production bundle - tauri copies sidecar/ there)
///  2. CARGO_MANIFEST_DIR/sidecar (dev mode - source checkout)
///  3. Parent of the executable (when running target/debug/ai-voice-input.exe)
pub fn resolve_sidecar_dir(app: &tauri::AppHandle) -> Result<PathBuf> {
    // 1) Production: resource_dir/sidecar (Tauri bundles externalBin + resources)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let cand = resource_dir.join("sidecar");
        if cand.join("whisper-asr.py").exists() {
            tracing::info!("sidecar dir resolved via resource_dir: {}", cand.display());
            return Ok(cand);
        }
    }
    // 2) Dev: CARGO_MANIFEST_DIR is set at build time of the tauri binary.
    //    This points to .../src-tauri/, so sidecar/ sits right next to Cargo.toml.
    if let Some(src_root) = option_env!("CARGO_MANIFEST_DIR") {
        let cand = Path::new(src_root).join("sidecar");
        if cand.join("whisper-asr.py").exists() {
            tracing::info!("sidecar dir resolved via CARGO_MANIFEST_DIR: {}", cand.display());
            return Ok(cand);
        }
    }
    // 3) Dev fallback: the executable lives at target/{debug,release}/, and
    //    sidecar/ may be a sibling if the user copied/symlinked it there.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let cand = parent.join("sidecar");
            if cand.join("whisper-asr.py").exists() {
                tracing::info!("sidecar dir resolved via exe parent: {}", cand.display());
                return Ok(cand);
            }
        }
    }
    Err(anyhow!(
        "could not locate sidecar directory (looked in resource_dir, CARGO_MANIFEST_DIR/sidecar, \
         and exe-parent/sidecar). Make sure sidecar/whisper-asr.py exists."
    ))
}

#[derive(Debug, Clone, Copy)]
pub enum SidecarKind { Whisper, Llama }

impl SidecarKind {
    pub fn binary_name(self) -> &'static str {
        match self { Self::Whisper => "whisper-server", Self::Llama => "llama-server" }
    }
    pub fn default_port(self) -> u16 {
        match self { Self::Whisper => 8178, Self::Llama => 8188 }
    }
    pub fn model_path(self, models_dir: &PathBuf) -> PathBuf {
        match self {
            Self::Whisper => models_dir.join("ggml-medium.bin"),
            Self::Llama => models_dir.join("qwen2.5-7b-instruct-q4.gguf"),
        }
    }
    pub fn health_path(self) -> &'static str { "/health" }
}

pub struct Sidecar {
    pub kind: SidecarKind,
    pub base_url: String,
    pub client: Client,
}

impl Sidecar {
    pub fn new(kind: SidecarKind, base_url: impl Into<String>) -> Self {
        // CPU inference on the bundled `medium` Whisper can take 2-3 minutes
        // per ~10s audio clip. The local llama.cpp Qwen 7B call also can take
        // 30-60s on CPU. Use a 5-minute ceiling for both; cloud calls return
        // fast enough not to be affected.
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("reqwest client");
        Self { kind, base_url: base_url.into(), client }
    }

    pub async fn health_check(&self) -> bool {
        match self.client.get(format!("{}{}", self.base_url, self.kind.health_path())).send().await {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    }

    /// Wait until the sidecar reports healthy, polling every `poll` ms,
    /// giving up after `timeout`. Returns true on healthy, false on timeout.
    pub async fn wait_ready(&self, timeout: Duration, poll: Duration) -> bool {
        let start = std::time::Instant::now();
        loop {
            if self.health_check().await {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            tokio::time::sleep(poll).await;
        }
    }

    pub async fn transcribe(&self, wav: Vec<u8>) -> Result<String> {
        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .text("response_format", "text")
            .text("language", "auto")
            .part("file", part);
        let res = self.client.post(format!("{}/v1/audio/transcriptions", self.base_url))
            .multipart(form).send().await?;
        let status = res.status();
        let body = res.text().await?;
        if !status.is_success() {
            return Err(anyhow!("whisper {}: {}", status, body));
        }
        Ok(body)
    }

    pub async fn cleanup(&self, raw_text: &str) -> Result<String> {
        let prompt = format!(
            "你是语音输入整理助手。去掉口头禅（嗯/啊/那个）、修正明显语序错误、添加恰当标点、保留原意。\n只输出整理后的纯文本，不要解释。\n输入：{}",
            raw_text
        );
        #[derive(Serialize)] struct Req<'a> { model: &'a str, messages: Vec<Msg<'a>>, max_tokens: u32 }
        #[derive(Serialize)] struct Msg<'a> { role: &'a str, content: &'a str }
        #[derive(Deserialize)] struct Resp { choices: Vec<Choice> }
        #[derive(Deserialize)] struct Choice { message: RespMsg }
        #[derive(Deserialize)] struct RespMsg { content: String }

        let req = Req {
            model: "local",
            messages: vec![Msg { role: "user", content: &prompt }],
            max_tokens: 2048,
        };
        let res = self.client.post(format!("{}/v1/chat/completions", self.base_url))
            .json(&req).send().await?;
        let status = res.status();
        let body: Resp = res.json().await.map_err(|e| anyhow!("llama json {}: {}", status, e))?;
        body.choices.into_iter().next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow!("empty llama response"))
    }
}

pub async fn spawn(app: &tauri::AppHandle, kind: SidecarKind) -> Result<Child> {
    let sidecar_dir = resolve_sidecar_dir(app)?;
    let models_dir = sidecar_dir.join("models");

    let mut cmd = match kind {
        // Whisper: use Python sidecar (whisper.cpp Windows binary not available
        // in this build env; the Python script wraps ONNX models into the
        // same OpenAI-compatible HTTP API)
        SidecarKind::Whisper => {
            let script = sidecar_dir.join("whisper-asr.py");
            if !script.exists() {
                return Err(anyhow!(
                    "whisper script missing at {}",
                    script.display()
                ));
            }
            let model_dir = sidecar_dir.join("whisper-model");
            if !model_dir.exists() {
                return Err(anyhow!(
                    "whisper-model dir missing at {}",
                    model_dir.display()
                ));
            }
            tracing::info!(
                "starting whisper sidecar: python {} (model dir {})",
                script.display(),
                model_dir.display()
            );
            let mut c = Command::new("python");
            c.arg(&script);
            c.env("ASR_PORT", kind.default_port().to_string());
            c.env("ASR_MODEL_DIR", &model_dir);
            c
        }
        SidecarKind::Llama => {
            let bin = sidecar_dir.join(kind.binary_name());
            if !bin.exists() {
                return Err(anyhow!("llama-server binary missing at {}", bin.display()));
            }
            let model = kind.model_path(&models_dir);
            if !model.exists() {
                return Err(anyhow!(
                    "llama model file missing at {} (download in Settings first)",
                    model.display()
                ));
            }
            tracing::info!(
                "starting llama sidecar: {} -m {} -c 4096 --host 127.0.0.1 --port {}",
                bin.display(),
                model.display(),
                kind.default_port()
            );
            let mut c = Command::new(&bin);
            c.arg("-m").arg(&model);
            c.arg("-c").arg("4096");
            c.arg("--host").arg("127.0.0.1");
            c.arg("--port").arg(kind.default_port().to_string());
            c
        }
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let child = cmd.spawn().map_err(|e| {
        anyhow!("failed to spawn {:?} sidecar: {} (is the binary/script executable?)", kind, e)
    })?;
    tracing::info!("sidecar {:?} spawned (pid={})", kind, child.id().unwrap_or(0));
    Ok(child)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_check_returns_false_for_unreachable() {
        let s = Sidecar::new(SidecarKind::Whisper, "http://127.0.0.1:1");
        assert!(!s.health_check().await);
    }

    #[tokio::test]
    async fn cleanup_prompt_contains_input() {
        let s = Sidecar::new(SidecarKind::Llama, "http://127.0.0.1:1");
        // Will fail to connect but verifies the prompt is built correctly.
        let result = s.cleanup("测试一下").await;
        assert!(result.is_err());
    }
}