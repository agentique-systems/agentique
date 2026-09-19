//! Regression witness for the pinned, unresolved KERML11-81 authority conflict.
//! No participant derivation is asserted by these deliberately invalid proposals.
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries, SemanticContext};
use agq_kernel::{
    AssociationOccurrenceId, ChangeSet, ElementId, MetaclassId, ModelError, Snapshot,
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use std::{collections::BTreeMap, sync::Arc};

fn origin() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
fn create(base: &Snapshot, changes: &mut ChangeSet, id: ElementId, class: MetaclassId) {
    let registry = base.model().registry();
    changes.create(id, class, origin());
    for property in registry.effective_properties(class).unwrap() {
        if property.derived || property.multiplicity.lower == 0 {
            continue;
        }
        let value = match registry.storage_kind(property.value_kind).unwrap() {
            ValueKind::Boolean => Value::Boolean(false),
            ValueKind::String => Value::String(id.to_string()),
            ValueKind::Enumeration(domain) => Value::Enumeration(
                *registry
                    .enumeration(domain)
                    .unwrap()
                    .literals
                    .iter()
                    .find(|(_, name)| *name == "public")
                    .unwrap()
                    .0,
            ),
            ValueKind::Reference(_) => continue,
            _ => panic!("unexpected required primitive"),
        };
        changes.set(id, property.id, SlotValue::Scalar(value), origin());
    }
}

#[test]
fn inherited_end_identity_cannot_satisfy_the_pinned_participant_opposite_bound() {
    witness(agq_kerml::BaselineProfile::PublishedKerMl10);
}

#[test]
fn operational_profile_publishes_shared_inherited_ends_without_participant_facts() {
    witness(agq_kerml::BaselineProfile::OPERATIONAL);
}

fn witness(profile: agq_kerml::BaselineProfile) {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let published = agq_kerml::registry().unwrap();
    let registry = &published;
    let participant = registry
        .property(p::A_PARTICIPANT_FEATURE_INTERACTION_PARTICIPANT_FEATURE)
        .expect("pinned external association end");
    let inverse = registry
        .property(*participant.opposite_ends.first().unwrap())
        .unwrap();
    assert!(!participant.derived);
    assert!(!participant.ordered);
    assert_eq!(participant.multiplicity.lower, 2);
    assert_eq!(inverse.multiplicity.lower, 1);
    assert_eq!(inverse.multiplicity.upper, Some(1));
    let association = participant.association.unwrap();
    let parent = ElementId::from_u128(1);
    let child = ElementId::from_u128(2);
    let ends = [ElementId::from_u128(3), ElementId::from_u128(4)];
    let memberships = [ElementId::from_u128(5), ElementId::from_u128(6)];
    let specialization = ElementId::from_u128(7);
    let mut changes = base.change_set();
    for ty in [parent, child] {
        create(&base, &mut changes, ty, c::INTERACTION);
    }
    for (feature, membership) in ends.into_iter().zip(memberships) {
        create(&base, &mut changes, feature, c::FEATURE);
        create(&base, &mut changes, membership, c::FEATURE_MEMBERSHIP);
        changes.set(
            feature,
            p::FEATURE_IS_END,
            SlotValue::Scalar(Value::Boolean(true)),
            origin(),
        );
        changes.set(
            membership,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(feature)]),
            origin(),
        );
    }
    changes.set(
        parent,
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(memberships.map(Value::Reference).into()),
        origin(),
    );
    create(&base, &mut changes, specialization, c::SUBCLASSIFICATION);
    changes.set(
        specialization,
        p::SUBCLASSIFICATION_SUBCLASSIFIER,
        SlotValue::Scalar(Value::Reference(child)),
        origin(),
    );
    changes.set(
        specialization,
        p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        SlotValue::Scalar(Value::Reference(parent)),
        origin(),
    );
    changes.set(
        child,
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(specialization)]),
        origin(),
    );
    let construction = base.preview(&changes).unwrap();
    let queries = KerMlQueries::new(
        SemanticContext::for_construction(
            &construction,
            agq_kerml_semantics::SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let effective = queries.effective_features(child);
    assert_eq!(effective.completeness, Completeness::Complete);
    assert_eq!(effective.value, ends);
    if profile == agq_kerml::BaselineProfile::OPERATIONAL {
        assert!(base.model().registry().property(participant.id).is_err());
        assert!(base.model().registry().association(association).is_err());
        assert!(construction.obligations().is_empty());
        let snapshot = base.apply(&changes).unwrap();
        assert_eq!(snapshot.model().len(), 7);
        assert_eq!(snapshot.model().association_occurrences().count(), 0);
        println!(
            "Agentique operational errata profile: original inherited identities, no participant schema obligation or fabricated links; strict Snapshot accepted"
        );
        return;
    }
    assert_eq!(construction.obligations().len(), 2);
    assert!(
        construction
            .obligations()
            .iter()
            .all(|o| o.property == participant.id)
    );
    assert!(
        matches!(base.apply(&changes), Err(ModelError::Multiplicity { property, actual: 0, .. }) if property == participant.id)
    );
    // Hypothesis under test: treat the original inherited ends as participantFeature.
    // It contradicts the exact inverse 1..1, even in unpublished construction.
    for (index, (interaction, feature)) in [parent, child]
        .into_iter()
        .flat_map(|ty| ends.map(|end| (ty, end)))
        .enumerate()
    {
        changes.link(
            AssociationOccurrenceId::from_u128(100 + index as u128),
            association,
            BTreeMap::from([(participant.id, feature), (inverse.id, interaction)]),
            BTreeMap::new(),
            origin(),
        );
    }
    let error = base.preview(&changes).unwrap_err();
    assert!(
        matches!(error, ModelError::Multiplicity { property, required, actual: 2, .. } if property == inverse.id && required.upper == Some(1))
    );
    println!("Exact participantFeature mapping rejected: {error:?}");
    assert!(base.model().is_empty());
}
