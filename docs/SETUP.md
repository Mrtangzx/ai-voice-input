# AI 语音输入法 - 当前可用状态与上手指南

> 最近一次检查：2026-06-28

## TL;DR — 怎么才能立刻体验？

**最快的路径（推荐）**：用 **云端 API**（DeepSeek 或通义千问 DashScope）。两分钟搞定，立刻可用。

**完全离线路径**：本地 whisper medium 模型已下载可用，但 CPU 推理极慢（1 秒音频 ~3 分钟），不建议在没有 GPU 的机器上使用。

## 当前已修复的问题（本次提交）

| 问题 | 原因 | 修复 |
|---|---|---|
| App 启动后 sidecar 进程没起来 | `app.path().resource_dir()` 在 dev 模式下找不到侧车文件 | 新增 `resolve_sidecar_dir`，依次尝试 resource_dir / `CARGO_MANIFEST_DIR/sidecar` / exe 同目录 |
| 启动失败时没有任何错误提示 | `let _ = spawn(...)` 吞掉了错误 | 现在用 `tracing::error!` 输出，启动日志会写到 stdout |
| Whisper 模型加载失败 | optimum 找不到 `encoder_model_fp16.onnx` 等带后缀的文件 | `whisper-asr.py` 显式传入 `encoder_file_name` / `decoder_file_name` |
| 推理慢但客户端 60s 就超时 | medium 模型 CPU 推理 ~3 分钟 | reqwest timeout 提到 300s |
| 设置页只显示"已安装"不显示二进制缺失 | `status` 只检查 GGUF 文件 | `status` 现在返回 `whisper_installed` / `llama_model_installed` / `llama_binary_installed` 三项，UI 给出明确报错框 |
| 生产构建不会打包 Python 脚本和 ONNX 模型 | `tauri.conf.json` 的 `resources` 只配了 `sidecar/models/*` | 现在打包 `sidecar/whisper-asr.py` + `sidecar/whisper-model/*` |

## ⚠️ 仍未解决的问题

### 1. `llama-server.exe` 在你的机器上是 0 字节

```
src-tauri/sidecar/llama-server-x86_64-pc-windows-gnu.exe   0 bytes
```

GitHub 直接下载在这个环境下被阻断（`Connection was reset`），modelscope 也没有可用的 Windows 预编译包。

**不解决也能用**：因为 App 检测到本地 LLM 二进制缺失会自动降级——云端 API 模式可正常工作。

**解决路径**（任选其一）：
- 切到云端 API（推荐，2 分钟）
- 在另一台能访问 GitHub 的机器上下载 https://github.com/ggml-org/llama.cpp/releases/download/b9828/llama-b9828-bin-win-cpu-x64.zip，解出 `llama-server.exe`，重命名为 `llama-server-x86_64-pc-windows-gnu.exe`，放到 `src-tauri/sidecar/`
- 用 GPU 编译 llama.cpp（耗时，但能用 CUDA）

### 2. 本地 Whisper medium 在纯 CPU 上太慢

实测 1 秒音频推理耗时 **168 秒**（约 3 分钟）。原因：medium 模型 fp16 量化在 ONNX Runtime CPU 上速度不理想。

如果你坚持本地离线使用：
- 把 whisper 模型换成 **tiny** 或 **base**（小一个数量级，速度快 5-10 倍）
- 或启用 ONNX Runtime DirectML（Windows 自带 GPU 加速）

## 上手步骤（云端 API 路径，推荐）

### 1. 拿到 API Key（任选一家）

| 服务 | 免费额度 | 申请地址 |
|---|---|---|
| DeepSeek | 注册送 ¥1（够用很久） | https://platform.deepseek.com/api_keys |
| 通义千问 DashScope | qwen-turbo 100 万 tokens 免费 | https://dashscope.console.aliyun.com/apiKey |

### 2. 启动应用

```bash
cd D:/soft_work/02-MINE/ai-voice-input
npm run tauri dev
```

首次启动会弹 FirstRun 页面，**直接选 "☁️ 推荐：使用云端 API"**，填入 API Key。

### 3. 等 Whisper 模型下载完

- modelscope.cn 镜像，约 1.5GB，5-30 分钟（视网速）
- 如果下载失败，把 `models.rs::qwen_7b_q4` 的 URL 改成 HF mirror，或手动下载放到 `src-tauri/sidecar/models/ggml-medium.bin`

### 4. 开始录音

按 `Ctrl+Shift+Space`：
- 第一次按下 → "正在听…" 浮窗出现 → 说话
- 第二次按下 → "转写中…"（1-3 分钟，取决于本地 whisper 速度）→ "整理中…"（云端 1-2 秒）→ 文字自动粘贴到光标位置

如果嫌本地 whisper 慢而无法忍受，可以保持云端 API 但用更小的本地模型（修改 `models.rs::whisper_medium` 的 URL）。

## 上手步骤（纯本地路径，需要 GPU 或能下载 llama-server）

如果你的机器有 NVIDIA GPU：

```bash
# 安装 CUDA toolkit
# 下载 CUDA 版 llama.cpp
curl -L -o llama.zip https://github.com/ggml-org/llama.cpp/releases/download/b9828/llama-b9828-bin-win-cuda-12.4-x64.zip
# 解压并放到正确位置
```

如果你能访问 GitHub Release：

```bash
curl -L -o /tmp/llama.zip https://github.com/ggml-org/llama.cpp/releases/download/b9828/llama-b9828-bin-win-cpu-x64.zip
cd /tmp && unzip llama.zip
mv llama-server.exe "D:/soft_work/02-MINE/ai-voice-input/src-tauri/sidecar/llama-server-x86_64-pc-windows-gnu.exe"
```

## 调试技巧

### 查看 sidecar 启动日志

App 在 dev 模式下 stderr 会打印 sidecar 启动情况，包括失败原因：
```
[INFO] sidecar dir resolved via CARGO_MANIFEST_DIR: ...\src-tauri\sidecar
[INFO] starting whisper sidecar: python ...\whisper-asr.py
[INFO] whisper sidecar spawned (pid=12345)
[INFO] whisper sidecar healthy on :8178
[ERROR] failed to spawn llama sidecar: llama-server binary missing at ...
```

### 手动检查 sidecar 状态

```bash
curl -s --max-time 3 -w "\nwhisper(8178): %{http_code}\n" http://127.0.0.1:8178/health
curl -s --max-time 3 -w "\nllama(8188):   %{http_code}\n" http://127.0.0.1:8188/health
```

### 手动启动 whisper 调试

```bash
cd src-tauri/sidecar
ASR_MODEL_DIR="$(pwd)/whisper-model" ASR_PORT=8178 python whisper-asr.py
# 另一个窗口
curl -F "file=@test.wav" -F "response_format=text" http://127.0.0.1:8178/v1/audio/transcriptions
```

## 已知限制

- whisper medium 在 CPU 上 ~3 分钟/段（建议换 tiny/base，或用云端 ASR）
- WebView2 Runtime 必须安装（Win11 自带，Win10 需手动）
- 全局快捷键在某些全屏应用（如某些游戏）中可能被劫持