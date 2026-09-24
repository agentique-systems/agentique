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
