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
};

type ModelStatus = { whisper_installed: boolean; llama_installed: boolean };

export default function SettingsView() {
  const [s, setS] = useState<Settings | null>(null);
  const [status, setStatus] = useState<ModelStatus>({ whisper_installed: false, llama_installed: false });
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState<{ name: string; percent: number } | null>(null);

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

  if (!s) return <div style={{ padding: 24, color: '#94a3b8' }}>加载中…</div>;

  const bothInstalled = status.whisper_installed && status.llama_installed;

  return (
    <div style={{ padding: 24, background: '#0f172a', color: '#f1f5f9', height: '100vh', overflowY: 'auto' }}>
      <h2 style={{ marginTop: 0 }}>设置</h2>

      <fieldset style={{ border: '1px solid #334155', borderRadius: 8, padding: 16, marginBottom: 16 }}>
        <legend>模型状态</legend>
        <div>Whisper: {status.whisper_installed ? '✓ 已安装' : '✗ 未安装'}</div>
        <div>Qwen LLM: {status.llama_installed ? '✓ 已安装' : '✗ 未安装'}</div>
        {!bothInstalled && (
          <>
            <button
              onClick={download}
              disabled={downloading}
              style={{ marginTop: 12, padding: '6px 12px', background: '#2563eb', color: 'white', border: 'none', borderRadius: 4, cursor: 'pointer' }}
            >
              {downloading ? '下载中…' : '下载模型 (~6GB)'}
            </button>
            {progress && (
              <div style={{ marginTop: 8, color: '#94a3b8' }}>
                {progress.name}: {progress.percent.toFixed(1)}%
              </div>
            )}
          </>
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