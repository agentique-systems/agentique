mod common;
use agq_kernel::derived::*;
use agq_kernel::provenance::*;
use agq_kernel::value::*;
use agq_kernel::*;
use common::*;
use std::collections::BTreeSet;

const RULE: RuleId = RuleId::from_u128(1);

#[test]
fn shared_input_preserves_identity_evidence_and_caller_allocation() {
    use std::sync::Arc;
    let snapshot = vertical();
    let output = key(VEHICLE);
    let proof = Arc::new(evidence(&[]));
    let mut shared = DerivationBuilder::new(snapshot.clone());
    shared.element_with_explanation(output, PART_DEF, [(NAME, text("derived"))], proof.clone());
    let shared = shared.build().unwrap();
    assert!(
        proof.dependencies.is_empty(),
        "automatic evidence must not mutate caller input"
    );
    let mut owned = DerivationBuilder::new(snapshot.clone());
    owned.element(output, PART_DEF, [(NAME, text("derived"))], BTreeSet::new());
    let owned = owned.build().unwrap();
    assert!(shared.model().elements().eq(owned.model().elements()));
    assert!(shared.facts().eq(owned.facts()));

    let complete = Arc::new(evidence(&[Dependency::Declared(FactKey::Element(VEHICLE))]));
    let mut direct = DerivationBuilder::new(snapshot.clone());
    direct.element_with_explanation(
        output,
        PART_DEF,
        [(NAME, text("derived"))],
        complete.clone(),
    );
    let direct = direct.build().unwrap();
    let Origin::Derived(stored) = direct
        .model()
        .element(output.element_id())
        .unwrap()
        .origin()
    else {
        panic!()
    };
    assert!(Arc::ptr_eq(&complete, stored));

    let mut invalid = DerivationBuilder::new(snapshot.clone());
    invalid.element_with_explanation(
        output,
        PART_DEF,
        [(NAME, text("derived"))],
        Arc::new(Explanation {
            rule: RuleId::from_u128(999),
            dependencies: BTreeSet::new(),
        }),
    );
    assert!(matches!(
        invalid.build(),
        Err(DerivationError::ExplanationRuleMismatch { .. })
    ));
    assert!(snapshot.model().element(output.element_id()).is_none());
}

#[test]
fn equal_immutable_explanations_share_storage_and_preserve_source_evidence() {
    use std::sync::Arc;
    let snapshot = vertical();
    let mut changes = snapshot.change_set();
    changes.set(
        OWNS,
        SOURCES,
        SlotValue::Ordered(vec![Value::Reference(VEHICLE)]),
        authored(),
    );
    let snapshot = snapshot.apply(&changes).unwrap();
    let a = key(OWNS);
    let b = DerivationKey {
        output: OutputKey::from_u128(2),
        ..a
    };
    let mut builder = DerivationBuilder::new(snapshot.clone());
    for output in [a, b] {
        builder.element(output, PART_DEF, [(NAME, text("derived"))], BTreeSet::new());
    }
    builder.extend_ordered_references(OWNS, SOURCES, vec![a.element_id()], evidence(&[]));
    let overlay = builder.build().unwrap();
    let Origin::Derived(a_proof) = overlay.model().element(a.element_id()).unwrap().origin() else {
        panic!()
    };
    let Origin::Derived(b_proof) = overlay.model().element(b.element_id()).unwrap().origin() else {
        panic!()
    };
    assert!(Arc::ptr_eq(a_proof, b_proof));
    assert!(std::ptr::eq(
        a_proof.as_ref(),
        overlay.explain(FactKey::Element(a.element_id())).unwrap()
    ));
    assert_eq!(format!("{a_proof:?}"), format!("{:?}", a_proof.as_ref()));
    assert_eq!(
        overlay.model().declared_fact_origin(prop(OWNS, SOURCES)),
        snapshot.model().declared_fact_origin(prop(OWNS, SOURCES))
    );
    assert_eq!(
        overlay
            .model()
            .declared_fact_origin(FactKey::Element(a.element_id())),
        None
    );
    let dependent = Snapshot::with_immutable_dependency(Arc::new(overlay));
    let dependent = dependent.apply(&dependent.change_set()).unwrap();
    assert_eq!(
        dependent.model().declared_fact_origin(prop(OWNS, SOURCES)),
        snapshot.model().declared_fact_origin(prop(OWNS, SOURCES))
    );
}

#[test]
fn staged_collection_extensions_preserve_the_declared_prefix_and_all_contributors() {
    let snapshot = vertical();
    let mut changes = snapshot.change_set();
    changes.set(
        OWNS,
        SOURCES,
        SlotValue::Ordered(vec![Value::Reference(VEHICLE)]),
        authored(),
    );
    let snapshot = snapshot.apply(&changes).unwrap();
    let a = key(OWNS);
    let b = DerivationKey {
        output: OutputKey::from_u128(2),
        ..a
    };
    let mut first = DerivationBuilder::new(snapshot.clone());
    first.element(a, PART_DEF, [(NAME, text("first"))], BTreeSet::new());
    first.extend_ordered_references(OWNS, SOURCES, vec![a.element_id()], evidence(&[]));
    let first = first.build().unwrap();
    let mut second = DerivationBuilder::from_overlay(first.clone());
    second.element(b, PART_DEF, [(NAME, text("second"))], BTreeSet::new());
    second.extend_ordered_references(OWNS, SOURCES, vec![b.element_id()], evidence(&[]));
    let second = second.build().unwrap();
    let values = |m: &ModelView| {
        m.element(OWNS)
            .unwrap()
            .slot(SOURCES)
            .unwrap()
            .value()
            .clone()
    };
    let mut expected: Vec<_> = values(snapshot.model()).values().cloned().collect();
    expected.extend([
        Value::Reference(a.element_id()),
        Value::Reference(b.element_id()),
    ]);
    assert_eq!(values(second.model()), SlotValue::Ordered(expected));
    assert_ne!(values(first.model()), values(second.model()));
    let proof = second.explain(prop(OWNS, SOURCES)).unwrap();
    for contribution in [
        Dependency::Declared(prop(OWNS, SOURCES)),
        Dependency::Derived(FactKey::Element(a.element_id())),
        Dependency::Derived(FactKey::Element(b.element_id())),
    ] {
        assert!(proof.dependencies.contains(&contribution));
    }
    assert!(
        !proof
            .dependencies
            .contains(&Dependency::Derived(prop(OWNS, SOURCES)))
    );
}

#[test]
fn extending_an_earlier_fact_rechecks_cycles_through_retained_proofs() {
    let snapshot = vertical();
    let output = key(VEHICLE);
    let aggregate = prop(OWNS, SOURCES);
    let mut first = DerivationBuilder::new(snapshot.clone());
    first.extend_ordered_references(OWNS, SOURCES, vec![VEHICLE], evidence(&[]));
    first.element(
        output,
        PART_DEF,
        [(NAME, text("depends on earlier aggregate"))],
        BTreeSet::from([Dependency::Derived(aggregate)]),
    );
    let first = first.build().unwrap();
    let original = first.explain(aggregate).unwrap().clone();
    let mut second = DerivationBuilder::from_overlay(first.clone());
    second.extend_ordered_references(OWNS, SOURCES, vec![output.element_id()], evidence(&[]));
    assert_eq!(
        second.build().unwrap_err(),
        DerivationError::DependencyCycle(vec![FactKey::Element(output.element_id()), aggregate])
    );
    assert_eq!(first.explain(aggregate), Some(&original));
}
fn evidence(dependencies: &[Dependency]) -> Explanation {
    Explanation {
        rule: RULE,
        dependencies: dependencies.iter().copied().collect(),
    }
}
fn prop(element: ElementId, property: PropertyId) -> FactKey {
    FactKey::Property { element, property }
}
fn key(subject: ElementId) -> DerivationKey {
    DerivationKey {
        rule: RULE,
        subject,
        output: OutputKey::from_u128(1),
    }
}

#[test]
fn monotone_reference_extension_preserves_declared_order_and_evidence() {
    let base = vertical();
    let mut changes = base.change_set();
    changes.set(
        OWNS,
        SOURCES,
        SlotValue::Ordered(vec![Value::Reference(VEHICLE), Value::Reference(ENGINE)]),
        authored(),
    );
    let snapshot = base.apply(&changes).unwrap();
    let implied = key(OWNS);
    let build = |additions: Vec<ElementId>| {
        let mut b = DerivationBuilder::new(snapshot.clone());
        b.element(
            implied,
            PART_DEF,
            [(NAME, text("implied"))],
            BTreeSet::new(),
        );
        b.extend_ordered_references(OWNS, SOURCES, additions, evidence(&[]));
        b.build()
    };
    let result = build(vec![implied.element_id()]).unwrap();
    let values = |model: &ModelView| {
        model
            .element(OWNS)
            .unwrap()
            .slot(SOURCES)
            .unwrap()
            .value()
            .clone()
    };
    assert_eq!(
        values(snapshot.model()),
        SlotValue::Ordered(vec![Value::Reference(VEHICLE), Value::Reference(ENGINE)])
    );
    assert_eq!(
        values(result.model()),
        SlotValue::Ordered(vec![
            Value::Reference(VEHICLE),
            Value::Reference(ENGINE),
            Value::Reference(implied.element_id())
        ])
    );
    let proof = result.explain(prop(OWNS, SOURCES)).unwrap();
    assert!(
        proof
            .dependencies
            .contains(&Dependency::Declared(prop(OWNS, SOURCES)))
    );
    assert!(
        proof
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(implied.element_id())))
    );
    for additions in [
        vec![VEHICLE],
        vec![implied.element_id(), implied.element_id()],
    ] {
        assert!(matches!(
            build(additions),
            Err(DerivationError::InvalidCollectionExtension { .. })
        ));
    }
    let mut invalid = DerivationBuilder::new(snapshot);
    invalid.extend_ordered_references(VEHICLE, NAME, vec![ENGINE], evidence(&[]));
    assert!(matches!(
        invalid.build(),
        Err(DerivationError::InvalidCollectionExtension { .. })
    ));
}

#[test]
fn inherited_member_explanation_chain_keeps_declared_facts_separate() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    // Small illustrative queries only, not the full KerML inheritance algorithm.
    let member = snapshot
        .model()
        .element(OWNS)
        .unwrap()
        .slot(MEMBER)
        .unwrap()
        .value()
        .values()
        .next()
        .unwrap()
        .clone();
    builder.property(
        VEHICLE,
        EFFECTIVE,
        SlotValue::Set(BTreeSet::from([member.clone()])),
        evidence(&[
            Dependency::Declared(prop(OWNS, CONTAINER)),
            Dependency::Declared(prop(OWNS, MEMBER)),
        ]),
    );
    builder.property(
        SPORTS,
        EFFECTIVE,
        SlotValue::Set(BTreeSet::from([member])),
        evidence(&[
            Dependency::Derived(prop(VEHICLE, EFFECTIVE)),
            Dependency::Declared(prop(SPECIALIZES, SPECIFIC)),
            Dependency::Declared(prop(SPECIALIZES, GENERAL)),
        ]),
    );
    builder.property(
        SPORTS,
        COUNT,
        SlotValue::Scalar(Value::Integer(1.into())),
        evidence(&[Dependency::Derived(prop(SPORTS, EFFECTIVE))]),
    );
    let overlay = builder.build().unwrap();
    assert_eq!(
        overlay
            .model()
            .element(SPORTS)
            .unwrap()
            .slot(EFFECTIVE)
            .unwrap()
            .value(),
        &set_refs(&[ENGINE_USE])
    );
    assert!(matches!(
        overlay.model().element(SPORTS).unwrap().origin(),
        Origin::Declared(_)
    ));
    assert!(matches!(
        overlay
            .model()
            .element(SPORTS)
            .unwrap()
            .slot(EFFECTIVE)
            .unwrap()
            .origin(),
        Origin::Derived(_)
    ));
    assert_eq!(overlay.model().len(), snapshot.model().len());
    assert!(
        snapshot
            .model()
            .element(SPORTS)
            .unwrap()
            .slot(EFFECTIVE)
            .is_none()
    );
    let count = overlay.explain(prop(SPORTS, COUNT)).unwrap();
    assert_eq!(count.rule, RULE);
    assert!(
        count
            .dependencies
            .contains(&Dependency::Derived(prop(SPORTS, EFFECTIVE)))
    );
    assert!(
        overlay
            .explain(prop(VEHICLE, EFFECTIVE))
            .unwrap()
            .dependencies
            .contains(&Dependency::Declared(prop(OWNS, MEMBER)))
    );
    assert!(overlay.explain(prop(SPECIALIZES, GENERAL)).is_none());
    assert!(
        overlay
            .model()
            .incoming(ENGINE_USE)
            .any(|r| r.source == SPORTS && r.property == EFFECTIVE)
    );
    assert!(
        !snapshot
            .model()
            .incoming(ENGINE_USE)
            .any(|r| r.source == SPORTS)
    );
    let mut changes = snapshot.change_set();
    changes.remove(SPECIALIZES);
    let next = snapshot.apply(&changes).unwrap();
    assert_ne!(overlay.base_revision(), next.revision());
    assert!(
        next.model()
            .element(SPORTS)
            .unwrap()
            .slot(EFFECTIVE)
            .is_none()
    );
    assert!(overlay.declared().model().element(SPECIALIZES).is_some());
}

#[test]
fn transitive_specialization_is_an_ordinary_but_derived_relationship() {
    let old = vertical();
    let car = ElementId::from_u128(20);
    let specializes = ElementId::from_u128(21);
    let mut changes = old.change_set();
    changes
        .create(car, PART_DEF, authored())
        .set(car, NAME, text("TrackCar"), authored());
    changes
        .create(specializes, SPECIALIZATION, authored())
        .set(specializes, SPECIFIC, scalar_ref(car), authored())
        .set(specializes, GENERAL, scalar_ref(SPORTS), authored());
    let snapshot = old.apply(&changes).unwrap();
    let derived_key = key(car);
    let make = || {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        builder.element(
            derived_key,
            SPECIALIZATION,
            [(SPECIFIC, scalar_ref(car)), (GENERAL, scalar_ref(VEHICLE))],
            BTreeSet::from([
                Dependency::Declared(prop(specializes, SPECIFIC)),
                Dependency::Declared(prop(specializes, GENERAL)),
                Dependency::Declared(prop(SPECIALIZES, SPECIFIC)),
                Dependency::Declared(prop(SPECIALIZES, GENERAL)),
            ]),
        );
        builder.build().unwrap()
    };
    let overlay = make();
    let again = make();
    let id = derived_key.element_id();
    assert_eq!(overlay.model().element(id), again.model().element(id));
    assert!(snapshot.model().element(id).is_none());
    assert!(matches!(
        overlay.model().element(id).unwrap().origin(),
        Origin::Derived(_)
    ));
    assert!(matches!(
        overlay.model().element(specializes).unwrap().origin(),
        Origin::Declared(_)
    ));
    assert_eq!(overlay.model().outgoing(id).count(), 2);
    assert!(
        overlay
            .model()
            .instances(RELATIONSHIP, true)
            .unwrap()
            .any(|r| r.id() == id)
    );
    assert!(
        overlay
            .explain(FactKey::Element(id))
            .unwrap()
            .dependencies
            .contains(&Dependency::Declared(FactKey::Element(car)))
    );
    assert!(
        overlay
            .explain(prop(id, GENERAL))
            .unwrap()
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(id)))
    );
}

#[test]
fn derived_ids_distinguish_rule_subject_and_output_not_support_order() {
    let original = key(SPORTS);
    for different in [
        DerivationKey {
            rule: RuleId::from_u128(2),
            ..original
        },
        DerivationKey {
            subject: VEHICLE,
            ..original
        },
        DerivationKey {
            output: OutputKey::from_u128(2),
            ..original
        },
    ] {
        assert_ne!(original.element_id(), different.element_id());
    }
    assert_eq!(original.element_id(), key(SPORTS).element_id());
}

#[test]
fn derived_evidence_must_exist_in_the_correct_layer() {
    for dependency in [
        Dependency::Declared(FactKey::Element(ElementId::from_u128(99))),
        Dependency::Declared(prop(VEHICLE, EFFECTIVE)),
        Dependency::Derived(FactKey::Element(VEHICLE)),
    ] {
        let snapshot = vertical();
        let mut builder = DerivationBuilder::new(snapshot.clone());
        builder.property(SPORTS, EFFECTIVE, set_refs(&[]), evidence(&[dependency]));
        assert!(
            matches!(builder.build(),Err(DerivationError::MissingDependency { dependency:d,.. }) if d==dependency)
        );
        assert!(
            snapshot
                .model()
                .element(SPORTS)
                .unwrap()
                .slot(EFFECTIVE)
                .is_none()
        );
    }
}

#[test]
fn cyclic_explanations_and_self_support_are_rejected() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.property(
        SPORTS,
        EFFECTIVE,
        set_refs(&[]),
        evidence(&[Dependency::Derived(prop(VEHICLE, EFFECTIVE))]),
    );
    builder.property(
        VEHICLE,
        EFFECTIVE,
        set_refs(&[]),
        evidence(&[Dependency::Derived(prop(SPORTS, EFFECTIVE))]),
    );
    assert_eq!(
        builder.build().unwrap_err(),
        DerivationError::DependencyCycle(vec![prop(VEHICLE, EFFECTIVE), prop(SPORTS, EFFECTIVE)])
    );
    let mut builder = DerivationBuilder::new(snapshot);
    builder.property(
        SPORTS,
        EFFECTIVE,
        set_refs(&[]),
        evidence(&[Dependency::Derived(prop(SPORTS, EFFECTIVE))]),
    );
    assert_eq!(
        builder.build().unwrap_err(),
        DerivationError::DependencyCycle(vec![prop(SPORTS, EFFECTIVE)])
    );
}

#[test]
fn explanation_cycles_exclude_acyclic_evidence_and_dependents() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    // Edges point from a derived fact to its evidence:
    // ENGINE <-> VEHICLE -> SPORTS, with ENGINE_USE -> ENGINE upstream.
    for (element, dependencies) in [
        (ENGINE, vec![VEHICLE]),
        (VEHICLE, vec![ENGINE, SPORTS]),
        (SPORTS, vec![]),
        (ENGINE_USE, vec![ENGINE]),
    ] {
        builder.property(
            element,
            EFFECTIVE,
            set_refs(&[]),
            evidence(
                &dependencies
                    .into_iter()
                    .map(|id| Dependency::Derived(prop(id, EFFECTIVE)))
                    .collect::<Vec<_>>(),
            ),
        );
    }
    assert_eq!(
        builder.build().unwrap_err(),
        DerivationError::DependencyCycle(vec![prop(ENGINE, EFFECTIVE), prop(VEHICLE, EFFECTIVE)])
    );
    for element in [ENGINE, VEHICLE, ENGINE_USE, SPORTS] {
        assert!(
            snapshot
                .model()
                .element(element)
                .unwrap()
                .slot(EFFECTIVE)
                .is_none()
        );
    }
}

#[test]
fn overlays_cannot_overwrite_declared_or_conflicting_facts() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.property(SPORTS, NAME, text("no"), evidence(&[]));
    assert!(matches!(
        builder.build(),
        Err(DerivationError::NotDerivedProperty { .. })
    ));
    let mut builder = DerivationBuilder::new(snapshot);
    builder
        .property(SPORTS, EFFECTIVE, set_refs(&[]), evidence(&[]))
        .property(SPORTS, EFFECTIVE, set_refs(&[ENGINE_USE]), evidence(&[]));
    assert!(matches!(
        builder.build(),
        Err(DerivationError::DuplicateFact(_))
    ));
}

#[test]
fn implied_identity_cannot_collide_with_declared_or_retired_identity() {
    let snapshot = vertical();
    let derived_key = key(SPORTS);
    let id = derived_key.element_id();
    let mut changes = snapshot.change_set();
    changes.create(id, TYPE, authored());
    let occupied = snapshot.apply(&changes).unwrap();
    let mut changes = occupied.change_set();
    changes.remove(id);
    let retired = occupied.apply(&changes).unwrap();
    for snapshot in [occupied, retired] {
        let mut builder = DerivationBuilder::new(snapshot);
        builder.element(derived_key, TYPE, [], BTreeSet::new());
        assert_eq!(
            builder.build().unwrap_err(),
            DerivationError::IdentityCollision(id)
        );
    }
}

#[test]
fn derived_records_and_slots_receive_the_same_structural_validation() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.element(
        key(SPORTS),
        SPECIALIZATION,
        [(SPECIFIC, scalar_ref(SPORTS))],
        BTreeSet::new(),
    );
    assert!(matches!(
        builder.build(),
        Err(DerivationError::Model(ModelError::Multiplicity {
            property: GENERAL,
            ..
        }))
    ));
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.property(
        SPORTS,
        EFFECTIVE,
        set_refs(&[ElementId::from_u128(99)]),
        evidence(&[]),
    );
    assert!(matches!(
        builder.build(),
        Err(DerivationError::Model(ModelError::DanglingReference { .. }))
    ));
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.property(SPORTS, EFFECTIVE, set_refs(&[ENGINE]), evidence(&[]));
    assert!(matches!(
        builder.build(),
        Err(DerivationError::Model(ModelError::ReferenceType { .. }))
    ));
    let mut builder = DerivationBuilder::new(snapshot);
    builder.property(
        SPORTS,
        COUNT,
        SlotValue::Set(BTreeSet::new()),
        evidence(&[]),
    );
    assert!(matches!(
        builder.build(),
        Err(DerivationError::Model(ModelError::Shape { .. }))
    ));
}

#[test]
fn forward_derived_dependencies_and_iteration_are_deterministic() {
    let snapshot = vertical();
    let make = |reverse: bool| {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        let mut facts = vec![
            (
                SPORTS,
                evidence(&[Dependency::Derived(prop(VEHICLE, EFFECTIVE))]),
            ),
            (
                VEHICLE,
                evidence(&[Dependency::Declared(FactKey::Element(OWNS))]),
            ),
        ];
        if reverse {
            facts.reverse();
        }
        for (id, evidence) in facts {
            builder.property(id, EFFECTIVE, set_refs(&[ENGINE_USE]), evidence);
        }
        builder.build().unwrap()
    };
    let a = make(false);
    let b = make(true);
    assert_eq!(a.facts().collect::<Vec<_>>(), b.facts().collect::<Vec<_>>());
    assert_eq!(
        a.model().elements().collect::<Vec<_>>(),
        b.model().elements().collect::<Vec<_>>()
    );
    assert_eq!(
        a.model().incoming(ENGINE_USE).collect::<Vec<_>>(),
        b.model().incoming(ENGINE_USE).collect::<Vec<_>>()
    );
    fn send_sync<T: Send + Sync>() {}
    send_sync::<DerivedOverlay>();
}
