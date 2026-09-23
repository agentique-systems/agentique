mod common;
use agq_kernel::{derived::*, provenance::*, *};
use common::*;
use std::{collections::BTreeSet, io::Cursor, sync::Arc};

const RULE: RuleId = RuleId::from_u128(901);
fn proof(dependencies: impl IntoIterator<Item = Dependency>) -> Explanation {
    Explanation {
        rule: RULE,
        dependencies: dependencies.into_iter().collect(),
    }
}
fn property() -> FactKey {
    FactKey::Property {
        element: OWNS,
        property: SOURCES,
    }
}
fn declared() -> Snapshot {
    let base = vertical();
    let mut changes = base.change_set();
    changes.set(OWNS, SOURCES, ordered_refs(&[VEHICLE]), authored());
    base.apply(&changes).unwrap()
}
fn key(output: u128) -> DerivationKey {
    DerivationKey {
        rule: RULE,
        subject: OWNS,
        output: OutputKey::from_u128(output),
    }
}
fn contribution(model: &ModelView, target: ElementId) -> &OrderedReferenceContribution {
    model
        .ordered_reference_contribution(OWNS, SOURCES, target)
        .unwrap()
}

#[test]
fn initial_derived_references_keep_creation_reads_without_sibling_or_later_append_proofs() {
    let owner = key(10);
    let owner_id = owner.element_id();
    let sibling = key(11);
    let late = key(12);
    let owner_fact = FactKey::Element(owner_id);
    let slot_fact = FactKey::Property {
        element: owner_id,
        property: SOURCES,
    };
    let creation_read = StructuralSearch::Incoming(ElementId::from_u128(999));
    let slot_read = StructuralSearch::ElementIdentity(ElementId::from_u128(998));
    let explicit = Dependency::Declared(FactKey::Property {
        element: SPORTS,
        property: NAME,
    });
    let mut builder = DerivationBuilder::new(declared());
    builder.element(sibling, PART_DEF, [], BTreeSet::new());
    builder.element(
        owner,
        SPECIALIZATION,
        [
            (SPECIFIC, scalar_ref(VEHICLE)),
            (GENERAL, scalar_ref(ENGINE)),
            (SOURCES, ordered_refs(&[ENGINE, sibling.element_id()])),
        ],
        BTreeSet::from([explicit]),
    );
    builder.searches(owner_fact, BTreeSet::from([creation_read.clone()]));
    builder.searches(slot_fact, BTreeSet::from([slot_read.clone()]));
    let initial = builder.build().unwrap();
    let support = initial
        .model()
        .ordered_reference_contribution(owner_id, SOURCES, ENGINE)
        .unwrap()
        .clone();
    assert_eq!(support.position(), 0);
    assert_eq!(
        support.explanation().dependencies,
        BTreeSet::from([
            Dependency::Derived(owner_fact),
            Dependency::Declared(FactKey::Element(ENGINE)),
        ])
    );
    assert_eq!(
        support.searches(),
        &BTreeSet::from([creation_read.clone(), slot_read])
    );
    assert!(
        initial
            .explain(owner_fact)
            .unwrap()
            .dependencies
            .contains(&explicit)
    );
    assert!(
        initial
            .model()
            .computation_searches_for(owner_fact)
            .any(|search| search == &creation_read)
    );
    let sibling_support = initial
        .model()
        .ordered_reference_contribution(owner_id, SOURCES, sibling.element_id())
        .unwrap();
    assert_eq!(sibling_support.position(), 1);
    assert!(
        sibling_support
            .explanation()
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(sibling.element_id())))
    );

    let mut next = DerivationBuilder::from_overlay(initial.clone());
    next.element(late, PART_DEF, [], BTreeSet::new());
    next.extend_ordered_references(owner_id, SOURCES, vec![late.element_id()], proof([]));
    next.searches(slot_fact, BTreeSet::from([StructuralSearch::Model]));
    let next = next.build().unwrap();
    assert_eq!(
        next.model()
            .ordered_reference_contribution(owner_id, SOURCES, ENGINE),
        Some(&support)
    );
    assert!(!support.searches().contains(&StructuralSearch::Model));
    assert!(
        !support
            .explanation()
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(late.element_id())))
    );
    assert!(
        next.explain(slot_fact)
            .unwrap()
            .dependencies
            .contains(&Dependency::Derived(FactKey::Element(late.element_id())))
    );
}

#[test]
fn repeated_initial_targets_retain_aggregate_evidence_for_each_position() {
    let owner = key(20);
    let mut builder = DerivationBuilder::new(declared());
    builder.element(
        owner,
        SPECIALIZATION,
        [
            (SPECIFIC, scalar_ref(VEHICLE)),
            (GENERAL, scalar_ref(ENGINE)),
            (TARGETS, ordered_refs(&[ENGINE, ENGINE])),
        ],
        BTreeSet::new(),
    );
    let overlay = builder.build().unwrap();
    assert!(
        overlay
            .model()
            .ordered_reference_contribution(owner.element_id(), TARGETS, ENGINE)
            .is_none()
    );
    assert_eq!(
        overlay
            .model()
            .element(owner.element_id())
            .unwrap()
            .slot(TARGETS)
            .unwrap()
            .value(),
        &ordered_refs(&[ENGINE, ENGINE])
    );
}

#[test]
fn separate_append_events_retain_only_their_own_support_and_searches() {
    let snapshot = declared();
    let old_search = StructuralSearch::Incoming(ENGINE);
    let new_search = StructuralSearch::ElementIdentity(SPORTS);
    let explicit = Dependency::Declared(FactKey::Property {
        element: SPORTS,
        property: NAME,
    });
    let mut first = DerivationBuilder::new(snapshot);
    first.extend_ordered_references(OWNS, SOURCES, vec![ENGINE], proof([explicit]));
    first.searches(property(), BTreeSet::from([old_search.clone()]));
    let first = first.build().unwrap();
    let first_proof = contribution(first.model(), ENGINE).clone();
    assert_eq!(first_proof.position(), 1);
    assert!(first_proof.explanation().dependencies.contains(&explicit));
    assert_eq!(
        first_proof.searches(),
        &BTreeSet::from([old_search.clone()])
    );
    assert!(
        first
            .model()
            .ordered_reference_contribution(OWNS, SOURCES, VEHICLE)
            .is_none()
    );

    let a = key(1);
    let b = key(2);
    let mut next = DerivationBuilder::from_overlay(first.clone());
    for output in [a, b] {
        next.element(output, PART_DEF, [], BTreeSet::new());
    }
    next.extend_ordered_references(
        OWNS,
        SOURCES,
        vec![a.element_id(), b.element_id()],
        proof([]),
    );
    next.searches(property(), BTreeSet::from([new_search.clone()]));
    let next = next.build().unwrap();
    assert!(!next.build_metrics().reused_owned_storage);
    assert_eq!(contribution(next.model(), ENGINE), &first_proof);
    assert_eq!(contribution(first.model(), ENGINE), &first_proof);
    for (position, selected, sibling) in [(2, a, b), (3, b, a)] {
        let support = contribution(next.model(), selected.element_id());
        assert_eq!(support.position(), position);
        assert_eq!(support.searches(), &BTreeSet::from([new_search.clone()]));
        assert_eq!(
            support.explanation().dependencies,
            BTreeSet::from([
                Dependency::Declared(FactKey::Element(OWNS)),
                Dependency::Derived(FactKey::Element(selected.element_id())),
            ])
        );
        assert!(
            !support
                .explanation()
                .dependencies
                .contains(&Dependency::Derived(FactKey::Element(sibling.element_id())))
        );
    }
    assert_eq!(next.model().ordered_reference_contributions().count(), 3);
    assert_eq!(
        next.model()
            .computation_searches_for(property())
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([old_search, new_search])
    );
    let mut moved = DerivationBuilder::from_overlay(next);
    moved.extend_ordered_references(OWNS, SOURCES, vec![SPORTS], proof([]));
    let moved = moved.build().unwrap();
    assert!(moved.build_metrics().reused_owned_storage);
    assert_eq!(contribution(moved.model(), ENGINE), &first_proof);
    assert_eq!(contribution(moved.model(), SPORTS).position(), 4);
}

#[test]
fn adoption_keeps_explicit_context_beyond_existing_target_existence() {
    let mut builder = DerivationBuilder::new(declared());
    let decision = Dependency::Declared(FactKey::Property {
        element: SPORTS,
        property: NAME,
    });
    builder.extend_ordered_references(OWNS, SOURCES, vec![ENGINE], proof([decision]));
    let overlay = builder.build().unwrap();
    assert_eq!(
        contribution(overlay.model(), ENGINE)
            .explanation()
            .dependencies,
        BTreeSet::from([
            decision,
            Dependency::Declared(FactKey::Element(OWNS)),
            Dependency::Declared(FactKey::Element(ENGINE)),
        ])
    );
    assert!(
        overlay
            .explain(property())
            .unwrap()
            .dependencies
            .contains(&Dependency::Declared(property()))
    );
}

#[test]
fn later_unattributed_searches_invalidate_cached_precision() {
    for empty_append in [false, true] {
        let mut builder = DerivationBuilder::new(declared());
        builder.extend_ordered_references(OWNS, SOURCES, vec![ENGINE], proof([]));
        let before = builder.build().unwrap();
        let mut next = DerivationBuilder::from_overlay(before.clone());
        if empty_append {
            next.extend_ordered_references(OWNS, SOURCES, vec![], proof([]));
        }
        next.searches(property(), BTreeSet::from([StructuralSearch::Model]));
        let next = next.build().unwrap();
        assert!(
            next.model()
                .ordered_reference_contribution(OWNS, SOURCES, ENGINE)
                .is_none()
        );
        assert!(
            before
                .model()
                .ordered_reference_contribution(OWNS, SOURCES, ENGINE)
                .is_some()
        );
        assert!(next.explain(property()).is_some());
    }
}

#[test]
fn construction_and_immutable_mounts_preserve_contributions() {
    let snapshot = declared();
    let changes = snapshot.change_set();
    let construction = Arc::new(snapshot.preview(&changes).unwrap());
    let mut builder = ConstructionDerivationBuilder::for_construction(construction);
    builder.extend_ordered_references(OWNS, SOURCES, vec![ENGINE], proof([]));
    let overlay = builder.build().unwrap();
    let support = contribution(overlay.model(), ENGINE).clone();
    let mut next = ConstructionDerivationBuilder::from_construction_overlay(overlay);
    next.extend_ordered_references(OWNS, SOURCES, vec![SPORTS], proof([]));
    let next = next.build().unwrap();
    assert!(next.build_metrics().reused_owned_storage);
    assert_eq!(contribution(next.model(), ENGINE), &support);
    let strict = Arc::new(next.revalidate(snapshot.apply(&changes).unwrap()).unwrap());
    let mounted =
        Snapshot::with_immutable_dependency_in_registry(strict.clone(), registry()).unwrap();
    assert_eq!(contribution(mounted.model(), ENGINE), &support);
    assert_eq!(
        contribution(
            mounted.apply(&mounted.change_set()).unwrap().model(),
            ENGINE
        ),
        &support
    );
    assert_eq!(
        contribution(
            mounted.preview(&mounted.change_set()).unwrap().model(),
            ENGINE
        ),
        &support
    );
}

#[test]
fn archive_bytes_stay_canonical_and_missing_cache_requires_fallback() {
    let mut builder = DerivationBuilder::new(declared());
    builder.extend_ordered_references(OWNS, SOURCES, vec![ENGINE], proof([]));
    let overlay = builder.build().unwrap();
    let mut bytes = vec![];
    archive::write_overlay(&overlay, &mut bytes).unwrap();
    let restored = archive::read_overlay(Cursor::new(&bytes), registry()).unwrap();
    assert!(
        restored
            .model()
            .ordered_reference_contribution(OWNS, SOURCES, ENGINE)
            .is_none()
    );
    assert_eq!(restored.explain(property()), overlay.explain(property()));
    assert!(restored.model().elements().eq(overlay.model().elements()));
    let mut roundtrip = vec![];
    archive::write_overlay(&restored, &mut roundtrip).unwrap();
    assert_eq!(
        bytes, roundtrip,
        "optional contribution cache never changes archive bytes"
    );

    let dependency = Arc::new(overlay);
    let local = DerivationBuilder::new(Snapshot::with_immutable_dependency(dependency.clone()))
        .build()
        .unwrap();
    let mut bytes = vec![];
    archive::write_dependent_overlay(&local, &mut bytes).unwrap();
    let restored =
        archive::read_dependent_overlay(Cursor::new(bytes), registry(), dependency.clone())
            .unwrap();
    assert_eq!(
        contribution(restored.model(), ENGINE),
        contribution(dependency.model(), ENGINE)
    );
}
