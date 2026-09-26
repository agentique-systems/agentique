# Independent real journey review before first light

Reviewed root `7a0b7e8`, specifically `real_automation.rs`, `real_targets.rs`, the
ordinary widget recorders, native action handlers, platform bootstrap, source
seed, and projection contracts. This is source review before a real process
run; no screenshot or runtime journey pass is claimed.

## Ranked findings

1. **P1, corrected in `0e12066`: frozen preparation could pass responsiveness.**
   Original `real_automation.rs:1195-1200` required concurrent panning only after
   ten observed background frames. A UI blocked during the whole operation
   produces few frames and evades that test. Timing now starts before the actual
   Prepare release event and records every subsequent native input-hook gap
   through delivery of the semantic candidate response. Any gap over 250 ms
   fails this coarse non-freeze gate; preparation lasting at least 1000 ms also
   requires observed camera movement while the mutation remains pending.
   Frame count cannot waive either condition. A quick operation explicitly
   reports when concurrent pan was not observed. This is not a 60 Hz, OS-input,
   GPU-presentation, or physical-photon measurement. The negative test models a
   five-second operation completing between two hooks and requires rejection.

2. **P2, corrected in `0e12066`: candidate identity did not prove intended owner.**
   Original candidate/selected/committed checks established name, semantic kind,
   source substring and stable identity, but could accept creation under another
   owner. The journey now retains the selected ModelingPlatform identity when
   the Create dialog opens. It requires a source-backed authored PartUsage with
   that exact owner, absent from the same baseline lens, and verifies owner plus
   canonical identity after commit and second-process restart. A same-ID/name
   counterexample with a different owner must fail.

3. **P2, remaining qualification gap: the dependency assertion is weaker than
   its operator intent.** Original `real_automation.rs:1071-1078` checks a focused
   Graph projection, nonempty edges and the agent panel flag. It does not require
   `agent_activity.root` to equal the ModelRepository just inspected, nor a
   complete/error-free activity and exact overlay result population. Ordinary
   code currently supplies those fields, so this is an assertion gap rather than
   a demonstrated wrong result. Acceptance should inspect those actual fields
   and the real screenshot before claiming the agent selected the correct scope.

4. **P2, remaining qualification gap: world name alone is insufficient for
   Graph.** Original `real_automation.rs:1053-1069` explicitly checks the
   Requirements projection kind, but Graph only checks the World enum and a
   nonempty scene. Revision coherence in `assert_bound` does not establish that
   a successfully queried projection matches the requested world/definition.
   A same-revision stale projection can satisfy that weaker assertion after a
   failed view request. The real Graph capture should be checked against the
   actual Graph view definition and scene; a failed projection must not pass.

## Input and evidence limits

`real_targets.rs:36-47` checks finite, positive, recent widget rectangles within
the native viewport. Recording geometry does not mutate application state. Its
viewport test is not an occlusion or scroll-clip test; the runner must still
observe the resulting selected canonical object. Older `automation::target`
lookups used by this journey do not check rectangle age or current visibility.
This can cause a failed click after a panel moves, but the follow-up semantic
assertions should fail rather than manufacture success. The parent's separate
actual presentation-process test is investigating a real dialog focus failure.

Thirty lead frames are a practical click-separation delay, not a guaranteed wall
duration at all refresh rates. Keep actual input failures and improve the driver
or interaction instead of relaxing semantic assertions. Likewise an unavailable
derived edge should remain a real visibility failure; the runner correctly uses
native hit-tested geometry and does not substitute direct selection.

The existing report retains the complete validated manifest, publication binding,
source digests, validation receipt and canonical created ID. Restart compares the
exact manifest and branch head in a second native process. The initial source
check compares real `models/agentique/*.sysml` document digests. The screenshot
sidecars retain revision-bound state and image hashes. None of those automated
checks replaces independent review of hierarchy, relationship readability,
engineering meaning, or the complete operator experience.

Only formatting and diff checks were run in the isolated worktree for the fix;
the integration lead owns compilation and native execution. No standards
consumer or real semantic mutation was started by this review.
