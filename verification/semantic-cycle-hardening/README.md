# Exact semantic cycle diagnostics

Branch: `foundation/semantic-kernel-v2`.
Parent commit: `12ff2881d691476b4c97e9aacc673834566bd94f`.
Verification date: 2026-09-17, Windows x64, Rust 1.92.0, Node 22.11.0.
The tested working-tree source hashes, command exit codes, timestamps and durations
are recorded in [results.json](results.json).

## Change and scope

`cyclic_nodes` now uses iterative Kosaraju traversal: a DFS records finish order,
then a traversal of reversed edges in reverse finish order identifies SCCs.
Only components with multiple vertices or a singleton self-loop contribute nodes.
Ordered maps/sets and a sorted result make output independent of insertion order
and hash iteration. Both traversals use heap-backed stacks, including for deep
graphs. Auxiliary storage is O(V + E); ordered map/set operations give an
O((V + E) log(V + 1)) time bound. No dependency or unsafe code was added.

Both production callers were inspected: model containment validation in
`crates/kernel/src/model.rs` and explanation dependency validation in
`crates/kernel/src/derived.rs`. Their existing error variants now receive exact
cyclic members. Neither caller's validation contract needed alteration.
No additional invariant-threatening defect was found during this scoped review.

The ADR and contributor guide remain accurate: they describe acyclicity checks
and iterative detection without prescribing topological elimination. Architecture,
identity, snapshots, parsers, persistence and language support are unchanged.
Original standards, HTML/PDFs and library files are unchanged.

## Regression coverage

Seven helper tests were added: A <-> B -> C reports only A/B; A -> B -> C -> D
reports none; a self-loop; a three-node cycle; two cyclic SCCs linked by B -> C
with sorted output under reversed insertion order; upstream/downstream/isolated
vertices and an empty graph; and converging acyclic paths.

The existing deep test retains its 20,001-vertex acyclic chain and two-node cycle
at the end, and now also checks one SCC spanning all 20,001 vertices. Containment
tests assert exact members for a two-node cycle and a self-loop, each with an
acyclic descendant. Derivation tests assert exact two-node/self-loop errors; a new
integration test excludes both acyclic evidence and acyclic dependents from the
reported explanation cycle and confirms the declared snapshot stays unchanged.

Before replacing the algorithm, `cargo test -p agq-kernel --lib` produced the
expected regression failures: 6 passed, 2 failed. The old helper returned A/B/C
instead of A/B and B/C/D instead of B/C. After replacement,
`cargo test -p agq-kernel` passed all 39 tests, including the Rustdoc example.

## Required verification actually run

All eight final commands exited 0; none was skipped.

| Command | Result | Output |
| --- | --- | --- |
| `cargo fmt --all -- --check` | PASS (no output) | [log](rust-format.txt) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | [log](rust-clippy.txt) |
| `cargo test --workspace` | PASS, 87 tests including doctests | [log](rust-tests.txt) |
| `cargo doc -p agq-kernel --no-deps` | PASS | [log](kernel-rustdoc.txt) |
| `npm run check` | PASS | [log](frontend-types.txt) |
| `npm run build` | PASS | [log](frontend-build.txt) |
| `npm test` | PASS, 4 tests | [log](engineering-tests.txt) |
| `npm run test:e2e` | PASS, 4 tests; no skipped, flaky or unexpected results | [log](browser-tests.txt), [report](browser-results.json) |

Existing browser reports/screenshots were preserved and restored byte-for-byte.
The report from this run is retained here. Source hashes were rechecked after the
suite. No broader language-conformance or verification-success claim is made.

## Exact changed files

- `crates/kernel/src/model.rs`
- `crates/kernel/tests/derivations.rs`
- `crates/kernel/tests/snapshots.rs`
- `verification/semantic-cycle-hardening/README.md`
- `verification/semantic-cycle-hardening/browser-results.json`
- `verification/semantic-cycle-hardening/browser-tests.txt`
- `verification/semantic-cycle-hardening/engineering-tests.txt`
- `verification/semantic-cycle-hardening/frontend-build.txt`
- `verification/semantic-cycle-hardening/frontend-types.txt`
- `verification/semantic-cycle-hardening/kernel-rustdoc.txt`
- `verification/semantic-cycle-hardening/results.json`
- `verification/semantic-cycle-hardening/rust-clippy.txt`
- `verification/semantic-cycle-hardening/rust-format.txt`
- `verification/semantic-cycle-hardening/rust-tests.txt`
