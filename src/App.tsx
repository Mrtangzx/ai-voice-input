import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import Overlay from './views/Overlay';
import History from './views/History';
import SettingsView from './views/Settings';
import FirstRun from './views/FirstRun';

type ModelStatus = { whisper_installed: boolean; llama_installed: boolean };
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

type PipelineStatus = {
  phase: 'idle' | 'recording' | 'transcribing' | 'cleaning' | 'done' | 'error';
  text?: string;
  step?: string;
  clean?: string;
  error?: string;
  warning?: string;
};

const PHASE_LABEL: Record<PipelineStatus['phase'], string> = {
  idle: '就绪',
  recording: '🔴 正在听…  再按一次停止',
  transcribing: '🟡 转写中…',
  cleaning: '🔵 AI 整理中…',
  done: '✅ 完成',
  error: '❌ 出错了',
};

const PHASE_COLOR: Record<PipelineStatus['phase'], string> = {
  idle: '#475569',
  recording: '#ef4444',
  transcribing: '#f59e0b',
  cleaning: '#3b82f6',
  done: '#22c55e',
  error: '#ef4444',
};

export default function App() {
  const [route, setRoute] = useState(window.location.hash);
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [pipeline, setPipeline] = useState<PipelineStatus>({ phase: 'idle' });
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const onHash = () => setRoute(window.location.hash);
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  useEffect(() => {
    invoke<ModelStatus>('status').then(setStatus).catch(() => {});
    invoke<Settings>('get').then(setSettings).catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = listen<PipelineStatus>('pipeline-status', e => {
      setPipeline(e.payload);
      // Auto-clear terminal phases so the user knows the next press will work
      if (e.payload.phase === 'done' || e.payload.phase === 'error') {
        setTimeout(() => setPipeline({ phase: 'idle' }), 4000);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);

  if (route.startsWith('#/overlay')) return <Overlay />;
  if (route.startsWith('#/history')) return <History />;
  if (route.startsWith('#/settings')) return <SettingsView />;

  // FirstRun is only needed when:
  //   - Whisper model is missing (always required for ASR), AND
  //   - the user has no cloud LLM configured as a fallback for cleanup
  if (status && settings) {
    const needsWhisper = !status.whisper_installed;
    const hasCloudCleanup =
      (settings.llm_provider === 'deepseek' || settings.llm_provider === 'qwen_dashscope') &&
      settings.llm_api_key.trim().length > 0;
    if (needsWhisper && !hasCloudCleanup) {
      return <FirstRun onConfigured={() => {
        // Re-fetch settings after FirstRun finishes.
        invoke<Settings>('get').then(setSettings).catch(() => {});
        invoke<ModelStatus>('status').then(setStatus).catch(() => {});
      }} />;
    }
  }

  const activePhase = pipeline.phase !== 'idle';

  return (
    <div className="app" style={{ padding: 24, color: '#f1f5f9', background: '#0f172a', minHeight: '100vh' }}>
      <h1 style={{ marginTop: 0 }}>AI 语音输入法</h1>
      <p style={{ color: '#94a3b8' }}>按 <kbd style={{ background: '#334155', padding: '2px 6px', borderRadius: 3 }}>Ctrl+Shift+Space</kbd> 开始录音</p>

      {/* Status banner - shows what's happening right now */}
      <div
        style={{
          marginTop: 16,
          padding: '12px 16px',
          background: activePhase ? '#1e293b' : 'transparent',
          border: activePhase ? `1px solid ${PHASE_COLOR[pipeline.phase]}` : '1px dashed #334155',
          borderRadius: 8,
          color: PHASE_COLOR[pipeline.phase],
          fontSize: 14,
          minHeight: 44,
        }}
      >
        <div style={{ fontWeight: 600, fontSize: 15 }}>
          {PHASE_LABEL[pipeline.phase]}
        </div>
        {pipeline.step && (
          <div style={{ marginTop: 4, color: '#94a3b8', fontSize: 12 }}>{pipeline.step}</div>
        )}
        {pipeline.warning && (
          <div style={{ marginTop: 4, color: '#fbbf24', fontSize: 12 }}>⚠ {pipeline.warning}</div>
        )}
        {pipeline.error && (
          <div style={{ marginTop: 4, color: '#fca5a5', fontSize: 12 }}>{pipeline.error}</div>
        )}
        {pipeline.clean && pipeline.phase === 'done' && (
          <div style={{ marginTop: 6, color: '#e2e8f0', fontSize: 13 }}>
            <strong>已粘贴：</strong>{pipeline.clean}
          </div>
        )}
      </div>

      <p style={{ marginTop: 24 }}>
        <a href="#/history" style={{ color: '#60a5fa', marginRight: 16 }}>历史</a>
        <a href="#/settings" style={{ color: '#60a5fa' }}>设置</a>
      </p>

      <details style={{ marginTop: 32, color: '#64748b', fontSize: 12 }}>
        <summary>调试信息</summary>
        <div style={{ marginTop: 8, fontFamily: 'monospace' }}>
          time: {new Date(now).toLocaleTimeString()}<br />
          whisper: {status?.whisper_installed ? '已安装' : '未安装'}<br />
          llama: {status?.llama_installed ? '已安装' : '未安装'}<br />
          provider: {settings?.llm_provider ?? '...'}
        </div>
      </details>
    </div>
  );
}