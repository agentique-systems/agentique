# Operational KerML library publication: redefinition authority conflict

Gate 7 stops at **KOPV3-F-001**, a distinct authority conflict after the authorized
KERML11-81 descriptor correction. This does not reopen the participant decision.
The operational profile deletes the two obsolete associations and four owned
ends exactly; the independent diff and both participant witnesses pass.

The pinned KerML 1.0 PDF (`formal/2026-03-01`, SHA-256
`3bcc96f989bfa9d05cd28e026df3351b795fe8d494187b87bff3db7d96373697`),
8.2.3.5.1, printed page 83 / PDF page 109, gives a special lookup rule for an
owned redefinition. It tries basic resolution using the general type of each
owned specialization of the owning type, in turn. If those searches fail,
resolution fails. Basic resolution includes outer scopes of those general types;
it does not search outer scopes of the specific type instead.

[OMG KERML11-140](https://issues.omg.org/issues/KERML11-140), “Name resolution for
the target of a redefinition,” identifies precisely this conflict with the
intended nested time-slice pattern. The freshly captured issue is open, reported
against KerML 1.0, and contains a failing example and explanation. It does not
supply an unambiguous corrected resolution algorithm. The current preliminary
KerML 1.1 Beta 2 PDF still contains the same failure rule on PDF page 109. These
revision materials are corroboration, not the normative target.

This affects the two pending `monitoredFeature` declarations in the pinned
`FeatureReferencingPerformances.kerml`: `beforeTimeSlice` is typed by
`Occurrence` and subsets `timeSlices`; `afterSnapshot` is typed by `Occurrence`
and subsets `snapshots`. Their intended `monitoredFeature` declaration is in
the containing `monitoredOccurrence`. The explicit general types lead to the
Occurrences library's scopes, not that containing declaration. The analogous
nested `observations` case and the `InsideOf` source/target cases remain
unresolved; this record does not claim to settle all of their semantics.

The arbitrary-name regression in `crates/kerml-semantics/tests/imports.rs`,
`kerml11_140_nested_redefinition_cannot_search_the_specific_types_outer_scope`,
isolates the issue with no library bindings, imports, feature chains, conjugation,
or inherited population gaps. A feature in `Cobalt` specializes types/features
under `Quartz`; its nested redefinition cannot denote `Cobalt::signal` through
the required starting scopes. A separate lexical query can find that feature,
proving the difference in scope. Both profiles return a **complete empty answer**
for the required lookup and retain the missing endpoint obligation. The witness
therefore does not confuse an incomplete query with a missing normative rule.

Applying the extra lexical search would require a semantic errata decision about
scope precedence, visibility, ambiguity and redefinition validity. It is not the
hash-qualified descriptor deletion authorized for KERML11-81. The user's explicit
ban on permissive lexical fallback applies. No new errata entry, source rewrite,
fake relationship or validation exception has been introduced for KERML11-140.

Under the task's stop policy this is another genuine authority conflict requiring
an invented rule to proceed, so strict canonical library publication remains
unaccepted. The remaining ordinary effective-feature, expression, validation,
binding and authored-library integration work is still required; none is declared
complete because of this stop. No accepted-library facade is provided.

Evidence is in
[`verification/kerml-operational-errata-publication-v3`](../verification/kerml-operational-errata-publication-v3/README.md):
the fresh issue capture and hash, PDF inspection, synthetic witness, operational
corpus obligations, independent descriptor diff and final verification logs.
Historical authority/conflict reports and original artifacts remain preserved.
