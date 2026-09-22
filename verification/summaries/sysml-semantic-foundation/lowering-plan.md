# Canonical SysML lowering preparation

Preparation only, while the full KerML acceptance gate runs. This document changes
no parser, graph, producer or authority decision. Production SysML construction
starts after K10–K13. It complements [architecture-plan.md](architecture-plan.md)
and [query-contract-plan.md](query-contract-plan.md).

Authority is the pinned final [SysML 2.0 PDF](../../../SysML.pdf), especially
§8.2.2.5–12, printed pp.167–176 (PDF pages 199–208), and the matching
[SysML.xmi](../../../standards/normative/sysml-2.0/SysML.xmi). PDF, XMI and Systems
KPAR hashes are respectively
`46e6c0476a6f1f34f367d57e039d56659bff75e41d2e4b3d37ca4cadea84a83a`,
`caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`, and
`df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f`.
The existing grammar-preparation projection supplies readable production names;
the PDF actions and XMI determine this plan. Pending grammar corrections remain
[unadopted](grammar-preparation-review.md).

## Production-to-record map

Keep one lossless syntax arena and one canonical kernel graph. A grammar return
type does not by itself request a new record: many rules fill an existing element
or dispatch to another rule. In particular, `part def Vehicle` creates one
PartDefinition, not nested PartDefinition/Definition/Classifier records.

| Production | Canonical output / behavior |
| --- | --- |
| RootNamespace, Package, LibraryPackage | Existing KerML Namespace, Package, LibraryPackage records. |
| DefinitionElement, UsageElement, NonOccurrenceUsageElement, OccurrenceUsageElement, StructureUsageElement, BehaviorUsageElement | Transparent alternative dispatch. |
| Definition, DefinitionDeclaration, DefinitionBody, DefinitionBodyItem | Fill the concrete Definition owner; declaration/body wrappers create no extra Definition or Type. |
| Usage, UsageDeclaration, UsageCompletion, UsageBody | Fill the concrete Usage owner; no extra Usage or Feature. |
| DefinitionPrefix, BasicDefinitionPrefix, RefPrefix, BasicUsagePrefix, UsagePrefix, UnextendedUsagePrefix, EndUsagePrefix, OccurrenceDefinitionPrefix, OccurrenceUsagePrefix | Interpret flags and explicit child productions on the existing owner; no prefix records. |
| AttributeDefinition / AttributeUsage | SysML AttributeDefinition / AttributeUsage; inherited DataType / Feature behavior comes from descriptors, not additional records. |
| ItemDefinition / ItemUsage | SysML ItemDefinition / ItemUsage. |
| PartDefinition / PartUsage | SysML PartDefinition / PartUsage. |
| PortDefinition / PortUsage | SysML PortDefinition / PortUsage, plus the mandatory PortDefinition synthesis below. |
| DefaultReferenceUsage, ReferenceUsage, OwnedCrossFeature | SysML ReferenceUsage. In particular, SysML OwnedCrossFeature differs from KerML's Feature mapping. |
| PackageMember, DefinitionMember, OwnedCrossFeatureMember, OwnedMultiplicity, MultiplicityExpressionMember | KerML OwningMembership. A package-owned Usage is not automatically a FeatureMembership. |
| NonOccurrenceUsageMember, OccurrenceUsageMember, StructureUsageMember, BehaviorUsageMember | KerML FeatureMembership. These establish definition-owned or usage-nested features. |
| AliasMember | Existing non-owning Membership with member name/short name, visibility and pending memberElement endpoint. |
| Import | Existing dynamic MembershipImport / NamespaceImport dispatch; keep import wrapper behavior and avoid a second import record. |
| SubclassificationPart, FeatureSpecializationPart, FeatureSpecialization, Typings, TypedBy, Subsettings, Subsets, References, Crosses, Redefinitions, Redefines | Transparent owner operations, creating only their explicitly owned relationship children. |
| OwnedSubclassification | KerML Subclassification; pending superClassifier endpoint. |
| FeatureTyping | **Transparent SysML dispatch** to OwnedFeatureTyping or ConjugatedPortTyping; unlike the current KerML vocabulary, do not create a second FeatureTyping here. |
| OwnedFeatureTyping, OwnedSubsetting, OwnedReferenceSubsetting, OwnedCrossSubsetting, OwnedRedefinition | One corresponding KerML relationship per occurrence. |
| OwnedFeatureChain | A canonical KerML Feature with ordered FeatureChaining relationships. Preserve each path, even if two paths reach the same final Feature. |
| MultiplicityPart | Transparent flags/owned-multiplicity operations. |
| MultiplicityRange, FeatureValue, expression/literal leaf productions | Reuse existing structural KerML construction with the SysML grammar's actual child ownership; no evaluation of bounds or values. |

Shared production names must be dispatched by dialect and role, not blindly
passed through `vocabulary::class`. The FeatureTyping and OwnedFeatureChain cases
demonstrate why recognition reuse does not imply identical lowering. Generic
Definition and Usage are useful programmatic metaclasses; the first textual
vertical creates the specialized classes selected by their concrete productions.

## Ownership and endpoints

Reuse the existing two-step canonical ownership chain:

```text
namespace/type/feature --Element::ownedRelationship--> membership
membership --Relationship::ownedRelatedElement--> declaration
```

The first edge and the second edge retain their ordered source positions. Do not
also store derived `ownedMemberElement`, `ownedFeature`, `ownedUsage`, `owner`,
`owningNamespace` or `owningType`. The current builder's subtype-aware attachment
already handles OwningMembership and FeatureMembership without a second store.
Non-owning aliases retain `Membership::memberElement`; imported membership identity
is not copied.

| Relationship / owning declaration | Stored target, resolved under effective descriptors |
| --- | --- |
| OwnedSubclassification / Definition | `Subclassification::superClassifier` (effective `Specialization::general`); ownership supplies the specific endpoint when derived. |
| OwnedFeatureTyping / Usage | `FeatureTyping::type` (effective `Specialization::general`). |
| OwnedSubsetting / Usage | `Subsetting::subsettedFeature`. |
| OwnedReferenceSubsetting / ReferenceUsage or other Usage | `ReferenceSubsetting::referencedFeature`. |
| OwnedCrossSubsetting / owned cross ReferenceUsage | `CrossSubsetting::crossedFeature`. |
| OwnedRedefinition / Usage | `Redefinition::redefinedFeature`. |
| OwnedFeatureChaining / owned chain Feature | `FeatureChaining::chainingFeature`, preserving grammar order. |
| PortConjugation / synthesized ConjugatedPortDefinition | `PortConjugation::originalPortDefinition`; conjugated endpoint comes from ownership. |

Retain `PendingLibraryReference`'s relationship identity, effective property,
expected metaclass, qualified name and precise source origin. Resolve through the
same KerML imports, visibility, aliases, specialization and feature-chain queries.
An unresolved name remains a construction obligation; it must not disappear
because a SysML projection has no matches. Use descriptor-resolved endpoints,
including association occurrences where slot storage is prohibited, exactly as
the current builder's `publish` path does.

## Defaults and mandatory syntactic synthesis

`construction::Builder::create` currently initializes a fixed list of KerML
properties. Merely supplying the SysML registry will fail: effective
`Feature::isVariable` on Usage is derived `Usage::mayTimeVary`. The initializer
must resolve effective descriptors before writing defaults and omit derived
properties. Preserve all existing KerML profile behavior; do not catch and ignore
arbitrary failed writes. SysML defaults/parsed modifiers belong in an explicit
dialect policy.

`Usage::isReference` is also derived (`not isComposite`), including its
ReferenceUsage and AttributeUsage redefinitions. Record parsed referential intent
and establish stored `Feature::isComposite` accordingly. Attribute and Reference
usages are referential; directed/end/unfeatured usages have the published
referential constraint. Ordinary nested parts/items can be composite. Do not
initialize every Usage from KerML's `isComposite = false`, or write the derived
isReference property directly. Where featuring is still pending, retain the
corresponding obligation rather than guess its result. See §7.6.3–4 p.42,
§7.7 p.45, and `deriveUsageIsReference` / `validateUsageIsReferential` in the XMI.

`constant` sets isConstant; it does not copy KerML's `const` behavior of writing
isVariable. The note in §8.2.2.6.2 p.170 requires an end Usage with mayTimeVary=true
to become constant even without a constant token. That case depends on semantic
information and remains an explicit producer obligation, not a lexical default.
Variation/individual/portion flags and their additional implications similarly
must not be silently accepted as completed by a boolean write.

Every **textual PortDefinition** requires the following zero-token structure under
§8.2.2.12 p.175, even when no conjugated-port reference occurs:

```text
PortDefinition P
  ownedRelationship: OwningMembership M
    ownedRelatedElement: ConjugatedPortDefinition CP
      ownedRelationship: PortConjugation C
        originalPortDefinition: P
```

Do not recursively synthesize another conjugated definition for CP. Do not write
P.conjugatedPortDefinition, CP.originalPortDefinition or CP.ownedPortConjugator:
those properties are derived. CP's effective name is computed from P; inventing a
declared `~name` would lose rename behavior. The grammar note calls the relation
`PortDefinitionRelationship`, but the actual production and available descriptor
are PortConjugation; retain this wording difference rather than invent a class.

For future `: ~A::B::C`, create one ConjugatedPortTyping, resolve A::B::C to the
original PortDefinition, then select its actual canonical conjugated definition,
as the published Note 2 permits. Store
`ConjugatedPortTyping::conjugatedPortDefinition`, the stored redefinition of
FeatureTyping::type. Preserve the printed action's `originalPortDefinition` label
as a source discrepancy: that property does not exist on ConjugatedPortTyping;
its `portDefinition` is derived. Do not silently add or rename a descriptor.
The first Ports document needs PortDefinition synthesis but contains no `~`
typing, so this separate resolution case can remain explicitly unsupported until
required. No correction is adopted by this plan.

## Minimal builder extension seams

1. Pass an already validated base Snapshot and registry into construction. For
   SysML it must mount the actual accepted KerML dependency under the compatible
   combined registry proposed in architecture-plan.md. Do not use published
   `agq_sysml::registry()` as an implicit replacement for v9, or reconstruct
   accepted libraries as local declared records.
2. Add a dialect construction policy for production classification, wrapper
   handling, modifier interpretation, effective-property defaults, reference
   targets and zero-token synthesis. Keep generic `create`, `set`, `link`,
   source-map assembly and changeset validation shared. No parser dependency is
   needed inside agq-sysml-semantics; this policy belongs in the textual layer.
3. Separate input identity policy from graph operations. Library declarations keep
   `LibraryDocument::element_id(range, Canonical { role, ordinal })`, StandardLibrary
   origins and exact source bytes. Authored declarations use reconciled syntax
   identities and existing edit-isolation policy. Synthetic grammar children use
   owner plus explicit semantic role (`conjugate-membership`, `conjugate-definition`,
   `port-conjugation`) with the originating syntax node/range. No ElementId sorting
   or global allocation order establishes identity or semantic order.
4. Keep textual synthesis source-backed and separately identifiable from semantic
   derivation. Do not give required grammar children fictitious source text, and
   do not label them as producer results. Later inferred base specializations are
   ordinary derived kernel relationships with rule/evidence provenance.
5. Generalize draft/reference-refinement context construction to accept the
   existing immutable dependency and mixed authored/Systems roots. Keep accepted
   library roots isolated from authored lookup, and preserve complete dependency
   identity. The current `LibraryDraft::strict_snapshot` copies every record into
   an empty snapshot; that must not flatten the accepted dependency. Rebuild or
   validate only local changes against the protected base.

## Actual corpus drivers and focused acceptance

The original `Systems Library/Attributes.sysml` contains aliases to Base::DataValue
and Base::dataValues, not new AttributeDefinition anchors. `Items.sysml` introduces
ItemDefinition, ReferenceUsage, ItemUsage, AttributeUsage, PartUsage and expressions,
but also ConstraintUsage and ConnectionDefinition. `Parts.sysml` additionally needs
ActionUsage and StateUsage. Those documents cannot be declared fully lowered by
supporting only their filename's principal metaclass.

`Ports.sysml` requires one PortDefinition (`Ports::Port`), ReferenceUsages including
an anonymous redefinition, PortUsages, multiplicities, nonunique, imports and the
chain `interfacingPorts.incomingTransfersToSelf`. This makes PortDefinition's
conjugated child and shared feature-chain construction immediate corpus needs;
Interface/conjugated-port lookup behavior is not needed to parse that file alone.
Items/Parts/Ports have cross-document dependencies; construct candidates together,
resolve against the accepted KerML graph, and retain other unsupported productions
as explicit gaps.

After the gate, the smallest useful regression set is:

- Engine/Vehicle/engine/SportsCar: exact PartDefinition/PartUsage classes, correct
  membership kinds, one FeatureTyping and one Subclassification, and unchanged
  inherited engine identity with no copied record.
- Shared-name wrappers: one relationship for `part p : T`, one chain Feature for
  `ref :>> a :> b.c`, and every relationship's source/syntax identity retained.
- A port definition yields exactly the membership, conjugate and conjugation above;
  a rename changes the effective conjugate name without recreating an unrelated ID.
- Reference/attribute/directed/nested-part modifier fixtures distinguish stored
  composite state from derived isReference/mayTimeVary and preserve pending status.
- Text and direct changeset models return equivalent graph/query answers; authored
  syntax IDs are compared only where their identity contract actually matches.

These are proposed checks, not current implementation claims. No builds or corpus
closure were run for this document. Bounded read-only verification used `python -`
with `zipfile` to inspect the four original KPAR documents, `ElementTree` to inspect
the exact relevant XMI properties, and `pypdf` to inspect the cited final PDF pages;
all final inspections exited 0. The initial XMI probe used the wrong namespace and
returned 1, then was corrected to the pinned `20161101` namespace. `git diff --check`
passed. Only this plan is committed; original artifacts remain unchanged.
