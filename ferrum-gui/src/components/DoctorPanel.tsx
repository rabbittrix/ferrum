'use client';

import { useCallback, useEffect, useState } from 'react';

type CheckStatus = 'pass' | 'warn' | 'fail';

type HealthCheck = {
  name: string;
  status: CheckStatus;
  message: string;
  fix_hint?: string | null;
  help_url?: string | null;
  config_path?: string | null;
};

type DoctorReport = {
  checks: HealthCheck[];
  ferrum_version: string;
  os: string;
  arch: string;
};

function statusIcon(status: CheckStatus) {
  if (status === 'pass') return '✓';
  if (status === 'warn') return '⚠';
  return '✗';
}

function statusClass(status: CheckStatus) {
  if (status === 'pass') return 'text-emerald-400';
  if (status === 'warn') return 'text-amber-400';
  return 'text-red-400';
}

type Props = {
  onRunInTerminal?: (cmd: string) => void;
};

export function DoctorPanel({ onRunInTerminal }: Props) {
  const [report, setReport] = useState<DoctorReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runDoctor = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<DoctorReport>('ferrum_doctor', { version: null });
      setReport(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    runDoctor();
  }, [runDoctor]);

  const openUrl = async (url: string) => {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url.startsWith('http') ? url : `https://github.com/rabbittrix/ferrum/blob/main/${url}`);
    } catch {
      window.open(url, '_blank');
    }
  };

  const openConfig = async (path: string) => {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(path);
    } catch {
      onRunInTerminal?.(`ferrum init`);
    }
  };

  return (
    <div className="rounded-lg border border-space-700 bg-space-900/80 p-6">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h2 className="font-display text-lg text-cyan-neon">System Doctor</h2>
          <p className="text-sm text-slate-500">Step 1 of first-run verification</p>
        </div>
        <button
          onClick={runDoctor}
          disabled={loading}
          className="rounded border border-cyan-neon/40 px-3 py-1.5 text-sm text-cyan-neon hover:bg-cyan-neon/10 disabled:opacity-50"
        >
          {loading ? 'Running…' : 'Re-run'}
        </button>
      </div>

      {error && <p className="mb-4 text-sm text-red-400">{error}</p>}

      {report && (
        <>
          <p className="mb-4 text-xs text-slate-500">
            Ferrum {report.ferrum_version} · {report.os} / {report.arch}
          </p>
          <ul className="space-y-3">
            {report.checks.map((check) => (
              <li
                key={check.name}
                className="rounded border border-space-700 bg-space-950/60 px-4 py-3"
              >
                <div className="flex items-start gap-3">
                  <span className={`text-lg ${statusClass(check.status)}`}>
                    {statusIcon(check.status)}
                  </span>
                  <div className="min-w-0 flex-1">
                    <p className="font-medium text-slate-200">{check.name}</p>
                    <p className="text-sm text-slate-400">{check.message}</p>
                    {(check.status === 'warn' || check.status === 'fail') && (
                      <div className="mt-2 flex flex-wrap gap-2">
                        {check.fix_hint && (
                          <button
                            type="button"
                            onClick={() => {
                              if (check.config_path) openConfig(check.config_path);
                              else onRunInTerminal?.(check.fix_hint!.split(' ').slice(-3).join(' ') || 'ferrum doctor');
                            }}
                            className="rounded bg-rust-orange/20 px-2 py-1 text-xs text-rust-orange hover:bg-rust-orange/30"
                          >
                            Fix It
                          </button>
                        )}
                        {check.help_url && (
                          <button
                            type="button"
                            onClick={() => openUrl(check.help_url!)}
                            className="rounded border border-cyan-neon/30 px-2 py-1 text-xs text-cyan-neon hover:bg-cyan-neon/10"
                          >
                            Help
                          </button>
                        )}
                        {check.fix_hint && (
                          <span className="text-xs text-slate-600">{check.fix_hint}</span>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}
