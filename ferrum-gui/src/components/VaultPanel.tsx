'use client';

export function VaultPanel() {
  return (
    <div className="rounded-lg border border-space-700 bg-space-900/80 p-6">
      <h2 className="mb-2 font-display text-lg text-cyan-neon">Secure Vault</h2>
      <p className="mb-6 text-sm text-slate-400">
        Secrets are AES-256-GCM encrypted in ferrum.fstate — never stored in plain text.
      </p>
      <div className="space-y-3">
        {['AWS_ACCESS_KEY_ID', 'AWS_SECRET_ACCESS_KEY', 'DB_PASSWORD'].map((name) => (
          <div
            key={name}
            className="flex items-center justify-between rounded border border-space-700 bg-space-950 px-4 py-3"
          >
            <span className="font-mono text-sm text-slate-300">{name}</span>
            <span className="font-mono text-sm text-cyan-neon/60">••••••••••••</span>
          </div>
        ))}
      </div>
    </div>
  );
}
