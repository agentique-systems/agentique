import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  Family,
  SemanticDiff,
  ViewEdge,
  ViewNode,
  ViewProjection,
  layoutProjection,
  semanticDetail,
} from "./model";

export type Change = "added" | "removed" | "changed";
type Props = {
  projection: ViewProjection;
  selected?: string;
  selectedEdge?: string;
  filters: Family[];
  onSelect: (id: string) => void;
  onEdge: (edge: ViewEdge) => void;
  onFocus?: (id: string) => void;
  changes?: Map<string, Change>;
  edgeChanges?: Map<string, Change>;
};

export function Viewport({
  projection,
  selected,
  selectedEdge,
  filters,
  onSelect,
  onEdge,
  onFocus,
  changes,
  edgeChanges,
}: Props) {
  const svg = useRef<SVGSVGElement>(null);
  const drag = useRef<{
    x: number;
    y: number;
    tx: number;
    ty: number;
    moved: boolean;
  } | null>(null);
  const [camera, setCamera] = useState({ x: 80, y: 90, scale: 0.85 });
  const [panning, setPanning] = useState(false);
  const placed = useMemo(() => layoutProjection(projection), [projection]);
  const positions = useMemo(
    () => new Map(placed.map((item) => [item.node.id, item])),
    [placed],
  );
  const detail = semanticDetail(camera.scale);
  const fit = () => {
    const bounds = svg.current?.getBoundingClientRect();
    if (!bounds || !placed.length) return;
    const width = Math.max(...placed.map((item) => item.x + item.width));
    const height = Math.max(...placed.map((item) => item.y + item.height));
    const scale = Math.min(
      1.25,
      Math.max(
        0.15,
        Math.min((bounds.width - 100) / width, (bounds.height - 120) / height),
      ),
    );
    setCamera({
      x: (bounds.width - width * scale) / 2,
      y: (bounds.height - height * scale) / 2,
      scale,
    });
  };
  useEffect(() => {
    fit();
  }, [projection.revision_id, projection.view.name, projection.view.focus]);
  const zoom = (factor: number, clientX?: number, clientY?: number) => {
    const bounds = svg.current?.getBoundingClientRect();
    if (!bounds) return;
    const x = clientX === undefined ? bounds.width / 2 : clientX - bounds.left;
    const y = clientY === undefined ? bounds.height / 2 : clientY - bounds.top;
    setCamera((previous) => {
      const scale = Math.min(2.4, Math.max(0.15, previous.scale * factor));
      const ratio = scale / previous.scale;
      return {
        scale,
        x: x - (x - previous.x) * ratio,
        y: y - (y - previous.y) * ratio,
      };
    });
  };
  return (
    <div className="studio-viewport" data-testid="semantic-viewport">
      <svg
        ref={svg}
        className={panning ? "is-panning" : ""}
        aria-label={`${projection.view.name}, ${projection.nodes.length} semantic elements`}
        onWheel={(event) => {
          zoom(event.deltaY < 0 ? 1.1 : 1 / 1.1, event.clientX, event.clientY);
        }}
        onPointerDown={(event) => {
          if ((event.target as Element).closest("[data-semantic-object]"))
            return;
          drag.current = {
            x: event.clientX,
            y: event.clientY,
            tx: camera.x,
            ty: camera.y,
            moved: false,
          };
          event.currentTarget.setPointerCapture(event.pointerId);
          setPanning(true);
        }}
        onPointerMove={(event) => {
          if (!drag.current) return;
          const dx = event.clientX - drag.current.x,
            dy = event.clientY - drag.current.y;
          drag.current.moved = Math.abs(dx) + Math.abs(dy) > 3;
          setCamera((previous) => ({
            ...previous,
            x: drag.current!.tx + dx,
            y: drag.current!.ty + dy,
          }));
        }}
        onPointerUp={() => {
          drag.current = null;
          setPanning(false);
        }}
        onPointerCancel={() => {
          drag.current = null;
          setPanning(false);
        }}
      >
        <defs>
          <pattern
            id="studio-grid"
            width="24"
            height="24"
            patternUnits="userSpaceOnUse"
          >
            <circle cx="1" cy="1" r="0.8" className="studio-grid-dot" />
          </pattern>
          <marker
            id="studio-arrow"
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth="6"
            markerHeight="6"
            orient="auto-start-reverse"
          >
            <path d="M 0 0 L 10 5 L 0 10 z" fill="currentColor" />
          </marker>
        </defs>
        <rect width="100%" height="100%" fill="url(#studio-grid)" />
        <g
          transform={`translate(${camera.x} ${camera.y}) scale(${camera.scale})`}
        >
          {projection.edges
            .filter((edge) => filters.includes(edge.family))
            .map((edge) => {
              const source = positions.get(edge.source),
                target = positions.get(edge.target);
              if (!source || !target) return null;
              const sameColumn = Math.abs(source.x - target.x) < 1;
              const reverse = source.x > target.x;
              const sx = source.x + (reverse ? 0 : source.width),
                sy = source.y + source.height / 2;
              const tx = target.x + (reverse ? target.width : 0),
                ty = target.y + target.height / 2;
              const bend = sameColumn
                ? 72 + (edge.order % 4) * 12
                : Math.max(35, Math.abs(tx - sx) * 0.5);
              const path = `M ${sx} ${sy} C ${sx + (reverse ? -bend : bend)} ${sy}, ${tx + (sameColumn ? bend : reverse ? bend : -bend)} ${ty}, ${tx} ${ty}`;
              return (
                <g
                  key={edge.id}
                  data-semantic-object="edge"
                  data-testid="semantic-edge"
                  className={`studio-edge ${edge.family.toLowerCase()} ${edge.origin === "Derived" ? "derived" : ""} ${selectedEdge === edge.id ? "selected" : ""} ${edgeChanges?.get(edge.id) ?? ""}`}
                  role="button"
                  tabIndex={0}
                  aria-label={`${edge.label}: ${source.node.name} to ${target.node.name}`}
                  onClick={() => onEdge(edge)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") onEdge(edge);
                  }}
                >
                  <path d={path} className="edge-hit" />
                  <path
                    d={path}
                    className="edge-line"
                    markerEnd={edge.directed ? "url(#studio-arrow)" : undefined}
                  />
                  {detail !== "far" && (
                    <text
                      x={sameColumn ? sx + bend : (sx + tx) / 2}
                      y={(sy + ty) / 2 - 8}
                      textAnchor="middle"
                    >
                      {edge.label}
                    </text>
                  )}
                </g>
              );
            })}
          {placed.map(({ node, x, y, width, height }) => (
            <g
              key={node.id}
              transform={`translate(${x} ${y})`}
              data-semantic-object="node"
              data-element-id={node.id}
              data-testid="semantic-node"
              className={`studio-node ${selected === node.id ? "selected" : ""} ${changes?.get(node.id) ?? ""}`}
              role="button"
              tabIndex={0}
              aria-label={`${node.name}, ${node.semantic_kind}`}
              onClick={() => onSelect(node.id)}
              onDoubleClick={() => onFocus?.(node.id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(node.id);
                }
              }}
            >
              <rect
                width={width}
                height={detail === "far" ? 66 : height}
                rx="10"
                className="node-body"
              />
              <rect
                width="3"
                x="0"
                y="13"
                height={detail === "far" ? 40 : height - 26}
                rx="1.5"
                className="node-accent"
              />
              {detail === "far" ? (
                <text x="20" y="39" className="node-name">
                  {truncate(node.name, 25)}
                </text>
              ) : (
                <>
                  <text x="18" y="24" className="node-kind">
                    {node.semantic_kind
                      .replace(/([a-z])([A-Z])/g, "$1 $2")
                      .toUpperCase()}
                  </text>
                  <text x="18" y="49" className="node-name">
                    {truncate(node.name, 25)}
                  </text>
                  <line
                    x1="18"
                    x2={width - 18}
                    y1="64"
                    y2="64"
                    className="node-divider"
                  />
                  {detail === "medium" ? (
                    <>
                      <text x="18" y="88" className="node-detail">
                        {node.counts.parts} parts{" "}
                        <tspan dx="9">{node.counts.ports} ports</tspan>
                      </text>
                      <text x="18" y="112" className="node-detail">
                        {node.counts.requirements > 0
                          ? `${node.counts.requirements} requirements`
                          : node.origin === "Derived"
                            ? "Derived · evidence available"
                            : "Authored semantic element"}
                      </text>
                    </>
                  ) : (
                    <>
                      {node.features.slice(0, 3).map((feature, index) => (
                        <text
                          key={feature.id}
                          x="18"
                          y={84 + index * 19}
                          className="node-detail"
                        >
                          {feature.semantic_kind.includes("Port") ? "◉ " : "↳ "}
                          {truncate(feature.name, 28)}
                        </text>
                      ))}
                      {!node.features.length && (
                        <text x="18" y="89" className="node-detail">
                          {node.origin} ·{" "}
                          {node.source_available
                            ? "source linked"
                            : "semantic record"}
                        </text>
                      )}
                    </>
                  )}
                  {node.counts.ports > 0 && (
                    <>
                      <circle cx="0" cy="94" r="5" className="node-port" />
                      <circle cx={width} cy="94" r="5" className="node-port" />
                    </>
                  )}
                </>
              )}
              {changes?.has(node.id) && (
                <text
                  x={width - 14}
                  y="23"
                  textAnchor="end"
                  className="node-change"
                >
                  {changes.get(node.id) === "added"
                    ? "+"
                    : changes.get(node.id) === "removed"
                      ? "−"
                      : "Δ"}
                </text>
              )}
              <title>
                {node.qualified_name ?? node.name} · {node.semantic_kind} ·{" "}
                {node.revision_id}
              </title>
            </g>
          ))}
        </g>
      </svg>
      {!placed.length && (
        <div className="studio-empty-canvas">
          <span className="studio-orbit">◈</span>
          <h2>No elements in this view</h2>
          <p>
            Change the focus or relationship filters to explore this revision.
          </p>
        </div>
      )}
      <div className="studio-camera-tools">
        <span>{detail} detail</span>
        <button aria-label="Zoom out" onClick={() => zoom(1 / 1.2)}>
          −
        </button>
        <span>{Math.round(camera.scale * 100)}%</span>
        <button aria-label="Zoom in" onClick={() => zoom(1.2)}>
          +
        </button>
        <button onClick={fit}>Fit view</button>
      </div>
      <div className="studio-canvas-hint">
        Scroll to zoom · drag to pan · double-click to focus
      </div>
    </div>
  );
}

function truncate(value: string, limit: number) {
  return value.length > limit ? value.slice(0, limit - 1) + "…" : value;
}
export function projectionChanges(
  before: ViewProjection,
  after: ViewProjection,
  semantic?: SemanticDiff,
) {
  const nodeChanges = new Map<string, Change>(),
    edgeChanges = new Map<string, Change>();
  const compare = <T extends { id: string; revision_id: string }>(
    left: T[],
    right: T[],
    output: Map<string, Change>,
  ) => {
    const prior = new Map(left.map((value) => [value.id, value]));
    const next = new Map(right.map((value) => [value.id, value]));
    for (const item of left)
      if (!next.has(item.id)) output.set(item.id, "removed");
    for (const item of right) {
      const old = prior.get(item.id);
      if (!old) output.set(item.id, "added");
      else {
        const { revision_id: _oldRevision, ...oldValue } = old;
        const { revision_id: _newRevision, ...newValue } = item;
        if (JSON.stringify(oldValue) !== JSON.stringify(newValue))
          output.set(item.id, "changed");
      }
    }
  };
  compare<ViewNode>(before.nodes, after.nodes, nodeChanges);
  compare<ViewEdge>(before.edges, after.edges, edgeChanges);
  const canonical = canonicalChanges(
    {
      ...after,
      nodes: [...before.nodes, ...after.nodes],
      edges: [...before.edges, ...after.edges],
    },
    semantic,
  );
  for (const [id, change] of canonical.nodes) nodeChanges.set(id, change);
  for (const [id, change] of canonical.edges) edgeChanges.set(id, change);
  const nodes = [
    ...after.nodes,
    ...before.nodes.filter((node) => nodeChanges.get(node.id) === "removed"),
  ];
  const edges = [
    ...after.edges,
    ...before.edges.filter((edge) => edgeChanges.get(edge.id) === "removed"),
  ];
  return {
    nodes: nodeChanges,
    edges: edgeChanges,
    projection: { ...after, nodes, edges },
  };
}

/** Changed properties can be absent from the display DTO; canonical diff remains authoritative. */
export function canonicalChanges(
  projection: ViewProjection,
  semantic?: SemanticDiff,
) {
  const nodes = new Map<string, Change>(),
    edges = new Map<string, Change>();
  if (!semantic) return { nodes, edges };
  const visible = new Set(projection.nodes.map((node) => node.id));
  for (const change of ["changed", "added", "removed"] as Change[]) {
    for (const category of [semantic.declared, semantic.derived]) {
      for (const id of category?.[change] ?? [])
        if (visible.has(id)) nodes.set(id, change);
    }
    for (const identity of semantic[`relationships_${change}`] ?? []) {
      const id = "Element" in identity ? identity.Element : identity.Occurrence;
      for (const edge of projection.edges) {
        if (
          edge.relationship_id === id ||
          edge.id === id ||
          edge.id === `occurrence:${id}`
        )
          edges.set(edge.id, change);
      }
    }
  }
  return { nodes, edges };
}
