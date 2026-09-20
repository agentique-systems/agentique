# ADR 0017: Operational formal-constraint target corrections

Status: Accepted for Operational KerML 1.0/v5; standard-library publication withheld.

## Context

The pinned KerML 1.0 formal constraints contain six incorrect library targets.
The project explicitly authorizes the exact corrections documented in open OMG
issues KERML11-205, KERML11-206 and KERML11-207. This is an Agentique operational
decision, not an assertion that OMG has adopted the issues into KerML 1.0.

[The independent matrix](../../verification/kerml-semantic-closure-v7/authority-matrix.json)
retains each source-qualified rule ID, literal formal body, prose, incorrect and
corrected target, issue status and response hash, pinned ZIP declaration, reference
commit/map, and inspected reference tests where available. KERML11-206 itself
misspells the package name; the exact `Occurrences` spelling is independently
established by the pinned declarations, normative prose and reference map.

## Decision

Create `agentique-kerml-1.0-operational/5`, extending v4 with only these changes:

| Formal constraint | Published target | Effective v5 target | Issue |
| --- | --- | --- | --- |
| checkFeatureSubobjectSpecialization | Occurrence::Occurrence::suboccurrences | Objects::Object::subobjects | 205 |
| checkStepSubperformanceSpecialization | Performances::Performance::subperformance | Performances::Performance::subperformances | 205 |
| checkFeaturePortionSpecialization | Occurrence::Occurrence::portions | Occurrences::Occurrence::portions | 206 |
| checkFeatureSuboccurrenceSpecialization | Occurrence::Occurrence::suboccurrences | Occurrences::Occurrence::suboccurrences | 206 |
| checkStepOwnedPerformanceSpecialization | Objects::Object::ownedPerformance | Objects::Object::ownedPerformances | 207 |
| checkStepEnclosedPerformanceSpecialization | Performances::Performance::enclosedPerformance | Performances::Performance::enclosedPerformances | 207 |

The [manifest](../../standards/kerml-1.0-operational-formal-target-errata-v5.json)
is hashed into the profile identity. Published and v1–v4 retain their original
target contracts and manifest identities. The default `OPERATIONAL` remains v2.
V3 library correction facts preserve their original IDs and provenance under v5.

`FormalConstraintId` is a closed typed rule identity. Its contract binds an exact
owned path once against canonical memberships, checking uniqueness, visibility,
metaclasses and declared library identity. There is no alternate path search.
The bound target is a canonical `ElementId`; validators and implied specialization
use that identity, not a declaration's display name. Missing and invalid targets
remain explicit. Proofs include canonical facts, the typed target search dependency
and the profile-qualified semantic context. Reflexive specialization is allowed.

The implementation follows the retained normative semantic antecedents; it does
not claim to execute the malformed published OCL text as a program. Effective
owner typing that has not been established remains `Incomplete`, including when
a direct-typing lookup alone cannot establish the antecedent. An unproved negative
antecedent cannot become a vacuous successful validation.

This decision changes no pinned PDF, XMI, KPAR, canonical source declaration,
generic name-resolution algorithm or kernel storage rule. The ordinary end-member
lowering fix is a separate implementation correction: only the newly constructed
owned Feature receives `isEnd`; reused or inherited Features are not mutated.

## Consequences and verification

Permanent tests cover all six profiles, all six rules, arbitrary subject names,
reflexive targets, missing targets, wrong metaclasses, wrong libraries, private or
duplicate declarations, and profile mismatch. The historical v6 subobject witness
remains unchanged. Pinned corpus tests bind all six corrected targets and check
every EndFeatureMembership's end flag.

No other open issue is implicitly authorized. The independent inventory exposes
[KLCV7-F-001 / KERML11-145](../kerml-result-binding-authority-conflict.md) on the
exact pinned `ControlFunctions::'.'` declaration. It requires a different choice
about result-binding ownership/domain or related-feature identity, beyond a literal
target correction. Publication therefore remains withheld. Remaining structural
implementation obligations retain their category B status; they are neither
execution deferrals nor additional authority conflicts.

There is no accepted Snapshot, accepted binding regeneration, accepted library
facade or authored-project consumption of an accepted publication in this decision.
The canonical model remains the only semantic truth. Execution and SysML semantic
work remain outside this milestone.
