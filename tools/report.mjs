import fs from "node:fs";
import path from "node:path";
import { root } from "./extract.mjs";
const read = (p) =>
  JSON.parse(
    fs.readFileSync(path.join(root, p), "utf8").replace(/^\uFEFF/, ""),
  );
const results = read("verification/results.json"),
  register = read("verification/traceability.json"),
  benchmark = read("verification/benchmark.json"),
  pilot = read("verification/pilot-validation.json"),
  demo = read("verification/demo.json");
const rust = fs.readFileSync(
  path.join(root, "verification/logs/rust-tests.txt"),
  "utf8",
);
const tests = [...rust.matchAll(/test result: ok\. (\d+) passed/g)].reduce(
  (n, m) => n + Number(m[1]),
  0,
);
const b = benchmark.measurement;
const rows = register.requirements
  .map(
    (r) =>
      `| ${r.id} | ${r.acceptance_id} | ${r.assessment} | [evidence](../${r.evidence}) |`,
  )
  .join("\n");
const failed = results.records.filter((r) => r.exit_code !== 0);
const text = `# Agentique v0.1 implementation verification

This is a working integrated release candidate. The complete release remains
incomplete because AGQ-STD01's required semantic closure and validation are partial.
Passing bounded acceptance experiments do not establish full language conformance.

## Working end-to-end capabilities

The Rust Engine parses and resolves the self-model, validates supported declarations,
preserves source and identity across reviewed edits, and imports/exports text and
standard project ZIP metadata. CLI, browser Surface and typed Assistant tools use
the shared application boundary. SQLite commits revisions, drafts, scenarios,
approvals, receipts, manifests, checkpoints, traces and audit records atomically.

The accepted and rejected lifecycles run from resolved SysML; exhaustion, ambiguity,
unsupported semantics and limits have distinct outcomes. Manual and continuous
execution produce equal normalized traces on a pinned experiment. Old runs retain
their model revision after rename. Forced termination/restart preserves committed
state and marks unfinished runs interrupted. Completion is separate from a check
verdict. An independent Controller model exercises different names, state ordering,
multiple occurrences and exact named-payload guards.

The browser provides structure, properties, behaviour and paged run history,
scenario editing, all run controls, recoverable source drafts, cancellable model
work and approved Assistant changes. Tests use real Engine responses and Chromium;
screenshots are in [screenshots](screenshots/). Provider failure leaves manual work
available. The live provider adapter is tested with a local HTTP server; a real
credentialed provider was not available.

## Reproduction

From the repository root, with Rust/rustup 1.92.0, Node 22.11+, npm and native build tools:

\`\`\`sh
npm ci
npm run standards:check
cargo build --locked --workspace
npm run build
cargo run --locked -p agq-server -- --workspace .workspaces/local.db
\`\`\`

Open the printed loopback Console URL including its session fragment. The default
Assistant is clearly labelled TEST MODE. Live configuration uses
AGENTIQUE_ASSISTANT=live, AGENTIQUE_AI_URL, AGENTIQUE_AI_MODEL and AGENTIQUE_AI_KEY.
Use fresh workspace paths to reproduce demonstrations:

\`\`\`sh
cargo run --locked -p agentique -- validate
cargo run --locked -p agentique -- simulate scenarios/accepted.json
cargo run --locked -p agentique -- --workspace .workspaces/demo.db demo
npm exec playwright -- install chromium
node tools/pilot-setup.mjs
npm run verify
node tools/report.mjs
\`\`\`

The pilot setup provisions a portable Java 21 on Windows; other hosts need Java 21
or AGENTIQUE_JAVA. The normal application does not require Java. Exact individual
CLI, interchange and test commands are in [README](../README.md).

## Executed verification

Recorded run: ${results.completed_at}. Source manifest: \`${results.source_digest}\`.
There is no repository commit to cite if results.git_commit is null; the complete
source hash manifest is recorded instead. [results.json](results.json) contains
every command, exit code, elapsed time, environment and output-log path.

- Rust formatting and strict Clippy: passed. Rust unit/integration/negative tests: **${tests} passed**, none ignored.
- Frontend formatting, TypeScript and production build: passed.
- Engineering checks: four passed. Chromium browser checks: four passed.
- Forced process termination/recovery and headless acceptance demo: passed.
- Official SysML pilot 0.59.0: **${pilot.errors} errors**, ${pilot.issues.filter((i) => i.severity === "WARNING").length} warnings across all starter and independent fixture files.
- npm sysml-validate 0.43.1: starter passes; named-payload fixture reports RES001.
  The official pilot resolves that payload. Both results remain recorded; no failing
  command was skipped or relabelled successful. Consequently npm run verify exits 1.

Nonzero commands: ${failed.map((r) => `\`${r.id}\` (${r.exit_code})`).join(", ")}.
The automated CI configuration is provided; remote CI has not been executed here.

## Responsiveness

Actual host: ${benchmark.host.os}, ${benchmark.host.logical_processors} logical CPUs,
${(benchmark.host.installed_dimm_bytes / 2 ** 30).toFixed(0)} GiB installed DIMM RAM,
${(benchmark.host.physical_memory_bytes / 2 ** 30).toFixed(2)} GiB OS-usable RAM.
The release Engine used SQLite FULL synchronous commits with 1,000 authored elements,
${b.indexed_library_elements} indexed official library declarations, ${b.source_bytes} source bytes,
${b.scenario_bytes} scenario bytes and 10,000 committed semantic steps.

| Measurement | Actual result |
|---|---:|
| Total execution | ${(b.duration_ms / 1000).toFixed(2)} s |
| Step mean / p95 / maximum | ${b.step_ms.mean.toFixed(2)} / ${b.step_ms.p95.toFixed(2)} / ${b.step_ms.max.toFixed(2)} ms |
| Pause requested during a background step | ${b.contended_pause_ack_ms.toFixed(2)} ms |
| Stop requested during a background step | ${b.contended_stop_ack_ms.toFixed(2)} ms |
| Process peak working set | ${(benchmark.process.peak_working_set_bytes / 2 ** 20).toFixed(2)} MiB |
| Trace records | ${b.trace_records} |

Pause/stop are actual measured application-boundary acknowledgements including
contention with a step, not estimated network/UI response times. The host has two
8 GiB DIMMs; reserved hardware memory explains the smaller OS-usable figure.
The release build and debug-test build have different identifiers; both are pinned
to their source/toolchain/profile. Full measurements are in [benchmark.json](benchmark.json).

\`\`\`powershell
cargo build --release --locked -p agentique --example benchmark
powershell -NoProfile -ExecutionPolicy Bypass -File tools/benchmark.ps1
\`\`\`

## Twenty release obligations

The machine-readable [traceability register](traceability.json) links each requirement
to its SysML declaration, verification declaration, implementation and named tests.
"Verified for bounded acceptance" describes the recorded tests, not automatic
SysML requirement satisfaction. Empty verification bodies remain not_run.

| Requirement | Acceptance | Assessment | Evidence |
|---|---|---|---|
${rows}

## Standards and remaining work

Supplied publications: KerML 1.0 formal/2026-03-01; SysML 2.0 Part 1
formal/2026-03-02; Systems Modeling API and Services 1.0 formal/2026-03-04.
Original PDFs, HTML, ten extracted assets and original library bytes are preserved.
The active library closure selects the official corrective 2026-04 publication at
commit 9baca5908ca28b53da085de69336fde48420ea8f. All 57 source files and eight dependency
edges are checked. [Discrepancies](../docs/standards-discrepancies.md) explains the
original library defects and independent validation before/after that selection.

The [five-axis coverage register](../standards/coverage.json) distinguishes parsing,
representation, validation, interchange and execution. Remaining AGQ-STD01 work is
full required inherited-member/import visibility handling, connector feature-chain
and inherited-end validation, and completion of the supported implicit semantic
relationships. Official library declarations are indexed from verified bytes;
Agentique does not construct/validate their entire semantic graph. These are
implementation gaps, not an unavailable-network excuse, and prevent a complete
v0.1 claim. Source is retained and unsupported reachable execution is refused.

The separate model-service adapter maps nine of 35 published operations; all other
operations and partial element serialization are enumerated in [API coverage](../standards/api-coverage.json).
Text and project interchange are bounded; no JSON/XMI or full interchange/API
conformance is claimed. General concurrency, nested state execution, arbitrary
actions, timers, live effects and multiagent orchestration remain outside AGQ-SEQ-01.
Run/source budgets and queues are bounded, but aggregate workspace RAM has no
global quota. Real provider credentials and a remote CI execution remain unverified.
`;
fs.writeFileSync(path.join(root, "verification/RELEASE.md"), text);
console.log(
  `Wrote verification/RELEASE.md for ${results.source_digest}; manual/continuous comparison: ${demo.manual?.trace_digest === demo.continuous?.trace_digest}`,
);
