'use client';

type PanelId = 'graph' | 'vault' | 'history' | 'terminal' | 'doctor' | 'smoke';

type Props = {
  activePanel: PanelId;
  onNavigate: (panel: PanelId) => void;
};

const NAV = [
  { id: 'graph' as const, label: 'Graph', icon: '⬡' },
  { id: 'doctor' as const, label: 'Doctor', icon: '🩺' },
  { id: 'smoke' as const, label: 'Smoke Test', icon: '🐳' },
  { id: 'terminal' as const, label: 'Terminal', icon: '⌨' },
  { id: 'vault' as const, label: 'Vault', icon: '🔐' },
  { id: 'history' as const, label: 'History', icon: '⏱' },
];

export function Sidebar({ activePanel, onNavigate }: Props) {
  return (
    <aside className="flex w-56 flex-col border-r border-space-700 bg-space-950 p-4">
      <div className="mb-8 px-2">
        <span className="font-display text-xs tracking-widest text-rust-orange glow-rust">Fe</span>
        <span className="font-display text-xs tracking-widest text-slate-400">RRUM</span>
      </div>

      <nav className="flex flex-col gap-1">
        {NAV.map((item) => (
          <button
            key={item.id}
            onClick={() => onNavigate(item.id)}
            className={`flex items-center gap-3 rounded px-3 py-2 text-left text-sm transition ${
              activePanel === item.id
                ? 'border border-cyan-neon/30 bg-cyan-neon/10 text-cyan-neon'
                : 'text-slate-400 hover:bg-space-800 hover:text-slate-200'
            }`}
          >
            <span>{item.icon}</span>
            {item.label}
          </button>
        ))}
      </nav>

      <div className="mt-auto border-t border-space-700 pt-4 text-xs text-slate-600">
        Roberto de Souza
        <br />
        v0.1.0
      </div>
    </aside>
  );
}
