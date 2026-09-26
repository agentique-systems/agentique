# Independent effective-audit answer parity result review

Reviewed actual run `effective-audit-answer-parity02`, 2026-09-26. No build,
test or accepted-runtime consumer was started by the reviewer.

## Actual result and artifact identity

The exact ignored test
`sysml::publication::authored_audit_parity_tests::authored_audit_observer_matches_two_pass_queries_and_diagnostics`
passed: one passed, zero failed, zero ignored, 66 filtered out. Test-harness
elapsed time was **191.22 seconds**. The process measurement was **192.033698
seconds**, with maximum process peak working set **6,710,374,400 bytes**
(6.2495 GiB). The sampled process-tree resident peak was 6,575,726,592 bytes
at a 250 ms interval; it is a separate measurement, not interchangeable with
the operating-system process peak.

The command receipt records exit 0, invocation source
`1b5075dc357b93d34d16e6dd97b783291c986b15`, a clean tracked worktree and start
time `2026-09-25T22:18:35.036456+00:00`. The parent reports this run was the sole
runtime consumer, with no concurrent build. Review did not independently record
the process schedule.

Independent read-only SHA-256 checks passed for:

| Artifact | SHA-256 |
| --- | --- |
| Actual test executable | `9c02844a2199e7ddaf10f9f5a8dcff4382d8aa619651165c574baa873a11db77` |
| Successful command output | `a14a84bcf9c08511bba0ce2b468370614627301c18076f6125bf5f18fc268dc4` |
| Test-only performance log | `0685320554cdb80eb579ed558d926ff557a9abf36bc78a31e6b48fdf429c6179` |
| Build02 command output | `ec8c1f93ce9cd79a66e451fccf392788637bb612659339f4d8998d1c9d999d58` |
| Retained failed first-run output | `4ee06ee4d645c9eecddf4f12d50a2248d9258b0d928dd78841911cb5191435ec` |

The executable and command-output digests exactly match their receipts. The
source comparison from build commit `0bd685c` to invocation commit `1b5075dc`
found no change in kerml-text, KerML/SysML queries, the source fixture or runtime
transport catalogue. Read-only verification exited 0.

## What passed

The test requires the actual accepted runtime caches and restores them through
the ordinary KerML and Systems facades. It compiles both the ordinary Choices
source and the variation source using accepted SourceInputs. The two cases
exercise actual Complete and Incomplete effective-usage answers. Existing
wrong-kind and missing-identity probes exercise Invalid answers; the missing
probe also requires the original SQ_MISSING_ELEMENT diagnostic and composed
KerML Invalid status.

For each authored population the baseline executes the former two-pass order:
audit each 32-subject batch, then query effective usages again for each real
Definition or Usage. The observer path receives the first query's immutable
answer instead. Both use fresh evaluator forks over the same bound context.
Assertions compare the full Debug representation, including context, canonical
IDs, completeness, pending implications, dependencies, evidence, diagnostics
and private additional-completeness state. No revision or identity normalization
is used. Callback subject/order, capability diagnostics, complete audit reports,
and capability-then-audit diagnostic ordering must match exactly. The test also
compares against the actual stored SourceCompilation report and diagnostics.

The direct delivery probes require exactly one callback and unchanged family
check/finding behavior for Complete and non-Complete results. The new observer
accepts an immutable answer reference and cannot mutate that delivered value.
Production dispatcher and query semantics were source-reviewed separately.

## Qualification limits

This establishes answer-delivery equivalence for the exercised accepted-runtime
contexts and the source integration path. It is not a benchmark comparing the
old and new audit implementations, an independent old executable, a proof for
every authored model, or the full CreatePart reconstruction oracle. No native
frame-rate, p95, input latency, or overall alpha acceptance follows from it.

The first run stopped at an incorrect missing-subject expected status and did
not reach the variation iteration. Its exit-101 evidence remains retained.
Correction `0bd685c` changes that expectation to the unchanged public Invalid
contract and adds exact diagnostic assertions; it does not relax production
completeness. Comparing the aborted 127-second run with this complete run would
not be a valid performance comparison.

The forthcoming combined CreatePart/full-reconstruction oracle must be reviewed
separately for exact candidate identity, durable reconstruction, work retained
and wall time. No pin or accepted receipt update is authorized by this result.
