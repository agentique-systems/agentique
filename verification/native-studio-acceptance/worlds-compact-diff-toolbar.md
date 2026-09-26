# Diff toolbar compactness follow-up

The main native retest still measured a 127 px closed Diff panel after moving navigation controls. Inspector width passed. The remaining Diff height comes from applying the global form style to three toolbar rows: 30 px minimum interaction height, 14 px vertical button padding, and 8 px inter-row spacing.

The Diff toolbar now scopes 24 px controls, 8 px vertical button padding, and 4 px inter-row spacing to its own UI. It preserves all review modes, owner context, navigation, and exact comparison state. The <=110 px test remains unchanged. The scoped style does not shrink other Studio forms.

Commands from the isolated worlds worktree:

* `cargo fmt --manifest-path crates/studio-native/Cargo.toml --all -- --check` — exit 0; no output.
* `git diff --check` — exit 0; no output.

No compilation was run here; the integration lead owns the shared target and the targeted layout retest.
