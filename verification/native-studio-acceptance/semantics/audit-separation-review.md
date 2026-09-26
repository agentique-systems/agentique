# Independent review: audit and producer signature separation

**One soundness gap found and corrected in source; integrated tests and a new exact oracle remain required.** The review covered the combined working changes to `producer_closure_rebind.rs`, `closed_query_audit.rs` and their tests, then the explicitly assigned evidence-composition fix in `contract.rs`.

The separation itself is appropriate. A producer checkpoint promises to recreate its outputs and therefore must reopen when a detached output disappears even if all original inputs remain equal. A successful query audit promises only that the observed answer and its read obligations remain valid. An unread detached consumer of an unchanged input should not invalidate that input's audit. Full checkpoints and shared-dependency checkpoints still use output-support propagation; only the audit signature helper disables it.

Disabling that reverse output-to-input propagation requires explicit transitive positive reads. The collector now retains both immediate canonical roots and `positive_dependencies`. Ordinary fact expansion walks declared and derived proof dependencies, including occurrence endpoints and selected contribution evidence. A changed declared leaf can therefore invalidate a read of an otherwise equal derived root. The new test deliberately preserves the outer derived record and its immediate provenance IDs while changing that leaf. Negative/provider searches remain independent: ordinary and shared search sets still map to incoming, subject or global invalidation keys, and supported closure witnesses are checked again against the receiving closed certificate.

Incoming carriers remain in signatures. Local record references update target signatures; local association occurrences update all endpoints; local contribution, computation-search, navigation and failure payloads retain their own hashes. Both audit contexts must be fully closed. Their static query contract must match, and exact Weak allocation identity is required for a factored dependency. Sealed dependency proof expansion can stop at that authenticated immutable root without treating historical library searches as queries of a new project's local population. A separately minted equal mount still cannot reuse the factored snapshot.

## Finding: evidence composition could erase the producer guard

The proposed collector rejected an answer when `answer.producer_evidence` was true. However, `QueryResult::merge` retained only the receiver's mode, and public `merge_evidence` called it. An ordinary answer for an unrelated subject could absorb a compact producer answer for a derived root, retain `producer_evidence == false`, and omit declared transitive leaves that compact expansion intentionally does not visit. With equal derived roots and a changed declared leaf, the audit's new signatures could incorrectly preserve that mixed receipt. The direct compact-answer regression did not cover this composition boundary.

The first correction made `producer_evidence` itself sticky. The full semantic unit suite correctly rejected that implementation: `a_million_logical_search_entries_stay_shared_through_subquery_merges` requires an ordinary receiver to materialize merged search sets immediately. The producer flag also controls later explanation construction; it is an evaluation policy, not only proof provenance. That implementation was replaced, and the existing test was not relaxed.

`contract.rs` now retains an independent private `contains_compact_evidence` flag. `queries.rs::result` initializes it from the evaluator mode. Both merge entry points make only this provenance flag sticky, including propagation through an already-mixed ordinary answer. The receiver's `producer_evidence` mode remains unchanged. The audit collector rejects either marker. The public context check still occurs before any mutation. Clone and `map` preserve the provenance flag; search expansion and later ordinary evidence cannot reset it. Query equality includes this safety-relevant provenance while ordinary-only evidence remains unchanged.

The separate `query_evidence_mode.rs` regression uses an actual derived compact answer with a deferred incoming search. It tests public and private merge in both orders, then value projection, search expansion, another full-evidence merge, and relay through an ordinary receiver. All must retain compact provenance and exact canonical/search support. Ordinary receivers must immediately materialize searches and continue constructing full public explanations; compact receivers retain their prior expansion policy. A second test ensures a rejected foreign-context compact merge leaves the receiver equivalent in query equality and untainted. Formatting passes; root owns execution. The semantic-performance agent added the mixed transitive-root audit regression: an ordinary unrelated-root answer absorbs compact outer-root proof, then must fail audit reuse when its omitted declared leaf changes.

The SysML audit dispatcher already observes the main KerML answer, supporting queries, supporting names and all canonical observations individually. Thus a compact answer in any of those positions reaches the same rejection guard; no special bypass was found in that dispatcher. The prior invalid flag remains sticky after a failed/incomplete answer. Audit reuse transports a successful audit receipt and counts, not old QueryResults or revision-bound explanations.

No other correctness blocker was found by source review. This conclusion is bounded by existing read accounting: a semantic query must still record every value/proof/search it consults. The proposed change does not establish a speedup until the complete real command/cold oracle passes and records a wall-time improvement. The old a6 oracle and freshness proposal do not qualify these changed production files.

Reviewer: Codex `/root/worlds`. Only the explicitly assigned `contract.rs` change, its constructor initialization in `queries.rs`, and its separate unit test file were edited. No semantic build, runtime workload, collector/signature edit or freshness pin update was performed by this reviewer.

Integration execution after the separate-taint correction: all 321 KerML semantic
unit tests passed with the verification feature (two explicit scale probes remain
ignored), including the unchanged eager-search regression. The bounded source
trace test passed when invoked by its exact function filter; an earlier incorrect
module filter ran zero tests and is retained without counting as coverage.
Workspace Clippy with all targets and warnings denied passed; workspace formatting
passed. Actual commands, output and exit codes are retained as
`acceptance-audit-split-tests-03`, `acceptance-audit-trace-text-tests-02`,
`acceptance-workspace-clippy-03` and `acceptance-format-05` under
`verification/native-studio-alpha/checks`. These gates do not replace the real
command/cold oracle or provide a measured speedup.
