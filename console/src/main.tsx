import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "./style.css";
import { BehaviorGraph } from "./BehaviorGraph";
import { ModelEdits } from "./ModelEdits";
import { Studio } from "./studio/Studio";

type Json = any;
type Element = {
  id: string;
  name: string | null;
  kind: string;
  qualified_name: string;
  owner: string | null;
  span: {
    file: string;
    start: number;
    end: number;
    line: number;
    column: number;
  };
  references: { role: string; target: string | null; path: string }[];
  documentation: string;
  modifiers: string[];
  value: Json;
  unsupported: string[];
};
type Projection = {
  revision_id: string;
  source_digest: string;
  elements: Element[];
  sources: Record<string, string>;
  diagnostics: Json[];
};
const short = (s: string | undefined) => s?.slice(0, 8) ?? "—";
const pretty = (v: Json) => JSON.stringify(v, null, 2);
let sessionToken =
  new URLSearchParams(location.hash.slice(1)).get("token") ??
  sessionStorage.getItem("agentique-session") ??
  "";
if (sessionToken) {
  sessionStorage.setItem("agentique-session", sessionToken);
  history.replaceState(null, "", location.pathname);
}
async function api(path: string, body?: Json) {
  const r = await fetch("/api/agentique" + path, {
    headers: {
      Authorization: "Bearer " + sessionToken,
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    ...(body ? { method: "POST", body: JSON.stringify(body) } : {}),
  });
  if (!r.ok) {
    let e;
    try {
      e = await r.json();
    } catch {
      e = { message: await r.text() };
    }
    throw Error(`${e.code ?? r.status}: ${e.message ?? "Request failed"}`);
  }
  return r.json();
}
function App() {
  const [connected, setConnected] = useState(false),
    [credential, setCredential] = useState(""),
    [state, setState] = useState<Json>(),
    [model, setModel] = useState<Projection>(),
    [selection, setSelection] = useState<string>(),
    [runId, setRunId] = useState<string>(),
    [run, setRun] = useState<Json>(),
    [traceFrom, setTraceFrom] = useState(0),
    [job, setJob] = useState<Json>(),
    [scenario, setScenario] = useState(""),
    [templates, setTemplates] = useState<Json>({}),
    [message, setMessage] = useState(""),
    [messages, setMessages] = useState<Json[]>([]),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [proposal, setProposal] = useState<Json>(),
    [rename, setRename] = useState(""),
    [query, setQuery] = useState(""),
    [layout, setLayout] = useState(false),
    [sourceDraft, setSourceDraft] = useState(""),
    [identityDraft, setIdentityDraft] = useState("{}"),
    [draftOpen, setDraftOpen] = useState(false);
  const selected = model?.elements.find((e) => e.id === selection),
    head = state?.head_revision_id,
    revision = model?.revision_id;
  const old = revision !== head;
  async function refresh(keepRevision = true, refreshRunId = runId) {
    const s = await api("/state");
    setState(s);
    if (
      !keepRevision ||
      !model ||
      model.revision_id === state?.head_revision_id
    ) {
      setModel(s.model);
      if (!selection)
        setSelection(
          s.model.elements.find((e: Element) => e.kind === "ExhibitStateUsage")
            ?.id ?? s.model.elements[0]?.id,
        );
    }
    setConnected(true);
    if (refreshRunId)
      setRun(
        await api(
          `/runs/${refreshRunId}?from=${refreshRunId === runId ? traceFrom : 0}&limit=1000`,
        ),
      );
    return s;
  }
  async function load() {
    await refresh(false);
    const t = await api("/templates");
    setTemplates(t);
    setScenario(pretty(t.accepted));
  }
  useEffect(() => {
    if (sessionToken) void load().catch((e) => setError(e.message));
  }, []);
  useEffect(() => {
    if (!connected) return;
    const timer = setInterval(() => {
      void api("/events?after=" + (state?.event_cursor ?? 0))
        .then(async (e) => {
          if (e.events.length) await refresh();
        })
        .catch((e) => setError(e.message));
    }, 700);
    return () => clearInterval(timer);
  }, [
    connected,
    state?.event_cursor,
    model?.revision_id,
    runId,
    selection,
    traceFrom,
  ]);
  useEffect(() => {
    setRename(selected?.name ?? "");
    setSourceDraft(
      selected
        ? (state?.drafts?.find((d: Json) => d.file === selected.span.file)
            ?.source ??
            model?.sources[selected.span.file] ??
            "")
        : "",
    );
    setIdentityDraft(
      pretty(
        Object.fromEntries(
          (model?.elements ?? [])
            .filter((e) => e.span.file === selected?.span.file)
            .map((e) => [`${e.span.file}|${e.qualified_name}`, e.id]),
        ),
      ),
    );
  }, [selection, model?.revision_id]);
  async function task(f: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await f();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function command(payload: Json, base = revision) {
    const envelope = {
      command_id: crypto.randomUUID(),
      project_id: state.project_id,
      base_revision_id: base,
      payload,
    };
    if (!["propose_change", "save_draft"].includes(payload.op))
      return api("/commands", envelope);
    const queued = await api("/jobs", envelope);
    setJob({ job_id: queued.job_id, progress: { stage: "queued" } });
    try {
      for (;;) {
        const current = await api(`/jobs/${queued.job_id}`);
        setJob(current);
        if (current.status === "committed") return current.result;
        if (current.error)
          throw Error(`${current.error.code}: ${current.error.message}`);
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    } finally {
      setJob(undefined);
    }
  }
  async function action(a: Json, base = revision) {
    return command({ op: "act", action: a, approval_id: null }, base);
  }
  async function prepare() {
    const saved = await action({
      op: "save_scenario",
      scenario: JSON.parse(scenario),
    });
    const result = await action({
      op: "prepare_run",
      scenario_revision_id: saved.id,
    });
    setRunId(result.prepared_run_id);
    setTraceFrom(0);
    setRun(await api(`/runs/${result.prepared_run_id}?limit=1000`));
    await refresh(true, result.prepared_run_id);
  }
  async function control(operation: string) {
    if (!runId) return;
    const latest = await api(`/runs/${runId}?limit=1`);
    const result = await action(
      {
        op: "control_run",
        run_id: runId,
        expected_control_version: latest.control_version,
        operation,
      },
      head,
    );
    const id = result.run_id;
    setRunId(id);
    if (id !== runId) setTraceFrom(0);
    setRun(await api(`/runs/${id}?limit=1000`));
    await refresh(true, id);
  }
  async function selectRun(id: string) {
    setTraceFrom(0);
    setRunId(id);
    setRun(await api(`/runs/${id}?limit=1000`));
  }
  async function tracePage(from: number) {
    const page = await api(`/runs/${runId}?from=${from}&limit=1000`);
    setTraceFrom(from);
    setRun(page);
  }
  async function showRevision(id: string, element?: string) {
    setModel(await api("/inspect?revision_id=" + id));
    if (element) setSelection(element);
  }
  async function ask() {
    const text = message.trim();
    if (!text) return;
    setMessage("");
    setMessages((m) => [...m, { role: "operator", text, revision }]);
    const reply = await api("/assistant", {
      message: text,
      context: {
        project_id: state.project_id,
        revision_id: revision,
        selection: selection ?? null,
        run_id: runId ?? null,
      },
    });
    setMessages((m) => [...m, { role: "assistant", ...reply }]);
    await refresh();
  }
  async function approve(review: Json) {
    const approval = await command(
      {
        op: "approve",
        request_id: review.id,
        payload_digest: review.payload_digest,
        expires_in_seconds: 300,
      },
      review.base_revision_id,
    );
    const result = await api("/assistant/execute", {
      request_id: review.id,
      approval_id: approval.id,
      command_id: crypto.randomUUID(),
      base_revision_id: review.base_revision_id,
    });
    setMessages((items) =>
      items.map((m) =>
        m.review?.id === review.id
          ? { ...m, review: { ...m.review, executed: true } }
          : m,
      ),
    );
    setMessages((m) => [
      ...m,
      {
        role: "engine",
        text: "Approved action committed durably.",
        result,
        revision: result.new_revision_id ?? revision,
      },
    ]);
    if (result.prepared_run_id || result.run_id) {
      await selectRun(result.prepared_run_id ?? result.run_id);
    }
    await refresh(false);
  }
  async function download() {
    const response = await fetch("/api/agentique/export/" + revision, {
      headers: { Authorization: "Bearer " + sessionToken },
    });
    if (!response.ok) throw Error("Export failed");
    const url = URL.createObjectURL(await response.blob());
    const a = document.createElement("a");
    a.href = url;
    a.download = "Agentique.kpar";
    a.click();
    URL.revokeObjectURL(url);
  }
  let behavior = selected;
  const visitedOwners = new Set<string>();
  while (
    behavior &&
    !["StateDefinition", "StateUsage", "ExhibitStateUsage"].includes(
      behavior.kind,
    ) &&
    !visitedOwners.has(behavior.id)
  ) {
    visitedOwners.add(behavior.id);
    behavior = model?.elements.find((e) => e.id === behavior?.owner);
  }
  if (
    behavior?.kind === "ExhibitStateUsage" ||
    behavior?.kind === "StateUsage"
  ) {
    const type = behavior.references.find((r) => r.role === "type")?.target;
    if (type) behavior = model?.elements.find((e) => e.id === type);
  }
  if (behavior?.kind !== "StateDefinition" && behavior?.kind !== "StateUsage")
    behavior = model?.elements.find((e) => e.kind === "StateDefinition");
  const states =
      model?.elements.filter(
        (e) => e.owner === behavior?.id && e.kind === "StateUsage",
      ) ?? [],
    transitions =
      model?.elements.filter(
        (e) => e.owner === behavior?.id && e.kind === "TransitionUsage",
      ) ?? [];
  const elementName = (id: string | undefined | null) =>
    model?.elements.find((e) => e.id === id)?.name ?? short(id ?? undefined);
  const refName = (e: Element, role: string) =>
    e.references.find((r) => r.role === role)?.path ?? "—";
  const contextBadge = (
    <span className="context">
      {old ? "Historical model" : "Authored model"} · {short(revision)}
    </span>
  );
  function tree(owner: string | null, depth = 0): React.ReactNode {
    return model?.elements
      .filter((e) => e.owner === owner && e.name)
      .map((e) => {
        const children = model.elements.filter(
          (c) => c.owner === e.id && c.name,
        );
        const match =
          !query ||
          e.qualified_name.toLowerCase().includes(query.toLowerCase());
        return (
          <React.Fragment key={e.id}>
            {match && (
              <button
                className={"tree-row " + (selection === e.id ? "selected" : "")}
                style={{ paddingLeft: 12 + depth * 15 }}
                onClick={() => setSelection(e.id)}
                aria-pressed={selection === e.id}
              >
                <span
                  className={
                    "kind-dot " + (e.kind.includes("State") ? "state-dot" : "")
                  }
                >
                  {e.kind === "Package"
                    ? "▱"
                    : e.kind.endsWith("Definition")
                      ? "◇"
                      : "·"}
                </span>
                <span>{e.name}</span>
                <small>
                  {e.kind.replace("Definition", " def").replace("Usage", "")}
                </small>
              </button>
            )}
            {children.length > 0 && tree(e.id, depth + 1)}
          </React.Fragment>
        );
      });
  }
  if (!connected)
    return (
      <main className="login">
        <div className="brand">
          <span className="mark">A/</span>Agentique
        </div>
        <h1>Open your workspace.</h1>
        <p>
          Use the local Console link printed by the Rust server, or enter its
          session credential.
        </p>
        <form
          onSubmit={(e) => {
            e.preventDefault();
            sessionToken = credential;
            sessionStorage.setItem("agentique-session", sessionToken);
            void task(load);
          }}
        >
          <label htmlFor="credential">Local session credential</label>
          <input
            id="credential"
            type="password"
            value={credential}
            onChange={(e) => setCredential(e.target.value)}
          />
          <button className="primary">Connect to Engine</button>
        </form>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
      </main>
    );
  return (
    <>
      <a className="skip" href="#canvas">
        Skip to model views
      </a>
      <header className="topbar">
        <div className="brand">
          <span className="mark">A/</span>Agentique{" "}
          <span className="version">0.1</span>
        </div>
        <div className="workspace-name">
          <span className="online" />
          Local workspace <span>/</span> Self-model
        </div>
        <div className="top-actions">
          <span className="revision">rev {short(head)}</span>
          <button onClick={() => void task(download)}>Export project ↗</button>
        </div>
      </header>
      <div className="workspace">
        <aside className="structure panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">SURFACE / 01</span>
              <h2>Structure</h2>
            </div>
            <span className="count">
              {model?.elements.filter((e) => e.name).length}
            </span>
          </div>
          {contextBadge}
          <label className="sr-only" htmlFor="filter">
            Filter model elements
          </label>
          <input
            className="search"
            id="filter"
            type="search"
            placeholder="Find an element…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <nav aria-label="Model structure" className="tree">
            {tree(null)}
          </nav>
          <div className="structure-footer">
            <span className="online" />
            Engine connected <small>KerML 1.0 · SysML 2.0</small>
          </div>
        </aside>
        <main id="canvas" className={"canvas " + (layout ? "alternate" : "")}>
          <div className="canvas-top">
            <div>
              <span className="eyebrow">MODEL WORKSPACE</span>
              <h1>A model we can run.</h1>
              <p>
                Inspect the design. Configure an experiment. Follow the
                evidence.
              </p>
            </div>
            <button
              title="Rearrange the views without editing the model"
              onClick={() => setLayout((v) => !v)}
            >
              Rearrange views
            </button>
          </div>
          {old && (
            <div className="notice">
              Viewing historical revision {short(revision)}. Runs keep their
              original model.{" "}
              <button onClick={() => void task(() => showRevision(head))}>
                Return to accepted model
              </button>
            </div>
          )}
          {error && (
            <div className="error" role="alert">
              {error}
              <button aria-label="Dismiss error" onClick={() => setError("")}>
                ×
              </button>
            </div>
          )}
          <section className="panel behaviour">
            <div className="panel-heading">
              <div>
                <span className="eyebrow">SURFACE / 02</span>
                <h2>{behavior?.name ?? "Selected behaviour"}</h2>
              </div>
              {contextBadge}
            </div>
            <div className="behaviour-meta">
              <span>Exclusive state region</span>
              <span>{states.length} states</span>
              <span>{transitions.length} transitions</span>
              <span>AGQ-SEQ-01</span>
            </div>
            <BehaviorGraph
              states={states}
              transitions={transitions}
              active={
                run?.model_revision_id === revision ? run?.active : undefined
              }
              initial={
                model?.elements
                  .find(
                    (e) =>
                      e.owner === behavior?.id &&
                      e.kind === "SuccessionAsUsage",
                  )
                  ?.references.find((r) => r.role === "target")?.target ??
                undefined
              }
              onSelect={setSelection}
            />
            <div className="table-wrap">
              <table>
                <caption>
                  Model transitions · select a row to inspect its source
                </caption>
                <thead>
                  <tr>
                    <th>Transition</th>
                    <th>From</th>
                    <th>Input</th>
                    <th>To</th>
                  </tr>
                </thead>
                <tbody>
                  {transitions.map((t) => (
                    <tr key={t.id}>
                      <td>
                        <button
                          className="text-button"
                          onClick={() => setSelection(t.id)}
                        >
                          {t.name}
                        </button>
                      </td>
                      <td>{refName(t, "source")}</td>
                      <td>
                        <code>{refName(t, "payload_type")}</code>
                      </td>
                      <td>{refName(t, "target")}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
          <section className="panel properties">
            <div className="panel-heading">
              <div>
                <span className="eyebrow">SURFACE / 03</span>
                <h2>{selected?.name ?? "Properties"}</h2>
              </div>
              {contextBadge}
            </div>
            {selected && (
              <>
                <dl className="property-grid">
                  <div>
                    <dt>Metaclass</dt>
                    <dd>{selected.kind}</dd>
                  </div>
                  <div>
                    <dt>Stable identity</dt>
                    <dd>
                      <code>{selected.id}</code>
                    </dd>
                  </div>
                  <div>
                    <dt>Source</dt>
                    <dd>
                      {selected.span.file}:{selected.span.line}
                    </dd>
                  </div>
                  <div>
                    <dt>Locator</dt>
                    <dd>{selected.qualified_name}</dd>
                  </div>
                </dl>
                {selected.documentation && (
                  <p className="documentation">
                    {selected.documentation.trim()}
                  </p>
                )}
                {selected.references.length > 0 && (
                  <div className="references">
                    {selected.references.map((r, i) => (
                      <button
                        key={i}
                        onClick={() => {
                          if (model?.elements.some((e) => e.id === r.target))
                            setSelection(r.target!);
                        }}
                      >
                        {r.role} <strong>{r.path}</strong>
                      </button>
                    ))}
                  </div>
                )}
                <details>
                  <summary>Exact source</summary>
                  <pre>
                    {new TextDecoder().decode(
                      new TextEncoder()
                        .encode(model?.sources[selected.span.file] ?? "")
                        .slice(selected.span.start, selected.span.end),
                    )}
                  </pre>
                </details>
                <form
                  className="rename-form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void task(async () =>
                      setProposal(
                        await command({
                          op: "propose_change",
                          edits: [
                            {
                              kind: "rename",
                              element_id: selection,
                              name: rename,
                            },
                          ],
                        }),
                      ),
                    );
                  }}
                >
                  <label htmlFor="rename">Element name</label>
                  <div>
                    <input
                      id="rename"
                      value={rename}
                      onChange={(e) => setRename(e.target.value)}
                      disabled={old}
                    />
                    <button disabled={busy || old || rename === selected.name}>
                      Review rename
                    </button>
                  </div>
                </form>
                <ModelEdits
                  key={selection}
                  selected={selected}
                  elements={model?.elements ?? []}
                  disabled={busy || old}
                  onPropose={(edit) =>
                    void task(async () =>
                      setProposal(
                        await command({ op: "propose_change", edits: [edit] }),
                      ),
                    )
                  }
                />
                <details
                  open={draftOpen}
                  onToggle={(e) => setDraftOpen(e.currentTarget.open)}
                >
                  <summary>Source draft editor</summary>
                  <label htmlFor="source-draft">
                    {selected.span.file} · drafts do not replace the accepted
                    model
                  </label>
                  <textarea
                    id="source-draft"
                    value={sourceDraft}
                    onChange={(e) => setSourceDraft(e.target.value)}
                    spellCheck={false}
                  />
                  <button
                    disabled={busy || old}
                    onClick={() =>
                      void task(async () => {
                        const d = await command({
                          op: "save_draft",
                          file: selected.span.file,
                          source: sourceDraft,
                        });
                        setMessages((m) => [
                          ...m,
                          {
                            role: "engine",
                            text: "Draft saved. Accepted revision retained.",
                            result: d,
                          },
                        ]);
                        await refresh();
                      })
                    }
                  >
                    Save and validate draft
                  </button>
                  <details>
                    <summary>
                      Review source with explicit identity mapping
                    </summary>
                    <p className="editing-help">
                      Keep each existing ID attached to the same intended
                      element. For a rename, update its locator in this mapping.
                      The Engine rejects missing or unresolved mappings.
                    </p>
                    <label htmlFor="identity-mapping">
                      Identity mapping for source revision
                    </label>
                    <textarea
                      id="identity-mapping"
                      spellCheck={false}
                      value={identityDraft}
                      onChange={(e) => setIdentityDraft(e.target.value)}
                    />
                    <button
                      disabled={busy || old}
                      onClick={() =>
                        void task(async () =>
                          setProposal(
                            await command({
                              op: "propose_change",
                              edits: [
                                {
                                  kind: "replace_source",
                                  file: selected.span.file,
                                  source: sourceDraft,
                                  identity_mapping: JSON.parse(identityDraft),
                                },
                              ],
                            }),
                          ),
                        )
                      }
                    >
                      Review source change
                    </button>
                  </details>
                </details>
              </>
            )}
            {proposal && (
              <div className="proposal">
                <h3>Review model change</h3>
                <p>Proposal · base {short(proposal.base_revision_id)}</p>
                {proposal.diff.map((d: Json) => (
                  <details key={d.file} open>
                    <summary>{d.file}</summary>
                    <div className="diff">
                      <div>
                        <span>Before</span>
                        <pre>{d.before}</pre>
                      </div>
                      <div>
                        <span>After</span>
                        <pre>{d.after}</pre>
                      </div>
                    </div>
                  </details>
                ))}
                {proposal.diagnostics?.length > 0 && (
                  <pre>{pretty(proposal.diagnostics)}</pre>
                )}
                <button
                  className="primary"
                  disabled={
                    busy ||
                    !proposal.valid ||
                    proposal.base_revision_id !== head
                  }
                  onClick={() =>
                    void task(async () => {
                      await action(
                        {
                          op: "commit_change",
                          proposal_id: proposal.proposal_id,
                        },
                        proposal.base_revision_id,
                      );
                      setProposal(undefined);
                      await refresh(false);
                    })
                  }
                >
                  Commit reviewed change
                </button>
                <button onClick={() => setProposal(undefined)}>
                  Close review
                </button>
              </div>
            )}
          </section>
          <section className="panel experiment">
            <div className="panel-heading">
              <div>
                <span className="eyebrow">SURFACE / 04</span>
                <h2>Experiment</h2>
              </div>
              <span className="context">
                {runId ? "Run · " + short(runId) : "Scenario draft"}
              </span>
            </div>
            <div className="scenario-presets">
              <span>Scenario</span>
              {["accepted", "rejected", "exhausted"].map((name) => (
                <button
                  key={name}
                  onClick={() => {
                    const s = structuredClone(
                      templates[name === "exhausted" ? "accepted" : name],
                    );
                    if (name === "exhausted") s.inputs = s.inputs.slice(0, 1);
                    setScenario(pretty(s));
                  }}
                >
                  {name}
                </button>
              ))}
            </div>
            <details className="scenario-editor">
              <summary>Edit scenario inputs, bindings and limits</summary>
              <label htmlFor="scenario">
                Scenario configuration · references the authored model
              </label>
              <textarea
                id="scenario"
                spellCheck={false}
                value={scenario}
                onChange={(e) => setScenario(e.target.value)}
              />
            </details>
            <div className="controls">
              <button
                className="primary"
                disabled={busy || old}
                onClick={() => void task(prepare)}
              >
                Prepare new run
              </button>
              {[
                "initialise",
                "run",
                "pause",
                "step",
                "stop",
                "reset_as_new_run",
              ].map((op) => (
                <button
                  key={op}
                  disabled={
                    busy ||
                    !runId ||
                    (op === "initialise"
                      ? run?.status !== "prepared"
                      : op === "run" || op === "step"
                        ? run?.status !== "paused"
                        : op === "pause"
                          ? run?.status !== "running"
                          : op === "stop"
                            ? [
                                "completed",
                                "stopped",
                                "blocked",
                                "failed",
                                "interrupted",
                              ].includes(run?.status)
                            : false)
                  }
                  onClick={() => void task(() => control(op))}
                >
                  {op === "reset_as_new_run"
                    ? "Reset as new run"
                    : op[0].toUpperCase() + op.slice(1)}
                </button>
              ))}
            </div>
            {run ? (
              <>
                <div className="run-stats">
                  <div>
                    <span>Status</span>
                    <strong className={"status " + run.status}>
                      {run.status}
                    </strong>
                  </div>
                  <div>
                    <span>Active state</span>
                    <strong>{run.active_name ?? "Not initialised"}</strong>
                  </div>
                  <div>
                    <span>Inputs consumed</span>
                    <strong>
                      {run.next_input} / {run.plan.inputs.length}
                    </strong>
                  </div>
                  <div>
                    <span>Stop reason</span>
                    <strong>{run.stop_reason ?? "—"}</strong>
                  </div>
                </div>
                <div className="pin">
                  Pinned model{" "}
                  <button
                    className="text-button"
                    onClick={() =>
                      void task(() => showRevision(run.model_revision_id))
                    }
                  >
                    {short(run.model_revision_id)}
                  </button>{" "}
                  · scenario {short(run.scenario_revision_id)} ·{" "}
                  {run.model_revision_id !== head
                    ? "Older than accepted model"
                    : "Current accepted model"}
                </div>
                <div className="table-wrap">
                  <table>
                    <caption>
                      Simulation history · source-linked semantic trace
                    </caption>
                    <thead>
                      <tr>
                        <th>Step</th>
                        <th>Record</th>
                        <th>Model source</th>
                        <th>Resulting state</th>
                      </tr>
                    </thead>
                    <tbody>
                      {run.trace_page.map((t: Json) => (
                        <tr key={t.sequence}>
                          <td>{t.semantic_step}</td>
                          <td>{t.kind}</td>
                          <td>
                            <button
                              className="text-button"
                              onClick={() =>
                                void task(() =>
                                  showRevision(
                                    t.model_revision_id,
                                    t.source_element_id,
                                  ),
                                )
                              }
                            >
                              {t.kind === "transition"
                                ? (run.plan.transitions.find(
                                    (edge: Json) =>
                                      edge.id === t.source_element_id,
                                  )?.name ?? short(t.source_element_id))
                                : t.kind === "initial"
                                  ? "initial succession"
                                  : "input port"}
                            </button>
                          </td>
                          <td>{run.plan.states[t.after] ?? "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <div className="checks">
                  <nav aria-label="Trace pages">
                    <button
                      disabled={busy || traceFrom === 0}
                      onClick={() =>
                        void task(() =>
                          tracePage(Math.max(0, traceFrom - 1000)),
                        )
                      }
                    >
                      Previous trace page
                    </button>{" "}
                    <span>
                      {run.trace_count ? traceFrom + 1 : 0}–
                      {Math.min(
                        traceFrom + run.trace_page.length,
                        run.trace_count,
                      )}{" "}
                      of {run.trace_count} records
                    </span>{" "}
                    <button
                      disabled={busy || traceFrom + 1000 >= run.trace_count}
                      onClick={() =>
                        void task(() => tracePage(traceFrom + 1000))
                      }
                    >
                      Next trace page
                    </button>
                  </nav>
                  {run.checks.map((c: Json) => (
                    <span key={c.name}>
                      <strong>{c.status}</strong> {c.name.replaceAll("_", " ")}
                    </span>
                  ))}
                </div>
                <details>
                  <summary>Run manifest, diagnostics and trace digest</summary>
                  <pre>
                    {pretty({
                      trace_digest: run.trace_digest,
                      diagnostics: run.diagnostics,
                      manifest: run.plan,
                    })}
                  </pre>
                </details>
              </>
            ) : (
              <div className="empty">
                <span>◷</span>
                <p>Prepare an experiment to create an isolated run.</p>
                <small>
                  The authored model stays unchanged as the run advances.
                </small>
              </div>
            )}
            {state.runs.length > 0 && (
              <div className="run-history">
                <label htmlFor="run-history">Run history</label>
                <select
                  id="run-history"
                  value={runId ?? ""}
                  onChange={(e) => void task(() => selectRun(e.target.value))}
                >
                  <option value="" disabled>
                    Select a run
                  </option>
                  {state.runs.map((r: Json) => (
                    <option key={r.id} value={r.id}>
                      {short(r.id)} · {r.status} · rev{" "}
                      {short(r.model_revision_id)}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </section>
        </main>
        <aside className="conversation panel">
          <div className="panel-heading">
            <div>
              <span className="eyebrow">CONVERSATION</span>
              <h2>Assistant</h2>
            </div>
            <span className={"mode " + state.assistant_mode}>
              {state.assistant_mode === "test" ? "TEST MODE" : "LIVE PROVIDER"}
            </span>
          </div>
          <div className="conversation-context">
            <span>Shared context</span>
            <strong>{selected?.name ?? "Project"}</strong>
            <small>
              Model {short(revision)}
              {runId ? " · Run " + short(runId) : ""}
            </small>
          </div>
          <div className="messages" aria-live="polite">
            <div className="welcome">
              <span className="assistant-mark">A/</span>
              <h3>Explore with evidence.</h3>
              <p>
                I can inspect the selected model and propose changes through the
                Engine.
              </p>
              <p>
                {state.assistant_mode === "test"
                  ? "Deterministic test adapter. Use “inspect”, “read run”, or “rename NewName”."
                  : "Live provider selects tools. The Engine supplies all model information."}
              </p>
              <div className="suggestions">
                <button onClick={() => setMessage("inspect")}>
                  Inspect selection
                </button>
                <button onClick={() => setMessage("read run")}>
                  Explain run state
                </button>
              </div>
            </div>
            {messages.map((m, i) => (
              <article key={i} className={"message " + m.role}>
                <span className="eyebrow">
                  {m.role === "operator"
                    ? "YOU"
                    : m.role === "engine"
                      ? "ENGINE"
                      : `ASSISTANT · ${m.mode ?? state.assistant_mode}`}
                </span>
                <p>{m.text}</p>
                {m.context && (
                  <small>
                    Model {short(m.context.revision_id)} · selection{" "}
                    {short(m.context.selection)}
                  </small>
                )}
                {m.tool_calls && (
                  <details>
                    <summary>
                      {m.tool_calls.length} real Engine tool result
                      {m.tool_calls.length === 1 ? "" : "s"}
                    </summary>
                    <pre>{pretty(m.tool_calls)}</pre>
                  </details>
                )}
                {m.result && (
                  <details>
                    <summary>Engine result</summary>
                    <pre>{pretty(m.result)}</pre>
                  </details>
                )}
                {m.review && (
                  <div className="approval">
                    <strong>
                      {m.review.executed
                        ? "Action committed"
                        : "Approval required"}
                    </strong>
                    <p>
                      Actor {m.review.actor_id}
                      <br />
                      Base {short(m.review.base_revision_id)}
                      <br />
                      Expires 5 minutes after approval
                    </p>
                    <details open>
                      <summary>Exact reviewed payload</summary>
                      <pre>{pretty(m.review.payload)}</pre>
                    </details>
                    <code>SHA-256 {m.review.payload_digest}</code>
                    <button
                      className="primary"
                      disabled={
                        busy ||
                        m.review.executed ||
                        m.review.base_revision_id !== head
                      }
                      onClick={() => void task(() => approve(m.review))}
                    >
                      Approve and execute
                    </button>
                  </div>
                )}
              </article>
            ))}
          </div>
          <form
            className="composer"
            onSubmit={(e) => {
              e.preventDefault();
              void task(ask);
            }}
          >
            <label htmlFor="message">Ask about this model</label>
            <textarea
              id="message"
              placeholder="Inspect this selection…"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  void task(ask);
                }
              }}
            />
            <div>
              <small>Ctrl + Enter to send</small>
              <button className="primary" disabled={busy || !message.trim()}>
                Send ↑
              </button>
            </div>
          </form>
          <div className="authority-note">
            Changes and execution require your approval.
          </div>
        </aside>
      </div>
      <footer className="statusbar">
        <span>Agentique Engine · local</span>
        <span>
          {job ? (
            <span role="status">
              {job.progress.stage}{" "}
              {job.progress.total
                ? `${job.progress.completed}/${job.progress.total}`
                : ""}{" "}
              <button
                disabled={
                  job.progress.committing || job.progress.cancellation_requested
                }
                onClick={() =>
                  void api(`/jobs/${job.job_id}/cancel`, {})
                    .then(() =>
                      setJob((j: Json) =>
                        j
                          ? {
                              ...j,
                              progress: {
                                ...j.progress,
                                cancellation_requested: true,
                              },
                            }
                          : j,
                      ),
                    )
                    .catch((e) => setError(e.message))
                }
              >
                Cancel model work
              </button>
            </span>
          ) : busy ? (
            "Working…"
          ) : (
            "Ready"
          )}{" "}
          · {model?.elements.length} authored elements
        </span>
        <span>Source {short(model?.source_digest)}</span>
      </footer>
    </>
  );
}
createRoot(document.getElementById("root")!).render(
  location.pathname.startsWith("/expert") ? <App /> : <Studio />,
);
