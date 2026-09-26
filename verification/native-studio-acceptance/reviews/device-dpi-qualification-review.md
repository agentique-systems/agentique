# Device recovery and mixed-DPI acceptance review

Read-only review after real-run03. No production edits, GPU fault injection, monitor movement or new Cargo execution were performed. Existing native tests are useful automated evidence; none establishes physical device-loss recovery or interactive mixed-monitor qualification.

## Device-loss behavior present

`main.rs` installs the supported surface-error callback and `Recovery::attach` registers the wgpu device-loss callback. `surface_recovery.rs` latches non-destruction device loss, advances the graphics epoch, writes an explicit restart instruction to stderr and requests an OS window-title change. The title is useful when the renderer cannot display an in-canvas message. Normal `Destroyed` teardown is intentionally excluded.

`gpu.rs` stops its scene prepare/paint callbacks after device loss. `StudioApp::update` receives already-requested semantic/scene outcomes, detects the fault, attempts `save_session` on first notice and periodically, then returns before keyboard/editor UI processing. There is no new commit/edit operation in the recovery handler. An already-authorized in-flight commit may finish; that is not a blind new mutation and must not be mislabeled as forbidden head movement.

Session writes are separate from model state. `write_presentation` writes a bounded same-directory temporary, flushes it, then replaces the session file. `save_session` refuses unstable in-flight revision/lens restoration. This prevents saving a mismatched presentation, but means that an immediate fault during a transition may retain the previous saved stable view. A process-local candidate is not durably committed by saving presentation; candidate-only selections are reconciled away on ordinary durable restoration.

Existing tests cover five bounded surface retries, timeout/OOM handling, a sticky device-loss stop state that cannot be cleared by a successful-surface callback, normal teardown, presentation roundtrip and failed/oversized replacement preservation. In `verification/native-studio-alpha/checks/acceptance-native-tests-06.txt` those tests pass within the 158-test suite. The companion command receipt is exit 0 and records source `fea3339d` plus its actual working changes; it is not a claim that every later patch was tested by that run.

## Concrete gap and bounded fix

The terminal **surface** fault branch in `app.rs` sets status and returns before either `save_session` or the normal eight-second autosave. Exhausted Lost/Outdated retries and OutOfMemory enter this branch. The device-loss branch immediately attempts persistence, but the surface branch does not. Normal orderly exit still calls `save_session`; forcibly restarting an unresponsive window can retain an older autosave. This is a real behavior asymmetry, not evidence that model commits are corrupted.

Bounded correction: perform the same stable-presentation checkpoint attempt when entering either terminal graphics-fault state. Track first-notice/save state independently of the status string, since a save error can replace that string. Preserve existing revision/lens stability guards. If the save fails, retain the old file and emit an actionable stderr/title indication; GPU-only status text is insufficient during a device fault. Continue to block new blind editing and leave durable repository operations outside the graphics handler.

Missing automated gate: inject a device fault and a terminal surface fault into the real host recovery boundary without requiring a physical GPU reset. Start with a stable app presentation newer than its saved file; drive the fault-handling update; verify the saved world/focus/camera/filter/selection binding, the requested OS title, and zero newly enqueued edit operations. Repeat with a pending revision transition and a persistence failure to prove previous stable state survives. Reopen the session in a separate presentation host. For actual durable head stability, pair the injected host fault with a real repository identity/head before/after check; a fixture-only UI test cannot establish repository recovery.

No hardware device-reset/restart result is retained here. Ordinary real restart and fixture presentation restart are separate evidence and must not be reported as device-loss qualification.

## Mixed-DPI coverage and exact remaining gaps

| Requirement | Current automated coverage | Remaining gate |
| --- | --- | --- |
| Camera | `ordered_wheel_anchor_survives_12000_native_input_cycles` uses actual egui input passes, scale factors 1/1.25/1.5/2/2.5, frame cadence changes, zoom extremes and event-time pointers. It passes in native-tests-06. | It settles each DPI change before taking the anchor and always sends a fresh pointer event before the wheel. Add DPI transition plus wheel with a stationary pointer/no new PointerMoved event; do not claim a demonstrated production bug before that sequence is measured. |
| Hit testing / selection | `mixed_dpi_scene_hit_ports_and_popup_anchor_use_logical_coordinates` transforms node center through logical/physical coordinates and verifies exact spatial node identity; outliner-reveal tests vary logical viewport sizes. | Drive an actual native selection response after a scale transition and assert selected canonical ID/revision, including the real viewport offset. |
| Port positions | The mixed-DPI test roundtrips all port positions through the camera; separate scene tests assert semantic port hits and edge attachment. | Combine these in a native scaled viewport: actual port response/hit, selected semantic ID and attachment location after a scale-factor change. |
| Popup placement | The named mixed-DPI test calls a variable `popup`, but it only multiplies and divides coordinates. | It never opens a context menu or measures its response rectangle. Open the actual node/port context menu after a DPI transition, including a near-edge anchor; require viewport containment and correct semantic target. |
| Saved presentation | Session/Local View tests cover persistence, exact revision binding and invalid camera rejection. `apply_saved_presentation` deliberately keeps the current viewport size. A real fixture presentation restart passed separately. | Roundtrip a SavedPresentation while changing egui native scale/viewport; assert current viewport survives, world camera/selection/filter identities remain equal, and restored ports/popup still map correctly. The existing restart comparison ignores viewport size intentionally; it is not mixed-monitor evidence. |

No new production scaling defect is established by this review. The concrete missing DPI work is integrated input/popup/presentation coverage. The real camera stress evidence at DPI 1 and the algebraic checks must remain distinguished from manual mixed-monitor interaction.

## Follow-ups already visible in round 2

The candidate status in `08-candidate` still reads `nearby context for 0 of 1 visible changes` after the part is visibly selected. `history.rs::frame_change_group` computes that string at framing time; `actions.rs::select_from_outliner` moves the camera without invalidating it, and viewport pan/zoom can do the same. A bounded fix should make framing feedback operation-specific (for example, “Focused ModelingPlatform change context; full comparison retained”) or bind a structured observation to the camera/scene and clear it when those change. Avoid broadly clearing error/validation/cancellation status on every pointer movement. A regression must cover focus, then outliner reveal/manual camera movement, while unrelated error messages remain intact.

`inspector.rs` unconditionally draws the expanded recommendation body before the selected element, including a disabled “Recommended view is open” button. The correct already-applied state is known. Collapse only that body into a compact “Graph World is open · deterministic view mock” disclosure, keeping provider/intent/input revision/subject/result provenance visible in the dependency card and all decision evidence accessible on expansion. Preserve the active action for recommendations to a different World. This is a presentation change, not a new agent capability.

These follow-ups remain proposals until the requested post-fix capture and ownership coordination. Device/surface checkpointing has a correctness rationale; recommendation compactness is lower priority than the remaining durable Diff and semantic latency work.
