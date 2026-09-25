# Local views and extended session restoration

This increment adds local operator bookmarks beside the repository database, in
`<database-stem>.native-views.json`. The repository and service currently expose
no saved-view persistence operation. These files therefore do **not** claim to be
shared project metadata or a semantic repository commit.

Every bookmark retains its exact project/fixture and immutable revision, public
`ViewDefinition`, world, camera, layout, collapse/expansion settings, branch, and
panel preferences. Updating a bookmark from the current revision is explicit.
An unavailable saved revision is refused without silently advancing the bookmark.
An invalid existing store is reported rather than replaced with an empty store.
Writes flush a unique same-directory temporary before replacement. A size limit
and validation reject unsupported versions, inconsistent lenses, duplicate view
names, and unsafe camera coordinates.

## Parent integration

- Add `mod saved_views; mod presentation;` in `main.rs`.
- Call `self.local_views_menu(ui)` once from the toolbar. It caches its editor and
  list in egui memory, loading storage when the menu opens and after mutations.
- Replace the single-focus breadcrumb with `self.breadcrumb_navigation(ui)` after
  the project/home button. It reports missing ancestry or ownership cycles and
  never invents hierarchy by splitting qualified names.
- Add `presentation: Some(self.capture_presentation())` to the `Session` built in
  `save_session`. Add `presentation: None` to navigation/test Session literals.
- On project restoration, preload the saved presentation's world, focus,
  relationship families, include-standard flag and collapse/expansion settings
  before requesting the lens. If present and valid for that project, retain its
  branch preference.
- In `apply_pending_presentation`, use `self.apply_saved_presentation(presentation)`
  for an extended session, then rebuild. For a legacy session retain the existing
  camera/layout path. `open_local_view` sets this restoration and submits the exact
  saved `ViewDefinition` through the worker.
- Retain the saved definition's depth and hidden IDs in future lens requests if
  the operator remains in that lens. The capture helper already preserves these
  from the loaded projection.

Recovery removes stale references only from disposable layout/expansion state.
It labels missing IDs as outside the loaded projection, not semantically deleted.
Hidden IDs remain preserved because they are intentionally absent from a view.
Source/Explain dialogs require a selected revision-bound object and are closed on
restoration; the agent panel preference is restored. No evidence is shown for a
selection that was not saved.

## Checks performed

`rustfmt --edition 2024 crates/studio-native/src/presentation.rs crates/studio-native/src/saved_views.rs crates/studio-native/src/session.rs`

Exit 0, no output.

`git diff --check`

Exit 0, no output.

No Cargo build was launched in this worktree for this increment, at the integration
lead's request to avoid duplicate targets while disk space is constrained. The
parent gate must compile the modules after adding declarations and the three
Session initializer updates. New tests cover exact revision/lens round trips,
safe missing-ID recovery, cross-project isolation, corrupt-store preservation,
invalid-camera rejection, legacy sessions, atomic replacement rejection, and
bounded canonical breadcrumbs.
