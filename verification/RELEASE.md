# Agentique v0.1 implementation verification

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

```sh
npm ci
npm run standards:check
cargo build --locked --workspace
npm run build
cargo run --locked -p agq-server -- --workspace .workspaces/local.db
```

Open the printed loopback Console URL including its session fragment. The default
Assistant is clearly labelled TEST MODE. Live configuration uses
AGENTIQUE_ASSISTANT=live, AGENTIQUE_AI_URL, AGENTIQUE_AI_MODEL and AGENTIQUE_AI_KEY.
Use fresh workspace paths to reproduce demonstrations:

```sh
cargo run --locked -p agentique -- validate
cargo run --locked -p agentique -- simulate scenarios/accepted.json
cargo run --locked -p agentique -- --workspace .workspaces/demo.db demo
npm exec playwright -- install chromium
node tools/pilot-setup.mjs
npm run verify
node tools/report.mjs
```

The pilot setup provisions a portable Java 21 on Windows; other hosts need Java 21
or AGENTIQUE_JAVA. The normal application does not require Java. Exact individual
CLI, interchange and test commands are in [README](../README.md).

## Executed verification

Recorded run: 2026-09-16T20:55:30.000Z. Source manifest: `2fb00b79fb4f4ea27a91c8fd9f50bc7b38eca769ab50e0c3f7aaa7f937fa8d96`.
There is no repository commit to cite if results.git_commit is null; the complete
source hash manifest is recorded instead. [results.json](results.json) contains
every command, exit code, elapsed time, environment and output-log path.

- Rust formatting and strict Clippy: passed. Rust unit/integration/negative tests: **48 passed**, none ignored.
- Frontend formatting, TypeScript and production build: passed.
- Engineering checks: four passed. Chromium browser checks: four passed.
- Forced process termination/recovery and headless acceptance demo: passed.
- Official SysML pilot 0.59.0: **0 errors**, 2 warnings across all starter and independent fixture files.
- npm sysml-validate 0.43.1: starter passes; named-payload fixture reports RES001.
  The official pilot resolves that payload. Both results remain recorded; no failing
  command was skipped or relabelled successful. Consequently npm run verify exits 1.

Nonzero commands: `independent-fixtures` (1).
The automated CI configuration is provided; remote CI has not been executed here.

## Responsiveness

Actual host: Microsoft Windows 10 Home, 12 logical CPUs,
16 GiB installed DIMM RAM,
15.92 GiB OS-usable RAM.
The release Engine used SQLite FULL synchronous commits with 1,000 authored elements,
3467 indexed official library declarations, 22885 source bytes,
1039168 scenario bytes and 10,000 committed semantic steps.

| Measurement | Actual result |
|---|---:|
| Total execution | 53.79 s |
| Step mean / p95 / maximum | 5.38 / 10.81 / 18.74 ms |
| Pause requested during a background step | 10.51 ms |
| Stop requested during a background step | 1.16 ms |
| Process peak working set | 90.63 MiB |
| Trace records | 19999 |

Pause/stop are actual measured application-boundary acknowledgements including
contention with a step, not estimated network/UI response times. The host has two
8 GiB DIMMs; reserved hardware memory explains the smaller OS-usable figure.
The release build and debug-test build have different identifiers; both are pinned
to their source/toolchain/profile. Full measurements are in [benchmark.json](benchmark.json).

```powershell
cargo build --release --locked -p agentique --example benchmark
powershell -NoProfile -ExecutionPolicy Bypass -File tools/benchmark.ps1
```

## Twenty release obligations

The machine-readable [traceability register](traceability.json) links each requirement
to its SysML declaration, verification declaration, implementation and named tests.
"Verified for bounded acceptance" describes the recorded tests, not automatic
SysML requirement satisfaction. Empty verification bodies remain not_run.

| Requirement | Acceptance | Assessment | Evidence |
|---|---|---|---|
| AGQ-STD01 | AT-STD01 | incomplete | [evidence](../verification/independent-validation.json) |
| AGQ-STD02 | AT-STD02 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-SELF01 | AT-SELF01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-MOD01 | AT-MOD01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-MOD02 | AT-MOD02 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-MOD03 | AT-MOD03 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-SIM01 | AT-SIM01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-SIM02 | AT-SIM02 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-SIM03 | AT-SIM03 | verified_for_bounded_acceptance | [evidence](../verification/demo.json) |
| AGQ-SIM04 | AT-SIM04 | verified_for_bounded_acceptance | [evidence](../verification/demo.json) |
| AGQ-SIM05 | AT-SIM05 | verified_for_bounded_acceptance | [evidence](../verification/demo.json) |
| AGQ-SIM06 | AT-SIM06 | verified_for_bounded_acceptance | [evidence](../verification/browser-results.json) |
| AGQ-UI01 | AT-UI01 | verified_for_bounded_acceptance | [evidence](../verification/browser-results.json) |
| AGQ-UI02 | AT-UI02 | verified_for_bounded_acceptance | [evidence](../verification/browser-results.json) |
| AGQ-AI01 | AT-AI01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-SEC01 | AT-SEC01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-NFR01 | AT-NFR01 | verified_for_bounded_acceptance | [evidence](../verification/process-recovery.json) |
| AGQ-NFR02 | AT-NFR02 | verified_for_bounded_acceptance | [evidence](../verification/benchmark.json) |
| AGQ-EXT01 | AT-EXT01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |
| AGQ-TRACE01 | AT-TRACE01 | verified_for_bounded_acceptance | [evidence](../verification/results.json) |

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
