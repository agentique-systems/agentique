# Independent review: real candidate selection gate correction

Reviewed `eebd8a6` plus `eab19e6431c952ef65b263e711cacceb20029eff` as one change.
This is source review only; the reviewer ran no build, test, native process or
semantic consumer. No source-level blocking finding remains. Focused execution
and the next actual candidate/commit/restart journey remain required.

The correction addresses the actual run05 driver contradiction documented in
[the real image/state review](real-run05-editor-interaction.md), whose exact
artifacts are pinned in [its evidence manifest](real-run05-editor-evidence.json).
The failed step preserved ViewService on the same candidate revision and camera;
the driver incorrectly required the newly created part before the following
ordinary selection action. The failed report remains failed.

## Acceptance boundary

- Before an ordinary Mode action, the driver records the complete typed
  selection and focus. The initial Candidate switch must preserve that target
  set, primary target, revision and focus exactly. It cannot pass by clearing
  selection, selecting a different object, or changing the engineering focus.
- The created-part expectation starts only after the existing ordinary
  `Check::Selected(PART)` passes actual selection, Inspector revision/object,
  exact owner and retained canonical identity checks. Its marker is limited to
  the active candidate after-revision. Merely knowing that a part was created
  does not assert that the operator selected it.
- Subsequent Candidate/Diff checks retain exact candidate ID, after-revision,
  matching before/after ViewDefinition and actual mode checks. They require the
  explicitly selected created ID as a target and primary selection, plus actual
  `app.selected_element()` resolution against the active scene. That last check
  retains the stronger preexisting geometry/revision requirement.
- Current requires the explicit-selection marker, excludes the created node
  from its scene, and rejects the created ID anywhere in the selection, including
  a secondary target. Diff still requires the actual new part with Added status.
  Existing `assert_bound` runs before the mode assertion and retains its scene,
  projection, selection, Inspector and Explain revision/context fences.

No production selection behavior was changed to satisfy the gate. Ordinary
scenario action ordering, baseline/runtime authentication, safe resume policy,
source checks and restart identity obligations remain unchanged.

## Review finding and correction

The first patch replaced the later `app.selected_element()` check with raw
target identity/membership in a new helper. Raw identity alone could accept a
target absent from the actual scene, when no Inspector is present to expose the
mismatch. Independent review requested retaining the original scene-resolved
check and rejecting secondary Current leakage. `eab19e6` does both.

Prepared tests exercise production App presentation transitions with explicitly
labeled fixture data: existing selection -> Candidate, explicit added selection
-> Current exclusion -> Candidate restoration -> Diff retention. Negative cases
include cleared/substituted selections, wrong primary/revision/focus, malformed
explicit marker, a Part ID disguised as a nonexistent Port, an absent Node target,
and secondary created-part leakage into Current. These are useful application
mechanics tests, not authenticated semantic candidate acceptance. Their execution
and the real after-image are not claimed by this source review.
