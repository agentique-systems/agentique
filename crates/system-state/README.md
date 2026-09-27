# agq-system-state

The System State: the live, authoritative model of one project
(REALIGNMENT §3.1, part `SystemState` in `models/agentique/Agentique.sysml`).
It holds an `agq-language` element tree, and the Surface and the Assistant
change it through the same typed operations (§3.2).

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
  (`Stale`, `Locked`, `Invalid`) and leaves the state unchanged. A well-formed
  change that makes the model invalid is applied, and its problems are in
  `diagnostics()` at the elements concerned (R-18).
- **Locks** (R-11): a lock covers the element and everything it owns. Touching
  a covered element needs the lock in `Change::confirmed`. The Assistant cannot
  remove a lock without confirmation.
- **Undo and redo**: one step per change.
- **Change events**: every apply, undo, redo and load returns a `ChangeEvent`.

Saving and loading through git (History) are added in Stage 2.
