# KerML standard-library foundation review

Result: **KERML STANDARD LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**.

This is implementation work in generation 2, with no external-authority approval
blocker. Gates 0–2 pass. Later gates have substantial implementation, but the
three-library graph is an unpublished construction, not a validated Snapshot.
SysML semantics and the modeling platform must not depend on this candidate.

Authority remains the exact pinned KerML 1.0 specification, XMI and three KPAR
libraries. No Systems Library semantic loading, SysML frontend, execution,
persistence or platform work was introduced. The branch is
`semantics/kerml-standard-library-bootstrap-v1`, based on fetched main `1eb1424`.
The initial working tree was clean. Earlier verification evidence is preserved.

## Acceptance questions

| Question | Finding |
| --- | --- |
| 1. Do all 36 pinned documents parse without unexplained recovery? | **Yes.** The production frontend parses all 36 with zero recovery. The grammar-context inventory contains 199 used production kinds. |
| 2. Are all bytes preserved? | **Yes.** Exact contiguous UTF-8 token partitions reproduce every source; KPAR verification and repeated corpus tests retain immutable source identities. |
| 3. Are all structurally meaningful declarations lowered? | **Not accepted.** The builder traverses all production trees and constructs declarations, relationships, imports, aliases, multiplicities, documentation and expression structure. Unmet structural obligations and incomplete language interpretation remain. |
| 4. Are libraries canonical kernel elements rather than side objects? | **Candidate records use the ordinary complete KerML registry and kernel ModelView.** There is no parallel library model. There is not yet a successfully published library Snapshot. |
| 5. Are ElementIds deterministic for the exact bytes? | **Yes for the implemented construction.** Repeat loads compare every record, slot, association occurrence, source-map entry and unmet obligation. Locators include immutable document content, byte range and structural role; anonymous required children additionally include their owner ID. Qualified names are not identity. |
| 6. Is StandardLibrary provenance preserved? | **Yes in construction.** Every record, authored slot and association occurrence retains the exact LibraryId. DocumentId, revision, range and syntax-node evidence live separately in the source map. |
| 7. Are packages/namespaces/imports/aliases represented semantically? | **Yes in construction.** The corpus includes 204 canonical imports: 167 membership and 37 namespace imports. Aliases retain Membership identity. |
| 8. Does resolution handle imports and visibility? | **Implemented with remaining corpus gaps.** Semantic queries cover qualified/root lookup, membership and namespace imports, recursion, aliases, public/private/protected boundaries, inheritance, shadowing and ambiguity. Full-corpus complete resolution is not accepted. |
| 9. Are cyclic imports safe? | **Yes for the implemented queries.** Iterative graph traversal and fixed-point propagation terminate on tested cycles; repeated nonconvergent filtering states report incompleteness. No call-stack traversal proportional to import depth. |
| 10. Are unresolved/ambiguous structural references eliminated or justified? | **No.** The current quality report retains unresolved redefinitions and incomplete query evidence. They have not been relabeled as published-source anomalies. |
| 11. Do bindings point into the canonical graph? | **Yes, into the candidate kernel graph.** They are IDs of actual parsed declarations, not substitute objects. Whole-library publication remains unaccepted. |
| 12. Are bindings validated rather than name-hardcoded? | **Yes for 22 implemented roles.** Contracts specify full public ownership paths, exact metaclasses and exact library provenance; missing, duplicate, wrong-kind, wrong-library and inaccessible fixtures fail. The checked-in manifest is generated and stale-checked. These are Agentique IDs, not OMG-assigned IDs. |
| 13. Does SemanticContext identify actual loaded libraries? | **Partially.** Construction contexts include exact three-archive hashes, canonical StandardLibrary record/occurrence digest, validated target IDs, binding version and rule version. A first-class accepted publication input remains to be implemented. |
| 14. Can authored projects resolve against them? | **Not yet accepted or integrated.** Authored inheritance lookup was improved and project regressions pass, but no authored project is allowed to treat the unpublished library candidate as an accepted dependency. |
| 15. Are unsupported executable semantics distinguished? | **Yes.** Expression/function records and syntax survive. The quality report counts unevaluated bodies separately from syntax, resolution, structural/language diagnostics and reviewed grammar discrepancies. No evaluator was added. |
| 16. Is the quality gate green in its defined scope? | **No.** It requires strict publication and complete structural-semantic validation. Candidate endpoints and successful process execution cannot pass it. |
| 17. Do both structural runtime gates remain green? | **Yes in the recorded focused checks.** The final workspace verification also records both gate results; strict metamodel-authoring conformance is separate and retains its errors. |

## Implemented architecture

[ADR 0013](adr/0013-kerml-library-grammar.md) compares the frontend alternatives
against the actual corpus. Maintained grammar data generates deterministic Rust
recognition tables. A lossless production arena provides typed borrowed views,
Table 6 precedence, recovery, source ranges and conservative edit reconciliation.
The old bounded authored adapter remains available; it does not silently gain
support merely because the production recognizer can parse a construct.

Canonical construction is a transaction builder over complete `agq-kerml`
descriptors. `Snapshot::preview` provides an explicitly unpublished
`ConstructionView`, retaining mandatory lower-bound obligations. It still checks
present value kinds, targets, upper bounds, uniqueness, composition and association
constraints. `Snapshot::apply` continues to enforce complete structural publication.
No partially valid library Snapshot is exposed.

`refine_declarations` reconstructs immutable candidates from the same verified
sources while semantic queries establish relationship endpoints. A unique candidate
with incomplete evidence remains provisional. Stability of endpoint candidates is
not completeness, language validity or publication acceptance. Cyclic refinement
states fail explicitly. Neither parser names nor regex extraction resolve targets.

Namespace membership queries retain actual Membership identities and filter
redefinition populations before selecting names. Search evidence includes member
populations, import sets, imported namespaces, visibility/property reads and project
root availability. Proofs retain StandardLibrary fact origins. Per-evaluator
memoization is bound to one immutable context; there is no persistent incremental
cache or reuse across changed source revisions.

## Remaining implementation work

The completed quality run constructed 29,265 candidate elements, including
16,747 relationships. Of 4,003 reference assertions, 3,998 have provisional
endpoints; five names are unresolved, none ambiguous, and 57 reference answers
remain incomplete. No established endpoint differs from its current unique
candidate. There are ten unmet structural lower bounds: five redefinition
endpoints and five Interaction participant relationships. Resolution is complete
in 33 of the 36 document reports, but the whole publication is incomplete.

The audit evaluated 60,560 existing structural queries: 530 were incomplete and
none invalid. This does not establish the unimplemented broader language
constraint scope. It retains 3,899 unevaluated expression/function elements.

The quality report is the current machine-readable diagnostic register. In
particular, construction does not yet populate the five required
`Interaction::participantFeature` relationships in `Transfers.kerml`. Correct
handling must reconcile actual end-feature semantics, inherited features, the
association redefinition and its opposite multiplicity. The kernel gate has not
been weakened and inherited features have not been copied to satisfy the bounds.
This is a remaining implementation/interpretation obligation, not an external
authority blocker or a newly accepted source anomaly.

Remaining redefinition references include `InsideOf`'s `source`/`target`,
Observation's nested `observations`, and two nested `monitoredFeature` references
in FeatureReferencingPerformances. Candidate searches and their completeness are
recorded precisely. The implementation has not established a normative resolution
for these names. A more permissive lexical fallback or a manually manufactured
library declaration would not discharge this work.

The checked-in bindings support selected base specializations and positional
parameter/result/end redefinitions. They are not a complete implementation of all
implicit KerML relationships. Full constraint validation, complete feature-chain
interpretation and the existing effective-feature query's remaining unsupported
cases still require implementation. The quality command evaluates its named query
families and reports missing broader validation explicitly; null is never zero.

Next acceptance requires complete lowering and revalidation, strict transactional
Snapshot publication, a validated publication facade, authored SourceProject
integration, the full library-aware dependency audit, and combined corpus/project
stress tests. No `LoadedKermlStandardLibraries` facade is offered before it can
actually promise these invariants.

## Discrepancies and diagnostic boundaries

ADR 0013 documents grammar transcription dispositions. Anonymous invariants have
42 grammar discrepancies across nine corpus files, distinct from recovery. Casts
have a typed result and an appended zero-width empty-result production in the
printed grammar; construction uses the typed result to satisfy the empty default.
Both syntax productions remain present. The report identifies each affected cast.
This follows the expression's single-result constraint and the exact
`BaseFunctions::as` / `::meta` signatures. It does not execute either function.

Unknown ordinary semantics remain incomplete. Published XMI conformance errors
remain errors in their separate strict gate. Historical lexical reports and prior
milestone results have not been rewritten to imply later success.

## Verification and complexity

Evidence is in [the milestone directory](../verification/kerml-standard-library-bootstrap-v1/README.md).
`gate-0`, `gate-1`, `gate-2`, `construction-1` and `construction-2` contain focused
commands, full output and actual exit codes. `quality-1` records the current
semantic quality command. `final-1` records workspace and application checks;
`strict-1` preserves separate strict conformance failures.
All 19 commands in `final-1` passed, including both runtime gates and strict
Rustdoc for all eight public generation-2 crates. Preservation checked 693 prior
files. `review-1` repeats standards integrity after the coverage/review update.
The semantic quality command exited **1**; both separate strict metamodel
conformance commands exited **1**. These failures remain visible and are not
folded into the successful structural runtime result.

Tests include per-family syntax fixtures; repeated lossless corpus parses;
repeatable canonical construction; corrupt binding fixtures; import and alias
ambiguity; visibility changes; insertion after misses; import removal; inherited
and renamed diamond lookup; project boundaries; a 64-namespace import cycle;
256 levels of inheritance; and simultaneous immutable semantic queries. The
required combined accepted-library/many-authored-document stress case is not yet
available because accepted library publication is incomplete.

Chart parsing has the documented generalized-parser worst-case bounds and an
explicit work budget. Semantic graph traversal uses indexes and iterative work
queues. Namespace populations are cached per immutable evaluator; evidence is
shared through aggregate claims instead of repeated per-member proof expansion.
Fixed-point propagation can require many rounds; dense redefinition filtering and
library refinement are not claimed linear. There are no timing-based correctness
thresholds or claimed production-performance guarantees.

## Publication v2 review, 2026-09-19

Result: **KERML STANDARD LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**.

The historical review above is unchanged. Fresh Gate 0 reproduction on fetched
main `ad6203b` identifies authority conflict **KLPV2-F-001**. The exact pinned XMI
participant associations are the subject of open OMG issue KERML11-81. They lack
a normative derivation from effective ends, and their inverse `1..1` bound rejects
sharing an inherited Feature between Interactions. A synthetic witness preserves
original feature identities and confirms ordinary kernel rejection. See the
[authority analysis](kerml-standard-library-publication-authority-conflict.md)
and [v2 evidence](../verification/kerml-standard-library-publication-v2/README.md).

This corrects the earlier assumption that every participant blocker necessarily
has a missing implementation rule. It does not reclassify ordinary unresolved
references as authority conflicts. No production semantics or validation
invariants have been changed, and no library publication is accepted.

| V2 acceptance question | Result |
| --- | --- |
| 1. All 36 sources parse losslessly? | Yes; baseline reproduced with zero recovery. |
| 2. Ordinary strict Snapshot publication? | No; strict apply still rejects the candidate. |
| 3. Required structural references resolved? | No; five unresolved names and 57 incomplete reference answers. |
| 4. Interaction participants computed correctly? | Blocked by KLPV2-F-001; the required mapping is not normatively defined and conflicts with pinned bounds. |
| 5. Implicit redefinitions complete? | No new completeness claim; existing obligations remain. |
| 6. Structural expression/result construction valid? | `incomingTransitionTrigger` is complete in the fresh baseline; broader structural validation remains unaccepted. |
| 7. Imports/aliases/visibility complete for the corpus? | Substantial existing support; corpus acceptance remains incomplete. |
| 8. Effective features complete for required semantics? | No; existing query incompleteness is preserved. |
| 9. Inherited features never copied? | Yes; the new witness explicitly retains the original IDs. |
| 10. StandardLibrary origins exact? | Existing deterministic source construction retained; no new authored library facts. |
| 11. Accepted bindings are canonical IDs? | No accepted bindings; 22 candidate bindings remain. |
| 12. Authored projects resolve accepted libraries? | Not available without accepted publication. |
| 13. SemanticContext identifies accepted publication? | No; candidate semantics remain unchanged. |
| 14. Required structural KerML validations implemented? | Not complete; Gate 7 not entered. |
| 15. Unevaluated executable bodies separated? | Yes; 3,899 remain separately reported. |
| 16. Semantic quality gate green? | No; fresh command exits 1. |
| 17. Both structural runtime gates green? | Yes; both fresh commands exit 0. |
| 18. Strict metamodel diagnostics separately preserved? | Yes; both separate strict conformance commands exit 1. |

The stop follows the explicit authority-conflict policy. Resumption requires an
adopted correction or an explicit project authority decision about the spurious
association contract and the milestone's participant requirement. Neither an
open issue nor a computed effective-end set authorizes a silent exception to
Snapshot validation.

## Operational errata publication v3 review, 2026-09-19

The project-authorized operational interpretation resolves KLPV2-F-001 without
changing the published baseline. [ADR 0014](adr/0014-operational-standard-errata-profiles.md)
defines the two profiles and their migration contract. The current stop is the
separate **KOPV3-F-001 / KERML11-140**
[redefinition resolution conflict](kerml-operational-library-publication-authority-conflict.md),
independently reproduced without library-dependent semantics. The historical
incomplete sections above remain true records of their milestones.

| V3 acceptance question | Result |
| --- | --- |
| 1. Published 1.0 artifacts unchanged? | Yes; pinned bytes, generated descriptors and historical evidence are preserved. |
| 2. KERML11-81 preserved in published inspection? | Yes; published descriptors and the original contradiction witness remain available. |
| 3. Operational correction exact/hash-qualified? | Yes; reviewed artifact URI, SHA-256, external IDs, generated IDs and source ranges are checked. |
| 4. Only reviewed descriptors removed? | Yes; two associations and four exclusively owned ends. |
| 5. Independent verification confirms diff? | Yes; direct XML/UUID verification plus compiled descriptor comparison; zero unrelated changes. |
| 6. Profile identity first-class? | Yes; BaselineProfile, stable profile ID, frozen manifest digest and effective graph digest. |
| 7. Participants absent rather than fabricated? | Yes; the operational witness accepts original shared ends without participant links. The independently compared corpus obligations drop from five participant obligations to zero; the other five are unchanged. |
| 8. Remaining redefinitions resolved normatively? | No; KERML11-140 establishes a distinct conflict for nested resolution; other unresolved cases remain. |
| 9. Required effective-feature semantics complete? | No; existing incomplete structural queries remain implementation obligations after the authority stop. |
| 10. Strict library Snapshot publication passes? | No; missing redefinition endpoints still prevent ordinary strict publication. |
| 11. Bindings validated against accepted Snapshot? | No accepted Snapshot; the 22 existing canonical candidate anchor bindings remain separately validated. |
| 12. Authored projects use accepted libraries? | No; authored construction uses the operational profile, but an accepted dependency is unavailable. |
| 13. Profile/library dependencies in SemanticContext? | Yes for current construction: profile, review digest, effective graph, original artifact, library pins, candidate graph, rules and bindings. Accepted-library integration is pending. |
| 14. Library quality gate green? | No; structural publication remains incomplete. |
| 15. Both structural metamodel runtime gates green? | Yes; both complete structural runtime commands exit 0 in v3 `final-1`, independently of publication. |
| 16. Strict published diagnostics visible? | Yes; the raw profile, source findings and separate strict-conformance commands are retained. |
| 17. Executable semantics separated? | Yes; evaluation is neither implemented nor claimed by this structural milestone. |

The stop is caused by the conflict between a published resolution rule and an
intended nested modeling pattern, not by KERML11-81's issue status. A fallback
that changes the specified scope search has not been introduced. See the
[v3 evidence](../verification/kerml-operational-errata-publication-v3/README.md)
for actual commands, outputs, exit codes and preservation results.
