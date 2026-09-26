# Bounded cooperative cancellation plan

Read-only implementation plan. No production source changes were made for this plan.

The current Cancel button is a safe publication fence. It cannot stop the synchronous source compiler. `soak01` therefore waited for two complete Rename preparations before discarding them (178.862 and 175.148 seconds for the whole driver steps). A real improvement requires an explicit token through the existing call chain, including producer and effective-audit loops. Another executor or replacement worker would keep the expensive old computation alive and is not proposed.

## Control and error contract

Introduce one cloneable, operation-local `CancellationToken` in `agq-kerml-semantics`, backed by a private `Arc<AtomicBool>`. It supports request/check, never reset. It carries no revision, semantic facts, authority, cache key, serialization or acceptance state. `PublicationClosureOptions` already contains resource controls, derives Clone/Debug, and is not serialized; add an optional token there, defaulting to absent. Existing callers retain their exact scheduling behavior and public scheduler signatures. Do not incorporate the flag into graph, context or certificate digests.

Add distinct `PublicationOverlayError::Cancelled`, and a source operational cancellation variant rather than repurposing `ContextError`, incomplete closure, unsupported syntax, or an interpretation string. The source/workspace checkpoint wrappers must preserve this distinction. In particular, `modeling-service/src/source_identity.rs::reconstruct` currently stringifies every checkpoint failure: it must map typed cancellation to `ServiceError::Cancelled` before retaining the existing handling of genuine reconstruction failures. Agent/platform forwarding must preserve the typed outcome, and Bridge must map it to a dedicated preparation-cancelled reply before stringifying other errors.

An operation returns either its ordinary complete semantic result or cancellation. It must never return a partial `SourceCompilation`, usable closure certificate, accepted audit, candidate DTO, validation capability, or durable receipt on cancellation. Unpublished child state is dropped. The parent and accepted publication remain immutable. Shared immutable read memoization is allowed to remain populated; it is not a modified model checkpoint.

`agq-kerml-text::SourceCompilationControl` can contain the token and one bounded latest-stage observation, without a growing progress queue. Its stage is observational only. A control check reports an actually entered stage and tests the cancellation request; no percentage is inferred. UI cancellation-requested remains distinct from worker-acknowledged Cancelled. A fresh control is allocated for every exact request/binding/runtime epoch, so an old Cancel button cannot cancel a later candidate.

## Exact path and bounded file scope

| Layer | Existing boundary | Proposed scoped change |
| --- | --- | --- |
| Native | `app.rs::PendingPreparation`; `actions.rs::prepare_part`; `part_edit.rs::prepare_rename`; `panels.rs` Cancel button | Store the operation's control handle with its request. Button sets both the existing stale-publication fence and token. Show only last known stage and cancellation request. Keep Current/read lane active. |
| Bridge | `bridge.rs::propose` / `nested_part`, serialized `Work` closure | Add a controlled edit entry point for Create nested Part/Rename, passing the same handle into the existing worker. Check before work begins. Add dedicated terminal cancellation reply; do not replace the worker. |
| Platform | `studio-platform/src/candidates.rs::StudioPlatform::propose` | Controlled sibling method; old method uses no cancellation. Check before agent work, before retaining a candidate, and after producing its projection/diff. If cancellation is observed after insertion, remove only that uncommitted candidate and return Cancelled. No commit call is involved. |
| Agent | `modeling-agent/src/commands.rs::propose` | Thread control through the two visual Part edit branches. Existing source-import and provider behavior stays on the existing API; no new claim of cancellable import. |
| Service | `modeling-service/src/part_insertion.rs`, `part_rename.rs`, `source_identity.rs`, `lib.rs` error type | Controlled siblings reuse existing checks and proofs. Check around source proof/reconstruction, continuity/effective-rename checks, and preparation of the in-memory durable candidate. Preserve typed cancellation. Never call repository commit. |
| Workspace | `modeling-workspace/src/checkpoint.rs::ProjectRevisionCheckpoint::restore_sharing_dependency` | Controlled sibling threads the handle into source restoration, preserving exact predecessor/source/identity checks. Existing checkpoint API delegates without cancellation. |
| Source checkpoint | `kerml-text/src/source_checkpoint.rs::restore_maybe_cached` | Pass optional control; check before/after each source document parse and identity restoration, then before compilation. The Part paths use ordinary source restoration sharing the authenticated predecessor. |
| Source compilation | `kerml-text/src/source_inputs.rs::compile_with_history`; `lib.rs` exports; a small control module | Check between identity setup, preparation, strict declared validation, closure, effective audit and result publication. Existing ordinary compile/full-rebuild methods remain uncancelled by default. |
| Preparation and closure | `kerml-text/src/sysml/source.rs::prepare_accepted_source`, `finish_accepted_source` | Check before/after each declared construction/refinement entry, checkpoint/rebind, strict closure and final-reference pass. Supply token in all three authored scheduler options: construction, strict snapshot and restored overlay. Do not abuse the semantic context factory as a cancellation signal. |
| Producer loop | `kerml-semantics/src/producer_worklist.rs::close_frontiers`; `publication_overlay.rs` error; crate export/control module | Check at entry; before/after context construction and initial proof work; each round; each subject batch (and between subjects in worklist mode if cheap); before/after overlay application; before final certificate construction and return. Return only typed Cancelled, never `IncompleteProducers` with a partial success. |
| Effective audit | `kerml-text/src/source_effective_audit.rs::run` | Check before context/reuse setup, before every 32-subject batch, and before installing the completed audit. Cancelled local accumulators are dropped, including transportable audit receipts. |
| Native reply | `updates.rs` matching preparation branch | Acknowledged cancellation clears the matching task and releases the mutation gate without publishing a candidate. Keep the existing late-success discard/cancel path for the final race after the worker's last check. Other semantic failures remain failures. |

This is roughly 22 existing implementation files (including crate exports and the source-library error type) plus two small control modules and focused tests. A deeper reference-pass checkpoint would add `library/refinement.rs`. It spans the necessary layers but adds no executor, persistence format, general job framework or language capability. A coarse check only in Bridge/Platform would not interrupt the dominant work and should not be advertised as the requested fix.

The generic scheduler's `frontier_checkpoints` capability is unchanged. Authored candidate calls use `None`; cancellation of the proposed visual-edit path has no frontier-journal or repository write. Existing canonical publication builders continue with the absent token and unchanged acceptance obligations.

## Known stages and interruption bounds

- **Queued:** recorded when the actual controlled work is accepted by Bridge.
- **Parsing:** source checkpoint enters document parsing/identity restoration.
- **Declared model:** declared construction or strict declared validation actually starts.
- **Resolving:** a reference refinement/reference query pass actually starts.
- **Semantic closure:** construction/final producer scheduler or certificate work actually starts.
- **Effective validation:** the strict effective-query audit actually starts. This does not mean the operator has run the separate Validate command or gained a Validated capability.
- **Ready / Failed / Cancelled:** only the corresponding terminal worker result establishes these.

Stages can repeat during real refinement. Do not enforce a fictitious monotonic progress sequence. Cancellation-requested retains the last observed stage until acknowledgement.

Individual parser calls, declared kernel validation/materialization, context/digest construction, proof rebind and one effective query are currently indivisible. Check immediately around them; do not claim a hard subsecond cancellation bound. The next safe point is after the longest currently executing primitive, not after all remaining rounds/audit batches. Existing timings identify those primitives; the first real cancelled run must retain request-to-ack time and last observed stage to expose any remaining long boundary. If a whole refinement reference pass dominates, extend `library/refinement.rs` with the same explicit control at its 32-reference query-reset boundary; do not merely throw an error from its authentication query callback.

## Required tests and actual acceptance

1. **Token isolation and typed outcome:** pre-cancelled edit starts no source construction; cancelling one handle does not affect another; ordinary semantic/authentication failures remain distinguishable. No reset API exists.
2. **Real producer frontier:** use existing deterministic scheduler fixtures. Request cancellation from the existing batch-progress callback after the first batch. Assert cancellation before another batch/round, unchanged parent graph, and absence of any returned closure/certificate. Run the same fixture without cancellation and compare its full closure result with the current oracle.
3. **Audit interruption:** deterministic checkpoint observation requests cancellation at an actual later audit batch; no complete audit or reuse receipt escapes. Use a latch/checkpoint hook instead of timing-dependent sleeps. Uncancelled audit values/completeness/proof remain exact.
4. **Service boundaries:** cancelled CreatePart and Rename return no `PreparedChanges`, insert no retained candidate and do not change source bytes, identity reservations, branch head or repository contents. Late cancel after construction but before projection publication drops exactly the candidate allocated by that operation.
5. **Native races:** cancel before queued start, during compilation, after controlled completion but before reply processing, and after navigation/read requests. Each task is bound to its exact request/epoch/revision. Current selection, latest requested lens and ongoing immutable Inspector/Graph read remain valid. The next candidate becomes admissible after the terminal acknowledgement without waiting for a stale worker.
6. **Real runtime gate:** on the Agentique model, start nested Part, observe a real running stage and current-view read, cancel through the actual button, record click/request/worker-ack timestamps, and then prepare another candidate through the ordinary UI. Repeat for Rename. Require unchanged durable head and no candidate published from the cancelled task. Complete the new candidate normally and compare with cold reconstruction. Restart checks remain separate.

This change touches implementation files covered by publication freshness. Their digests may be updated only after the required non-cancelled semantic oracle verifies exact equivalence; cancellation cannot be used to avoid closure/audit or to reissue acceptance from a flag. The existing failed soak receipt is retained, and a fresh real soak is required after integration.
