# Structural query closure

Base: `b3cc853ab7ae763a9bdf07347437f665c444e2bf`.

The inherited positioned-feature query now gathers visible candidates from all
generals before applying explicit transitive redefinition. The canonical
identities and semantic order survive; inherited records are never copied.
This closes the interface with no local ends whose PortUsages redefine the
intermediate ReferenceUsages and ancestral BinaryLink Features.

Return inheritance decomposes unresolved specialization components, establishes
finite candidate reachability, applies explicit/positional suppression, and
checks the ordered fixed point against the ordinary inheritance equations.
Empty results retain every reachable owned-return and general-population search.
Each proven SCC member also has a `ResultPopulation` conclusion with the
`InheritedResultFixedPoint` rule, following the existing whole-population
evidence contract. Its premises retain the owned populations, component edges
and external boundary evidence. No result Feature is invented, and incomplete
inputs cannot issue that proof.
Pending namespaces/specializations remain Incomplete. A nonempty cycle with an
unstable semantic order remains Incomplete. The sealed KerML-only interpretation
is preserved; cyclic proofs require an independently identified language extension.

The real retained-graph diagnosis corrected an earlier assumption: the three
`KQ_END_CYCLE` subjects failed `effective_parameters`, not connection ends. The
actual `effective_connection_ends` API already returned two canonical ends with
Complete KerML answers. On `messages`/`flows`, the real parameter evaluator found
zero owned parameters and nonempty external vectors of length two. The existing
owned-position coverage guard therefore correctly failed; owned end counts
could not prove directed-parameter closure.

A separate finite proof handles an SCC with zero owned positions when every
nonempty external vector agrees in both identity and order. Replaying the
ordinary visibility/redefinition transfer must reproduce that vector at every
member. The original owned-position coverage condition is unchanged. Different
external orders, uncovered local positions and incomplete source populations
retain Incomplete status. The diagnostic now identifies the actual population
and records unresolved component ownership/external counts.

The retained frontier was read only for diagnosis. No old checkpoint was reused
for producer closure or accepted as a current publication. Its missing current
certificate remains visible: six of the 21 result probes retain only producer
closure diagnostics; all 21 no longer report result-cycle diagnostics. Fresh
scheduler and strict publication gates remain the lead's responsibility.

Focused commands executed in this worktree (all `--locked --offline` where
applicable):

| Command | Exit | Relevant output |
| --- | ---: | --- |
| `cargo test -p agq-kerml-semantics --test ordered_results` | 0 | 11 passed |
| `cargo test -p agq-kerml-semantics --lib owned_position_cycles` | 0 | 7 passed |
| `cargo test -p agq-sysml-semantics --lib finalizer_structural_apis` | 0 | 1 passed; real combined producer certificate, effective return/parameter/interface APIs |
| `cargo clippy -p agq-kerml-semantics -p agq-sysml-semantics --all-targets -- -D warnings` | 0 | finished successfully |
| `cargo fmt --all -- --check` | 0 | no differences |
| `cargo build --release --config profile.release.lto=false -p agq-kerml-text --example structural_frontier_probe` | 0 | optimized diagnostic built |
| `structural_frontier_probe ROOT JOURNAL KERML_CACHE` before fix | 0 | real parameter-cycle rejection with owned/external counts; 21 result subjects inspected |
| `structural_frontier_probe ROOT JOURNAL KERML_CACHE` after fix | 0 | all three parameter KerML answers Complete with the two original Message parameters; zero end-cycle or result-cycle diagnostics |
| `cargo clippy -p agq-kerml-text --example structural_frontier_probe -- -D warnings` | 0 | diagnostic example clean |

The journal argument was
`verification/generated/final-language-acceptance/full-frontiers/journal-735481e45453e104dd2bfe5d1e35bdb4ca89974d71b78dce5c51d267adfb6df5.json`;
the cache argument was
`verification/generated/kerml-v9-publication/canonical.publication.zip`.
Both were read from the lead checkout. The executable authenticates stored
archive bytes but intentionally issues no current certificate or publication.

Debug/test builds used `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2` and the shared
`target/platform-finalization` directory. Raw logs live under ignored
`verification/generated/semantic-closure-structural`. The original verbose end
probe output was reduced from 662,940,580 bytes to its unique diagnostic/input
packet after retaining its SHA-256. No supplied authority or library bytes
were changed or removed.

Development failures were corrected before these gates: probe imports/private
snapshot access, a missing pending-source guard exposed by the negative return
fixture, a Windows link lock from a concurrent test executable, and one Clippy
needless-borrow warning. The optimized after-fix probe is recorded in
`real-structural-query-closure.json`. The final return regression command was
repeated after adding explicit population evidence: 11 tests passed, including
proof coverage and absence of a Complete proof for pending inputs. These results
do not claim Systems acceptance.
