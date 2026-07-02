'use client';

type Node = { id: string; label: string; type: string; x: number; y: number };
type Edge = { from: string; to: string };

const DEMO_NODES: Node[] = [
  { id: 'vpc', label: 'aws_vpc.main', type: 'aws_vpc', x: 200, y: 80 },
  { id: 'subnet', label: 'aws_subnet.public', type: 'aws_subnet', x: 200, y: 180 },
  { id: 'instance', label: 'aws_instance.web', type: 'aws_instance', x: 200, y: 280 },
  { id: 'sg', label: 'aws_security_group.web', type: 'aws_security_group', x: 400, y: 180 },
];

const DEMO_EDGES: Edge[] = [
  { from: 'vpc', to: 'subnet' },
  { from: 'subnet', to: 'instance' },
  { from: 'sg', to: 'instance' },
];

export function InfrastructureGraph() {
  return (
    <div className="rounded-lg border border-space-700 bg-space-900/80 p-4">
      <h2 className="mb-4 font-display text-lg text-cyan-neon">Infrastructure Graph</h2>
      <svg viewBox="0 0 600 380" className="h-96 w-full">
        <defs>
          <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
            <path d="M0,0 L6,3 L0,6" fill="#00e5ff" opacity="0.6" />
          </marker>
        </defs>

        {DEMO_EDGES.map((e) => {
          const from = DEMO_NODES.find((n) => n.id === e.from)!;
          const to = DEMO_NODES.find((n) => n.id === e.to)!;
          return (
            <line
              key={`${e.from}-${e.to}`}
              x1={from.x}
              y1={from.y + 20}
              x2={to.x}
              y2={to.y - 20}
              stroke="#00e5ff"
              strokeOpacity={0.4}
              strokeWidth={1.5}
              markerEnd="url(#arrow)"
            />
          );
        })}

        {DEMO_NODES.map((node) => (
          <g key={node.id} transform={`translate(${node.x - 80}, ${node.y - 20})`}>
            <rect
              width={160}
              height={40}
              rx={4}
              fill="#111827"
              stroke="#00e5ff"
              strokeOpacity={0.5}
            />
            <text x={80} y={18} textAnchor="middle" fill="#00e5ff" fontSize={9} fontFamily="monospace">
              {node.type}
            </text>
            <text x={80} y={32} textAnchor="middle" fill="#94a3b8" fontSize={8} fontFamily="monospace">
              {node.label}
            </text>
          </g>
        ))}
      </svg>
    </div>
  );
}
