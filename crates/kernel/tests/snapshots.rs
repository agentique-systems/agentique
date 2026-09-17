mod common;
use agq_kernel::metamodel::Multiplicity;
use agq_kernel::provenance::*;
use agq_kernel::value::*;
use agq_kernel::*;
use common::*;

#[test]
fn vehicle_engine_sports_car_without_source_text() {
    let snapshot = vertical();
    let model = snapshot.model();
    assert_eq!(
        ids(model),
        vec![
            ENGINE,
            VEHICLE,
            ENGINE_USE,
            SPORTS,
            OWNS,
            TYPED,
            SPECIALIZES
        ]
    );
    for (id, class) in [
        (OWNS, OWNING),
        (TYPED, TYPING),
        (SPECIALIZES, SPECIALIZATION),
    ] {
        assert_eq!(model.element(id).unwrap().metaclass(), class);
        assert_eq!(model.outgoing(id).count(), 2);
    }
    assert_eq!(
        model
            .instances(RELATIONSHIP, true)
            .unwrap()
            .map(ElementRecord::id)
            .collect::<Vec<_>>(),
        vec![OWNS, TYPED, SPECIALIZES]
    );
    assert_eq!(model.instances(SPECIALIZATION, false).unwrap().count(), 1);
    assert_eq!(
        model
            .incoming(ENGINE_USE)
            .map(|r| (r.source, r.property))
            .collect::<Vec<_>>(),
        vec![(OWNS, MEMBER), (TYPED, SPECIFIC)]
    );
    assert_eq!(model.element(VEHICLE).unwrap().slots().count(), 1);
    assert_eq!(model.element(ENGINE_USE).unwrap().slots().count(), 1);
    assert_eq!(model.element(SPORTS).unwrap().slots().count(), 1);
    assert_eq!(model.outgoing(SPORTS).count(), 0); // No copied inherited engine.
}

#[test]
fn renaming_preserves_semantic_identity_and_previous_snapshot() {
    let old = vertical();
    let mut changes = old.change_set();
    changes.set(ENGINE, NAME, text("Renamed"), authored());
    let next = old.apply(&changes).unwrap();
    assert_ne!(old.revision(), next.revision());
    assert_eq!(ids(old.model()), ids(next.model()));
    assert_eq!(
        old.model()
            .element(ENGINE)
            .unwrap()
            .slot(NAME)
            .unwrap()
            .value(),
        &text("Engine")
    );
    assert_eq!(
        next.model()
            .element(ENGINE)
            .unwrap()
            .slot(NAME)
            .unwrap()
            .value(),
        &text("Renamed")
    );
    assert!(std::ptr::eq(
        old.model().element(VEHICLE).unwrap(),
        next.model().element(VEHICLE).unwrap()
    ));
    assert_eq!(
        next.model().incoming(ENGINE).copied().collect::<Vec<_>>(),
        old.model().incoming(ENGINE).copied().collect::<Vec<_>>()
    );
}

#[test]
fn names_are_not_identity_and_source_origin_is_not_identity() {
    let old = vertical();
    let mut changes = old.change_set();
    let source = SourceOrigin {
        document: DocumentId::from_u128(1),
        revision: SourceRevisionId::from_u128(1),
        range: ByteRange::new(0, 6).unwrap(),
        syntax_node: Some(SyntaxNodeId::from_u128(1)),
    };
    changes.set(
        VEHICLE,
        NAME,
        text("Engine"),
        DeclaredOrigin::Authored {
            source: Some(source.clone()),
        },
    );
    let next = old.apply(&changes).unwrap();
    assert_ne!(
        next.model().element(ENGINE).unwrap().id(),
        next.model().element(VEHICLE).unwrap().id()
    );
    assert_eq!(
        next.model()
            .element(VEHICLE)
            .unwrap()
            .slot(NAME)
            .unwrap()
            .origin(),
        &Origin::Declared(DeclaredOrigin::Authored {
            source: Some(source)
        })
    );
    assert!(ByteRange::new(7, 2).is_err());
}

#[test]
fn failed_changes_are_atomic_and_illegal_slots_rejected() {
    let old = vertical();
    let mut changes = old.change_set();
    changes
        .set(ENGINE, NAME, text("Should not publish"), authored())
        .set(ENGINE, MEMBER, scalar_ref(ENGINE_USE), authored());
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::IllegalProperty {
            element: ENGINE,
            property: MEMBER,
            ..
        })
    ));
    assert_eq!(
        old.model()
            .element(ENGINE)
            .unwrap()
            .slot(NAME)
            .unwrap()
            .value(),
        &text("Engine")
    );
    assert_eq!(old.model().element(ENGINE).unwrap().slots().count(), 1);
    let mut changes = old.change_set();
    changes.clear(ENGINE, MEMBER);
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::IllegalProperty { .. })
    ));
}

#[test]
fn dangling_and_wrongly_typed_references_are_rejected() {
    let old = vertical();
    for (target, dangling) in [(ElementId::from_u128(99), true), (OWNS, false)] {
        let mut changes = old.change_set();
        changes.set(TYPED, GENERAL, scalar_ref(target), authored());
        let error = old.apply(&changes).unwrap_err();
        if dangling {
            assert!(matches!(error,ModelError::DanglingReference { target:t,.. } if t==target));
        } else {
            assert!(matches!(
                error,
                ModelError::ReferenceType { target: OWNS, .. }
            ));
        }
    }
    let mut changes = old.change_set();
    changes.remove(ENGINE);
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::DanglingReference { target: ENGINE, .. })
    ));
}

#[test]
fn deletion_rebuilds_all_indexes_and_ids_cannot_be_reused() {
    let old = vertical();
    let mut changes = old.change_set();
    changes.remove(OWNS).remove(TYPED).remove(ENGINE_USE);
    let next = old.apply(&changes).unwrap();
    for id in [OWNS, TYPED, ENGINE_USE] {
        assert!(next.model().element(id).is_none());
        assert_eq!(next.model().incoming(id).count(), 0);
        assert_eq!(next.model().outgoing(id).count(), 0);
    }
    assert_eq!(next.model().incoming(ENGINE).count(), 0);
    assert_eq!(next.model().instances(TYPING, true).unwrap().count(), 0);
    assert_eq!(
        next.model().instances(RELATIONSHIP, true).unwrap().count(),
        1
    );
    assert_eq!(old.model().len(), 7);
    let mut reuse = next.change_set();
    reuse.create(ENGINE_USE, PART_USAGE, authored());
    assert_eq!(
        next.apply(&reuse).unwrap_err(),
        ModelError::ReusedIdentity(ENGINE_USE)
    );
}

#[test]
fn lower_upper_multiplicity_and_scalar_shape_are_enforced() {
    let old = vertical();
    let mut changes = old.change_set();
    changes.clear(TYPED, GENERAL);
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::Multiplicity {
            property: GENERAL,
            required: Multiplicity::ONE,
            actual: 0,
            ..
        })
    ));
    let mut changes = old.change_set();
    changes.set(
        OWNS,
        TARGETS,
        ordered_refs(&[ENGINE, VEHICLE, SPORTS, ENGINE_USE]),
        authored(),
    );
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::Multiplicity {
            property: TARGETS,
            actual: 4,
            ..
        })
    ));
    let mut changes = old.change_set();
    changes.set(
        ENGINE,
        NAME,
        SlotValue::Ordered(vec![Value::String("x".into())]),
        authored(),
    );
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::Shape {
            expected: SlotShape::Scalar,
            ..
        })
    ));
    let mut changes = old.change_set();
    changes.set(
        ENGINE,
        NAME,
        SlotValue::Scalar(Value::Boolean(true)),
        authored(),
    );
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::ValueKind { .. })
    ));
}

#[test]
fn ordered_nonbinary_relationships_and_duplicate_reference_positions_are_preserved() {
    let old = vertical();
    let mut changes = old.change_set();
    changes
        .set(
            OWNS,
            SOURCES,
            ordered_refs(&[SPORTS, VEHICLE, ENGINE]),
            authored(),
        )
        .set(
            OWNS,
            TARGETS,
            ordered_refs(&[ENGINE_USE, ENGINE_USE]),
            authored(),
        );
    let next = old.apply(&changes).unwrap();
    assert_eq!(
        next.model()
            .element(OWNS)
            .unwrap()
            .slot(SOURCES)
            .unwrap()
            .value(),
        &ordered_refs(&[SPORTS, VEHICLE, ENGINE])
    );
    assert_eq!(
        next.model()
            .outgoing(OWNS)
            .filter(|r| r.property == SOURCES)
            .map(|r| (r.position, r.target))
            .collect::<Vec<_>>(),
        vec![(0, SPORTS), (1, VEHICLE), (2, ENGINE)]
    );
    assert_eq!(
        next.model()
            .incoming(ENGINE_USE)
            .filter(|r| r.property == TARGETS)
            .map(|r| r.position)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    let mut bad = old.change_set();
    bad.set(OWNS, SOURCES, ordered_refs(&[ENGINE, ENGINE]), authored());
    assert!(matches!(
        old.apply(&bad),
        Err(ModelError::DuplicateValue {
            property: SOURCES,
            ..
        })
    ));
}

#[test]
fn unordered_values_are_canonical_and_absent_differs_from_empty() {
    let old = vertical();
    let mut changes = old.change_set();
    changes
        .set(ENGINE, TAGS, string_set(&["b", "a", "a"]), authored())
        .set(
            ENGINE,
            BAG,
            SlotValue::Bag(vec![
                Value::Integer(2),
                Value::Integer(1),
                Value::Integer(2),
            ]),
            authored(),
        );
    let next = old.apply(&changes).unwrap();
    assert_eq!(
        next.model()
            .element(ENGINE)
            .unwrap()
            .slot(TAGS)
            .unwrap()
            .value()
            .values()
            .cloned()
            .collect::<Vec<_>>(),
        vec![Value::String("a".into()), Value::String("b".into())]
    );
    assert_eq!(
        next.model()
            .element(ENGINE)
            .unwrap()
            .slot(BAG)
            .unwrap()
            .value(),
        &SlotValue::Bag(vec![
            Value::Integer(1),
            Value::Integer(2),
            Value::Integer(2)
        ])
    );
    let mut changes = next.change_set();
    changes
        .set(ENGINE, TAGS, string_set(&[]), authored())
        .clear(ENGINE, BAG);
    let final_model = next.apply(&changes).unwrap();
    let engine = final_model.model().element(ENGINE).unwrap();
    assert_eq!(engine.slot(TAGS).unwrap().value().values().count(), 0);
    assert!(engine.slot(BAG).is_none());
}

#[test]
fn composites_reject_multiple_owners_and_cycles_without_cascading() {
    let old = vertical();
    let mut changes = old.change_set();
    changes.set(ENGINE, CONTAINS, set_refs(&[ENGINE_USE]), authored());
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::MultipleContainers {
            target: ENGINE_USE,
            ..
        })
    ));
    let mut changes = old.change_set();
    changes
        .set(ENGINE, CONTAINS, set_refs(&[VEHICLE]), authored())
        .set(VEHICLE, CONTAINS, set_refs(&[ENGINE, SPORTS]), authored());
    assert_eq!(
        old.apply(&changes).unwrap_err(),
        ModelError::ContainmentCycle(vec![ENGINE, VEHICLE])
    );
    let mut changes = old.change_set();
    changes.set(ENGINE, CONTAINS, set_refs(&[ENGINE, SPORTS]), authored());
    assert_eq!(
        old.apply(&changes).unwrap_err(),
        ModelError::ContainmentCycle(vec![ENGINE])
    );
    let mut changes = old.change_set();
    changes.remove(OWNS);
    assert!(
        old.apply(&changes)
            .unwrap()
            .model()
            .element(ENGINE_USE)
            .is_some()
    );
}

#[test]
fn changeset_base_and_revision_identity_are_not_ambiguous() {
    let old = vertical();
    let mut changes = old.change_set();
    changes.set(ENGINE, NAME, text("one"), authored());
    let next = old.apply(&changes).unwrap();
    let replay = old.apply(&changes).unwrap();
    assert_eq!(next.revision(), replay.revision());
    assert_eq!(
        next.model().elements().collect::<Vec<_>>(),
        replay.model().elements().collect::<Vec<_>>()
    );
    assert!(matches!(
        next.apply(&changes),
        Err(ModelError::StaleChangeSet { .. })
    ));
    changes.set(ENGINE, NAME, text("two"), authored());
    let branch = old.apply(&changes).unwrap();
    assert_ne!(next.revision(), branch.revision());
    assert_eq!(
        next.model()
            .element(ENGINE)
            .unwrap()
            .slot(NAME)
            .unwrap()
            .value(),
        &text("one")
    );
}

#[test]
fn abstract_instances_unknown_classes_and_derived_writes_are_rejected() {
    let old = vertical();
    for class in [ELEMENT, MetaclassId::from_u128(99)] {
        let mut changes = old.change_set();
        changes.create(ElementId::from_u128(99), class, authored());
        assert!(old.apply(&changes).is_err());
    }
    let mut changes = old.change_set();
    changes.set(SPORTS, EFFECTIVE, set_refs(&[ENGINE_USE]), authored());
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::DerivedWrite { .. })
    ));
    let mut changes = old.change_set();
    changes.clear(SPORTS, EFFECTIVE);
    assert!(matches!(
        old.apply(&changes),
        Err(ModelError::DerivedWrite { .. })
    ));
    // A missing required derived property does not pretend elaboration is complete.
    assert!(old.model().element(SPORTS).unwrap().slot(COUNT).is_none());
}

#[test]
fn snapshot_supports_concurrent_readers() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Snapshot>();
    send_sync::<ModelView>();
    let snapshot = vertical();
    let clone = snapshot.clone();
    assert_eq!(
        std::thread::spawn(move || ids(clone.model()))
            .join()
            .unwrap(),
        ids(snapshot.model())
    );
}

#[test]
fn public_iteration_and_indexes_do_not_depend_on_creation_order() {
    let empty = Snapshot::new(registry());
    let make = |order: &[ElementId]| {
        let mut changes = empty.change_set();
        for id in order {
            changes
                .create(*id, TYPE, authored())
                .set(*id, NAME, text("same"), authored());
        }
        empty.apply(&changes).unwrap()
    };
    let a = make(&[SPORTS, VEHICLE, ENGINE]);
    let b = make(&[ENGINE, SPORTS, VEHICLE]);
    assert_eq!(
        a.model().elements().collect::<Vec<_>>(),
        b.model().elements().collect::<Vec<_>>()
    );
    assert_eq!(
        a.model()
            .instances(ELEMENT, true)
            .unwrap()
            .collect::<Vec<_>>(),
        b.model()
            .instances(ELEMENT, true)
            .unwrap()
            .collect::<Vec<_>>()
    );
}
