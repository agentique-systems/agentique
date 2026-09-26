# Graphics fault presentation checkpoint

Base: `16639b66`. This change closes the terminal surface-error early return that
previously bypassed presentation persistence. Device and terminal surface errors
now use the same stable-view checkpoint path before returning without dispatching
new editor input. Existing pending-view and opt-in automation guards remain.

Save failure is retained independently of status text, reported in the recovery
notice and OS title, and retried at the normal eight-second cadence. A skipped
pending view can save on the next stable update. No repository API is added to the
fault handler; previously requested semantic operations may still complete.

Three added tests run the real eframe host update with injected recovery callback
state and no GPU:

- Both terminal surface and device faults save a presentation that the ordinary
  constructor reopens. A real Prepare-widget positive control establishes that
  the same pointer input would prepare a fixture candidate without the fault;
  faulted updates issue no mutation or candidate and leave revision data intact.
- An unresolved revision preserves the last checkpoint until the view is stable.
- Failed atomic replacement produces an actionable OS title, retains the earlier
  valid checkpoint, and does not perform a failed write on every subsequent frame.

This is host control-flow and presentation persistence coverage. It does not
establish physical device-loss, renderer recreation, or mixed-monitor manual
qualification, nor does a fixture candidate establish semantic acceptance.

Source checks in the isolated worktree:

```text
rustfmt --edition 2024 crates/studio-native/src/app.rs crates/studio-native/src/surface_recovery.rs crates/studio-native/src/surface_recovery_host_tests.rs
exit 0; no output

git diff --check
exit 0; no output
```

Native test execution and Clippy are pending the integration lead's shared build
slot. No local Cargo build or semantic workload was run for this change.
