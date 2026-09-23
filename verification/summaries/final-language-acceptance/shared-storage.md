# Held shared standard storage

Status: implemented and kernel-verified on the isolated held branch. This is not
foundation readiness, an accepted Systems publication, or workspace acceptance.
The lead retains the H?K integration gate.

Canonical records, occurrence tables, computed navigation/status/search tables,
ordered contribution tables and identity reservations now retain immutable base
tables with local deltas. Class, incidence and incoming/outgoing indexes borrow
immutable base indexes and merge deterministic local results. Explanation lookup,
interning, proof adjacency and search interning retain the same base table chain.
Accepted dependency graph entries are not copied into each authored revision.

A local unordered association can shift an inherited occurrence's enumeration
position. Only its touched source/property groups are reprojected, including
incoming buckets at other targets. The independent flattened-input oracle checks
canonical records, proofs, searches, navigation, occurrence positions, incidence,
class populations and per-property incoming results before/after local removals.
The accepted dependency and earlier revision remain unchanged. Sparse local
projections are measured separately from authoritative duplicated rows.

The `verification` feature exposes `agq_kernel::storage_observer` with actual
physical table allocation tokens. It inspects the retained declared and effective
graphs, proof/search pools and reservation tables; shared allocations are counted
once. Its own counterexample detects a deliberately flattened/copied table.
Nested publication tests require exact inherited allocation sets and zero copied
dependency entries for snapshots, construction views, construction overlays,
strict overlays and multiple edits/proof frontiers. No test-only zero stub is used.

Compatible registry mounting keeps read-only required-navigation checks: a new
association-owned required end can constrain existing instances even when the
class-owned effective properties are unchanged. A permanent counterexample
ensures the shared mount rejects such an invalid graph atomically.

Search metadata submission now uses the same immutable-dependency write guard as
canonical writes. Previously such a submission could shadow accepted search
metadata or invalidate inherited contribution precision. The regression verifies
atomic rejection and unchanged accepted evidence.

Archive field layout and deterministic serialization are unchanged. Dependent
archive restore borrows the supplied accepted graph/proof/search tables; the
projection oracle requires byte-identical dependent archive write/restore/write.
Kernel neutral archives still carry no language-publication acceptance.

Verification used one build job, debug information disabled, incremental disabled,
and the coordinated `target/bridge-closure` directory. The full kernel run passed
127 tests and doctests in 24.875s before the final observer/registry tests; those
final focused checks passed independently. Package all-target Clippy with denied
warnings, strict Rustdoc, and workspace formatting checks passed. Exact commands,
exits, durations and retained-log SHA-256 are in `shared-storage-commands.json`.
An unrelated baseline rustfmt change in `accepted_self_model_tests.rs` is excluded
from this implementation commit.

Observed intermediate compilation failures were corrected before these passes:
container migration type/iterator errors; one proof-graph test BTreeMap type;
new test used nonexistent StructuralSearch::Class (changed to Model); extracted
required-navigation function needed the qualified model::Validation type.

No standard publication run, SCC planning, or language-rule change occurred in
this held workstream. Shared storage does not imply incremental authored semantic
recomputation; local authored populations are still recomputed under the existing
closure contract.

## Integrated held verification

Resolved the shared tables with authenticated frontier commits 2953a66/bf02cf8
and declared-history commit 5fbe883. Strict declared revalidation passes the
existing immutable index base explicitly. Construction and strict frontier
archives preserve shared dependency tables and exact contribution metadata.

The integrated kernel suite passed 136 tests/doctests in 27.781s. All-target
Clippy with warnings denied passed in 5.672s, strict Rustdoc in 3.422s, and workspace
formatting in 2.703s. Actual logs/hashes are appended to the command ledger.
The initial integrated compile correctly reported the removed ModelView::build
helper at history revalidation; the explicit strict/base-index call fixed it.
