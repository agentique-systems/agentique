# Audit reuse adversarial code review

This review covers the closed-audit collector, source audit dispatcher, and
signature/invalidation boundary. It is not a substitute for the real independent
CreatePartUsage/cold-reconstruction oracle, whose wall time and equality result
remain a separate gate.

| Adversarial change | Checked boundary | Review result |
| --- | --- | --- |
| Rename an existing Part | The subject's record, original declared slots and proof/search metadata enter its signature. Namespace lookup merges effective-name reads for every considered member, including members that did not match the old name. | An unchanged owner ID does not establish name stability. A changed subject or consulted member invalidates reuse. A direct rename regression remains desirable. |
| Add a previously absent relationship/provider match | Incoming canonical carriers and occurrences contribute to endpoint signatures. Native and kernel source/incoming searches retain their invalidation keys. | The negative feature-typing test passes, including unchanged target record bytes. Global population searches remain global. |
| Retarget or delete an existing carrier | Signatures are rebuilt for both revisions; the old endpoint loses its carrier contribution and the new endpoint gains it. Missing record IDs remain in the old/new key union. | Both sides of the changed relationship invalidate the corresponding reads. |
| Make another producer applicable | Reuse is available only after both exact registry/context frontiers are fully closed. Registry changes reject the context. Actual materialized outputs/proofs and changed pending scopes participate in the delta. | No scheduler work is skipped by audit reuse. An open or missing certificate disables reuse entirely; this is tested. |
| Change SysML profile or standard bindings | The source dispatcher compares the complete non-KerML SysML context, including dependency contract, bound role IDs, profile, rule identity and descriptor identities. KerML's static contract is independently compared. | Neither a similarly named binding nor equal subject IDs suffice. A dedicated changed-profile/binding regression is not yet present. |
| Replace a dependency with an equal-looking allocation | Factored signatures require the same `ProducerClosedDependency` allocation via `Weak::ptr_eq`; the weak handle also prevents address reuse while the old snapshot exists. | Optional proof/contribution tables cannot be replaced merely by presenting the same semantic publication digest. A separate-mount rejection test remains desirable. |
| Reuse an earlier failed or incomplete audit | Only subjects with no findings are retained. Every observed typed/supporting answer must be Complete and have the exact expected context; otherwise its read receipt is invalid. | Incomplete/foreign-answer rejection is tested. Existing audit delivery parity covers Complete, Incomplete and Invalid answers. A full mixed-success/failure edit sequence remains a broader integration case. |
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

No demonstrated unsound reuse path was found in this review. Four new collector
tests and both immutable producer-row/certificate-equivalence tests passed; see
the exact command records in `../../native-studio-alpha/checks/`. The independent
real-model oracle and broader rename/failure sequences remain required evidence.
