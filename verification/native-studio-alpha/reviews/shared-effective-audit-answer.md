# Reuse an already evaluated authored audit answer

Status: **held, not built and not run**. This separate change follows the held
mount-sharing commits `089fde4` / `edb8775`; no publication freshness pin, receipt,
profile, rule, source artifact or runtime installation has been changed.

## Observed duplicate work

The accepted-runtime baseline recorded 159.459 s command preparation and
144.408 s compilation, including 73.651 s final closure and 48.022 s effective
audit. The 14.549 s source-preparation phase is **inside** compilation and
includes declaration construction and reference refinement. The 15.051 s
command-minus-compilation residual mixes checkpoint, source proof, mount and
service postchecks; it is not a mount measurement.

The authored audit called `effective_usages(subject)` in the shared strict
dispatcher, then called it again after each batch to retain incomplete capability
diagnostics. Both calls used the same immutable query context. The SysML answer
itself is not memoized: it composes current usages, ancestors, observations and
pending implications, even when constituent KerML queries have warmed caches.
The existing timer does not isolate the duplicate call's cost. No wall-time or
peak-memory improvement is claimed by this patch.

## Exact change

`audit_sysml_population` remains the standard-publication entry point and calls
the same dispatcher with a no-op observer. The authored entry point supplies a
private observer. At the existing effective-usages slot, the dispatcher:

1. Evaluates the same public query on the same subject and context.
2. Gives its exact answer to the observer by immutable borrow.
3. Passes that answer to the unchanged `audit_typed_answer`.

The authored observer clones only non-Complete answers into the existing
`SourceDiagnostic::Capability`, including context, pending implications,
diagnostics, observations, positive/negative search evidence and provenance.
The observer runs under the existing Definition-or-Usage guard. Local subject
selection, deterministic batches of 32, complete family checks and all other
queries are unchanged. Capability diagnostics retain subject order; report
findings are still appended after all capability diagnostics. No local closure,
audit subject, validation check or source reconstruction is skipped.

For incomplete/invalid answers the original and its clone briefly coexist.
Peak memory on unsuccessful Working graphs therefore needs measurement; reduced
query calls do not establish reduced memory.

## Required parity tests

`crates/kerml-text/src/sysml/publication_audit_tests.rs` adds one ignored,
accepted-runtime test. It restores the supplied exact cache pair through the
ordinary authenticated facades, then compiles the existing shared ordinary and
variation Working-state source fixtures. No test-only accepted context is made.

On each immutable compilation it compares the previous audit-plus-second-query
loop against the observer flow, including:

- Independently derived local subject population and exact callback order.
- Every returned answer field, including its complete context and private
  additional-completeness field, through derived Debug with no normalization.
- All audit family counts, ordered findings and complete capability diagnostics.
- Actual stored source diagnostics, including capability-before-audit ordering.

The ordinary/variation populations exercise Complete and Incomplete answers.
The private answer-delivery boundary additionally receives actual public query
answers for the existing canonical root (wrong kind: Invalid) and a missing ID
(Incomplete). Those probes do not widen the production subject guard and do not
claim to construct a semantically accepted invalid graph. The test remains
unexecuted until the lead permits a serial standards consumer.

The real CreatePartUsage oracle also logs the existing final closure invocation
counters and cumulative `certificate_build_micros`, for both reconstructions.
Latest-round-only metrics remain omitted. Certificate time is already included
in final closure time and must not be added to it.

## Source freshness review

Only these additional interpretation source fingerprints change:

| Source | Recorded normalized SHA-256 | Proposed normalized SHA-256 |
| --- | --- | --- |
| `crates/kerml-text/src/source_inputs.rs` | `b5b9aebdfbb0f5ecafb730fbd68beee8b7200a0eb86d0eebc601b6c3259c1e87` | `f15171c15dd27c2e61c05c3dfffc426b1714b14cc09e7c8716bbb046d1f30128` |
| `crates/kerml-text/src/sysml/publication.rs` | `c3c8c13d7a7a8c0ecbee5042e13b919ab15035b8e32340c0e536d096b481d8ac` | `682f079a4e2c6c412b897d159f93f3b495f7fd8d69a675aec11f8ae6df9fbef7` |

The new `_tests.rs` file is excluded by the existing interpretation inventory.
The earlier mount-sharing source checkpoint fingerprint remains a separate
held change. Independent code review, exact runtime parity and cold full
reconstruction equivalence must precede any reviewed fingerprint update.

## Validation status and commands

Only formatting and diff checks ran locally; their actual results are retained
in `checks/shared-effective-audit-source-format.*`. No compiler, test process,
accepted-runtime restoration or standards consumer ran for this change.

After the lead allocates the existing target directory and a serial runtime
window, with the exact installed `AGENTIQUE_KERML_CACHE` and
`AGENTIQUE_SYSTEMS_CACHE` paths:

```text
cargo test --release --config profile.release.lto=false --locked --offline -p agq-kerml-text --lib authored_audit_observer_matches_two_pass_queries_and_diagnostics -j 1 -- --ignored --nocapture --test-threads=1
cargo test --release --config profile.release.lto=false --locked --offline -p agq-modeling-agent --features verification --test create_part_performance -j 1 -- --ignored --nocapture --test-threads=1
```

The first test compares old/new audit delivery on the same graph. The second
retains the independent cold full-reconstruction oracle. Record actual command,
exit, timings and whole-process peak memory. Test output from this optimization
must remain distinguishable from the preserved pre-sharing baseline.
