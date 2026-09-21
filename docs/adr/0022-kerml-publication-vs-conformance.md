# ADR 0022: canonical KerML publication and conformance are separate

Status: accepted. Normative scope: pinned KerML 1.0, Operational v7, and SysML 2.0.

## Three acceptance levels

`StructurallyValidSnapshot` means an ordinary kernel `Snapshot` produced by strict
`apply`. A construction preview or a successful parser run is insufficient. The
kernel remains language agnostic and none of its invariants are relaxed.

`CanonicalKerMlPublication` means an immutable semantic dependency safe for
queries, authored KerML, future SysML rules, bindings and repository storage. Its
graph has one deterministic interpretation under an exact profile and library set.
It does not mean that every KerML constraint has an executable validator.

`KerMlConformanceReport` records checked constraints, diagnostics, coverage and
authority conflicts separately. `ValidationCoverage::Incomplete` is permitted on
a canonical publication when each gap or failure is demonstrably independent of
the canonical graph. Coverage is not inferred from a successful command exit.

## Publication acceptance

All of the following are required:

1. Strict kernel Snapshot validity.
2. Exact operational profile identity.
3. Exact pinned library-set identity, distinct from artifact/specification IDs.
4. Deterministic canonical element and relationship IDs.
5. Exact provenance and source mapping, including reviewed corrections.
6. No unresolved mandatory structural references.
7. No ambiguous mandatory structural references.
8. No invalid or incomplete mandatory structural references.
9. Complete namespace, import, alias and visibility semantics for lookup.
10. Complete specialization, subsetting and redefinition for effective structure.
11. Complete effective membership and feature semantics needed by SysML.
12. Complete essential typing and featuring, including variable domains.
13. Deterministic required implied facts and relationships.
14. Explicit retained authority conflicts with publication relevance.
15. No invented source facts or unreviewed authority correction.
16. No contradiction making essential graph interpretation unusable.

Exhaustive validation, runtime expression evaluation, validation of every possible
KerML model, and negative applicability proofs for every open OMG issue are not
publication requirements. A symbolic multiplicity bound remains a canonical
expression; evaluating its value is a separate obligation.

## Relevance and authority

Each inventory rule has one relevance: `PublicationCritical`, `ValidatorOnly`,
`ExecutionDependent`, `AuthorityBlocked`, or evidenced `NotApplicableToCorpus`.
Authority-blocked rules additionally state whether the disagreement changes the
published graph. A missing validator is distinct from a missing producer or query
implementation for the same formal rule. A constraint's spelling (`check`,
`derive`, `validate`) alone never proves its relevance or implementation status.

Identity, relationships, mandatory endpoints, membership, inheritance, typing,
featuring, structural multiplicity bounds and essential derived facts are
publication critical. Constraints that only judge an already determined graph
remain conformance obligations. Mixed execution/structural rules keep their
structural obligation; the execution label cannot defer graph construction.

An authority disagreement that only changes a conformance verdict is a
`ValidationOnlyAuthorityConflict`. A disagreement changing canonical members,
required relationships or essential domains is a
`PublicationBlockingAuthorityConflict`. Existing query behavior is not authority
to choose between two conflicting published definitions. No new operational
correction is authorized by this ADR.

## Overlay boundary

`PartialDerivationOverlay` carries produced facts without claiming all required
implications exist. `CompletePublicationOverlay` requires independently checked
closure of publication-critical producer families on an exact Snapshot/profile/
library set. A producer's local success is insufficient. Queries preserve their
completeness, evidence and search dependencies in either phase.

`validateElementIsImpliedIncluded` is an interchange/completeness assertion. A
partial staging overlay can legitimately contain some implied relationships
while its source `isImpliedIncluded` remains false. Report that phase explicitly;
do not change the source flag or treat the staging mismatch as thousands of
independent contradictions. Complete publication requires its own closure proof.
Exporting a model that asserts implied inclusion requires the corresponding
serialization contract and validation.

## Dependency boundary

Canonical records remain in the kernel; semantics depend inward. An eventual
`CanonicalKermlStandardLibraries` owns the strict Snapshot, complete overlay,
profile, bindings, library-set identity, source map and semantic digest. Its
constructor must check the acceptance contract rather than accept caller-supplied
success booleans. Authored projects share it immutably and preserve standard IDs.
No accepted facade or bindings may be issued while publication-critical gates
fail. This is independent of the conformance coverage count.

The retained issue map is an index for specific questions. No exhaustive issue
acquisition or applicability sweep is required. ADR 0021 governs new evidence.
