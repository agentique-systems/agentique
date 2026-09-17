type Node = { id: string; name: string | null };
type Edge = Node & { references: { role: string; target: string | null }[] };
export function BehaviorGraph({
  states,
  transitions,
  active,
  initial,
  onSelect,
}: {
  states: Node[];
  transitions: Edge[];
  active?: string;
  initial?: string;
  onSelect: (id: string) => void;
}) {
  const width = Math.max(480, states.length * 145),
    height = Math.max(200, 145 + transitions.length * 24);
  const x = (id: string) => states.findIndex((s) => s.id === id) * 145 + 75;
  return (
    <div className="state-graph">
      <svg
        viewBox={`0 0 ${width} ${height}`}
        role="img"
        aria-label="Resolved states and transitions"
        style={{ minWidth: Math.min(width, 480) }}
      >
        <defs>
          <marker
            id="transition-arrow"
            viewBox="0 0 8 8"
            refX="7"
            refY="4"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 8 4 L 0 8 z" fill="#69907a" />
          </marker>
        </defs>
        {initial && (
          <path
            d={`M ${x(initial)} 10 V 36`}
            stroke="#416f5d"
            markerEnd="url(#transition-arrow)"
          />
        )}
        {transitions.map((t, i) => {
          const from = t.references.find((r) => r.role === "source")?.target,
            to = t.references.find((r) => r.role === "target")?.target;
          if (
            !from ||
            !to ||
            !states.some((s) => s.id === from) ||
            !states.some((s) => s.id === to)
          )
            return null;
          const sx = x(from),
            tx = x(to),
            y = 128 + i * 24;
          return (
            <g key={t.id}>
              <path
                data-transition-id={t.id}
                d={`M ${sx} 98 V ${y} H ${tx} V 100`}
                fill="none"
                stroke="#8bac98"
                strokeWidth="1.5"
                markerEnd="url(#transition-arrow)"
              />
              <text
                x={(sx + tx) / 2}
                y={y - 5}
                textAnchor="middle"
                fontSize="9"
                fill="#567761"
              >
                {t.name}
              </text>
            </g>
          );
        })}
        {states.map((s) => (
          <g
            key={s.id}
            role="button"
            tabIndex={0}
            aria-label={`Inspect state ${s.name}`}
            onClick={() => onSelect(s.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelect(s.id);
              }
            }}
            style={{ cursor: "pointer" }}
          >
            <rect
              x={x(s.id) - 57}
              y="38"
              width="114"
              height="60"
              rx="8"
              fill={active === s.id ? "#e0f1e3" : "#fcfefb"}
              stroke={active === s.id ? "#237b60" : "#b7ccbd"}
              strokeWidth={active === s.id ? 2 : 1}
            />
            <circle
              cx={x(s.id) - 41}
              cy="53"
              r="3"
              fill={active === s.id ? "#278369" : "#94ad96"}
            />
            <text
              x={x(s.id) - 41}
              y="70"
              fill="#224a3d"
              fontSize="12"
              fontWeight="550"
            >
              {s.name}
            </text>
            <text x={x(s.id) - 41} y="86" fill="#7d917c" fontSize="8">
              {active === s.id ? "Active in run" : "Authored state"}
            </text>
          </g>
        ))}
      </svg>
    </div>
  );
}
