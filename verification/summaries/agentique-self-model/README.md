# Agentique self-model bridge

The five documents in `models/agentique` parse with the real Operational v2
frontend, lower into the canonical graph, and resolve every authored mandatory
reference with matching stored endpoints and zero kernel obligations. Four
permanent tests check architectural dependencies, richer platform semantics,
deterministic identity and source provenance, and separate implementation-path
traceability. An independent kernel ChangeSet model agrees on platform features,
typing, inheritance, redefinition, ordered interface endpoints, actions, states
and requirements; syntax IDs need not match its independently assigned IDs.

This is **current-graph structural acceptance**. Accepted Systems consumption,
combined authored producer closure and effective SysML-query acceptance remain
explicit integration gates. The model does not claim execution or completed
constraint verification. No standard publication was run for these tests.

Commands executed in the isolated bridge self-model worktree, with
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`, and target directory
`../agentique/target/bridge-self-model`:

| Command | Exit | Result |
| --- | --- | --- |
| `cargo test -p agq-kerml-text agentique_self_model --lib` | 0 | 4 passed |
| `cargo fmt --all` | 0 | Formatting applied |

Toolchain: `rustc 1.92.0 (ded5c06cf 2025-12-08)`,
`cargo 1.92.0 (344c4567c 2025-10-21)`. Initial source base:
`0fbdf3ee19788a260b99d89e71dbd28f432d8243`; the self-model implementation is
commit `8c172b5`. Follow-up effective-query verification is recorded in
[`effective-queries.md`](effective-queries.md).

Initial development runs found a reserved identifier and a missing test-project
root; both fixture defects were corrected before the passing run. The public
language behavior was not weakened to accept the fixture. Final branch-wide
checks are recorded by the integration milestone summary.
