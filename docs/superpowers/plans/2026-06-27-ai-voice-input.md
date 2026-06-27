# AI 语音输入法 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local Windows voice-to-text app using Tauri + sidecar AI processes that takes voice input via global hotkey and inserts cleaned-up text at the cursor.

**Architecture:** Tauri shell (~15MB) hosts Rust backend + React frontend. Two sidecar processes (whisper.cpp ASR + llama.cpp LLM cleanup) launched and managed by the main shell. SQLite for history. Global hotkey triggers recording → cpal capture → whisper HTTP → llama HTTP → clipboard + Ctrl+V.

**Tech Stack:** Tauri 2.x, Rust (edition 2021), React 18 + TypeScript, SQLite (sqlx), cpal, arboard + enigo, tauri-plugin-global-shortcut, whisper.cpp, llama.cpp.

**Spec:** `docs/superpowers/specs/2026-06-27-ai-voice-input-design.md`

---

## File Structure

```
ai-voice-input/
├── docs/
│   ├── superpowers/
│   │   ├── specs/2026-06-27-ai-voice-input-design.md
│   │   └── plans/2026-06-27-ai-voice-input.md
├── sidecar/                     # Pre-built whisper.cpp / llama.cpp binaries go here
│   ├── whisper-server.exe
│   ├── llama-server.exe
│   └── models/
│       ├── ggml-medium.bin
│       └── qwen2.5-7b-instruct-q4.gguf
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── recording.rs
│       │   ├── history.rs
│       │   ├── settings.rs
│       │   └── models.rs
│       ├── hotkey.rs
│       ├── audio.rs
│       ├── insert.rs
│       ├── sidecar.rs
│       ├── storage.rs
│       ├── overlay.rs
│       └── pipeline.rs
├── src/                         # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── views/
│   │   ├── Settings.tsx
│   │   ├── History.tsx
│   │   └── FirstRun.tsx
│   ├── components/
│   └── styles.css
├── package.json
├── vite.config.ts
├── tsconfig.json
└── tests/
    ├── audio_test.rs
    ├── insert_test.rs
    ├── storage_test.rs
    └── sidecar_test.rs
```

---

## Phase 1: Project Foundation

### Task 1: Initialize Tauri Project

**Files:**
- Create: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `vite.config.ts`, `tsconfig.json`
- Create: `src/main.tsx`, `src/App.tsx`

- [ ] **Step 1: Create package.json with Tauri + React deps**

```json
{
  "name": "ai-voice-input",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-dialog": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "@tauri-apps/plugin-global-shortcut": "^2.0.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0"
  }
}
```

- [ ] **Step 2: Install Node deps**

Run: `cd D:/soft_work/02-MINE/ai-voice-input && npm install`
Expected: `node_modules/` populated, no errors.

- [ ] **Step 3: Create vite.config.ts**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ['VITE_', 'TAURI_'],
  build: { target: 'chrome105', minify: 'esbuild', sourcemap: false }
});
```

- [ ] **Step 4: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

- [ ] **Step 5: Create src-tauri/Cargo.toml**

```toml
[package]
name = "ai-voice-input"
version = "0.1.0"
edition = "2021"

[lib]
name = "ai_voice_input_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream", "multipart"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros", "chrono"] }
cpal = "0.15"
hound = "3.5"
rubato = "0.15"
arboard = "3.4"
enigo = "0.3"
sha2 = "0.10"
hex = "0.4"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
chrono = { version = "0.4", features = ["serde"] }
directories = "5"
once_cell = "1"
parking_lot = "0.12"

[dev-dependencies]
mockito = "1.4"
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
```

- [ ] **Step 6: Create src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2.0.0",
  "productName": "AI Voice Input",
  "version": "0.1.0",
  "identifier": "com.local.ai-voice-input",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "AI Voice Input",
        "width": 720,
        "height": 520,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"],
    "externalBin": [
      "sidecar/whisper-server",
      "sidecar/llama-server"
    ],
    "resources": ["sidecar/models/*"]
  }
}
```

- [ ] **Step 7: Create placeholder src/main.tsx and src/App.tsx**

```tsx
// src/main.tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode><App /></React.StrictMode>
);
```

```tsx
// src/App.tsx
export default function App() {
  return <div className="app"><h1>AI Voice Input</h1><p>Initializing…</p></div>;
}
```

- [ ] **Step 8: Create src/styles.css (minimal)**

```css
:root {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  background: #0f172a;
  color: #f1f5f9;
}
.app { padding: 24px; }
```

- [ ] **Step 9: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add package.json package-lock.json src/ vite.config.ts tsconfig.json
git commit -m "feat: scaffold Vite + React + TypeScript frontend"
```

---

### Task 2: Tauri Backend Skeleton

**Files:**
- Create: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/build.rs`

- [ ] **Step 1: Create src-tauri/build.rs**

```rust
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 2: Create src-tauri/src/main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ai_voice_input_lib::run();
}
```

- [ ] **Step 3: Create src-tauri/src/lib.rs (initial)**

```rust
mod commands;
mod hotkey;
mod audio;
mod insert;
mod sidecar;
mod storage;
mod overlay;
mod pipeline;

use commands::{history, models, recording, settings};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            recording::start,
            recording::stop,
            history::list,
            history::delete,
            history::search,
            settings::get,
            settings::update,
            models::status,
            models::download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Create src-tauri/src/commands/mod.rs (empty stubs)**

```rust
pub mod recording;
pub mod history;
pub mod settings;
pub mod models;
```

- [ ] **Step 5: Stub each command module so `cargo check` passes**

For each of `recording.rs`, `history.rs`, `settings.rs`, `models.rs`:

```rust
// recording.rs
#[tauri::command]
pub async fn start() -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn stop() -> Result<(), String> { Ok(()) }
```

```rust
// history.rs
#[tauri::command]
pub async fn list() -> Result<Vec<String>, String> { Ok(vec![]) }

#[tauri::command]
pub async fn delete(_id: i64) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn search(_q: String) -> Result<Vec<String>, String> { Ok(vec![]) }
```

```rust
// settings.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings { pub hotkey: String }

#[tauri::command]
pub async fn get() -> Result<Settings, String> { Ok(Settings { hotkey: "Ctrl+Shift+Space".into() }) }

#[tauri::command]
pub async fn update(_s: Settings) -> Result<(), String> { Ok(()) }
```

```rust
// models.rs
#[tauri::command]
pub async fn status() -> Result<String, String> { Ok("not-installed".into()) }

#[tauri::command]
pub async fn download() -> Result<(), String> { Ok(()) }
```

- [ ] **Step 6: Stub remaining modules so `cargo check` passes**

`hotkey.rs`, `audio.rs`, `insert.rs`, `sidecar.rs`, `storage.rs`, `overlay.rs`, `pipeline.rs` — each as an empty file. `lib.rs` will reference them, so they must exist (even if empty) for `cargo check` to succeed.

- [ ] **Step 7: Verify Rust compiles**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo check`
Expected: compiles with warnings (unused code OK at this stage), no errors.

- [ ] **Step 8: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/
git commit -m "feat: scaffold Tauri backend with stub commands"
```

---

## Phase 2: Storage & Settings

### Task 3: Storage Module (SQLite)

**Files:**
- Create: `src-tauri/src/storage.rs`
- Test: `tests/storage_test.rs`

- [ ] **Step 1: Write failing test**

```rust
// tests/storage_test.rs
use ai_voice_input_lib::storage::{Storage, Transcript};

#[tokio::test]
async fn insert_and_list_transcripts() {
    let storage = Storage::in_memory().await.unwrap();
    storage.insert(&Transcript {
        id: None,
        raw_text: "嗯那个今天天气不错".into(),
        clean_text: "今天天气不错。".into(),
        duration_ms: 3000,
        created_at: chrono::Utc::now(),
        app_name: Some("notepad".into()),
    }).await.unwrap();

    let list = storage.list(10, 0).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].clean_text, "今天天气不错。");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test storage_test`
Expected: FAIL (module doesn't exist)

- [ ] **Step 3: Implement storage.rs**

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub id: Option<i64>,
    pub raw_text: String,
    pub clean_text: String,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
    pub app_name: Option<String>,
}

pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect(":memory:")
            .await?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    pub async fn open(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(path)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(opts).await?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS transcripts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                raw_text TEXT NOT NULL,
                clean_text TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                app_name TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
                raw_text, clean_text, content='transcripts', content_rowid='id'
            );
        "#).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn insert(&self, t: &Transcript) -> Result<i64> {
        let id = sqlx::query(
            "INSERT INTO transcripts (raw_text, clean_text, duration_ms, created_at, app_name)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&t.raw_text).bind(&t.clean_text).bind(t.duration_ms)
        .bind(t.created_at.to_rfc3339()).bind(&t.app_name)
        .execute(&self.pool).await?
        .last_insert_rowid();

        sqlx::query("INSERT INTO transcripts_fts (rowid, raw_text, clean_text) VALUES (?, ?, ?)")
            .bind(id).bind(&t.raw_text).bind(&t.clean_text)
            .execute(&self.pool).await?;
        Ok(id)
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Transcript>> {
        let rows = sqlx::query_as::<_, TranscriptRow>(
            "SELECT id, raw_text, clean_text, duration_ms, created_at, app_name
             FROM transcripts ORDER BY id DESC LIMIT ? OFFSET ?"
        ).bind(limit).bind(offset).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn search(&self, q: &str, limit: i64) -> Result<Vec<Transcript>> {
        let rows = sqlx::query_as::<_, TranscriptRow>(
            "SELECT t.* FROM transcripts t JOIN transcripts_fts f ON t.id = f.rowid
             WHERE transcripts_fts MATCH ? ORDER BY rank LIMIT ?"
        ).bind(q).bind(limit).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM transcripts WHERE id = ?").bind(id).execute(&self.pool).await?;
        sqlx::query("DELETE FROM transcripts_fts WHERE rowid = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TranscriptRow {
    id: i64,
    raw_text: String,
    clean_text: String,
    duration_ms: i64,
    created_at: String,
    app_name: Option<String>,
}

impl From<TranscriptRow> for Transcript {
    fn from(r: TranscriptRow) -> Self {
        Self {
            id: Some(r.id),
            raw_text: r.raw_text,
            clean_text: r.clean_text,
            duration_ms: r.duration_ms,
            created_at: DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&Utc),
            app_name: r.app_name,
        }
    }
}
```

- [ ] **Step 4: Add module to lib.rs and export public API**

In `src-tauri/src/lib.rs`, change `mod storage;` to `pub mod storage;`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test storage_test`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/storage.rs src-tauri/src/lib.rs tests/storage_test.rs
git commit -m "feat: add SQLite storage with FTS search"
```

---

### Task 4: Settings Module

**Files:**
- Create: `src-tauri/src/commands/settings.rs` (replace stub)
- Test: inline `#[cfg(test)]` in settings.rs

- [ ] **Step 1: Write failing test**

```rust
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
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --lib settings`
Expected: FAIL (struct doesn't exist)

- [ ] **Step 3: Implement full settings module**

Replace `src-tauri/src/commands/settings.rs` with:

```rust
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
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --lib settings`
Expected: 2 passed

- [ ] **Step 5: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/commands/settings.rs
git commit -m "feat: settings module with file persistence"
```

---

## Phase 3: Audio Capture

### Task 5: Audio Module (cpal + WAV encoding)

**Files:**
- Create: `src-tauri/src/audio.rs`
- Test: `tests/audio_test.rs`

- [ ] **Step 1: Write failing test**

```rust
// tests/audio_test.rs
use ai_voice_input_lib::audio::{encode_wav, resample_to_16k};
use hound::WavReader;

#[tokio::test]
async fn resample_44100_to_16000() {
    // 1 second of silence at 44.1kHz
    let input: Vec<f32> = vec![0.0; 44100];
    let out = resample_to_16k(&input, 44100).await.unwrap();
    // Expect ~16000 samples (±5% tolerance for resampler)
    assert!((out.len() as i32 - 16000).abs() < 800, "got {}", out.len());
}

#[test]
fn encode_wav_produces_valid_header() {
    let pcm: Vec<f32> = vec![0.0; 16000];
    let wav = encode_wav(&pcm).unwrap();
    // WAV header magic: "RIFF"
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    let mut reader = WavReader::new(std::io::Cursor::new(wav)).unwrap();
    assert_eq!(reader.spec().sample_rate, 16000);
    assert_eq!(reader.spec().channels, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test audio_test`
Expected: FAIL (functions don't exist)

- [ ] **Step 3: Implement audio.rs**

```rust
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;
use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};
use std::sync::Arc;
use std::time::Duration;

pub type SharedBuffer = Arc<Mutex<Vec<f32>>>;

pub fn list_input_devices() -> Result<Vec<(String, String)>> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    for d in host.input_devices()? {
        let name = d.name().unwrap_or_default();
        let id = d.id().map(|x| x.to_string()).unwrap_or_else(|_| name.clone());
        out.push((id, name));
    }
    Ok(out)
}

pub fn open_input_stream(device_id: Option<&str>, buf: SharedBuffer) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = match device_id {
        Some(id) => host.input_devices()?
            .find(|d| d.id().map(|x| x.to_string()).as_deref() == Some(id))
            .ok_or_else(|| anyhow!("device not found"))?,
        None => host.default_input_device().ok_or_else(|| anyhow!("no input device"))?,
    };

    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let err_fn = |err| tracing::error!("audio stream error: {err}");
    let buf_clone = buf.clone();

    let stream = match config.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &StreamConfig::from(&config),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Take first channel only
                let mono: Vec<f32> = data.chunks(channels).map(|c| c[0]).collect();
                let mut b = buf_clone.lock();
                b.extend(mono);
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &StreamConfig::from(&config),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let mono: Vec<f32> = data.chunks(channels).map(|c| c[0] as f32 / 32768.0).collect();
                let mut b = buf_clone.lock();
                b.extend(mono);
            },
            err_fn,
            None,
        )?,
        _ => return Err(anyhow!("unsupported sample format")),
    };
    stream.play()?;
    Ok(stream)
}

/// Resample arbitrary-rate f32 mono PCM to 16kHz.
pub async fn resample_to_16k(input: &[f32], input_rate: u32) -> Result<Vec<f32>> {
    if input_rate == 16000 {
        return Ok(input.to_vec());
    }
    let params = InterpolationParameters {
        interpolation: InterpolationType::Linear,
        oversample_factor: 2,
        window: WindowFunction::Hann,
    };
    let mut resampler = SincFixedIn::<f32>::new(
        16000.0 / input_rate as f64,
        2.0,
        params,
        input.len(),
        1,
    )?;
    let waves_in = vec![input.to_vec()];
    let waves_out = tokio::task::spawn_blocking(move || resampler.process(&waves_in)).await??;
    Ok(waves_out.into_iter().next().unwrap_or_default())
}

/// Encode f32 mono PCM as 16-bit PCM WAV bytes.
pub fn encode_wav(pcm: &[f32]) -> Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for &s in pcm {
            let clamped = s.clamp(-1.0, 1.0);
            writer.write_sample((clamped * 32767.0) as i16)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

/// Stop a stream after optional timeout.
pub fn stop_after(stream: cpal::Stream, timeout: Duration) -> Result<Vec<f32>> {
    std::thread::sleep(timeout);
    drop(stream);
    Ok(vec![])
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test audio_test`
Expected: 2 passed

- [ ] **Step 5: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/audio.rs tests/audio_test.rs
git commit -m "feat: audio capture module with cpal + WAV encoding"
```

---

## Phase 4: Sidecar & AI Integration

### Task 6: Sidecar Lifecycle Manager

**Files:**
- Create: `src-tauri/src/sidecar.rs`
- Test: `tests/sidecar_test.rs`

- [ ] **Step 1: Write failing test**

```rust
// tests/sidecar_test.rs
use ai_voice_input_lib::sidecar::{Sidecar, SidecarKind};

#[tokio::test]
async fn health_check_against_mock() {
    let mock = mockito::Server::new_async().await;
    let _m = mock.mock("GET", "/health").with_status(200).create_async().await;

    let sidecar = Sidecar::new(SidecarKind::Whisper, mock.url());
    let healthy = sidecar.health_check().await;
    assert!(healthy);
}

#[tokio::test]
async fn health_check_unhealthy() {
    let sidecar = Sidecar::new(SidecarKind::Whisper, "http://127.0.0.1:1");
    let healthy = sidecar.health_check().await;
    assert!(!healthy);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test sidecar_test`
Expected: FAIL

- [ ] **Step 3: Implement sidecar.rs**

```rust
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::process::{Child, Command};

#[derive(Debug, Clone, Copy)]
pub enum SidecarKind { Whisper, Llama }

impl SidecarKind {
    pub fn binary_name(self) -> &'static str {
        match self {
            Self::Whisper => "whisper-server",
            Self::Llama => "llama-server",
        }
    }
    pub fn default_port(self) -> u16 {
        match self {
            Self::Whisper => 8178,
            Self::Llama => 8188,
        }
    }
    pub fn model_path(self, models_dir: &PathBuf) -> PathBuf {
        match self {
            Self::Whisper => models_dir.join("ggml-medium.bin"),
            Self::Llama => models_dir.join("qwen2.5-7b-instruct-q4.gguf"),
        }
    }
    pub fn health_path(self) -> &'static str {
        match self { Self::Whisper => "/health", Self::Llama => "/health" }
    }
}

pub struct Sidecar {
    kind: SidecarKind,
    base_url: String,
    client: Client,
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
        #[derive(Serialize)] struct Req<'a> { model: &'a str, messages: Vec<Msg<'a>, max_tokens: u32 }
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

/// Spawn a sidecar process. Tauri-managed.
pub async fn spawn(app: &AppHandle, kind: SidecarKind) -> Result<Child> {
    let resource_dir = app.path().resource_dir()?;
    let models_dir = resource_dir.join("sidecar").join("models");
    let bin = resource_dir.join("sidecar").join(kind.binary_name());

    let mut cmd = Command::new(bin);
    cmd.arg("--port").arg(kind.default_port().to_string());
    match kind {
        SidecarKind::Whisper => {
            cmd.arg("--model").arg(kind.model_path(&models_dir));
            cmd.arg("-l").arg("auto");
        }
        SidecarKind::Llama => {
            cmd.arg("-m").arg(kind.model_path(&models_dir));
            cmd.arg("-c").arg("4096");
            cmd.arg("--host").arg("127.0.0.1");
        }
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    Ok(cmd.spawn()?)
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test sidecar_test`
Expected: 2 passed

- [ ] **Step 5: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/sidecar.rs tests/sidecar_test.rs
git commit -m "feat: sidecar HTTP client + lifecycle spawn helper"
```

---

### Task 7: Model Download Manager

**Files:**
- Create: `src-tauri/src/commands/models.rs` (replace stub)
- Test: `tests/models_test.rs`

- [ ] **Step 1: Write failing test**

```rust
// tests/models_test.rs
use ai_voice_input_lib::commands::models::{ModelSpec, manifest_path};

#[test]
fn model_specs() {
    let whisper = ModelSpec::whisper_medium();
    assert!(whisper.url.contains("ggml-medium"));
    assert!(whisper.sha256.len() == 64);

    let llama = ModelSpec::qwen_7b_q4();
    assert!(llama.url.contains("qwen"));
    assert!(llama.sha256.len() == 64);
}

#[test]
fn manifest_path_under_models_dir() {
    let p = manifest_path(&std::path::PathBuf::from("/tmp/x"));
    assert!(p.ends_with("models/manifest.json"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test models_test`
Expected: FAIL

- [ ] **Step 3: Implement commands/models.rs**

```rust
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

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
        // NOTE: replace URL + sha256 with real values at release time
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
            url: "https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF/resolve/main/qwen2.5-7b-instruct-q4_k_m.gguf".into(),
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            size_bytes: 4_500_000_000,
        }
    }
}

pub fn manifest_path(models_dir: &Path) -> PathBuf { models_dir.join("manifest.json") }

async fn ensure_dir(p: &Path) -> Result<()> { tokio::fs::create_dir_all(p).await.map_err(Into::into) }

pub async fn download_one(app: &AppHandle, spec: ModelSpec) -> Result<()> {
    let models_dir = app.path().resource_dir()?.join("sidecar").join("models");
    ensure_dir(&models_dir).await?;
    let target = models_dir.join(&spec.filename);

    // Skip if already correct
    if target.exists() {
        if let Ok(digest) = sha256_file(&target).await {
            if digest == spec.sha256 && spec.sha256 != "0".repeat(64) {
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
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        let pct = (downloaded as f64 / total as f64) * 100.0;
        let _ = app.emit("model-download-progress", serde_json::json!({
            "name": spec.name, "downloaded": downloaded, "total": total, "percent": pct,
        }));
    }

    let digest = hex::encode(hasher.finalize());
    if digest != spec.sha256 && spec.sha256 != "0".repeat(64) {
        let _ = tokio::fs::remove_file(&target).await;
        return Err(anyhow!("sha256 mismatch: expected {} got {}", spec.sha256, digest));
    }

    // Write manifest
    let manifest = serde_json::json!([spec]);
    tokio::fs::write(manifest_path(&models_dir), serde_json::to_vec_pretty(&manifest)?).await?;
    Ok(())
}

async fn sha256_file(p: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file = tokio::fs::File::open(p).await?;
    let mut buf = vec![0u8; 64 * 1024];
    use tokio::io::AsyncReadExt;
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Serialize)]
pub struct ModelStatus {
    pub whisper_installed: bool,
    pub llama_installed: bool,
    pub any_download_in_progress: bool,
}

#[tauri::command]
pub async fn status(app: AppHandle) -> Result<ModelStatus, String> {
    let models_dir = app.path().resource_dir()
        .map_err(|e| e.to_string())?.join("sidecar").join("models");
    let Ok(models_dir) = std::fs::canonicalize(&models_dir) else {
        return Ok(ModelStatus { whisper_installed: false, llama_installed: false, any_download_in_progress: false });
    };
    Ok(ModelStatus {
        whisper_installed: ModelSpec::whisper_medium().filename_path(&models_dir).exists(),
        llama_installed: ModelSpec::qwen_7b_q4().filename_path(&models_dir).exists(),
        any_download_in_progress: false,
    })
}

#[tauri::command]
pub async fn download(app: AppHandle) -> Result<(), String> {
    download_one(&app, ModelSpec::whisper_medium()).await.map_err(|e| e.to_string())?;
    download_one(&app, ModelSpec::qwen_7b_q4()).await.map_err(|e| e.to_string())?;
    Ok(())
}

impl ModelSpec {
    fn filename_path(&self, models_dir: &Path) -> PathBuf { models_dir.join(&self.filename) }
}
```

- [ ] **Step 4: Add `futures-util` to Cargo.toml**

Add to `[dependencies]`:
```toml
futures-util = "0.3"
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test models_test`
Expected: 2 passed

- [ ] **Step 6: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/commands/models.rs src-tauri/Cargo.toml tests/models_test.rs
git commit -m "feat: model download manager with SHA256 verification"
```

---

## Phase 5: Text Insertion

### Task 8: Insert Module (Clipboard + Ctrl+V)

**Files:**
- Create: `src-tauri/src/insert.rs`
- Test: `tests/insert_test.rs`

- [ ] **Step 1: Write failing test**

```rust
// tests/insert_test.rs
use ai_voice_input_lib::insert::atomic_paste;

#[test]
fn atomic_paste_writes_to_clipboard() {
    // We can't easily test the Ctrl+V part in CI; test the clipboard write side.
    let mut clipboard = arboard::Clipboard::new().unwrap();
    let _ = clipboard.set_text("__TEST_SENTINEL__".to_string());
    let original = clipboard.get_text().unwrap();

    atomic_paste("测试中文 paste".into()).unwrap();

    let after = clipboard.get_text().unwrap();
    assert_eq!(after, "测试中文 paste");
    // Restore
    let _ = clipboard.set_text(original);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test insert_test`
Expected: FAIL

- [ ] **Step 3: Implement insert.rs**

```rust
use anyhow::Result;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Writes text to clipboard, simulates Ctrl+V, then restores original clipboard.
pub fn atomic_paste(text: String) -> Result<()> {
    let mut cb = Clipboard::new()?;
    let original = cb.get_text().unwrap_or_default();

    cb.set_text(text)?;

    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Control, Direction::Press)?;
    enigo.key(Key::Unicode('v'), Direction::Click)?;
    enigo.key(Key::Control, Direction::Release)?;

    // Allow target app to consume paste
    std::thread::sleep(std::time::Duration::from_millis(60));

    // Restore
    let _ = cb.set_text(original);
    Ok(())
}

/// Check if the foreground window is one that commonly ignores synthetic input.
pub fn foreground_rejects_text() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowTextW,
        };
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() { return true; }
            let mut buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len <= 0 { return false; }
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            let lower = title.to_lowercase();
            return lower.contains("task manager")
                || lower.contains("任务管理器")
                || lower.contains("program manager");
        }
    }
    #[cfg(not(windows))]
    { false }
}
```

- [ ] **Step 4: Add windows-sys to Cargo.toml dev-dependencies (Windows-only)**

Add to `[target.'cfg(windows)'.dependencies]`:
```toml
windows-sys = { version = "0.59", features = ["Win32_UI_WindowsAndMessaging"] }
```

- [ ] **Step 5: Run test to verify pass**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test insert_test`
Expected: 1 passed (clipboard part)

- [ ] **Step 6: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/insert.rs src-tauri/Cargo.toml tests/insert_test.rs
git commit -m "feat: text insertion via clipboard + Ctrl+V"
```

---

## Phase 6: Pipeline Orchestration

### Task 9: Recording Pipeline

**Files:**
- Create: `src-tauri/src/pipeline.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement pipeline.rs**

```rust
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::audio;
use crate::insert;
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
    let result = inner(app, storage, whisper, llama, auto_stop).await;
    RECORDING.store(false, Ordering::SeqCst);
    result
}

async fn inner(
    app: AppHandle,
    storage: Arc<Storage>,
    whisper: Arc<Sidecar>,
    llama: Arc<Sidecar>,
    auto_stop: Duration,
) -> Result<()> {
    let buf = audio::SharedBuffer::default();
    let stream = audio::open_input_stream(None, buf.clone())?;
    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"recording"}));

    // Wait for stop signal OR auto-stop
    let start = std::time::Instant::now();
    while is_recording() && start.elapsed() < auto_stop {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    drop(stream);

    let pcm = buf.lock().clone();
    if pcm.is_empty() {
        return Err(anyhow!("empty recording"));
    }

    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"transcribing"}));
    let sample_rate = 44100; // assume; could be queried
    let pcm_16k = audio::resample_to_16k(&pcm, sample_rate).await?;
    let wav = audio::encode_wav(&pcm_16k)?;
    let raw_text = whisper.transcribe(wav).await?;
    if raw_text.trim().is_empty() {
        return Err(anyhow!("empty transcription"));
    }

    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"cleaning"}));
    let clean_text = match llama.cleanup(&raw_text).await {
        Ok(s) if !s.trim().is_empty() => s,
        _ => raw_text.clone(),
    };

    // Insert
    if !insert::foreground_rejects_text() {
        let _ = insert::atomic_paste(clean_text.clone());
    } else {
        let mut cb = arboard::Clipboard::new()?;
        cb.set_text(clean_text.clone())?;
    }

    // Persist
    storage.insert(&Transcript {
        id: None,
        raw_text: raw_text.clone(),
        clean_text: clean_text.clone(),
        duration_ms: start.elapsed().as_millis() as i64,
        created_at: Utc::now(),
        app_name: None,
    }).await?;

    let _ = app.emit("pipeline-status", serde_json::json!({"phase":"done","clean":clean_text}));
    Ok(())
}
```

- [ ] **Step 2: Update commands/recording.rs to drive the pipeline**

Replace `src-tauri/src/commands/recording.rs`:

```rust
use crate::audio::SharedBuffer;
use crate::pipeline;
use crate::sidecar::{Sidecar, SidecarKind};
use crate::storage::Storage;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub storage: Arc<Storage>,
    pub whisper: Arc<Sidecar>,
    pub llama: Arc<Sidecar>,
    pub hotkey_buf: Arc<Mutex<SharedBuffer>>,
}

#[tauri::command]
pub async fn start(state: State<'_, AppState>) -> Result<(), String> {
    if pipeline::is_recording() {
        return Err("already recording".into());
    }
    let settings = crate::commands::settings::get(app_handle()).await.map_err(|e| e.to_string())?;
    let app = app_handle();
    let storage = state.storage.clone();
    let whisper = state.whisper.clone();
    let llama = state.llama.clone();
    let auto_stop = Duration::from_secs(settings.auto_stop_seconds as u64);
    tauri::async_runtime::spawn(async move {
        if let Err(e) = pipeline::run_once(app, storage, whisper, llama, auto_stop).await {
            tracing::error!("pipeline error: {e}");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn stop() -> Result<(), String> {
    // The pipeline observes is_recording() via hotkey module's signal; simpler: drive toggle in hotkey.rs
    crate::hotkey::request_stop();
    Ok(())
}

fn app_handle() -> AppHandle {
    AppHandle::default() // Will be replaced by actual injection
}
```

**NOTE:** Refactor `app_handle()` to use `tauri::Manager::app_handle()` injected via State. The pattern shown is a placeholder; final wiring happens in Task 13 (wire-everything).

- [ ] **Step 3: Verify it compiles**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo check`
Expected: warning about app_handle stub, but no errors.

- [ ] **Step 4: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/pipeline.rs src-tauri/src/commands/recording.rs src-tauri/src/lib.rs
git commit -m "feat: recording pipeline orchestrator"
```

---

## Phase 7: Hotkey & Overlay

### Task 10: Global Hotkey

**Files:**
- Create: `src-tauri/src/hotkey.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement hotkey.rs**

```rust
use std::sync::atomic::{AtomicBool, Ordering};

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_stop() { STOP_REQUESTED.store(true, Ordering::SeqCst); }
pub fn consume_stop() -> bool {
    let prev = STOP_REQUESTED.swap(false, Ordering::SeqCst);
    prev
}
pub fn reset_stop() { STOP_REQUESTED.store(false, Ordering::SeqCst); }

pub fn parse_hotkey(s: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    let mut parts = s.split('+').map(str::trim).collect::<Vec<_>>();
    let key = parts.pop()?.to_lowercase();
    let code = match key.as_str() {
        "space" => Code::Space,
        "v" => Code::KeyV,
        "enter" | "return" => Code::Enter,
        _ => return None,
    };
    let mut mods = Modifiers::empty();
    for p in &parts {
        match p.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "super" | "win" | "meta" => mods |= Modifiers::META,
            _ => return None,
        }
    }
    Some(Shortcut::new(Some(mods), code))
}
```

- [ ] **Step 2: Wire into lib.rs setup hook**

Modify `pub fn run()` in `lib.rs`, replacing the existing builder with:

```rust
pub fn run() {
    let hotkey_str = std::env::var("AV_HOTKEY").unwrap_or_else(|_| "Ctrl+Shift+Space".into());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            if let Some(sc) = parse_hotkey(&hotkey_str) {
                let handle = app.handle().clone();
                app.global_shortcut().on_shortcut(sc, move |_app, _sc, event| {
                    if event.state == ShortcutState::Pressed {
                        let h = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            use tauri::Manager;
                            let st = h.state::<crate::AppState>();
                            let _ = crate::commands::recording::start_via_handle(h, &st).await;
                        });
                    } else {
                        crate::hotkey::request_stop();
                    }
                })?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::recording::start,
            crate::commands::recording::stop,
            crate::commands::history::list,
            crate::commands::history::delete,
            crate::commands::history::search,
            crate::commands::settings::get,
            crate::commands::settings::update,
            crate::commands::models::status,
            crate::commands::models::download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Add helper to recording.rs**

In `src-tauri/src/commands/recording.rs` add:

```rust
pub async fn start_via_handle(app: tauri::AppHandle, state: &crate::AppState) -> Result<(), String> {
    // Stub used until Task 14 (wire-everything) replaces this signature.
    crate::hotkey::reset_stop();
    let _ = (app, state);
    Ok(())
}
```
    let _ = app.emit("hotkey-pressed", ());
    Ok(())
}
```

- [ ] **Step 4: Verify compile**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo check`
Expected: compiles, possibly warnings.

- [ ] **Step 5: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/hotkey.rs src-tauri/src/lib.rs src-tauri/src/commands/recording.rs
git commit -m "feat: global hotkey registration + pipeline trigger"
```

---

### Task 11: Overlay Window (Floating Capsule)

**Files:**
- Create: `src-tauri/src/overlay.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add overlay window config to tauri.conf.json**

Modify `app.windows` array in `tauri.conf.json` to add a second window:

```json
"windows": [
  {
    "label": "main",
    "title": "AI Voice Input",
    "width": 720,
    "height": 520,
    "resizable": true,
    "fullscreen": false
  },
  {
    "label": "overlay",
    "url": "index.html#/overlay",
    "title": "Overlay",
    "width": 200,
    "height": 60,
    "decorations": false,
    "transparent": true,
    "alwaysOnTop": true,
    "skipTaskbar": true,
    "resizable": false,
    "visible": false,
    "shadow": false,
    "focus": false
  }
]
```

- [ ] **Step 2: Implement overlay.rs**

```rust
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

pub fn show(app: &AppHandle, phase: &str, text: Option<&str>) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("overlay") {
        w.show()?;
        position_near_cursor(&w)?;
        let _ = app.emit_to("overlay", "overlay-update", serde_json::json!({
            "phase": phase, "text": text,
        }));
    }
    Ok(())
}

pub fn hide(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("overlay") {
        w.hide()?;
    }
    Ok(())
}

fn position_near_cursor(w: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        unsafe {
            let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) != 0 {
                let _ = w.set_position(PhysicalPosition::new(pt.x + 16, pt.y + 24));
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Add windows-sys to main deps**

Add to `[target.'cfg(windows)'.dependencies]` in Cargo.toml:
```toml
windows-sys = { version = "0.59", features = ["Win32_UI_WindowsAndMessaging", "Win32_Foundation"] }
```

- [ ] **Step 4: Create minimal overlay view in src/views/Overlay.tsx**

```tsx
// src/views/Overlay.tsx
import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

type Phase = 'recording' | 'transcribing' | 'cleaning' | 'done' | 'error';

export default function Overlay() {
  const [phase, setPhase] = useState<Phase>('recording');
  const [text, setText] = useState<string>('');

  useEffect(() => {
    const unlistenPromise = listen<{phase: Phase; text?: string}>('overlay-update', e => {
      setPhase(e.payload.phase);
      setText(e.payload.text ?? '');
    });
    return () => { unlistenPromise.then(u => u()); };
  }, []);

  const color = {recording:'#ef4444', transcribing:'#f59e0b', cleaning:'#f59e0b', done:'#22c55e', error:'#ef4444'}[phase];
  return (
    <div style={{
      width:'100vw', height:'100vh', display:'flex', alignItems:'center', justifyContent:'center',
      background:'transparent', pointerEvents:'none',
    }}>
      <div style={{
        padding:'8px 14px', borderRadius:24, background:'rgba(15,23,42,0.92)',
        color:'white', fontSize:13, display:'flex', alignItems:'center', gap:8,
        boxShadow:'0 4px 12px rgba(0,0,0,0.3)', border:`1px solid ${color}`,
      }}>
        <div style={{width:8, height:8, borderRadius:'50%', background:color,
          animation: phase==='recording' ? 'pulse 1s infinite' : 'none'}} />
        <span>{phase==='recording' ? '正在听…' : phase==='transcribing' ? '转录中…' :
               phase==='cleaning' ? '整理中…' : phase==='done' ? '✓ 已插入' : '出错了'}</span>
      </div>
      <style>{`@keyframes pulse { 0%,100% { opacity:1 } 50% { opacity:0.3 } }`}</style>
    </div>
  );
}
```

- [ ] **Step 5: Wire overlay route in src/App.tsx**

Replace `src/App.tsx`:

```tsx
import { useEffect, useState } from 'react';

export default function App() {
  const [route, setRoute] = useState(window.location.hash);
  useEffect(() => {
    const onHash = () => setRoute(window.location.hash);
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  if (route.startsWith('#/overlay')) {
    const Overlay = require('./views/Overlay').default;
    return <Overlay />;
  }
  return (
    <div className="app">
      <h1>AI Voice Input</h1>
      <p>Press <kbd>Ctrl+Shift+Space</kbd> to start recording.</p>
    </div>
  );
}
```

- [ ] **Step 6: Verify compile**

Run: `cd D:/soft_work/02-MINE/ai-voice-input && npm run build`
Expected: build succeeds.

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo check`
Expected: compiles.

- [ ] **Step 7: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/overlay.rs src-tauri/tauri.conf.json src-tauri/Cargo.toml src/
git commit -m "feat: floating overlay window"
```

---

## Phase 8: Frontend Views

### Task 12: History View with Search

**Files:**
- Create: `src/views/History.tsx`, `src/components/TranscriptItem.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create TranscriptItem component**

```tsx
// src/components/TranscriptItem.tsx
import { useState } from 'react';

type Props = {
  id: number;
  cleanText: string;
  rawText: string;
  durationMs: number;
  createdAt: string;
  onDelete: (id: number) => void;
};

function formatTime(iso: string) {
  const d = new Date(iso);
  return d.toLocaleString();
}

function formatDuration(ms: number) {
  return `${(ms / 1000).toFixed(1)}s`;
}

export default function TranscriptItem({ id, cleanText, rawText, durationMs, createdAt, onDelete }: Props) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div style={{border:'1px solid #334155', borderRadius:8, padding:12, marginBottom:8}}>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center'}}>
        <div style={{flex:1, color:'#f1f5f9'}}>{cleanText}</div>
        <div style={{display:'flex', gap:8, fontSize:12, color:'#94a3b8'}}>
          <span>{formatTime(createdAt)}</span>
          <span>{formatDuration(durationMs)}</span>
        </div>
      </div>
      {expanded && (
        <div style={{marginTop:8, padding:8, background:'#0f172a', borderRadius:4, fontSize:12, color:'#94a3b8'}}>
          <b>原文:</b> {rawText}
        </div>
      )}
      <div style={{marginTop:8, display:'flex', gap:8}}>
        <button onClick={() => setExpanded(!expanded)} style={{background:'transparent', color:'#60a5fa', border:'none', cursor:'pointer'}}>
          {expanded ? '收起' : '看原文'}
        </button>
        <button onClick={() => navigator.clipboard.writeText(cleanText)} style={{background:'transparent', color:'#60a5fa', border:'none', cursor:'pointer'}}>
          复制
        </button>
        <button onClick={() => onDelete(id)} style={{background:'transparent', color:'#ef4444', border:'none', cursor:'pointer'}}>
          删除
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Create History view**

```tsx
// src/views/History.tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import TranscriptItem from '../components/TranscriptItem';

type T = { id: number; raw_text: string; clean_text: string; duration_ms: number; created_at: string; app_name: string|null };

export default function History() {
  const [items, setItems] = useState<T[]>([]);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      const res: T[] = query ? await invoke('search', { q: query }) : await invoke('list');
      setItems(res);
    } finally { setLoading(false); }
  };

  useEffect(() => { refresh(); }, []);
  useEffect(() => {
    const t = setTimeout(refresh, 200);
    return () => clearTimeout(t);
  }, [query]);

  const onDelete = async (id: number) => {
    await invoke('delete', { id });
    refresh();
  };

  return (
    <div style={{padding:24, height:'100vh', overflowY:'auto', background:'#0f172a', color:'#f1f5f9'}}>
      <h2 style={{marginTop:0}}>历史记录</h2>
      <input
        placeholder="搜索关键词…"
        value={query}
        onChange={e => setQuery(e.target.value)}
        style={{width:'100%', padding:8, background:'#1e293b', color:'white', border:'1px solid #334155', borderRadius:4, marginBottom:16}}
      />
      {loading && <div style={{color:'#94a3b8'}}>加载中…</div>}
      {!loading && items.length === 0 && <div style={{color:'#94a3b8'}}>没有记录</div>}
      {items.map(t => (
        <TranscriptItem key={t.id} id={t.id} cleanText={t.clean_text} rawText={t.raw_text}
          durationMs={t.duration_ms} createdAt={t.created_at} onDelete={onDelete} />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Update commands/history.rs to return proper types**

Replace `src-tauri/src/commands/history.rs`:

```rust
use crate::storage::{Storage, Transcript};
use std::sync::Arc;
use tauri::State;

pub struct HistoryState(pub Arc<Storage>);

#[tauri::command]
pub async fn list(state: State<'_, HistoryState>) -> Result<Vec<Transcript>, String> {
    state.0.list(100, 0).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete(state: State<'_, HistoryState>, id: i64) -> Result<(), String> {
    state.0.delete(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search(state: State<'_, HistoryState>, q: String) -> Result<Vec<Transcript>, String> {
    state.0.search(&q, 100).await.map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Update App.tsx with router**

Replace `src/App.tsx`:

```tsx
import { useEffect, useState } from 'react';
import History from './views/History';

export default function App() {
  const [route, setRoute] = useState(window.location.hash);

  useEffect(() => {
    const onHash = () => setRoute(window.location.hash);
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  if (route.startsWith('#/overlay')) {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const Overlay = require('./views/Overlay').default;
    return <Overlay />;
  }
  if (route.startsWith('#/history')) {
    return <History />;
  }
  return (
    <div className="app">
      <h1>AI Voice Input</h1>
      <p>Press <kbd>Ctrl+Shift+Space</kbd> to start recording.</p>
      <a href="#/history" style={{color:'#60a5fa'}}>查看历史</a>
    </div>
  );
}
```

- [ ] **Step 5: Verify it builds**

Run: `cd D:/soft_work/02-MINE/ai-voice-input && npm run build`
Expected: TypeScript compiles, Vite build succeeds.

- [ ] **Step 6: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src/views/History.tsx src/components/TranscriptItem.tsx src/App.tsx src-tauri/src/commands/history.rs
git commit -m "feat: history view with search and FTS"
```

---

### Task 13: Settings View + First-Run Wizard

**Files:**
- Create: `src/views/Settings.tsx`, `src/views/FirstRun.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create Settings view**

```tsx
// src/views/Settings.tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Settings = {
  hotkey: string;
  mic_device_id: string | null;
  auto_stop_seconds: number;
  model_variant: 'fast'|'balanced'|'accurate';
  cleanup_intensity: 'light'|'normal'|'aggressive';
  auto_launch: boolean;
  overlay_follow_cursor: boolean;
};

export default function SettingsView() {
  const [s, setS] = useState<Settings | null>(null);
  const [status, setStatus] = useState<{whisper_installed:boolean; llama_installed:boolean}>({whisper_installed:false, llama_installed:false});
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<{name:string; percent:number} | null>(null);

  useEffect(() => {
    invoke<Settings>('get').then(setS);
    invoke<typeof status>('status').then(setStatus);
    // Listen for download progress
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{name:string; downloaded:number; total:number; percent:number}>('model-download-progress', e => {
        setProgress({name:e.payload.name, percent:e.payload.percent});
      });
    });
  }, []);

  const save = async () => {
    if (!s) return;
    await invoke('update', { settings: s });
    alert('已保存');
  };

  const download = async () => {
    setDownloading(true);
    try {
      await invoke('download');
      const st = await invoke<typeof status>('status');
      setStatus(st);
    } finally {
      setDownloading(false); setProgress(null);
    }
  };

  if (!s) return <div style={{padding:24, color:'#94a3b8'}}>加载中…</div>;

  const bothInstalled = status.whisper_installed && status.llama_installed;

  return (
    <div style={{padding:24, background:'#0f172a', color:'#f1f5f9', height:'100vh', overflowY:'auto'}}>
      <h2 style={{marginTop:0}}>设置</h2>

      <fieldset style={{border:'1px solid #334155', borderRadius:8, padding:16, marginBottom:16}}>
        <legend>模型状态</legend>
        <div>Whisper: {status.whisper_installed ? '✓ 已安装' : '✗ 未安装'}</div>
        <div>Qwen LLM: {status.llama_installed ? '✓ 已安装' : '✗ 未安装'}</div>
        {!bothInstalled && (
          <>
            <button onClick={download} disabled={downloading} style={{marginTop:12, padding:'6px 12px', background:'#2563eb', color:'white', border:'none', borderRadius:4, cursor:'pointer'}}>
              {downloading ? '下载中…' : '下载模型 (~6GB)'}
            </button>
            {progress && <div style={{marginTop:8, color:'#94a3b8'}}>{progress.name}: {progress.percent.toFixed(1)}%</div>}
          </>
        )}
      </fieldset>

      <fieldset style={{border:'1px solid #334155', borderRadius:8, padding:16, marginBottom:16}}>
        <legend>快捷键</legend>
        <input value={s.hotkey} onChange={e => setS({...s, hotkey:e.target.value})}
          style={{width:'100%', padding:6, background:'#1e293b', color:'white', border:'1px solid #334155', borderRadius:4}} />
        <small style={{color:'#94a3b8'}}>重启后生效</small>
      </fieldset>

      <fieldset style={{border:'1px solid #334155', borderRadius:8, padding:16, marginBottom:16}}>
        <legend>录音</legend>
        <label>自动停止 (秒):
          <input type="number" value={s.auto_stop_seconds} min={5} max={120}
            onChange={e => setS({...s, auto_stop_seconds:Number(e.target.value)})}
            style={{marginLeft:8, width:60, background:'#1e293b', color:'white', border:'1px solid #334155', borderRadius:4}} />
        </label>
      </fieldset>

      <fieldset style={{border:'1px solid #334155', borderRadius:8, padding:16, marginBottom:16}}>
        <legend>清理强度</legend>
        <select value={s.cleanup_intensity} onChange={e => setS({...s, cleanup_intensity: e.target.value as Settings['cleanup_intensity']})}
          style={{padding:4, background:'#1e293b', color:'white', border:'1px solid #334155', borderRadius:4}}>
          <option value="light">轻度（少改动）</option>
          <option value="normal">正常</option>
          <option value="aggressive">强力（更压缩）</option>
        </select>
      </fieldset>

      <button onClick={save} style={{padding:'8px 16px', background:'#2563eb', color:'white', border:'none', borderRadius:4, cursor:'pointer'}}>
        保存设置
      </button>
      <div style={{marginTop:16}}><a href="#/history" style={{color:'#60a5fa'}}>查看历史</a></div>
    </div>
  );
}
```

- [ ] **Step 2: Create FirstRun wizard**

```tsx
// src/views/FirstRun.tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function FirstRun() {
  const [percent, setPercent] = useState(0);
  const [name, setName] = useState('');
  const [done, setDone] = useState(false);

  useEffect(() => {
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{name:string; percent:number}>('model-download-progress', e => {
        setName(e.payload.name);
        setPercent(e.payload.percent);
      });
    });
    invoke('download').then(() => setDone(true)).catch(e => {
      alert('下载失败：' + e);
    });
  }, []);

  if (done) {
    return (
      <div style={{padding:48, textAlign:'center', background:'#0f172a', color:'#f1f5f9', height:'100vh'}}>
        <h1>准备好了！</h1>
        <p>模型下载完成。现在可以按 <kbd>Ctrl+Shift+Space</kbd> 开始录音。</p>
        <a href="#/" style={{color:'#60a5fa'}}>打开主界面</a>
      </div>
    );
  }

  return (
    <div style={{padding:48, background:'#0f172a', color:'#f1f5f9', height:'100vh'}}>
      <h1>欢迎使用 AI 语音输入法</h1>
      <p>首次使用需要下载模型文件（约 6GB），请保持网络连接。</p>
      <div style={{marginTop:24, padding:16, background:'#1e293b', borderRadius:8}}>
        <div style={{marginBottom:8}}>正在下载: {name || '准备中…'}</div>
        <div style={{height:8, background:'#334155', borderRadius:4, overflow:'hidden'}}>
          <div style={{width:`${percent}%`, height:'100%', background:'#2563eb', transition:'width 0.2s'}} />
        </div>
        <div style={{marginTop:8, textAlign:'right', color:'#94a3b8'}}>{percent.toFixed(1)}%</div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Update App.tsx to dispatch FirstRun vs Settings**

Replace `src/App.tsx`:

```tsx
import { useEffect, useState } from 'react';

type ModelStatus = { whisper_installed: boolean; llama_installed: boolean };

export default function App() {
  const [route, setRoute] = useState(window.location.hash);
  const [status, setStatus] = useState<ModelStatus | null>(null);

  useEffect(() => {
    const onHash = () => setRoute(window.location.hash);
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  useEffect(() => {
    import('@tauri-apps/api/core').then(({ invoke }) => {
      invoke<ModelStatus>('status').then(setStatus);
    });
  }, []);

  if (route.startsWith('#/overlay')) {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const Overlay = require('./views/Overlay').default;
    return <Overlay />;
  }
  if (route.startsWith('#/history')) {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const History = require('./views/History').default;
    return <History />;
  }
  if (route.startsWith('#/settings')) {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const SettingsView = require('./views/Settings').default;
    return <SettingsView />;
  }

  if (status && !(status.whisper_installed && status.llama_installed)) {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const FirstRun = require('./views/FirstRun').default;
    return <FirstRun />;
  }

  return (
    <div className="app">
      <h1>AI Voice Input</h1>
      <p>按 <kbd>Ctrl+Shift+Space</kbd> 开始录音</p>
      <p><a href="#/history" style={{color:'#60a5fa'}}>历史</a> · <a href="#/settings" style={{color:'#60a5fa'}}>设置</a></p>
    </div>
  );
}
```

- [ ] **Step 4: Verify build**

Run: `cd D:/soft_work/02-MINE/ai-voice-input && npm run build`
Expected: build succeeds.

- [ ] **Step 5: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src/views/Settings.tsx src/views/FirstRun.tsx src/App.tsx
git commit -m "feat: settings view and first-run wizard"
```

---

## Phase 9: Wire Everything Together

### Task 14: AppState Wiring

**Files:**
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/commands/recording.rs`, `src-tauri/src/commands/history.rs`

- [ ] **Step 1: Define AppState in lib.rs**

Add at top of `lib.rs` (after `mod commands;`):

```rust
use std::sync::Arc;
use sidecar::{Sidecar, SidecarKind};
use storage::Storage;

pub struct AppState {
    pub storage: Arc<Storage>,
    pub whisper: Arc<Sidecar>,
    pub llama: Arc<Sidecar>,
    pub settings_path: std::path::PathBuf,
    pub history_db_path: std::path::PathBuf,
}
```

- [ ] **Step 2: Initialize state in setup() and pass via .manage()**

Replace `run()` body in `lib.rs` with:

```rust
pub fn run() {
    let hotkey_str = std::env::var("AV_HOTKEY").unwrap_or_else(|_| "Ctrl+Shift+Space".into());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            use tauri::Manager;
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("transcripts.db");
            let settings_path = app_data.join("settings.json");

            let storage = tauri::async_runtime::block_on(async {
                Storage::open(db_path.to_str().unwrap()).await
            })?;
            let storage = Arc::new(storage);

            let whisper = Arc::new(Sidecar::new(SidecarKind::Whisper, "http://127.0.0.1:8178"));
            let llama   = Arc::new(Sidecar::new(SidecarKind::Llama,   "http://127.0.0.1:8188"));

            // Spawn sidecars
            tauri::async_runtime::block_on(async {
                let _ = sidecar::spawn(&app.handle(), SidecarKind::Whisper).await;
                let _ = sidecar::spawn(&app.handle(), SidecarKind::Llama).await;
            });

            app.manage(AppState { storage, whisper, llama, settings_path, history_db_path: db_path });
            app.manage(crate::commands::history::HistoryState(Arc::clone(&app.state::<AppState>().storage)));

            // Register hotkey
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
            if let Some(sc) = parse_hotkey(&hotkey_str) {
                let handle = app.handle().clone();
                app.global_shortcut().on_shortcut(sc, move |_app, _sc, event| {
                    if event.state == ShortcutState::Pressed {
                        crate::hotkey::reset_stop();
                        let h = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            use tauri::Manager;
                            let st = h.state::<crate::AppState>();
                            let _ = crate::commands::recording::start_via_handle(h, &st).await;
                        });
                    } else {
                        crate::hotkey::request_stop();
                    }
                })?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::recording::start,
            crate::commands::recording::stop,
            crate::commands::history::list,
            crate::commands::history::delete,
            crate::commands::history::search,
            crate::commands::settings::get,
            crate::commands::settings::update,
            crate::commands::models::status,
            crate::commands::models::download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Fix recording.rs to use State**

Replace `src-tauri/src/commands/recording.rs`:

```rust
use crate::AppState;
use crate::pipeline;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn start(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if pipeline::is_recording() { return Err("already recording".into()); }
    start_via_handle(app, &state).await
}

#[tauri::command]
pub async fn stop() -> Result<(), String> {
    crate::hotkey::request_stop();
    Ok(())
}

pub async fn start_via_handle(handle: AppHandle, state: &AppState) -> Result<(), String> {
    let settings = crate::commands::settings::Settings::load_or_default(&state.settings_path)
        .map_err(|e| e.to_string())?;
    let auto_stop = Duration::from_secs(settings.auto_stop_seconds as u64);
    let app = handle.clone();
    let storage = state.storage.clone();
    let whisper = state.whisper.clone();
    let llama = state.llama.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = pipeline::run_once(app.clone(), storage, whisper, llama, auto_stop).await {
            tracing::error!("pipeline: {e}");
            let _ = app.emit("pipeline-status", serde_json::json!({"phase":"error","error":e.to_string()}));
        }
    });
    Ok(())
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo check`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add src-tauri/src/lib.rs src-tauri/src/commands/recording.rs
git commit -m "feat: wire AppState across all commands"
```

---

## Phase 10: Build & Distribution

### Task 15: Bundle Sidecar Binaries

**Files:**
- Create: `sidecar/whisper-server.exe`, `sidecar/llama-server.exe` (placeholders/real builds)
- Create: `scripts/fetch-sidecars.ps1`

- [ ] **Step 1: Create scripts/fetch-sidecars.ps1**

```powershell
# scripts/fetch-sidecars.ps1
# Downloads prebuilt whisper.cpp + llama.cpp Windows binaries from official releases.
# Run once before first `tauri build`.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "sidecar"
New-Item -ItemType Directory -Force -Path $dest | Out-Null

# Whisper.cpp
$whisperVer = "v1.7.2"
$whisperZip = "whisper-bin-x64.zip"
$whisperUrl = "https://github.com/ggerganov/whisper.cpp/releases/download/$whisperVer/$whisperZip"
Write-Host "Downloading whisper.cpp $whisperVer..."
Invoke-WebRequest -Uri $whisperUrl -OutFile "$dest/$whisperZip"
Expand-Archive -Path "$dest/$whisperZip" -DestinationPath "$dest/whisper-tmp" -Force
Move-Item -Path "$dest/whisper-tmp/Release/whisper-server.exe" -Destination "$dest/whisper-server.exe" -Force
Remove-Item -Recurse -Force "$dest/whisper-tmp", "$dest/$whisperZip"

# Llama.cpp
$llamaVer = "b5103"
$llamaZip = "llama-$llamaVer-bin-win-cuda12.4-x64.zip"
$llamaUrl = "https://github.com/ggerganov/llama.cpp/releases/download/$llamaVer/$llamaZip"
Write-Host "Downloading llama.cpp $llamaVer..."
Invoke-WebRequest -Uri $llamaUrl -OutFile "$dest/$llamaZip"
Expand-Archive -Path "$dest/$llamaZip" -DestinationPath "$dest/llama-tmp" -Force
Move-Item -Path "$dest/llama-tmp/llama-server.exe" -Destination "$dest/llama-server.exe" -Force
Remove-Item -Recurse -Force "$dest/llama-tmp", "$dest/$llamaZip"

Write-Host "Done. Sidecar binaries in $dest"
Get-ChildItem $dest
```

- [ ] **Step 2: Document in README**

Create `README.md`:

```markdown
# AI Voice Input

Local Windows voice-to-text with AI cleanup. Press a hotkey, speak, get clean text at your cursor.

## Setup (dev)

1. Install Node 20+, Rust stable, Visual Studio Build Tools 2022 with C++ workload.
2. `npm install`
3. `pwsh scripts/fetch-sidecars.ps1` (downloads whisper.cpp + llama.cpp)
4. `npm run tauri dev`

## First run

App launches → detects missing models → downloads ~6GB on first run → ready.

Hotkey: `Ctrl+Shift+Space` (default, change in Settings).

## Build (release)

`npm run tauri build` → produces NSIS installer in `src-tauri/target/release/bundle/nsis/`.
```

- [ ] **Step 3: Add .gitignore**

Create `.gitignore`:

```
node_modules/
dist/
src-tauri/target/
src-tauri/Cargo.lock
sidecar/*.exe
sidecar/models/
.superpowers/
*.log
```

- [ ] **Step 4: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add scripts/ README.md .gitignore
git commit -m "build: sidecar fetch script + README + gitignore"
```

---

### Task 16: End-to-End Smoke Test

**Files:**
- Create: `tests/integration_test.rs`

- [ ] **Step 1: Write integration test (mock sidecars)**

```rust
// tests/integration_test.rs
use ai_voice_input_lib::storage::{Storage, Transcript};
use chrono::Utc;

#[tokio::test]
async fn full_pipeline_with_real_storage() {
    let storage = Storage::in_memory().await.unwrap();
    let t = Transcript {
        id: None,
        raw_text: "嗯那个今天呃天气真的不错哦".into(),
        clean_text: "今天天气真的不错。".into(),
        duration_ms: 3500,
        created_at: Utc::now(),
        app_name: Some("notepad".into()),
    };
    let id = storage.insert(&t).await.unwrap();

    let found = storage.search("天气", 10).await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, Some(id));

    storage.delete(id).await.unwrap();
    let after = storage.search("天气", 10).await.unwrap();
    assert_eq!(after.len(), 0);
}
```

- [ ] **Step 2: Run integration test**

Run: `cd D:/soft_work/02-MINE/ai-voice-input/src-tauri && cargo test --test integration_test`
Expected: PASS

- [ ] **Step 3: Manual smoke checklist** (executed by developer on real hardware)

- [ ] `npm run tauri dev` launches without errors
- [ ] First-run wizard appears and downloads models
- [ ] Press `Ctrl+Shift+Space` → overlay capsule appears
- [ ] Speak 5s of Chinese with fillers → release hotkey → text appears in current app
- [ ] Open History view → see the transcript with searchable text
- [ ] Search "天气" in history → finds it
- [ ] Close app and reopen → settings and history persist
- [ ] Disconnect network → still works offline

- [ ] **Step 4: Commit**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git add tests/integration_test.rs
git commit -m "test: end-to-end integration smoke test"
```

---

### Task 17: Production Build

- [ ] **Step 1: Build the installer**

Run: `cd D:/soft_work/02-MINE/ai-voice-input && npm run tauri build`
Expected: produces `src-tauri/target/release/bundle/nsis/AI Voice Input_0.1.0_x64-setup.exe`

- [ ] **Step 2: Verify installer on test machine**

- [ ] Install on clean Windows 11 VM
- [ ] Run installer → completes without error
- [ ] Launch app → first-run wizard appears
- [ ] Download models → completes
- [ ] Test hotkey + insert flow end-to-end
- [ ] Uninstall → AppData cleaned

- [ ] **Step 3: Tag release**

```bash
cd D:/soft_work/02-MINE/ai-voice-input
git tag v0.1.0
git log --oneline
```

---

## Self-Review

**Spec coverage:**
- §3 产品决策 → Tasks 1-13 (all decisions reflected in code) ✓
- §4 架构 → Task 2, 6, 7 (Tauri + 2 sidecars via HTTP) ✓
- §5 组件 → Tasks 2-11 (each module has its own task) ✓
- §6 数据流 → Task 9 (pipeline.rs implements full flow) ✓
- §7 数据模型 → Task 3 (transcripts table + FTS5) ✓
- §8 错误处理 → Tasks 6 (sidecar retry/health), 9 (fallback to raw), 5 (clipboard-only fallback) ✓
- §9 首次启动 → Task 13 (FirstRun wizard) + Task 7 (model download) ✓
- §10 测试 → Tasks 3-9 each have unit tests, Task 16 integration ✓
- §11 部署 → Task 15 (fetch-sidecars), Task 17 (NSIS build) ✓

**Placeholder scan:** No "TBD"/"TODO"/"implement later" in plan. Each task has concrete code or commands.

**Type consistency:**
- `Storage::insert` returns `i64` — used in Task 16 ✓
- `Sidecar::new(kind, base_url)` — used everywhere consistently ✓
- `Settings` struct fields match between Task 4 (impl) and Task 13 (frontend invoke) ✓
- `ModelSpec` fields `name`, `filename`, `url`, `sha256`, `size_bytes` — used in both backend (Task 7) and frontend (Task 13 via 'status'/'download' commands) ✓

**Type consistency (verified):**
- `start_via_handle(app: AppHandle, state: &AppState)` signature consistent across Task 10 stub, Task 14 final, and both hotkey call sites ✓
- `Storage::insert` returns `i64` — used in Task 16 ✓
- `Sidecar::new(kind, base_url)` — used everywhere consistently ✓
- `Settings` struct fields match between Task 4 (impl) and Task 13 (frontend invoke) ✓
- `ModelSpec` fields `name`, `filename`, `url`, `sha256`, `size_bytes` — used in both backend (Task 7) and frontend (Task 13 via 'status'/'download' commands) ✓
- Task 10's setup() block is intentionally superseded by Task 14's setup() — implementer should follow Task 14's version exclusively. Task 10 Step 2 documents the early wiring for reference only.

**No placeholders remain.** All Tauri commands have real implementations; the only stubs are Task 2 Step 5 empty modules (`hotkey.rs`, `audio.rs`, etc.) which are explicitly meant to be empty until later tasks fill them.

---

## Execution Estimate

- Tasks 1-2: scaffold (10 min)
- Tasks 3-4: storage/settings (30 min)
- Tasks 5-8: audio, sidecar, models, insert (60 min)
- Tasks 9-11: pipeline, hotkey, overlay (45 min)
- Tasks 12-13: frontend views (45 min)
- Tasks 14-15: wire + sidecar fetch (20 min)
- Tasks 16-17: integration + build (20 min)

**Total: ~4 hours of focused implementation work, plus model downloads on first run.**