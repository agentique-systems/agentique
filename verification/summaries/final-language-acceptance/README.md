# Final language acceptance and modeling workspace

Started from a clean worktree after fetching `origin/main` at
`7528a97762c7d808fcac8e1e7bd772bcba9638ff`. Integration branch:
`foundation/final-language-acceptance-and-modeling-workspace`. Scheduler,
incremental certificate and checkpoint work use separate worktrees and branches.
The authority remains pinned KerML 1.0 / SysML 2.0. Accepted KerML Operational v9
and generation-1 release obligations are unchanged.

## Decision and workflow

The current acceptance authority is the monolithic dependency-driven fixed point.
ADR 0027 remains proposed research, separate from ADR 0026's adoption gate. Its
component planner, semantic comparator, boundary audits and invalid-partition
tests are retained. Broad current read-side footprints do not prove Systems is
semantically indivisible. No new Systems SCC planning run is part of this work.

One new full attempt is authorized only after the 13-document medium candidate
passes exact uninterrupted/resumed equivalence and retains its previously
accepted semantic identities. Both paths must close all 695 references, have no
kernel obligations, converge Complete and carry a fully closed certificate.
Incremental certificate maintenance retains the full rebuild oracle. A frontier
checkpoint resumes computation and conveys no accepted publication authority.

The full attempt has a 3,600-second wall budget, 6,656 MiB private-memory limit,
and 1,024 MiB disk reserve. The watchdog aborts after 600 continuous seconds
without a completed frontier, changed closed-pair count or changed planned/output
population. Repeated messages, timestamps and evaluated-subject counters do not
reset this clock. No competing heavy Rust build runs during full publication.
External interruption may resume an authenticated checkpoint; a semantic defect
invalidates it. The [medium prerequisite](phase-f.md) passed, including exact
uninterrupted/resumed artifact equivalence. The single full attempt is now
running with the same pinned executable; `publication-commands.json` will retain
its completed watchdog record. This is an execution status, not acceptance.

Strict Systems acceptance still requires 21/21 exact parses and constructions,
zero kernel obligations, converged Complete producers, a fully closed certificate,
all 1,327 mandatory references Complete with no failure category, zero authority
or capability findings, and exact identity/provenance. Only the accepted facade
can issue bindings and a cache. Only actual accepted publication, self-model and
effective API gates can adopt ADR 0026 and integrate the production workspace.

## Measurements and verification

The historical medium certificate timing was cumulative. The reported late
~100-second value was not a single certificate rebuild. Consecutive retained late
frontiers add approximately 2.9–5.3 seconds to that counter; their callbacks also
preceded certificate issuance, so those observations cannot attribute a duration
to the same displayed frontier. New profiling separates those costs.

Actual command arguments, source/patch identities, output hashes and exit codes
are recorded in `commands.json`; raw output remains under ignored
`verification/generated/final-language-acceptance/`. Agent ledgers retain their
own source identities. Finished commands and prepared fixtures do not establish
publication or foundation acceptance.
