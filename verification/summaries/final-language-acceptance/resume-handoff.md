# Exact declared-input handoff after frontier restoration

The real medium resume restored completed construction invocation 0, round 13,
then failed `LibraryDraft::set_semantic_candidate`'s required
`Arc::ptr_eq(overlay.declared_shared(), &self.candidate)` assertion. The failure
log remains at `verification/generated/overnight-convergence/foundation-medium-resumed/output.log`.
The archive reader had reconstructed an independent declared candidate instead
of attaching the restored overlay to the caller's original candidate.

New kernel `read_construction_frontier_on` and `read_publication_frontier_on`
readers authenticate the decoded declared input against the supplied input:
canonical records and slot provenance, association occurrences, navigation,
statuses, searches, selected contribution support, descriptor registry,
reserved element/occurrence identities, immutable dependency allocation, and
construction obligations. Only after an exact match does decoding adopt the
supplied transaction handle. Full existing archive structural, monotonicity,
ownership, proof-dependency, search and cycle validation then runs against that
input. Both the overlay and its model's declared-source pointer retain it.

A fresh revision allocation during source reconstruction is allowed only after
this exact input authentication. The immutable supplied transaction retains its
identity. The frontend pointer invariant is unchanged. No language rule,
scheduler computation, closure certificate, archive encoding, checkpoint source
identity or authentication pin changed. Existing pinned checkpoints remain
usable subject to all existing authentication checks and the new exact input
check; this repair does not invalidate their semantic state or grant acceptance.

Regression coverage includes interrupted Structural construction continuation,
completed construction invocation restoration with no producer replay, strict
continuation and completed invocation restoration. Freshly reconstructed inputs
must retain their exact caller handles. Queries, graph facts, ordered support,
searches and certificates retain exact uninterrupted semantics. Kernel tests
also reject changed declared values/provenance, lower-bound obligations and
otherwise valid archives with changed retired element/occurrence reservations.
Exact query comparison uses an uninterrupted run on the same supplied revision:
`QueryResult.context.revision` must bind that revision, not the old archived
allocation. The exact comparator is unchanged and includes context identity.

## Actual focused verification

Worktree: `agentique-frontier-checkpoint`. Corpus processes were stopped; no
competing Cargo build ran. Environment:

```powershell
$env:CARGO_TARGET_DIR='C:/Users/phili/github/agentique-systems/agentique-frontier-checkpoint/target/resume-handoff'
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_INCREMENTAL='0'
```

| Command | Exit | Actual output |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kernel --test archive` | 0 | 12 passed, 0 failed; cold build 29.53 seconds, tests 0.05 seconds. |
| `AGQ_CERTIFICATE_VERIFY_FULL_REBUILD=1 cargo test --locked --offline -p agq-kerml-semantics --lib frontier_tests -- --nocapture` (initial) | 101 | 2 passed, 2 failed; build 1 minute 39 seconds, tests 4.49 seconds. The new test oracle compared queries from different revision identities. Graph, proof/search and certificate checks passed before the query assertion. Corrected the oracle to run uninterrupted on the reconstructed caller's revision; no production change or comparator weakening. |
| Same command after oracle correction | 0 | 4 passed, 0 failed, 293 filtered out; build 1 minute 15 seconds, tests 5.92 seconds. Completed construction restore reported invocation 1, round 4, converged true; no producer replay. |
| `cargo clippy --locked --offline -p agq-kernel -p agq-kerml-semantics --all-targets -- -D warnings` | 0 | Finished dev profile in 33.12 seconds, no warnings. |
| `cargo fmt --all -- --check` | 0 | No output. |
| `git diff --check` | 0 | No output. |

Detailed local command output: `resume-handoff-kernel.log` and
`resume-handoff-semantics.log`, `resume-handoff-semantics-corrected.log` and
`resume-handoff-clippy.log` beside this ledger (ignored generated logs).
No corpus acceptance claim follows from these focused tests.
