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
