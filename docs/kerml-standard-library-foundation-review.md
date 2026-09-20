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

## Name-resolution errata publication v4 review, 2026-09-19

The authorized [operational v2 profile](operational-kerml-profile.md) implements
AGQ-KERML10-002 / KERML11-140 as an explicit, versioned semantic interpretation.
KERML11-140 remains open, last updated 2025-11-11 23:25 GMT. It is not represented
as an adopted OMG correction. Published KerML 1.0 remains the formal baseline;
operational v1 adds only KERML11-81; operational v2 adds the reviewed redefinition
resolution algorithm. The historical v3 stop above records the authority state
before this task's authorization.

The new stop is **KNRV4-F-001**, the independently demonstrated
[pinned Objects library conflict](kerml-pinned-objects-authority-conflict.md).
`StructuredSpaceObject::faces`, `edges` and `vertices` each inherit two distinct
same-name `innerSpaceDimension` Features. Published redefinition suppression
retains both, violating namespace distinguishability. The matching historical
reference source is byte-for-byte identical to the pinned file. Current
reference source and XMI explicitly add common redefinitions through combined
types. This later library-source correction is not covered by KERML11-140.

| V4 acceptance question | Result |
| --- | --- |
| Published and operational v1 behavior reproducible? | Yes; the arbitrary-name KERML11-140 witness still fails under both profiles. |
| Operational v2 semantic correction explicit and separate? | Yes; AGQ-KERML10-002, algorithm version 1, profile `/2`, frozen manifest and evidence hashes. No descriptor anomaly is introduced. |
| KERML11-81 reopened or broadened? | No; its reviewed bytes, descriptor correction and independent contradiction/diff witnesses remain. Git's normalization of the v1 manifest was repaired to restore its already-frozen CRLF digest. |
| Generic lexical fallback introduced? | No; only owned Redefinition targets dispatch to v2. Complete empty inherited-prefix lookup permits the actual containing lexical scopes. Qualified suffix failure, wrong kind, ambiguity and incomplete search do not retry. |
| Deterministic adversarial coverage? | Yes; 21 scenarios in both relationship insertion orders, plus focused tests for incomplete search, derived source ownership, effective membership identity, ordering, ordinary lookup and implied result naming. |
| Current reference artifacts treated as normative? | No; their source, tests, release notes and XMI are explicitly implementation/interchange corroboration. Original issue and formal/preliminary clause bytes are retained. |
| Two exact nested monitoredFeature targets independently identified? | Yes; both reference XMI relationships point to the enclosing monitored feature. The full comparison report checks individual assertions and complete target sets. |
| Remaining ordinary semantics investigated? | Yes; binary association specialization, inherited end ordering, imports, membership identity, conjugation, chains and name/result construction received ordinary implementation fixes. Full obligation reports retain all remaining findings. |
| Validation inventory complete? | No; evaluated constraints are named explicitly. Further structural, derived and implied relationship constraints remain mandatory. The new distinguishability check exposes KNRV4-F-001. |
| Kernel validation weakened? | No. The strict Snapshot experiment uses ordinary `Snapshot::apply` and preserves its empty base. Passing kernel storage checks is recorded separately from semantic acceptance. |
| Accepted library Snapshot and facade published? | No; semantic publication is blocked. No ConstructionView is exported as `LoadedKermlStandardLibraries`. |
| Accepted bindings regenerated? | No; the historical 22-entry candidate manifest is preserved. BinaryLink adds a 23rd runtime role, so the old manifest check reports stale. No new candidate IDs are labeled accepted. |
| Authored projects select exact profiles? | Yes; explicit project metadata and contexts distinguish published, v1 and v2, with the same source producing profile-dependent resolution. Consumption of accepted libraries remains unavailable. |
| Operational conclusions explain their authority? | Yes; query proofs identify the rule, profile/context, source anchor, searched general and lexical scopes, visibility facts, winning target and positive/search dependencies. |
| Executable evaluation claimed? | No; unevaluated expression/function bodies remain a separate quality category. |
| Both complete structural runtime gates green? | Yes; fresh KerML and SysML structural runtime commands exit 0. This does not establish SysML language semantics or accepted library publication. |

The [v4 evidence index](../verification/kerml-name-resolution-errata-publication-v4/README.md)
links actual commands, outputs, exit codes, full profile obligation sets, XMI
comparison, structural quality, and the separate authority witness. It preserves
failed and superseded runs alongside successful rechecks. The
[structural rule review](kerml-v4-structural-rule-review.md) states the implemented
scope and remaining limits. No generation-1 release obligation is discharged by
these generation-2 changes, and no SysML semantics work is started.

The completed operational-v2 quality audit reports 4,003 references, zero empty
candidate sets, zero ambiguous candidate sets, zero mismatched stored endpoints,
and zero mandatory lower bounds. **453 reference answers remain incomplete**,
so this is not complete structural resolution. The expanded checks also report
3,494 expression-result validation findings and 21 distinguishability findings.
The implied-naming and expression-result findings remain ordinary structural
implementation/validation work; only the independently checked Objects cases
are attributed to KNRV4-F-001. Other distinguishability findings require their
own review. The inventory identifies 258 named formal class constraints and 27
explicitly implemented validation checks without claiming complete coverage.

All 124 redefinition assertions matched by the independent reference-XMI
comparison have complete answers and matching target sets, including the exact
two KERML11-140 cases. Ordinary kernel Snapshot validation accepts the 29,087
canonical records in its isolated experiment, while accepted semantic library
publication remains false. These results are intentionally reported separately.

The final published and operational-v1 audits each retain 513 reference
obligations, including the five original unresolved redefinitions. Their
mandatory lower-bound counts are 10 and 5 respectively; ordinary strict kernel
Snapshot validation rejects both. Operational v2 retains 453 incomplete
reference obligations and zero mandatory lower bounds. The complete profile
comparison and individual disposition of all five original cases are recorded
in the [final summary](../verification/kerml-name-resolution-errata-publication-v4/summary.json).

## V5 — Reviewed operational library-content corrections

The authorized KERML11-76 correction mechanism is implemented as explicit
canonical-model changes under Operational KerML 1.0/v3. The exact pinned sources
and all historical evidence remain unchanged. This milestone does **not** accept
a semantic library publication. The independently reproduced
[KLCV5-F-001 / KERML11-68](kerml-feature-chain-end-authority-conflict.md) is a
separate authority stop: the prescribed feature-chain expansion redefines an end
with a non-end feature, violating the pinned end-conformance constraint. Current
reference implied XMI independently contains the same failing endpoint pair.

The [authority matrix](../verification/kerml-library-content-errata-publication-v5/authority-matrix.json)
reviews ten root collision patterns across all four KERML11-76 models, including
four positional-end collisions masked by incomplete naming. It classifies all
21 freshly reproduced v2 distinguishability findings individually: 18 concern
the reviewed models and three remain ordinary semantic work outside that source
correction. The issue is open; no adopted OMG correction is claimed.

The frozen manifest contains 21 source-qualified selectors and 401 operations.
An independent comparison of the Published parsed model and operational-v3 model
verifies 53 new records, no removed identities and no unreviewed changes. All
4,003 original source assertions survive unchanged: 4,000 remain active and the
three replaced Objects typing assertions remain separately inspectable. New facts
carry reviewed-correction provenance and deterministic private IDs. The complete
source/XMI comparison, both common-redefinition patterns and positional-end/result
witnesses are retained; no current KPAR or whole library file was substituted.

The full v3 quality run reports 29,140 records, 36 lossless files, zero recovery,
zero mandatory lower bounds, zero empty/ambiguous reference candidate sets and
zero mismatched stored endpoints. It still reports **453 incomplete reference
answers, 3,496 expression-result findings and six distinguishability findings**.
The executable/function count is 3,901. These are measured results, not a claim
that the required structural gate passes.

The completed obligation audit groups every incomplete reference answer under
`KQ_IMPLIED_NAMING_ORDER` and confirms ordinary kernel storage accepts all 29,140
records with its base unchanged. The historical 124 reference-XMI assertions
still match completely. Comparing all 36 current reference XMI documents matches
291 assertions; one additional current target-set relationship and two removed
named declarations are independently classified as unrelated reference changes
and excluded from the operational correction set.

Objects and VectorFunctions no longer emit distinguishability failures.
FeatureReferencingPerformances and Observation contain the reviewed corrective
facts but still expose ordinary inherited-result and feature-chain end-population
inference gaps. The other three findings concern Performances and Triggers. All
six remaining findings are category B and remain unwaived. The correction layer
does not insert additional library-specific relationships to conceal missing
ordinary inference. The independent KERML11-68 conflict, rather than these
implementation gaps or the open status of KERML11-76, is the authority stop.

| Required review question | V5 answer |
| --- | --- |
| 1. Original KPAR bytes untouched? | Yes; all 1,319 original pinned/historical artifacts pass hash preservation. |
| 2. KERML11-76 directly covers the corrected collisions? | Yes; all four named models and the explicit issue-associated commits are recorded. It remains open. |
| 3. Correction set explicit and versioned? | Yes; `agentique-kerml-1.0-operational/3`, four entries, frozen manifest digest and typed operations. |
| 4. Inserted facts distinguishable from published facts? | Yes; generic `ReviewedCorrection` origins identify profile, entry, authority, library, source key and output key. |
| 5. Objects correction minimal? | Yes; combined types, superclass/intersection relationships, common redefinitions and three typing replacements. Unrelated `that`, nested subsetting and comment changes are excluded. |
| 6. All four KERML11-76 models audited? | Yes; ten source/XMI root witnesses, original memberships/features, common ancestors, source ranges and actual diagnostics are recorded. |
| 7. Reference behavior matched without wholesale replacement? | Reviewed declaration/relationship facts match independently. Complete inferred behavior remains unfinished in two models. |
| 8. Remaining distinguishability errors zero or independently justified? | No; six ordinary implementation findings remain, individually classified and unwaived. |
| 9. Implied naming results complete? | No; the full audit retains 408 document diagnostic rows for `KQ_IMPLIED_NAMING_ORDER`. No arbitrary ordering was introduced. |
| 10. Structurally required references complete? | No; 453 answers remain incomplete despite non-empty candidate sets. |
| 11. Structural expression constraints complete? | No; 3,496 result-membership findings remain. None is waived as unevaluated execution. |
| 12. Required formal coverage sufficient? | No; all 258 named constraints are inventoried, with 28 explicit checks. Unestablished structural coverage remains required. |
| 13. Strict semantic publication passes? | No; the quality gate fails and KLCV5-F-001 independently blocks acceptance. Kernel storage acceptance is separate. |
| 14. Accepted bindings generated? | No; the historical candidate manifest remains stale and is not relabeled accepted. |
| 15. Authored projects consume an accepted publication? | No; no accepted publication/facade exists. Explicit authored profile selection is tested for all four profiles. |
| 16. Operational conclusions explain their authority? | Correction facts use the existing fact-origin and proof dependency contract, exact context/profile/content identities and frozen review entries. Contexts reject facts from a mismatched correction profile. |
| 17. Historical profile witnesses pass? | Yes; the published/v1/v2 distinctions remain, and v3 is added. The exact v2 definition is preserved with its original hash and is an unchanged prefix of the extended profile document. |
| 18. Both complete structural runtime gates green? | Yes; fresh KerML and SysML runtime commands exit 0. Strict raw metamodel conformance remains a separate failing authoring audit. |
| 19. Executable semantics explicitly deferred? | Yes; 3,901 elements are counted. No execution, simulation or SysML semantics was implemented. |

[ADR 0015](adr/0015-operational-standard-library-corrections.md) records the
correction boundary and identity design. The [v5 evidence index](../verification/kerml-library-content-errata-publication-v5/README.md)
links actual command outputs, failed/superseded attempts, independent verification,
quality classification, constraint coverage and the separate authority stop.
Workspace/front-end/end-to-end checks and strict Rustdoc pass. The stale binding
check and strict raw conformance failures remain explicit; generation-1 release
obligations and the historical independent fixture diagnostic are preserved.

## V6 — Explicit validation erratum and independent subobject authority stop

Operational v4 implements exactly the reviewed KERML11-68 owner restriction.
The official Git commit, validator diff, changed reference tests, pinned formal
rule, preliminary wording, current implied-XMI witness and 220 arbitrary-name
canonical cases are independently verified. Published through v3 retain their
end-conformance behavior. All source/model correction IDs remain those of v3;
the validation-only successor carries a distinct semantic context and manifest.

Ordinary implementation advanced: owned specialization order is projected from
stored ownership; canonical inherited result memberships participate in positional
result redefinition; feature-chain terminal features supply structural end
populations; and implied names are complete when all possible targets agree.
Different possible names remain typed ambiguity, with no ID/order tie-break.
Additional local structural constraints are checked. These changes do not claim
full structural closure or full positional reference-XMI agreement.

The full inventory independently reproduced
[KLCV6-F-001 / KERML11-205](kerml-subobject-specialization-authority-conflict.md):
the subobject specialization rule's prose and formal target disagree. The actual
pinned subobjects declaration is a witness using explicit facts and reflexive
specialization, independent of incomplete inference. This new authority choice
is outside the four authorized errata. It is the stop reason. Ordinary incomplete
queries and remaining implementation workload are not stop reasons.

Fresh grouped measurements, retained failures, exact commands and exit codes are
in the [v6 evidence index](../verification/kerml-semantic-closure-v6/README.md).
No historical count is used as an expected final result.

The final audit measures 29,140 canonical records and 4,000 references: four
unresolved, 60 incomplete reference answers, zero ambiguous candidate sets and
zero stored endpoint mismatches. Two distinguishability findings remain in
Triggers. Observed expression-result cardinality findings fall to zero, while a
new end-membership check exposes 180 construction defects. The 258-constraint
inventory has 38 explicit checks; full structural validation is not established.
All remaining diagnostics are individually classified as implementation defects
or missing structural rules. The 3,901 executable/function elements remain a
separate deferred population.

| Required review question | V6 answer |
| --- | --- |
| 1. Pinned source bytes unchanged? | Yes; the v6 starting-byte preservation audit checks all 1,692 original files. Historical v5 acquisition-index discrepancies are separately retained and documented. |
| 2. Published KERML11-68 behavior reproducible? | Yes; Published and v1–v3 retain the unconditional implication in the five-profile matrix. |
| 3. V4 exactly implements the reviewed validator change? | Yes; the owning-Type restriction matches the independently verified commit; missing facts remain incomplete. |
| 4. Association/Connector end redefinitions still checked? | Yes, including AssociationStructure and BindingConnector metaclass subtypes. |
| 5. StatePerformances passes without source rewrite? | Its independently captured prescribed/reference end pair passes the v4 rule and fails the historical rule with unchanged flags. Full Agentique feature-chain structural construction remains unfinished. |
| 6. Implied positional naming complete? | Not established. Ordered ownership and name agreement/typed ambiguity are implemented; corpus closure and the complete independent target comparison remain obligations. |
| 7. All required references Complete? | No. Four unresolved references and 60 incomplete reference answers remain; nonempty candidates never count as success. |
| 8. All distinguishability findings resolved? | No. Two findings remain in Triggers. |
| 9. Expression results valid? | The complete recount has zero result-cardinality findings. Full structural expression coverage remains unestablished, so semantic acceptance is withheld. |
| 10. Feature-chain interpretation complete? | No. Terminal end population is improved, but the full prescribed structural expansion and evidence remain work. |
| 11. Required formal validation coverage complete? | No. There are 38 explicit checks among 258 inventoried constraints. The independent target conflict blocks authority closure and other structural coverage remains required. |
| 12. Strict semantic publication passes? | No; no language-semantic acceptance operation issues acceptance in this change. Kernel storage acceptance is separate. |
| 13. Accepted immutable facade published? | No. `LoadedKermlStandardLibraries` is withheld. |
| 14. Accepted bindings current? | No. Regeneration is withheld until accepted publication; the historical candidate manifest is unchanged. |
| 15. Authored projects consume accepted libraries? | No. Explicit v4 authored profile metadata is tested; accepted library consumption remains unavailable. |
| 16. Corrections/proofs explainable? | Yes for the implemented correction: profile, manifest, rule, endpoint flags, actual owning Type/metaclass, fact origins and search dependencies are retained. Full publication dependency coverage is not claimed. |
| 17. Historical profiles reproducible? | Yes for the frozen profile matrices and manifests. Ordinary query implementation has an independently versioned rule-set identity. |
| 18. Execution still deferred separately? | Yes. Structural deficiencies are not moved into the execution count. |
| 19. Both complete structural runtime gates green? | Yes; both fresh runtime commands exit 0. Strict published metamodel authoring conformance remains separately failing. |

See [ADR 0016](adr/0016-operational-semantic-validation-corrections.md) for the
v4 authority boundary. No Systems Library publication, SysML semantics/frontend,
repository/API, transforms, simulation or application migration is introduced.

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

## V7 — exact formal targets and independently isolated result-binding conflict

Operational v5 implements the six explicitly authorized target replacements in
KERML11-205/206/207. The typed contracts, profile-scoped canonical bindings,
manifest, arbitrary-name and six-profile tests are described in
[ADR 0017](adr/0017-operational-formal-constraint-target-corrections.md).
Published/v1–v4 retain their formal targets. Pinned source declarations and all
prior correction manifests remain unchanged.

The generic EndFeatureMembership constructor now sets `isEnd` on the Feature it
newly constructs and owns. The complete lower-corpus audit checks all 180 such
memberships with zero end-flag findings. It never changes a referenced or inherited
Feature. Implied library-base redundancy also follows required bases of general
types, with bounded memoization. This does not establish full positional or
feature-chain closure. Unestablished effective owner typing is explicitly incomplete.

The independent inventory exposes **KLCV7-F-001 / KERML11-145** on the exact
pinned `ControlFunctions::'.'` result expression. The formal result binding must
be an owned Feature relating raw result identities. Its required Function domain
is incompatible with the nested result's expression domain. The reference instead
uses plain OwningMembership and the nested-expression domain, which does not
satisfy the literal ownedFeature selection. A chain also changes the required
related Feature identity. [The full proof](kerml-result-binding-authority-conflict.md)
distinguishes these alternatives and supplies independent pinned source/XMI and
arbitrary-name canonical witnesses. This is not an implementation workload stop.
No KERML11-145 correction is applied.

Fresh measurements, rule coverage, command results and every failed/superseded
attempt are retained under [v7 verification](../verification/kerml-semantic-closure-v7/README.md).
The inventory conservatively retains structural obligations rather than asserting
non-applicability from an unfinished construction. No structural graph rule is
hidden under execution deferral or accepted merely because a candidate exists.

| Required review question | V7 answer |
| --- | --- |
| 1. Pinned standards/library bytes unchanged? | Yes. The preservation audit checks all starting bytes and new captures. The inherited v5 acquisition-index discrepancies already present on main are reported separately, without rewriting history. |
| 2. Does v5 correct only 205/206/207? | Yes: six exact typed rule-target substitutions, in addition to the inherited v1–v4 lineage. |
| 3. Subobject target is Objects::Object::subobjects? | Yes, including reflexive canonical witnesses and the independently verified exact pinned declaration. Published through v4 retain the wrong formal target. |
| 4. Subperformance/owned/enclosed targets correct? | Yes: subperformances, ownedPerformances and enclosedPerformances under their exact owning paths. |
| 5. Portion/suboccurrence targets correct? | Yes: Occurrences::Occurrence::portions and Occurrences::Occurrence::suboccurrences. The issue's own spelling error is retained as evidence, not copied into the implementation. |
| 6. All EndFeatureMembership lowering findings gone? | Yes for the complete 180-membership pinned construction audit. Full accepted-publication construction remains withheld. |
| 7. Unresolved required references zero? | No: four remain in the fresh v5 audit. Renamed inherited-member closure is still required. |
| 8. Incomplete required answers zero? | No: 64. Six Clocks answers become complete, while ten Observation answers expose missing effective owner typing under the additional formal implications. Supporting incompleteness is retained. |
| 9. Implied positional semantics complete? | No. The inherited-parameter experiment was removed because the formal rule requires the general type's owned-parameter projection. Full independent target comparison remains required. |
| 10. Triggers distinguishability findings gone? | No: two remain. They are ordinary inference work, not a new erratum. |
| 11. Feature-chain structural expansion complete? | No; all required canonical relationships and their evidence are not yet established. |
| 12. Expression structural coverage complete? | No. The fresh audit preserves zero result-cardinality findings, but binding/domain and all-family structural validity are not established. The new conflict is exercised by a result expression. |
| 13. All 258 constraints classified with no unknown structural coverage? | Every rule is inventoried: A=38, B=213, E=6, F=1. B obligations remain explicit and the publication coverage gate fails. This is not a closed final acceptance classification. |
| 14. Strict semantic publication passes? | No. No accepted Snapshot is issued; kernel construction and runtime coverage are separate. |
| 15. Accepted bindings current? | No. The historical candidate manifest remains unchanged; accepted regeneration is withheld. |
| 16. LoadedKermlStandardLibraries available? | No accepted facade is exposed. |
| 17. Authored projects consume accepted libraries? | No. Six-profile SourceProject selection is tested, but is not accepted-library consumption. |
| 18. Operational conclusions explainable? | Implemented corrections carry typed rule, exact target, profile/manifest, canonical facts and search dependencies. Full publication dependency closure remains required. |
| 19. Historical profiles reproducible? | Yes for frozen identities/manifests and permanent historical plus six-profile tests. Ordinary query implementation has a separately versioned rule identity. |
| 20. Executable semantics explicitly deferred? | Yes. No runtime values are evaluated and no structural incompleteness is moved into that category. |
| 21. Both structural runtime gates green? | Yes; both complete commands exit zero. Strict raw authoring conformance remains a separate nonzero report. |

The scope prohibitions remain in force. No Systems Library semantic publication,
SysML semantic crate/frontend, repository/API, diagrams, transformations, execution,
simulation or application migration is introduced.

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

## V8 — authority preflight isolates a separate inner reference-binding conflict

The v8 task authorizes all five KERML11-145 rule families. Their exact published
authority, proposed contextual-result interpretation, pinned witnesses and
nonuniform reference behavior are frozen in the
[five-rule matrix](../verification/kerml-semantic-closure-v8/authority-matrix.json).
KERML11-145 is no longer an unauthorized stop. It remains an open OMG issue.

The mandatory up-front sweep cross-references all 258 formal constraints against
410 retained official issue records and exposes **KLCV8-F-001 / KERML11-8** early.
This is the inner FeatureReferenceExpression binding in the exact pinned
ControlFunctions declaration, not the outer Function binding from v7. The
[independent proof](kerml-feature-reference-binding-authority-conflict.md)
includes result subsetting, result/end positional redefinitions, base
specializations, ordered chain structure and complete local domain closures.
It also constructs the authorized outer contextual-result direction separately:
the outer domain becomes valid while the inner connector still fails.

An arbitrary-name canonical fixture checks complete supporting queries under
Published and v1–v5. No Function-to-Expression specialization, result identity
change, invented domain or weakened connector check is used. Correcting the
inner connector requires an additional rule-family decision, outside the exact
five-rule authorization. The stop occurs during Gate 1, before v6 registration.
No unimplemented profile is presented as a usable successor.

The [authority map](../standards/kerml-1.0-constraint-authority-map.json) also
exposes other known issues as unapproved risk metadata. Null applicability means
unproven at this stop, not non-applicability or execution deferral. The remaining
renamed-member, incomplete-answer, Triggers, feature-chain, positional and
validation work remains ordinary implementation work.

Fresh corpus measurements and the complete verification command results are
recorded in [v8 verification](../verification/kerml-semantic-closure-v8/README.md).
Historical measurements are comparisons, not expected counts. The semantic
coverage and publication gates remain explicitly failed.

| Required review question | V8 answer |
| --- | --- |
| 1. All source/spec/KPAR bytes unchanged? | Yes. The v8 preservation audit checks 2,610 starting standards/evidence files and all new acquisitions. Historical acquisition-index discrepancies already present on clean main are separately retained. |
| 2. Does v6 correct exactly the five KERML11-145 families? | No implemented v6 exists at this Gate 1 stop. Exactly those five families are authorized and individually documented; no sixth correction is adopted. |
| 3. Raw result identity distinct from contextual result identity? | Yes in the authority design and independent/canonical witnesses. No production contextual-result API is claimed. |
| 4. Feature valuation uses the correct contextual result? | Not implemented in Agentique v6. The matrix records the required Subsetting and the narrower pilot antecedent. |
| 5. Expression result binding satisfies domain/ownership semantics? | The correction direction is documented; production implementation and its full profile matrix remain outstanding. |
| 6. Function result binding satisfies domain/ownership semantics? | The independent contextual outer graph satisfies the domain proof. Production v6 implementation remains outstanding. |
| 7. ControlFunctions::'.' passes without source rewriting? | No. Its separate inner reference binding independently fails under KERML11-8 even after the outer correction is represented. The source remains unchanged. |
| 8. Index/Select use correct result semantics? | Their distinct authorities and required chain direction are retained; implementation is outstanding. The pilot's KERML11-69 Collection guard is not silently adopted. |
| 9. Unresolved references zero? | No; the fresh reference-obligation audit retains ordinary unresolved structural obligations. |
| 10. Incomplete required references zero? | No; incomplete support remains incomplete even with a unique candidate. |
| 11. Distinguishability findings zero? | No; Triggers inference remains ordinary work, not a new source erratum. |
| 12. Feature-chain structural expansion complete? | Complete for the independently evaluated local authority chain; full corpus expansion remains outstanding. |
| 13. Implied positional rules complete? | Result/end redefinitions are included in the local proof; full corpus closure remains outstanding. |
| 14. All 258 finally classified without unimplemented structural categories? | No. Every rule is inventoried, but the final coverage gate is open. The authorized KERML11-145 Function rule moves from the old conflict to pending implementation; KERML11-8 is the new independent conflict. |
| 15. Strict semantic publication passes? | No accepted semantic publication is issued. |
| 16. Accepted bindings current? | No. The stale candidate manifest is checked and preserved; accepted regeneration requires acceptance first. |
| 17. LoadedKermlStandardLibraries available? | No accepted facade is exposed. |
| 18. Authored projects consume it? | No accepted-library integration is claimed. Existing profile/project regression checks remain separate. |
| 19. All operational rules explainable? | Existing implemented corrections retain their provenance. The proposed five-rule authority is explicit; complete v6 production explanations are not implemented. |
| 20. Historical profiles reproducible? | Frozen identities/manifests and prior evidence are preserved; six-profile historical regressions and the new canonical witness are rerun. |
| 21. Execution semantics explicitly separate? | Yes. No structural rule is moved into DeferredExecution and no runtime evaluation is introduced. |
| 22. Both structural runtime gates green? | Yes. Both full `--require-runtime --check` commands exit zero. Raw `--require-conformance` exits remain separately nonzero. |

[ADR 0018](adr/0018-operational-result-domain-corrections.md) records the
authorization and the precise stop boundary. The scope prohibitions remain in
force, and ordinary structural implementation remains required after this new
authority conflict is resolved.

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**

## V9 — narrow reference-binding correction and independent cross-feature stop

Operational v6 is registered with separately pinned result-domain and
reference-binding manifests. Its typed implied-binding role retains the raw
referent/result identities, and only the expression-owned result endpoint can
qualify for the reviewed featuring exception. Ordinary connector featuring
validation remains strict. Direct FeatureChainExpression bindings were reviewed
separately and excluded. The five KERML11-145 structural producers and ordered
contextual-result primitive are implemented and tested under all seven profiles.
This is not a completed corpus publication.

The applicability review independently establishes **KLCV9-F-001 / KERML11-1**
in the unchanged incoming/outgoing self-transfer end values in Occurrences.
The literal published cross-feature selector selects the value Expression. Even
if its misplaced FeatureValue exclusion is interpreted as a membership exclusion,
it selects the required value BindingConnector instead. Both require the exact
containing-feature domain while cross-feature semantics require a different
domain. The literal Cartesian branch also fails. Complete local source/reference
facts and an arbitrary-name canonical seven-profile reproduction are retained in
the [authority report](kerml-cross-feature-authority-conflict.md).

The current pilot excludes these members, but that changes cross-feature
selection and is outside v1–v6. The issue remains open. No correction is adopted.
This new independently proven contradiction is the stop reason. The ordinary
reference, Triggers, chain, positional, validation and publication work remains
required; none is treated as a new authority conflict or runtime evaluation.

The fresh unchanged-v5 audit measures 29,140 canonical records, 4,000 references,
4 unresolved references, 64 incomplete reference answers, no ambiguous candidate
sets or stored endpoint mismatches, no EndFeatureMembership or result-cardinality
findings, 2 distinguishability findings, and 3,901 unevaluated executable elements.
The inventory still contains 258 named constraints and 410 retained tracker
issues. These are fresh measurements, not asserted historical expectations, and
are explicitly not represented as measurements of a completed v6 expansion.

| Required review question | V9 answer |
| --- | --- |
| 1. All pinned bytes unchanged? | Yes; preservation verifies all 2,848 starting standards/evidence files and 18 new acquisitions. No source/spec/library bytes are patched. |
| 2. KERML11-8 accurately open? | Yes; current issue status, update date, pilot, reference XMI and preliminary evidence are retained separately. |
| 3. V6 keeps raw referent/result identities? | Yes; the reference binding retains both. Contextual results belong to the separately reviewed result-domain rules. |
| 4. Featuring exception limited to the reviewed role? | Yes; typed rule provenance, exact endpoints, membership ownership, binary shape and selected context are checked. |
| 5. Ordinary bindings still strictly checked? | Yes for featuring conformance; authored, unclassified implied and other-role connectors receive no exception. Full corpus constraint acceptance is not claimed. |
| 6. FeatureChainExpression independently justified? | Its direct case is excluded; nested FeatureReferenceExpressions qualify only through their own role. |
| 7. All five KERML11-145 rules implemented? | Five distinct producers and the seven-profile fixture matrix pass. Full corpus structural expansion/validation remains unclosed at the new conflict. |
| 8. ControlFunctions::'.' passes inner and outer rules? | The exact authority graph and independent canonical inner/outer tests establish the two corrections separately; no full accepted-corpus pass is claimed. |
| 9. Unresolved references zero? | No; the fresh unchanged-v5 baseline has 4. Renamed-member lookup remains ordinary work. |
| 10. Incomplete required references zero? | No; the fresh unchanged-v5 baseline has 64. A unique incomplete answer remains Incomplete. |
| 11. Distinguishability findings zero? | No; the two Triggers findings remain ordinary inference work. |
| 12. Feature-chain expansion complete? | The reusable contextual-result chain is implemented; complete corpus chain closure is outstanding. |
| 13. Positional semantics complete? | Existing positional behavior is regression-tested; full corpus differential closure is outstanding. |
| 14. All 258 statuses closed? | No; the gate remains failed at KLCV9-F-001. Missing structural implementation is explicit. |
| 15. Exercised open-issue risks disposed? | KERML11-8/145 are covered by the authorized profile; KERML11-1 has a complete positive conflict proof. The remaining applicability review is not claimed complete. |
| 16. Strict semantic publication passes? | No accepted publication is issued. |
| 17. Accepted bindings current? | No; accepted regeneration requires publication first. Historical bindings remain preserved. |
| 18. LoadedKermlStandardLibraries available? | No accepted facade is issued. |
| 19. Authored projects consume it? | No accepted-library consumption is claimed. |
| 20. Historical profiles reproducible? | Frozen manifests/identities are preserved; seven-profile boundaries and existing historical regressions are tested. |
| 21. Executable semantics separate? | Yes; no values execute and no structural constraint is reclassified as DeferredExecution. |
| 22. Both structural runtime gates green? | Yes; both complete runtime commands exit zero. Strict Rustdoc also exits zero. These remain separate from library acceptance and raw metamodel conformance. |

[ADR 0019](adr/0019-operational-reference-binding-correction.md) records the
operational decision. [V9 verification](../verification/kerml-semantic-closure-v9/README.md)
records actual commands, exit codes, failures, repairs and the precise uncompleted
gates. Systems Library publication, SysML semantics/frontends, persistence/APIs,
execution and application migration remain outside this change.

**KERML OPERATIONAL LIBRARY FOUNDATION INCOMPLETE — DO NOT PROCEED**
