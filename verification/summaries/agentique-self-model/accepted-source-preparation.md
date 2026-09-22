# Accepted authored source integration preparation

The additive accepted Systems constructor, shared dependencies, exact closure
checkpoint transport, and revision query facades are implemented. The original
current-graph constructor remains available. No accepted Systems receipt was
created and the permanent self-model acceptance gate has not passed yet.

Checks on `d8165d0` plus the authored source and harness changes, using the same
low-disk profile recorded in `closed-dependency.md`:

| Command | Result |
| --- | --- |
| `cargo test -p agq-kerml-text --lib` | exit 0; 39 passed, 1 explicit accepted-cache gate ignored |
| `cargo clippy -p agq-kerml-text --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |
| `cargo test -p agq-kerml-text --test project` | exit 0; 13 passed after shared-document storage change |
| `cargo test -p agq-kerml-text --test project edit_preserves_other_document` | exit 0; strengthened pointer-sharing assertion passed |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p agq-kerml-text --no-deps` | exit 0 |

The separate ignored harness requires `AGENTIQUE_KERML_CACHE` and
`AGENTIQUE_SYSTEMS_CACHE`, restores only trusted receipts, and compares effective
queries against a directly authored ChangeSet fixture sharing the same standards.
It also checks edits, retained canonical IDs, original inherited-member identity,
ordered endpoints, architectural dependencies, shared syntax arenas and parallel
reads. See the model README for the exact command. No requested accepted run was
reported as successful while the Systems receipt was unavailable.

The self-model's ordinary structural tests remain real frontend and canonical
query tests. Their success does not substitute for the accepted-cache gate.
Production workspace integration remains conditional on the lead's readiness
decision. Local source reconstruction and scheduling are not yet a complete
incremental editing algorithm; immutable standard subjects are never scheduled.
