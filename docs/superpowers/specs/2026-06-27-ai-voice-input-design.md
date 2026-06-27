# AI 语音输入法 — 设计文档

- 日期：2026-06-27
- 状态：草稿，待用户批准
- 作者：Claude（经 brainstorming 流程产出）

## 1. 背景与目标

做一个 **本地 Windows 桌面应用**，让用户通过全局快捷键快速录音，由本地 AI 完成「语音 → 整理后文字」并插入到当前光标位置。

**核心价值**：
- 比打字快 3-5 倍
- 完全离线，语音数据不离开电脑
- 输出不是生硬的转录文本，而是经过整理的可直接使用的句子

**非目标**（明确不做）：
- 跨平台（仅 Windows 11 / Windows 10 21H2+）
- 语音指令识别（如「换行」「句号」作为口令）—— 后续可加，本期不做
- 云端 API 备份或云同步
- 多用户 / 多账号
- 移动端 / Web 版本

## 2. 用户故事

1. **会议中快速记录**：用户在 Word 里按下 `Ctrl+Shift+Space`，口述一段会议内容，再次按下快捷键停止，文字直接出现在光标处。
2. **写代码注释**：在 VS Code 里用语音写注释，识别出的中文带标点直接插入。
3. **回查上次说了什么**：打开历史面板，搜一个关键词，找到昨天的某段录音转录。

## 3. 产品决策（已与用户确认）

| 维度 | 选择 | 理由 |
|---|---|---|
| AI 处理深度 | 智能整理（去口头禅 + 加标点 + 修正语序） | 平衡准确度与可用性 |
| 部署模式 | 纯本地 | 隐私 + 无网络依赖 |
| 触发方式 | 全局快捷键 | 最快、无 UI 切换成本 |
| 平台 | 仅 Windows | 用 Windows API 做最丝滑的体验 |
| 语言 | 中文 + 英文（混合识别） | 用户主要场景 |
| 历史记录 | 本地保留，可搜索 | 方便回查 |
| 录音反馈 | 跟随光标的悬浮胶囊 | 存在感强且不打扰 |
| 模型规格 | Whisper medium + Qwen2.5-7B-Instruct Q4 | 默认推荐配置（需 ~6GB 显存或 16GB 内存） |
| 技术栈 | Tauri（Rust + Web） | 体积小、启动快、Windows 集成能力强 |
| 语音指令 | 暂不支持 | 简化本期范围 |

## 4. 架构

```
┌─────────────────────────────────────────────────────────────┐
│                       你的电脑 (Windows)                      │
│                                                              │
│  ┌─────────────────── Tauri 主壳 (~15MB) ──────────────────┐ │
│  │                                                         │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │ │
│  │  │ Web UI       │  │ Rust 后端    │  │ 进程管理器    │  │ │
│  │  │ (设置/历史)  │  │ (Tauri cmd)  │  │ (启停        │  │ │
│  │  │              │  │              │  │  sidecar)    │  │ │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │ │
│  │         │ Tauri IPC       │                  │          │ │
│  └─────────┼─────────────────┼──────────────────┼──────────┘ │
│            │                 │                  │            │
│  ┌─────────▼─────────────────▼─────┐  ┌─────────▼─────────┐ │
│  │  Sidecar 1: whisper-server.exe   │  │ Sidecar 2:        │ │
│  │  (whisper.cpp, 端口 8178)        │  │ llama-server.exe  │ │
│  │  OpenAI 兼容 ASR API             │  │ (llama.cpp, 8188) │ │
│  │  模型: ggml-medium.bin (~1.5GB)  │  │ OpenAI 兼容 Chat  │ │
│  │                                 │  │ 模型: qwen2.5-7b  │ │
│  │                                 │  │ -instruct-q4.gguf │ │
│  │                                 │  │ (~4.5GB)          │ │
│  └─────────────────────────────────┘  └───────────────────┘ │
│                                                              │
│  存储 (SQLite, %APPDATA%/ai-voice-input/)                    │
│  ├─ transcripts.db                                            │
│  ├─ settings.json                                             │
│  └─ models/                                                   │
└─────────────────────────────────────────────────────────────┘
```

## 5. 组件拆分

### 5.1 Rust 后端模块（`src-tauri/src/`）

| 模块 | 职责 | 关键依赖 |
|---|---|---|
| `commands/recording.rs` | start/stop 录音 IPC 命令 | cpal, hound |
| `commands/history.rs` | 查/删历史 IPC 命令 | sqlx |
| `commands/settings.rs` | 读写设置 IPC 命令 | serde_json |
| `commands/models.rs` | 模型下载/状态 IPC 命令 | reqwest, sha2 |
| `hotkey.rs` | 全局快捷键注册 | tauri-plugin-global-shortcut |
| `audio.rs` | 麦克风采集 → 16kHz mono PCM | cpal, rubato |
| `insert.rs` | 剪贴板 + 模拟 Ctrl+V 插入（详见 §6 关键技术点） | enigo, arboard |
| `sidecar.rs` | 启停 sidecar + HTTP 调用 | reqwest, tokio |
| `storage.rs` | SQLite 封装 | sqlx |
| `overlay.rs` | 悬浮胶囊窗口 | tauri::WebviewWindow |

### 5.2 前端（`src/`）

React + TypeScript + Vite。3 个视图：

- **主窗口（设置页）**：模型状态、热键、麦克风选择、清理强度、关于
- **历史窗口**：列表 + 全文搜索（按 SQLite FTS5），单条可复制/删除
- **悬浮胶囊（独立透明窗口）**：录音中显示红点 + 时长，处理中显示「AI 整理中...」

### 5.3 Sidecar 进程

- `whisper-server.exe`：whisper.cpp 官方 HTTP server，加载 medium 模型
- `llama-server.exe`：llama.cpp 官方 HTTP server，加载 Qwen 7B Q4 模型
- 都从 Tauri 资源目录启动，由 `sidecar.rs` 管理生命周期
- 通过 stdout 探测「ready」，通过 HTTP 调用

## 6. 数据流（核心管道）

```
1. 用户按下热键（默认 Ctrl+Shift+Space）
   └─ hotkey.rs 拦截 → emit 'recording-started'
   └─ overlay.rs 在光标附近显示胶囊
2. audio.rs 用 cpal 采集 16kHz mono PCM
   └─ 流式 push 到 ring buffer（最大 60s，作为 buffer 容量上限；正常使用时由步骤 3 的自动停止提前终止）
3. 用户再次按热键（或达到 auto_stop_seconds 后自动停止，默认 30s）
   └─ audio.rs 停止 → 编码 WAV → POST /v1/audio/transcriptions 到 whisper-server
4. whisper-server 返回 segments JSON
   └─ 合并为纯文本 → POST /v1/chat/completions 到 llama-server
      Prompt: "你是语音输入整理助手。去掉口头禅（嗯/啊/那个）、
              修正明显语序错误、添加恰当标点、保留原意。
              只输出整理后的纯文本，不要解释。输入：{raw_text}"
5. llama-server 返回清理后的文本
   └─ 写 SQLite → insert.rs 模拟 Ctrl+V 插入到当前焦点
   └─ 通知前端：胶囊变绿「✓」闪一下 → 自动消失
```

**关键技术点 — 文本插入**：
- 用 `arboard` 备份剪贴板 → 写入新文本 → `enigo` 模拟 `Ctrl+V` → 延迟 50ms → 还原剪贴板
- 插入前检测焦点窗口是否接受文本（通过窗口类名黑名单：cmd、任务管理器等），不接受则只复制到剪贴板并提示

## 7. 数据模型

```sql
CREATE TABLE transcripts (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  raw_text     TEXT NOT NULL,         -- whisper 原文
  clean_text   TEXT NOT NULL,         -- llama 整理后
  duration_ms  INTEGER NOT NULL,      -- 录音时长
  created_at   TEXT NOT NULL,         -- ISO 8601
  app_name     TEXT,                  -- 录音时焦点应用（用于检索）
  word_count   INTEGER GENERATED ALWAYS AS (length(clean_text) - length(replace(clean_text, ' ', '')) + 1) STORED
);

CREATE VIRTUAL TABLE transcripts_fts USING fts5(
  raw_text, clean_text, content='transcripts', content_rowid='id'
);
```

```json
// settings.json
{
  "hotkey": "Ctrl+Shift+Space",
  "mic_device_id": null,
  "auto_stop_seconds": 30,
  "model_variant": "balanced",   // "fast" | "balanced" | "accurate"
  "cleanup_intensity": "normal", // "light" | "normal" | "aggressive"
  "auto_launch": true,
  "overlay_follow_cursor": true
}
```

## 8. 错误处理

| 场景 | 处理 |
|---|---|
| 麦克风权限拒绝 | 胶囊显示「需要麦克风权限」+ 引导跳 Windows 设置页 |
| Sidecar 启动失败 | 自动重试 3 次（间隔 2s）；仍失败则通知用户并保留录音到磁盘 |
| Whisper 超时（>30s 音频） | 强制结束录音，提示「音频过长，请分次录制」 |
| Llama 返回空或异常 | 回退到 whisper 原文插入，记录 warn 日志 |
| 当前窗口不接受文本 | 检测失败则气泡「无法插入，已复制到剪贴板」 |
| 模型文件损坏 | 启动时 SHA256 校验，损坏自动重新下载 |
| 快捷键被占用 | 设置页检测冲突并提示用户改键 |
| 磁盘空间不足 | 下载模型前检查，< 10GB 时提示用户清理 |

**日志**：结构化日志写到 `%LOCALAPPDATA%/ai-voice-input/logs/app-YYYY-MM-DD.log`，UI 设置页可一键打开日志目录。

## 9. 首次启动流程

```
1. Tauri 主壳启动
2. 检查 models/ 目录：两个模型文件都在？SHA256 对？
   └─ 不在 → 弹出引导页：显示硬件检测结果 + 推荐配置
   └─ 用户确认 → 后台下载（带进度条）→ 完成后写 manifests
3. 启动两个 sidecar 进程，等待各自 stdout 报告 "ready"
4. 注册全局快捷键
5. 最小化到托盘，主窗口关闭
6. 监听热键 → 进入录音循环
```

## 10. 测试策略

### 10.1 单元测试（Rust，`cargo test`）

| 模块 | 测试 |
|---|---|
| `audio.rs` | 合成 PCM → 16kHz 重采样 → 字节数正确 |
| `insert.rs` | 在测试窗口 mock 焦点，验证 SendInput 序列 |
| `storage.rs` | 内存 SQLite：CRUD + FTS 搜索 + 触发器 |
| `sidecar.rs` | mock HTTP server：重试、超时、错误码处理 |
| `settings.rs` | JSON 序列化往返 + 默认值合并 |

### 10.2 集成测试

- 录一段固定测试音频（fixture），跑完整管道，对比预期输出文本
- Sidecar 启停顺序、崩溃恢复（kill -9 后自动重启）
- 并发：连续按 5 次热键，验证状态机不出错

### 10.3 手动验收清单

1. ✅ 全新安装 → 首次启动引导下模型
2. ✅ 录音 10s 中英混合 → 文字插入正确
3. ✅ 录音含「嗯」「啊」「那个」→ 输出整理后无口头禅
4. ✅ 长时间录音达到 auto_stop_seconds（默认 30s）→ 自动停止
5. ✅ 重启电脑 → 快捷键和 sidecar 都自动拉起
6. ✅ 故意断网 → 离线模式仍能用
7. ✅ 卸载 → AppData 被清理（除日志外）
8. ✅ VS Code、Word、微信、Edge 各测试一次插入

## 11. 部署与分发

- 安装包：NSIS `.exe`，~15MB（不含模型，模型首次启动下载）
- 系统要求：Windows 10 21H2+ 或 Windows 11
- 推荐硬件：8GB 内存 + 集成显卡；推荐 16GB 内存 + NVIDIA GPU
- 自动更新：v1 不做，用户自行下载新版本；v2 考虑 Tauri's updater

## 12. 风险与权衡

| 风险 | 缓解 |
|---|---|
| Whisper medium 在低端 CPU 上慢（>2x 实时） | 提供「fast」档位（small 模型） |
| Qwen 7B Q4 量化可能偶尔丢细节 | prompt 里强调「保留原意」；后续可换 14B |
| Tauri + sidecar 多进程管理复杂度 | 用 `tauri-plugin-shell` 的 sidecar API，生命周期跟主进程绑定 |
| Windows UI Automation 在某些应用不稳定 | 提供「只复制到剪贴板」fallback |
| 首次下载模型 6GB 体验差 | 引导页明确告知大小和预计时间，用户确认后再下 |

## 13. 后续规划（不在本期）

- 语音指令（「换行」「句号」「撤销」）
- 自定义词库（人名 / 技术术语）
- 按应用配置不同的清理风格（写代码注释 vs 写邮件）
- 多 LLM 模型可选（Llama、DeepSeek、Mistral）
- Tauri updater 自动更新
- 简版 macOS 版本（用 accessibility API）