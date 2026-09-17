# KerML query foundation verification

Branch: `semantics/kerml-query-foundation`. Verified 2026-09-17.
Architecture, exact normative anchors, dependencies and omissions:
[ADR 0005](../../docs/adr/0005-kerml-semantic-query-foundation.md).

The separate `agq-kerml-semantics` crate evaluates immutable normative snapshots
and overlays through generated typed views. It has no parser, mutable expansion
or query cache. A minimal generic association storage extension supplies canonical
ownership links and reconstructible scalar inverses; kernel code stays language-neutral.

## Actual results

All commands below exited 0. Machine-readable timestamps, durations and statuses
are in [results.json](results.json); links below contain captured raw output.

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | [Passed](format.txt) |
| `cargo clippy --workspace --all-targets -- -D warnings` | [Passed](clippy.txt) |
| `cargo test --workspace` | [146 passed, 0 failed](rust-tests.txt), including 23 semantic-query integration tests and 2 doc tests |
| `npm run check` | [Passed](frontend-check.txt) |
| `npm run build` | [Passed](frontend-build.txt) |
| `npm test` | [6 passed](node-tests.txt) |
| `npm run test:e2e` | [4 passed](browser-tests.txt); [structured report](browser-results.json) |
| `cargo run --locked --offline -p agq-metamodel-gen -- --check` | [Passed](generated.txt); checked-in descriptors/views remain current |
| `npm run standards:check` | [Passed](standards.txt); [artifact/entry integrity](standards-integrity.json) |

Reproduce from the repository root with
`node verification/kerml-query-foundation/run.mjs`. The final Rust checks were
repeated with `--rust` after proof-chain and regression-test refinements. The
frontend, browser and generated-byte checks apply to unchanged corresponding code.
The runner preserves prior global browser/screenshots and standards reports and
copies this run's outputs here. Original publications/library artifacts were not edited.

## Evidence covered

[Normative integration fixtures](../../crates/kerml-semantics/tests/foundation.rs)
exercise specialization chains, diamonds and alternative proofs; generated
FeatureTyping/Subsetting/Redefinition endpoint resolution; direct ownership and
membership; inheritance without copying; private visibility; intermediate and
transitive redefinition and competing replacements; and subsetting without
redefinition suppression. Specialization reachability supports cycles; ownership
cycles are invalid, including cycles beyond the query root. Cyclic effective-feature
inheritance is explicitly incomplete.

Tests also cover malformed memberships/subjects, unsupported conjugation and
feature aliases, lookup misses and renamed nonmatching members, incoming reference
retargeting, added redefinition, atomic inverse uniqueness, ownership moves,
recursive overlay provenance, options/library pins, exact descriptor rejection,
different overlays on one revision, and creation-order-independent contents/proofs.
The synthetic model has 1,500 types and 1,499 specialization edges; assertions
check results and bounded proof size, without a flaky elapsed-time threshold.

## Completion boundary

Implemented: direct ownership/owning relationships, owned membership traversal,
declared-name lookup, direct/transitive specialization, direct feature typing,
direct subsetting/redefinition, bounded effective features, and rule-labelled
proof graphs with positive and negative/search dependencies. Context identities
pin actual revision, model/provenance content, exact metamodel, semantic rules,
library content identities and options. No cache or incremental invalidator is claimed.

Deliberately omitted: full derived Feature::type, imported/global name resolution,
conjugation, feature chains, cyclic inheritance computation, normative inherited
ordering, full constraint validation, library implications, text parsing/import,
SysML, API server, diagrams and execution runtime. Existing application semantics
are not migrated in this change.

Ready for a bounded textual lowerer to emit supported canonical elements,
relationship endpoints and ownership links and consume these APIs. Not a complete
KerML textual semantic implementation. Passing these checks is not full normative
conformance, library certification or verification success of a modeled system.
