# Rich authored vehicle preparation

`crates/kerml-text/tests/fixtures/vehicle.sysml` describes fuel, typed ports, an
engine and tank, a vehicle fuel connection, a specialized sports car, a driving
action, a three-role state and a requirement. It includes every textual family
required by the authored vertical milestone.

The focused test establishes Operational v2 parsing, strict canonical lowering,
direct definition endpoints, inherited identities without copied records,
redefinition suppression, inherited port members, resolved connection endpoints,
and source provenance. Its KerML context explicitly excludes implied semantics;
these are declared/current-graph results, not accepted Systems-dependent closure.

Existing SysML current APIs cover these type/member results through
`current_usage_types`, `current_attribute_definitions`, `current_item_definitions`,
`current_part_definitions`, `current_port_definitions`, `owned_usages`,
`current_effective_usages`, `redefined_features` and
`current_connection_related_features`. The effective certificate contract,
accepted standard bases, stable accepted IDs and rich programmatic equivalence
remain acceptance-gated follow-up work.

Verified on 2026-09-22 with Rust/Cargo 1.92.0, Windows, isolated preparation branch
from `155edec`, target `target/foundation-vehicle`, and
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2`.

| Command | Actual result | Exit |
| --- | --- | --- |
| `cargo test -p agq-kerml-text rich_vehicle_vertical_parses_and_lowers_with_current_graph_identity -- --nocapture` | 1 passed, 0 failed; 2.12 s | 0 |
| `cargo clippy -p agq-kerml-text --all-targets -- -D warnings` | No warnings | 0 |
| `cargo fmt --all -- --check` | No diff | 0 |

The first compile attempt in the Actions worktree exited 1 because its previously
committed `EffectiveOwnership` call awaited the closure agent's corresponding API.
This independent fixture verification uses the already compiling preceding
revision; no producer rule or completeness requirement was bypassed.
