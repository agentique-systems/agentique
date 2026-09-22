# Operational KerML profiles

Published KerML 1.0 (`omg-kerml-1.0-published/1`) is the exact formal baseline.
Operational KerML 1.0/v1 (`agentique-kerml-1.0-operational/1`) adds only
AGQ-KERML10-001 / KERML11-81. Its manifest and identity remain frozen.
Operational KerML 1.0/v2 (`agentique-kerml-1.0-operational/2`) additionally
applies AGQ-KERML10-002 / KERML11-140. This is an Agentique semantic
interpretation authorized by the project, not an adopted OMG erratum.
KERML11-140 remains open, last updated 2025-11-11 23:25 GMT, as captured
2026-09-19. It does not apply to any future KerML version.

## Reviewed resolution algorithm, version 1

The entry point is exclusively `Redefinition::redefinedFeature`, for an owned
redefinition whose owning feature has an owning type. Ordinary names, typing,
subsetting, and ordinary qualified-name operations retain their existing rules.

1. Explicit global qualification uses basic resolution from the declared root
   availability. It does not try inherited or lexical alternatives.
2. For a relative name, search the first segment independently in every
   general type of the specific owning type, excluding the redefining declaration.
   Each general namespace applies its ordinary inherited redefinition suppression.
   Collect all matching memberships across the general scopes, then remove a
   matching candidate only when another matching candidate redefines it. A
   differently named redefinition on another branch does not suppress a match
   independently reachable through this branch. Retain canonical membership identity. Do not search lexical parents of the general types.
3. Only a complete empty first-segment search permits searching the lexical
   namespace containing the specific owning type, then its actual containing
   namespaces using basic resolution. The redefining declaration is excluded
   throughout membership computation, including its suppression of inherited
   members. There is no search of unrelated namespaces.
4. If any first segment is found, resolve remaining segments through ordinary
   public visible membership lookup. Prefix shadowing, wrong kind, ambiguity,
   incomplete evidence, and a missing qualified suffix never trigger another
   scope search. Inherited members are non-private; private members are visible
   only through normal internal lexical access. Imports and aliases retain their
   normal visibility and identity contracts.
5. A target must be a Feature distinct from the redefining feature. Name
   denotation does not waive separate redefinition conformance obligations.
   Multiple incomparable memberships remain ambiguous. No traversal order picks
   a winner. Positive and search dependencies include the relationship, searched
   scopes, visibility, and the explicit operational rule path and profile.

This refines the proposed "published search then lexical" wording using the
actual reference implementation: `KerMLScope.resolve` begins with `gen`, which
searches inherited members without general lexical parents; its parent scopes
come from `KerMLScopeProvider.scopeFor` on the specific namespace. All
specializations participate; matched same-name redefinitions suppress their targets. Agentique does
not adopt the implementation's `findFirst` selection of incomparable candidates.

## Evidence boundaries

The [machine-readable authority packet](../verification/kerml-name-resolution-errata-publication-v4/authority-packet.json)
pins original issue HTML, formal and preliminary PDF clauses, reference source,
tests, release notes, and generated XMI. The reference repository commit is
`5cca16d846016e62bb1e54e0e50e675254a022ef`; the release repository commit is
`fb97b754f29588b8e9c7a35f370880cd15eb29e7`.

The two nested `monitoredFeature` redefinitions both point to XMI element
`6b06695b-e0d3-5d4e-aca5-af32f4ad40d8`, the enclosing monitored feature. Their
source patterns are unchanged in the pinned library. Current reference library
files also contain unrelated revisions; the packet records their exact byte
identities and source differences. They are not substitutes for the pinned
source or normative authority.

Publication acceptance remains subject to the full structural obligation,
validation, binding, and authored-project gates. This profile definition alone
does not establish accepted library publication or SysML readiness.

## Operational KerML 1.0/v3 — reviewed library content

The preceding v2 definition remains unchanged. Its exact historical bytes and
SHA-256 `d25df7cca2777bc87c9c667949249f346db62192689cacfbd96e66574a7f48b5`
are retained in the [frozen v2 document](../verification/kerml-library-content-errata-publication-v5/operational-profile-v2-frozen.md).
This appended section does not change either historical errata manifest.

`agentique-kerml-1.0-operational/3` adds the reviewed
[library-content manifest](../standards/kerml-1.0-operational-library-errata-v3.json)
to v2. KERML11-76 directly covers the inherited-member collisions in
FeatureReferencingPerformances, Objects, Observation and VectorFunctions. It is
open; the correction is authorized Agentique operational policy, not an adopted
OMG correction. The reference implementation supplies corroboration, never
replacement authority for the pinned source set.

| Profile | Descriptor correction | Resolution correction | Library-content correction |
| --- | --- | --- | --- |
| Published | none | none | none |
| Operational v1 | KERML11-81 | none | none |
| Operational v2 | KERML11-81 | KERML11-140 | none |
| Operational v3 | KERML11-81 | KERML11-140 | reviewed KERML11-76 entries |

V3 is explicitly selected. `BaselineProfile::OPERATIONAL` remains v2 until a
separate accepted-publication decision. Source parsing and lossless inspection
continue to expose the exact pinned documents. The correction transform operates
on canonical facts after parsing and before semantic refinement/publication.
Superseded source assertions remain inspectable. Inserted and changed facts carry
generic reviewed-correction provenance, exact profile/entry/content identities
and deterministic output keys. The ordinary semantic rules then process those
facts; distinguishability is not weakened.

[ADR 0015](adr/0015-operational-standard-library-corrections.md) specifies the
model boundary, provenance, deterministic ID domain and independent semantic
diff. The [v5 authority matrix](../verification/kerml-library-content-errata-publication-v5/authority-matrix.json)
audits every named library and classifies the fresh baseline findings
individually. It also records unrelated reference changes that were excluded.

V3 construction is not an accepted publication. The separately reproduced
[KERML11-68 end-conformance conflict](kerml-feature-chain-end-authority-conflict.md)
blocks strict semantic acceptance. Incomplete naming, required references,
expression structure and formal-validation coverage remain explicit obligations;
no candidate bindings or facade are labeled accepted.

## Operational KerML 1.0/v4 — reviewed validation semantics

`agentique-kerml-1.0-operational/4` adds only the reviewed KERML11-68 validation
interpretation to v3. Published, v1 (+81), v2 (+140), and v3 (+76) remain frozen
and separately selectable. V4 adds no library-model or descriptor changes.

For a non-end Feature redefining an end Feature, the revised constraint rejects
the redefinition only if its actual owning Type conforms to Association or
Connector, including their metaclass subtypes. Published through v3 retain the
unconditional end implication. Neither end flags nor feature-chain lowering are
changed to satisfy validation.

[ADR 0016](adr/0016-operational-semantic-validation-corrections.md) defines the
exact rule, profile lineage, proof contract and metadata obligations. The
[frozen manifest](../standards/kerml-1.0-operational-validation-errata-v4.json)
and [independent evidence](../verification/kerml-semantic-closure-v6/authority-packet.json)
pin official pilot commit `d9231d21e621aeabeafa92aef026b3929c859116`, including
the actual validator and regression-test changes. KERML11-68 remains open; this
is not an adopted final KerML 1.1 correction.

Canonical v3 correction facts retain their original v3 IDs and origins under
this validation-only successor. SemanticContext includes the v4 identity and
manifest digest, and authored SourceProjects can select v4 explicitly. The
default `OPERATIONAL` remains v2. Future accepted publication/API metadata must
identify both the profile and correction manifest; no acceptance is inferred
from profile availability.

The former end-conformance contradiction now has an explicit operational rule.
The expanded inventory independently exposes
[KLCV6-F-001 / KERML11-205](kerml-subobject-specialization-authority-conflict.md).
V4 does not choose that rule's conflicting required specialization target, and
strict semantic publication remains unaccepted.

## Operational KerML 1.0/v5 — exact formal-constraint targets

`agentique-kerml-1.0-operational/5` extends v4 with exactly six reviewed target
replacements from KERML11-205, KERML11-206 and KERML11-207. All three issues remain
open; this is explicit project policy. [ADR 0017](adr/0017-operational-formal-constraint-target-corrections.md)
contains the exact six-row table and the typed binding/proof contract. The
[manifest](../standards/kerml-1.0-operational-formal-target-errata-v5.json) and
[independent matrix](../verification/kerml-semantic-closure-v7/authority-matrix.json)
freeze the formal/prose, issue, declaration, reference-map and available test evidence.

| Profile | Earliest newly effective correction |
| --- | --- |
| Published | None |
| v1 | KERML11-81 descriptor |
| v2 | KERML11-140 redefinition search |
| v3 | Reviewed KERML11-76 library facts |
| v4 | KERML11-68 end-conformance validation |
| v5 | Exact KERML11-205/206/207 formal targets |

Published through v4 use the published formal targets. V5 uses
`Objects::Object::subobjects`, `Performances::Performance::subperformances`,
`Occurrences::Occurrence::portions`, `Occurrences::Occurrence::suboccurrences`,
`Objects::Object::ownedPerformances`, and
`Performances::Performance::enclosedPerformances`, each only for its typed rule.
Paths are bound once through owned canonical memberships with metaclass,
uniqueness, library and visibility checks; there is no fallback target search.

The profile and target bindings participate in SemanticContext and proof
dependencies. Missing targets and unestablished effective owner typing remain
explicit failures/incompleteness. Canonical declarations, pinned bytes and prior
profile manifests remain unchanged. V3 correction origins/IDs survive under v5.
The default operational profile remains v2.

V5 resolves the previous subobject target stop, but does not grant authority for
KERML11-145, 182, 210 or other open issues. The new independent
[result-binding conflict](kerml-result-binding-authority-conflict.md), exercised by
the pinned ControlFunctions library, prevents strict publication. Profile
availability and passing runtime gates do not constitute accepted libraries.

## V8 preflight: authorized v6 result-domain design, separate authority stop

The subsequent v8 task explicitly authorizes the five rule families in open
KERML11-145. This supersedes the v7 authorization boundary for those five rules;
it does not rewrite any historical profile or verification record. The
[five-rule matrix](../verification/kerml-semantic-closure-v8/authority-matrix.json)
and [ADR 0018](adr/0018-operational-result-domain-corrections.md) record the exact
published authority, contextual-result direction, ownership/domain obligations
and differences from the current reference implementation.

The mandatory early authority sweep independently reproduces
[KLCV8-F-001 / KERML11-8](kerml-feature-reference-binding-authority-conflict.md)
on the inner FeatureReferenceExpression connector of the same pinned
ControlFunctions declaration. It persists with complete local result, chain,
positional and base inference, including after the separately authorized outer
Function contextual-result correction is represented in the independent proof.
Changing this inner connector requires a sixth rule-family correction.

There is therefore **no registered operational v6 profile or immutable v6
manifest yet**. Published and v1–v5 remain the implemented profiles; the default
remains v2. No accepted bindings, library facade or authored accepted-library
dependency is exposed. The
[258-rule authority map](../standards/kerml-1.0-constraint-authority-map.json)
records the early conflict and other unapproved risk metadata. It is a preflight
inventory, not a closed validation or publication gate.

## V9: registered v6 result-domain and reference-binding interpretation

The v9 authorization supersedes the v8 stop on KERML11-8. The new immutable
`agentique-kerml-1.0-operational/6` identity extends v5 with the five KERML11-145
result-domain producers and the narrow KERML11-8 reference-result validation
rule. [ADR 0019](adr/0019-operational-reference-binding-correction.md) and the
[v6 aggregate manifest](../standards/kerml-1.0-operational-profile-v6.json) specify
the independent correction digests and scope. V1–v5, their IDs and frozen
manifests remain unchanged; the default stays v2.

`ImpliedBindingRole::FeatureReferenceResult` is recorded through deterministic
rule provenance. It retains the referent and raw result. Only the raw result
owned by the containing FeatureReferenceExpression can receive the domain
exception; the referent remains subject to ordinary domain conformance.
Authored bindings, other implied roles, extra ends and reversed endpoint roles
do not qualify. Context selection requires a unique nearest valid domain.
The separately reviewed FeatureChainExpression case is excluded; nested
FeatureReferenceExpressions keep their own role.

The five KERML11-145 producers use distinct antecedents and ordered contextual
result chains. The published IndexExpression Array guard remains intact.
Seven-profile tests establish the v6 boundary independently for the inner
reference binding and outer Function binding. Snapshot overlays preserve the
declared input and carry profile-qualified provenance. Cross-profile overlay
reuse is rejected.

This registration does **not** establish complete library semantic acceptance.
The subsequent review independently proves
[KLCV9-F-001 / KERML11-1](kerml-cross-feature-authority-conflict.md) in two pinned
Occurrences end-value declarations. Cross-feature selection imposes a domain
incompatible with the Expression/FeatureValue binding requirements. V6 does not
adopt the pilot's extra cross-feature exclusions. The corpus closure, acceptance,
accepted binding and facade stages remain blocked at that new authority decision.
The [v9 record](../verification/kerml-semantic-closure-v9/README.md) separates the
implemented producer tests, fresh unchanged-v5 measurements, and uncompleted
publication gates. No structural obligation is deferred as executable semantics.


## Operational v7: owned cross Features

Operational v7 (`agentique-kerml-1.0-operational/7`) adds exactly the reviewed
KERML11-1 `Feature::ownedCrossFeature` correction to immutable v6. The default
`BaselineProfile::OPERATIONAL` remains v2. See [ADR 0020](adr/0020-operational-owned-cross-feature-correction.md),
the [profile manifest](../standards/kerml-1.0-operational-profile-v7.json), and the
[authority packet](../verification/kerml-semantic-closure-v10/authority-packet.json).

The operation requires an end Feature and an owning Type. It selects the first
eligible owned Feature in normative owned-membership order. Multiplicity,
MetadataFeature and BindingConnector instances, including subclasses, are
excluded, as are members owned through FeatureMembership or FeatureValue.
Incomplete order/population evidence yields Incomplete, with no guessed identity.
No eligible member yields none. No origin-based exclusion is added.

The exact pilot commit `553cf8205c19241c9127ab264f8372f5b58d3895` is independently
verified from Git objects and its full diff. Its regression puts a BindingConnector
before the legitimate cross Feature. KERML11-1 remains formally open; this is an
explicit Agentique operational authority decision, not final OMG adoption.

V7 repairs the contextual FeatureValue infrastructure ownership defect exposed by
integration with this selector. That infrastructure is owned by its consuming
relationship, preserving FeatureValue domains and all v6 identities. Published
through v6 retain the historical selection boundary. The independent verifier
compares all 11,632 corpus Features and 120 synthetic profile cases, including the
two exact Occurrences ends.

V10's inventory sweep continues beyond each conflict. Its aggregate blocker
register and unfinished applicability/structural work remain publication gates.
Registration of v7 does not accept a library Snapshot, regenerate accepted
bindings, or enable a production accepted-library facade. Full structural runtime
readiness remains separate from strict raw metamodel conformance and KerML
semantic publication.

## Operational v9: multiplicity featuring context

Operational v9 (`agentique-kerml-1.0-operational/9`) extends immutable v8 with
KERML11-4 and KERML11-3 only. The ordinary multiplicity context comes from the
owning Namespace when it is a Feature. For an owned cross Feature, it comes from
the owning end Feature instead. MultiplicityRange bound Expressions receive
that same context without evaluating numeric bounds.

These are explicitly authorized Agentique operational interpretations, not
adopted KerML 1.0 corrections. The 1.1 RTF ballot approval for KERML11-3 is
corroboration outside the pinned 1.0 baseline. Published through v8 preserve their
previous behavior, including the symbolic multiplicity-bound failure recorded
under v8. [ADR 0023](adr/0023-operational-multiplicity-context-v9.md) and the
[authority evidence](../verification/summaries/kerml-v9-publication/authority-decision.json)
define the boundary.

The default `OPERATIONAL` alias was intentionally frozen at v2 while successive
profiles lacked an accepted complete canonical library publication. It may move
to v9 only after that acceptance; a passing local fixture or scoped producer
closure does not satisfy this gate.
