# Generation 1 / generation 2 architecture disposition

## Current integration note

At evidence snapshot `108de93`, accepted KerML Operational v9 and Systems
Operational v3 are shared dependencies of the integrated `agq-modeling-workspace`.
The [language-readiness gate](../verification/summaries/final-language-acceptance/semantic-closure-readiness.md)
passed and ADR 0026 is adopted. `SourceInputs`/`SourceCompilation` provide the
additive recovery/construction boundary while the existing strict `SourceProject`
contract remains intact. Workspace revisions retain their exact declared/derived
graph, references, closure evidence and effective audit; validation requires a
finding-free report matching that revision's semantic context.

Immutable kernel base tables retain standard graph, index, proof/search and
reservation allocations. Local copy-on-write and authored reconstruction remain
permitted. The [current integration evidence](../verification/summaries/final-audit-semantic-closure/README.md)
records 139 passing kernel tests, exact issued-cache identity restoration, five
passing Working-state tests and the passing real workspace self-model gate.
The latter validates two revisions after an edit and checks old answers,
authored identities and physical standard sharing. Four edit/recovery lifecycle
tests and the five-Validated-revision scale test also passed. The latter retains
100 mixed documents per revision and checks four parallel readers and shared
standard tables. The [runtime record](../verification/summaries/final-audit-semantic-closure/workspace-runtime-acceptance.md)
therefore records 11 of 12 unique passes; the separate recovery-scale test and
full workspace acceptance remain pending. The current 20-package dependency
audit remains clean. ADR 0024 remains proposed.

No Gen1 engine or release obligation is replaced by these results. The workspace
provides in-memory atomic visibility, not durable persistence, application/server
migration, full language conformance or execution. KerML acceptance is unchanged;
ADR 0027 remains proposed research. The original source audit below is preserved
at its explicit baseline; its future-tense integration descriptions are historical.

## Historical source audit

This audit inspected the source implementations and manifests at main
`0fbdf3ee19788a260b99d89e71dbd28f432d8243`. The functioning Gen1 product stays in
service. Generation 2 becomes an additive modeling foundation after the explicit
language readiness gate; package names alone do not determine reuse.

| Existing component | Actual responsibilities inspected | Disposition and migration boundary |
| --- | --- | --- |
| [`crates/model`](../crates/model/src/lib.rs) | Mutable `Model` maps combine source, semantic elements, relationships, precomputed owner/name fields and diagnostics. `accepted()` checks error severity. String IDs and JSON digests are private Gen1 contracts. [`work.rs`](../crates/model/src/work.rs) separately supplies progress and cancellation/commit serialization. | **Replace with Gen2** for semantic storage; **Migrate concepts** for work control. Kernel records, provenance and evidence replace `Model`; never translate `accepted()` into producer closure. Extract a neutral work-control contract later if needed, without importing `agq-model` for its convenience utilities. |
| [`crates/syntax`](../crates/syntax/src/lib.rs) | Lexer, handwritten declarations and Pratt expressions directly populate the Gen1 `Model`; identity input uses source/path mappings. Source spans support reviewed editing. | **Replace with Gen2**; **Retire eventually** only after application migration. Reuse malformed-source and edit scenarios as behavioral tests, using `agq-kerml-syntax`'s lossless syntax/reconciliation and Gen2 lowering. No parser-output adapter becomes canonical truth. |
| [`crates/semantics`](../crates/semantics/src/lib.rs) | Scoped resolution, validation and identity maps; once-parsed compact library index is cloned into authored models. [`expansion.rs`](../crates/semantics/src/expansion.rs) adds explicit implied nodes and relationships in place. | **Replace with Gen2**. Preserve supported product behavior through migration fixtures, while replacing compact library copies and in-place expansion with accepted shared publications, composed queries and derived overlays. Gen1 library indexing does not confer Gen2 publication acceptance. |
| [`crates/workspace`](../crates/workspace/src/lib.rs) | `Revision` owns a Gen1 `Model`; candidate edits rename/move/retype/connect by source spans, reconcile identities, reject stale bases and commit accepted candidates. Text/KPAR import/export includes private identity metadata and validates archive contents. | **Migrate concepts** into additive `agq-modeling-workspace`; **Adapt behind compatibility layer** for future interchange. Keep Gen1 revision/edit types intact. Reuse atomic candidate and stale-base behavior, not its model carrier, archive identity schema or whole-model clones. Phase 1 only needs document changes. |
| [`crates/simulation`](../crates/simulation/src/lib.rs) | `prepare` validates bounded AGQ-SEQ-01 execution against Gen1 types, creates immutable `Plan`, and `Run` handles atomic sequential steps, traces, limits and check verdicts. Unsupported behavior is explicitly rejected. | **Migrate concepts**, later **Replace with Gen2** compiler/IR/runtime. Preserve determinism, rejection, checkpoint and run/verdict distinctions as acceptance scenarios. A future compiler consumes validated Gen2 snapshots; runtime consumes versioned execution IR. Current Plan is not a general Gen2 IR. No execution work in this milestone. |
| [`crates/application`](../crates/application/src/lib.rs) | Actor-authenticated commands, proposals, reviews, single-use approvals, expected revisions, idempotency receipts, scenarios and run control. Store transactions persist writes/event/receipt before head/run updates. Public commands and store values are coupled to Gen1 Revision, Model and Run. | **Migrate concepts** and later **Adapt behind compatibility layer**. Keep authority, transaction and receipt boundaries above both semantic engines; introduce explicit Gen2 application contracts when migration is authorized. Do not make the new workspace depend on `Application` or `Store`. |
| [`adapters/storage`](../adapters/storage/src/lib.rs) | SQLite process lock, schema-version refusal, WAL/FULL synchronous mode, checksums, immediate transactions and atomic objects/events/receipts. Implements Gen1 application `Store` with JSON records; restoration refers to Gen1 revision/run types. | **Reuse infrastructure** through future **Adapt behind compatibility layer**. Reuse reliability practices and failure tests, not a claim that existing JSON records serialize Gen2 graphs or acceptance receipts. Define versioned Gen2 repository contracts and migration before adding new records. No persistence now. |
| [`adapters/assistant`](../adapters/assistant/src/lib.rs) | Provider transport chooses one typed intent with contextual revision/selection; execution delegates to application authority. Tool schemas contain Gen1 action, value and run types. | **Unrelated / keep** for this milestone; future **Adapt behind compatibility layer**. HTTP provider infrastructure may remain; tools must target explicit future application contracts. Never put transport, approval or semantic inference in the language crates. |
| [`crates/server`](../crates/server/src/main.rs) | Local authenticated HTTP adapter, bounded asynchronous command jobs and cancellation; [`model_api.rs`](../crates/server/src/model_api.rs) maps Gen1 project/branch/commit/element reads and paging. It calls application/store/model types directly. | **Reuse infrastructure**, later **Adapt behind compatibility layer**. Keep session security, bounded jobs, durable-receipt recovery and paging tests. Replace projections behind new application/query contracts; do not expose a Working revision as a validated commit. Current API coverage remains Gen1-only. |
| [`crates/cli`](../crates/cli/src/main.rs) (`agentique`) | Filesystem collection and resource checks, validation, source interchange, application commands and standalone AGQ-SEQ-01 simulation. Validation calls Gen1 semantics directly. | **Unrelated / keep**, later **Adapt behind compatibility layer**. Retain operational commands and add explicitly named Gen2 modes only when authorized. Transport-independent validation/simulation behavior is useful; do not silently change the engine behind existing CLI promises. |

The [Gen2 `SourceProject`](../crates/kerml-text/src/project.rs) is the useful
existing source-history building block. It already separates project/document,
source revision and semantic identity, retains immutable revisions, reconciles
syntax edits, checks stale bases, and shares an accepted KerML publication.
Its current `with_sysml_standard_libraries` accepts **KerML only**; its name is
not evidence of accepted Systems integration. Production mixed parsing currently
rejects recovered documents in `apply`. A Working workspace revision therefore
needs an explicit recovery/construction carrier; wrapping this API unchanged
would not satisfy the Working/Validated contract.

## Dependency boundary

The current manifests keep kernel/language packages free of the entire Gen1
stack. KerML semantics' development-only dependency on `agq-sysml` is for tests
and does not invert production language layering. Syntax and standard-library
test dependencies also form a development cycle, so a guard must terminate on
cycles rather than assume a DAG.

[`generation_boundary.py`](../verification/scripts/generation_boundary.py)
checks all declared workspace normal/build/development, optional and target
dependency edges from Cargo metadata, including package renames. It detects
transitive paths from `agq-kernel`, `agq-kerml*`, `agq-sysml*`, standard-library
infrastructure and a future `agq-modeling-workspace` into any Gen1 package. It
separately rejects a kernel path to language or platform crates. This is an
implementation dependency check outside the language engine; it does not inspect
Cargo files from a semantic query or replace self-model architecture tests.

## Migration map and sequence

| Gen1 capability | Gen2 replacement | Migration/adaptor strategy |
| --- | --- | --- |
| Model storage in `Model`/JSON | Kernel Snapshot + derived overlay + provenance + exact context/certificate | Reparse retained source with explicit identity reconciliation where possible. Keep a versioned old-to-new ID map outside canonical language meaning. Never deserialize Gen1 fields as canonical Gen2 facts. |
| Source workspace and revision history | Additive `agq-modeling-workspace` over accepted publications and the Gen2 frontend | Preserve stale-base and atomic revision behavior. Keep old application workspaces on Gen1. Phase 1 is in-memory add/edit/remove, shared standards, Working/Validated and borrowed query access. |
| Reviewed semantic edits | Future Gen2 application edit commands over document changes | Port each command with resolved source evidence and identity tests. Do not automatically carry assumptions encoded in Gen1 path-based identity maps. |
| SQLite repository | Future repository contract for Gen2 revision manifests and exact publication identities | Preserve locks, durable atomic commit, schema versioning and failure recovery. Convert/export via explicit migration tooling; schema v1 and private KPAR sidecars are not the new persistence format. |
| Server/model API | Future application/query adapters projecting Gen2 revisions | Introduce explicit version/generation selection and immutable revision handles. Preserve paging/security/job behavior; revalidate standard API claims independently. |
| Simulation | Future validated-snapshot compiler -> execution IR -> runtime | Port AGQ-SEQ-01 acceptance scenarios after execution semantics/IR are versioned. Keep unsupported semantics explicit and separate completion from verification verdicts. |

First finish the accepted Systems and authored/self-model gates, then adopt the
stability contract and implement only the in-memory workspace. Keep Gen1 release
checks operational throughout. Subsequent application/repository/transport and
execution migrations require separate milestones; none is implied by this
disposition. No legacy crate is retired in this bridge.
