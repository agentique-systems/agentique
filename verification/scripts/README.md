# Verification tools

Raw command output belongs under ignored `verification/generated/`. Keep command
arguments, actual exit status, source identity and concise resource observations
in a milestone summary, following [ADR 0021](../../docs/adr/0021-verification-evidence-policy.md).

`run.py` records ordinary commands. `watchdog.py` additionally monitors the whole
process tree, samples working set and private memory, and stops on workflow limits.
These limits never establish semantic success. A nonzero child exit remains a
failure; a workflow stop returns 124 and a monitoring failure returns 125.

`low_artifact.py` wraps `run.py` with a disk preflight and the low-artifact Cargo
environment. It records the resolved target directory, current target size and
free bytes under generated evidence, refuses builds below its reserve, and never
deletes caches. Its default target is `target/foundation-integration`; normal developer Cargo
profiles remain unchanged. For example:

```powershell
python verification/scripts/low_artifact.py --name workspace-tests -- cargo test --workspace
```

Isolated worktrees must use distinct target directories (the wrapper's `--target`
argument). A shared Cargo lock serializes builds but does not make one worktree's
artifacts suitable as another worktree's verification evidence.

The language-closure milestone's full Systems attempt uses the watchdog with
`--wall-seconds 2100 --private-mib 6656 --min-free-mib 1024`. Run the targeted
Actions and authority slices first. The example's `--documents=Actions,Connections,Constraints,Flows,Items,Parts,Ports,States`
and `--audit-only` arguments select exact pinned documents by stem; all transitive source and
implicit semantic dependencies must be included. The report distinguishes a
scoped preflight from canonical acceptance, and a restricted selection cannot
publish the Systems Library.

On Windows the command starts suspended, joins a kill-on-close Job, then resumes.
This avoids the packaged-Python launcher escape discovered during this milestone.
Native descendants are tested, including termination with no surviving processes.
On Linux a new process group is monitored through `/proc`. Windows records the
Job's peak private memory as well as sampled working set; Linux memory peaks are
sampled observations. RSS can double-count shared pages across processes.

Example focused check:

```powershell
python verification/scripts/watchdog.py --name kernel-tests --wall-seconds 600 --env CARGO_BUILD_JOBS=2 -- cargo test --locked --offline -j 2 -p agq-kernel
```

Release verification uses `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_RELEASE_DEBUG=0`,
`CARGO_PROFILE_RELEASE_INCREMENTAL=false` and `CARGO_PROFILE_RELEASE_LTO=false`
to bound compilation concurrency and avoid unneeded verification artifacts.
Optimization remains enabled. Commands and overrides are retained in the summary.

For workspace tests on disk-constrained Windows hosts, use
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_INCREMENTAL=0`.
These disable debug symbols and incremental caches; optimization levels, debug
assertions and overflow checks retain their normal test-profile defaults.
`--min-free-mib 1024` adds an optional workspace disk reserve to the watchdog.
Crossing it terminates the command tree with a recorded `disk_space` stop; the
wrapper never removes files to recover space.

Only after all publication preflights pass, the lead may run the release binary
under the milestone's limits above. This Systems milestone permits one full
attempt and one retry only after fixing a concrete defect found in that attempt.
`--progress-pattern` optionally captures named regex groups from a bounded stdout
tail into resource observations. Create the run directory's `stop` file to request
a recorded process-tree termination. Raw logs and samples remain available there.

`publication_slices --slice=A|B|C|D|E` runs an independently scoped closure and
mandatory-reference/capability audit. `--slice=all` shares immutable declaration
preparation across those independent runs. With `--output`, use `{slice}` in the
filename for multiple slices. `--inventory` reports actual dependency populations
without running producers. Scoped reports cannot seal a publication.
Comma-separated selections such as `--slice=A,B` share preparation for a bounded
group. `--fail-fast` stops after its first failed slice so a shared defect can be
fixed before spending time on the remaining groups.
Every completed producer frontier is immediately written to an adjacent generated
`.stages.jsonl` file, preserving diagnostics across watchdog stops. An incomplete
producer fixed point produces a failed report before capability/reference audits;
those skipped audits are explicitly marked `not_run`.

Binding regeneration reuses the accepted publication instance:
`canonical_publication --write-bindings` writes and immediately stale-checks the
manifest only after closure, capabilities and every mandatory reference pass.
Construction-only binding inventory cannot overwrite the accepted manifest.
`--conformance-output=...` records separate incomplete coverage cheaply;
`--validate-conformance` explicitly opts into additional validator execution.
