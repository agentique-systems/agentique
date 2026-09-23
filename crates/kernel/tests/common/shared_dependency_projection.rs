//! Phase-I semantic acceptance oracle for the planned shared-table refactor.
//!
//! These execute on today's public kernel API and deliberately reuse this test
//! binary's neutral descriptors. They do NOT establish allocation sharing.
//! HELD: once the private storage observer exists, additionally assert shared
//! base record/occurrence/index/proof tables, zero copied dependency entries,
//! and sparse project-only replacements for the touched reference groups.
use super::*;
use agq_kernel::archive;
use std::io::Cursor;

const LOCAL_EARLY: AssociationOccurrenceId = AssociationOccurrenceId::from_u128(1);
const LOCAL_DUPLICATE: AssociationOccurrenceId = AssociationOccurrenceId::from_u128(2);

fn projection_registry(scalar_inverse: bool) -> Arc<MetamodelRegistry> {
    let mut set = descriptors(false, false, None);
    for property in &mut set.properties {
        property.unique = false;
        if scalar_inverse && property.id == LEFT {
            property.multiplicity.upper = Some(1);
        }
    }
    Arc::new(MetamodelRegistry::from_descriptors(set).unwrap())
}

fn base_link() -> AssociationOccurrenceId {
    key(1000).association_occurrence_id(A, &ends(ENGINE, VEHICLE))
}

/// Construct from input data, never flatten by reading the mounted model.
fn independent_flattened(
    registry: Arc<MetamodelRegistry>,
    local_links: &[(AssociationOccurrenceId, ElementId, ElementId)],
) -> Result<DerivedOverlay, DerivationError> {
    let empty = Snapshot::new(registry);
    let mut edit = empty.change_set();
    for id in [ENGINE, VEHICLE, ENGINE_USE] {
        edit.create(id, ELEMENT, authored());
    }
    if !local_links.is_empty() {
        edit.create(SPORTS, ELEMENT, authored());
    }
    for &(link, left, right) in local_links {
        edit.link(link, A, ends(left, right), BTreeMap::new(), authored());
    }
    let mut builder = DerivationBuilder::new(empty.apply(&edit)?);
    builder.association_occurrence(
        key(1000),
        A,
        ends(ENGINE, VEHICLE),
        BTreeMap::new(),
        BTreeSet::from([Dependency::Declared(FactKey::Element(ENGINE))]),
    );
    builder.searches(
        FactKey::AssociationOccurrence(base_link()),
        BTreeSet::from([StructuralSearch::Association {
            element: VEHICLE,
            association: A,
        }]),
    );
    builder.build()
}

fn assert_projection_equivalent(actual: &DerivedOverlay, expected: &DerivedOverlay) {
    let a = actual.model();
    let b = expected.model();
    assert!(a.elements().eq(b.elements()));
    assert!(a.association_occurrences().eq(b.association_occurrences()));
    assert!(
        a.derived_navigation_results()
            .eq(b.derived_navigation_results())
    );
    assert!(a.computation_failures().eq(b.computation_failures()));
    assert!(a.computation_searches().eq(b.computation_searches()));
    assert!(actual.facts().eq(expected.facts()));
    for include_subtypes in [false, true] {
        for class in [ELEMENT, TYPE] {
            assert!(
                a.instances(class, include_subtypes)
                    .unwrap()
                    .eq(b.instances(class, include_subtypes).unwrap())
            );
        }
        assert!(
            a.association_instances(A, include_subtypes)
                .unwrap()
                .eq(b.association_instances(A, include_subtypes).unwrap())
        );
    }
    for element in [ENGINE, VEHICLE, ENGINE_USE, SPORTS] {
        assert_eq!(a.element(element), b.element(element));
        assert!(a.incoming(element).eq(b.incoming(element)));
        assert!(a.outgoing(element).eq(b.outgoing(element)));
        assert!(
            a.incident_associations(element)
                .eq(b.incident_associations(element))
        );
        for property in [LEFT, RIGHT] {
            assert_eq!(
                a.navigation_slot(element, property),
                b.navigation_slot(element, property)
            );
            assert_eq!(
                a.declared_slot(element, property),
                b.declared_slot(element, property)
            );
            assert!(
                a.incoming_for_property(element, property)
                    .eq(b.incoming_for_property(element, property))
            );
            if a.element(element).is_some() {
                assert_eq!(
                    a.property_state(element, property).unwrap(),
                    b.property_state(element, property).unwrap()
                );
            }
        }
    }
    assert_incoming_property_index(a);
    assert_incoming_property_index(b);
}

#[test]
fn shared_storage_oracle_local_occurrences_reindex_touched_groups_and_removal_restores_base() {
    let registry = projection_registry(false);
    let dependency = Arc::new(independent_flattened(registry.clone(), &[]).unwrap());
    let independent_base = independent_flattened(registry.clone(), &[]).unwrap();
    let mount = Snapshot::with_immutable_dependency(dependency.clone());
    #[cfg(feature = "verification")]
    {
        let storage = agq_kernel::storage_observer::snapshot_storage(&mount);
        assert_eq!(
            storage.base_tables,
            agq_kernel::storage_observer::publication_storage(&dependency)
        );
        assert!(storage.copied_dependency_entries.is_zero(), "{storage:?}");
    }
    let local_links = [
        (LOCAL_EARLY, ENGINE, SPORTS),
        (LOCAL_DUPLICATE, ENGINE, VEHICLE),
    ];
    assert!(LOCAL_DUPLICATE < base_link());
    let mut additions = mount.change_set();
    additions.create(SPORTS, ELEMENT, authored());
    // Reverse insertion order must not become merged query order.
    for &(link, left, right) in local_links.iter().rev() {
        additions.link(link, A, ends(left, right), BTreeMap::new(), authored());
    }
    let revision = mount.apply(&additions).unwrap();
    let combined = DerivationBuilder::new(revision.clone()).build().unwrap();
    let oracle = independent_flattened(registry.clone(), &local_links).unwrap();
    assert!(oracle.declared().immutable_dependency().is_none());
    assert_projection_equivalent(&combined, &oracle);
    #[cfg(feature = "verification")]
    {
        let storage = agq_kernel::storage_observer::overlay_storage(&combined);
        assert_eq!(
            storage.base_tables,
            agq_kernel::storage_observer::publication_storage(&dependency)
        );
        assert!(storage.copied_dependency_entries.is_zero(), "{storage:?}");
        assert!(storage.local_projection_entries["inherited_link_reprojections"] > 0);
    }
    assert_eq!(
        combined
            .model()
            .navigation_slot(ENGINE, RIGHT)
            .unwrap()
            .value(),
        &SlotValue::Bag(vec![
            Value::Reference(VEHICLE),
            Value::Reference(VEHICLE),
            Value::Reference(SPORTS)
        ])
    );
    assert_eq!(
        combined
            .model()
            .navigation_slot(ENGINE, RIGHT)
            .unwrap()
            .origin(),
        &Origin::AssociationOccurrences(BTreeSet::from([
            base_link(),
            LOCAL_EARLY,
            LOCAL_DUPLICATE
        ]))
    );
    let positions: Vec<_> = combined
        .model()
        .outgoing(ENGINE)
        .filter(|r| r.property == RIGHT)
        .map(|r| (r.carrier, r.position, r.target))
        .collect();
    assert_eq!(
        positions,
        [
            (
                ReferenceCarrier::AssociationOccurrence(LOCAL_EARLY),
                0,
                SPORTS
            ),
            (
                ReferenceCarrier::AssociationOccurrence(LOCAL_DUPLICATE),
                1,
                VEHICLE
            ),
            (
                ReferenceCarrier::AssociationOccurrence(base_link()),
                2,
                VEHICLE
            ),
        ]
    );
    // The original base entry must be replaced in the VEHICLE incoming bucket,
    // even though the earlier occurrence points at a different target.
    assert_eq!(
        combined
            .model()
            .incoming_for_property(VEHICLE, RIGHT)
            .map(|r| r.position)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        dependency
            .model()
            .incoming_for_property(VEHICLE, RIGHT)
            .map(|r| r.position)
            .collect::<Vec<_>>(),
        [0]
    );
    assert_ne!(
        combined.model().navigation_slot(VEHICLE, LEFT),
        dependency.model().navigation_slot(VEHICLE, LEFT)
    );
    assert!(
        combined
            .model()
            .element(VEHICLE)
            .unwrap()
            .slot(LEFT)
            .is_none()
    );
    assert!(Arc::ptr_eq(
        combined.declared().immutable_dependency().unwrap(),
        &dependency
    ));
    for record in dependency.model().elements() {
        assert!(std::ptr::eq(
            record,
            combined.model().element(record.id()).unwrap()
        ));
    }
    let mut bytes = Vec::new();
    archive::write_dependent_overlay(&combined, &mut bytes).unwrap();
    let restored =
        archive::read_dependent_overlay(Cursor::new(&bytes), registry.clone(), dependency.clone())
            .unwrap();
    assert_projection_equivalent(&restored, &oracle);
    #[cfg(feature = "verification")]
    {
        let storage = agq_kernel::storage_observer::overlay_storage(&restored);
        assert_eq!(
            storage.base_tables,
            agq_kernel::storage_observer::publication_storage(&dependency)
        );
        assert!(storage.copied_dependency_entries.is_zero(), "{storage:?}");
    }
    assert!(Arc::ptr_eq(
        restored.declared().immutable_dependency().unwrap(),
        &dependency
    ));
    let mut rewritten = Vec::new();
    archive::write_dependent_overlay(&restored, &mut rewritten).unwrap();
    assert_eq!(bytes, rewritten);
    let mut removal = revision.change_set();
    removal
        .unlink(LOCAL_EARLY)
        .unlink(LOCAL_DUPLICATE)
        .remove(SPORTS);
    let removed = DerivationBuilder::new(revision.apply(&removal).unwrap())
        .build()
        .unwrap();
    assert_projection_equivalent(&removed, &independent_base);
    assert_projection_equivalent(&dependency, &independent_base);
    assert_projection_equivalent(&combined, &oracle);
    assert_eq!(revision.model().association_occurrences().count(), 3);
}

#[test]
fn shared_storage_oracle_rejects_combined_scalar_inverse_conflict_atomically() {
    let registry = projection_registry(true);
    let dependency = Arc::new(independent_flattened(registry.clone(), &[]).unwrap());
    let oracle = independent_flattened(registry.clone(), &[]).unwrap();
    let mount = Snapshot::with_immutable_dependency(dependency.clone());
    #[cfg(feature = "verification")]
    {
        let storage = agq_kernel::storage_observer::snapshot_storage(&mount);
        assert_eq!(
            storage.base_tables,
            agq_kernel::storage_observer::publication_storage(&dependency)
        );
        assert!(storage.copied_dependency_entries.is_zero(), "{storage:?}");
    }
    let mut conflict = mount.change_set();
    conflict.create(SPORTS, ELEMENT, authored()).link(
        LOCAL_EARLY,
        A,
        ends(SPORTS, VEHICLE),
        BTreeMap::new(),
        authored(),
    );
    let error = mount.apply(&conflict).unwrap_err();
    assert!(matches!(
        error,
        ModelError::Multiplicity {
            element: VEHICLE,
            property: LEFT,
            actual: 2,
            ..
        }
    ));
    assert!(matches!(
        independent_flattened(registry, &[(LOCAL_EARLY, SPORTS, VEHICLE)]),
        Err(DerivationError::Model(ModelError::Multiplicity {
            element: VEHICLE,
            property: LEFT,
            actual: 2,
            ..
        }))
    ));
    assert!(mount.model().element(SPORTS).is_none());
    assert!(mount.model().association_occurrence(LOCAL_EARLY).is_none());
    assert_projection_equivalent(&dependency, &oracle);
    let mut legal = mount.change_set();
    legal.create(SPORTS, ELEMENT, authored()).link(
        LOCAL_EARLY,
        A,
        ends(SPORTS, ENGINE_USE),
        BTreeMap::new(),
        authored(),
    );
    let next = mount.apply(&legal).unwrap();
    assert!(
        next.model().association_occurrence(LOCAL_EARLY).is_some(),
        "rejected edit must not reserve local identities"
    );
    assert_eq!(dependency.model().association_occurrences().count(), 1);
}
