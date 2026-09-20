mod common;
use agq_kernel::derived::*;
use agq_kernel::provenance::*;
use agq_kernel::value::*;
use agq_kernel::*;
use common::*;
use std::collections::BTreeSet;

const RULE: RuleId = RuleId::from_u128(1);
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
