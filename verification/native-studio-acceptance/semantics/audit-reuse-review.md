# Audit reuse adversarial code review

This review covers the closed-audit collector, source audit dispatcher, and
signature/invalidation boundary. It is not a substitute for the real independent
CreatePartUsage/cold-reconstruction oracle, whose wall time and equality result
remain a separate gate.

| Adversarial change | Checked boundary | Review result |
| --- | --- | --- |
| Rename an existing Part | The subject's record, original declared slots and proof/search metadata enter its signature. Namespace lookup merges effective-name reads for every considered member, including members that did not match the old name. | The negative-name regression renames an existing member while its namespace record stays unchanged. The formerly empty lookup becomes positive and the old audit receipt is rejected. This tests the collector; the full native Rename journey is separate. |
| Add a previously absent relationship/provider match | Incoming canonical carriers and occurrences contribute to endpoint signatures. Native and kernel source/incoming searches retain their invalidation keys. | The negative feature-typing test passes, including unchanged target record bytes. Global population searches remain global. |
| Retarget or delete an existing carrier | Signatures are rebuilt for both revisions; the old endpoint loses its carrier contribution and the new endpoint gains it. Missing record IDs remain in the old/new key union. | Both sides of the changed relationship invalidate the corresponding reads. |
| Make another producer applicable | Reuse is available only after both exact registry/context frontiers are fully closed. Registry changes reject the context. Actual materialized outputs/proofs and changed pending scopes participate in the delta. | No scheduler work is skipped by audit reuse. Open or missing certificates disable reuse. The changed-writer-registry regression also rejects equal records with a different fully closed static contract. |
| Change SysML profile or standard bindings | The source dispatcher compares the complete non-KerML SysML context, including dependency contract, bound role IDs, profile, rule identity and descriptor identities. KerML's static contract is independently compared. | The changed-query-options and writer-registry regressions pass. These test the KerML static contract; a dedicated changed-SysML-profile/binding regression is not yet present. |
| Replace a dependency with an equal-looking allocation | Factored signatures require the same `ProducerClosedDependency` allocation via `Weak::ptr_eq`; the weak handle also prevents address reuse while the old snapshot exists. | The separate-mount regression uses the exact same overlay allocation and context in two distinct dependency mounts. It rejects cross-mount reuse and accepts the same-mount control. Optional proof/contribution tables cannot be replaced merely by presenting the same semantic publication digest. |
| Reuse an earlier failed or incomplete audit | Only subjects with no findings are retained. Every observed typed/supporting answer must be Complete and have the exact expected context; otherwise its read receipt is invalid. | A Complete/Invalid/Complete observation sequence remains unusable even on the unchanged graph. Incomplete/foreign-answer rejection also passes. Existing audit delivery parity covers all three completeness states; a full mixed-success/failure edit sequence remains a broader integration case. |
| Change a class that previously caused no queries | Every subject registers a direct subject read before audit dispatch. Its class, properties, origins and static descriptor contract are bound even when no query result was emitted. | Zero-query subjects can be retained, but they contribute zero actual reused checks. Reused-subject counts alone do not demonstrate a speedup. |
| Change direct mayTimeVary storage or its rule evidence | The dispatcher directly checks the canonical property, Boolean type and expected derivation rule. The mandatory subject signature includes slot/navigation values and origins. | The non-query check remains covered by the signature and checked-family count. |

Only successful audit counts and checked read receipts move between revisions.
No `QueryResult`, value, explanation or old certificate digest is relabeled as a
child-revision result. Every closure witness used by the old outcome is re-proved
against the new fully closed certificate. Fresh public queries remain bound to
the child context and are compared in the independent oracle.

The current signature deliberately merges positive facts, incoming carriers and
output-support changes conservatively. A new relationship pointing at a common
accepted type can invalidate unrelated readers of that type. This is a possible
performance limitation, not permission to ignore provider or negative reads.
The measured `effective_audit_checks_reused` and reuse-setup timing must determine
whether the implementation actually saves semantic work.

No demonstrated unsound reuse path was found in this review. All eight collector
tests passed, as did both immutable producer-row/certificate-equivalence tests.
The independent real-model oracle and broader rename/failure sequences remain
separate gates.

## Adversarial test evidence

The integration lead ran:

```text
cargo test --locked --offline -p agq-kerml-semantics --lib closed_query_audit -- --nocapture
```

Exit code 0; eight passed, zero failed, zero ignored; test execution 0.26 s.
The command took 83.953 s including compilation. The command record identifies
source commit `2dfe03e46dc093808b7976b9cf47c363a5273c4b` plus the recorded unstaged
test changes. Actual command, environment, output and output digest are retained
in [the JSON record](../../native-studio-alpha/checks/acceptance-closed-audit-adversarial-tests.json)
and [test output](../../native-studio-alpha/checks/acceptance-closed-audit-adversarial-tests.txt).

## Independent oracle operation coverage

Read-only comparison of `write_observations` in
`crates/modeling-agent/tests/create_part_performance.rs` with
`audit_authored_effective_subject` and its shared dispatcher in
`crates/kerml-text/src/sysml/publication.rs` found all 29 distinct typed query
operations represented. A Python static operation-name comparison returned exit
code 0, with 29 operations in each set and no difference. Metaclass guards were
also checked manually: the oracle combines equivalent connection/interface,
parameter and subaction applicability guards with `or`; it does not drop those
queries.

Each observed `SysmlQueryResult` uses its complete derived `Debug` representation,
including the base value, context, completeness and evidence, supporting queries,
supporting names, canonical observations, pending implications, rejected/filtered
targets and private additional completeness. The oracle normalizes only the
freshly allocated kernel revision label. The direct mayTimeVary storage check is
covered by canonical records, declared slots, derived navigation and the audit
report. No effective query family used by this authored acceptance dispatcher is
omitted. This coverage statement concerns the bounded audited self-model domain,
not every possible parameterized semantic query.

## Warm restoration trust boundary

`SourceSemanticCache` persists the effective frontier and source, model, context,
registry and closure digests. It does not persist complete certificate transport,
successful audit outcomes, checked audit read receipts or query memoization.
`ValidationReceipt` is public, serializable consistency metadata; repository-level
verification checks its acceptance contract and source binding. The service then
compares reconstructed graph/context/certificate identities and calls workspace
validation, which consumes the newly computed effective audit. Matching a cache
and receipt supplied together cannot independently prove that the acceptance
audit ran. Skipping that audit under the current persisted contract would change
the trust boundary.

An exact already-authenticated `BoundRevision` in memory can safely be reused and
the service already does so. Persisted reuse would require a separately justified
acceptance/receipt boundary plus the missing checked material, exact identity
authentication, and discard-to-source behavior. This review introduces no such
boundary and does not treat a cache format number as authority.

## Remaining avoidable work to measure

The authored audit creates a fresh query evaluator every 32 subjects. Its exact
immutable context is shared, but namespace, effective-name, library, relationship
source and origin memoization is discarded with each batch. For 791 local
subjects this is 25 evaluators, so common standard/inheritance traversals can
repeat. Sharing bounded memoization could preserve all acceptance queries;
memory measurements are required because batching deliberately releases large
proof structures. No savings are claimed from this observation.

Selecting the local audit population also traverses all roughly 76,000 model
elements and performs dependency lookups to exclude accepted elements. Its cost
has not been isolated; it must not be assumed to dominate the audit.

`SourceCompilation` retains SysML and KerML contexts in separate cells. The first
audit initializes the SysML context, including its KerML context; the first later
KerML reader constructs the same final project context again. Reusing the exact
retained inner KerML context is a possible way to remove that duplicate full-model
authentication without weakening it. No source change was made for this finding.
Workspace `validate()` itself does not run effective queries again; it consumes
the completed audit and checks its exact context and certificate. New measured
phase timings, rather than these code-review observations, must establish the
remaining cost and any actual speedup.
