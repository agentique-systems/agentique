# ADR 0015 — Reviewed operational standard-library model corrections

Status: implemented for the explicit Operational KerML 1.0/v3 construction;
semantic publication remains blocked by KLCV5-F-001 (KERML11-68).

## Authority and decision

KERML11-76 directly identifies inherited-member collisions in
FeatureReferencingPerformances, Objects, Observation and VectorFunctions. The
issue is open. Agentique's project authority authorizes a reviewed operational
correction; this is **not an adopted OMG correction**.

The pinned KerML 1.0 sources, archives, metamodel and specifications remain
unchanged. Parsing produces the same source declarations and identities. An
explicit canonical-model transform then constructs a separate operational
candidate. No syntax nodes or replacement library files simulate published
declarations that did not exist.

The frozen [v3 manifest](../../standards/kerml-1.0-operational-library-errata-v3.json)
contains four entries, exact source-qualified selectors and typed operations.
Its SHA-256 is
`c762275c6a7c4cb310c4ba3084683d62125a9319228d8653f39956c61794ae30`.
Runtime registration rejects changed manifest bytes. Acquisition is an explicit
maintenance activity, never a build step.

## Model changes

The implemented operations are declaration creation, metaclass correction,
scalar/reference replacement and ordered relationship append. Each operation
exists because the reviewed correction requires it. Selectors check canonical
ID, metaclass, document identity, source hash, byte range and original library
origin. Display names do not select runtime inputs.

Objects receives three combined Structures, their superclass/intersection
relationships and common dimension redefinitions. The three existing feature
typing endpoints change to the corresponding combined type. The intersections
are corroborated by commit `692849294651ad8031f25f4323d38e51acb0a1dc` and preserve
the combined type's extent. Changes to `that`, nested subsettings and comments
are excluded. This is the minimal reviewed combined-type repair, rather than a
replacement of the current Objects file.

FeatureReferencingPerformances receives the Expression/return-membership
correction, a common Boolean result, and the two ordered ends in each monitor's
`endWhen`. Observation receives the missing second `target` end. VectorFunctions
receives the common dimension redefinition. The source spelling
`CartesianThreeVectorValue::dimension` resolves to the inherited declaration
owned by `ThreeVectorValue`; no inherited feature is copied.

The [authority matrix](../../verification/kerml-library-content-errata-publication-v5/authority-matrix.json)
and [independent four-model witnesses](../../verification/kerml-library-content-errata-publication-v5/four-model-witnesses.json)
retain source, XMI, historical commit and individual diagnostic evidence.

## Provenance and identity

The language-neutral kernel adds `DeclaredOrigin::ReviewedCorrection`, containing
profile, entry, authority references, originating LibraryId, source key and
output key. The kernel contains no KerML issue-specific enum or algorithm.
Introduced facts have this origin, not `StandardLibrary`. Unchanged input facts
retain their exact origins. Corrected existing facts retain their identities but
carry correction provenance. Original source assertions remain inspectable;
replaced typing assertions are exposed as `superseded_references` and do not
pretend to assert the operational endpoint.

New IDs use a private UUID-v5 domain over an unambiguous encoded tuple of identity
schema, operational profile, exact library set, originating library, entry and
operation path/output role. Reference-XMI IDs are comparison evidence only.
Changing a profile/content identity changes generated IDs. A frozen profile
cannot silently acquire different semantics.

Query evidence already records FactKeys, declared origins, surrounding positive
and search dependencies, and the exact SemanticContext. Correction facts flow
through that same contract. Context identity contains the correction manifest
digest and library pins; the library graph digest includes correction records.
Synthetic tests check entry/output provenance and context dependencies.

## Profiles and acceptance

Published, operational v1, v2 and v3 remain distinct. V3 retains v1's descriptor
correction and v2's redefinition algorithm. V1/v2 manifests and identities are
unchanged. The default `OPERATIONAL` constant stays at v2; v3 is selected
explicitly. Synthetic collisions are not automatically waived under any profile.

The independent Python verifier compares every exported element, slot and origin
against the reviewed operations without calling transform helpers. It finds
53 introduced records, no removed IDs and no unreviewed changes. Canonical
construction, query completeness and language publication are separate gates.

No accepted bindings, accepted library facade or accepted authored-library
dependency is issued while semantic publication fails. The separately reproduced
[KERML11-68 conflict](../kerml-feature-chain-end-authority-conflict.md) requires
new authority before its semantic correction can be chosen. Outstanding ordinary
naming, expression and formal-validation work remains explicit; it is not
reclassified as execution or waived by this ADR.
