# Native view-intent regression handoff

`crates/studio-native/src/view_intent_tests.rs` contains nine native state regressions for the integration lead's requested/deferred view-definition implementation. Register it with `#[cfg(test)] mod view_intent_tests;` in native `main.rs`.

The tests use the existing native app kittest setup and actual Bridge mutation bookkeeping. They inject terminal worker DTOs into `receive_replies`, without opening a runtime or creating a repository. The unauthenticated worker cannot run their closures. Its actual error replies remain unconsumed, so injected reply ordering is deterministic. Passing these tests would establish presentation request routing, not semantic validation or real-model acceptance.

Coverage: coalesced latest lens with no queued read or scene reconstruction while mutation is pending; one flush after terminal failure; stale success/error versus a newer request; matching projection installation; wrong returned depth/scope rejection; exact candidate-pair scope; coherent but wrong-scope candidate pair refresh; preparation changing the candidate context; cancellation and cancellation requested before preparation completes.

Commands actually run in the isolated worktree:

```text
rustfmt --edition 2024 crates/studio-native/src/view_intent_tests.rs
exit 0; no output

git diff --check
exit 0; no output
```

No build or runtime consumer was run. After integrating the production changes and registering the module, the integration lead can use the existing shared native target/configuration with this test filter:

```powershell
cargo test --locked --offline --manifest-path crates/studio-native/Cargo.toml --bin agq-studio-native view_intent_tests -- --nocapture
```

Retain the actual integrated command, output and exit code separately. Source review alone does not establish that these tests compile or pass.
