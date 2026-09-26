# Cooperative cancellation review draft

Based on `42e7e187de823a3b855d530dcee84dbe8c06b59f` in isolated worktree `agentique-cancellation`. The source changes are not yet compiled or accepted. Main checkout, running acceptance binaries, historical reports and publication freshness pins are unchanged.

The enabled visual Part operation owns one non-resettable atomic cancellation token and one bounded last-entered stage. Its handle is explicit across Bridge, StudioPlatform, ModelingAgent, ModelingService, workspace/source checkpoints, source construction, producer scheduler and effective audit. Existing public entry points pass a disabled default control. Cancellation does not participate in semantic identity, cache keys, canonical state, certificates or validation authority.

A request is distinct from acknowledgement. The native Cancel button requests a safe stop; the serial worker returns typed `PreparationCancelled` only after discarding unpublished work. This terminal reply releases the existing mutation lane. A late completed candidate still follows the prior cancellation/discard fence. No replacement worker, global flag, thread-local state or detached computation was added. Source imports retain their earlier finish/discard behavior and display that limitation separately.

Checkpoint checks surround parse/identity restoration, declared construction and strict validation, reference passes, closure contexts/materialization, each producer batch/subject, each effective-audit batch/subject, review construction and final candidate retention. A cancelled source operation returns no SourceCompilation, completed effective audit, candidate DTO or durable receipt. Producer options in this authored path have no frontier journal. Real semantic/authentication failures retain their previous treatment; an error message containing cancellation text cannot become typed cancellation.

The largest remaining indivisible operations are a single parser call, declared kernel/index work, semantic context/digest or certificate setup, one query, and review/diff projection. Their actual cancellation latency must be measured; no subsecond cancellation guarantee is asserted.

## Regression coverage added (not executed yet)

- Token clone sees its own request; a fresh operation remains unaffected; disabled control records no stage and preserves ordinary behavior.
- A pre-cancelled real producer fixture never constructs a query context. Cancellation after its first batch returns a typed error before a second batch, leaves the parent records/occurrences unchanged, and a new token produces the exact ordinary closure, query values, completeness, evidence and certificate.
- Direct source cancellation and nested producer cancellation survive SourceCheckpointError -> workspace CheckpointError -> ServiceError translation. Malformed checkpoints and cancellation-looking ordinary error strings remain failures.
- Agent/platform wrappers retain the typed outcome. Native terminal acknowledgement preserves revision, scene, projection, camera and selection, releases the mutation gate, and admits another task. Existing late successful completion/cancel tests remain intact.
- The current soak gate requires an exact request/epoch/revision acknowledgement, known entered stage, measured request-to-ack time and an empty candidate/mutation lane. It then observes a distinct enabled candidate preparation through the ordinary Rename dialog. Tests reject stale/foreign/missing receipts, fabricated stages and inconsistent timing. Historical reports are not rewritten.

There is no new deterministic test that interrupts an accepted effective audit midway through a later batch yet. The checkpoints are present, but this remains a coverage limit until an explicit real audit-stage cancellation test is run. The producer fixture test does exercise interruption inside semantic work, without replacing its algorithm or weakening its full-result oracle.

## Enabled, uncancelled exact oracle

The portable `create_part_performance.rs` harness now accepts an existing-signature command function and its exact child test entry name. Its graph/identity/occurrence/proof/query/diagnostic observations and all negative checkpoint cases are unchanged. The baseline still invokes ordinary `propose`; no new API symbol is required by the single overlaid baseline file.

Current-only test `create_part_controlled` passes `propose_controlled` with `CompilationControl::new()` into that same independent-process oracle. It verifies the enabled control was never requested, the actual final stage is PreparingReview, and records `command_control_mode = enabled-not-requested`. Its cold child uses the ordinary disabled source restoration. Both children are explicitly filtered to the same current-only test entry. Example coordinated command:

```
cargo test --release --locked --offline -p agq-modeling-agent --features agq-modeling-agent/verification --test create_part_controlled enabled_control_create_part_matches_full_self_model_reconstruction -- --exact --ignored --nocapture --test-threads=1
```

Required next evidence: remote compile/Clippy/unit tests, this enabled-control full oracle over the installed accepted publications, and a fresh real Cancel -> acknowledgement -> next candidate soak. None is claimed from formatting or code inspection. Freshness updates remain forbidden until exact oracle qualification.
