# Native Studio engineering experience: focused checks

Worktree: `agentique-alpha-experience`, branch `work/alpha-experience`.

The first experience increment changes presentation only. It groups canonical
Inspector projections by engineering concerns, draws only supplied Explain
proof edges, exposes truthful candidate lifecycle labels, and adds bounded graph
exploration and fuzzy command search. It does not establish real-model runtime
acceptance.

## Commands and results

1. `cargo fmt --manifest-path crates/studio-native/Cargo.toml`
   Exit 0. No output.
2. `cargo test --manifest-path crates/studio-native/Cargo.toml --locked --offline --no-default-features`
   Exit 1. During the in-progress edit: `E0004`, `ExpandBoth` and
   `CollapseNeighborhood` not yet covered by `actions.rs`. The complete action
   arms were added before the next check.
3. `cargo fmt --manifest-path crates/studio-native/Cargo.toml`
   Exit 0. No output.
4. `cargo test --manifest-path crates/studio-native/Cargo.toml --locked --offline`
   Exit 1. `22 passed; 1 failed`. Fuzzy search `grph` ranked the new next-hop action
   before Graph World. The world command's visible label was simplified to
   `Graph World`, making the primary world the first equally close match.

5. `cargo test --manifest-path crates/studio-native/Cargo.toml --locked --offline`
   Exit 0. `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured;
   0 filtered out; finished in 0.06s`. Build completed in 34.55s.
6. `git diff --check`
   Exit 0. No output.

## Limits

- Graph expansion here stays inside the loaded projection and says so in the
  status text. This is not a claim of an unbounded canonical graph query.
- The Explain diagram shows immediate recorded support, including truncation;
  it does not invent a complete transitive proof.
- Fixture Explain remains prominently illustrative. Runtime acceptance is a
  separate gate.
- No fine-grained reconstruction progress, cancel-during-prepare, saved-view UI,
  or additional semantic edit command is claimed by this increment.
