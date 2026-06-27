import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
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

export default function App() {
  const [route, setRoute] = useState(window.location.hash);
  const [status, setStatus] = useState<ModelStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    const onHash = () => setRoute(window.location.hash);
    window.addEventListener('hashchange', onHash);
    return () => window.removeEventListener('hashchange', onHash);
  }, []);

  useEffect(() => {
    invoke<ModelStatus>('status').then(setStatus).catch(() => {});
    invoke<Settings>('get').then(setSettings).catch(() => {});
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

  return (
    <div className="app">
      <h1>AI 语音输入法</h1>
      <p>按 <kbd>Ctrl+Shift+Space</kbd> 开始录音</p>
      <p>
        <a href="#/history" style={{ color: '#60a5fa', marginRight: 16 }}>历史</a>
        <a href="#/settings" style={{ color: '#60a5fa' }}>设置</a>
      </p>
    </div>
  );
}