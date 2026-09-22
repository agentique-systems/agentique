# Bounded Actions slice diagnosis

The lead's first eight-document slice stopped before its acceptance audit with
`Derivation(InputContextMismatch)`. Its last frontier was Structural, so the
remaining negative-query diagnostics alone do not establish a final closure
failure. No corpus execution was performed by this workstream.

The `sysml_diagnostic_sources` example maps canonical declared syntax locators
from an existing report using pinned source bytes and parsing only. It does not
load an accepted graph, lower records, resolve references, or replay producers.
It mapped all 24 distinct diagnostic subjects in the first Actions slice:
16 Actions usages, two Flows connections, five Items constraints, and one States
constraint. The three formal-negative subjects were:

| Subject | Actions.sysml byte range | Declaration |
| --- | --- | --- |
| `dcbeadeb-51ff-58e1-bc18-50a95aa2d31c` | 6086–6193 | `AcceptAction::aState` |
| `8cf3c953-ad02-5312-bf90-b8ab9dca34ee` | 6105–6189 | `aState::aTransition`, including its accept payload |
| `b7597899-b3ac-5da6-a5be-46141714bb05` | 13726–13822 | `ForLoopAction::whileLoop` action body |

Two focused regressions reproduce missing coverage without a corpus run:

- Directed Usage value: changing the existing combined variable/value fixture
  to `direction=in` reproduces `Derivation(InputContextMismatch)` (exit 101).
  This tests the non-valuation path that the undirected fixture did not cover.
- Nested state: a local ActionDefinition owns a composite StateUsage, which owns
  a composite TransitionUsage with a source Membership. A broad transition-source
  population creates a false dependency on pending positional producers.
  Pinned SysML.xmi lines 3019–3032 explicitly exclude FeatureMembership and all
  its subtypes from `TransitionUsage::sourceFeature`.

The generic excluded-subtype query regression passes (exit 0):
`cargo test -p agq-kerml-semantics --lib excluded_owned_population -- --nocapture`.
It preserves canonical order, non-feature Membership subclasses, ownership
proofs, and a relevant membership whose endpoint remains pending.

Both combined regressions now pass. Non-valuation contextual features are created
atomically with their value binding. Transition source lookup records its exact
Membership population with FeatureMembership subtypes excluded. Original selected
ownership entries retain their declared support when later derived entries are
outside that population; a selected derived entry or later broad observation
retains the complete derived proof. Typed structural searches survive persistence.

Final focused preflight on 2026-09-22, after `b959ad1` and `80fca95`, used
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`, and isolated target
`target/foundation-actions`:

| Command | Actual result |
| --- | --- |
| `cargo test -p agq-sysml-semantics` | Exit 0; 53 passed, including nested state, directed value, absent owner, v2 authority, and shared snapshot/value contexts |
| `cargo test -p agq-kerml-semantics --lib filtered_declared_ownership` | Exit 0; 1 passed, including narrow/broad observation orders and materialized proof reread |
| `cargo test -p agq-kerml-semantics --lib variable` | Exit 0; 4 passed after declared ownership support change |
| `cargo test -p agq-kerml-semantics --lib feature_values` | Exit 0; 2 passed after declared ownership support change |
| `cargo test -p agq-kerml-semantics --test initial_values_v10 --test ordered_results --test positional_publication` | Exit 0; 1 + 7 + 3 passed after declared ownership support change |
| `cargo fmt --all -- --check` | Exit 0 after the final proof expansion guard |
| `cargo clippy -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` | Exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p agq-kerml-semantics -p agq-sysml-semantics` | Exit 0 |

One preliminary command used nonexistent integration target `variable_featuring`
and exited 1 before running tests; the actual variable/value checks above use
their unit-test filters. An initial `selected_declared_ownership` filter selected
zero tests; the corrected `filtered_declared_ownership` command ran the regression.

These focused results do not establish corpus publication or language readiness.
The lead owns the subsequent bounded Actions slice and publication gates.
