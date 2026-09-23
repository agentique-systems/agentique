# Working identity retirement preflight

Source base: `0e3d7f5`. Test/design changes only; no production frontend,
workspace API, manifest, language support or accepted-publication changes.
The command ledger records each exact source/worktree digest and raw output hash.

The existing held unresolved-reference case now continues through provider removal
while already Working, restoration of the original reference name while its
provider is absent, and provider re-addition. It requires current source origins
and query contexts, no stale raw typing endpoint or type answer, a fresh provider
identity after re-addition, and immutable earlier revisions. This discriminates
retirement based on current Working inputs from reconstruction based on the last
Validated snapshot. Existing recovery, removal and unsupported-variation cases
remain in place.

One permanent production-parser preflight deletes and reinserts a declaration
while the document remains recovered, then repairs the other package. Identical
source bytes must not resurrect the deleted syntax ID. Temporary omission alone
still preserves disjoint syntax nodes. This exercises syntax reconciliation only;
it does not establish semantic retirement or accepted Working publication.

| Check | Actual result |
| --- | --- |
| `cargo test --locked --offline -p agq-kerml-text --test workspace_working_inputs -- --nocapture --test-threads=1` | Exit 0; 3 passed, 0 failed, 0 ignored. First run passed; after changing reinsertion to a zero-width edit, the final run also passed. |
| `cargo clippy --locked --offline -p agq-kerml-text --test workspace_working_inputs -- -D warnings` | Exit 0. |
| `cargo fmt --all -- --check` | Exit 0. |
| `rustfmt --edition 2024 --check crates/modeling-workspace/tests/working_states.rs` | Exit 0; syntax/format check only. |
| `cargo test --manifest-path crates/modeling-workspace/Cargo.toml --test working_states --features verification -- --ignored --test-threads=1` | Cargo exit 101: manifest does not exist. Held semantic tests remain uncompiled and unexecuted. |

Inspection of `SourceProject::apply` and accepted `lower_accepted_source` confirms
the current strict boundary still rejects incomplete production documents before
lowering and requires strict promotion before the final reference audit. No
failure was converted into a successful Working revision in this change. The
additive input/compilation carrier remains an implementation prerequisite.

Checks used one build job, no incremental/debug artifacts, and the isolated
`target/bridge-closure` directory. No accepted cache, corpus, publication or broad
package/workspace test was run. Full command metadata is in
[`working-retirement-commands.json`](working-retirement-commands.json).
