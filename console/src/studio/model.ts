/** Presentation contracts. Every model identity is supplied by an immutable backend projection. */
export type Family =
  | "Ownership"
  | "Typing"
  | "Specialization"
  | "Subsetting"
  | "Redefinition"
  | "Connection"
  | "Requirement"
  | "Verification"
  | "Reference";
export const families: Family[] = [
  "Ownership",
  "Typing",
  "Specialization",
  "Subsetting",
  "Redefinition",
  "Connection",
  "Requirement",
  "Verification",
  "Reference",
];
export type Origin = "Authored" | "Derived" | "Standard" | "Generated";
export type Lens =
  | "System"
  | "Graph"
  | "Requirements"
  | "History"
  | "Agents"
  | "Source";
export type ViewDefinition = {
  version: number;
  name: string;
  kind: "Architecture" | "SemanticGraph" | "Requirements";
  focus: string | null;
  depth: number;
  relationship_families: Family[];
  include_standard_library: boolean;
  hidden_elements: string[];
};
export type ViewNode = {
  id: string;
  revision_id: string;
  semantic_kind: string;
  name: string;
  qualified_name: string | null;
  owner: string | null;
  origin: Origin;
  source_available: boolean;
  features: { id: string; name: string; semantic_kind: string }[];
  counts: { parts: number; ports: number; requirements: number };
  badges: string[];
};
export type ViewEdge = {
  id: string;
  relationship_id: string | null;
  revision_id: string;
  family: Family;
  semantic_kind: string;
  source: string;
  target: string;
  origin: Origin;
  rule_id: string | null;
  label: string;
  order: number;
  directed: boolean;
};
export type ViewProjection = {
  revision_id: string;
  view: ViewDefinition;
  nodes: ViewNode[];
  edges: ViewEdge[];
  groups: unknown[];
  metadata: Record<string, unknown>;
};
export type Validation = string | { Validated?: unknown; Working?: unknown };
export const validationLabel = (value: Validation | undefined) =>
  typeof value === "string"
    ? value
    : value && "Validated" in value
      ? "Validated"
      : "Working";
export type Revision = {
  revision_id: string;
  parent_revision_id: string | null;
  validation: Validation;
  metadata?: Record<string, unknown>;
};
export type Session = {
  session_token: string;
  project: { id: string; name: string; default_branch: string };
  branches: { id: string; name: string; head: string }[];
  revisions: Revision[];
  default_revision: string;
  saved_views: ViewDefinition[];
};
export type Source = {
  revision_id: string;
  path: string;
  source: string;
  start: number;
  end: number;
};
export type Candidate = {
  id: string;
  revision_id: string;
  base_revision: string;
  validation: Validation;
  projection: ViewProjection;
  changes: SemanticDiff;
  source_preview: { before: string; after: string; path: string };
};
export type Diff = {
  from: string;
  to: string;
  before: ViewProjection;
  after: ViewProjection;
  diff: SemanticDiff;
};
export type SemanticDiff = {
  declared?: { added: string[]; removed: string[]; changed: string[] };
  derived?: { added: string[]; removed: string[]; changed: string[] };
  relationships_added?: ({ Element: string } | { Occurrence: string })[];
  relationships_removed?: ({ Element: string } | { Occurrence: string })[];
  relationships_changed?: ({ Element: string } | { Occurrence: string })[];
};
export type AgentResult = {
  message: string;
  view?: ViewProjection;
  diff?: Diff;
  decision?: Record<string, unknown>;
};
export type FeatureSummary = {
  id: string;
  name: string;
  semantic_kind: string;
};
export type Inspector = {
  revision_id: string;
  element: ViewNode;
  owner: FeatureSummary | null;
  effective_types: FeatureSummary[];
  owned_features: FeatureSummary[];
  effective_features: FeatureSummary[];
  specializations: FeatureSummary[];
  subsettings: FeatureSummary[];
  redefinitions: FeatureSummary[];
  relationships: ViewEdge[];
  source: {
    document_id: string;
    path: string;
    source_revision_id: string;
    start: number;
    end: number;
  } | null;
  queries: {
    name: string;
    completeness: string;
    diagnostics: string[];
    positive_dependency_count: number;
    search_dependency_count: number;
  }[];
  profile: string;
};
export type Explanation = {
  revision_id: string;
  subject_id: string;
  origin: Origin;
  rule_id: string | null;
  rule_name: string | null;
  profile: string;
  nodes: {
    id: string;
    element_id: string | null;
    label: string;
    kind: "Element" | "Rule" | "Fact";
  }[];
  edges: {
    source: string;
    target: string;
    label: string;
    presentation_only: boolean;
  }[];
  evidence_count: number;
  truncated: boolean;
};
export const short = (id: string | null | undefined) => id?.slice(0, 8) ?? "—";
export function viewDefinition(
  lens: Lens,
  focus: string | null = null,
): ViewDefinition {
  return {
    version: 1,
    name: lens === "System" ? "Agentique architecture" : `${lens} view`,
    kind:
      lens === "System"
        ? "Architecture"
        : lens === "Requirements"
          ? "Requirements"
          : "SemanticGraph",
    focus,
    depth: 2,
    relationship_families:
      lens === "System" ? ["Ownership", "Typing", "Connection"] : [...families],
    include_standard_library: false,
    hidden_elements: [],
  };
}

export type Detail = "far" | "medium" | "near";
/** Semantic detail is presentation policy, independent of CSS and model mutations. */
export function semanticDetail(scale: number): Detail {
  return scale < 0.65 ? "far" : scale < 1.35 ? "medium" : "near";
}
export type PlacedNode = {
  node: ViewNode;
  x: number;
  y: number;
  width: number;
  height: number;
};
/** Stable ownership ranks with bounded cycle handling; semantic order is never rewritten. */
export function layoutProjection(projection: ViewProjection): PlacedNode[] {
  const byId = new Map(projection.nodes.map((node) => [node.id, node]));
  const ranks = new Map<string, number>();
  const rank = (node: ViewNode, trail: Set<string>): number => {
    const cached = ranks.get(node.id);
    if (cached !== undefined) return cached;
    if (trail.has(node.id) || !node.owner || !byId.has(node.owner)) return 0;
    const next = new Set(trail);
    next.add(node.id);
    const result = Math.min(12, rank(byId.get(node.owner)!, next) + 1);
    ranks.set(node.id, result);
    return result;
  };
  const columns = new Map<number, ViewNode[]>();
  for (const node of [...projection.nodes].sort(
    (a, b) =>
      (a.owner ?? "").localeCompare(b.owner ?? "") ||
      a.name.localeCompare(b.name) ||
      a.id.localeCompare(b.id),
  )) {
    const depth = rank(node, new Set());
    columns.set(depth, [...(columns.get(depth) ?? []), node]);
  }
  const tallest = Math.max(
    1,
    ...[...columns.values()].map((nodes) => nodes.length),
  );
  return [...columns.entries()]
    .sort(([a], [b]) => a - b)
    .flatMap(([depth, nodes]) =>
      nodes.map((node, index) => ({
        node,
        x: depth * 310,
        y: index * 172 + (tallest - nodes.length) * 86,
        width: 246,
        height: 138,
      })),
    );
}

let operatorToken = "";
/** The host's local operator capability stays in memory, never in project or persistent browser state. */
export function setOperatorToken(token: string) {
  operatorToken = token;
}
export async function studioApi<T>(
  path: string,
  body?: unknown,
  method?: string,
  signal?: AbortSignal,
): Promise<T> {
  const response = await fetch(`/api/gen2/studio${path}`, {
    method: method ?? (body === undefined ? "GET" : "POST"),
    signal,
    headers: {
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
      ...(operatorToken ? { Authorization: `Bearer ${operatorToken}` } : {}),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let value: unknown;
  try {
    value = text ? JSON.parse(text) : null;
  } catch {
    value = null;
  }
  if (!response.ok) {
    const error = value as {
      message?: string;
      description?: string;
      error?: string;
    } | null;
    throw new Error(
      error?.message ??
        error?.description ??
        error?.error ??
        `Studio service unavailable (${response.status}).`,
    );
  }
  if (value === null)
    throw new Error("The Studio service returned no project data.");
  return value as T;
}
