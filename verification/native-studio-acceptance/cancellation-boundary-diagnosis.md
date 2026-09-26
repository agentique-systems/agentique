# Candidate cancellation boundary

The actual `soak01` run confirms safe discard, not prompt interruption of semantic construction. The first two `soak cancels preparation while exploring current graph` assertions took 178,862 ms and 175,148 ms. Those are complete driver steps, including opening/preparing the Rename, background navigation, clicking Cancel, waiting for completion and settling; they are not measured Cancel-click-to-acknowledgement intervals. Both completed without moving durable head. The driver observed a Graph response during each preparation.

The request travels through these production boundaries:

1. `studio-native/src/panels.rs` sets `PendingPreparation.cancelled = true` from the actual Cancel button. It displays **CANCELLATION REQUESTED** and “Current reconstruction will finish safely; its result will be discarded.” It does not claim the compiler stopped.
2. `studio-native/src/bridge.rs` is already executing an indivisible `Work` closure with exclusive `&mut StudioPlatform`. Its request/reply epoch fences stale presentation updates; the epoch is not a cancellation token.
3. `studio-platform/src/candidates.rs::StudioPlatform::propose` calls `modeling-agent/src/commands.rs::propose`, which invokes `ModelingService::prepare_part_rename` or `prepare_part_insertion`. No operational cancellation capability is passed.
4. `modeling-service/src/source_identity.rs::reconstruct` calls the identity checkpoint's `restore_sharing_dependency`, which completes source reconstruction, closure and effective audit synchronously. Service identity/equivalence checks and discardable candidate-cache preparation follow. No durable head update occurs here.
5. Only after the closure returns does `studio-native/src/updates.rs` observe the cancelled flag, suppress candidate display, retain its cancellation handle, and enqueue ordinary `StudioPlatform::cancel`. The retained uncommitted candidate is removed on that serial acknowledgement. The immutable read lane remains usable throughout.

There is no existing cooperative source-compilation cancellation token. The semantic team independently confirmed that scheduler progress callbacks return `()` and cannot stop work. Authentication/context construction callbacks return `Result`, but using an authentication failure to signal operator cancellation would conflate different contracts and is not safe design.

A Studio-only replacement worker would leave the expensive old operation running and permit overlapping reconstruction/memory retention. It would not provide real cancellation and would undermine the single mutation owner. No such workaround was introduced.

The bounded source fix from this investigation preserves an actual construction failure after a cancellation request: the terminal status now shows the failure and says Current was retained, instead of incorrectly saying a prepared candidate is being discarded when no candidate exists. A regression verifies the unchanged projection, scene revision, binding, camera and selection, absence of a phantom candidate, and released mutation state. The existing successful-cancellation regression still requires stale candidate completion never to replace the Current scene and retains cancellation authority until acknowledgement.

The next correct implementation is a distinct operational cancellation contract carried from the service into source compilation, with checks before/after parse, lowering and refinement; between producer scheduler batches/rounds; and before each effective-audit batch. Cancellation must return a distinct cancelled outcome and discard only unpublished child state. A cancelled partial closure/audit cannot become a complete query, authenticated cache, candidate or durable revision. Full context/digest primitives remain uncancellable until their own bounded substeps are exposed. Progress UI must report only stages actually emitted by this path.

## Validation and retained limitations

`rustfmt --edition 2024 crates/studio-native/src/updates.rs` and `git diff --check` exited 0. No runtime or compiler was launched alongside the integration lead's soak. The new terminal-error regression awaits the coordinated integrated native test run. This document does not claim faster cancellation, completed soak acceptance or native device-loss recovery.

Separately, `gpu_timing.rs`, `gpu.rs` and the metrics serialization in `updates.rs` now distinguish zero-duration timestamps, backwards pairs, mapping failures, polling failures and surface invalidations for future captures. Valid zero durations contribute 0 to the timing distribution. Historical aggregate counts, including real-run02's 17, remain unattributed. The added regression verifies a zero and positive interval both contribute while a backwards pair is rejected and classified. Formatting and diff checks passed; integrated tests remain pending.
