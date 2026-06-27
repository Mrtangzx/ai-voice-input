use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub name: String,
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

impl ModelSpec {
    pub fn whisper_medium() -> Self {
        Self {
            name: "whisper-medium".into(),
            filename: "ggml-medium.bin".into(),
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin".into(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            size_bytes: 1_500_000_000,
        }
    }
    pub fn qwen_7b_q4() -> Self {
        Self {
            name: "qwen2.5-7b-instruct-q4".into(),
            filename: "qwen2.5-7b-instruct-q4.gguf".into(),
            // modelscope.cn mirror (HF/GitHub blocked in this environment)
            url: "https://www.modelscope.cn/qwen/Qwen2.5-7B-Instruct-GGUF/resolve/master/qwen2.5-7b-instruct-q4_k_m.gguf".into(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            size_bytes: 4_683_073_536,
        }
    }
    pub fn filename_path(&self, models_dir: &Path) -> PathBuf { models_dir.join(&self.filename) }
}

pub fn manifest_path(models_dir: &Path) -> PathBuf { models_dir.join("manifest.json") }

async fn ensure_dir(p: &Path) -> Result<()> { tokio::fs::create_dir_all(p).await.map_err(Into::into) }

async fn sha256_file(p: &Path) -> Result<String> {
    use tokio::io::AsyncReadExt;
    let mut hasher = Sha256::new();
    let mut file = tokio::fs::File::open(p).await?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub async fn download_one(app: &AppHandle, spec: ModelSpec) -> Result<()> {
    let models_dir = app.path().resource_dir()?.join("sidecar").join("models");
    ensure_dir(&models_dir).await?;
    let target = spec.filename_path(&models_dir);

    // Skip if already present (and sha is not the placeholder all-zeros)
    if target.exists() {
        if let Ok(digest) = sha256_file(&target).await {
            if digest == spec.sha256 && !spec.sha256.chars().all(|c| c == '0') {
                return Ok(());
            }
        }
    }

    let client = reqwest::Client::new();
    let res = client.get(&spec.url).send().await?.error_for_status()?;
    let total = res.content_length().unwrap_or(spec.size_bytes);
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();
    let mut file = tokio::fs::File::create(&target).await?;
    let mut hasher = Sha256::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        let pct = (downloaded as f64 / total as f64) * 100.0;
        let _ = app.emit("model-download-progress", serde_json::json!({
            "name": spec.name, "downloaded": downloaded, "total": total, "percent": pct,
        }));
    }

    let digest = hex::encode(hasher.finalize());
    if !spec.sha256.chars().all(|c| c == '0') && digest != spec.sha256 {
        let _ = tokio::fs::remove_file(&target).await;
        return Err(anyhow!("sha256 mismatch: expected {} got {}", spec.sha256, digest));
    }

    let manifest = serde_json::json!([spec]);
    tokio::fs::write(manifest_path(&models_dir), serde_json::to_vec_pretty(&manifest)?).await?;
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct ModelStatus {
    pub whisper_installed: bool,
    pub llama_installed: bool,
}

#[tauri::command]
pub async fn status(app: AppHandle) -> Result<ModelStatus, String> {
    let models_dir = app.path().resource_dir()
        .map_err(|e| e.to_string())?.join("sidecar").join("models");
    let whisper_installed = ModelSpec::whisper_medium().filename_path(&models_dir).exists();
    let llama_installed = ModelSpec::qwen_7b_q4().filename_path(&models_dir).exists();
    Ok(ModelStatus { whisper_installed, llama_installed })
}

#[tauri::command]
pub async fn download(app: AppHandle) -> Result<(), String> {
    download_one(&app, ModelSpec::whisper_medium()).await.map_err(|e| e.to_string())?;
    download_one(&app, ModelSpec::qwen_7b_q4()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_specs_have_expected_filenames() {
        assert_eq!(ModelSpec::whisper_medium().filename, "ggml-medium.bin");
        assert!(ModelSpec::qwen_7b_q4().filename.contains("qwen"));
    }

    #[test]
    fn manifest_path_is_inside_models_dir() {
        let p = manifest_path(&std::path::PathBuf::from("/tmp/x"));
        assert!(p.ends_with("models/manifest.json"));
    }

    #[test]
    fn filename_path_resolves_under_dir() {
        let spec = ModelSpec::whisper_medium();
        let p = spec.filename_path(&std::path::PathBuf::from("/data"));
        assert_eq!(p, std::path::PathBuf::from("/data/ggml-medium.bin"));
    }
}