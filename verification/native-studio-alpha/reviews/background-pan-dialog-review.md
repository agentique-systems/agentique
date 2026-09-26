# Background pan: a dismissed Window still participates in the next hit test

The failed `real-run03` background gesture recorded five input hooks with a
maximum gap of 7 ms and no camera displacement. It remains a failed journey.
The source trace identifies an input-target explanation consistent with that
evidence; it does not establish responsiveness during a completed preparation.

## Exact source trace

- `studio-native/src/app.rs`, `raw_input_hook` and `update`: the scenario injects
  input before the next egui pass. Shell/viewport rendering precedes dialogs.
- `studio-native/src/part_edit.rs`, `part_edit_dialog`: the centered Window is
  constructed on the same pass in which the actual Prepare button releases and
  calls `prepare_part`.
- `studio-native/src/actions.rs`, `prepare_part`: a real request is enqueued and
  the dialog flag becomes false. That flag does not erase the already registered
  Window widgets from the completed pass.
- `studio-native/src/real_automation.rs`, the reviewed `background_input`: the
  next hook immediately presses at the previous viewport's center, over the
  dismissed centered Window.
- The locally installed **egui 0.33.3**, `src/context.rs:472-491`, computes hits
  and interactions from `viewport.prev_pass.widgets`. These were inspected in
  `C:/Users/phili/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.33.3/`.
- That exact dependency's `src/interaction.rs:164-170` latches the potential
  click/drag widget on press. Lines 124-130 intentionally retain a potential
  drag whose widget disappears; lines 182-185 clear it on release. Subsequent
  movement therefore does not transfer the held gesture to the viewport.
- `studio-native/src/viewport.rs` has no mutation-pending suppression around
  ordinary `response.dragged()` camera movement. Existing fixture automation
  starts its pan near the viewport's upper-left corner, avoiding this overlap.

The newly visible preparation footer also changes viewport height on the first
post-submit shell pass. Waiting for real completed UI passes refreshes both the
widget hit map and viewport geometry; a sleep or direct camera write would not
test this lifecycle correctly.

## Bounded regression coverage

[background_pan_tests.rs](../../../crates/studio-native/src/background_pan_tests.rs)
uses actual `egui::Context::run`, a centered Window, the actual egui Prepare
button's click, `Sense::click_and_drag`, `Response::dragged`, actual pointer delta
and `Camera2D::pan_screen`. It does not set a drag ID or mutate the camera to make
an assertion succeed.

The first fresh harness closes the Window through native-style primary
press/release events and immediately drags its former center, expecting no
viewport drag. A separate fresh harness closes the same Window, completes two
pointer-up/move-only UI passes, then drags from the production runner's bounded
corner helper. It requires actual drag responses and the exact expected camera
translation. A third test covers the helper's minimum 104 by 88 pixel viewport
and rejects insufficient geometry.

These tests isolate toolkit gesture mechanics. They do not contain a semantic
worker, real model, candidate or fabricated responsiveness result. The actual
real journey must still prove concurrent pan and current-revision inspection.

## Qualification boundary

The parent implementation retains the submission-to-result input-hook clock,
including the settling passes, the 250 ms maximum gap check, actual camera
displacement, exact unchanged baseline binding and mutation-pending condition.
Completion before the gesture cannot be counted as concurrent panning. No
retry-until-pass or direct camera/interaction override is recommended.

Only source formatting was run by this subagent; the Rust tests were **not yet
compiled or executed** at handoff. The parent will run the integrated focused
gate before the fresh `real-run04` database/authentication/seed journey. The
failed run03 database and report remain intact.
