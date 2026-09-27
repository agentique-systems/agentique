# agq-system-state

The System State: the live, authoritative model of one project
(ROADMAP §4.1, part `SystemState` in `models/agentique/Agentique.sysml`).
It holds an `agq-language` element tree, and the Surface and the Assistant
change it through the same typed operations (ROADMAP §4.2).

```rust
let mut state = SystemState::new(tree, locks);
let event = state.apply(Change::new(Actor::Operator, "Rename the store", vec![
    Operation::Rename { element: store, name: "Warehouse".into() },
]))?;
// event.created / updated / deleted name the elements to redraw.
state.undo();
```

- **Operations**: create, delete, rename, move, connect, set property, lock and
  unlock. Elements are named by identity; references stay bound through rename
  and move.
- **Atomic changes**: a `Change` applies completely or is rejected
  (`Stale`, `Locked`, `Invalid`) and leaves the model and locks unchanged.
  `Invalid` covers what cannot be done or could not be saved as SysML text and
  read back the same (`agq_language::writable`): an unknown element, a property the kind does not have
  (`Property::applies_to`), an empty name or one with a line break, a
  malformed value, a connection with three ends. A well-formed change that
  makes the model invalid (a missing type, ends that do not fit, a duplicate
  name) is applied, and its problems are in `diagnostics()` at the elements
  concerned (R-18). `SystemState::explain` describes a rejection by name.
- **Locks** (R-11): a lock covers the element and everything it owns. Touching
  a covered element needs the lock in `Change::confirmed`. The Assistant cannot
  remove a lock without confirmation.
- **Undo and redo**: one step per change. `undo_since(revision)` undoes
  every change made after a revision, such as the Assistant's work (R-12).
- **Stale changes**: `Change::with_base(revision)` rejects the change if the
  model moved on after it was prepared.
- **Change events**: every apply, undo, redo and load returns a `ChangeEvent`.

One edit on a 2,000-element model (apply, relink, validate, event) takes
about 47 ms in a debug build and 8 ms in release (`tests/performance.rs`).

## Projects

A `Project` keeps a System State saved in a project folder through History
(`agq-history`, R-6): `model/*.sysml` plus `agentique.json`, in git.

```rust
let mut project = Project::create(folder, "UrlShortener")?; // first checkpoint
let event = project.apply(change)?;       // applied and saved before it returns
project.checkpoint("Add the link store")?; // a git commit of the model folder
let before = project.tree_at(&project.checkpoints()?[1].id)?;
let what = compare(&before, project.state().tree()); // the "what changed" view
```

- **Identity**: `agentique.json` maps each element id to a locator, its kind
  and path of names (`part def Shop::Store`; `#n` for the n-th unnamed
  member, `name#2` for a repeated name), and holds the next id, so no id is
  handed out twice (also across branches and undo). Renames and moves in the
  app keep ids. An element renamed by hand in the text gets a new id on open;
  it and the entry nothing took are listed in `unmatched()`. Elements are
  never matched by name or similarity. Limitation: unnamed elements are
  matched by position, so one inserted by hand before another takes that
  one's id, and the other is reported.
- **Continuous save**: `apply`, `undo` and `redo` save before they return. A
  change that cannot be saved is undone and reported. Only documents whose
  content changed are written; the others keep their text, formatting and
  `//` notes. A crash loses at most the change being saved, never mixes old
  and new.
- **Checkpoints and branches**: `checkpoint`, `checkpoints` (newest first),
  `tree_at`, `branches`, `create_branch`, `switch_branch` (refused with
  `UncommittedChanges` while the model has changes since the last
  checkpoint, and refused before switching if the branch's model cannot be
  read).
- **One window**: a second `open` of the same folder fails with `Locked`.
