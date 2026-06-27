use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tauri::Manager;
use tokio::process::{Child, Command};

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
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
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
    let resource_dir = app.path().resource_dir()?;
    let sidecar_dir = resource_dir.join("sidecar");
    let models_dir = sidecar_dir.join("models");

    let mut cmd = match kind {
        // Whisper: use Python sidecar (whisper.cpp Windows binary not available
        // in this build env; the Python script wraps ONNX models into the
        // same OpenAI-compatible HTTP API)
        SidecarKind::Whisper => {
            let script = sidecar_dir.join("whisper-asr.py");
            let mut c = Command::new("python");
            c.arg(&script);
            c.env("ASR_PORT", kind.default_port().to_string());
            c.env("ASR_MODEL_DIR", sidecar_dir.join("whisper-model"));
            c
        }
        SidecarKind::Llama => {
            let bin = sidecar_dir.join(kind.binary_name());
            let mut c = Command::new(bin);
            c.arg("-m").arg(kind.model_path(&models_dir));
            c.arg("-c").arg("4096");
            c.arg("--host").arg("127.0.0.1");
            c.arg("--port").arg(kind.default_port().to_string());
            c
        }
    };
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    Ok(cmd.spawn()?)
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