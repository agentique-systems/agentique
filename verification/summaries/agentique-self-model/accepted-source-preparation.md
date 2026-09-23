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

## Authored construction lifetime reduction

Implementation `9d32eefababeed68cc80964b0955217d3868e7d5`, based on `5cbd666`:
accepted authored lowering captures the same reference/source metadata and checked
closure checkpoint, then releases the construction draft, partial overlay and
temporary mount before final strict closure. The intermediate desired snapshot
is scoped to publication. Declared/derived separation, previous revisions,
reference evidence and accepted dependency identities are unchanged. The shared
reference-audit helper now accepts metadata rather than a graph-bearing draft.
No kernel sharing or workspace implementation is included; peak savings were not
measured and no accepted-cache/publication gate was run.

Checks used the environment above (`CARGO_BUILD_JOBS=2`, no debug information or
incremental compilation, isolated `../agentique/target/bridge-self-model`). Raw
combined PowerShell output is ignored under
`verification/generated/agentique-self-model/authored-lifetimes/`.

| Exact command | Exit / result | Output SHA-256 |
| --- | --- | --- |
| `cargo test -p agq-kerml-text --lib --test project --test frontend --test workspace_frontend_inputs` | 0; 42 unit, 12 frontend, 13 project and 2 input tests passed; 1 accepted-cache gate ignored | `eb4827e35071b1855e70aba03c12b29f31099690aa7720e766ab1cf6df2024f0` |
| `cargo clippy -p agq-kerml-text --all-targets -- -D warnings` | 0 | `a2925c39c849faf88b31d2557168a384ff30ecd345edabc0aa84b2754a6bb1e8` |
| `cargo fmt --all -- --check` | 0; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `git diff --check` | 0; empty output | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The new regression releases the construction allocation, then checks Complete
typing and private-alias endpoints, visibility and exact reference provenance.
Its first focused run,
`cargo test -p agq-kerml-text --lib source_reference_audit_outlives_construction_without_retaining_its_graph`,
exited 1 because the test equated a reference origin's syntax node with its
canonical relationship origin's syntax node. Those identities legitimately differ;
the corrected assertion checks the original pending reference's origin. This was
a fixture-oracle correction, with no semantic change. That precommit working-tree
run's output hash is unavailable; the passing combined run above covers it.
