# Definition/Usage query contract preparation

**Preparation only. SysML production parsing and semantics remain unstarted.**
Canonical KerML acceptance is still the gate. This plan adopts no authority
correction and does not change the retained source discrepancies. The first
implementation scope is ordinary, non-variation, non-individual Attribute, Item
and Part definitions/usages with ordinary memberships; other applicable rules
must remain visible as pending obligations.

Authority is the pinned final [SysML 2.0 PDF](../../../SysML.pdf) and
[SysML.xmi](../../../standards/normative/sysml-2.0/SysML.xmi), with the original
Systems KPAR identified in [corpus-inventory.json](corpus-inventory.json).
PDF SHA-256: `46e6c0476a6f1f34f367d57e039d56659bff75e41d2e4b3d37ca4cadea84a83a`;
XMI: `caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f`;
KPAR: `df7d8b2c6e08232ca7ce123a63148949c383fcbeaeba8d89c27ceece43793a1f`.
Page references below are printed pages; the PDF page number is 32 greater.

## Reuse contracts, with an explicit closure boundary

SysML 2.0 §8.4.2.1–2, p.402, gives Definition the semantics of KerML Classifier
and Usage the semantics of KerML Feature. Compose `KerMlQueries` over the same
canonical graph. Do not copy inherited records or fork the type algorithm.

| Question | Existing KerML operation | SysML contract |
| --- | --- | --- |
| Direct typing of a Usage | `direct_feature_types` | Direct canonical FeatureTyping endpoints. This is not effective typing or necessarily authored-only typing. |
| Effective typing | `feature_types` | Reuse typing through subsetting, redefinition, chains and conjugation. Preserve completeness, evidence and search dependencies; additionally require applicable SysML producer closure. |
| Immediate/general definitions | `direct_specializations`, `supertypes`, `all_supertypes` | Keep immediate and transitive APIs distinct; `all_supertypes` is reflexive. Valid KerML general classifiers remain in the result. A separate explicitly named projection can select SysML Definitions. |
| Owned and effective usages | `direct_features`, `effective_features`, then metaclass conformance to Usage | Definition-owned and Usage-nested usages are the owned projection. Effective usages include inherited identities and existing redefinition suppression. |
| Subsetting/redefinition | `subsetted_features`, `redefined_features`, `all_redefined_features` | Preserve original endpoints and proof dependencies. Effective edges can include implied relationships. |
| Effective qualified name | `effective_names` plus canonical namespace ownership | Compose existing naming rules; retain ambiguity, missing ownership and cycle diagnostics. Variant Usage naming has a SysML override and cannot be silently treated as ordinary Feature naming. |

`direct_specializations` includes KerML library implications unless excluded by
query options. `exclude_implied` does not remove already materialized derived
records; an authored-only API needs relationship provenance. The existing
`effective_features` API returns an **identity set**, not normative ordered
`Type::feature`. The first SysML API should promise identity sets as well;
ElementId order must never establish semantic order for the ordered derived
SysML properties.

The exact XMI property `Systems-DefinitionAndUsage-Usage-definition` redefines
`Feature::type` and targets **Classifier**, not Definition. AttributeUsage's
`attributeDefinition` redefines it with DataType targets; OccurrenceUsage's
`occurrenceDefinition` uses Class; ItemUsage's `itemDefinition` selects Structure;
PartUsage's `partDefinition` selects PartDefinition. Keep valid KerML DataTypes
and Structures. Report invalid endpoint kinds rather than hiding them through a
filter that produces an apparently successful empty result.

The relevant formal projection bodies are:

| Exact XMI rule ID | Body |
| --- | --- |
| `Systems-DefinitionAndUsage-Definition-deriveDefinitionUsage` | `usage = feature->selectByKind(Usage)` |
| `Systems-DefinitionAndUsage-Definition-deriveDefinitionOwnedUsage` | `ownedUsage = ownedFeature->selectByKind(Usage)` |
| `Systems-DefinitionAndUsage-Usage-deriveUsageUsage` | `usage = feature->selectByKind(Usage)` |
| `Systems-DefinitionAndUsage-Usage-deriveUsageNestedUsage` | `nestedUsage = ownedFeature->selectByKind(Usage)` |
| `Systems-Items-ItemUsage-deriveItemUsageItemDefinition` | `itemDefinition = occurrenceDefinition->selectByKind(Structure)` |
| `Systems-Parts-PartUsage-derivePartUsagePartDefinition` | `itemDefinition->selectByKind(PartDefinition)` |

See §8.3.6.2 p.267, §8.3.6.4 pp.275–276, §8.3.10.3 p.288 and
§8.3.11.3 p.291. The last body is reproduced without adding an assignment
absent from the XMI. Owned/nested Attribute, Item and Part queries are typed
projections of these populations, not separate inheritance engines.

## Required producers before effective SysML completion

§8.4.1 p.395 explicitly classifies specialization checks as semantic constraints
and describes implied **Subclassification** for Definitions and **Subsetting**
for Usages. Tables 31–32, pp.396–397, give the relationship categories. These are
producer obligations, distinct from validation-only checks such as
`validateAttributeDefinitionFeatures` or `validateAttributeUsageIsReference`.
Add implied relationships only when required and not already satisfied; apply
the published redundancy suppression. Do not add one edge for every inherited
metaclass or duplicate an existing direct/indirect specialization.

| Exact formal rule ID | Exact body | Existing/new responsibility |
| --- | --- | --- |
| `Kernel-DataTypes-DataType-checkDataTypeSpecialization` (pinned KerML XMI) | `specializesFromLibrary('Base::DataValue')` | Existing KerML DataType implication applies to AttributeDefinition through descriptor conformance; SysML §8.4.3.1 p.404 explicitly reuses it. |
| `Systems-Attributes-AttributeUsage-checkAttributeUsageSpecialization` | `specializesFromLibrary('Base::dataValues')` | SysML metaclass implication is needed even without authored typing; reuse the accepted KerML anchor. |
| `Systems-Items-ItemDefinition-checkItemDefinitionSpecialization` | `specializesFromLibrary('Items::Item')` | SysML base Subclassification. |
| `Systems-Items-ItemUsage-checkItemUsageSpecialization` | `specializesFromLibrary('Items::items')` | SysML base Subsetting; effective typing then comes from the existing KerML query. |
| `Systems-Parts-PartDefinition-checkPartDefinitionSpecialization` | `specializesFromLibrary('Parts::Part')` | SysML base Subclassification, also satisfying the Item base through the library graph. |
| `Systems-Parts-PartUsage-checkPartUsageSpecialization` | `specializesFromLibrary('Parts::parts')` | SysML base Subsetting, also satisfying the Item base through the library graph. |
| `Systems-Occurrences-OccurrenceUsage-checkOccurrenceUsageSpecialization` | `specializesFromLibrary('Occurrences::occurrences')` | Inherited obligation; reuse the accepted KerML anchor, and suppress an edge when the more specific library base already satisfies it. |

Formal anchors: AttributeUsage §8.3.7.3 p.279; ItemDefinition/Usage
§8.3.10.2–3 pp.287–288; PartDefinition/Usage §8.3.11.2–3 pp.289–291;
OccurrenceUsage §8.3.9.4 pp.284–285. For example, `part spare;` has no direct
FeatureTyping, but must acquire effective typing through `Parts::parts`.
Returning an empty **effective SysML** type set as Complete before that producer
runs would be incorrect. Do not insert a redundant FeatureTyping to compensate.

Composite nesting additionally requires these exact bodies:

```ocl
-- Systems-Items-ItemUsage-checkItemUsageSubitemSpecialization
isComposite and owningType <> null and
(owningType.oclIsKindOf(ItemDefinition) or
 owningType.oclIsKindOf(ItemUsage)) implies
    specializesFromLibrary('Items::Item::subitem')

-- Systems-Parts-PartUsage-checkPartUsageSubpartSpecialization
isComposite and owningType <> null and
(owningType.oclIsKindOf(ItemDefinition) or
 owningType.oclIsKindOf(ItemUsage)) implies
    specializesFromLibrary('Items::Item::subparts')

-- Systems-Occurrences-OccurrenceUsage-checkOccurrenceUsageSuboccurrenceSpecialization
isComposite and owningType <> null and
(owningType.oclIsKindOf(Class) or
 owningType.oclIsKindOf(OccurrenceUsage) or
 owningType.oclIsKindOf(Feature) and
    owningType.oclAsType(Feature).type->exists(oclIsKind(Class))) implies
    specializesFromLibrary('Occurrences::Occurrence::suboccurrences')
```

The first body deliberately preserves **singular `subitem`**. The retained
[source discrepancy](../../../standards/sysml-semantic-coverage.json) is real:
the final formal body (§8.3.10.3 p.288, visually checked) and XMI say `subitem`;
Table 32 p.397, §8.4.6.2 p.408 and actual `Items.sysml:110` say `subitems`.
PartUsage inherits ItemUsage, so a composite `Vehicle::engine` also triggers
this producer obligation. No alias, spelling correction or waiver is adopted
here. The first vertical can establish structural typing/inheritance witnesses
while its **SysML producer closure remains explicitly pending**. This finding
does not block KerML publication or prohibit beginning Phase S after acceptance.

For precision, Table 32 also prints `Attributes::attributes`, whereas the
AttributeUsage formal body, §8.4.3.2 p.404 and actual attribute aliases identify
`Base::dataValues`. Record those distinct source statements; this plan does not
silently adopt a precedence decision between conflicting printed targets.

## Necessary library anchors and property dependency

These are binding requirements for later algorithms, not accepted bindings.
All Systems paths below belong to LibraryId
`6c418b74-7704-5af0-bce2-28b92196cb15`, with exact document identities/digests
in the existing inventory. ElementIds must come from canonical lowering.

| Anchor | Actual original KPAR declaration | Required metaclass |
| --- | --- | --- |
| `Items::Item` | `Systems Library/Items.sysml:24`, `abstract item def Item :> Object` | ItemDefinition |
| `Items::items` | `Items.sysml:142`, `abstract item items : Item[0..*] nonunique :> objects` | ItemUsage |
| `Parts::Part` | `Systems Library/Parts.sysml:19`, `abstract part def Part :> Item` | PartDefinition |
| `Parts::parts` | `Parts.sysml:74`, `abstract part parts: Part[0..*] nonunique :> items` | PartUsage |
| `Items::Item::subparts` | `Items.sysml:117`, `abstract part subparts: Part[0..*] :> subitems, parts` | PartUsage |
| `Items::Item::subitems` | `Items.sysml:110`, `abstract item subitems: Item[0..*] :> items, subobjects` | ItemUsage; mismatch with formal singular target remains unresolved |
| `Actions::Action` | `Systems Library/Actions.sysml:33`, `abstract action def Action :> Performance` | ActionDefinition; needed only when evaluating the `mayTimeVary` predicate below |

Validate path, concrete metaclass, source/library identity, visibility and
uniqueness; never select a same-named authored declaration or synthesize missing
standards. `Attributes.sysml:11,19` contains aliases `AttributeValue` and
`attributeValues` to `Base::DataValue` and `Base::dataValues`. Reuse accepted
KerML `StandardRole::DataValue`/`DataValues`, not fictitious SysML declarations.
Reuse `Occurrence`/`Occurrences` as well. If the inherited suboccurrence check
needs an explicit anchor, bind the existing accepted KerML
`Occurrences::Occurrence::suboccurrences`; it is not currently one of the 31
KerML StandardRoles and must not be silently substituted with `snapshots`.

`Usage::mayTimeVary` redefines KerML `Feature::isVariable` as derived. Exact
rule `Systems-DefinitionAndUsage-Usage-deriveUsageMayTimeVary`, §8.3.6.4 p.273:

```ocl
mayTimeVary =
    owningType <> null and
    owningType.specializesFromLibrary('Occurrences::Occurrence') and
    not (
        isPortion or
        specializesFromLibrary('Links::SelfLink') or
        specializesFromLibrary('Occurrences::HappensLink') or
        isComposite and specializesFromLibrary('Actions::Action')
    )
```

This is not required to enumerate a direct FeatureTyping edge, but it is required
before featuring-dependent queries/closure can succeed. Resolve those predicates
against canonical standard identities; `SelfLink` and `HappensLink` are accepted
KerML declarations, not Systems anchors. A missing derived value must remain
NotComputed/Incomplete, never default to KerML's stored `isVariable = false`.
Variation, individual/portion, actor/stakeholder and metadata-specific producers
remain separately pending when applicable; do not bind their whole libraries for
the ordinary first slice.

## Implementation and verification boundary

1. After KerML acceptance, establish the combined descriptor registry and exact
   immutable dependency contract from [architecture-plan.md](architecture-plan.md).
   Construct the unpublished Systems candidate and validate only needed anchors.
2. Implement thin structural queries that retain all KerML result metadata.
   Preserve a separate, visible SysML capability/producer status. A Complete
   **current-graph** answer is permitted; it is not evidence of SysML closure.
3. Implement applicable base/conditional producers as derived canonical KerML
   relationships, with deterministic identity and rule provenance. Feed pending
   specialization/namespace scopes into shared queries and retain SysML pending
   obligations in the wrapper so effective results cannot falsely claim closure.
4. Test direct vs effective typing with untyped attribute/item/part usages, valid
   KerML DataType/Structure targets, and missing/wrong standard bindings. Remove
   a required base edge to prove the relevant effective result becomes Incomplete.
5. Test Engine/Vehicle/engine/SportsCar in text and kernel APIs: explicit Engine
   typing, SportsCar specializing Vehicle, the same inherited engine ElementId and no
   copied records. Keep producer closure pending for the retained composite Item
   target discrepancy. Add an owned redefinition fixture to verify suppression,
   and prove an unresolved specialization cannot become Complete after filtering.

No build or production semantic test was run for this planning change. Bounded
read-only verification on 2026-09-22:

| Actual command | Result | Exit |
| --- | --- | ---: |
| `python verification/generated/sysml-grammar-preparation/query-contract-probe.py` | 3/3 pinned hashes, 16/16 SysML rule bodies/anchors, 9/9 original declaration lines; printed producer classification and retained discrepancy confirmed | 0 |
| `git diff --check` | No whitespace findings | 0 |
| `git diff --cached --check` | No whitespace findings in the staged new plan | 0 |

The probe uses `ElementTree`, `pypdf` and `zipfile`. A separate inline
`python -` command using `pypdfium2` rendered PDF page 320 and exited 0; the
printed singular target was visually confirmed. The probe, rendered page and
other raw preparation artifacts remain ignored under
`verification/generated/sysml-grammar-preparation/`. No parser or semantics
completion is claimed.
