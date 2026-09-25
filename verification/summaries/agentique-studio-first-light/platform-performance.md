# First Light platform and performance status

Accepted runtime bytes were unavailable during this workstream. No accepted
publication was rebuilt, and no real-cache platform gate was started. All current
product performance categories remain **unmeasured**: cold/warm launch,
publication restore, durable project/cache restore, first real view, interactive
visual reads, candidate preparation, validation and commit. Historical Phase 2
measurements are not rerun results.

| Required platform gate | First Light result |
| --- | --- |
| Compact cache restoration and failure authentication | Not run; missing accepted runtime bytes |
| Real Gen2 HTTP and stable continuation | Not run; missing accepted runtime bytes |
| 100-document durable semantic scale | Not run; missing accepted runtime bytes |
| Incremental/full reconstruction comparison | Not run; missing accepted runtime bytes |

The cache implementation already writes compact format `/2` and retains `/1`
decoding. Three focused codec tests passed, including deterministic exact
roundtrip and invalid archive/digest rejection. This establishes codec behavior,
not semantic restoration acceptance. The ignored real gate now explicitly asserts
that its persisted cache is `/2` before testing authenticated restoration and
source fallback.

The previous full-versus-incremental times do not identify the current dominant
phase. New observational timings expose construction, reference refinement,
preparatory producers, final kernel validation, final closure/certification,
closed-reference checks, effective audit and edit-delta capture. They accompany
the existing exact oracle and durable-restoration logs. No semantic computation,
identity, receipt, cache authority or validation predicate was changed.

Effective audit reuse is not currently justified: `SourceEffectiveAudit` retains
the full context, local subjects and findings but not the complete successful
query read sets, negative/provider searches and potential-writer footprints.
Unchanged Element records cannot establish unaffected semantic answers. The
strict local-subject audit remains mandatory. No latency improvement or safe
bounded-invalidation path is claimed.

Focused compile checks for the workspace and service (all targets, verification
feature) passed. Focused Clippy across `agq-kerml-text`,
`agq-modeling-workspace` and `agq-modeling-service` passed with warnings denied.
Three orchestration tests verify that development paths are rejected, failed
bundle authentication stops all gates, and the scale gate uses installed paths
after its lifecycle prerequisite. These are harness tests, not semantic gates.
Exact commands, exits and output hashes are in `platform-commands.json`;
two early concurrent-manifest/lockfile preparation failures are retained in
`platform-preparation.json`.

After authenticated installation, run
`python verification/scripts/first_light_platform.py` (or pass `--runtime-dir`).
It discovers the installed bundle using the runtime CLI, authenticates it before
running gates, supplies its exact cache paths to existing test adapters, and
records each serial process. `platform-gates.md` documents the individual
commands and timing boundaries. None of these gates requires the old Phase 2
SQLite fixture.
