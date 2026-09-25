# History spatial review followup — source handoff

## Actual criticism

The authenticated real capture `real-run05/gallery/06-history-diff.png` and its
sidecar show Graph/Diff on project `07d1f9bd-8f5b-4811-a9ab-7dd6408f91d9`, from
`8d7fa8e8-3834-4048-b3bd-e85d1d675f16` to
`62af4537-f2c9-4bf1-90d6-e0862d6b4141`. This is real semantic data, not a fixture.
The sidecar reports 159 projected nodes / 425 edges, 130 scene nodes / 405 edges,
30 visible nodes, no selection or Inspector, and camera zoom 0.51925194. The
status says AgentArchitecture has nearby context for 14 of 26 changes.

The expanded review card permanently shows three sample rows, a list disclosure,
and explanatory text. Together with Graph filters and disabled exploration
controls, it consumes roughly 400 screen pixels beneath navigation. The short
remaining canvas clips the graph. AgentArchitecture is the named review group,
but its actual owner node is outside the frame.

The retained layout explains the missing context: AgentArchitecture occupies
world bounds `(0,640)..(232,758)`; AgentRuntime `(656,480)..(888,598)`;
alphabetically first Agent `(2898,0)..(3130,118)`. Existing focus starts at that
first change and considers the owner last, after filling the readable frame.
These coordinates are observed source data, not a new rendering measurement.

## Change

- The review uses two compact rows. Total object, relationship and owner counts,
  selected owner, selected group's object/relationship counts, and full-fit
  action remain visible.
- **Browse all changes** opens the complete scrollable list of every group and
  change in the exact view pair. Identity, revision, origin and change details
  remain on each record. Selecting a record closes the overlay and frames that
  exact scene target; there is no permanently expanded sample list.
- Group focus starts at actual owner geometry and gathers the nearest changes
  that fit legibly. Explicitly selected changes still take priority for the
  ordinary Focus changes action. No coordinates, ghosts, semantic projections,
  revision bindings, or candidate phases change.
- **Inspect owner** targets the canonical node/port actually present in the scene,
  with its displayed revision, including a removed ghost's revision. It is
  unavailable for an owner outside the scene. Labels never manufacture targets.
- In Diff, Graph filters move into one menu with the selected family count and
  standard-expansion setting visible. Filters still query both exact revisions.
  Invalid local-neighborhood commands do not occupy this row. Ordinary Graph
  retains its existing family controls, exploration actions and standard toggle.

## Checks performed

Source changes are limited to native `history.rs` and the Graph-control section
of `panels.rs`. The scene/projection/language/runtime/model sources are unchanged.

```text
rustfmt --edition 2024 --check crates/studio-native/src/history.rs crates/studio-native/src/panels.rs
exit 0
git diff --check
exit 0
```

Three added tests are **prepared, not executed** in this worktree:

```text
history::tests::group_focus_frames_actual_owner_and_nearby_changes_before_alphabetical_changes
history::tests::owner_inspection_targets_actual_revision_or_remains_unavailable
history::tests::closed_change_list_leaves_canvas_space_and_preserves_exact_pair
```

The existing dispersed explicit selection, port/relationship, ghost-revision,
mixed-lens rejection, and candidate phase/binding tests remain. The added UI test
asserts the closed review uses at most 110 logical pixels at a 1000-pixel content
width and preserves the full pair, full scene and complete change counts.

Integration should run the native history tests and capture the same real
History/Diff with the popup closed, then open the full list and inspect an owner
and a removed change. No new screenshot, native execution, build, speedup, or
visual acceptance is claimed here. Distant changes still require list selection
or full-fit navigation; this patch preserves spatial continuity rather than
repacking the graph to manufacture a better screenshot.
