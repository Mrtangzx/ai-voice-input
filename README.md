# AI 语音输入法

本地 Windows 桌面应用：按全局快捷键录音，本地 AI 转写并整理为文字，直接插入到光标处。

## 架构

- **Tauri shell** (~15MB) — Rust 后端 + React 前端
- **Sidecar 1: whisper-server** — OpenAI 兼容 ASR API（端口 8178）
- **Sidecar 2: llama-server** — OpenAI 兼容 Chat API（端口 8188）
- **SQLite + FTS5** — 本地历史与全文搜索
- 数据流：麦克风 → cpal → whisper-server → llama-server → 剪贴板 + Ctrl+V

## 开发环境

1. Node 20+
2. Rust stable (toolchain: `stable-x86_64-pc-windows-gnu`)
3. Visual Studio Build Tools 2022（C++ workload）
4. WebView2 Runtime（Windows 11 自带；Win10 需要单独安装）

## 设置

```bash
# 1. 安装 Node 依赖
npm install

# 2. 下载 sidecar 二进制（首次）
pwsh scripts/fetch-sidecars.ps1

# 3. 启动开发服务器
npm run tauri dev
```

首次启动会引导下载模型文件（约 6GB）。模型缓存在 `%APPDATA%/ai-voice-input/`。

## 使用

- 默认快捷键：`Ctrl+Shift+Space`（在设置页可改）
- 按下快捷键 → 听到「正在听…」悬浮胶囊 → 再按一次停止 → 文字插入到当前光标
- 设置页：`#settings`
- 历史页：`#history`

## 构建发布版

```bash
npm run tauri build
```

产出：`src-tauri/target/release/bundle/nsis/AI Voice Input_0.1.0_x64-setup.exe`

## 项目结构

```
src-tauri/src/
├── lib.rs            # AppState + 启动逻辑
├── audio.rs          # cpal 录音 + WAV
├── sidecar.rs        # whisper/llama HTTP 客户端
├── models.rs         # 模型下载与 SHA256 校验
├── pipeline.rs       # 录音→转写→整理→插入 管道
├── insert.rs         # 剪贴板 + Ctrl+V
├── hotkey.rs         # 全局快捷键解析与状态
├── overlay.rs        # 悬浮胶囊窗口
├── storage.rs        # SQLite + FTS5
└── commands/         # Tauri IPC 命令
    ├── recording.rs
    ├── history.rs
    ├── settings.rs
    └── models.rs

src/views/
├── FirstRun.tsx
├── Settings.tsx
├── History.tsx
└── Overlay.tsx
```

## 数据隐私

- 所有语音数据、模型文件、运行结果都在本地
- 无任何云端上传
- 离线可用（首次启动后）

## 已知限制

- WebView2 Runtime 必须安装（Win11 自带）
- mingw 工具链的 DLL 导出限制导致集成测试需要 webview2 loader；单元测试通过 `cargo test --lib` 验证
- 重采样使用线性插值（v1 权衡，质量不足可换 SincFixedIn）