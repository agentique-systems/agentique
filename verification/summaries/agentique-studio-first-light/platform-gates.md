# First Light platform gates

These gates remain **not run in First Light** until an accepted runtime bundle
has been installed and authenticated. Absence of accepted publication bytes is
not permission to rebuild publications or substitute a graph fixture. Existing
Phase 2 results remain historical evidence, not First Light acceptance.

All durable gates below create fresh temporary SQLite repositories. None needs
the historical `agentique-dogfood.sqlite`. The two test process environment
variables select the already authenticated installed KerML and Systems cache
files; they are verification adapters, not Studio launch requirements.

Run serially, after bundle installation, with the installed paths assigned to
`AGENTIQUE_KERML_CACHE` and `AGENTIQUE_SYSTEMS_CACHE`. The existing accepted facade
restorers authenticate both publications again inside each test process. Release
LTO is disabled, debug artifacts and incrementality are disabled, and build
parallelism is bounded to two jobs.

The supported gate helper discovers through the runtime CLI, rejects legacy
environment fallback, authenticates the installed bundle before any semantic
gate, and records commands, exits, output hashes and logs. It runs the four
platform gates serially and stops at the first failure:

```powershell
python verification/scripts/first_light_platform.py --runtime-dir <installed-runtime-directory>
```

Omit `--runtime-dir` for the normal per-user location. `--gate compact`,
`--gate http` and `--gate edit-oracle` select focused acceptance;
`--gate scale` first runs its lifecycle prerequisite. The helper does not install,
acquire or rebuild publications. The equivalent individual test commands are:

```powershell
$env:CARGO_PROFILE_RELEASE_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_BUILD_JOBS = '2'

cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-service --features verification --test durable_platform durable_cache_and_failure_authentication -- --exact --ignored --nocapture --test-threads=1

cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-service --features verification --test durable_platform durable_restore_branch_binding_diff_and_cas -- --exact --ignored --nocapture --test-threads=1

cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-http --test vertical durable_project_revision_http_vertical_and_stable_continuation -- --exact --ignored --nocapture --test-threads=1

cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-service --features verification --test durable_platform durable_scale_100_mixed_documents_10_revisions_four_readers -- --exact --ignored --nocapture --test-threads=1
```

The compact-cache gate now explicitly requires `agq-project-semantic-cache/2`.
It compares restored checkpoints, semantic fingerprints and evidence-bearing
queries. Missing, corrupted, stale, unknown-format and absent cache references
must fall back to durable source. Forged validation receipts and missing durable
source must fail; a stored Working revision cannot acquire Validated authority
from a usable cache. Compact /2 was already the write path on fetched main;
legacy /1 remains read-only compatibility. First Light has not yet accepted the
full runtime gate.

The lifecycle gate checks immutable revision binding, branch separation, durable
CAS and source-backed semantic diff before the heavier scale run. The HTTP gate
uses the real Gen2 Axum router and authentic publications, with stable revision
continuations across branch movement. The scale gate exercises 100 mixed
documents across ten authored revisions, two competing R10 writers, four
readers, a fresh child process and retained history. Do not directly invoke
`internal_cold_repository_restore_child`: its parent supplies the exact database
and authenticated expectations.

## Edit latency evidence

`ProjectRevision::compilation_timings()` exposes observational microsecond
measurements outside semantic identities, checkpoints and cache payloads:

- Identity preparation and complete declaration/reference preparation.
- Declared kernel construction, reference refinement and preparatory producers.
- Strict kernel validation and final closure (including certification).
- Final reference checking, strict effective audit and exact edit-delta capture.

The complete source-preparation measurement contains its construction,
refinement and preparatory-producer measurements; do not sum nested measurements.
The total excludes parsing, repository operations, candidate serialization and
UI projection. Final closure currently includes scheduler and certificate work;
these are not claimed as separate measurements. The existing exact full oracle
prints incremental and full phase timings along with counters:

```powershell
cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-workspace --features verification --test phase2_checkpoint local_edit_classes_match_full_semantic_oracle -- --exact --ignored --nocapture --test-threads=1
```

This oracle includes adding a nested PartUsage and compares canonical records,
derived facts, references, effective audit, identities, query completeness and
evidence, and closure identity. It remains an authored fixture correctness oracle;
only the real Studio operator sequence can establish product acceptance.

No fast-path speedup is claimed without accepted-runtime measurements. The
current effective audit retains context, subjects and report, but does not retain
all successful queries' read dependencies, negative/provider searches and
potential-writer footprints. Unchanged Element records therefore cannot justify
audit reuse. No audit bypass or unproved reuse was introduced.
