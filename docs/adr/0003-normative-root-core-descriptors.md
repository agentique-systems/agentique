# ADR 0003: Normative Root/Core descriptors and canonical association boundaries

Status: implemented for descriptor generation and structural validation. Canonical
association instance storage and KerML rule execution remain deferred.

## Authority and inspection

This decision uses the exact artifacts pinned by [ADR 0002](0002-normative-metamodel-pipeline.md),
not recollection of EMF or another KerML release. The
[OMG KerML 1.0 publication](https://www.omg.org/spec/KerML/1.0) identifies the
20250201 XMI as the normative abstract syntax and JSON as its representation schema.
The [local lock](../../standards/normative/kerml-1.0/lock.json) is unchanged.
All input hashes are checked before generation.

Concrete evidence below is in [the pinned XMI](../../standards/normative/kerml-1.0/KerML.xmi).
Identifiers are exact document-local XMI IDs. The [golden manifest](../../standards/generated/kerml-1.0/root-core.golden.json)
records source-qualified keys, byte ranges, explicit attributes, effective defaults,
referenced identities, and complete property documentation.

| Concept | Actual encoding and examples |
| --- | --- |
| Redefinition | `redefinedProperty` children with `xmi:idref`. At line 4970, `Core-Types-FeatureMembership-ownedMemberFeature` redefines `Root-Namespaces-OwningMembership-ownedMemberElement`, narrowing Element to Feature. At line 7158, `Core-Features-FeatureTyping-typedFeature` redefines `Core-Types-Specialization-specific`. |
| Subsetting | `subsettedProperty` children, independently of redefinition. `Root-Namespaces-Namespace-ownedMembership` subsets Namespace.membership, Element.ownedRelationship, and association-owned `Root-Elements-A_source_sourceRelationship-sourceRelationship`. Association-owned references must resolve without fabricating class slots. |
| Opposites | A property references its association through an `association` child. The association lists `memberEnd` references; the other end is its opposite. `Root-Elements-Element-ownedRelationship` (line 8867) and `Root-Elements-Relationship-owningRelatedElement` (line 8733) are opposite, class-owned, **both non-derived** properties. |
| Derived | `isDerived="true"`; omitted means false. Element.owner and Element.ownedElement are derived. The retained `Root-Elements-Element-deriveElementOwner` and `Root-Elements-Element-deriveElementOwnedElement` constraints (lines 9123 and 9146) describe traversal through ownership relationships. |
| Derived union | `isDerivedUnion="true"` is separate from `isDerived`. `Root-Namespaces-Namespace-membership` (line 8050) has both flags. Association-owned `Root-Namespaces-A_membership_membershipNamespace-membershipNamespace` and `Root-Elements-A_relatedElement_relationship-relationship` (line 9197) are the other two unions. The latter is nonunique. Relationship.relatedElement is derived but **not** marked as a derived union, despite being subsetted by source and target. |
| Composition | No `aggregation` attribute occurs anywhere in the pinned KerML XMI; the effective MOF aggregation is `none`. KerML still has ownership semantics: Relationship documentation (line 8698) describes deletion of owned related elements, and owner/ownedElement constraints describe transitive ownership. Feature.isComposite is Boolean model data, not the MOF composition attribute. |
| Multiplicity | `lowerValue` and `upperValue` contain UML literals. Absent bound nodes mean one; a present integer/unlimited-natural node with no `value` means zero; upper `value="-1"` means unlimited. Element.ownedRelationship is 0..*, Relationship.owningRelatedElement is 0..1, FeatureTyping.typedFeature is 1..1. |
| Ordered/unique | `isOrdered` defaults false; `isUnique` defaults true. Element.ownedRelationship is ordered and unique. Relationship.relatedElement is ordered and nonunique. The association-owned relationship union is unordered and nonunique. |
| Directionality | `ownedAttribute` declares a class-owned, structurally navigable end; `ownedEnd` declares an association-owned end. `navigableOwnedEnd` can mark navigable association-owned ends, but none occurs here. Member-end order is not a storage direction. Separately, KerML Relationship documentation defines direction from model source to target elements, not a MOF storage convention. |
| Inheritance | `generalization` records contain `general` identity references; properties remain declared on their actual owner. FeatureMembership inherits OwningMembership, Membership, Relationship and Element. Its inherited target resolves through redefinitions to ownedMemberFeature. In this slice redefinitions rename properties; matching display names do not establish identity. |

The closure contains 46 redefinition references and 135 subsetting references.
No selected property has `isReadOnly=true`; generation refuses a future true value
until the write contract is extended. Source `isID`, defaults and retained rule
bodies survive in the golden but are not executed. Element.elementId does not
replace the separately allocated semantic ElementId.

## Generic kernel changes

The kernel gains redefinition/subset identity sets, derived-union metadata,
association/opposite identities, and Class versus Association property ownership.
Binary association descriptors preserve source-ordered member ends and explicit
navigable owned ends. Associations are not registered as metaclasses.

Package paths participate in class name-conflict checking; IDs remain authoritative.
VisibilityKind and FeatureDirectionKind require generic enumeration/literal IDs,
finite domain descriptors and typed enum values instead of arbitrary strings.
Only String and Boolean primitives are needed; no normative unbounded numeric
domain is narrowed to the kernel's existing i64 domain.

Registration checks ownership, targets, bounds, reciprocal opposites, binary end
membership, navigation, subset context/type compatibility and metadata cycles.
Redefinition requires a more specific context, compatible/narrowed target and
bounds, and preservation of uniqueness, collection ordering and composition where
applicable. Scalar narrowing need not retain collection ordering.

Effective inheritance collects declarations by identity and removes transitive
redefined identities before checking names. Diamonds deduplicate one declaration.
Competing sibling redefinitions require an explicit joining redefinition even when
their names differ. Unresolved name conflicts remain errors. `resolve_property`
maps an inherited identity to its effective replacement; `property_named` sees
final effective names. Replaced identities remain metadata, never second slots.
This is structural inheritance, not namespace/member lookup.

The kernel contains no KerML names, hierarchy or rule logic. It is not a MOF/UML
bootstrap. This version refuses n-ary association descriptors (all selected
associations are binary), shared aggregation, read-only property generation and
unsupported value domains. Generic model relationships may still have nonbinary
source/target collections; those are distinct from metamodel associations.

## Canonical truth and storage contract

Model elements inheriting KerML Relationship are distinct from metamodel
associations connecting properties. The pinned Relationship documentation says
direct associations between non-relationship model elements are derived from
reified model relationships. That rule is retained, not implemented.

| Property kind | Current storage contract |
| --- | --- |
| Effective non-derived class property without an association | May be authored with existing typed-slot/cardinality/containment validation. |
| Redefined ancestor identity | Metadata/resolution input only; not a second legal slot on the redefining class. |
| Derived class property, including a union | Cannot be authored. Only revision-bound derived overlays with explicit evidence may supply values. Absence means not computed. No automatic evaluator is installed. |
| Association-owned end, derived or non-derived | Metadata only, never a class slot. Non-navigability does not rewrite the normative derivation flag. |
| Non-derived class-owned association end | Normatively eligible as an authored end, but slot writes, clears and implied-element slot materialization fail explicitly with UnsupportedAssociationStorage until canonical link storage exists. |

This boundary prevents two independently editable slots from becoming duplicate
canonical truth. It also prevents descriptor registration from implying ownership
validation merely because all generated MOF composition flags are false.

The selected future storage contract is one logical link per association occurrence,
with endpoints identified by property IDs and order information for each ordered
end. Both non-derived ends remain authoring/navigation surfaces over the same link.
Neither member-end position, lexical name, UUID order nor an `owned` prefix chooses
a universally authoritative end. The two concrete class-owned pairs requiring this
are Element.ownedRelationship / Relationship.owningRelatedElement and
Relationship.ownedRelatedElement / Element.owningRelationship. Each pairs an ordered
0..* end with a 0..1 end; both ends' non-derived flags are preserved.

Indexes can provide inverse traversal over logical links without a duplicate slot.
Existing `incoming`/`outgoing` indexes demonstrate reference-occurrence indexing,
but are not yet normative association views: the future adapter must account for
redefinition, subsetting, per-end order, nonuniqueness and uncomputed derivations.
An inverse index must not manufacture a writable class property from a
non-navigable association-owned end.

Future union evaluation must collect effective subset contributors in context,
including transitive subset/redefinition relationships, while honoring order and
uniqueness contracts. Flags alone do not provide a union evaluation order or full
membership semantics. Missing contributors/rules must yield not-computed or
unsupported, never a silently empty union. Overlays remain revision-bound views.

Containment direction is separate: generic `composite=true` means owner slot ->
contained target for metamodels declaring MOF composition. Generated KerML flags
are false. KerML ownership directions and cascade/acyclicity obligations remain
in the pinned Relationship/Element documentation and constraints. Enforcing them
belongs above the kernel, with a reviewed association write contract.

## Computed slice and generation

The twelve requested concepts are seed external IDs. Closure follows supertypes,
all declared properties, owners/targets, redefinitions/subsets/opposites,
associations and all their ends. Primitive references resolve against the pinned
dependency. It does not traverse subclasses, operation signatures or rule bodies:
this is structural, not semantic-rule closure. Whole-input import remains the
existing validation step; only this closure becomes runtime descriptors.

The result is 29 classes, 146 class properties, 50 association-owned properties,
80 associations and two enums. No Kernel-layer class is generated. The golden
inventories every property and its exact identity/owner/facts. Class counts below
are declared/effective, after removing redefined ancestors:

| Package | Class: declared/effective properties |
| --- | --- |
| Root::Elements | Element: 18/18; Relationship: 6/24 |
| Root::Namespaces | Namespace: 6/24; Membership: 6/28; OwningMembership: 4/28; Import: 5/28 |
| Root::Annotations | AnnotatingElement: 4/22; Annotation: 5/27; Comment: 2/24; Documentation: 1/24; TextualRepresentation: 3/24 |
| Core::Types | Type: 24/48; Specialization: 3/25; FeatureMembership: 2/28; Multiplicity: 0/73; Conjugation: 3/25; Differencing: 2/24; Disjoining: 3/25; Intersecting: 2/24; Unioning: 2/24 |
| Core::Features | Feature: 25/73; FeatureTyping: 3/25; Subsetting: 3/25; Redefinition: 2/25; CrossSubsetting: 2/24; ReferenceSubsetting: 2/24; FeatureChaining: 2/24; FeatureInverting: 3/25; TypeFeaturing: 3/25 |

Checked-in output is `crates/kerml/src/generated/root_core.rs` and
`standards/generated/kerml-1.0/root-core.golden.json`. Both have generated notices,
generator version `agq-kerml-descriptors/1`, all three input hashes and reproduction
instructions. Rust also pins the golden SHA-256. Runtime `agq-kerml` depends only
on `agq-kernel`: no build script, importer, XML/schema reader, database or transport.

IDs use ADR 0002's unchanged source-qualified v1 encoding and UUID namespace.
Class/property conversions retain kind checks; association/enum/literal IDs wrap
the UUID of their respective kinded source key. Metamodel identity wraps the root
package key. Display names never allocate IDs.

After ordinary locked dependency provisioning, from the repository root:

```sh
cargo run --locked --offline -p agq-metamodel-gen
cargo run --locked --offline -p agq-metamodel-gen -- --check
```

Default commands generate/check IR, Rust and golden together. The existing npm
metamodel check and verification runner automatically cover all three. `--output`
alone preserves Prompt 1's IR-only export/test mode; `--descriptor-output DIRECTORY`
redirects descriptor artifacts by repository-relative path and can accompany it.
All generation is deterministic, with sorted maps, source-ordered lists, LF and no
timestamps. `--check` compares bytes without writes; original inputs are untouched.

## Validation, limitations and next step

Tests compare every compiled descriptor to fresh authoritative import: identity,
package/name, abstractness, direct/effective inheritance, owner, value/reference
target, bounds, ordered/unique/composite/derived/union flags and all property links.
A second small XML reader checks golden flags, bounds, links, owners, names and
direct inheritance without reusing importer field/default helpers. Golden
effective inheritance is traversed independently of the kernel resolver.

Additional tests cover diamonds, legitimate same-name generic and renamed normative
redefinition, conflicting definitions, metadata corruption, atomic derived and
association write refusals, identity independent of display names, deterministic
closure/generation, and stale Rust/golden checks without writes. New KerML tests
use generated descriptors; the illustrative kernel fixture remains isolated.

Actual full-suite results are in [verification/kerml-descriptors-v1](../../verification/kerml-descriptors-v1/).
No passing check implies full semantic conformance. OCL/English rules, defaults,
union evaluation, subset value validation, canonical association storage, ownership
cascade enforcement, JSON scalar nullability and cross-release migration remain
implementation obligations. No parser, namespace lookup, libraries, SysML,
simulation, repository API or existing application runtime integration is added.

Next: implement and validate the two ownership association pairs in `agq-kerml`,
including edits from either end, ordering and atomic link updates, before allowing
normative association writes. Then add evidence-backed derived ownership views.
