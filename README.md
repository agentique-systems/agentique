# Agentique 0.1

A Rust modelling and simulation Engine with a persistent local workspace, browser
Console and one tool-equipped Assistant. This repository contains a working
integrated **release candidate**, not a completed standards-verified v0.1 release.
The [implementation report](verification/RELEASE.md), [requirement register](verification/traceability.json) and
[five-axis coverage](standards/coverage.json) identify remaining obligations.

The original HTML specification, three OMG PDFs, extracted self-model,
requirements, scenarios and proposed contracts are preserved. The official KerML
and SysML libraries are vendored with hashes and verified dependency metadata.
See [baseline discrepancies](docs/standards-discrepancies.md) and
[architecture](docs/architecture.md).

## Setup and launch

The launch instructions and release claims below describe **generation 1**:
`agq-model`, `agq-syntax`, `agq-semantics` and the integrated workspace, simulation,
application, server and Console. They have not been migrated to generation 2.

**Generation 2** is the standards-driven language engine under active development:
`agq-kernel`, `agq-kerml`, `agq-kerml-semantics`, `agq-kerml-syntax`,
`agq-kerml-text` and the normative metamodel generator. New language work targets
this architecture, including the planned SysML layer. Its bounded capabilities
and gaps are tracked separately in [generation-2 status](standards/v2-coverage.json).
See the [dependency diagram](docs/architecture.md) and
[semantic kernel guide](docs/semantic-kernel.md). Descriptor support, semantic
queries, text support and execution are separate claims.

Prerequisites: Rust/rustup with the pinned 1.92.0 toolchain, Node 22.11 or newer,
npm, and a native C/C++ build toolchain for Rust and bundled SQLite. On Windows,
install the Visual Studio C++ Build Tools. From the repository root:

```sh
npm ci
npm run standards:check
cargo build --locked --workspace
npm run build
cargo run --locked -p agq-server -- --workspace .workspaces/local.db
```

Open the Console URL printed by the server, including its session fragment. It
binds to `127.0.0.1:7331`; `--port` selects another port. By default the Assistant
is visibly in deterministic **TEST MODE**. The Engine and Surface are fully live.
The local database accepts one Engine process at a time. Stop the server before
using the CLI against that same workspace, or use a different `--workspace` path.

For a live chat-completions-compatible provider, set environment variables before
launching: `AGENTIQUE_ASSISTANT=live`, `AGENTIQUE_AI_URL` (complete endpoint URL),
`AGENTIQUE_AI_MODEL`, and `AGENTIQUE_AI_KEY`. Optionally set a persistent
`AGENTIQUE_SESSION_TOKEN` of at least 16 characters. Credentials never belong in
source or scenario files. A provider failure does not disable manual operations.
Real provider credentials were not supplied for acceptance; the transport is
tested against a local HTTP fixture.

## Demonstration

The Console selects the self-model's request lifecycle. Prepare the accepted
scenario, Initialise, and Step twice: `idle → checking → accepted`. Click each
timeline source to inspect its element in the exact pinned revision. The rejected
preset ends in `rejected`; exhausted ends with `input_exhausted`. Run and manual
Step produce the same normalized semantic trace. Completion and check verdicts
are displayed separately.

Select a named element and send `inspect` in the Conversation. In test mode,
`rename NewName` proposes a real source change. Review the diff and approve it.
Both interfaces show the new revision and stable selection; old runs stay pinned.
`read run` inspects the selected run. `action <JSON>` requests approval for an
Engine Action such as `{"op":"control_run","run_id":"…",
"expected_control_version":1,"operation":"step"}`. No Assistant mutation or
execution is permitted without its matching, unexpired, single-use approval.

The Surface also offers manual rename, type, scalar value, move and connection
proposals, plus source drafts with explicit identity reconciliation. Invalid
drafts survive restart and do not replace the accepted model. The behaviour table
and property inspector provide keyboard-accessible alternatives to the graph.

Headless examples (use a fresh demo path when reproducing recorded results):

```sh
cargo run --locked -p agentique -- validate
cargo run --locked -p agentique -- simulate scenarios/accepted.json
cargo run --locked -p agentique -- --workspace .workspaces/demo.db demo
cargo run --locked -p agentique -- --workspace .workspaces/demo.db inspect --element AgentiqueBehaviour::RequestLifecycle
cargo run --locked -p agentique -- --workspace .workspaces/demo.db prepare scenarios/rejected.json
cargo run --locked -p agentique -- --workspace .workspaces/demo.db control RUN_ID initialise --version 0
cargo run --locked -p agentique -- --workspace .workspaces/demo.db control RUN_ID run --version 1
cargo run --locked -p agentique -- --workspace .workspaces/demo.db trace RUN_ID
cargo run --locked -p agentique -- --workspace .workspaces/demo.db rename AgentiqueBehaviour::RequestLifecycle::accepted validated
cargo run --locked -p agentique -- --workspace .workspaces/demo.db export Agentique.kpar
cargo run --locked -p agentique -- --workspace .workspaces/imported.db import Agentique.kpar
cargo run --locked -p agentique -- --workspace .workspaces/demo.db export text-project --text
cargo run --locked -p agentique -- --workspace .workspaces/text-import.db import text-project
```

`command FILE.json` exposes the shared typed operations; `--assistant` exercises
Assistant authority. `move`, revision-qualified `inspect`, `status`, `export`,
`prepare` and all run controls are available through `--help`. Commands emit JSON
and failures return a nonzero exit code. `simulate` and the core crate tests require
no HTTP server, database, browser or AI provider.

## Verification

```sh
npm exec playwright -- install chromium
node tools/pilot-setup.mjs
npm run verify
```

The verification runner executes formatting, strict Clippy, all Rust tests, the
build, frontend formatting/type/build checks, engineering tests, independent
SysML validation, Chromium end-to-end tests, forced process recovery and the
headless demo. It records exact commands, outputs, exit codes, durations, environment
and source hashes in [verification/results.json](verification/results.json).
It intentionally exits nonzero for the recorded npm validator diagnostic;
CI does not conceal that result. Screenshots are in
[verification/screenshots](verification/screenshots).

Individual checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run format:check
npm run check
npm run build
npm test
npm run test:e2e
npm run test:process
node tools/extract.mjs --check
node tools/pilot-validate.mjs
node node_modules/sysml-validate/out/main.js models/ --library standards/libraries-2026-04 --no-config --format json --out verification/independent-validation.json
```

Windows responsiveness measurement, with actual process memory sampling:

```powershell
cargo build --release --locked -p agentique --example benchmark
powershell -NoProfile -ExecutionPolicy Bypass -File tools/benchmark.ps1
```

The workload contains 1,000 authored elements plus pinned libraries and a run
bounded to 10,000 semantic steps. Results, host specification and pause/stop
measurements are in [verification/benchmark.json](verification/benchmark.json).

## Current release boundaries

The integrated self-model workflow, isolated execution, durable operations,
approval boundary, real browser views and headless tools work. The complete v0.1
release remains **incomplete**: some required implicit/inherited semantic closure
and connector feature-chain validation remain partial. The official pilot validates
the starter and independent fixture with zero errors against the selected official
April 2026 library corrections; the separate npm validator retains a recorded
named-payload limitation. Source jobs support progress/cancellation and bounded
work queues; the measured run limits and pause/stop targets pass. Aggregate workspace
memory has no global quota. The model-service API is a documented
read subset, not a conformance claim. See every numbered obligation and its
evidence in the [release register](verification/traceability.json).

General KerML/SysML execution, parallel/hierarchical regions, live simulation
effects and multiagent orchestration are outside AGQ-SEQ-01 and are explicitly
refused. The test Assistant is never presented as a live AI provider.
