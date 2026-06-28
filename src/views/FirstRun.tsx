import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type ModelStatus = {
  whisper_installed: boolean;
  llama_model_installed: boolean;
  llama_binary_installed: boolean;
  llama_installed: boolean;
  sidecar_dir: string;
};
type Mode = 'choose' | 'downloading' | 'cloud' | 'error' | 'done';

export default function FirstRun({ onConfigured }: { onConfigured: () => void }) {
  const [mode, setMode] = useState<Mode>('choose');
  const [percent, setPercent] = useState(0);
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);

  // Provider + key state for cloud mode
  const [provider, setProvider] = useState<'deepseek' | 'qwen_dashscope'>('deepseek');
  const [apiKey, setApiKey] = useState('');
  const [model, setModel] = useState('deepseek-chat');
  const [showKey, setShowKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null);

  useEffect(() => {
    listen<{ name: string; percent: number }>('model-download-progress', e => {
      setName(e.payload.name);
      setPercent(e.payload.percent);
    });
  }, []);

  const startDownload = async () => {
    setMode('downloading');
    setError(null);
    try {
      await invoke('download');
      await invoke<ModelStatus>('status');
      setMode('done');
    } catch (e) {
      setError(String(e));
      setMode('error');
    }
  };

  const testAndSaveCloud = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      await invoke('update', {
        settings: {
          hotkey: 'Ctrl+Shift+Space',
          mic_device_id: null,
          auto_stop_seconds: 30,
          model_variant: 'balanced',
          cleanup_intensity: 'normal',
          auto_launch: true,
          overlay_follow_cursor: true,
          llm_provider: provider,
          llm_api_key: apiKey.trim(),
          llm_model: model.trim() || (provider === 'deepseek' ? 'deepseek-chat' : 'qwen-turbo'),
        },
      });
      const r = await invoke<{ ok: boolean; msg: string }>('test_llm');
      setTestResult(r);
      if (r.ok) {
        onConfigured();
        location.hash = '#/';
      }
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    } finally {
      setTesting(false);
    }
  };

  // Cloud-config UI
  if (mode === 'cloud' || mode === 'choose') {
    if (mode === 'choose') {
      return (
        <div style={{ padding: 48, background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
          <h1>欢迎使用 AI 语音输入法</h1>
          <p style={{ color: '#94a3b8' }}>
            首次使用需要下载 <strong>Whisper 语音识别模型</strong>（~1.5GB）。AI 文本整理可以选用本地大模型或在线 API。
          </p>

          <div style={{ marginTop: 24, display: 'grid', gap: 12 }}>
            <button
              onClick={() => { setMode('cloud'); setProvider('deepseek'); setModel('deepseek-chat'); }}
              style={{ padding: '16px 20px', background: '#0ea5e9', color: 'white', border: 'none', borderRadius: 8, cursor: 'pointer', textAlign: 'left', borderLeft: '4px solid #38bdf8' }}
            >
              <div style={{ fontSize: 16, fontWeight: 600 }}>☁️ 推荐：使用云端 API</div>
              <div style={{ fontSize: 12, opacity: 0.85, marginTop: 4 }}>
                仅下载 Whisper 模型（~1.5GB），AI 整理用 DeepSeek/通义千问在线 API。启动快、效果更好。
              </div>
            </button>

            <button
              onClick={startDownload}
              style={{ padding: '16px 20px', background: '#2563eb', color: 'white', border: 'none', borderRadius: 8, cursor: 'pointer', textAlign: 'left' }}
            >
              <div style={{ fontSize: 16, fontWeight: 600 }}>📦 完全离线：下载所有本地模型</div>
              <div style={{ fontSize: 12, opacity: 0.85, marginTop: 4 }}>
                Whisper + 本地 Qwen LLM（约 6GB）。需要联网下载且磁盘空间充足。
              </div>
            </button>
          </div>

          <p style={{ color: '#94a3b8', fontSize: 12, marginTop: 24 }}>
            如果本地 LLM 二进制（llama-server.exe）下载失败，App 会自动用原始转写结果，不需要重装。
          </p>
        </div>
      );
    }

    // mode === 'cloud'
    return (
      <div style={{ padding: 48, background: '#0f172a', color: '#f1f5f9', height: '100vh', overflowY: 'auto' }}>
        <h1>配置云端 AI</h1>
        <p style={{ color: '#94a3b8' }}>
          选择一个云端 API 提供方并填入 API Key。Whisper 模型仍会下载以保证离线语音识别。
        </p>

        <div style={{ marginTop: 20 }}>
          <label style={{ display: 'block', marginBottom: 8 }}>服务提供方</label>
          <div style={{ display: 'flex', gap: 10 }}>
            <button
              onClick={() => { setProvider('deepseek'); setModel('deepseek-chat'); }}
              style={{
                flex: 1, padding: 12,
                background: provider === 'deepseek' ? '#2563eb' : '#1e293b',
                color: 'white', border: '1px solid #334155', borderRadius: 6, cursor: 'pointer',
              }}
            >
              DeepSeek
              <div style={{ fontSize: 11, opacity: 0.8 }}>¥1/M tokens</div>
            </button>
            <button
              onClick={() => { setProvider('qwen_dashscope'); setModel('qwen-turbo'); }}
              style={{
                flex: 1, padding: 12,
                background: provider === 'qwen_dashscope' ? '#2563eb' : '#1e293b',
                color: 'white', border: '1px solid #334155', borderRadius: 6, cursor: 'pointer',
              }}
            >
              通义千问 DashScope
              <div style={{ fontSize: 11, opacity: 0.8 }}>qwen-turbo 免费额度</div>
            </button>
          </div>
        </div>

        <div style={{ marginTop: 16 }}>
          <label style={{ display: 'block', marginBottom: 4 }}>
            API Key
            <a
              href={provider === 'deepseek' ? 'https://platform.deepseek.com/api_keys' : 'https://dashscope.console.aliyun.com/apiKey'}
              target="_blank" rel="noreferrer"
              style={{ marginLeft: 8, color: '#60a5fa', fontSize: 12 }}
            >获取 →</a>
          </label>
          <div style={{ display: 'flex', gap: 6 }}>
            <input
              type={showKey ? 'text' : 'password'}
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
              placeholder="sk-..."
              style={{ flex: 1, padding: 6, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
            />
            <button
              onClick={() => setShowKey(v => !v)}
              style={{ padding: '6px 10px', background: '#334155', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
            >{showKey ? '隐藏' : '显示'}</button>
          </div>
        </div>

        <div style={{ marginTop: 12 }}>
          <label style={{ display: 'block', marginBottom: 4 }}>模型名</label>
          <input
            list="firstrun-model-list"
            value={model}
            onChange={e => setModel(e.target.value)}
            style={{ width: '100%', padding: 6, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4 }}
          />
          <datalist id="firstrun-model-list">
            {(provider === 'deepseek'
              ? ['deepseek-chat', 'deepseek-reasoner']
              : ['qwen-turbo', 'qwen-plus', 'qwen-max', 'qwen-long']
            ).map(m => <option key={m} value={m} />)}
          </datalist>
        </div>

        <div style={{ marginTop: 20, display: 'flex', gap: 8 }}>
          <button
            onClick={() => setMode('choose')}
            style={{ padding: '8px 16px', background: '#334155', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
          >返回</button>
          <button
            onClick={testAndSaveCloud}
            disabled={testing || !apiKey.trim()}
            style={{ padding: '8px 16px', background: '#0ea5e9', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
          >{testing ? '测试中…' : '保存并继续下载 Whisper'}</button>
        </div>

        {testResult && (
          <div style={{ marginTop: 12, color: testResult.ok ? '#22c55e' : '#ef4444', fontSize: 13 }}>
            {testResult.ok ? '✓ ' : '✗ '}{testResult.msg}
          </div>
        )}
      </div>
    );
  }

  if (mode === 'downloading') {
    return (
      <div style={{ padding: 48, background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
        <h1>下载模型中…</h1>
        <div style={{ marginTop: 24, padding: 16, background: '#1e293b', borderRadius: 8 }}>
          <div style={{ marginBottom: 8 }}>正在下载: {name || '准备中…'}</div>
          <div style={{ height: 8, background: '#334155', borderRadius: 4, overflow: 'hidden' }}>
            <div style={{ width: `${percent}%`, height: '100%', background: '#2563eb', transition: 'width 0.2s' }} />
          </div>
          <div style={{ marginTop: 8, textAlign: 'right', color: '#94a3b8' }}>{percent.toFixed(1)}%</div>
        </div>
      </div>
    );
  }

  if (mode === 'done') {
    return (
      <div style={{ padding: 48, textAlign: 'center', background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
        <h1>准备好了！</h1>
        <p>模型下载完成。现在可以按 <kbd>Ctrl+Shift+Space</kbd> 开始录音。</p>
        <a href="#/" onClick={onConfigured} style={{ color: '#60a5fa' }}>打开主界面</a>
      </div>
    );
  }

  // mode === 'error'
  return (
    <div style={{ padding: 48, background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
      <h1>下载失败</h1>
      <p style={{ color: '#ef4444' }}>{error}</p>
      <div style={{ display: 'flex', gap: 8 }}>
        <button onClick={() => setMode('choose')} style={{ padding: '8px 16px', background: '#2563eb', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}>返回</button>
        <button onClick={() => location.reload()} style={{ padding: '8px 16px', background: '#334155', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}>重试</button>
      </div>
    </div>
  );
}