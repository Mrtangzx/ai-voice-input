import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import Overlay from './views/Overlay';
import History from './views/History';
import SettingsView from './views/Settings';
import FirstRun from './views/FirstRun';

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
    invoke<ModelStatus>('status').then(setStatus).catch(() => {});
  }, []);

  if (route.startsWith('#/overlay')) return <Overlay />;
  if (route.startsWith('#/history')) return <History />;
  if (route.startsWith('#/settings')) return <SettingsView />;

  if (status && !(status.whisper_installed && status.llama_installed)) {
    return <FirstRun />;
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