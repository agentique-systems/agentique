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
