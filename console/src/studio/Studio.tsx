import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  AgentResult,
  Candidate,
  Diff,
  Explanation,
  Family,
  FeatureSummary,
  Inspector,
  Lens,
  Session,
  Source,
  ViewEdge,
  ViewProjection,
  families,
  setOperatorToken,
  short,
  studioApi,
  validationLabel,
  viewDefinition,
} from "./model";
import { Viewport, canonicalChanges, projectionChanges } from "./Viewport";
import { RuntimeSetup, RuntimeStatus } from "./RuntimeSetup";
import "./studio.css";

const lenses: { name: Lens; icon: string; caption: string }[] = [
  { name: "System", icon: "◈", caption: "Architecture" },
  { name: "Graph", icon: "⌘", caption: "Relationships" },
  { name: "Requirements", icon: "◎", caption: "Engineering knowledge" },
  { name: "History", icon: "◷", caption: "Immutable revisions" },
  { name: "Agents", icon: "✧", caption: "Model participants" },
  { name: "Source", icon: "⌁", caption: "Expert lens" },
];

export function Studio() {
  const [session, setSession] = useState<Session>();
  const [revision, setRevision] = useState("");
  const [branch, setBranch] = useState("");
  const [lens, setLens] = useState<Lens>("System");
  const [definition, setDefinition] = useState(viewDefinition("System"));
  const [projectionResult, setProjection] = useState<ViewProjection>();
  const [selection, setSelection] = useState<string>();
  const [edge, setEdge] = useState<ViewEdge>();
  const [inspectorResult, setInspector] = useState<Inspector>();
  const [source, setSource] = useState<Source>();
  const [explanation, setExplanation] = useState<Explanation>();
  const [diffResult, setDiff] = useState<Diff>();
  const [candidate, setCandidate] = useState<Candidate>();
  const [filters, setFilters] = useState<Family[]>([...families]);
  const [search, setSearch] = useState("");
  const [intent, setIntent] = useState("");
  const [activity, setActivity] = useState<
    { revision: string; result: AgentResult }[]
  >([]);
  const [partName, setPartName] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [busy, setBusy] = useState("");
  const [loadingView, setLoadingView] = useState(false);
  const [error, setError] = useState("");
  const [startupError, setStartupError] = useState("");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus>();
  const [theme, setTheme] = useState(
    () => localStorage.getItem("agentique-studio-theme") ?? "dark",
  );
  const currentRevision = useRef(revision);
  currentRevision.current = revision;
  const currentSubject = useRef(edge?.relationship_id ?? selection);
  currentSubject.current = edge?.relationship_id ?? selection;
  const current = session?.revisions.find(
    (item) => item.revision_id === revision,
  );
  const projection =
    projectionResult?.revision_id === revision ? projectionResult : undefined;
  const diff = diffResult?.to === revision ? diffResult : undefined;
  const parent = current?.parent_revision_id;
  const revisionStatus =
    projection?.revision_id === revision
      ? validationLabel(current?.validation)
      : loadingView
        ? "Opening revision"
        : error
          ? "Revision unavailable"
          : "Opening revision";
  const diffView = useMemo(
    () =>
      diff ? projectionChanges(diff.before, diff.after, diff.diff) : undefined,
    [diff],
  );
  const selectedNode = (diffView?.projection ?? projection)?.nodes.find(
    (node) => node.id === selection,
  );
  const inspectionRevision =
    edge?.revision_id ??
    (selection && diffView?.nodes.get(selection) === "removed"
      ? diff!.from
      : revision);
  const inspector =
    inspectorResult?.revision_id === inspectionRevision &&
    inspectorResult.element.id === selection
      ? inspectorResult
      : undefined;
  const candidateChanges = useMemo(
    () =>
      candidate
        ? canonicalChanges(candidate.projection, candidate.changes)
        : undefined,
    [candidate],
  );

  async function refreshSession(nextRevision?: string) {
    setStartupError("");
    try {
      const next = await studioApi<Session>("/session");
      setOperatorToken(next.session_token);
      setSession(next);
      setBranch((previous) => previous || next.project.default_branch);
      setRevision(
        (previous) => nextRevision ?? (previous || next.default_revision),
      );
    } catch (caught) {
      setStartupError(message(caught));
    }
  }
  useEffect(() => {
    void refreshSession();
  }, []);
  useEffect(() => {
    if (session || !startupError) return;
    let cancelled = false;
    async function poll() {
      try {
        const status = await studioApi<RuntimeStatus>("/runtime");
        if (cancelled) return;
        if (status.session_token) setOperatorToken(status.session_token);
        setRuntimeStatus(status);
        if (status.ready) await refreshSession();
      } catch {
        /* Keep the connection error visible if the host cannot be reached. */
      }
    }
    void poll();
    const timer = setInterval(() => void poll(), 1500);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [session, startupError]);
  useEffect(() => {
    localStorage.setItem("agentique-studio-theme", theme);
  }, [theme]);
  useEffect(() => {
    if (!session || !revision) return;
    const abort = new AbortController();
    setLoadingView(true);
    setProjection(undefined);
    setInspector(undefined);
    setSource(undefined);
    setEdge(undefined);
    setDiff(undefined);
    setExplanation(undefined);
    void studioApi<ViewProjection>(
      "/view",
      { project: session.project.id, revision, definition },
      undefined,
      abort.signal,
    )
      .then((result) => {
        assertRevision(result.revision_id, revision);
        setProjection(result);
      })
      .catch((caught) => {
        if (!abort.signal.aborted) setError(message(caught));
      })
      .finally(() => {
        if (!abort.signal.aborted) setLoadingView(false);
      });
    return () => abort.abort();
  }, [session?.project.id, revision, definition]);
  useEffect(() => {
    setInspector(undefined);
    setSource(undefined);
    if (!session || !inspectionRevision || !selection) return;
    const abort = new AbortController();
    void studioApi<Inspector>(
      "/inspect",
      {
        project: session.project.id,
        revision: inspectionRevision,
        element: selection,
      },
      undefined,
      abort.signal,
    )
      .then((result) => {
        assertRevision(result.revision_id, inspectionRevision);
        setInspector(result);
      })
      .catch((caught) => {
        if (!abort.signal.aborted) setError(message(caught));
      });
    return () => abort.abort();
  }, [session?.project.id, inspectionRevision, selection]);

  async function task(label: string, work: () => Promise<void>) {
    setBusy(label);
    setError("");
    try {
      await work();
    } catch (caught) {
      setError(message(caught));
    } finally {
      setBusy("");
    }
  }
  function select(id: string) {
    setSelection(id);
    setEdge(undefined);
  }
  function changeLens(next: Lens) {
    setLens(next);
    setDiff(undefined);
    setExplanation(undefined);
    if (["System", "Graph", "Requirements"].includes(next))
      setDefinition(viewDefinition(next));
    if (next === "Source" && selection) void openSource();
  }
  function changeRevision(next: string) {
    setRevision(next);
    setSelection(undefined);
    setEdge(undefined);
    setDiff(undefined);
    setExplanation(undefined);
    setError("");
  }
  function focus(id: string) {
    setDefinition((previous) => ({
      ...(projection?.view ?? previous),
      focus: id,
      depth: Math.min((projection?.view ?? previous).depth + 1, 5),
    }));
  }
  async function explain(element = edge?.relationship_id ?? selection) {
    if (!session || !element) return;
    await task("Reading semantic evidence", async () => {
      const result = await studioApi<Explanation>("/explain", {
        project: session.project.id,
        revision: inspectionRevision,
        element,
      });
      if (
        currentRevision.current !== revision ||
        currentSubject.current !== element
      )
        return;
      assertRevision(result.revision_id, inspectionRevision);
      setExplanation(result);
    });
  }
  async function openSource() {
    if (!session || !selection) return;
    await task("Opening source", async () => {
      const result = await studioApi<Source>("/source", {
        project: session.project.id,
        revision: inspectionRevision,
        element: selection,
      });
      if (
        currentRevision.current !== revision ||
        currentSubject.current !== selection
      )
        return;
      assertRevision(result.revision_id, inspectionRevision);
      setSource(result);
      setLens("Source");
    });
  }
  async function compare(from = parent) {
    if (!session || !from) return;
    await task("Comparing immutable revisions", async () => {
      const result = await studioApi<Diff>("/diff", {
        project: session.project.id,
        from,
        to: revision,
      });
      if (currentRevision.current !== revision) return;
      assertRevision(result.after.revision_id, revision);
      setDiff(result);
      setLens("Graph");
      setEdge(undefined);
    });
  }
  async function askAgent(text: string) {
    if (!session) return;
    await task("Agent is preparing a semantic view", async () => {
      const result = await studioApi<AgentResult>("/agent", {
        project: session.project.id,
        revision,
        selection: selection ? [selection] : [],
        intent: text,
        compare_to: parent ?? undefined,
      });
      if (currentRevision.current !== revision) return;
      if (result.view) {
        assertRevision(result.view.revision_id, revision);
        setProjection(result.view);
        setLens(
          result.view.view.kind === "Architecture"
            ? "System"
            : result.view.view.kind === "Requirements"
              ? "Requirements"
              : "Graph",
        );
        setDiff(undefined);
      }
      if (result.diff) {
        assertRevision(result.diff.after.revision_id, revision);
        setDiff(result.diff);
        setLens("Graph");
      }
      setActivity((previous) => [{ revision, result }, ...previous]);
      setIntent("");
    });
  }
  async function createPart() {
    if (!session || !selection || !partName.trim()) return;
    await task("Preparing a source-backed Working revision", async () => {
      const result = await studioApi<Candidate>("/candidates", {
        project: session.project.id,
        branch,
        revision,
        command: {
          kind: "CreatePartUsage",
          owner: selection,
          name: partName.trim(),
        },
      });
      setCandidate(result);
      setShowCreate(false);
      setPartName("");
    });
  }
  async function saveView() {
    if (!session || !projection) return;
    await task("Saving view definition", async () => {
      await studioApi("/views", {
        project: session.project.id,
        definition: { ...projection.view, relationship_families: filters },
      });
      await refreshSession();
    });
  }

  const outlinerNodes = (projection?.nodes ?? []).filter((node) =>
    node.name.toLowerCase().includes(search.toLowerCase()),
  );
  const categories = [
    {
      name: "System & definitions",
      match: (kind: string) =>
        kind.includes("Definition") ||
        kind.includes("Namespace") ||
        kind === "Package",
    },
    {
      name: "Parts",
      match: (kind: string) =>
        kind.includes("Part") && !kind.includes("Definition"),
    },
    {
      name: "Ports & interfaces",
      match: (kind: string) =>
        kind.includes("Port") || kind.includes("Interface"),
    },
    {
      name: "Connections",
      match: (kind: string) =>
        kind.includes("Connection") || kind.includes("Connector"),
    },
    {
      name: "Requirements",
      match: (kind: string) => kind.includes("Requirement"),
    },
    { name: "Behavior & other", match: (_kind: string) => true },
  ];
  const classified = new Set<string>();
  return (
    <div className={`studio studio-${theme}`}>
      <header className="studio-topbar">
        <a className="studio-brand" href="/studio">
          <span>◈</span> AGENTIQUE <small>PROTOTYPE 0</small>
        </a>
        <span className="studio-top-divider" />
        <div className="studio-project-name">
          {session?.project.name ?? "Engineering workspace"}
        </div>
        {session && (
          <>
            <span className="studio-top-slash">/</span>
            <select
              aria-label="Branch"
              value={branch}
              disabled={!!busy}
              onChange={(event) => {
                const next = session.branches.find(
                  (item) => item.id === event.target.value,
                )!;
                setBranch(next.id);
                changeRevision(next.head);
              }}
            >
              {session.branches.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
            <select
              aria-label="Revision"
              value={revision}
              disabled={!!busy}
              onChange={(event) => changeRevision(event.target.value)}
            >
              {session.revisions.map((item) => (
                <option key={item.revision_id} value={item.revision_id}>
                  {short(item.revision_id)} · {validationLabel(item.validation)}
                </option>
              ))}
            </select>
          </>
        )}
        <div className="studio-top-spacer" />
        <span
          className={`studio-validation ${session ? revisionStatus.toLowerCase() : "unavailable"}`}
        >
          {session
            ? `${revisionStatus === "Validated" ? "✓" : "○"} ${revisionStatus}`
            : "○ Awaiting repository"}
        </span>
        <button
          className="studio-icon-button"
          aria-label="Toggle color theme"
          onClick={() =>
            setTheme((previous) => (previous === "dark" ? "light" : "dark"))
          }
        >
          {theme === "dark" ? "☼" : "☾"}
        </button>
        <button
          className="studio-agent-button"
          onClick={() => changeLens("Agents")}
        >
          ✧ Agents {activity.length > 0 && <b>{activity.length}</b>}
        </button>
      </header>
      {!session ? (
        <RuntimeSetup
          status={runtimeStatus}
          error={startupError}
          onRetry={async () => {
            if (runtimeStatus) await studioApi("/runtime/retry", {});
            else await refreshSession();
          }}
          onInstall={async (bundle) => {
            await studioApi("/runtime/install", { bundle });
            setRuntimeStatus((previous) =>
              previous
                ? {
                    ...previous,
                    running: true,
                    phase: "locating_package",
                    error: null,
                  }
                : previous,
            );
          }}
        />
      ) : (
        <>
          <div className="studio-workspace">
            <aside className="studio-outliner">
              <div className="studio-panel-title">
                <span>MODEL EXPLORER</span>
                <span className="studio-muted">
                  {projection?.nodes.length ?? "—"}
                </span>
              </div>
              <label className="studio-search">
                <span>⌕</span>
                <input
                  aria-label="Find an element"
                  placeholder="Find an element…"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                />
              </label>
              <div className="studio-tree-root">
                <span>◈</span>
                <b>{session.project.name}</b>
                <small>MODEL</small>
              </div>
              <div className="studio-tree-scroll">
                {categories.map((category) => {
                  const nodes = outlinerNodes.filter(
                    (node) =>
                      !classified.has(node.id) &&
                      category.match(node.semantic_kind),
                  );
                  nodes.forEach((node) => classified.add(node.id));
                  return nodes.length ? (
                    <details
                      className="studio-tree-group"
                      open
                      key={category.name}
                    >
                      <summary>
                        {category.name}
                        <span>{nodes.length}</span>
                      </summary>
                      {nodes.map((node) => (
                        <button
                          className={selection === node.id ? "active" : ""}
                          key={node.id}
                          onClick={() => select(node.id)}
                          title={node.qualified_name ?? node.name}
                        >
                          <span className="studio-tree-glyph">
                            {node.semantic_kind.includes("Part")
                              ? "◇"
                              : node.semantic_kind.includes("Port")
                                ? "◉"
                                : "◈"}
                          </span>
                          <span>{node.name || node.semantic_kind}</span>
                          {node.origin === "Derived" && <small>ƒ</small>}
                        </button>
                      ))}
                    </details>
                  ) : null;
                })}
              </div>
              <div className="studio-saved-views">
                <div className="studio-panel-title">
                  SAVED VIEWS
                  <button
                    disabled={!projection || !!busy}
                    onClick={() => void saveView()}
                    title="Save this view definition"
                  >
                    +
                  </button>
                </div>
                {session.saved_views.map((view, index) => (
                  <button
                    key={`${view.name}-${index}`}
                    onClick={() => {
                      setDefinition(view);
                      setFilters(view.relationship_families);
                      setLens(
                        view.kind === "Architecture"
                          ? "System"
                          : view.kind === "Requirements"
                            ? "Requirements"
                            : "Graph",
                      );
                    }}
                  >
                    ▧ {view.name}
                  </button>
                ))}
              </div>
              <a className="studio-expert-link" href="/expert">
                Legacy expert console ↗
              </a>
            </aside>
            <main className="studio-world">
              <div className="studio-world-toolbar">
                <div>
                  <p className="studio-eyebrow">
                    {diff
                      ? "REVISION COMPARISON"
                      : lenses
                          .find((item) => item.name === lens)
                          ?.caption.toUpperCase()}
                  </p>
                  <h1>
                    {diff
                      ? `${short(diff.from)} → ${short(diff.to)}`
                      : lens === "System"
                        ? "System World"
                        : lens === "Graph"
                          ? "Graph World"
                          : lens === "Requirements"
                            ? "Requirements World"
                            : lens === "History"
                              ? "History"
                              : lens === "Agents"
                                ? "Agent activity"
                                : "Source"}
                  </h1>
                </div>
                <div className="studio-world-actions">
                  {projection?.view.focus && (
                    <button
                      onClick={() =>
                        setDefinition((previous) => ({
                          ...(projection?.view ?? previous),
                          focus: null,
                        }))
                      }
                    >
                      All elements
                    </button>
                  )}
                  {selection &&
                    ["System", "Graph", "Requirements"].includes(lens) && (
                      <button onClick={() => focus(selection)}>
                        {lens === "Graph"
                          ? "Expand neighbors"
                          : "Focus selection"}
                      </button>
                    )}
                  {parent && !diff && (
                    <button disabled={!!busy} onClick={() => void compare()}>
                      Compare parent
                    </button>
                  )}
                  {diff && (
                    <button onClick={() => setDiff(undefined)}>
                      Close comparison
                    </button>
                  )}
                </div>
              </div>
              {error && (
                <div role="alert" className="studio-error">
                  <span>{error}</span>
                  <button
                    aria-label="Dismiss error"
                    onClick={() => setError("")}
                  >
                    ×
                  </button>
                </div>
              )}
              {busy && (
                <div role="status" className="studio-progress">
                  <span />
                  {busy}…
                </div>
              )}
              {["System", "Graph", "Requirements"].includes(lens) && (
                <>
                  <div className="studio-filterbar">
                    {(lens === "Graph"
                      ? families
                      : (["Ownership", "Typing", "Connection"] as Family[])
                    ).map((family) => (
                      <label key={family}>
                        <input
                          type="checkbox"
                          checked={filters.includes(family)}
                          onChange={() =>
                            setFilters((previous) =>
                              previous.includes(family)
                                ? previous.filter((item) => item !== family)
                                : [...previous, family],
                            )
                          }
                        />
                        <span
                          className={`family-dot ${family.toLowerCase()}`}
                        />
                        {family}
                      </label>
                    ))}
                    {lens === "Graph" && (
                      <label className="studio-standards-toggle">
                        <input
                          type="checkbox"
                          checked={definition.include_standard_library}
                          onChange={(event) =>
                            setDefinition((previous) => ({
                              ...previous,
                              include_standard_library: event.target.checked,
                            }))
                          }
                        />
                        Standards
                      </label>
                    )}
                  </div>
                  {diff && (
                    <div className="studio-diff-legend">
                      <span className="added">+ Added</span>
                      <span className="removed">− Removed</span>
                      <span className="changed">Δ Changed</span>
                      <small>
                        Comparison spans two explicitly named revisions
                      </small>
                    </div>
                  )}
                  {loadingView ? (
                    <div className="studio-loading">
                      <span className="studio-orbit">◈</span>
                      <p>Projecting the semantic model…</p>
                      <small>
                        Pan, zoom, and selection use the loaded immutable
                        revision.
                      </small>
                    </div>
                  ) : (
                    (diffView?.projection ?? projection) && (
                      <Viewport
                        projection={(diffView?.projection ?? projection)!}
                        selected={selection}
                        selectedEdge={edge?.id}
                        filters={filters}
                        onSelect={select}
                        onEdge={(value) => {
                          setEdge(value);
                          setSelection(undefined);
                        }}
                        onFocus={focus}
                        changes={diffView?.nodes}
                        edgeChanges={diffView?.edges}
                      />
                    )
                  )}
                </>
              )}
              {lens === "History" && (
                <History
                  session={session}
                  revision={revision}
                  disabled={!!busy}
                  onSelect={changeRevision}
                  onCompare={(from) => void compare(from)}
                />
              )}
              {lens === "Agents" && (
                <div className="studio-agent-world">
                  <div className="studio-agent-intro">
                    <span>✧</span>
                    <h2>Agents act on the model.</h2>
                    <p>
                      Give an intent. Receive a focused semantic view or a
                      revision comparison.
                    </p>
                    <small>
                      Deterministic demonstration · Read + Propose authority
                    </small>
                  </div>
                  <div className="studio-intent-cards">
                    <button
                      disabled={!!busy}
                      onClick={() => void askAgent("Show me its dependencies")}
                    >
                      <span>⌘</span>
                      <b>Explore dependencies</b>
                      <small>
                        {selectedNode
                          ? `Focus on ${selectedNode.name}`
                          : "Explore the current architecture"}{" "}
                        →
                      </small>
                    </button>
                    <button
                      disabled={!!busy || !parent}
                      onClick={() =>
                        void askAgent("Show what changed between revisions")
                      }
                    >
                      <span>◷</span>
                      <b>Explain what changed</b>
                      <small>Compare this revision with its parent →</small>
                    </button>
                    <button
                      disabled={!!busy}
                      onClick={() =>
                        void askAgent("Show the system architecture")
                      }
                    >
                      <span>◈</span>
                      <b>See the architecture</b>
                      <small>Return a System World view →</small>
                    </button>
                  </div>
                  <form
                    className="studio-intent-form"
                    onSubmit={(event) => {
                      event.preventDefault();
                      void askAgent(intent);
                    }}
                  >
                    <input
                      aria-label="Operator intent"
                      value={intent}
                      onChange={(event) => setIntent(event.target.value)}
                      placeholder="Show the requirements for this system…"
                    />
                    <button
                      className="studio-primary"
                      disabled={!!busy || !intent.trim()}
                    >
                      Create view ↗
                    </button>
                  </form>
                  <div className="studio-activity-list">
                    {activity.map((item, index) => (
                      <article key={index}>
                        <span>✧</span>
                        <div>
                          <b>View agent</b>
                          <p>{item.result.message}</p>
                          <small>
                            Revision {short(item.revision)} · Read + Propose
                          </small>
                        </div>
                        <button
                          disabled={item.revision !== revision}
                          onClick={() => {
                            if (item.result.view) {
                              setProjection(item.result.view);
                              setLens("Graph");
                            }
                            if (item.result.diff) {
                              setDiff(item.result.diff);
                              setLens("Graph");
                            }
                          }}
                        >
                          Show
                        </button>
                      </article>
                    ))}
                  </div>
                </div>
              )}
              {lens === "Source" && (
                <div className="studio-source-world">
                  {source ? (
                    <>
                      <div className="studio-source-heading">
                        <b>{source.path}</b>
                        <span>
                          Revision {short(source.revision_id)} · read only
                        </span>
                      </div>
                      <pre>{source.source}</pre>
                    </>
                  ) : (
                    <div className="studio-loading">
                      <span>⌁</span>
                      <h2>Source is one lens.</h2>
                      <p>
                        Select an element, then open its source from the
                        inspector.
                      </p>
                    </div>
                  )}
                </div>
              )}
            </main>
            <aside className="studio-inspector">
              <div className="studio-panel-title">
                INSPECTOR
                <span className="studio-muted">
                  {short(inspectionRevision)}
                </span>
              </div>
              {edge ? (
                <>
                  <div className="studio-inspector-heading">
                    <span className="studio-kind-pill">RELATIONSHIP</span>
                    <h2>{edge.semantic_kind}</h2>
                    <p>{edge.label}</p>
                  </div>
                  <dl>
                    <dt>Family</dt>
                    <dd>{edge.family}</dd>
                    <dt>Origin</dt>
                    <dd>{edge.origin}</dd>
                    <dt>Source</dt>
                    <dd>
                      <button onClick={() => select(edge.source)}>
                        {projection?.nodes.find(
                          (node) => node.id === edge.source,
                        )?.name ?? short(edge.source)}
                      </button>
                    </dd>
                    <dt>Target</dt>
                    <dd>
                      <button onClick={() => select(edge.target)}>
                        {projection?.nodes.find(
                          (node) => node.id === edge.target,
                        )?.name ?? short(edge.target)}
                      </button>
                    </dd>
                    {edge.rule_id && (
                      <>
                        <dt>Rule</dt>
                        <dd>{edge.rule_id}</dd>
                      </>
                    )}
                  </dl>
                  <button
                    className="studio-explain-button"
                    disabled={!!busy || !edge.relationship_id}
                    onClick={() => void explain()}
                  >
                    ✧ Why this relationship?
                  </button>
                  <details className="studio-advanced">
                    <summary>Advanced</summary>
                    <p>Element ID</p>
                    <code>
                      {edge.relationship_id ?? "Presentation projection"}
                    </code>
                    <p>Revision</p>
                    <code>{edge.revision_id}</code>
                  </details>
                </>
              ) : selectedNode || inspector ? (
                <>
                  <div className="studio-inspector-heading">
                    <span className="studio-kind-pill">
                      {(inspector?.element ?? selectedNode)?.semantic_kind}
                    </span>
                    <h2>{(inspector?.element ?? selectedNode)?.name}</h2>
                    <p>
                      {(inspector?.element ?? selectedNode)?.qualified_name}
                    </p>
                  </div>
                  <div className="studio-inspector-actions">
                    <button
                      disabled={
                        !!busy ||
                        inspectionRevision !== revision ||
                        !(inspector?.element ?? selectedNode)?.source_available
                      }
                      onClick={() => void openSource()}
                    >
                      ⌁ Open source
                    </button>
                    <button disabled={!!busy} onClick={() => void explain()}>
                      ✧ Why?
                    </button>
                  </div>
                  <dl>
                    <dt>Origin</dt>
                    <dd>{(inspector?.element ?? selectedNode)?.origin}</dd>
                    <dt>Owner</dt>
                    <dd>
                      {inspector?.owner ? (
                        <button onClick={() => select(inspector.owner!.id)}>
                          {inspector.owner.name}
                        </button>
                      ) : (
                        "Root context"
                      )}
                    </dd>
                    <dt>Revision</dt>
                    <dd>{short(inspectionRevision)}</dd>
                  </dl>
                  {inspector ? (
                    <>
                      <FeatureList
                        title="Effective types"
                        values={inspector.effective_types}
                        onSelect={select}
                      />
                      <FeatureList
                        title="Owned features"
                        values={inspector.owned_features}
                        onSelect={select}
                      />
                      <FeatureList
                        title="Effective features"
                        values={inspector.effective_features}
                        onSelect={select}
                      />
                      <FeatureList
                        title="Specializations"
                        values={inspector.specializations}
                        onSelect={select}
                      />
                      <FeatureList
                        title="Subsettings"
                        values={inspector.subsettings}
                        onSelect={select}
                      />
                      <FeatureList
                        title="Redefinitions"
                        values={inspector.redefinitions}
                        onSelect={select}
                      />
                    </>
                  ) : (
                    <p className="studio-inspector-loading">
                      Reading semantic context…
                    </p>
                  )}
                  <div className="studio-model-command">
                    <button
                      disabled={!!busy}
                      onClick={() => {
                        if (!selection || !projection) return;
                        setDefinition({
                          ...projection.view,
                          hidden_elements: [
                            ...projection.view.hidden_elements,
                            selection,
                          ],
                        });
                        setSelection(undefined);
                      }}
                    >
                      Hide in this view
                    </button>
                    <button
                      disabled={
                        !!busy ||
                        inspectionRevision !== revision ||
                        !["PartDefinition", "PartUsage"].includes(
                          (inspector?.element ?? selectedNode)?.semantic_kind ??
                            "",
                        ) ||
                        !(inspector?.element ?? selectedNode)?.source_available
                      }
                      onClick={() => setShowCreate((previous) => !previous)}
                    >
                      + Create nested part
                    </button>
                    {showCreate && (
                      <form
                        onSubmit={(event) => {
                          event.preventDefault();
                          void createPart();
                        }}
                      >
                        <label>
                          Part name
                          <input
                            aria-label="New part name"
                            value={partName}
                            onChange={(event) =>
                              setPartName(event.target.value)
                            }
                            placeholder="newSubsystem"
                            required
                          />
                        </label>
                        <small>Prepares a Working revision for review.</small>
                        <button
                          className="studio-primary"
                          disabled={!!busy || !partName.trim()}
                        >
                          Prepare candidate
                        </button>
                      </form>
                    )}
                  </div>
                  <details className="studio-advanced">
                    <summary>Advanced & evidence</summary>
                    <p>Element ID</p>
                    <code>{selection}</code>
                    <p>Profile</p>
                    <code>{inspector?.profile ?? "Loading…"}</code>
                    {inspector?.source && (
                      <>
                        <p>Source location</p>
                        <code>
                          {inspector.source.path}:{inspector.source.start}–
                          {inspector.source.end}
                        </code>
                      </>
                    )}
                    {inspector?.queries.map((query) => (
                      <div className="studio-query-evidence" key={query.name}>
                        <b>{query.name}</b>
                        <span>{query.completeness}</span>
                        <small>
                          {query.positive_dependency_count} evidence ·{" "}
                          {query.search_dependency_count} searches
                        </small>
                        {query.diagnostics.map((item, index) => (
                          <p key={index}>{item}</p>
                        ))}
                      </div>
                    ))}
                  </details>
                </>
              ) : (
                <div className="studio-inspector-empty">
                  <span>◇</span>
                  <h3>Everything has context.</h3>
                  <p>
                    Select an element or relationship to inspect its semantic
                    meaning, source, and evidence.
                  </p>
                  <small>Selection is shared across every lens.</small>
                </div>
              )}
            </aside>
          </div>
          <footer className="studio-footer">
            <nav aria-label="Studio worlds">
              {lenses.map((item) => (
                <button
                  key={item.name}
                  className={lens === item.name ? "active" : ""}
                  onClick={() => changeLens(item.name)}
                >
                  <span>{item.icon}</span>
                  {item.name}
                </button>
              ))}
            </nav>
            <div className="studio-footer-status">
              <span className="studio-status-dot" />
              {busy ||
                `${projection?.nodes.length ?? 0} elements · immutable revision ${short(revision)}`}
            </div>
          </footer>
        </>
      )}
      {explanation && (
        <ExplanationDialog
          value={explanation}
          onClose={() => setExplanation(undefined)}
          onSelect={(id) => {
            select(id);
            setExplanation(undefined);
          }}
        />
      )}
      {candidate && (
        <div className="studio-modal-backdrop">
          <section
            className="studio-candidate-dialog"
            role="dialog"
            aria-modal="true"
            aria-label="Review candidate revision"
          >
            <header>
              <div>
                <p className="studio-eyebrow">SOURCE-BACKED SEMANTIC COMMAND</p>
                <h2>Review candidate</h2>
                <p>
                  {short(candidate.base_revision)} →{" "}
                  {short(candidate.revision_id)}{" "}
                  <span className="studio-kind-pill">
                    {validationLabel(candidate.validation)}
                  </span>
                </p>
              </div>
              <button
                aria-label="Discard candidate"
                disabled={!!busy}
                onClick={() =>
                  void task("Discarding candidate", async () => {
                    await studioApi(
                      `/candidates/${candidate.id}`,
                      undefined,
                      "DELETE",
                    );
                    setCandidate(undefined);
                  })
                }
              >
                ×
              </button>
            </header>
            <div className="studio-candidate-canvas">
              <Viewport
                projection={candidate.projection}
                filters={families}
                changes={candidateChanges?.nodes}
                edgeChanges={candidateChanges?.edges}
                onSelect={() => {}}
                onEdge={() => {}}
              />
            </div>
            <details className="studio-candidate-source" open>
              <summary>
                Source reconciliation · {candidate.source_preview.path}
              </summary>
              <div>
                <section>
                  <h3>Before</h3>
                  <pre>{candidate.source_preview.before}</pre>
                </section>
                <section>
                  <h3>Candidate</h3>
                  <pre>{candidate.source_preview.after}</pre>
                </section>
              </div>
            </details>
            <footer>
              <p>Only your commit advances the durable branch.</p>
              <button
                disabled={!!busy}
                onClick={() =>
                  void task("Rejecting candidate", async () => {
                    await studioApi(
                      `/candidates/${candidate.id}`,
                      undefined,
                      "DELETE",
                    );
                    setCandidate(undefined);
                  })
                }
              >
                Reject
              </button>
              <button
                disabled={
                  !!busy ||
                  validationLabel(candidate.validation) === "Validated"
                }
                onClick={() =>
                  void task("Validating candidate", async () => {
                    const next = await studioApi<Candidate>(
                      `/candidates/${candidate.id}/validate`,
                      {},
                    );
                    setCandidate(next);
                  })
                }
              >
                Validate
              </button>
              <button
                className="studio-primary"
                disabled={
                  !!busy ||
                  validationLabel(candidate.validation) !== "Validated"
                }
                onClick={() =>
                  void task("Committing candidate", async () => {
                    const receipt = await studioApi<{ revision_id: string }>(
                      `/candidates/${candidate.id}/commit`,
                      {},
                    );
                    setCandidate(undefined);
                    await refreshSession(receipt.revision_id);
                    setDefinition(viewDefinition("System"));
                    setLens("System");
                  })
                }
              >
                Accept & commit
              </button>
            </footer>
            {busy && (
              <p role="status" className="studio-candidate-busy">
                {busy}…
              </p>
            )}
            {error && (
              <p role="alert" className="studio-error">
                {error}
              </p>
            )}
          </section>
        </div>
      )}
    </div>
  );
}

function FeatureList({
  title,
  values,
  onSelect,
}: {
  title: string;
  values: FeatureSummary[];
  onSelect: (id: string) => void;
}) {
  if (!values.length) return null;
  return (
    <section className="studio-feature-list">
      <h3>
        {title}
        <span>{values.length}</span>
      </h3>
      {values.slice(0, 20).map((item) => (
        <button key={item.id} onClick={() => onSelect(item.id)}>
          <span>◇</span>
          {item.name || item.semantic_kind}
          <small>{item.semantic_kind}</small>
        </button>
      ))}
      {values.length > 20 && (
        <p className="studio-muted">{values.length - 20} more features</p>
      )}
    </section>
  );
}
function History({
  session,
  revision,
  disabled,
  onSelect,
  onCompare,
}: {
  session: Session;
  revision: string;
  disabled: boolean;
  onSelect: (id: string) => void;
  onCompare: (id: string) => void;
}) {
  const remaining = new Map(
    session.revisions.map((item) => [item.revision_id, item]),
  );
  const ordered: Session["revisions"] = [];
  while (remaining.size) {
    const next = [...remaining.values()]
      .filter(
        (item) =>
          !item.parent_revision_id || !remaining.has(item.parent_revision_id),
      )
      .sort((a, b) => a.revision_id.localeCompare(b.revision_id));
    const items = next.length
      ? next
      : [
          [...remaining.values()].sort((a, b) =>
            a.revision_id.localeCompare(b.revision_id),
          )[0],
        ];
    for (const item of items) {
      ordered.push(item);
      remaining.delete(item.revision_id);
    }
  }
  const lanes = new Map<string, number>();
  const childCount = new Map<string, number>();
  let nextLane = 0;
  for (const item of ordered) {
    const parentLane = item.parent_revision_id
      ? lanes.get(item.parent_revision_id)
      : undefined;
    const siblings = childCount.get(item.parent_revision_id ?? "") ?? 0;
    lanes.set(
      item.revision_id,
      parentLane !== undefined && siblings === 0 ? parentLane : nextLane++,
    );
    childCount.set(item.parent_revision_id ?? "", siblings + 1);
  }
  const railWidth = 35 + Math.max(0, nextLane - 1) * 16;
  return (
    <div className="studio-history-world">
      <p className="studio-world-description">
        Every revision is immutable. Select one to move the entire Studio
        context through time.
      </p>
      <div className="studio-history-branches">
        {session.branches.map((item) => (
          <button
            key={item.id}
            disabled={disabled}
            onClick={() => onSelect(item.head)}
          >
            <span>⑂</span>
            <b>{item.name}</b>
            <code>{short(item.head)}</code>
          </button>
        ))}
      </div>
      <div className="studio-timeline">
        <svg
          className="studio-history-links"
          width={railWidth}
          height={ordered.length * 127}
          aria-label="Revision parent relationships"
        >
          {ordered.map((item, index) => {
            const parentIndex = ordered.findIndex(
              (parent) => parent.revision_id === item.parent_revision_id,
            );
            if (parentIndex < 0) return null;
            const sx = 17.5 + (lanes.get(item.parent_revision_id!) ?? 0) * 16;
            const tx = 17.5 + (lanes.get(item.revision_id) ?? 0) * 16;
            const sy = parentIndex * 127 + 27.5,
              ty = index * 127 + 27.5;
            return (
              <path
                key={item.revision_id}
                d={`M ${sx} ${sy} C ${sx} ${sy + 55}, ${tx} ${ty - 55}, ${tx} ${ty}`}
              >
                <title>
                  {short(item.parent_revision_id)} → {short(item.revision_id)}
                </title>
              </path>
            );
          })}
        </svg>
        {ordered.map((item, index) => (
          <article
            key={item.revision_id}
            className={item.revision_id === revision ? "active" : ""}
          >
            <div className="studio-timeline-rail" style={{ width: railWidth }}>
              <span
                style={{ left: 5 + (lanes.get(item.revision_id) ?? 0) * 16 }}
              >
                {index + 1}
              </span>
            </div>
            <button
              className="studio-revision-card"
              disabled={disabled}
              onClick={() => onSelect(item.revision_id)}
            >
              <div>
                <code>{short(item.revision_id)}</code>
                <span
                  className={`studio-validation ${validationLabel(item.validation).toLowerCase()}`}
                >
                  {validationLabel(item.validation)}
                </span>
              </div>
              <h3>
                {session.branches
                  .filter((branch) => branch.head === item.revision_id)
                  .map((branch) => branch.name)
                  .join(" · ") || "Project revision"}
              </h3>
              <p>
                {item.parent_revision_id
                  ? `Parent ${short(item.parent_revision_id)}`
                  : "Initial project"}
              </p>
            </button>
            {item.revision_id !== revision && (
              <button
                disabled={disabled}
                className="studio-compare-button"
                onClick={() => onCompare(item.revision_id)}
              >
                Compare →
              </button>
            )}
          </article>
        ))}
      </div>
    </div>
  );
}
function ExplanationDialog({
  value,
  onClose,
  onSelect,
}: {
  value: Explanation;
  onClose: () => void;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="studio-modal-backdrop">
      <section
        className="studio-explanation-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Semantic explanation"
      >
        <header>
          <div>
            <p className="studio-eyebrow">
              EXPLAIN · {short(value.revision_id)}
            </p>
            <h2>Why is this true?</h2>
          </div>
          <button aria-label="Close explanation" onClick={onClose}>
            ×
          </button>
        </header>
        <div className="studio-explanation-summary">
          <span className="studio-kind-pill">{value.origin}</span>
          <p>
            {value.rule_name ?? value.rule_id ?? "Canonical semantic evidence"}
          </p>
          <small>
            {value.profile} · {value.evidence_count} evidence records
          </small>
        </div>
        <div className="studio-evidence-graph">
          {value.nodes.map((node) => (
            <article key={node.id} className={node.kind.toLowerCase()}>
              <span>{node.kind === "Rule" ? "ƒ" : "◇"}</span>
              <div>
                <small>{node.kind}</small>
                {node.element_id ? (
                  <button onClick={() => onSelect(node.element_id!)}>
                    {node.label}
                  </button>
                ) : (
                  <b>{node.label}</b>
                )}
                {value.edges
                  .filter((edge) => edge.source === node.id)
                  .map((edge, index) => (
                    <p key={index}>
                      ↓ {edge.label}{" "}
                      <strong>
                        {value.nodes.find((target) => target.id === edge.target)
                          ?.label ?? edge.target}
                      </strong>
                    </p>
                  ))}
              </div>
            </article>
          ))}
        </div>
        <footer>
          {value.truncated
            ? "Bounded explanation: additional evidence remains available through semantic queries."
            : "Connections in this explanation are presentation links over canonical evidence."}
        </footer>
      </section>
    </div>
  );
}
function assertRevision(actual: string, expected: string) {
  if (actual !== expected)
    throw new Error(
      "Revision mismatch: this response belongs to another immutable context.",
    );
}
function message(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}
