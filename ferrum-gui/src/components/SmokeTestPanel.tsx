'use client';

import { useCallback, useEffect, useState } from 'react';

type SmokeTestResult = {
  success: boolean;
  message: string;
  project_dir: string;
  graph_path: string;
  docker_available: boolean;
};

type Props = {
  onGraphUpdate?: (graphPath: string) => void;
  onStatus?: (msg: string) => void;
};

export function SmokeTestPanel({ onGraphUpdate, onStatus }: Props) {
  const [dockerOk, setDockerOk] = useState<boolean | null>(null);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<SmokeTestResult | null>(null);

  useEffect(() => {
    async function checkDocker() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        setDockerOk(await invoke<boolean>('ferrum_docker_available'));
      } catch {
        setDockerOk(false);
      }
    }
    checkDocker();
  }, []);

  const runSmoke = useCallback(async () => {
    setRunning(true);
    setResult(null);
    onStatus?.('Running smoke test…');
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<SmokeTestResult>('ferrum_test_drive', { cleanup: false });
      setResult(res);
      if (res.success && res.graph_path) {
        onGraphUpdate?.(res.graph_path);
        onStatus?.(res.message);
      } else if (!res.docker_available) {
        onStatus?.('Install Docker to run a test.');
      } else {
        onStatus?.(res.message);
      }
    } catch (e) {
      onStatus?.(String(e));
    } finally {
      setRunning(false);
    }
  }, [onGraphUpdate, onStatus]);

  const cleanup = useCallback(async () => {
    setRunning(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<SmokeTestResult>('ferrum_test_drive', { cleanup: true });
      setResult(res);
      onStatus?.('Smoke test cleaned up.');
    } catch (e) {
      onStatus?.(String(e));
    } finally {
      setRunning(false);
    }
  }, [onStatus]);

  return (
    <div className="rounded-lg border border-space-700 bg-space-900/80 p-6">
      <h2 className="mb-2 font-display text-lg text-cyan-neon">Smoke Test</h2>
      <p className="mb-4 text-sm text-slate-400">
        Deploys a minimal Hello Ferrum nginx container via Docker. The graph node should turn green
        after apply.
      </p>

      {dockerOk === false && (
        <div className="mb-4 rounded border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
          <strong>Install Docker to run a test.</strong> Docker Desktop (Windows) or Docker Engine
          (Linux) must be running. See MANUAL.md#docker-local.
        </div>
      )}

      <div className="flex flex-wrap gap-3">
        <button
          onClick={runSmoke}
          disabled={running || dockerOk === false}
          className="rounded bg-rust-orange px-4 py-2 text-sm font-semibold text-white hover:bg-rust-ember disabled:opacity-50"
        >
          {running ? 'Running…' : 'Run Smoke Test'}
        </button>
        <button
          onClick={cleanup}
          disabled={running}
          className="rounded border border-space-600 px-4 py-2 text-sm text-slate-300 hover:bg-space-800 disabled:opacity-50"
        >
          Auto-Cleanup
        </button>
      </div>

      {result && (
        <div
          className={`mt-4 rounded px-4 py-3 text-sm ${
            result.success ? 'border border-emerald-500/30 text-emerald-300' : 'border border-red-500/30 text-red-300'
          }`}
        >
          {result.message}
          {result.success && result.graph_path && (
            <p className="mt-1 text-xs text-slate-500">Graph: {result.graph_path}</p>
          )}
        </div>
      )}

      <p className="mt-4 text-xs text-slate-600">
        CLI equivalent: <code className="text-slate-400">ferrum test-drive</code> · cleanup:{' '}
        <code className="text-slate-400">ferrum test-drive --cleanup</code>
      </p>
    </div>
  );
}
