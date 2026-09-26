#![cfg(feature = "verification")]
mod common;
use agq_kernel::{derived::*, provenance::*, *};
use common::*;
use std::{collections::BTreeSet, sync::Arc};

fn key(subject: ElementId, output: u128) -> DerivationKey {
    DerivationKey {
        rule: RuleId::from_u128(990),
        subject,
        output: OutputKey::from_u128(output),
    }
}
fn fixture(snapshot: &Snapshot) -> DerivedOverlay {
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.element(
        key(VEHICLE, 1),
        PART_DEF,
        [(NAME, text("derived"))],
        BTreeSet::new(),
    );
    builder.element(
        key(key(VEHICLE, 1).element_id(), 2),
        PART_DEF,
        [],
        BTreeSet::new(),
    );
    builder.searches(
        FactKey::Element(key(VEHICLE, 1).element_id()),
        BTreeSet::from([StructuralSearch::Incoming(ENGINE)]),
    );
    builder.build().unwrap()
}

#[test]
fn unchanged_support_retains_exact_records_proofs_searches_and_declared_separation() {
    let snapshot = vertical();
    let overlay = fixture(&snapshot);
    let mut edit = snapshot.change_set();
    edit.create(ElementId::from_u128(99), PART_DEF, authored());
    let next = snapshot.apply(&edit).unwrap();
    let retained = overlay
        .verification_retain_on(next.clone(), &overlay.verification_local_facts())
        .unwrap();
    assert_eq!(retained.retracted, 0);
    for (fact, proof) in overlay.facts() {
        assert_eq!(retained.overlay.explain(fact), Some(proof));
        assert_eq!(
            retained
                .overlay
                .model()
                .computation_searches_for(fact)
                .collect::<Vec<_>>(),
            overlay
                .model()
                .computation_searches_for(fact)
                .collect::<Vec<_>>()
        );
    }
    let derived = key(VEHICLE, 1).element_id();
    assert!(std::ptr::eq(
        overlay.model().element(derived).unwrap(),
        retained.overlay.model().element(derived).unwrap()
    ));
    assert!(next.model().element(derived).is_none());
    assert!(snapshot.model().element(ElementId::from_u128(99)).is_none());
}

#[test]
fn changed_declared_support_retracts_transitive_chain_and_partial_created_records() {
    let snapshot = vertical();
    let overlay = fixture(&snapshot);
    let mut edit = snapshot.change_set();
    edit.set(VEHICLE, NAME, text("changed"), authored());
    let next = snapshot.apply(&edit).unwrap();
    let rebuilt = overlay
        .verification_retain_on(next, &overlay.verification_local_facts())
        .unwrap();
    assert!(rebuilt.retained.is_empty());
    assert!(
        rebuilt
            .overlay
            .model()
            .element(key(VEHICLE, 1).element_id())
            .is_none()
    );
    let mut partial = overlay.verification_local_facts();
    partial.remove(&FactKey::Property {
        element: key(VEHICLE, 1).element_id(),
        property: NAME,
    });
    let rebuilt = overlay.verification_retain_on(snapshot, &partial).unwrap();
    assert!(rebuilt.retained.is_empty());
}

#[test]
fn retained_ordered_append_keeps_contribution_and_changed_prefix_retracts_it() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder.extend_ordered_references(
        OWNS,
        SOURCES,
        vec![ENGINE],
        Explanation {
            rule: RuleId::from_u128(991),
            dependencies: BTreeSet::new(),
        },
    );
    let overlay = builder.build().unwrap();
    let rebuilt = overlay
        .verification_retain_on(snapshot.clone(), &overlay.verification_local_facts())
        .unwrap();
    assert_eq!(
        rebuilt
            .overlay
            .model()
            .ordered_reference_contribution(OWNS, SOURCES, ENGINE),
        overlay
            .model()
            .ordered_reference_contribution(OWNS, SOURCES, ENGINE)
    );
    let mut edit = snapshot.change_set();
    edit.set(OWNS, SOURCES, ordered_refs(&[SPORTS]), authored());
    let next = snapshot.apply(&edit).unwrap();
    let rebuilt = overlay
        .verification_retain_on(next.clone(), &overlay.verification_local_facts())
        .unwrap();
    assert!(rebuilt.retained.is_empty());
    assert_eq!(
        rebuilt.overlay.model().navigation_slot(OWNS, SOURCES),
        next.model().navigation_slot(OWNS, SOURCES)
    );
}

#[test]
fn failed_computations_are_never_retained() {
    let snapshot = vertical();
    let mut builder = DerivationBuilder::new(snapshot.clone());
    builder
        .failure(
            VEHICLE,
            EFFECTIVE,
            ComputationFailure::Invalid {
                diagnostic: "fixture".into(),
                explanation: Explanation {
                    rule: RuleId::from_u128(992),
                    dependencies: BTreeSet::new(),
                },
                searches: BTreeSet::new(),
            },
        )
        .unwrap();
    let overlay = builder.build().unwrap();
    let rebuilt = overlay
        .verification_retain_on(snapshot, &overlay.verification_local_facts())
        .unwrap();
    assert!(rebuilt.retained.is_empty());
    assert_eq!(rebuilt.overlay.model().computation_failures().count(), 0);
}

#[test]
fn different_registry_and_equal_but_separately_mounted_dependency_are_rejected() {
    let snapshot = vertical();
    let overlay = fixture(&snapshot);
    assert!(matches!(
        overlay.verification_retain_on(vertical(), &overlay.verification_local_facts()),
        Err(DerivationError::InputContextMismatch)
    ));
    let dependency = Arc::new(DerivationBuilder::new(snapshot).build().unwrap());
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let local = DerivationBuilder::new(base).build().unwrap();
    let other = Snapshot::with_immutable_dependency(Arc::new((*dependency).clone()));
    assert!(matches!(
        local.verification_retain_on(other, &BTreeSet::new()),
        Err(DerivationError::InputContextMismatch)
    ));
}
