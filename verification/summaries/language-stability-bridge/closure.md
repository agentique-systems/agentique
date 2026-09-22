# Producer closure bridge

Implemented initial zero-writer evidence, dependency-aware revalidation between
additive frontiers, and compact `ProducerClosureCheckpoint` transport across
reconstructed graphs without retaining old model indexes. Checkpoints compare
actual record, endpoint, pending-input, proof-search and output-support identities.
New producer opportunities reopen causal closure even if prior values are equal.

Explain distinguishes accepted immutable dependencies from local producer closure.
Subject-scoped local producers cannot reopen protected dependency populations;
external source/inverse carriers remain open when a registered writer can change
them. An arbitrary immutable overlay is not accepted publication authority.
Absent ownership is Complete only with closed ownership and complete navigation.
The independently versioned optional certificate/context contract is now version 2;
accepted historical KerML publication/profile/receipt identities are unchanged.

Focused verification (Windows, low-disk profile, 2026-09-22):

| Command | Actual output | Exit |
| --- | --- | --- |
| `cargo check --locked --offline -p agq-kerml-semantics -p agq-sysml-semantics` | Initial compile identified missing private dependency field in accepted-query constructor; corrected before tests | 1 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure --no-run` | Test binary compiled | 0 |
| `cargo fmt --all` | No output | 0 |
| `cargo test --locked --offline -p agq-kerml-semantics --lib producer_closure -- --nocapture` | 47 passed, 0 failed; 10.73 seconds | 0 |

The 60,000-subject/80-family proof remains 2,220,216 bytes (300,000
applicable pairs, 299,995 closed, one incomplete; 6,305 ms issuance). Optional
revalidation metadata is accounted separately from the compact receipt proof.

Environment: `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`; target directory
`target/bridge-closure`. Package-wide tests, Clippy and strict Rustdoc follow at
integration. These checks do not establish Systems publication acceptance.
