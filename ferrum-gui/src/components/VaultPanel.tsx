'use client';

import { useCallback, useEffect, useState, type ReactNode } from 'react';

type VaultRow = {
  name: string;
  value: string;
  visible: boolean;
  editing: boolean;
  draft: string;
  loaded: boolean;
};

type Props = {
  statePath: string | null;
  onError?: (msg: string) => void;
  onStatus?: (msg: string) => void;
};

function IconBtn({
  label,
  onClick,
  className = '',
  children,
}: {
  label: string;
  onClick: () => void;
  className?: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      aria-label={label}
      className={`rounded p-1.5 text-slate-400 transition hover:bg-space-800 ${className}`}
    >
      {children}
    </button>
  );
}

const iconClass = 'h-4 w-4';

function EyeIcon() {
  return (
    <svg className={iconClass} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function EyeOffIcon() {
  return (
    <svg className={iconClass} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M9.88 9.88a3 3 0 1 0 4.24 4.24" />
      <path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68" />
      <path d="M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61" />
      <line x1="2" x2="22" y1="2" y2="22" />
    </svg>
  );
}

function PencilIcon() {
  return (
    <svg className={iconClass} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg className={iconClass} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg className={iconClass} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}

function XIcon() {
  return (
    <svg className={iconClass} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  );
}

export function VaultPanel({ statePath, onError, onStatus }: Props) {
  const [rows, setRows] = useState<VaultRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [newKeyName, setNewKeyName] = useState('');
  const [resolvedPath, setResolvedPath] = useState<string | null>(statePath);

  const loadVault = useCallback(async () => {
    setLoading(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const list = await invoke<{ secrets: { name: string }[]; state_path: string }>(
        'ferrum_vault_list',
        { statePath, passphrase: null }
      );
      setResolvedPath(list.state_path);
      setRows(
        list.secrets.map((s) => ({
          name: s.name,
          value: '',
          visible: false,
          editing: false,
          draft: '',
          loaded: false,
        }))
      );
    } catch (e) {
      onError?.(String(e));
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, [statePath, onError]);

  useEffect(() => {
    loadVault();
  }, [loadVault]);

  const reveal = async (name: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const res = await invoke<{ name: string; value: string }>('ferrum_vault_reveal', {
        name,
        statePath: resolvedPath,
        passphrase: null,
      });
      setRows((prev) =>
        prev.map((r) =>
          r.name === name
            ? { ...r, value: res.value, visible: true, loaded: true, draft: res.value }
            : r
        )
      );
    } catch (e) {
      onError?.(String(e));
    }
  };

  const hide = (name: string) => {
    setRows((prev) =>
      prev.map((r) => (r.name === name ? { ...r, visible: false } : r))
    );
  };

  const startEdit = async (name: string) => {
    const row = rows.find((r) => r.name === name);
    if (!row?.loaded) {
      await reveal(name);
    }
    setRows((prev) =>
      prev.map((r) =>
        r.name === name
          ? {
              ...r,
              editing: true,
              draft: r.loaded ? r.value : r.draft,
              visible: true,
            }
          : r
      )
    );
  };

  const cancelEdit = (name: string) => {
    setRows((prev) =>
      prev.map((r) =>
        r.name === name ? { ...r, editing: false, draft: r.value, visible: false } : r
      )
    );
  };

  const save = async (name: string) => {
    const row = rows.find((r) => r.name === name);
    if (!row) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('ferrum_vault_set', {
        name,
        value: row.draft,
        statePath: resolvedPath,
        passphrase: null,
      });
      setRows((prev) =>
        prev.map((r) =>
          r.name === name
            ? {
                ...r,
                value: r.draft,
                editing: false,
                loaded: true,
                visible: false,
              }
            : r
        )
      );
      onStatus?.(`Secret '${name}' saved (AES-256-GCM encrypted).`);
    } catch (e) {
      onError?.(String(e));
    }
  };

  const addSecret = async () => {
    const name = newKeyName.trim().toUpperCase().replace(/\s+/g, '_');
    if (!name) return;
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('ferrum_vault_add', {
        name,
        statePath: resolvedPath,
        passphrase: null,
      });
      setNewKeyName('');
      await loadVault();
      onStatus?.(`Secret '${name}' added.`);
    } catch (e) {
      onError?.(String(e));
    }
  };

  const removeSecret = async (name: string) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('ferrum_vault_delete', {
        name,
        statePath: resolvedPath,
        passphrase: null,
      });
      await loadVault();
      onStatus?.(`Secret '${name}' removed.`);
    } catch (e) {
      onError?.(String(e));
    }
  };

  const mask = (len: number) => '•'.repeat(Math.max(len, 12));

  return (
    <div className="rounded-lg border border-space-700 bg-space-900/80 p-6">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="font-display text-lg text-cyan-neon">Secure Vault</h2>
          <p className="mt-1 text-sm text-slate-400">
            Secrets are AES-256-GCM encrypted in ferrum.fstate — never stored in plain text.
          </p>
          {resolvedPath && (
            <p className="mt-1 font-mono text-xs text-slate-600">{resolvedPath}</p>
          )}
        </div>
        <button
          type="button"
          onClick={loadVault}
          className="rounded border border-space-700 px-3 py-1 text-xs text-slate-400 hover:border-cyan-neon/40 hover:text-cyan-neon"
        >
          Refresh
        </button>
      </div>

      {loading ? (
        <p className="animate-pulse text-sm text-slate-500">Loading vault…</p>
      ) : rows.length === 0 ? (
        <p className="text-sm text-slate-500">
          No secrets found. Run <code className="text-cyan-neon">ferrum init</code> in the project root
          or add a secret below.
        </p>
      ) : (
        <div className="space-y-3">
          {rows.map((row) => (
            <div
              key={row.name}
              className="rounded border border-space-700 bg-space-950 px-4 py-3"
            >
              <div className="flex flex-wrap items-center gap-3">
                <span className="min-w-[10rem] font-mono text-sm text-slate-300">{row.name}</span>

                {row.editing ? (
                  <input
                    type="text"
                    autoFocus
                    value={row.draft}
                    onChange={(e) =>
                      setRows((prev) =>
                        prev.map((r) =>
                          r.name === row.name ? { ...r, draft: e.target.value } : r
                        )
                      )
                    }
                    className="flex-1 rounded border border-cyan-neon/30 bg-space-900 px-3 py-1.5 font-mono text-sm text-cyan-neon outline-none focus:border-cyan-neon"
                    placeholder="Enter secret value…"
                  />
                ) : (
                  <span className="flex-1 font-mono text-sm text-cyan-neon/80">
                    {row.visible
                      ? row.value || <span className="text-slate-600">(empty)</span>
                      : mask(row.loaded ? row.value.length : 12)}
                  </span>
                )}

                <div className="flex shrink-0 items-center gap-0.5">
                  {!row.editing && (
                    <>
                      <IconBtn
                        label={row.visible ? 'Hide secret' : 'Show secret'}
                        onClick={() => (row.visible ? hide(row.name) : reveal(row.name))}
                        className="hover:text-cyan-neon"
                      >
                        {row.visible ? <EyeOffIcon /> : <EyeIcon />}
                      </IconBtn>
                      <IconBtn
                        label="Edit secret"
                        onClick={() => startEdit(row.name)}
                        className="hover:text-rust-orange"
                      >
                        <PencilIcon />
                      </IconBtn>
                      <IconBtn
                        label="Delete secret"
                        onClick={() => removeSecret(row.name)}
                        className="hover:text-red-400"
                      >
                        <TrashIcon />
                      </IconBtn>
                    </>
                  )}
                  {row.editing && (
                    <>
                      <IconBtn
                        label="Save secret"
                        onClick={() => save(row.name)}
                        className="text-rust-orange hover:bg-rust-orange/10 hover:text-rust-orange"
                      >
                        <CheckIcon />
                      </IconBtn>
                      <IconBtn
                        label="Cancel edit"
                        onClick={() => cancelEdit(row.name)}
                        className="hover:text-slate-200"
                      >
                        <XIcon />
                      </IconBtn>
                    </>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="mt-6 flex flex-wrap gap-2 border-t border-space-700 pt-4">
        <input
          type="text"
          value={newKeyName}
          onChange={(e) => setNewKeyName(e.target.value)}
          placeholder="NEW_SECRET_NAME"
          className="flex-1 rounded border border-space-700 bg-space-950 px-3 py-2 font-mono text-sm text-slate-300 outline-none focus:border-cyan-neon/40"
        />
        <button
          type="button"
          onClick={addSecret}
          className="rounded border border-cyan-neon/40 px-4 py-2 text-sm text-cyan-neon hover:bg-cyan-neon/10"
        >
          + Add Secret
        </button>
      </div>
    </div>
  );
}
