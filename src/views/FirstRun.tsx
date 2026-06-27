import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function FirstRun() {
  const [percent, setPercent] = useState(0);
  const [name, setName] = useState('');
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listen<{ name: string; percent: number }>('model-download-progress', e => {
      setName(e.payload.name);
      setPercent(e.payload.percent);
    });
    invoke('download')
      .then(() => setDone(true))
      .catch(e => setError(String(e)));
  }, []);

  if (error) {
    return (
      <div style={{ padding: 48, background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
        <h1>下载失败</h1>
        <p style={{ color: '#ef4444' }}>{error}</p>
        <button
          onClick={() => location.reload()}
          style={{ padding: '8px 16px', background: '#2563eb', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
        >
          重试
        </button>
      </div>
    );
  }

  if (done) {
    return (
      <div style={{ padding: 48, textAlign: 'center', background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
        <h1>准备好了！</h1>
        <p>模型下载完成。现在可以按 <kbd>Ctrl+Shift+Space</kbd> 开始录音。</p>
        <a href="#/" style={{ color: '#60a5fa' }}>打开主界面</a>
      </div>
    );
  }

  return (
    <div style={{ padding: 48, background: '#0f172a', color: '#f1f5f9', height: '100vh' }}>
      <h1>欢迎使用 AI 语音输入法</h1>
      <p>首次使用需要下载模型文件（约 6GB），请保持网络连接。</p>
      <div style={{ marginTop: 24, padding: 16, background: '#1e293b', borderRadius: 8 }}>
        <div style={{ marginBottom: 8 }}>正在下载: {name || '准备中…'}</div>
        <div style={{ height: 8, background: '#334155', borderRadius: 4, overflow: 'hidden' }}>
          <div
            style={{
              width: `${percent}%`,
              height: '100%',
              background: '#2563eb',
              transition: 'width 0.2s',
            }}
          />
        </div>
        <div style={{ marginTop: 8, textAlign: 'right', color: '#94a3b8' }}>
          {percent.toFixed(1)}%
        </div>
      </div>
    </div>
  );
}