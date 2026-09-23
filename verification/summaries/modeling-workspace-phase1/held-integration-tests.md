# Phase-1 held integration test preparation

Status: tests and fixture inputs prepared; production workspace remains gated.

`crates/modeling-workspace/tests/phase1.rs` contains five executable Rust test
bodies covering mixed document operations, immutable revisions, Working repair,
removal/re-addition identity, borrowed query facades, shared accepted standards,
operational failure atomicity, and 100 documents/five revisions/parallel readers.
Its README specifies the proposed facade and actual missing implementation.
There is no workspace crate manifest, root member or production implementation.

Every semantic test requires exact accepted KerML and Systems restoration. There
is no synthetic publication, producer replay fallback or missing-cache success.
The proposed verification-only scheduling observer checks actual evaluated
subjects against the immutable standards, rather than interpreting aggregate
counters as proof of no replay. The existing language self-model acceptance
harness remains separate and held until genuine acceptance.

Verification, low-disk profile (`CARGO_INCREMENTAL=0`, dev/test debug 0, jobs 2):

| Command | Actual result |
| --- | --- |
| `cargo test --locked --offline -p agq-kerml-text --test workspace_edit_inputs` | Exit 0: 2 tests passed, 0.57 seconds; real production frontend validates fixture inputs and 100 generated mixed documents. |
| `cargo clippy --locked --offline -p agq-kerml-text --test workspace_edit_inputs -- -D warnings` | Exit 0, no warnings. |
| `cargo fmt --all -- --check` | Exit 0. |
| `rustfmt --edition 2024 --check crates/modeling-workspace/tests/phase1.rs crates/modeling-workspace/tests/support/mod.rs verification/fixtures/modeling-workspace-phase1/executable_inputs.rs` | Exit 0; Rust syntax/format only. |
| `cargo test --manifest-path crates/modeling-workspace/Cargo.toml --test phase1 --features verification -- --ignored --test-threads=1` | Exit 1: missing manifest `crates/modeling-workspace/Cargo.toml`. Held tests were not type-checked or executed. |
| `git diff --check` | Exit 0. |

No publication or workspace performance result is claimed. The held tests will
record elapsed time, source bytes, syntax nodes, closure size and producer
counters when the production gate is available. Authored semantic reconstruction
remains an explicit phase-1 allowance; standard copying or replay does not.

Subsequent I3–I8 review on source `695e67ef44c40dd5d7191b9ce1f856989fe934ca`
strengthens the held assertions: full graph closure, unchanged declaration
syntax/semantic IDs inside edited documents, and document/source/syntax/reference/
closure baselines captured before each later edit. Empty mounts, all retained
revisions and a second workspace must expose actual shared base-table tokens and
zero copied dependency entries through an internal verification observer.
Locally contributed inverse/navigation projections remain permitted. This
observer still requires implementation; facade or record pointers alone do not
establish I8. Both cache files and the compiled receipt now precede any large
KerML restoration. No cache was loaded for this review.

Only these checks were rerun for the review; each exited 0 with empty output
(SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`):

- `rustfmt --edition 2024 --check crates/modeling-workspace/tests/phase1.rs crates/modeling-workspace/tests/support/mod.rs`
- `git diff --check`

Reviewed test-file SHA-256 values: `phase1.rs`
`3deba0a6474051a2d87ce06368256b6eb8773172f7e6f472980f3d5fc3823cfa`;
`support/mod.rs`
`379ab0f15968f6e8f904d7a2f202daa648a0886154fa58382b2fef207408cd3d`.
The prepared workspace tests remain uncompiled and unexecuted, pending the same
readiness and implementation gates.
