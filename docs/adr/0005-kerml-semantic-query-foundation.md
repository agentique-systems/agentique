# ADR 0005: KerML semantic-query foundation

Status: implemented for the bounded Root/Core relationship slice.

`agq-kerml-semantics` evaluates pure queries over an immutable `ModelView` from a
declared `Snapshot` or a kernel `DerivedOverlay`. It is above `agq-kerml`; the
kernel remains language neutral and descriptors remain generated facts.

## Identity and purity

A `SemanticContext` contains declared revision, exact metamodel version, semantic
rule-set version, pinned-library identities, and answer-affecting options. A
revision is consequently necessary but not sufficient semantic identity. Queries
have no mutation and no cache is canonical. A future discardable per-context cache
must key on the complete context and invalidate every recorded dependency.

## Result and dependency contract

Each `QueryResult` carries a value, `Complete`/`Incomplete` status, diagnostics,
positive `FactKey` dependencies, search-space dependencies, and rule-labelled
explanations. Search dependencies record population reads (currently metaclass
instances; namespace/member and property-set forms are reserved) so an empty
answer is not treated as dependency-free. Malformed/missing evidence is incomplete,
not silently absent. Explanations name the rule, conclusion, relationship and
immediate evidence; kernel overlay explanations remain available for derived facts.

## Bounded rules and authority

The implementation follows the pinned KerML 1.0 Root/Core descriptors and their
retained KerML Core/Root constraints, specifically the generated `Specialization`,
`FeatureTyping`, `Subsetting`, `Redefinition`, `FeatureMembership`, and
`OwningMembership` properties (see ADR 0003/0004 and the pinned XMI evidence).
It evaluates direct generalization, transitive generalization, feature typing,
subsetting, redefinition, direct/effective features, and ownership where overlay
facts establish the derived membership endpoints. Effective features preserve the
same feature identities; direct redefinitions suppress inherited targets rather
than copying elements. Cyclic specialization evidence is reported incomplete.

No parser, textual library import, SysML rule, namespace lookup, full ownership
derivation, constraint execution, or incremental cache is claimed. The engine is
not ready for textual lowering: lowering must first produce the required
evidence-bearing derived ownership/feature-membership facts and validate the
remaining normative constraints.
