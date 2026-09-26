# Call-local view query reuse: qualification pending

This patch targets two measured repeated context constructions, without changing
language inputs, publication pins, closure, audit or repository authority. Real
run04's ModelingPlatform projection took 5,582.54 ms, including 5,316.88 ms in
two KerML context constructions. Its Inspector took 5,573.84 ms, including
5,306.90 ms in separate KerML/SysML contexts. These are baseline observations,
not measurements of this patch.

Projection retains its first ordinary KerML evaluator until the call returns,
and reuses it for focused interfaces. Construction remains lazy; query order,
warnings and early version/model/focus errors are unchanged. Inspector first
constructs the ordinary checked SysML evaluator and borrows its contained KerML
evaluator. A failed SysML factory still falls back to the ordinary KerML factory,
retaining the original KerML failure if both fail and the baseline profile if
only SysML fails. No evaluator or query cache survives the call.

The old `project_observed` and `inspect_observed` function bodies are preserved
under `crates/modeling-view/src/query_reuse/`, from commit
`d404a8759891acc5d621dcad655fa4a981e17d63`, blobs
`af7e104dae64a3bdff47e564441a001a2891926e` and
`df8ae3617e355e6db9497d6d94a69e2eda768a10`. They use the same unchanged helpers.
Both old/new oracle wrappers disable optional profiling to measure equal work.

The ignored oracle compares 17 full projection cases and eight full Inspector
cases, including error variants/payloads and serialization. It independently
compares raw query contexts, values, completeness, diagnostics, dependencies,
origins and explanations; full derived Debug also compares private producer and
shared-search evidence. **No context, revision or identity normalization occurs.**
Actual accepted-model factories are required; fixture tests establish only the
fallback dispatch and incomplete/invalid raw-query evidence boundaries.

The accepted baseline is Validated and exercises successful SysML construction.
Missing-ID and unsupported-version cases return before context construction;
they do **not** qualify SysML-factory failure. The failed-SysML/raw-KerML fallback
and both-factories-fail error priority have a private dispatch regression and
source review only. Actual Working-revision runtime fallback parity remains
unqualified; no fake semantic context is presented as that evidence.

Three alternating old/new full-call timing pairs follow correctness warm-up.
Three separately rotated context/cache trials distinguish fresh context, forked
context with fresh evaluator caches, and reuse of the connector-primed evaluator.
Construction/fork and focused-query intervals are separate; constructor absence
is `null`, never an invented zero-time measurement. These are serial wall times
on one restored revision, not disk-cold load, UI latency or a p95 estimate. Peak
memory from the whole process includes old/new DTOs and all qualification work.

## Inputs and commands for the integration worktree

The lead prepared a read-only SQLite backup from the original database and its
committed WAL. The original DB/WAL hashes did not change. The snapshot receipt
is `performance/view-query-baseline-snapshot.json`; snapshot SHA-256 is
`66c9f29a02203b9ffdf6f89608040409e98f152ee81f71d9802802f46a653a20`.
Do not copy the original main DB alone: it has a nonempty WAL. The oracle accepts
the closed backup below, refuses nonempty WAL/journal or unknown sidecar state,
copies it to a fresh temporary repository, then uses ordinary authenticated
runtime and durable revision restoration. It requires the exact Validated
baseline revision; it neither seeds nor commits a model.

Run only when allocated the sole runtime slot. Use the build/measurement helpers
in [held-reconstruction-qualification.md](held-reconstruction-qualification.md),
whose initial environment selects the ordinary installed accepted cache files.

```powershell
$env:AGENTIQUE_VIEW_ORACLE_DATABASE = 'C:\Users\phili\github\agentique-systems\agentique\verification\generated\native-studio-alpha\view-query-oracle\baseline.sqlite'
$env:AGENTIQUE_VIEW_ORACLE_PROJECT = 'c20c4a5c-ea43-45ea-89d2-db2742e2c20a'
$env:AGENTIQUE_VIEW_ORACLE_REVISION = '8dcfb224-55e6-4d0d-ac3a-84fcfffb9a46'
$queryStem = 'verification/native-studio-alpha/performance/view-query-reuse'
$queryExe = Build-QualificationTest "$queryStem" 'agq_modeling_view' @('-p', 'agq-modeling-view', '--lib')
& $queryExe --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'Modeling-view non-runtime regressions failed' }
Run-QualificationTest $queryStem $queryExe 'query_reuse_oracle::accepted_same_revision_call_local_query_reuse_matches_full_results'
```

The ignored gate is deliberately not run by the preceding ordinary test command.
Inspect the retained output for the exact 17/eight case summary and distinct
`agentique-view-query-reuse-pair/1` and `agentique-view-context-cache-arms/1`
records. Stop on any equality or factory failure. Separately qualify the native
journey and actual first/repeat Inspector latency; neither this gate nor the
bounded reader cache removes time spent waiting for another queued read.

## Source-only checks performed here

- `cargo metadata --locked --offline --format-version 1` initially failed with
  exit 101 because the new test dependencies needed lock entries.
- `cargo metadata --offline --format-version 1` passed (exit 0), changing only
  the three test-dependency names in modeling-view's existing lock entry.
- Full root and native-workspace `cargo metadata --locked --offline
  --format-version 1` then both passed (exit 0). Native lock was unchanged.
- Source formatting, diff whitespace and exact preserved-body checks passed
  (exit 0); both baseline bodies match byte-for-byte after newline normalization.
  No compile, test, runtime restore or new timing has been performed in this
  isolated worktree for this patch.

No accepted-input freshness pins need updating: production changes are confined
to modeling-view. The separate held language mount/audit patches retain their
independent review and qualification obligations.
