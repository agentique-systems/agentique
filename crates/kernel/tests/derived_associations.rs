mod common;
use agq_kernel::{derived::*, metamodel::*, provenance::*, value::*, *};
use common::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const A: AssociationId = AssociationId::from_u128(400);
const LEFT: PropertyId = PropertyId::from_u128(401);
const RIGHT: PropertyId = PropertyId::from_u128(402);

fn descriptors(ordered: bool, derived: bool, upper: Option<usize>) -> DescriptorSet {
    let mut properties = vec![];
    for (id, opposite, name) in [(LEFT, RIGHT, "left"), (RIGHT, LEFT, "right")] {
        let mut p = property(
            id,
            name,
            ELEMENT,
            ValueKind::Reference(ELEMENT),
            Multiplicity::MANY,
        );
        p.owner = PropertyOwner::Association(A);
        p.association = Some(A);
        p.opposite_ends.insert(opposite);
        p.ordered = ordered;
        p.derived = derived && id == RIGHT;
        p.multiplicity.upper = upper;
        properties.push(p);
    }
    DescriptorSet {
        models: vec![model_descriptor()],
        classes: vec![
            class(ELEMENT, "Element", &[]),
            class(TYPE, "Unrelated", &[]),
        ],
        associations: vec![AssociationDescriptor {
            id: A,
            name: "Relation".into(),
            package: vec![],
            metamodel: MM,
            member_ends: vec![LEFT, RIGHT],
            navigable_owned_ends: BTreeSet::from([LEFT, RIGHT]),
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
        }],
        properties,
        ..Default::default()
    }
}
fn snapshot(ordered: bool, derived: bool, upper: Option<usize>) -> Snapshot {
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors(ordered, derived, upper)).unwrap(),
    ));
    let mut edit = base.change_set();
    for id in [ENGINE, VEHICLE, ENGINE_USE] {
        edit.create(id, ELEMENT, authored());
    }
    base.apply(&edit).unwrap()
}
fn key(role: u128) -> DerivationKey {
    DerivationKey {
        rule: RuleId::from_u128(100),
        subject: ENGINE,
        output: OutputKey::from_u128(role),
    }
}
fn ends(a: ElementId, b: ElementId) -> BTreeMap<PropertyId, ElementId> {
    BTreeMap::from([(LEFT, a), (RIGHT, b)])
}

#[test]
fn identity_uses_rule_subject_role_association_and_oriented_endpoints() {
    let participants = ends(ENGINE, VEHICLE);
    let id = key(1).association_occurrence_id(A, &participants);
    assert_eq!(
        id,
        key(1).association_occurrence_id(
            A,
            &participants.iter().rev().map(|(a, b)| (*a, *b)).collect()
        )
    );
    for changed in [
        key(2),
        DerivationKey {
            subject: VEHICLE,
            ..key(1)
        },
        DerivationKey {
            rule: RuleId::from_u128(101),
            ..key(1)
        },
    ] {
        assert_ne!(id, changed.association_occurrence_id(A, &participants));
    }
    assert_ne!(
        id,
        key(1).association_occurrence_id(AssociationId::from_u128(401), &participants)
    );
    assert_ne!(
        id,
        key(1).association_occurrence_id(A, &ends(VEHICLE, ENGINE))
    );
}

#[test]
fn canonical_navigation_is_additive_indexed_and_explainable_in_both_directions() {
    let base = snapshot(true, false, None);
    let declared_id = AssociationOccurrenceId::from_u128(10);
    let mut edit = base.change_set();
    edit.link(
        declared_id,
        A,
        ends(ENGINE, VEHICLE),
        BTreeMap::from([(LEFT, 0), (RIGHT, 0)]),
        authored(),
    );
    let base = base.apply(&edit).unwrap();
    let original = base
        .model()
        .association_occurrence(declared_id)
        .unwrap()
        .clone();
    let implied = key(3).element_id();
    let participants = ends(implied, VEHICLE);
    let id = key(1).association_occurrence_id(A, &participants);
    let search = StructuralSearch::Association {
        element: VEHICLE,
        association: A,
    };
    let mut builder = DerivationBuilder::new(base.clone());
    // Forward dependencies do not depend on insertion order.
    builder.association_occurrence(
        key(1),
        A,
        participants,
        BTreeMap::from([(LEFT, 1), (RIGHT, 0)]),
        BTreeSet::from([Dependency::Declared(FactKey::AssociationOccurrence(
            declared_id,
        ))]),
    );
    builder.element(key(3), ELEMENT, [], BTreeSet::new());
    builder.searches(
        FactKey::AssociationOccurrence(id),
        BTreeSet::from([search.clone()]),
    );
    let overlay = builder.build().unwrap();
    let model = overlay.model();
    assert_eq!(base.model().association_occurrences().count(), 1);
    assert_eq!(model.association_occurrences().count(), 2);
    assert_eq!(model.association_occurrence(declared_id), Some(&original));
    let link = model.association_occurrence(id).unwrap();
    assert!(link.declared_origin().is_none());
    let explanation = overlay.explain(FactKey::AssociationOccurrence(id)).unwrap();
    assert_eq!(link.origin(), &Origin::Derived(explanation.clone().into()));
    assert!(
        explanation
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(implied)))
    );
    assert!(explanation.dependencies.contains(&Dependency::Declared(
        FactKey::AssociationOccurrence(declared_id)
    )));
    assert_eq!(
        model
            .computation_searches_for(FactKey::AssociationOccurrence(id))
            .cloned()
            .collect::<Vec<_>>(),
        vec![search]
    );
    let left = model.navigation_slot(VEHICLE, LEFT).unwrap();
    assert_eq!(
        left.value(),
        &SlotValue::Ordered(vec![Value::Reference(ENGINE), Value::Reference(implied)])
    );
    assert_eq!(
        left.origin(),
        &Origin::AssociationOccurrences(BTreeSet::from([declared_id, id]))
    );
    assert_eq!(
        model.navigation_slot(implied, RIGHT).unwrap().value(),
        &SlotValue::Ordered(vec![Value::Reference(VEHICLE)])
    );
    assert!(model.element(implied).unwrap().slot(RIGHT).is_none());
    assert_eq!(model.incident_associations(VEHICLE).count(), 2);
    assert!(
        model
            .incoming(implied)
            .any(|r| r.carrier == ReferenceCarrier::AssociationOccurrence(id))
    );
    assert!(
        model
            .outgoing(implied)
            .any(|r| r.carrier == ReferenceCarrier::AssociationOccurrence(id))
    );
    assert_eq!(model.derived_navigation_results().count(), 0);
}

#[test]
fn derived_opposite_is_computed_from_the_occurrence_without_a_synthetic_slot() {
    let base = snapshot(false, true, None);
    let participants = ends(ENGINE, VEHICLE);
    let mut builder = DerivationBuilder::new(base.clone());
    builder.association_occurrence(key(1), A, participants, BTreeMap::new(), BTreeSet::new());
    let overlay = builder.build().unwrap();
    assert!(base.model().navigation_slot(ENGINE, RIGHT).is_none());
    assert!(matches!(
        overlay.model().property_state(ENGINE, RIGHT).unwrap(),
        PropertyState::Computed(_)
    ));
    assert!(matches!(
        overlay
            .model()
            .navigation_slot(ENGINE, RIGHT)
            .unwrap()
            .origin(),
        Origin::AssociationOccurrences(_)
    ));
    assert_eq!(overlay.model().derived_navigation_results().count(), 0);
}

#[test]
fn derived_only_associations_require_derived_provenance_and_one_storage_surface() {
    let mut descriptors = descriptors(false, true, None);
    for property in &mut descriptors.properties {
        property.derived = true;
        property.owner = PropertyOwner::Class(ELEMENT);
    }
    descriptors.associations[0].navigable_owned_ends.clear();
    let registry = Arc::new(MetamodelRegistry::from_descriptors(descriptors).unwrap());
    assert!(!registry.supports_occurrence_storage(A).unwrap());
    assert!(registry.supports_derived_occurrence_storage(A).unwrap());
    let base = Snapshot::new(registry);
    let mut edit = base.change_set();
    for id in [ENGINE, VEHICLE] {
        edit.create(id, ELEMENT, authored());
    }
    let base = base.apply(&edit).unwrap();
    let participants = ends(ENGINE, VEHICLE);
    let mut authored_link = base.change_set();
    authored_link.link(
        AssociationOccurrenceId::from_u128(900),
        A,
        participants.clone(),
        BTreeMap::new(),
        authored(),
    );
    assert!(base.apply(&authored_link).is_err());
    for duplicate_slot in [false, true] {
        let mut builder = DerivationBuilder::new(base.clone());
        builder.association_occurrence(
            key(1),
            A,
            participants.clone(),
            BTreeMap::new(),
            BTreeSet::new(),
        );
        if duplicate_slot {
            builder.property(
                ENGINE,
                RIGHT,
                SlotValue::Set(BTreeSet::from([Value::Reference(VEHICLE)])),
                Explanation {
                    rule: key(1).rule,
                    dependencies: BTreeSet::new(),
                },
            );
            assert!(matches!(
                builder.build(),
                Err(DerivationError::Model(
                    ModelError::UnsupportedAssociationStorage(RIGHT)
                ))
            ));
        } else {
            let overlay = builder.build().unwrap();
            for (element, property) in [(VEHICLE, LEFT), (ENGINE, RIGHT)] {
                assert!(matches!(
                    overlay
                        .model()
                        .navigation_slot(element, property)
                        .unwrap()
                        .origin(),
                    Origin::AssociationOccurrences(_)
                ));
                assert!(
                    overlay
                        .model()
                        .element(element)
                        .unwrap()
                        .slot(property)
                        .is_none()
                );
            }
        }
    }
}

#[test]
fn duplicates_collisions_and_retired_declared_identities_fail_atomically() {
    let base = snapshot(false, false, None);
    let participants = ends(ENGINE, VEHICLE);
    let id = key(1).association_occurrence_id(A, &participants);
    let mut builder = DerivationBuilder::new(base.clone());
    for _ in 0..2 {
        builder.association_occurrence(
            key(1),
            A,
            participants.clone(),
            BTreeMap::new(),
            BTreeSet::new(),
        );
    }
    assert!(
        matches!(builder.build(), Err(DerivationError::DuplicateFact(FactKey::AssociationOccurrence(i))) if i == id)
    );
    let mut edit = base.change_set();
    edit.link(id, A, participants.clone(), BTreeMap::new(), authored());
    let declared = base.apply(&edit).unwrap();
    let mut edit = declared.change_set();
    edit.unlink(id);
    let retired = declared.apply(&edit).unwrap();
    for source in [declared, retired] {
        let mut builder = DerivationBuilder::new(source);
        builder.association_occurrence(
            key(1),
            A,
            participants.clone(),
            BTreeMap::new(),
            BTreeSet::new(),
        );
        assert!(
            matches!(builder.build(), Err(DerivationError::AssociationIdentityCollision(i)) if i == id)
        );
    }
    assert_eq!(base.model().association_occurrences().count(), 0);
}

#[test]
fn ordinary_incidence_order_uniqueness_and_multiplicity_checks_apply() {
    for (ordered, upper, duplicate, positions) in [
        (false, None, true, BTreeMap::new()),
        (false, Some(1), false, BTreeMap::new()),
        (true, None, false, BTreeMap::new()),
        (true, None, false, BTreeMap::from([(LEFT, 2), (RIGHT, 0)])),
    ] {
        let base = snapshot(ordered, false, upper);
        let mut builder = DerivationBuilder::new(base.clone());
        builder.association_occurrence(
            key(1),
            A,
            ends(ENGINE, VEHICLE),
            positions.clone(),
            BTreeSet::new(),
        );
        builder.association_occurrence(
            key(2),
            A,
            ends(if duplicate { ENGINE } else { ENGINE_USE }, VEHICLE),
            positions,
            BTreeSet::new(),
        );
        assert!(matches!(builder.build(), Err(DerivationError::Model(_))));
        assert_eq!(base.model().association_occurrences().count(), 0);
    }
    let base = snapshot(false, false, None);
    for participants in [
        BTreeMap::from([(LEFT, ENGINE)]),
        ends(ENGINE, ElementId::from_u128(999)),
    ] {
        let mut builder = DerivationBuilder::new(base.clone());
        builder.association_occurrence(key(1), A, participants, BTreeMap::new(), BTreeSet::new());
        assert!(matches!(builder.build(), Err(DerivationError::Model(_))));
    }
    let mut edit = base.change_set();
    edit.create(ElementId::from_u128(999), TYPE, authored());
    let base = base.apply(&edit).unwrap();
    let mut builder = DerivationBuilder::new(base);
    builder.association_occurrence(
        key(1),
        A,
        ends(ENGINE, ElementId::from_u128(999)),
        BTreeMap::new(),
        BTreeSet::new(),
    );
    assert!(matches!(builder.build(), Err(DerivationError::Model(_))));
}

#[test]
fn missing_and_cyclic_occurrence_evidence_is_rejected() {
    let base = snapshot(false, false, None);
    let participants = ends(ENGINE, VEHICLE);
    let id = key(1).association_occurrence_id(A, &participants);
    for dependency in [
        Dependency::Derived(FactKey::AssociationOccurrence(id)),
        Dependency::Declared(FactKey::AssociationOccurrence(id)),
    ] {
        let mut builder = DerivationBuilder::new(base.clone());
        builder.association_occurrence(
            key(1),
            A,
            participants.clone(),
            BTreeMap::new(),
            BTreeSet::from([dependency]),
        );
        match dependency {
            Dependency::Derived(_) => assert!(matches!(
                builder.build(),
                Err(DerivationError::DependencyCycle(_))
            )),
            Dependency::Declared(_) => assert!(matches!(
                builder.build(),
                Err(DerivationError::MissingDependency { .. })
            )),
        }
    }
    let implied = key(3).element_id();
    let participants = ends(implied, VEHICLE);
    let id = key(1).association_occurrence_id(A, &participants);
    let mut builder = DerivationBuilder::new(base);
    builder.element(
        key(3),
        ELEMENT,
        [],
        BTreeSet::from([Dependency::Derived(FactKey::AssociationOccurrence(id))]),
    );
    builder.association_occurrence(key(1), A, participants, BTreeMap::new(), BTreeSet::new());
    assert!(matches!(
        builder.build(),
        Err(DerivationError::DependencyCycle(_))
    ));
}

#[test]
fn later_stages_retain_occurrence_evidence_and_reject_replacement_atomically() {
    let base = snapshot(false, false, None);
    let mut first = DerivationBuilder::new(base.clone());
    let participants = ends(ENGINE, VEHICLE);
    let a = key(1).association_occurrence_id(A, &participants);
    first.association_occurrence(
        key(1),
        A,
        participants.clone(),
        BTreeMap::new(),
        BTreeSet::new(),
    );
    let first = first.build().unwrap();
    let search = StructuralSearch::Incoming(VEHICLE);
    let mut second = DerivationBuilder::from_overlay(first.clone());
    let other = ends(ENGINE_USE, VEHICLE);
    let b = key(2).association_occurrence_id(A, &other);
    second.association_occurrence(
        key(2),
        A,
        other,
        BTreeMap::new(),
        BTreeSet::from([Dependency::Derived(FactKey::AssociationOccurrence(a))]),
    );
    second.searches(
        FactKey::AssociationOccurrence(b),
        BTreeSet::from([search.clone()]),
    );
    let second = second.build().unwrap();
    assert_eq!(second.declared().revision(), base.revision());
    assert_eq!(
        second.explain(FactKey::AssociationOccurrence(a)),
        first.explain(FactKey::AssociationOccurrence(a))
    );
    assert!(
        second
            .explain(FactKey::AssociationOccurrence(b))
            .unwrap()
            .dependencies
            .contains(&Dependency::Derived(FactKey::AssociationOccurrence(a)))
    );
    assert_eq!(
        second
            .model()
            .computation_searches_for(FactKey::AssociationOccurrence(b))
            .cloned()
            .collect::<Vec<_>>(),
        vec![search]
    );
    assert_eq!(first.model().association_occurrences().count(), 1);
    assert_eq!(second.model().association_occurrences().count(), 2);
    assert_eq!(base.model().association_occurrences().count(), 0);
    let mut conflict = DerivationBuilder::from_overlay(second.clone());
    conflict.association_occurrence(key(1), A, participants, BTreeMap::new(), BTreeSet::new());
    assert!(
        matches!(conflict.build(), Err(DerivationError::DuplicateFact(FactKey::AssociationOccurrence(id))) if id == a)
    );
    assert_eq!(second.model().association_occurrences().count(), 2);
}

#[test]
fn authored_consumers_cannot_unlink_or_reorder_dependency_occurrences() {
    let base = snapshot(true, false, None);
    let participants = ends(ENGINE, VEHICLE);
    let id = key(1).association_occurrence_id(A, &participants);
    let mut builder = DerivationBuilder::new(base);
    builder.association_occurrence(
        key(1),
        A,
        participants,
        BTreeMap::from([(LEFT, 0), (RIGHT, 0)]),
        BTreeSet::new(),
    );
    let dependency = Arc::new(builder.build().unwrap());
    let project = Snapshot::with_immutable_dependency(dependency.clone());
    for reorder in [false, true] {
        let mut changes = project.change_set();
        if reorder {
            changes.reorder_link(id, BTreeMap::from([(LEFT, 1), (RIGHT, 0)]), authored());
        } else {
            changes.unlink(id);
        }
        assert!(
            matches!(project.apply(&changes),Err(ModelError::ImmutableDependency(FactKey::AssociationOccurrence(link))) if link==id)
        );
    }
    let mut changes = project.change_set();
    changes.create(ElementId::from_u128(800), ELEMENT, authored());
    let next = project.apply(&changes).unwrap();
    assert_eq!(
        next.model().association_occurrence(id),
        dependency.model().association_occurrence(id)
    );
    let derived = DerivationBuilder::new(next).build().unwrap();
    assert_eq!(
        derived.explain(FactKey::AssociationOccurrence(id)),
        dependency.explain(FactKey::AssociationOccurrence(id))
    );
}
