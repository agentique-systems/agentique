# Authenticated finalization API focused checks

Base: `0c4c0adae19125c5a72b3e3521c7df1c6c6d84f0`.
Tools: `rustc 1.92.0 (ded5c06cf 2025-12-08)`,
`cargo 1.92.0 (344c4567c 2025-10-21)` on Windows.

The finalizer authenticates an independently pinned journal, restores only its
last strict converged graph under the accepted immutable dependency, reconstructs
exact source declarations and provenance through the real frontend, and checks
its graph-bound context, registry, certificate, local evaluation rows and counts.
It does not enter the producer scheduler or replay source-reference refinement.
The old and restored publication paths share all strict final acceptance audits.
Effective SysML queries are now audited explicitly beside current-graph queries.
Read-only audit phases append synchronized begin/end observations to a fresh
ignored output directory. These observations never confer publication authority.

| Command | Exit | Result |
| --- | ---: | --- |
| `cargo check --locked --offline -p agq-kerml-text` | 0 | Compilation passed. |
| `cargo test --locked --offline -p agq-kerml-semantics --lib direct_` | 0 | Initial implementation: 13 tests passed. |
| `cargo test --locked --offline -p agq-kerml-text --lib sysml::publication::tests` | 0 | Six publication gate regressions passed, including pinned source provenance. |
| `cargo test --locked --offline -p agq-kerml-semantics --lib frontier_tests` | 0 | Final hardened implementation: six tests passed, including direct restoration, open-frontier rejection and unrelated equal-digest context rejection. |
| `cargo fmt --all -- --check` | 0 | Final formatting passed after initial formatting-only failures (exit 1) were corrected. |
| `cargo clippy --locked --offline -p agq-kerml-text -p agq-kerml-semantics --lib -- -D warnings` | 0 | Focused library lint passed. |

The final implementation digest is
`1a37ea186c3f2790b41d9d9a43b0a498832d56366d3baa2286c346abb66b5780`:
SHA-256 over the lexically ordered six changed Rust paths, each encoded as UTF-8
path, NUL, little-endian 64-bit byte length, then its exact file bytes. The paths
are `producer_worklist.rs`, `publication_frontier.rs`, `publication_overlay.rs`
under `crates/kerml-semantics/src`,
`crates/kerml-semantics/tests/unit/publication_frontier.rs`, and
`publication.rs`, `publication_finalization.rs` under `crates/kerml-text/src/sysml`.

Builds used the shared generated target `target/platform-finalization`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0` for tests and `CARGO_BUILD_JOBS=2`.
Raw outputs are ignored under `verification/generated/systems-finalization/`
in the isolated finalization worktree. Small scheduler unit fixtures establish
restoration equivalence; the retained Systems scheduler was not replayed.
These checks do not claim Systems publication acceptance.
