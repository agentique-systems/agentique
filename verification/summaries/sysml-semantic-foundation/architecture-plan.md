# SysML foundation implementation seams

This is a proposed implementation plan under ADR 0012, not accepted publication
or implemented semantic behavior. Production semantic work starts only after
K10–K13 accept the real KerML corpus and authored integration.

## S0–S3: one graph and a composed context

`agq-sysml` already provides all 93 SysML descriptors and borrowed views. Retain
`{ElementId, &ModelView}` and the language-agnostic kernel records. Add
`agq-sysml-semantics` depending on kernel, KerML descriptors/semantics and SysML
descriptors. Neither parsing nor transport belongs in that crate.

Two concrete integration seams need attention:

1. `agq_sysml::descriptors()` currently starts with the published
   `agq_kerml::descriptors()`. Operational SysML construction must combine
   `agq_kerml::descriptors_for_profile(v9)` with
   `agq_sysml::own_descriptors()`, preserving shared descriptor identities and
   validating the exact combined graph. Do not silently use published KerML
   metadata for a dependency accepted under v9.
2. `CanonicalKermlStandardLibraries` lives in `agq-kerml-text`. A SysML semantic
   crate must not depend on that frontend just to consume its acceptance identity.
   Expose a parser-independent, immutable accepted dependency contract from
   `agq-kerml-semantics`; the source facade can issue it only after its existing
   producer/capability/source/mandatory-reference checks. A raw
   `CompletePublicationOverlay` alone does not certify those source/reference
   checks. Preserve the facade's unforgeable acceptance boundary.

A composed SysML context holds the existing KerML context/query evaluator plus
the SysML rule-set identity, combined descriptor digest, exact Systems candidate
or accepted identity, and validated `StandardSysmlBindings`. Its full identity
includes the authored/current revision, accepted KerML publication digest,
explicit operational profile, KerML library-set identity, both language rule-set
identities, descriptor graph and source set. Candidate and accepted Systems
states must remain distinct.

Reject a supplied dependency with a different publication digest, operational
profile, library set or descriptor graph before query construction. Use the
existing immutable dependency mechanism; local records may reference library
IDs, but cannot edit their ownership, slots or derived facts. Two authored
projects share immutable library storage and keep independent revisions. No
lookup may make the standard-library roots see authored roots.

Reuse `QueryResult`, completeness, evidence, positive reads and search dependencies.
Filtering a KerML result by SysML metaclass must preserve its incompleteness and
account for descriptor reads. An incomplete base query must never become Complete
just because the filter returned no elements.

## S4–S7: shared source and textual architecture

SysML 2.0 §8.2.2.1.2 explicitly reuses KerML lexical structure with different
reserved keywords and special terminals. Reuse
`crates/kerml-syntax/src/lexer.rs`, source/revision identity, `ByteRange`,
`SourceOrigin` and syntax reconciliation. Make keyword recognition dialect-aware:
SysML permits names reserved only by KerML, and reserves its own added keywords.
Do not change KerML name acceptance globally.

Reuse the lossless `production::Document` arena and iterative chart machinery in
`crates/kerml-syntax/src/production/`. Parameterize the generated grammar vocabulary
and root/keyword set, preserving KerML parsing unchanged. Common package, import,
alias, annotation, name, specialization, multiplicity and expression productions
should share their existing implementation. SysML Definition/Usage and their
specialized body/member productions extend that grammar. Syntax nodes retain
their grammar role and source origin; they are never semantic model records.

`SourceProject` currently stores SysML with `FrontendUnavailable`, while authored
KerML uses a bounded declaration frontend. Full library KerML uses the production
arena and `library/construction.rs`. Introduce a shared canonical construction
interface rather than translating SysML into fake KerML source or expanding the
bounded authored AST into a second full parser. Library provenance and authored
edit identity policies remain distinct inputs to the same kernel construction.

The current library builder fixes its registry to KerML and dispatches through
`library/vocabulary.rs`. Generalize that seam to the combined descriptors and
SysML production-to-metaclass/relationship mappings. Reuse ownership assembly,
pending endpoint assertions, source maps, imports, FeatureTyping,
Subclassification, Subsetting and Redefinition. Preserve relationship identities
and source syntax identities; never copy inherited records.

Implement parser slices in this order, then run all 21 documents:

| Slice | Exact corpus drivers | Required added grammar |
| --- | --- | --- |
| 1 | Attributes, Metadata, SysML, Items, Parts, Ports | Definition/Usage prefixes, metadata/item/part/port/attribute, anonymous reference/parameter usages, derived/abstract/end/direction modifiers |
| 2 | Connections, Interfaces, Flows, Allocations | Connection/end/port members, connect, message/flow, event occurrence, succession and symbolic bounds |
| 3 | Calculations, Constraints, Requirements, Cases, AnalysisCases, UseCases, VerificationCases | Calculation/result bodies, assert, subject/objective/requirement members, analysis/verification/use case, enum literals, metadata |
| 4 | Actions, States | Assignment, perform, while, action successions, entry/do/exit members, binding connectors |
| 5 | Views, StandardViewDefinitions | View/viewpoint/rendering, short names, satisfy/require bodies; resolve the recorded published grammar discrepancy explicitly |

The parser goal is 21/21 complete production trees, zero unexplained recovery and
exact bytes. Lexical inventory success does not satisfy that gate. Structural
expression construction is required; execution/evaluation is a later milestone.

## S8–S15: semantic foundation and authored vertical

Construct an unpublished Systems candidate over authored SysML/KerML documents
and the accepted KerML graph. Existing KerML lookup handles imports, aliases,
visibility, shadowing, cycles, qualified names and root availability. Retain
pending populations and endpoint assertions until resolved; do not turn missing
SysML frontend records into successful empty lookups.

Definition/Usage queries compose `feature_types`, `supertypes`,
`direct_features`, `effective_features`, `subsetted_features`,
`redefined_features` and `effective_names`. Definition-owned usages and
Usage-nested usages filter the existing canonical feature population by Usage.
Use the pinned property domains when selecting typing targets: existing KerML
classifiers can type specialized usages, so a blanket filter to SysML Definition
would lose valid typing. Separate direct from effective typing/specialization.
Qualified names use semantic ownership/naming and propagate ambiguous names.

Add Attribute, Item and Part queries as typed applications of those contracts.
Add Port, Connection and ConnectorAsUsage when required by the actual corpus;
do not claim that metaclass casting alone executes conjugation or connector rules.
Validate only standard anchors actually used by algorithms, including qualified
path, correct metaclass, exact library identity, visibility and uniqueness.
`Attributes::AttributeValue` is an alias to a KerML DataType, which matters to
binding validation. Reflection metadata definitions in `SysML.sysml` are ordinary
source declarations, not replacements for compiled runtime descriptors.

The first acceptance fixture is Engine/Vehicle/engine/SportsCar. Check textual
and direct-kernel semantic equivalence, original inherited `engine` identity,
unchanged record count after querying inheritance and evidence-bearing typing.
Syntax IDs need not match a programmatic model. Test wrong dependency identities,
mixed-source resolution and edit isolation independently of that happy path.
