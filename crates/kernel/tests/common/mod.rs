//! Illustrative descriptors only: NOT the normative KerML/SysML metamodel.
#![allow(dead_code)]
use agq_kernel::metamodel::*;
use agq_kernel::provenance::DeclaredOrigin;
use agq_kernel::value::{SlotValue, Value};
use agq_kernel::*;
use std::collections::BTreeSet;
use std::sync::Arc;

pub const MM: MetamodelId = MetamodelId::from_u128(1);
pub const ELEMENT: MetaclassId = MetaclassId::from_u128(1);
pub const RELATIONSHIP: MetaclassId = MetaclassId::from_u128(2);
pub const TYPE: MetaclassId = MetaclassId::from_u128(3);
pub const FEATURE: MetaclassId = MetaclassId::from_u128(4);
pub const MEMBERSHIP: MetaclassId = MetaclassId::from_u128(5);
pub const OWNING: MetaclassId = MetaclassId::from_u128(6);
pub const SPECIALIZATION: MetaclassId = MetaclassId::from_u128(7);
pub const TYPING: MetaclassId = MetaclassId::from_u128(8);
pub const PART_DEF: MetaclassId = MetaclassId::from_u128(9);
pub const PART_USAGE: MetaclassId = MetaclassId::from_u128(10);

pub const NAME: PropertyId = PropertyId::from_u128(1);
pub const CONTAINER: PropertyId = PropertyId::from_u128(2);
pub const MEMBER: PropertyId = PropertyId::from_u128(3);
pub const SPECIFIC: PropertyId = PropertyId::from_u128(4);
pub const GENERAL: PropertyId = PropertyId::from_u128(5);
pub const SOURCES: PropertyId = PropertyId::from_u128(6);
pub const TARGETS: PropertyId = PropertyId::from_u128(7);
pub const EFFECTIVE: PropertyId = PropertyId::from_u128(8);
pub const COUNT: PropertyId = PropertyId::from_u128(9);
pub const TAGS: PropertyId = PropertyId::from_u128(10);
pub const BAG: PropertyId = PropertyId::from_u128(11);
pub const CONTAINS: PropertyId = PropertyId::from_u128(12);

pub const ENGINE: ElementId = ElementId::from_u128(1);
pub const VEHICLE: ElementId = ElementId::from_u128(2);
pub const ENGINE_USE: ElementId = ElementId::from_u128(3);
pub const SPORTS: ElementId = ElementId::from_u128(4);
pub const OWNS: ElementId = ElementId::from_u128(5);
pub const TYPED: ElementId = ElementId::from_u128(6);
pub const SPECIALIZES: ElementId = ElementId::from_u128(7);

pub fn authored() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
pub fn model_descriptor() -> MetamodelDescriptor {
    MetamodelDescriptor {
        id: MM,
        name: "Kernel architecture fixture (non-normative)".into(),
        version: Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        uri: "urn:agentique:test:kernel:1".into(),
    }
}
pub fn class(id: MetaclassId, name: &str, supers: &[MetaclassId]) -> MetaclassDescriptor {
    MetaclassDescriptor {
        id,
        name: name.into(),
        package: Vec::new(),
        metamodel: MM,
        direct_supertypes: supers.iter().copied().collect(),
        is_abstract: false,
    }
}
pub fn property(
    id: PropertyId,
    name: &str,
    owner: MetaclassId,
    value_kind: ValueKind,
    multiplicity: Multiplicity,
) -> PropertyDescriptor {
    PropertyDescriptor {
        id,
        name: name.into(),
        owner: PropertyOwner::Class(owner),
        value_kind,
        multiplicity,
        ordered: false,
        unique: true,
        derived: false,
        composite: false,
        redefines: BTreeSet::new(),
        subsets: BTreeSet::new(),
        derived_union: false,
        association: None,
        opposite_ends: BTreeSet::new(),
    }
}
pub fn descriptors() -> (Vec<MetaclassDescriptor>, Vec<PropertyDescriptor>) {
    let mut classes = vec![
        class(ELEMENT, "Element", &[]),
        class(RELATIONSHIP, "Relationship", &[ELEMENT]),
        class(TYPE, "Type", &[ELEMENT]),
        class(FEATURE, "Feature", &[TYPE]),
        class(MEMBERSHIP, "Membership", &[RELATIONSHIP]),
        class(OWNING, "OwningMembership", &[MEMBERSHIP]),
        class(SPECIALIZATION, "Specialization", &[RELATIONSHIP]),
        class(TYPING, "FeatureTyping", &[SPECIALIZATION]),
        class(PART_DEF, "PartDefinition", &[TYPE]),
        class(PART_USAGE, "PartUsage", &[FEATURE]),
    ];
    classes[0].is_abstract = true;
    classes[1].is_abstract = true;
    let mut member = property(
        MEMBER,
        "fixtureOwnedMember",
        OWNING,
        ValueKind::Reference(FEATURE),
        Multiplicity::ONE,
    );
    member.composite = true;
    let mut sources = property(
        SOURCES,
        "fixtureSources",
        RELATIONSHIP,
        ValueKind::Reference(ELEMENT),
        Multiplicity::MANY,
    );
    sources.ordered = true;
    let mut targets = property(
        TARGETS,
        "fixtureTargets",
        RELATIONSHIP,
        ValueKind::Reference(ELEMENT),
        Multiplicity {
            lower: 0,
            upper: Some(3),
        },
    );
    targets.ordered = true;
    targets.unique = false;
    let mut effective = property(
        EFFECTIVE,
        "fixtureEffectiveMembers",
        TYPE,
        ValueKind::Reference(FEATURE),
        Multiplicity::MANY,
    );
    effective.derived = true;
    let mut count = property(
        COUNT,
        "fixtureEffectiveCount",
        TYPE,
        ValueKind::Integer,
        Multiplicity::ONE,
    );
    count.derived = true;
    let mut bag = property(
        BAG,
        "testBag",
        ELEMENT,
        ValueKind::Integer,
        Multiplicity::MANY,
    );
    bag.unique = false;
    let mut contains = property(
        CONTAINS,
        "testContains",
        ELEMENT,
        ValueKind::Reference(ELEMENT),
        Multiplicity::MANY,
    );
    contains.composite = true;
    (
        classes,
        vec![
            property(
                NAME,
                "declaredName",
                ELEMENT,
                ValueKind::String,
                Multiplicity::OPTIONAL,
            ),
            property(
                CONTAINER,
                "fixtureNamespace",
                OWNING,
                ValueKind::Reference(TYPE),
                Multiplicity::ONE,
            ),
            member,
            property(
                SPECIFIC,
                "fixtureSpecific",
                SPECIALIZATION,
                ValueKind::Reference(TYPE),
                Multiplicity::ONE,
            ),
            property(
                GENERAL,
                "fixtureGeneral",
                SPECIALIZATION,
                ValueKind::Reference(TYPE),
                Multiplicity::ONE,
            ),
            sources,
            targets,
            effective,
            count,
            property(
                TAGS,
                "testTags",
                ELEMENT,
                ValueKind::String,
                Multiplicity::MANY,
            ),
            bag,
            contains,
        ],
    )
}
pub fn registry() -> Arc<MetamodelRegistry> {
    let (classes, properties) = descriptors();
    Arc::new(MetamodelRegistry::new([model_descriptor()], classes, properties).unwrap())
}
pub fn scalar_ref(id: ElementId) -> SlotValue {
    SlotValue::Scalar(Value::Reference(id))
}
pub fn text(value: &str) -> SlotValue {
    SlotValue::Scalar(Value::String(value.into()))
}
pub fn set_refs(ids: &[ElementId]) -> SlotValue {
    SlotValue::Set(ids.iter().map(|id| Value::Reference(*id)).collect())
}
pub fn ordered_refs(ids: &[ElementId]) -> SlotValue {
    SlotValue::Ordered(ids.iter().map(|id| Value::Reference(*id)).collect())
}
pub fn vertical() -> Snapshot {
    let empty = Snapshot::new(registry());
    let mut changes = empty.change_set();
    // References may point forward in this transaction.
    changes
        .create(OWNS, OWNING, authored())
        .set(OWNS, CONTAINER, scalar_ref(VEHICLE), authored())
        .set(OWNS, MEMBER, scalar_ref(ENGINE_USE), authored());
    changes
        .create(TYPED, TYPING, authored())
        .set(TYPED, SPECIFIC, scalar_ref(ENGINE_USE), authored())
        .set(TYPED, GENERAL, scalar_ref(ENGINE), authored());
    changes
        .create(SPECIALIZES, SPECIALIZATION, authored())
        .set(SPECIALIZES, SPECIFIC, scalar_ref(SPORTS), authored())
        .set(SPECIALIZES, GENERAL, scalar_ref(VEHICLE), authored());
    for (id, class, name) in [
        (ENGINE, PART_DEF, "Engine"),
        (VEHICLE, PART_DEF, "Vehicle"),
        (ENGINE_USE, PART_USAGE, "engine"),
        (SPORTS, PART_DEF, "SportsCar"),
    ] {
        changes
            .create(id, class, authored())
            .set(id, NAME, text(name), authored());
    }
    empty.apply(&changes).unwrap()
}
pub fn ids(model: &ModelView) -> Vec<ElementId> {
    model.elements().map(ElementRecord::id).collect()
}
/// Compare indexed references to the independent filtered population, including
/// order, duplicate value positions, carrier provenance and borrowed storage.
pub fn assert_incoming_property_index(model: &ModelView) {
    for target in model
        .elements()
        .map(ElementRecord::id)
        .chain([ElementId::from_u128(u128::MAX)])
    {
        for property in model
            .registry()
            .properties()
            .map(|p| p.id)
            .chain([PropertyId::from_u128(u128::MAX)])
        {
            let expected: Vec<_> = model
                .incoming(target)
                .filter(|r| r.property == property)
                .collect();
            let actual: Vec<_> = model.incoming_for_property(target, property).collect();
            assert_eq!(actual, expected, "target={target}, property={property}");
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| std::ptr::eq(*a, b))
            );
        }
    }
}
pub fn string_set(values: &[&str]) -> SlotValue {
    SlotValue::Set(
        values
            .iter()
            .map(|s| Value::String((*s).into()))
            .collect::<BTreeSet<_>>(),
    )
}
