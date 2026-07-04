'use client';

import { useCallback, useEffect, useState } from 'react';
import { InfrastructureGraph, type GraphNode } from '@/components/InfrastructureGraph';
import { PlanPanel, type PlanWithCost } from '@/components/PlanPanel';
import { Sidebar } from '@/components/Sidebar';
import { VaultPanel } from '@/components/VaultPanel';

const STATE_PATH: string | null = null;

export default function Dashboard() {
  const [activePanel, setActivePanel] = useState<'graph' | 'vault' | 'history'>('graph');
  const [applying, setApplying] = useState(false);
  const [planning, setPlanning] = useState(false);
  const [plan, setPlan] = useState<PlanWithCost | null>(null);
  const [graphPath, setGraphPath] = useState('ferrum.graph.json');
  const [statePath, setStatePath] = useState<string | null>(null);
  const [graphKey, setGraphKey] = useState(0);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [plannedActions, setPlannedActions] = useState<Record<string, string>>({});
  const [applyStatuses, setApplyStatuses] = useState<Record<string, GraphNode['status']>>({});
  const [showGraphHelp, setShowGraphHelp] = useState(false);

  useEffect(() => {
    async function resolvePaths() {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const resolved = await invoke<{ state_path: string; exists: boolean }>(
          'ferrum_resolve_state',
          { statePath: STATE_PATH }
        );
        setStatePath(resolved.state_path);
        const gPath = await invoke<string>('ferrum_default_graph_path', {
          statePath: resolved.state_path,
        });
        setGraphPath(gPath);
        if (!resolved.exists) {
          setStatusMessage(
            `State not found at ${resolved.state_path}. Run ferrum init in project root.`
          );
        }
      } catch {
        setGraphPath('ferrum.graph.json');
      }
    }
    resolvePaths();
  }, []);

  const reloadGraph = useCallback(() => {
    setGraphKey((k) => k + 1);
  }, []);

  const handlePlan = async () => {
    setPlanning(true);
    setStatusMessage(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<PlanWithCost>('ferrum_plan_with_cost', {
        statePath,
        passphrase: null,
      });
      setPlan(result);
      const actions: Record<string, string> = {};
      for (const c of result.changes) {
        actions[c.address] = c.action;
      }
      setPlannedActions(actions);
    } catch (e) {
      setStatusMessage(String(e));
      setPlan(null);
    } finally {
      setPlanning(false);
    }
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    async function setup() {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen<{ address: string; status: string }>(
          'apply-progress',
          (event) => {
            const status = event.payload.status as GraphNode['status'];
            setApplyStatuses((prev) => ({ ...prev, [event.payload.address]: status }));
            reloadGraph();
          }
        );
      } catch {
        /* not in Tauri */
      }
    }
    setup();
    return () => unlisten?.();
  }, [reloadGraph]);

  const handleApply = async () => {
    setApplying(true);
    setApplyStatuses({});
    setStatusMessage(null);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const result = await invoke<{ applied: boolean; message: string; graph_path: string }>(
        'ferrum_apply',
        { statePath, passphrase: null }
      );
      setStatusMessage(result.message);
      if (result.graph_path) setGraphPath(result.graph_path);
      reloadGraph();
      setPlan(null);
      setPlannedActions({});
    } catch (e) {
      setStatusMessage(String(e));
    } finally {
      setTimeout(() => setApplying(false), 2000);
    }
  };

  return (
    <div className="flex min-h-screen">
      <Sidebar activePanel={activePanel} onNavigate={setActivePanel} />

      <main className="flex-1 p-6">
        <header className="mb-6 flex items-center justify-between border-b border-cyan-neon/20 pb-4">
          <div>
            <h1 className="font-display text-2xl font-bold tracking-wider text-cyan-neon glow-cyan">
              FERRUM
            </h1>
            <p className="text-sm text-slate-500">Infrastructure Control Plane v0.1.0</p>
          </div>
          <div className="flex gap-3">
            <button
              onClick={handlePlan}
              disabled={planning || applying}
              className="rounded border border-cyan-neon/40 px-4 py-2 text-sm text-cyan-neon transition hover:bg-cyan-neon/10 hover:shadow-neon disabled:opacity-50"
            >
              {planning ? 'Planning…' : 'Plan'}
            </button>
            <button
              onClick={handleApply}
              disabled={applying || planning}
              className="rounded bg-rust-orange px-4 py-2 text-sm font-semibold text-white transition hover:bg-rust-ember hover:shadow-rust disabled:opacity-50"
            >
              {applying ? 'Applying…' : 'Apply'}
            </button>
          </div>
        </header>

        {statusMessage && (
          <div className="mb-4 rounded border border-space-700 bg-space-900/80 px-4 py-2 text-sm text-slate-300">
            {statusMessage}
          </div>
        )}

        {(plan || planning) && (
          <PlanPanel plan={plan} loading={planning} onClose={() => setPlan(null)} />
        )}

        {activePanel === 'graph' && (
          <InfrastructureGraph
            key={graphKey}
            graphPath={graphPath}
            applyPulse={applying}
            plannedActions={plannedActions}
            applyStatuses={applyStatuses}
            showHelp={showGraphHelp}
            onToggleHelp={() => setShowGraphHelp((v) => !v)}
          />
        )}
        {activePanel === 'vault' && (
          <VaultPanel
            statePath={statePath}
            onError={(msg) => setStatusMessage(msg)}
            onStatus={(msg) => setStatusMessage(msg)}
          />
        )}
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
