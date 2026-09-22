mod common;
use agq_kernel::{derived::*, provenance::*, value::*, *};
use common::*;
use std::{collections::BTreeSet, sync::Arc};

fn key(subject: ElementId, output: u128) -> DerivationKey {
    DerivationKey {
        rule: RuleId::from_u128(801),
        subject,
        output: OutputKey::from_u128(output),
    }
}
fn proof(dependencies: impl IntoIterator<Item = Dependency>) -> Explanation {
    Explanation {
        rule: RuleId::from_u128(801),
        dependencies: dependencies.into_iter().collect(),
    }
}
fn incomplete() -> Arc<ConstructionView> {
    let base = Snapshot::new(registry());
    let mut changes = base.change_set();
    changes
        .create(ENGINE, TYPE, authored())
        .create(SPORTS, TYPE, authored())
        .create(SPECIALIZES, SPECIALIZATION, authored())
        .set(SPECIALIZES, SPECIFIC, scalar_ref(SPORTS), authored())
        .set(SPECIALIZES, SOURCES, ordered_refs(&[SPORTS]), authored());
    assert!(matches!(
        base.apply(&changes),
        Err(ModelError::Multiplicity { .. })
    ));
    Arc::new(base.preview(&changes).unwrap())
}
#[test]
fn unpublished_derivation_preserves_obligations_and_original_declared_evidence() {
    let declared = incomplete();
    let output = key(ENGINE, 1);
    let mut builder = ConstructionDerivationBuilder::for_construction(declared.clone());
    builder.element(output, FEATURE, [(NAME, text("inferred"))], BTreeSet::new());
    builder.property(
        SPORTS,
        EFFECTIVE,
        set_refs(&[output.element_id()]),
        proof([]),
    );
    builder.extend_ordered_references(SPECIALIZES, SOURCES, vec![output.element_id()], proof([]));
    let first = builder.build().unwrap();
    assert_eq!(first.obligations(), declared.obligations());
    assert_eq!(first.obligations().len(), 1);
    assert_eq!(first.declared().model().len(), 3);
    assert_eq!(first.model().len(), 4);
    assert!(Arc::ptr_eq(first.declared_shared(), &declared));
    assert_eq!(first.base_revision(), declared.revision());
    let original = FactKey::Property {
        element: SPECIALIZES,
        property: SOURCES,
    };
    assert!(first.model().declared_fact_origin(original).is_some());
    assert!(
        first
            .explain(original)
            .unwrap()
            .dependencies
            .contains(&Dependency::Declared(original))
    );
    assert_eq!(
        declared
            .model()
            .navigation_slot(SPECIALIZES, SOURCES)
            .unwrap()
            .value(),
        &ordered_refs(&[SPORTS])
    );
    assert_eq!(
        first
            .model()
            .navigation_slot(SPECIALIZES, SOURCES)
            .unwrap()
            .value(),
        &ordered_refs(&[SPORTS, output.element_id()])
    );
    let mut next = ConstructionDerivationBuilder::for_construction(first.declared_shared().clone());
    next.extend_ordered_references(SPECIALIZES, SOURCES, vec![ENGINE], proof([]));
    let next = next.build_on_construction_overlay(first).unwrap();
    assert_eq!(next.obligations(), declared.obligations());
    assert!(next.build_metrics().reused_owned_storage);
    assert!(next.model().declared_fact_origin(original).is_some());
    assert_eq!(
        next.model()
            .navigation_slot(SPECIALIZES, SOURCES)
            .unwrap()
            .value(),
        &ordered_refs(&[SPORTS, output.element_id(), ENGINE])
    );
}
#[test]
fn missing_derived_record_bounds_remain_explicit_and_cannot_become_snapshot() {
    let declared = incomplete();
    let mut builder = ConstructionDerivationBuilder::for_construction(declared);
    let relation = key(ENGINE, 2);
    builder.element(relation, SPECIALIZATION, [], BTreeSet::new());
    let result = builder.build().unwrap();
    assert_eq!(result.obligations().len(), 3);
    assert_eq!(
        result
            .obligations()
            .iter()
            .filter(|o| o.element == relation.element_id())
            .count(),
        2
    );
    assert!(
        result
            .model()
            .navigation_slot(relation.element_id(), GENERAL)
            .is_none()
    );
}
#[test]
fn absent_premises_cycles_invalid_endpoints_and_upper_bounds_are_rejected() {
    let declared = incomplete();
    let mut absent = ConstructionDerivationBuilder::for_construction(declared.clone());
    absent.element(
        key(ENGINE, 1),
        TYPE,
        [],
        BTreeSet::from([Dependency::Declared(FactKey::Property {
            element: SPECIALIZES,
            property: GENERAL,
        })]),
    );
    assert!(matches!(
        absent.build(),
        Err(DerivationError::MissingDependency { .. })
    ));
    let a = key(ENGINE, 1);
    let b = key(ENGINE, 2);
    let mut cycle = ConstructionDerivationBuilder::for_construction(declared.clone());
    cycle.element(
        a,
        TYPE,
        [],
        BTreeSet::from([Dependency::Derived(FactKey::Element(b.element_id()))]),
    );
    cycle.element(
        b,
        TYPE,
        [],
        BTreeSet::from([Dependency::Derived(FactKey::Element(a.element_id()))]),
    );
    assert!(matches!(
        cycle.build(),
        Err(DerivationError::DependencyCycle(_))
    ));
    let mut endpoint = ConstructionDerivationBuilder::for_construction(declared.clone());
    endpoint.element(
        a,
        SPECIALIZATION,
        [
            (SPECIFIC, scalar_ref(ENGINE)),
            (GENERAL, scalar_ref(ElementId::from_u128(999))),
        ],
        BTreeSet::new(),
    );
    assert!(matches!(
        endpoint.build(),
        Err(DerivationError::Model(ModelError::DanglingReference { .. }))
    ));
    let mut wrong = ConstructionDerivationBuilder::for_construction(declared.clone());
    wrong.element(
        a,
        SPECIALIZATION,
        [
            (SPECIFIC, scalar_ref(ENGINE)),
            (GENERAL, scalar_ref(SPECIALIZES)),
        ],
        BTreeSet::new(),
    );
    assert!(matches!(
        wrong.build(),
        Err(DerivationError::Model(ModelError::ReferenceType { .. }))
    ));
    let mut upper = ConstructionDerivationBuilder::for_construction(declared);
    upper.element(
        a,
        SPECIALIZATION,
        [(
            TARGETS,
            SlotValue::Ordered(vec![Value::Reference(ENGINE); 4]),
        )],
        BTreeSet::new(),
    );
    assert!(matches!(
        upper.build(),
        Err(DerivationError::Model(ModelError::Multiplicity {
            actual: 4,
            ..
        }))
    ));
}
#[test]
fn construction_frontiers_reject_mismatched_inputs() {
    let input = incomplete();
    let other = incomplete();
    let overlay = ConstructionDerivationBuilder::for_construction(input.clone())
        .build()
        .unwrap();
    let builder = ConstructionDerivationBuilder::for_construction(other);
    assert!(matches!(
        builder.build_on_construction_overlay(overlay.clone()),
        Err(DerivationError::InputContextMismatch)
    ));
    let builder = ConstructionDerivationBuilder::from_construction_overlay(overlay.clone());
    assert!(matches!(
        builder.build_on_construction_overlay(overlay),
        Err(DerivationError::InputContextMismatch)
    ));
    assert_eq!(input.obligations().len(), 1);
}
#[test]
fn construction_retains_staged_retired_ids_for_derivation_collision_checks() {
    let base = Snapshot::new(registry());
    let mut changes = base.change_set();
    let retired = key(ENGINE, 1);
    changes
        .create(ENGINE, TYPE, authored())
        .create(retired.element_id(), TYPE, authored())
        .remove(retired.element_id());
    let candidate = Arc::new(base.preview(&changes).unwrap());
    assert!(candidate.obligations().is_empty());
    let mut builder = ConstructionDerivationBuilder::for_construction(candidate);
    builder.element(retired, TYPE, [], BTreeSet::new());
    assert!(
        matches!(builder.build(),Err(DerivationError::IdentityCollision(id)) if id==retired.element_id())
    );
}
#[test]
fn construction_derivation_cannot_mutate_dependency_slots_or_ownership() {
    let dependency = Arc::new(DerivationBuilder::new(vertical()).build().unwrap());
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let local = ElementId::from_u128(999);
    let relation = ElementId::from_u128(998);
    let mut changes = base.change_set();
    changes
        .create(local, TYPE, authored())
        .create(relation, SPECIALIZATION, authored())
        .set(relation, SPECIFIC, scalar_ref(local), authored());
    let input = Arc::new(base.preview(&changes).unwrap());
    let mut slot = ConstructionDerivationBuilder::for_construction(input.clone());
    slot.property(
        ENGINE,
        COUNT,
        SlotValue::Scalar(Value::Integer(1.into())),
        proof([]),
    );
    assert!(matches!(
        slot.build(),
        Err(DerivationError::Model(ModelError::ImmutableDependency(_)))
    ));
    let mut owner = ConstructionDerivationBuilder::for_construction(input.clone());
    owner.element(
        key(local, 1),
        TYPE,
        [(CONTAINS, set_refs(&[ENGINE]))],
        BTreeSet::new(),
    );
    assert!(matches!(
        owner.build(),
        Err(DerivationError::Model(ModelError::ImmutableDependency(_)))
    ));
    let result = ConstructionDerivationBuilder::for_construction(input)
        .build()
        .unwrap();
    assert_eq!(result.obligations().len(), 1);
    assert!(Arc::ptr_eq(
        result.declared().immutable_dependency().unwrap(),
        &dependency
    ));
    for element in dependency.model().elements() {
        assert!(std::ptr::eq(
            element,
            result.model().element(element.id()).unwrap()
        ));
    }
}
