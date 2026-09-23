# Systems finalization

The retained scheduler frontier is authenticated; it has not been replayed.
The first read-only finalization attempt finished with a semantic rejection,
not a watchdog stop. No accepted artifact bundle was promoted.

## Authenticated input

[authentication.json](authentication.json) records the independently checked
journal, original executable, source session, all original source bytes, graph,
certificate and selected proof/search identities. The retained round 28 frontier
has an empty worklist, 26,532/26,532 closed applicable producer pairs, 452,052
closed requirements, 75,342 certificate subjects and 87 producer families.
Its journal SHA-256 is
`735481e45453e104dd2bfe5d1e35bdb4ca89974d71b78dce5c51d267adfb6df5`.
Authentication alone does not issue publication authority.

## First finalization attempt

[watchdog.json](watchdog.json) records the actual executable invocation and
exit code 1 after 1,632.078 seconds. Peak process-tree private memory was
6,040,027,136 bytes, below the 6.5 GiB limit. The watchdog did not stop the run.
[executable.json](executable.json) pins the tested finalizer executable.

Frontier/graph reconstruction, registry and closure verification, KerML
capabilities, derived provenance, authority and Systems bindings passed with
zero findings. The independent mandatory-reference audit completed
**1,327/1,327** references. The effective SysML population audit then completed
in 1,131.739659 seconds and reported 674 family-weighted findings.

[rejection.json](rejection.json) records their concise classification: 238
distinct diagnostics across 50 subjects, involving variation implications,
cyclic result/end inheritance and narrowed typing/interface-end domains.
The raw diagnostic output and stage observations remain in ignored generated
storage. The [enumeration diagnosis](enumeration-variation-diagnosis.md) identifies
missing canonical owner typing that cannot be supplied by read-only finalization.
The [typing/interface diagnosis](effective-audit-applicability.md) and
[cycle diagnosis](cycle-diagnosis.md) separate additional graph incompatibilities
from query defects and state the limits of their static analysis. None of these
findings is waived by producer convergence.

## Implemented boundary

[api-focused-checks.md](api-focused-checks.md) and
[artifact-issuance.md](artifact-issuance.md) describe the authenticated direct
finalizer and transactional candidate issuance. All read-only audits must pass
before a live accepted facade can authenticate a candidate cache. Cache,
receipt, bindings and report become visible together through one fresh-directory
promotion. Candidate files and individual audit checkpoints convey no authority.

The accepted Systems catalogue, Operational default, ADR 0026 adoption and
production workspace integration remain gated on actual acceptance. KerML v9
and generation-1 release obligations remain unchanged. ADR 0027 remains proposed.

## Implementation verification

[commands.json](commands.json) records actual commands, tested source identities,
exit codes and hashes of raw generated logs. The final workspace run passed
923 tests with zero failures and five explicitly ignored acceptance/large-input
tests across 126 test groups. The separate publication example passed all five
CLI and atomic-promotion tests. The new evidence-sharing equality, deterministic
parallel merge, worker-panic and Viewpoint frontend regressions passed.

Formatting, workspace Clippy with warnings denied, strict Rustdoc for all seven
Gen2 language/kernel crates, metamodel stale verification, both runtime gates,
both grammar stale gates, the generation boundary, Node tests and standards
checks passed. Frontend TypeScript checking and the production build passed in
the [workspace preparation command ledger](../modeling-workspace-phase1/commands.json).
Browser behavior did not change, so browser tests were not run, following this
milestone's explicit verification scope.

One redundant optimized unit-test compilation was stopped after the equivalent
six public-search regressions had passed in the full workspace run. Its actual
exit 101 is retained as `public-search-evidence-equivalence`; it is not reported
as a passing command or as a test assertion failure. The optimized publication
executable used for the measured rejection was built successfully and remains
separately pinned.

The original Gen1 coverage/traceability bytes and accepted KerML receipt/bindings
match their retained pre-task hashes. Standards checks preserve the supplied
authority/library bytes. The reproducible enumeration diagnosis reauthenticated
the original frontier archive and decoded graph hashes. No accepted Systems
cache restoration, self-model acceptance or production workspace acceptance is
claimed by these implementation checks.
