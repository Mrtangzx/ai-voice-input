import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

type Phase = 'recording' | 'transcribing' | 'cleaning' | 'done' | 'error';

export default function Overlay() {
  const [phase, setPhase] = useState<Phase>('recording');

  useEffect(() => {
    const p = listen<{ phase: Phase; text?: string }>('overlay-update', e => {
      setPhase(e.payload.phase);
      void e.payload.text;
    });
    return () => { p.then(u => u()); };
  }, []);

  const color = {
    recording: '#ef4444',
    transcribing: '#f59e0b',
    cleaning: '#f59e0b',
    done: '#22c55e',
    error: '#ef4444',
  }[phase];

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'transparent',
        pointerEvents: 'none',
      }}
    >
      <div
        style={{
          padding: '8px 14px',
          borderRadius: 24,
          background: 'rgba(15,23,42,0.92)',
          color: 'white',
          fontSize: 13,
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
          border: `1px solid ${color}`,
        }}
      >
        <div
          style={{
            width: 8,
            height: 8,
            borderRadius: '50%',
            background: color,
            animation: phase === 'recording' ? 'pulse 1s infinite' : 'none',
          }}
        />
        <span>
          {phase === 'recording' ? '正在听…' :
            phase === 'transcribing' ? '转录中…' :
              phase === 'cleaning' ? '整理中…' :
                phase === 'done' ? '✓ 已插入' : '出错了'}
        </span>
      </div>
      <style>{`@keyframes pulse { 0%,100% { opacity: 1 } 50% { opacity: 0.3 } }`}</style>
    </div>
  );
}