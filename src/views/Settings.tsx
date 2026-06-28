import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type Settings = {
  hotkey: string;
  mic_device_id: string | null;
  auto_stop_seconds: number;
  model_variant: 'fast' | 'balanced' | 'accurate';
  cleanup_intensity: 'light' | 'normal' | 'aggressive';
  auto_launch: boolean;
  overlay_follow_cursor: boolean;
  llm_provider: 'local' | 'deepseek' | 'qwen_dashscope';
  llm_api_key: string;
  llm_model: string;
};

type ModelStatus = {
  whisper_installed: boolean;
  sensevoice_installed: boolean;
  llama_model_installed: boolean;
  llama_binary_installed: boolean;
  llama_installed: boolean;
  sidecar_dir: string;
};

const PROVIDER_OPTIONS = [
  { value: 'local', label: '本地 Qwen (4.7GB 下载)', needsKey: false, defaultModel: 'local' },
  { value: 'deepseek', label: '☁️ DeepSeek 在线 API', needsKey: true, defaultModel: 'deepseek-chat' },
  { value: 'qwen_dashscope', label: '☁️ 通义千问 DashScope', needsKey: true, defaultModel: 'qwen-turbo' },
] as const;

const MODEL_PRESETS: Record<string, string[]> = {
  deepseek: ['deepseek-chat', 'deepseek-reasoner'],
  qwen_dashscope: ['qwen-turbo', 'qwen-plus', 'qwen-max', 'qwen-long'],
  local: [],
};

export default function SettingsView() {
  const [s, setS] = useState<Settings | null>(null);
  const [status, setStatus] = useState<ModelStatus>({
    whisper_installed: false,
    sensevoice_installed: false,
    llama_model_installed: false,
    llama_binary_installed: false,
    llama_installed: false,
    sidecar_dir: '',
  });
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<{ name: string; percent: number } | null>(null);
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null);

  useEffect(() => {
    invoke<Settings>('get').then(setS).catch(e => console.error(e));
    invoke<ModelStatus>('status').then(setStatus).catch(() => {});
    listen<{ name: string; percent: number }>('model-download-progress', e => {
      setProgress({ name: e.payload.name, percent: e.payload.percent });
    });
  }, []);

  const save = async () => {
    if (!s) return;
    try {
      await invoke('update', { settings: s });
      alert('已保存');
    } catch (e) {
      alert('保存失败：' + e);
    }
  };

  const download = async () => {
    setDownloading(true);
    try {
      await invoke('download');
      const st = await invoke<ModelStatus>('status');
      setStatus(st);
    } catch (e) {
      alert('下载失败：' + e);
    } finally {
      setDownloading(false);
      setProgress(null);
    }
  };

  const downloadSenseVoice = async () => {
    setDownloading(true);
    try {
      await invoke('download_sensevoice');
      const st = await invoke<ModelStatus>('status');
      setStatus(st);
      alert('SenseVoice 模型已下载。重启 App 后会自动切换到 SenseVoice。');
    } catch (e) {
      alert('SenseVoice 下载失败：' + e);
    } finally {
      setDownloading(false);
      setProgress(null);
    }
  };

  const testCloudKey = async () => {
    if (!s) return;
    setTesting(true);
    setTestResult(null);
    try {
      // Save first so the backend picks up the current key
      await invoke('update', { settings: s });
      const r = await invoke<{ ok: boolean; msg: string }>('test_llm');
      setTestResult(r);
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    } finally {
      setTesting(false);
    }
  };

  const onProviderChange = (p: Settings['llm_provider']) => {
    if (!s) return;
    const preset = PROVIDER_OPTIONS.find(o => o.value === p);
    setS({
      ...s,
      llm_provider: p,
      // Auto-fill the default model for the chosen provider, but only if
      // the model field is empty or is a known preset for the OLD provider.
      llm_model: preset?.defaultModel ?? 'local',
    });
  };

  if (!s) return <div style={{ padding: 24, color: '#94a3b8' }}>加载中…</div>;

  const bothInstalled = status.whisper_installed && status.llama_installed;
  const providerMeta = PROVIDER_OPTIONS.find(o => o.value === s.llm_provider);
  const needsKey = providerMeta?.needsKey ?? false;
  const modelPresets = MODEL_PRESETS[s.llm_provider] ?? [];

  return (
    <div style={{ padding: 24, background: '#0f172a', color: '#f1f5f9', height: '100vh', overflowY: 'auto' }}>
      <h2 style={{ marginTop: 0 }}>设置</h2>

      <fieldset style={{ border: '1px solid #334155', borderRadius: 8, padding: 16, marginBottom: 16 }}>
        <legend>AI 整理模型</legend>
        <p style={{ color: '#94a3b8', marginTop: 0, fontSize: 13 }}>
          选择本地模型还是在线云端大模型。云端 API 更轻量，无需下载 4.7GB 模型文件。
        </p>
        <select
          value={s.llm_provider}
          onChange={e => onProviderChange(e.target.value as Settings['llm_provider'])}
          style={{ width: '100%', padding: 6, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
        >
          {PROVIDER_OPTIONS.map(o => (
            <option key={o.value} value={o.value}>{o.label}</option>
          ))}
        </select>

        {needsKey && (
          <div style={{ marginTop: 12 }}>
            <label style={{ display: 'block', marginBottom: 4, fontSize: 13 }}>
              API Key
              <a
                href={s.llm_provider === 'deepseek' ? 'https://platform.deepseek.com/api_keys' : 'https://dashscope.console.aliyun.com/apiKey'}
                target="_blank"
                rel="noreferrer"
                style={{ marginLeft: 8, color: '#60a5fa', fontSize: 12 }}
              >
                获取 →
              </a>
            </label>
            <div style={{ display: 'flex', gap: 6 }}>
              <input
                type={showKey ? 'text' : 'password'}
                value={s.llm_api_key}
                onChange={e => setS({ ...s, llm_api_key: e.target.value })}
                placeholder={s.llm_provider === 'deepseek' ? 'sk-...' : 'sk-...'}
                style={{ flex: 1, padding: 6, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
              />
              <button
                type="button"
                onClick={() => setShowKey(v => !v)}
                style={{ padding: '6px 10px', background: '#334155', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
              >
                {showKey ? '隐藏' : '显示'}
              </button>
            </div>

            <label style={{ display: 'block', marginTop: 10, marginBottom: 4, fontSize: 13 }}>模型</label>
            <input
              list="llm-model-list"
              value={s.llm_model}
              onChange={e => setS({ ...s, llm_model: e.target.value })}
              placeholder={providerMeta?.defaultModel ?? '模型名'}
              style={{ width: '100%', padding: 6, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
            />
            <datalist id="llm-model-list">
              {modelPresets.map(m => <option key={m} value={m} />)}
            </datalist>

            <button
              type="button"
              onClick={testCloudKey}
              disabled={testing || !s.llm_api_key.trim()}
              style={{ marginTop: 10, padding: '6px 12px', background: '#0ea5e9', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
            >
              {testing ? '测试中…' : '测试连接'}
            </button>
            {testResult && (
              <div style={{ marginTop: 8, fontSize: 13, color: testResult.ok ? '#22c55e' : '#ef4444' }}>
                {testResult.ok ? '✓ ' : '✗ '}{testResult.msg}
              </div>
            )}
          </div>
        )}

        {s.llm_provider === 'local' && (
          <fieldset style={{ border: '1px solid #334155', borderRadius: 6, padding: 12, marginTop: 12 }}>
            <legend style={{ fontSize: 12, color: '#94a3b8' }}>本地模型状态</legend>
            <div>Whisper (英文优先): {status.whisper_installed ? '✓ 已安装' : '✗ 未安装'}</div>
            <div>SenseVoice (中文优化，CPU 快 10×): {status.sensevoice_installed ? '✓ 已安装' : '✗ 未安装'}</div>
            <div style={{ fontSize: 11, color: '#94a3b8', marginTop: 2 }}>
              启动时优先用 SenseVoice，没有时回退 Whisper
            </div>
            <div style={{ marginTop: 6 }}>Qwen 模型文件: {status.llama_model_installed ? '✓ 已下载' : '✗ 未下载'}</div>
            <div>llama-server 二进制: {status.llama_binary_installed ? '✓ 已安装' : '✗ 缺失或为空'}</div>
            {status.llama_model_installed && !status.llama_binary_installed && (
              <div style={{ marginTop: 8, padding: 8, background: '#7c2d12', color: '#fed7aa', borderRadius: 4, fontSize: 12 }}>
                模型文件已下载但 llama-server 二进制缺失，本地 LLM 无法启动。<br />
                路径: <code style={{ fontSize: 11 }}>{status.sidecar_dir}</code><br />
                解决：① 切换到上方"☁️ 在线云端 API" ② 或手动把 llama-server.exe 放到上述目录
              </div>
            )}
            <div style={{ display: 'flex', gap: 8, marginTop: 12, flexWrap: 'wrap' }}>
              {!status.sensevoice_installed && (
                <button
                  onClick={downloadSenseVoice}
                  disabled={downloading}
                  style={{ padding: '6px 12px', background: '#16a34a', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
                >
                  {downloading ? '下载中…' : '下载 SenseVoice (893MB，中文推荐)'}
                </button>
              )}
              {!bothInstalled && (
                <button
                  onClick={download}
                  disabled={downloading}
                  style={{ padding: '6px 12px', background: '#2563eb', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
                >
                  {downloading ? '下载中…' : '下载 Whisper + Qwen (~6GB)'}
                </button>
              )}
            </div>
            {progress && (
              <div style={{ marginTop: 8, color: '#94a3b8' }}>
                {progress.name}: {progress.percent.toFixed(1)}%
              </div>
            )}
            <p style={{ color: '#94a3b8', fontSize: 12, marginTop: 8, marginBottom: 0 }}>
              提示：使用在线云端 API 可避免下载 4.7GB 模型，速度也更快。
            </p>
          </fieldset>
        )}
      </fieldset>

      <fieldset style={{ border: '1px solid #334155', borderRadius: 8, padding: 16, marginBottom: 16 }}>
        <legend>快捷键</legend>
        <input
          value={s.hotkey}
          onChange={e => setS({ ...s, hotkey: e.target.value })}
          style={{ width: '100%', padding: 6, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
        />
        <small style={{ color: '#94a3b8' }}>重启后生效</small>
      </fieldset>

      <fieldset style={{ border: '1px solid #334155', borderRadius: 8, padding: 16, marginBottom: 16 }}>
        <legend>录音</legend>
        <label>
          自动停止 (秒):
          <input
            type="number"
            value={s.auto_stop_seconds}
            min={5}
            max={120}
            onChange={e => setS({ ...s, auto_stop_seconds: Number(e.target.value) })}
            style={{ marginLeft: 8, width: 60, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
          />
        </label>
      </fieldset>

      <fieldset style={{ border: '1px solid #334155', borderRadius: 8, padding: 16, marginBottom: 16 }}>
        <legend>清理强度</legend>
        <select
          value={s.cleanup_intensity}
          onChange={e => setS({ ...s, cleanup_intensity: e.target.value as Settings['cleanup_intensity'] })}
          style={{ padding: 4, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
        >
          <option value="light">轻度（少改动）</option>
          <option value="normal">正常</option>
          <option value="aggressive">强力（更压缩）</option>
        </select>
      </fieldset>

      <button
        onClick={save}
        style={{ padding: '8px 16px', background: '#2563eb', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
      >
        保存设置
      </button>
      <div style={{ marginTop: 16 }}>
        <a href="#/history" style={{ color: '#60a5fa' }}>查看历史</a>
      </div>
    </div>
  );
}