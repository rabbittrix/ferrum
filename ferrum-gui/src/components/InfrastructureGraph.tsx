'use client';

import { useCallback, useEffect, useRef, useState } from 'react';

export type GraphNode = {
  id: string;
  address: string;
  resource_type: string;
  provider: string;
  has_sensitive: boolean;
  status: 'pending' | 'creating' | 'active' | 'failed' | 'drifted';
  x: number;
  y: number;
};

export type GraphEdge = { from: string; to: string };

export type InfrastructureGraphData = {
  nodes: GraphNode[];
  edges: GraphEdge[];
};

const STATUS_STROKE: Record<GraphNode['status'], string> = {
  pending: '#64748b',
  creating: '#3b82f6',
  active: '#22c55e',
  failed: '#ef4444',
  drifted: '#f59e0b',
};

const PLAN_STROKE: Record<string, string> = {
  create: '#22c55e',
  update: '#f59e0b',
  delete: '#ef4444',
  rename: '#00e5ff',
};

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const VIEW_W = 640;
const VIEW_H = 420;

const DEMO: InfrastructureGraphData = {
  nodes: [
    { id: 'aws_vpc.main', address: 'aws_vpc.main', resource_type: 'aws_vpc', provider: 'aws', has_sensitive: false, status: 'active', x: 300, y: 80 },
    { id: 'aws_subnet.public', address: 'aws_subnet.public', resource_type: 'aws_subnet', provider: 'aws', has_sensitive: false, status: 'active', x: 200, y: 200 },
    { id: 'aws_instance.web', address: 'aws_instance.web', resource_type: 'aws_instance', provider: 'aws', has_sensitive: true, status: 'creating', x: 300, y: 320 },
    { id: 'aws_security_group.web', address: 'aws_security_group.web', resource_type: 'aws_security_group', provider: 'aws', has_sensitive: false, status: 'active', x: 460, y: 200 },
  ],
  edges: [
    { from: 'aws_vpc.main', to: 'aws_subnet.public' },
    { from: 'aws_subnet.public', to: 'aws_instance.web' },
    { from: 'aws_security_group.web', to: 'aws_instance.web' },
  ],
};

type Props = {
  graphPath?: string;
  applyPulse?: boolean;
  plannedActions?: Record<string, string>;
};

function clampZoom(z: number) {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
}

function fitView(nodes: GraphNode[]): { zoom: number; offset: { x: number; y: number } } {
  if (nodes.length === 0) return { zoom: 1, offset: { x: 0, y: 0 } };

  const xs = nodes.map((n) => n.x);
  const ys = nodes.map((n) => n.y);
  const minX = Math.min(...xs) - 100;
  const maxX = Math.max(...xs) + 100;
  const minY = Math.min(...ys) - 40;
  const maxY = Math.max(...ys) + 40;
  const contentW = maxX - minX;
  const contentH = maxY - minY;

  const zoom = clampZoom(Math.min(VIEW_W / contentW, VIEW_H / contentH, 1.5));
  const offsetX = (VIEW_W - contentW * zoom) / 2 - minX * zoom;
  const offsetY = (VIEW_H - contentH * zoom) / 2 - minY * zoom;

  return { zoom, offset: { x: offsetX, y: offsetY } };
}

export function InfrastructureGraph({ graphPath, applyPulse, plannedActions = {} }: Props) {
  const [graph, setGraph] = useState<InfrastructureGraphData>(DEMO);
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const lastPointer = useRef({ x: 0, y: 0 });
  const svgRef = useRef<SVGSVGElement>(null);

  const resetView = useCallback((nodes: GraphNode[]) => {
    const fit = fitView(nodes);
    setZoom(fit.zoom);
    setOffset(fit.offset);
  }, []);

  useEffect(() => {
    async function load() {
      if (!graphPath) return;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const data = await invoke<InfrastructureGraphData>('ferrum_load_graph', { graphPath });
        if (data.nodes.length > 0) {
          setGraph(data);
          resetView(data.nodes);
        }
      } catch {
        resetView(DEMO.nodes);
      }
    }
    load();
  }, [graphPath, resetView]);

  const zoomAt = useCallback(
    (clientX: number, clientY: number, factor: number) => {
      const svg = svgRef.current;
      if (!svg) return;
      const rect = svg.getBoundingClientRect();
      const mx = clientX - rect.left;
      const my = clientY - rect.top;
      const sx = (mx - offset.x) / zoom;
      const sy = (my - offset.y) / zoom;
      const newZoom = clampZoom(zoom * factor);
      setOffset({ x: mx - sx * newZoom, y: my - sy * newZoom });
      setZoom(newZoom);
    },
    [offset, zoom]
  );

  const zoomAtCenter = useCallback(
    (factor: number) => {
      const svg = svgRef.current;
      if (!svg) return;
      const rect = svg.getBoundingClientRect();
      zoomAt(rect.left + rect.width / 2, rect.top + rect.height / 2, factor);
    },
    [zoomAt]
  );

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      zoomAt(e.clientX, e.clientY, e.deltaY < 0 ? 1.12 : 1 / 1.12);
    },
    [zoomAt]
  );

  const handlePointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    (e.currentTarget as SVGSVGElement).setPointerCapture(e.pointerId);
    setDragging(true);
    lastPointer.current = { x: e.clientX, y: e.clientY };
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (!dragging) return;
    const dx = e.clientX - lastPointer.current.x;
    const dy = e.clientY - lastPointer.current.y;
    lastPointer.current = { x: e.clientX, y: e.clientY };
    setOffset((o) => ({ x: o.x + dx, y: o.y + dy }));
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    setDragging(false);
    try {
      (e.currentTarget as SVGSVGElement).releasePointerCapture(e.pointerId);
    } catch {
      /* already released */
    }
  };

  const nodeById = (id: string) => graph.nodes.find((n) => n.id === id || n.address === id);

  return (
    <div className="rounded-lg border border-space-700 bg-space-900/80 p-4">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <h2 className="font-display text-lg text-cyan-neon">Infrastructure Graph</h2>

        <div className="flex items-center gap-2">
          <div className="flex overflow-hidden rounded border border-space-700 text-xs">
            <button
              type="button"
              onClick={() => zoomAtCenter(1 / 1.2)}
              className="px-2.5 py-1 text-slate-300 transition hover:bg-space-800"
              aria-label="Zoom out"
            >
              −
            </button>
            <span className="border-x border-space-700 px-2.5 py-1 font-mono text-cyan-neon/80">
              {Math.round(zoom * 100)}%
            </span>
            <button
              type="button"
              onClick={() => zoomAtCenter(1.2)}
              className="px-2.5 py-1 text-slate-300 transition hover:bg-space-800"
              aria-label="Zoom in"
            >
              +
            </button>
          </div>
          <button
            type="button"
            onClick={() => resetView(graph.nodes)}
            className="rounded border border-space-700 px-2.5 py-1 text-xs text-slate-400 transition hover:border-cyan-neon/40 hover:text-cyan-neon"
          >
            Fit
          </button>
        </div>

        <div className="flex gap-3 text-xs text-slate-500">
          <span><span className="inline-block h-2 w-2 rounded-full bg-blue-500" /> Creating</span>
          <span><span className="inline-block h-2 w-2 rounded-full bg-green-500" /> Complete</span>
          <span><span className="inline-block h-2 w-2 rounded-full bg-red-500" /> Error</span>
          <span>🔐 Vault</span>
        </div>
      </div>

      <p className="mb-2 text-xs text-slate-600">
        Scroll to zoom · drag to pan
      </p>

      <svg
        ref={svgRef}
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
        className={`h-[28rem] w-full touch-none select-none rounded border border-space-800 bg-space-950/50 ${
          dragging ? 'cursor-grabbing' : 'cursor-grab'
        }`}
        onWheel={handleWheel}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
      >
        <defs>
          <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
            <path d="M0,0 L6,3 L0,6" fill="#00e5ff" opacity="0.6" />
          </marker>
        </defs>

        <g transform={`translate(${offset.x}, ${offset.y}) scale(${zoom})`}>
          {graph.edges.map((e) => {
            const from = nodeById(e.from);
            const to = nodeById(e.to);
            if (!from || !to) return null;
            return (
              <line
                key={`${e.from}-${e.to}`}
                x1={from.x}
                y1={from.y + 24}
                x2={to.x}
                y2={to.y - 24}
                stroke="#00e5ff"
                strokeOpacity={0.4}
                strokeWidth={1.5 / zoom}
                markerEnd="url(#arrow)"
              />
            );
          })}

          {graph.nodes.map((node) => {
            const planned = plannedActions[node.address];
            const stroke = planned ? PLAN_STROKE[planned] ?? '#00e5ff' : STATUS_STROKE[node.status];
            const pulse = applyPulse && (node.status === 'creating' || planned === 'create');
            const dash = planned === 'delete' ? '6 3' : undefined;
            return (
              <g key={node.id} transform={`translate(${node.x - 84}, ${node.y - 24})`}>
                <rect
                  width={168}
                  height={48}
                  rx={4}
                  fill="#111827"
                  stroke={stroke}
                  strokeWidth={(planned || pulse ? 2.5 : 1.5) / zoom}
                  strokeDasharray={dash}
                  opacity={pulse ? 0.95 : 1}
                >
                  {pulse && (
                    <animate attributeName="stroke-opacity" values="1;0.4;1" dur="1.2s" repeatCount="indefinite" />
                  )}
                </rect>
                {node.has_sensitive && (
                  <g transform="translate(148, 4)">
                    <rect width={14} height={14} rx={2} fill="#0a0f1a" stroke="#f74c00" strokeWidth={1 / zoom} />
                    <text x={7} y={11} textAnchor="middle" fontSize={9 / zoom} fill="#f74c00">
                      🔒
                    </text>
                  </g>
                )}
                <text
                  x={84}
                  y={20}
                  textAnchor="middle"
                  fill="#00e5ff"
                  fontSize={9 / zoom}
                  fontFamily="monospace"
                >
                  {node.resource_type}
                </text>
                <text
                  x={84}
                  y={36}
                  textAnchor="middle"
                  fill="#94a3b8"
                  fontSize={8 / zoom}
                  fontFamily="monospace"
                >
                  {node.address}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
