mod common;
use agq_kernel::{metamodel::*, value::*, *};
use common::*;
use std::{collections::BTreeSet, sync::Arc};

const ASSOCIATION: AssociationId = AssociationId::from_u128(1);
fn descriptors() -> DescriptorSet {
    let mut forward = property(
        NAME,
        "targets",
        ELEMENT,
        ValueKind::Reference(TYPE),
        Multiplicity::MANY,
    );
    forward.ordered = true;
    forward.association = Some(ASSOCIATION);
    forward.opposite_ends.insert(MEMBER);
    let mut inverse = property(
        MEMBER,
        "sources",
        TYPE,
        ValueKind::Reference(ELEMENT),
        Multiplicity::MANY,
    );
    inverse.owner = PropertyOwner::Association(ASSOCIATION);
    inverse.association = Some(ASSOCIATION);
    inverse.opposite_ends.insert(NAME);
    DescriptorSet {
        sources: Default::default(),
        reviews: Vec::new(),
        primitives: Vec::new(),
        models: vec![model_descriptor()],
        classes: vec![class(ELEMENT, "Source", &[]), class(TYPE, "Target", &[])],
        properties: vec![forward, inverse],
        associations: vec![AssociationDescriptor {
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
            id: ASSOCIATION,
            name: "Links".into(),
            package: vec![],
            metamodel: MM,
            member_ends: vec![NAME, MEMBER],
            navigable_owned_ends: BTreeSet::new(),
        }],
        enumerations: vec![],
    }
}

#[test]
fn a_single_navigation_slot_is_canonical_and_atomic() {
    let registry = Arc::new(MetamodelRegistry::from_descriptors(descriptors()).unwrap());
    assert!(registry.supports_slot_storage(NAME).unwrap());
    assert!(!registry.supports_slot_storage(MEMBER).unwrap());
    let empty = Snapshot::new(registry);
    let mut change = empty.change_set();
    change
        .create(ENGINE, ELEMENT, authored())
        .create(VEHICLE, TYPE, authored())
        .create(SPORTS, TYPE, authored());
    change.set(
        ENGINE,
        NAME,
        SlotValue::Ordered(vec![Value::Reference(SPORTS), Value::Reference(VEHICLE)]),
        authored(),
    );
    let before = empty.apply(&change).unwrap();
    assert_eq!(before.model().element(ENGINE).unwrap().slots().count(), 1);
    assert_eq!(before.model().element(VEHICLE).unwrap().slots().count(), 0);
    assert_eq!(before.model().incoming(VEHICLE).count(), 1);
    let mut change = before.change_set();
    change.clear(ENGINE, NAME);
    let after = before.apply(&change).unwrap();
    assert_eq!(after.model().incoming(VEHICLE).count(), 0);
    assert_eq!(before.model().incoming(VEHICLE).count(), 1);
    let mut invalid = before.change_set();
    invalid.remove(VEHICLE);
    assert!(matches!(
        before.apply(&invalid),
        Err(ModelError::DanglingReference { .. })
    ));
}

#[test]
fn inverse_order_bounds_navigation_and_two_class_ends_require_link_storage() {
    for case in 0..6 {
        let mut input = descriptors();
        match case {
            0 => input.properties[1].ordered = true,
            1 => input.properties[1].multiplicity.upper = Some(1),
            2 => input.properties[1].multiplicity.lower = 1,
            3 => {
                input.associations[0].navigable_owned_ends.insert(MEMBER);
            }
            4 => input.properties[1].owner = PropertyOwner::Class(TYPE),
            5 => input.properties[0].unique = false,
            _ => unreachable!(),
        }
        let registry = Arc::new(MetamodelRegistry::from_descriptors(input).unwrap());
        assert!(
            !registry.supports_slot_storage(NAME).unwrap(),
            "case {case}"
        );
        let snapshot = Snapshot::new(registry);
        let mut change = snapshot.change_set();
        change
            .create(ENGINE, ELEMENT, authored())
            .create(VEHICLE, TYPE, authored());
        change.set(
            ENGINE,
            NAME,
            SlotValue::Ordered(vec![Value::Reference(VEHICLE)]),
            authored(),
        );
        assert!(matches!(
            snapshot.apply(&change),
            Err(ModelError::UnsupportedAssociationStorage(NAME))
        ));
        assert!(snapshot.model().is_empty());
    }
}
