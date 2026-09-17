# ADR 0005: KerML semantic-query foundation

Status: implemented for the bounded Root/Core query slice, rule set
`agq-kerml-query/2`. Supersedes the initial four-test query scaffold.

## Architecture and purity

`agq-kernel <- agq-kerml <- agq-kerml-semantics`. Generated descriptors and
borrowed views remain facts/projections; the separate semantic crate implements
queries. There is no parser, mutable expansion, feature copying, global state,
cache framework or external I/O. An evaluator borrows exactly one immutable
snapshot or overlay through `SemanticContext`. New kernel consumers must use this
facade rather than duplicating these rules. The older application `agq-semantics`
model is not migrated or declared equivalent to this kernel model.

Queries compose other queries. Inheritance uses a local dependency-first traversal
and disposable maps of feature IDs, merging evidence into one result without
recursively copying ancestor proofs. Incoming indexes select relationships without
a population scan per type. Traversals are iterative. Cost is output-sensitive:
dense inheritance/redefinition can require quadratic intermediate/result space;
there is no universal linear-cost claim.

## Context identity and rule version

`SemanticContextId` includes the actual declared revision, a SHA-256 digest of
all records and slot/element provenance, exact normative metamodel source identity,
registered descriptor digest, rule-set version, sorted library names with content
SHA-256 pins, and typed options. `exclude_implied` affects specialization and
inheritance. Library pins identify caller-supplied inputs; they do not load or
certify libraries. Constructors bind the actual snapshot/overlay revision; no
unchecked model-plus-revision constructor exists. Overlay values or explanations
change identity even on the same revision. Only the exact generated descriptor
slice is accepted; extensions require a reviewed rule version.

Digest encoding is private and rule-versioned: deterministic Debug encoding of
ordered descriptor/record structures, with byte-length-prefixed records. It is
not cross-language interchange. Changes to this encoding, rules, dependencies or
proof construction require a new rule-set version. Record hashing occurs once
at context construction. Revision alone is never a semantic cache key.

## Generic ownership-storage prerequisite

The prior kernel refused the two ownership association pairs. It now supports an
additional generic shape: two non-derived class-owned ends, one unique ordered
0..* end and one unique unordered 0..1 inverse. Each occurrence is stored once
in the ordered slot with its origin/position. Scalar inverse navigation is a
reconstructible index, never a second canonical record slot. Conflicting inverse
sources fail atomically. Typed views borrow the inverse using `navigation_slot`.

This refines ADR 0003's anticipated link store for this shape. Only the ordered
end is currently writable; inverse writes/clears remain explicitly refused.
Ownership moves edit canonical ordered slots. No generated descriptor or normative
flag changes. The kernel contains no KerML names, inferred containment or ownership
cycle rule. Inverse navigation retains the canonical slot origin and is not a
new derived assertion. Semantic evidence maps it to canonical facts and incoming
set reads. Other association shapes and a general link-edit API remain deferred.

## Implemented rules and authority

Authority is the hash-pinned [KerML XMI](../../standards/normative/kerml-1.0/KerML.xmi),
whose property identities, operation bodies and descriptions were inspected, with
[KerML 1.0](https://www.omg.org/spec/KerML/1.0/PDF) as publication reference.
Known XMI spelling inconsistencies, such as `special` in
`deriveTypeOwnedSpecialization`, are resolved using the declared `specific`
property and normative description; this is not an OCL interpreter.

| Query | Normative anchor and implemented boundary |
| --- | --- |
| `owned_relationships`, `owning_relationship`, `owning_related_element` | 8.3.2.1.2–3; `Element-ownedRelationship`, `Element-owningRelationship`, `Relationship-owningRelatedElement`, `Relationship-ownedRelatedElement`: direct links/inverses. |
| `owner` | 8.3.2.1.2, `Element-deriveElementOwner`: owning relationship followed by owning related element; detect ownership cycles including beyond the root, and invalid membership namespace ownership along the path. |
| `memberships`, `member` | 8.3.2.4.3/.5/.8; `Namespace-ownedMembership`, `Membership-memberElement`, `OwningMembership-ownedMemberElement`: direct owned membership traversal; exactly one owned member; FeatureMembership requires a Feature. |
| `lookup_declared_member` | 8.3.2.4.3/.8, `Membership-memberName` and `Element-declaredName`: exact declared full names on direct memberships, returning all matches. Not full name resolution. |
| `direct_specializations` | 8.3.3.1.8, `Specialization-specific/general`: all explicit matching relationships, including FeatureTyping/Subsetting/Redefinition through generated endpoint redefinitions; optional exclusion of implied relationships. |
| `all_specializations` | Specialization transitivity, 7.3.2 and 8.3.3.1.8: non-reflexive reachability over explicit edges. Cycles terminate. Not the reflexive, ownership-based `Type::allSupertypes`. |
| `direct_feature_types` | 8.3.3.3.7, `FeatureTyping-typedFeature/type`: direct assertions, not full derived `Feature::type`. |
| `subsetted_features`, `redefined_features` | 8.3.3.3.10/.8, `Subsetting-subsettingFeature/subsettedFeature`, `Redefinition-redefiningFeature/redefinedFeature`: direct assertions. Redefinition is also a Subsetting and Specialization. |
| `direct_features` | 8.3.3.1.6/.10, `Type-deriveTypeOwnedFeatureMembership`, `Type-deriveTypeOwnedFeature`: owned features in canonical membership order. |
| `effective_features` | 8.3.3.1.10, `Type-supertypes`, `-inheritableMemberships`, `-removeRedefinedFeatures`, `-deriveTypeFeatureMembership`, `-deriveTypeFeature`; 8.3.3.3.4, `Feature-allRedefinedFeatures`: owned specialization DAGs, FeatureMemberships, public/protected inheritance and both redefinition-removal conditions. |

Effective features use transitive owned redefinition to suppress inherited targets.
Direct owned-feature redefinitions also suppress inherited competing replacements
with a common redefined base. Direct owned features remain. Subsetting alone does
not suppress inheritance. Results are sorted identity sets, not normative ordered
`Type::feature` serialization. No inherited element is copied or reparented.

Specialization cycles alone are not declared illegal. Cyclic inheritance,
conjugation, imports, feature chaining and feature aliases through other membership
kinds are outside the effective-feature slice and yield `Incomplete` when detected.
Direct assertions do not imply validation of every metaclass constraint or inference
of required library specializations.

## Completeness and explanation contract

Every result carries context, value, completeness, deterministic diagnostics,
positive dependencies, search dependencies, conclusion-keyed alternative proofs
and fact origins. `Complete` means complete for the named bounded query, never
full conformance or verification success. `Incomplete` identifies unsupported or
unavailable applicable evidence/rules; `Invalid` identifies bad subjects, malformed
memberships or illegal ownership evidence. Composition retains the most severe
status and all diagnostics. Incomplete/invalid values are provisional, especially
for non-monotonic inheritance filtering; clients must not treat them as authoritative.

Proofs identify query kind, subject, target, versioned rule and immediate facts,
conclusions or search premises. Alternative relationship witnesses are retained.
`FactKey` and `Origin` reuse kernel contracts. Overlay dependencies expand
iteratively, retaining declared/derived origins and producer RuleIds. This graph
is not inserted into a kernel overlay, whose positive-only evidence cannot encode
negative/search dependencies.

Specialization closure retains all traversed direct edges and a finite witness
DAG of increasing breadth-first depth, including diamond alternatives. Longer
or non-increasing-depth paths remain reconstructible from direct-edge proofs but
are not expanded into full paths. This avoids exponential enumeration and circular
self-support. Effective proofs trace ownership, inheritance, visibility and
redefinition; removals have explicit suppression conclusions. Proofs are scoped
to one query invocation. Future cache keys must include all arguments, including
lookup name, not merely a proof conclusion.

## Dependencies, invalidation and cache boundary

| Dependency | Invalidation obligation |
| --- | --- |
| Positive `FactKey` | Element, effective slot, provenance or transitive overlay evidence changes/removal. Generated redefinitions record actual stored property IDs. |
| `Element(id)` | Existence/metaclass change, including creation of a missing subject. |
| `PropertySet(element, property)` | Absence/presence, values, order or provenance changes, including empty sets. |
| `Incoming(target)` | Any incoming reference population change; retargeting affects both old and new targets. Covers newly matching relationships. |
| `NamespaceMembers(namespace)` | Membership insertion/removal/move and endpoint/name/visibility changes. Canonical relationship sets and individually read names are also recorded. Empty lookup is not dependency-free. |
| `Instances(class)` | Reserved subtype-inclusive population dependency for future queries; insertion/deletion/class changes invalidate it. Current relationship queries use incoming indexes. |

No persistent or per-context query cache exists. Inputs are immutable within one
context; any context change prevents reuse by default. An incremental layer may
reuse only after checking positive AND search dependencies against pre- and
post-change state, including transitive origins. It must cache the whole result,
including status/diagnostics/proofs, remain discardable and never allocate canonical
IDs. Tests cover rename after lookup misses, retargeting, ownership moves, added
redefinition and distinct overlays on one revision. There is no implemented
change-to-dependency invalidator or claim of incremental recomputation yet.

## Verification, omissions and textual lowering

See [actual verification](../../verification/kerml-query-foundation/README.md).
Fixtures use unchanged generated normative descriptors and normal ChangeSets or
evidence-bearing overlays. Structural fixture acceptance is not full conformance.
The larger model has 1,500 types, 1,499 specialization edges and one inherited
feature; tests bound proof size and exercise iterative traversal without timing gates.

No parser, textual library import, SysML, API server, diagrams, runtime, Salsa,
global name resolution, general association storage, full constraint execution,
library implications, effective typing, normative inherited ordering or cyclic
inheritance fixed point is included.

Ready for bounded textual lowering into these declared elements, relationship
endpoints and ordered ownership links. A lowerer can use these queries immediately,
but must diagnose unsupported constructs and add remaining validation. Not ready
to claim general KerML textual semantics. No textual lowering is implemented here.
