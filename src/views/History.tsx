import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type Transcript = {
  id: number;
  raw_text: string;
  clean_text: string;
  duration_ms: number;
  created_at: string;
  app_name: string | null;
};

function formatTime(iso: string) {
  return new Date(iso).toLocaleString();
}

function formatDuration(ms: number) {
  return `${(ms / 1000).toFixed(1)}s`;
}

export default function History() {
  const [items, setItems] = useState<Transcript[]>([]);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      const res: Transcript[] = query
        ? await invoke('search', { q: query })
        : await invoke('list');
      setItems(res);
    } catch (e) {
      console.error('Failed to load history:', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { refresh(); }, []);
  useEffect(() => {
    const t = setTimeout(refresh, 200);
    return () => clearTimeout(t);
  }, [query]);

  const onDelete = async (id: number) => {
    try {
      await invoke('delete', { id });
      refresh();
    } catch (e) {
      console.error('Failed to delete:', e);
    }
  };

  return (
    <div style={{ padding: 24, height: '100vh', overflowY: 'auto', background: '#0f172a', color: '#f1f5f9' }}>
      <h2 style={{ marginTop: 0 }}>历史记录</h2>
      <input
        placeholder="搜索关键词…"
        value={query}
        onChange={e => setQuery(e.target.value)}
        style={{ width: '100%', padding: 8, background: '#1e293b', color: 'white', border: '1px solid #334155', borderRadius: 4, marginBottom: 16 }}
      />
      {loading && <div style={{ color: '#94a3b8' }}>加载中…</div>}
      {!loading && items.length === 0 && <div style={{ color: '#94a3b8' }}>没有记录</div>}
      {items.map(t => (
        <div key={t.id} style={{ border: '1px solid #334155', borderRadius: 8, padding: 12, marginBottom: 8 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div style={{ flex: 1, color: '#f1f5f9' }}>{t.clean_text}</div>
            <div style={{ display: 'flex', gap: 8, fontSize: 12, color: '#94a3b8' }}>
              <span>{formatTime(t.created_at)}</span>
              <span>{formatDuration(t.duration_ms)}</span>
            </div>
          </div>
          <details style={{ marginTop: 8 }}>
            <summary style={{ cursor: 'pointer', color: '#60a5fa', fontSize: 12 }}>看原文</summary>
            <div style={{ marginTop: 4, padding: 8, background: '#0f172a', borderRadius: 4, fontSize: 12, color: '#94a3b8' }}>
              {t.raw_text}
            </div>
          </details>
          <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
            <button
              onClick={() => navigator.clipboard.writeText(t.clean_text)}
              style={{ background: 'transparent', color: '#60a5fa', border: 'none', cursor: 'pointer' }}
            >
              复制
            </button>
            <button
              onClick={() => onDelete(t.id)}
              style={{ background: 'transparent', color: '#ef4444', border: 'none', cursor: 'pointer' }}
            >
              删除
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}