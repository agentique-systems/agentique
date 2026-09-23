# Held workspace follow-up and final run order

This is a run plan, not an execution report. Run it after H–K pass and the held
production changes are integrated. `cb84f64` and `60bfa21` are now consolidated on
`7223938` in `foundation/workspace-authenticated-frontier-held`; apply that single
consolidated commit, not both historical inputs again. Serialize all
Cargo commands; do not overlap them with medium/full publication. All commands
run from the final integration checkout and record actual output/exit status
through `verification/scripts/run.py` or `low_artifact.py`.

## Exact cache inputs and resource settings

The existing accepted KerML cache is
`C:/Users/phili/github/agentique-systems/agentique/verification/generated/kerml-v9-publication/canonical.publication.zip`.
There is no accepted Systems cache yet. Parameterize `$systemsReportPath` with
the successful full publication's actual report path, and obtain its cache from
the report's `exported_cache` field. The publication exporter writes
`canonical.publication.zip` beside that report. A medium report/frontier is not
a substitute. Tests independently require the activated compiled receipt
`sysml-systems-operational-v2` and authenticate both original caches.

```powershell
$env:AGENTIQUE_KERML_CACHE = 'C:/Users/phili/github/agentique-systems/agentique/verification/generated/kerml-v9-publication/canonical.publication.zip'
$acceptedReport = Get-Content -Raw -LiteralPath $systemsReportPath | ConvertFrom-Json
if ($acceptedReport.publication_accepted -ne $true) { throw 'The full report is not accepted' }
$env:AGENTIQUE_SYSTEMS_CACHE = (Resolve-Path -LiteralPath $acceptedReport.exported_cache).Path
$env:CARGO_TARGET_DIR = 'C:/Users/phili/github/agentique-systems/agentique/target/foundation-integration'
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
```

Retain the wrapper's disk preflight (at least 4096 MiB free for final workspace
builds); it must not silently lower reserves or remove publication evidence.

## Serialized order

1. Check the final source boundary and formatting:
   `cargo fmt --all -- --check` and
   `python verification/scripts/generation_boundary.py`.
   The boundary check includes normal/dev/build/optional/target-specific edges;
   the earlier normal-only Cargo tree observation does not replace it.
2. Compile and lint the final feature combination:
   `cargo clippy --locked --offline --workspace --all-targets --features agq-modeling-workspace/verification -- -D warnings`.
   This also type-checks the strengthened held tests before cache loading.
3. Run the relevant package groups once on the final integration:
   `cargo test --locked --offline -p agq-kernel -p agq-kerml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-sysml -p agq-sysml-semantics -p agq-standard-libraries --features agq-kerml-text/verification -- --test-threads=1`.
   Default ignored gates remain ignored here; do not use a blanket `--ignored`
   on the language packages.
4. Check accepted Systems restore against the integrated shared-storage kernel:
   `cargo test --locked --offline -p agq-kerml-text --test accepted_systems_cache accepted_systems_cache_roundtrip_and_tampering -- --ignored --exact --test-threads=1 --nocapture`.
   This is a restore/roundtrip/tampering gate, without producer publication replay.
5. Run the three held workspace targets in this order, each in its own recorded
   command:
   `cargo test --locked --offline -p agq-modeling-workspace --features verification --test working_states -- --ignored --test-threads=1 --nocapture`,
   then the same command with `--test self_model`, then `--test phase1`.
   These execute all ten acceptance tests. The last target includes the
   100-document/five-revision/four-reader fixture and emits size/counter/timing
   observations. Each test binary restores the accepted caches once via OnceLock.
6. Run strict Rustdoc using `RUSTDOCFLAGS=-D warnings`:
   `cargo doc --locked --offline --no-deps -p agq-kernel -p agq-kerml -p agq-kerml-semantics -p agq-kerml-syntax -p agq-kerml-text -p agq-sysml -p agq-sysml-semantics -p agq-modeling-workspace --features agq-modeling-workspace/verification`.
7. Near completion, run one low-artifact full workspace test:
   `python verification/scripts/low_artifact.py --name final-workspace-tests --summary verification/summaries/final-language-acceptance/final-verification.json --target target/foundation-integration --min-free-mib 4096 --env CARGO_BUILD_JOBS=1 -- cargo test --locked --offline --workspace --features agq-modeling-workspace/verification -- --test-threads=1`.
   This preserves ordinary debug assertions/overflow checks and does not repeat
   the explicitly ignored accepted-cache tests.

The H–K language self-model/effective API command is
`cargo test --locked --offline -p agq-kerml-text --lib accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics -- --ignored --test-threads=1 --nocapture`.
Reuse its passing final-revision evidence if its implementation/dependencies are
unchanged; rerun once after shared-storage integration if H–K ran on the earlier
kernel. Independent traceability has its own existing passed test filter
`agentique_implementation_traceability_is_separate`.

## Existing evidence to retain

`commands.json` records successful `npm run check`, `npm run build`, `npm test`,
`npm run standards:check`, both grammar stale gates, the metamodel stale gate,
KerML runtime gate and SysML runtime gate. `frontend-commands.json` records the
successful browser run. Reuse unchanged frontend/browser evidence; there is no
browser-facing change in the held workspace. Re-run a gate only if its checked
inputs changed during H–K or final integration. Exact commands for retained
generator gates are:

```text
python tools/kerml-grammar/generate.py --check
python tools/sysml-grammar/generate.py
cargo run --locked --offline -p agq-metamodel-gen -- --check
cargo run --locked --offline -p agq-metamodel-gen -- --check --baseline kerml-1.0 --require-runtime
cargo run --locked --offline -p agq-metamodel-gen -- --check --baseline sysml-2.0 --require-runtime
```

The SysML grammar generator checks by default; `--write` is deliberately absent.
Do not invoke the broad `npm run verify` wrapper, which would repeat browser,
build and unrelated pilot/process work. Preserve generation-1 evidence separately.

The held `cb84f64` integration already passed 49 focused kernel tests, 21 frontend
tests, feature-enabled Clippy and formatting, and compiled all ten workspace
tests. Its accepted-cache tests were never executed. These facts do not establish
acceptance of the test additions below or replace the final required workspace run.

## Coverage audit and test-only follow-up

Physical sharing checks already inspect actual retained graph/index/reservation/
proof/search/contribution tables, and zero copied dependency rows; pointer checks
alone are not their authority. Working and independent-workspace cases are
covered. Validated construction is private and checks current strict state,
diagnostics, reference results, convergence and exact certificate/context closure;
no unchecked constructor or prior-graph fallback was found.

Three gaps are strengthened by this follow-up:

- A nonzero scheduler evaluation count now requires a nonempty observed subject
  set, preventing a disconnected observer from passing the no-replay assertion
  vacuously.
- Both self-model revisions assert the ModelingPlatform workspace dependency,
  accepted-publication/language-query targets and ExecutionCompiler's validation
  service dependency, as well as the original inward language dependencies.
  Exact Agentique, LanguageEngine and ExecutionSubsystem compositions and exact
  ProjectWorkspace/ExecutionCompiler reference dependency sets match the native
  language acceptance assertions prepared in `639736a`.
- The accepted Working test explicitly deletes the retained declaration while
  its source document remains recovered, then repairs/re-adds it and requires a
  fresh syntax/semantic identity with unchanged retained revisions.

No production code changed. No Cargo build or test was run for these additions
during the corpus pause. The only executed source checks were
`rustfmt --edition 2024 crates/modeling-workspace/tests/support/mod.rs crates/modeling-workspace/tests/self_model.rs crates/modeling-workspace/tests/working_states.rs`
and `git diff --check`; both exited 0. Their runtime and compile obligations are
the final Clippy and workspace acceptance commands above.
