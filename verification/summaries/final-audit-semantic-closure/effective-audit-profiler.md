# Observational effective-audit profiler

`verification/scripts/profile_systems_effective_audit.py` reads the exact
`AuditLog` JSONL format emitted by
`crates/kerml-text/src/sysml/publication_finalization.rs`. It never edits the
journal and never grants semantic or publication authority. No timing threshold
is an acceptance condition.

Run after a completed scoped or full effective audit:

```text
python verification/scripts/profile_systems_effective_audit.py --journal PATH/audit-events.jsonl --output PATH/effective-audit-profile.json
```

The curated JSON pins the complete input bytes by SHA-256. It records worker
policy, completed batch/stage pairs, missing/unmatched batch populations, total
subject count, ordered nonoverlapping endpoint ranges, effective elapsed time,
total/median/nearest-rank-p95/maximum per-batch durations, and the ten slowest
batch ranges. The source log does not contain interior subject identities, so
the profiler expressly limits its population claim to logged counts and ranges.

Full finalization uses the enclosing effective-audit stage duration. Scoped
audits currently emit only batches; their overall duration is explicitly labeled
as the observed first-batch-begin to last-batch-end span. Summed batch durations
are worker elapsed times and need not equal wall elapsed time with two workers.
The recorded worker limit is policy evidence, not independent proof of actual
thread scheduling.

Truncated JSONL, missing or duplicate pairs, mismatched batch metadata, missing
batch indexes, subject count/range drift, unclosed stages and invalid worker
window order fail with exit 1 and an explicit rejected JSON report. Failed
reports omit timing statistics. A report with findings may still be profiled;
this script does not decide whether those findings permit publication. Legacy
journals without batch observations cannot establish the requested performance
profile and fail rather than inventing missing timings.

Verification, 2026-09-24 (isolated enumeration worktree):

```text
python -m unittest discover -s verification/scripts -p test_profile_systems_effective_audit.py -v
test_cli_writes_explicit_failure_and_never_changes_journal ... ok
test_duplicate_and_mismatched_pairs_are_rejected ... ok
test_full_journal_other_stage_pairs_remain_consistent ... ok
test_missing_batch_population_and_overlapping_ranges_are_rejected ... ok
test_missing_end_and_truncated_or_missing_stage_are_rejected ... ok
test_population_workers_and_order_cannot_drift ... ok
test_scoped_serial_span_is_labeled_and_findings_do_not_reject_profile ... ok
test_two_workers_keep_identity_order_and_independent_timings ... ok
Ran 8 tests in 0.290s
OK
exit code: 0
```

The two-worker fixture logs ordered joins even when the second batch is faster.
The CLI regression verifies successful output, failed output replacing a prior
profile, and refusal to overwrite the input journal. No corpus, scheduler or
Rust build was run for this change; actual medium timing remains to be collected.

## Bounded-window timing extension

Based on root `a16fc0e`, the profiler also reports summed per-window maximum and
minimum batch durations, their difference, the observed span outside the summed
maxima, and windows pairing a batch over one second with a batch under one
millisecond. Extrema consider scheduled batches only; a singleton tail or serial
window contributes zero peer imbalance. The occupancy ratio uses configured
workers and observed batch span. It is explicitly a wall-timing capacity proxy,
not measured CPU utilization or serial speedup. The outside-maxima residual
includes imperfect overlap and is not an isolated measurement of overhead.

Exact focused verification command, 2026-09-24:

```text
python -m unittest discover -s verification/scripts -p test_profile_systems_effective_audit.py -v
test_serial_windows_have_zero_peer_imbalance_and_zero_span_has_no_ratio ... ok
test_window_imbalance_uses_batch_timers_and_observed_span_not_cpu_time ... ok
(all eight existing journal-integrity/CLI tests also passed)
Ran 10 tests in 0.275s
OK
exit code: 0
```

The synthetic two-worker fixture proves exact maxima/minima, imbalance, residual,
strict duration thresholds and the proxy formula; a singleton tail, serial mode
and zero recorded span are covered independently.

The combined16 profile was regenerated in the isolated worktree by reading the
lead's unchanged raw journal. SHA-256
`3baebbde98f0666a3652b6903ad0e261854b6aa179a8b6505128ea3e6fab71da` was verified,
and every prior output field was asserted equal before adding window metrics.
Result: 808 windows, summed maximum 1,016,620,485 us, summed minimum 93,033,116 us,
imbalance 923,587,369 us, residual 2,106,300 us, 197 uneven windows, occupancy
proxy 0.5446276751229232. Regeneration exited 0.

Equivalent lead-checkout regeneration command:

```text
python verification/scripts/profile_systems_effective_audit.py --journal verification/generated/final-audit-semantic-closure/combined16/report.effective-audit/audit-events.jsonl --output verification/summaries/final-audit-semantic-closure/combined16-audit-profile.json
```

No scheduler, query, producer or worker policy changed. No Rust build or corpus
run was performed; medium execution continues under its existing implementation.
