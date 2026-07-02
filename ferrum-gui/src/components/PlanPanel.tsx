'use client';

export type PlanChangeItem = {
  address: string;
  resource_type: string;
  action: string;
  reason: string;
  symbol: string;
};

export type CostLineItem = {
  address: string;
  resource_type: string;
  monthly_usd: number;
  action: string;
};

export type CostEstimate = {
  monthly_delta_usd: number;
  total_monthly_usd: number;
  line_items: CostLineItem[];
  summary: string;
};

export type PlanWithCost = {
  summary_text: string;
  has_changes: boolean;
  create: number;
  update: number;
  delete: number;
  rename: number;
  noop: number;
  cost: CostEstimate;
  changes: PlanChangeItem[];
};

type Props = {
  plan: PlanWithCost | null;
  loading: boolean;
  onClose: () => void;
};

export function PlanPanel({ plan, loading, onClose }: Props) {
  if (loading) {
    return (
      <div className="mb-6 rounded-lg border border-cyan-neon/30 bg-space-900/90 p-6">
        <p className="animate-pulse text-sm text-cyan-neon">Computing plan and cost estimate…</p>
      </div>
    );
  }

  if (!plan) return null;

  const delta = plan.cost.monthly_delta_usd;
  const deltaColor = delta > 0 ? 'text-rust-orange' : delta < 0 ? 'text-green-400' : 'text-slate-400';

  return (
    <div className="mb-6 rounded-lg border border-cyan-neon/30 bg-space-900/90 p-6 shadow-neon">
      <div className="mb-4 flex items-start justify-between">
        <div>
          <h2 className="font-display text-lg text-cyan-neon">Execution Plan</h2>
          <p className="mt-1 text-xs text-slate-500">
            {plan.create} create · {plan.update} update · {plan.delete} delete · {plan.rename} rename ·{' '}
            {plan.noop} unchanged
          </p>
        </div>
        <button
          onClick={onClose}
          className="text-slate-500 transition hover:text-slate-300"
          aria-label="Close plan"
        >
          ✕
        </button>
      </div>

      <div className="mb-4 rounded border border-rust-orange/30 bg-rust-orange/5 px-4 py-3">
        <p className="text-xs uppercase tracking-wider text-slate-500">Cost Estimate</p>
        <p className={`mt-1 font-display text-xl ${deltaColor}`}>{plan.cost.summary}</p>
        <p className="mt-1 text-xs text-slate-500">
          Monthly delta:{' '}
          <span className={deltaColor}>
            {delta >= 0 ? '+' : ''}${delta.toFixed(2)}/mo
          </span>
        </p>
      </div>

      {!plan.has_changes ? (
        <p className="text-sm text-slate-400">Infrastructure is up-to-date — no changes required.</p>
      ) : (
        <ul className="max-h-48 space-y-2 overflow-y-auto font-mono text-xs">
          {plan.changes.map((c) => (
            <li
              key={`${c.symbol}-${c.address}`}
              className="flex gap-2 rounded border border-space-700 bg-space-950/60 px-3 py-2"
            >
              <span
                className={
                  c.action === 'create'
                    ? 'text-green-400'
                    : c.action === 'delete'
                      ? 'text-red-400'
                      : c.action === 'update'
                        ? 'text-amber-400'
                        : 'text-cyan-neon'
                }
              >
                {c.symbol}
              </span>
              <div>
                <span className="text-slate-200">{c.address}</span>
                {c.resource_type && (
                  <span className="ml-2 text-slate-500">({c.resource_type})</span>
                )}
                <p className="text-slate-500">{c.reason}</p>
              </div>
            </li>
          ))}
        </ul>
      )}

      {plan.cost.line_items.length > 0 && (
        <div className="mt-4 border-t border-space-700 pt-4">
          <p className="mb-2 text-xs uppercase tracking-wider text-slate-500">Line Items</p>
          <div className="grid gap-1 font-mono text-xs text-slate-400">
            {plan.cost.line_items.map((item) => (
              <div key={item.address} className="flex justify-between">
                <span>{item.address}</span>
                <span>${item.monthly_usd.toFixed(2)}/mo</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
