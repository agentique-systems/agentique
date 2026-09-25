# Independent History space and owner-context review

Reviewed commit `c9bd1d572bafa4ea8026324b812eafb7b8bfa5ee`, 2026-09-26.
Scope: `crates/studio-native/src/history.rs` and the bounded Diff Graph controls
in `panels.rs`. Reviewer ran no build, tests, native process or runtime consumer.

## Source judgment

No blocking semantic identity, revision or scene-mutation issue was found. This
is conditional source approval, not visual acceptance.

The patch addresses the actual run05 History screenshot's excessive vertical
chrome: complete changes move into a scrollable popup, while the closed panel
keeps counts, the owner group, group focus and owner inspection visible. All
groups and changes remain reachable, including rows without a current scene
target. The exact before/after identifiers and provenance tooltips remain.
`change_pair`, cached review scope and canonical comparison DTOs are unchanged.

`owner_target` resolves only an actual displayed port or node with a recorded
scene revision. Owners outside the scene remain unavailable for inspection.
The ordinary selection/inspection path uses that target's revision, preserving
removed-owner ghosts rather than binding them to the newer revision. Group
labels do not mint a semantic owner or navigation target.

Group framing now starts from the actual owner header when there is no selected
change. Explicit changed objects, ports and edges retain priority. Remaining
changes are ranked by geometric distance, and the existing minimum readable
fit decides how much nearby context to include. This changes only camera intent;
it does not filter the full comparison, relayout nodes, or discard ghosts.
Explicit Focus group intentionally frames the owner even if an older selection
is distant. The remembered explicit-group selection behavior remains intact.

Diff's Graph family and standard controls move into a compact menu; ordinary
Graph controls remain available in their previous context. Incoming/outgoing
and collapse controls are not accidentally reenabled while viewing a paired
comparison. Existing request paths still apply filters to both revisions.

## Prepared coverage and remaining product gate

The three new tests are useful: they check owner-first local framing without
scene mutation, current/removed/absent owner targets, and closed-panel height
while retaining the exact pair and complete counts. They were unexecuted at
this review. The existing explicit-selection and ghost tests also need to pass
after integration.

A 1000-pixel headless panel-height check does not establish the real viewport's
space or legibility. Retain an actual after screenshot showing the owner group,
readable local changes and meaningful canvas area, plus a popup capture or
ordinary interaction proving the complete list remains usable. Check narrow
windows for wrapped controls. The actual run05 History rating remains
Foundation until this new evidence exists.
