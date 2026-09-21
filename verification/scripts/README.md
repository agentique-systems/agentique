# Verification tools

Raw command output belongs under ignored `verification/generated/`. Keep command
arguments, actual exit status, source identity and concise resource observations
in a milestone summary, following [ADR 0021](../../docs/adr/0021-verification-evidence-policy.md).

`run.py` records ordinary commands. `watchdog.py` additionally monitors the whole
process tree, samples working set and private memory, and stops on workflow limits.
These limits never establish semantic success. A nonzero child exit remains a
failure; a workflow stop returns 124 and a monitoring failure returns 125.

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

Only after all publication preflights pass, the lead may run the release binary
with `--wall-seconds 5400 --private-mib 6144`. At most two full-corpus attempts are
allowed in this milestone; a stop requires implementation work before another.
`--progress-pattern` optionally captures named regex groups from a bounded stdout
tail into resource observations. Create the run directory's `stop` file to request
a recorded process-tree termination. Raw logs and samples remain available there.

`publication_slices --slice=A|B|C|D|E` runs an independently scoped closure and
mandatory-reference/capability audit. `--slice=all` shares immutable declaration
preparation across those independent runs. With `--output`, use `{slice}` in the
filename for multiple slices. `--inventory` reports actual dependency populations
without running producers. Scoped reports cannot seal a publication.

Binding regeneration reuses the accepted publication instance:
`canonical_publication --write-bindings` writes and immediately stale-checks the
manifest only after closure, capabilities and every mandatory reference pass.
Construction-only binding inventory cannot overwrite the accepted manifest.
`--conformance-output=...` records separate incomplete coverage cheaply;
`--validate-conformance` explicitly opts into additional validator execution.
