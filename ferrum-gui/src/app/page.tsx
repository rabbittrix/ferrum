'use client';

import { useState } from 'react';
import { InfrastructureGraph } from '@/components/InfrastructureGraph';
import { Sidebar } from '@/components/Sidebar';
import { VaultPanel } from '@/components/VaultPanel';

export default function Dashboard() {
  const [activePanel, setActivePanel] = useState<'graph' | 'vault' | 'history'>('graph');

  return (
    <div className="flex min-h-screen">
      <Sidebar activePanel={activePanel} onNavigate={setActivePanel} />

      <main className="flex-1 p-6">
        <header className="mb-8 flex items-center justify-between border-b border-cyan-neon/20 pb-4">
          <div>
            <h1 className="font-display text-2xl font-bold tracking-wider text-cyan-neon glow-cyan">
              FERRUM
            </h1>
            <p className="text-sm text-slate-500">Infrastructure Control Plane v0.1.0</p>
          </div>
          <div className="flex gap-3">
            <button className="rounded border border-cyan-neon/40 px-4 py-2 text-sm text-cyan-neon transition hover:bg-cyan-neon/10 hover:shadow-neon">
              Plan
            </button>
            <button className="rounded bg-rust-orange px-4 py-2 text-sm font-semibold text-white transition hover:bg-rust-ember hover:shadow-rust">
              Apply
            </button>
          </div>
        </header>

        {activePanel === 'graph' && <InfrastructureGraph />}
        {activePanel === 'vault' && <VaultPanel />}
        {activePanel === 'history' && (
          <div className="rounded-lg border border-space-700 bg-space-900/80 p-6">
            <h2 className="mb-4 font-display text-lg text-cyan-neon">State History</h2>
            <p className="text-slate-400">Timeline of encrypted state revisions will appear here.</p>
          </div>
        )}
      </main>
    </div>
  );
}
