//! Formal SysML redefined domains and selectByKind subsets are distinct.
use super::*;
use agq_kernel::{
    AssociationOccurrenceId, ChangeSet, Snapshot, metamodel::ValueKind, provenance::DeclaredOrigin,
};
use std::sync::Arc;

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn create(base: &Snapshot, changes: &mut ChangeSet, n: u128, class: MetaclassId) {
    let origin = DeclaredOrigin::Authored { source: None };
    changes.create(id(n), class, origin.clone());
    for property in base.model().registry().effective_properties(class).unwrap() {
        if property.derived || property.multiplicity.lower == 0 {
            continue;
        }
        let value = match base
            .model()
            .registry()
            .storage_kind(property.value_kind)
            .unwrap()
        {
            ValueKind::Boolean => Value::Boolean(false),
            ValueKind::String => Value::String(n.to_string()),
            ValueKind::Enumeration(domain) => Value::Enumeration(
                *base
                    .model()
                    .registry()
                    .enumeration(domain)
                    .unwrap()
                    .literals
                    .iter()
                    .find(|(_, name)| name.as_str() == "public")
                    .unwrap()
                    .0,
            ),
            ValueKind::Reference(_) => continue,
            other => panic!("fixture {other:?}"),
        };
        changes.set(id(n), property.id, SlotValue::Scalar(value), origin.clone());
    }
}
fn reference(
    base: &Snapshot,
    changes: &mut ChangeSet,
    n: u128,
    property: PropertyId,
    target: u128,
) {
    let origin = DeclaredOrigin::Authored { source: None };
    if base
        .model()
        .registry()
        .supports_slot_storage(property)
        .unwrap()
    {
        changes.set(
            id(n),
            property,
            SlotValue::Scalar(Value::Reference(id(target))),
            origin,
        );
    } else {
        let p = base.model().registry().property(property).unwrap();
        changes.link(
            AssociationOccurrenceId::new(),
            p.association.unwrap(),
            BTreeMap::from([
                (property, id(target)),
                (*p.opposite_ends.first().unwrap(), id(n)),
            ]),
            BTreeMap::new(),
            origin,
        );
    }
}
fn fixture(
    usage: MetaclassId,
    types: &[(u128, MetaclassId)],
    specialization: Option<(u128, u128)>,
) -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut changes = base.change_set();
    create(&base, &mut changes, 1, usage);
    let mut owned = vec![];
    for (index, &(number, class)) in types.iter().enumerate() {
        create(&base, &mut changes, number, class);
        let relationship = 100 + index as u128;
        create(&base, &mut changes, relationship, kc::FEATURE_TYPING);
        owned.push(Value::Reference(id(relationship)));
        reference(
            &base,
            &mut changes,
            relationship,
            kp::FEATURE_TYPING_TYPED_FEATURE,
            1,
        );
        reference(
            &base,
            &mut changes,
            relationship,
            kp::FEATURE_TYPING_TYPE,
            number,
        );
    }
    changes.set(
        id(1),
        kp::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(owned),
        DeclaredOrigin::Authored { source: None },
    );
    if let Some((specific, general)) = specialization {
        create(&base, &mut changes, 200, kc::SUBCLASSIFICATION);
        changes.set(
            id(specific),
            kp::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![Value::Reference(id(200))]),
            DeclaredOrigin::Authored { source: None },
        );
        reference(
            &base,
            &mut changes,
            200,
            kp::SUBCLASSIFICATION_SUBCLASSIFIER,
            specific,
        );
        reference(
            &base,
            &mut changes,
            200,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            general,
        );
    }
    base.apply(&changes).unwrap()
}
fn queries(snapshot: &Snapshot) -> SysmlQueries<'_> {
    SysmlQueries::new(crate::context::fixture_context(snapshot, BTreeSet::new()))
}

#[test]
fn part_and_item_are_subsets_not_narrowed_domains() {
    let snapshot = fixture(
        sc::PART_USAGE,
        &[(10, sc::PART_DEFINITION), (11, sc::ITEM_DEFINITION)],
        None,
    );
    let q = queries(&snapshot);
    let answer = q.current_part_definitions(id(1));
    assert_eq!(answer.value(), &[id(10)]);
    assert_eq!(answer.completeness(), Completeness::Complete);
    assert_eq!(answer.filtered_targets, BTreeSet::from([id(11)]));
    assert!(answer.rejected_targets.is_empty());
    let snapshot = fixture(
        sc::ITEM_USAGE,
        &[(10, kc::STRUCTURE), (11, kc::CLASS)],
        None,
    );
    let answer = queries(&snapshot).current_item_definitions(id(1));
    assert_eq!(answer.value(), &[id(10)]);
    assert_eq!(answer.completeness(), Completeness::Complete);
    assert_eq!(answer.filtered_targets, BTreeSet::from([id(11)]));
}

#[test]
fn proven_classifier_ancestor_is_pruned_with_kerml_hierarchy_evidence() {
    // The raw candidate population mirrors partial projects where virtual
    // metaclass ancestry survives feature_types' canonical-edge pruning.
    let snapshot = fixture(
        sc::PART_USAGE,
        &[(10, sc::PART_DEFINITION), (11, kc::CLASSIFIER)],
        Some((10, 11)),
    );
    let q = queries(&snapshot);
    let raw = q.wrap(q.kerml().direct_feature_types(id(1)));
    assert_eq!(raw.value().len(), 2);
    let answer = q.prune_general_types(raw);
    assert_eq!(answer.value(), &[id(10)]);
    assert_eq!(answer.filtered_targets, BTreeSet::from([id(11)]));
    assert!(
        answer
            .supporting_queries
            .iter()
            .any(|proof| proof.value.contains(&id(11)) && !proof.positive_dependencies.is_empty())
    );
    assert_eq!(
        q.current_part_definitions(id(1)).completeness(),
        Completeness::Complete
    );
    assert_eq!(q.current_usage_types(id(1)).value(), &[id(10)]);
    assert_eq!(q.direct_usage_types(id(1)).value().len(), 2);

    let pending = SysmlQueries::new(crate::context::fixture_context(
        &snapshot,
        BTreeSet::from([id(10)]),
    ));
    let answer =
        pending.prune_general_types(pending.wrap(pending.kerml().direct_feature_types(id(1))));
    assert_eq!(answer.value(), &[id(10)]);
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert!(
        answer
            .supporting_queries
            .iter()
            .any(|proof| proof.completeness == Completeness::Incomplete)
    );
}

#[test]
fn incompatible_nonancestors_remain_invalid_and_untyped_part_remains_pending() {
    for (usage, target) in [
        (sc::PART_USAGE, kc::DATA_TYPE),
        (sc::ITEM_USAGE, kc::DATA_TYPE),
        (sc::PORT_USAGE, kc::STRUCTURE),
        (sc::ATTRIBUTE_USAGE, kc::CLASS),
    ] {
        let snapshot = fixture(usage, &[(10, target)], None);
        let q = queries(&snapshot);
        let answer = match usage {
            sc::PART_USAGE => q.current_part_definitions(id(1)),
            sc::ITEM_USAGE => q.current_item_definitions(id(1)),
            sc::PORT_USAGE => q.current_port_definitions(id(1)),
            _ => q.current_attribute_definitions(id(1)),
        };
        assert_eq!(answer.completeness(), Completeness::Invalid);
        assert_eq!(answer.rejected_targets, BTreeSet::from([id(10)]));
        assert!(answer.filtered_targets.is_empty());
    }
    let snapshot = fixture(sc::PART_USAGE, &[], None);
    let answer = queries(&snapshot).current_part_definitions(id(1));
    assert!(answer.value().is_empty());
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert!(
        answer
            .diagnostics
            .iter()
            .any(|d| d.code == "SQ_PART_DEFINITION_PENDING")
    );
}
