mod common;
use agq_kernel::{derived::*, provenance::*, value::*, *};
use common::*;
use std::{collections::BTreeSet, sync::Arc};

fn key(subject: ElementId, output: u128) -> DerivationKey {
    DerivationKey {
        rule: RuleId::from_u128(400),
        subject,
        output: OutputKey::from_u128(output),
    }
}
fn publication() -> Arc<DerivedOverlay> {
    let source = vertical();
    let mut edit = source.change_set();
    edit.set(OWNS, SOURCES, ordered_refs(&[VEHICLE]), authored());
    let source = source.apply(&edit).unwrap();
    let mut b = DerivationBuilder::new(source);
    b.element(
        key(VEHICLE, 1),
        PART_DEF,
        [(NAME, text("derived library type"))],
        BTreeSet::new(),
    );
    b.extend_ordered_references(
        OWNS,
        SOURCES,
        vec![key(VEHICLE, 1).element_id()],
        Explanation {
            rule: RuleId::from_u128(400),
            dependencies: BTreeSet::new(),
        },
    );
    b.property(
        VEHICLE,
        EFFECTIVE,
        SlotValue::Set(BTreeSet::from([Value::Reference(ENGINE_USE)])),
        Explanation {
            rule: RuleId::from_u128(400),
            dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(ENGINE_USE))]),
        },
    );
    Arc::new(b.build().unwrap())
}

#[test]
fn independent_authored_histories_share_records_and_preserve_dependency_provenance() {
    let library = publication();
    let a = Snapshot::with_immutable_dependency(library.clone());
    let b = Snapshot::with_immutable_dependency(library.clone());
    assert_ne!(a.revision(), b.revision());
    assert!(Arc::ptr_eq(
        a.immutable_dependency().unwrap(),
        b.immutable_dependency().unwrap()
    ));
    for id in [VEHICLE, key(VEHICLE, 1).element_id()] {
        assert!(std::ptr::eq(
            a.model().element(id).unwrap(),
            library.model().element(id).unwrap()
        ));
    }
    let local = ElementId::from_u128(900);
    let relationship = ElementId::from_u128(901);
    let mut edit = a.change_set();
    edit.create(local, PART_DEF, authored())
        .set(local, NAME, text("authored"), authored());
    edit.create(relationship, SPECIALIZATION, authored())
        .set(relationship, SPECIFIC, scalar_ref(local), authored())
        .set(
            relationship,
            GENERAL,
            scalar_ref(key(VEHICLE, 1).element_id()),
            authored(),
        );
    let a = a.apply(&edit).unwrap();
    assert_incoming_property_index(library.model());
    assert_incoming_property_index(a.model());
    assert_incoming_property_index(b.model());
    assert!(b.model().element(local).is_none());
    assert!(library.model().element(local).is_none());
    for record in library.model().elements() {
        assert!(std::ptr::eq(
            a.model().element(record.id()).unwrap(),
            record
        ));
    }
    assert!(matches!(
        a.model()
            .navigation_slot(VEHICLE, EFFECTIVE)
            .unwrap()
            .origin(),
        Origin::Derived(_)
    ));
    let mut derived = DerivationBuilder::new(a);
    let next = key(key(VEHICLE, 1).element_id(), 2);
    derived.element(
        next,
        PART_DEF,
        [(NAME, text("local implication"))],
        BTreeSet::new(),
    );
    let derived = derived.build().unwrap();
    assert_incoming_property_index(derived.model());
    assert!(
        derived
            .explain(FactKey::Element(next.element_id()))
            .unwrap()
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(
                key(VEHICLE, 1).element_id()
            )))
    );
    assert_eq!(
        derived.explain(FactKey::Element(key(VEHICLE, 1).element_id())),
        library.explain(FactKey::Element(key(VEHICLE, 1).element_id()))
    );
}

#[test]
fn dependency_records_and_collections_cannot_be_changed_in_authored_or_derived_layers() {
    let library = publication();
    let project = Snapshot::with_immutable_dependency(library.clone());
    for id in [VEHICLE, key(VEHICLE, 1).element_id()] {
        for case in 0..4 {
            let mut changes = project.change_set();
            match case {
                0 => {
                    changes.remove(id);
                }
                1 => {
                    changes.set(id, NAME, text("replacement"), authored());
                }
                2 => {
                    changes.clear(id, NAME);
                }
                _ => {
                    changes.set_origin(id, authored());
                }
            }
            assert!(matches!(
                project.apply(&changes),
                Err(ModelError::ImmutableDependency(_))
            ));
            assert_eq!(project.model().element(id), library.model().element(id));
        }
    }
    let mut derived = DerivationBuilder::new(project.clone());
    derived.extend_ordered_references(
        OWNS,
        SOURCES,
        vec![VEHICLE],
        Explanation {
            rule: RuleId::from_u128(401),
            dependencies: BTreeSet::new(),
        },
    );
    assert!(matches!(
        derived.build(),
        Err(DerivationError::Model(ModelError::ImmutableDependency(_)))
    ));
    let mut derived = DerivationBuilder::new(project);
    derived.property(
        VEHICLE,
        COUNT,
        SlotValue::Scalar(Value::Integer(1.into())),
        Explanation {
            rule: RuleId::from_u128(401),
            dependencies: BTreeSet::new(),
        },
    );
    assert!(matches!(
        derived.build(),
        Err(DerivationError::Model(ModelError::ImmutableDependency(_)))
    ));
}

#[test]
fn inverse_edits_cannot_attach_local_records_to_an_immutable_owner() {
    use agq_kernel::metamodel::*;
    let children = PropertyId::from_u128(700);
    let parent = PropertyId::from_u128(701);
    let association = AssociationId::from_u128(702);
    let mut a = property(
        children,
        "children",
        PART_DEF,
        ValueKind::Reference(PART_DEF),
        Multiplicity::MANY,
    );
    a.ordered = true;
    a.association = Some(association);
    a.opposite_ends.insert(parent);
    let mut b = property(
        parent,
        "parent",
        PART_DEF,
        ValueKind::Reference(PART_DEF),
        Multiplicity::OPTIONAL,
    );
    b.association = Some(association);
    b.opposite_ends.insert(children);
    let registry = MetamodelRegistry::from_descriptors(DescriptorSet {
        models: vec![model_descriptor()],
        classes: vec![class(PART_DEF, "Node", &[])],
        properties: vec![a, b],
        associations: vec![AssociationDescriptor {
            id: association,
            name: "Tree".into(),
            package: vec![],
            metamodel: MM,
            member_ends: vec![children, parent],
            navigable_owned_ends: BTreeSet::new(),
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
        }],
        ..Default::default()
    })
    .unwrap();
    let base = Snapshot::new(Arc::new(registry));
    let mut edit = base.change_set();
    edit.create(VEHICLE, PART_DEF, authored());
    let source = base.apply(&edit).unwrap();
    let dependency = Arc::new(DerivationBuilder::new(source).build().unwrap());
    let project = Snapshot::with_immutable_dependency(dependency.clone());
    let mut edit = project.change_set();
    edit.create(ENGINE, PART_DEF, authored());
    let project = project.apply(&edit).unwrap();
    let mut edit = project.change_set();
    edit.move_inverse(ENGINE, parent, Some((VEHICLE, 0)), authored());
    assert!(
        matches!(project.apply(&edit),Err(ModelError::ImmutableDependency(FactKey::Property{element,property})) if element==VEHICLE && property==children)
    );
    assert!(std::ptr::eq(
        project.model().element(VEHICLE).unwrap(),
        dependency.model().element(VEHICLE).unwrap()
    ));
}

#[test]
fn local_composite_slots_cannot_take_ownership_of_dependency_roots() {
    let dependency = publication();
    let project = Snapshot::with_immutable_dependency(dependency.clone());
    let local = ElementId::from_u128(900);
    let mut changes = project.change_set();
    changes.create(local, PART_DEF, authored()).set(
        local,
        CONTAINS,
        set_refs(&[VEHICLE]),
        authored(),
    );
    for result in [
        project.apply(&changes).map(|_| ()),
        project.preview(&changes).map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(ModelError::ImmutableDependency(FactKey::Element(VEHICLE)))
        ));
    }
    assert!(project.model().element(local).is_none());
    assert_eq!(
        project.model().incoming(VEHICLE).collect::<Vec<_>>(),
        dependency.model().incoming(VEHICLE).collect::<Vec<_>>()
    );
    let mut derived = DerivationBuilder::new(project.clone());
    derived.element(
        key(VEHICLE, 900),
        PART_DEF,
        [(CONTAINS, set_refs(&[VEHICLE]))],
        BTreeSet::new(),
    );
    assert!(matches!(
        derived.build(),
        Err(DerivationError::Model(ModelError::ImmutableDependency(
            FactKey::Element(VEHICLE)
        )))
    ));

    // Ordinary references to library elements and entirely local ownership
    // remain legal; the guard protects ownership across the dependency boundary.
    let child = ElementId::from_u128(901);
    let relationship = ElementId::from_u128(902);
    let mut changes = project.change_set();
    changes
        .create(local, PART_DEF, authored())
        .create(child, PART_DEF, authored())
        .set(local, CONTAINS, set_refs(&[child]), authored())
        .create(relationship, SPECIALIZATION, authored())
        .set(relationship, SPECIFIC, scalar_ref(local), authored())
        .set(relationship, GENERAL, scalar_ref(VEHICLE), authored());
    let authored = project.apply(&changes).unwrap();
    DerivationBuilder::new(authored).build().unwrap();
}
