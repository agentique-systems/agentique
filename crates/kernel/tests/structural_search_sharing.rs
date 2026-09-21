mod common;
use agq_kernel::{derived::*, provenance::*, *};
use common::*;
use std::{collections::BTreeSet, sync::Arc};

const RULE: RuleId = RuleId::from_u128(810);

fn key(index: usize) -> DerivationKey {
    DerivationKey {
        rule: RULE,
        subject: VEHICLE,
        output: OutputKey::from_u128(index as u128),
    }
}

fn reads(model: &ModelView, fact: FactKey) -> &BTreeSet<StructuralSearch> {
    model
        .computation_searches()
        .find(|(candidate, _)| **candidate == fact)
        .unwrap()
        .1
}

#[test]
fn interning_and_unions_use_exact_content_without_mutating_callers() {
    let mut pool = StructuralSearchPool::default();
    let mut caller = Arc::new(BTreeSet::from([
        StructuralSearch::Incoming(ENGINE),
        StructuralSearch::Element(ElementId::from_u128(999)),
    ]));
    let first = pool.intern_shared(caller.clone());
    assert!(Arc::ptr_eq(&first, &caller));
    let equal = pool.intern(first.as_ref().clone());
    assert!(Arc::ptr_eq(&first, &equal));
    let same = pool.union_shared(&first, equal);
    assert!(Arc::ptr_eq(&first, &same));
    let subset = Arc::new(BTreeSet::from([StructuralSearch::Incoming(ENGINE)]));
    assert!(Arc::ptr_eq(&first, &pool.union_shared(&first, subset)));
    let additional = Arc::new(BTreeSet::from([StructuralSearch::DescriptorGraph]));
    let union = pool.union_shared(&first, additional.clone());
    assert_eq!(union.len(), 3);
    assert_eq!(first.len(), 2);
    assert_eq!(additional.len(), 1);
    assert!(Arc::ptr_eq(
        &union,
        &pool.union_shared(&additional, first.clone())
    ));
    Arc::make_mut(&mut caller).insert(StructuralSearch::Model);
    assert!(!first.contains(&StructuralSearch::Model));
    assert!(!union.contains(&StructuralSearch::Model));
}

#[test]
fn shared_searches_survive_snapshots_failures_and_additive_union() {
    let snapshot = vertical();
    let a = FactKey::Element(key(1).element_id());
    let b = FactKey::Element(key(2).element_id());
    let common = Arc::new(BTreeSet::from([
        StructuralSearch::Incoming(ENGINE),
        StructuralSearch::Element(ElementId::from_u128(999)),
    ]));
    let mut builder = DerivationBuilder::new(snapshot);
    for index in [1, 2] {
        builder.element(key(index), PART_DEF, [], BTreeSet::new());
        builder.searches_shared(FactKey::Element(key(index).element_id()), common.clone());
    }
    let first = builder.build().unwrap();
    assert!(std::ptr::eq(
        reads(first.model(), a),
        reads(first.model(), b)
    ));
    let project = Snapshot::with_immutable_dependency(Arc::new(first.clone()));
    let mut changes = project.change_set();
    changes.create(ElementId::from_u128(998), PART_DEF, authored());
    let revision = project.apply(&changes).unwrap();
    assert!(std::ptr::eq(
        reads(first.model(), a),
        reads(revision.model(), a)
    ));

    let additional = StructuralSearch::Property {
        element: ENGINE,
        property: NAME,
    };
    let mut builder = DerivationBuilder::from_overlay(first.clone());
    builder.searches(a, BTreeSet::from([additional.clone()]));
    builder
        .failure(
            SPORTS,
            EFFECTIVE,
            ComputationFailure::Invalid {
                diagnostic: "unsuccessful search fixture".into(),
                explanation: Explanation {
                    rule: RULE,
                    dependencies: BTreeSet::new(),
                },
                searches: common.as_ref().clone(),
            },
        )
        .unwrap();
    let second = builder.build().unwrap();
    let mut expected = common.as_ref().clone();
    expected.insert(additional);
    assert_eq!(reads(second.model(), a), &expected);
    assert_eq!(reads(first.model(), a), common.as_ref());
    assert!(std::ptr::eq(
        reads(second.model(), b),
        reads(first.model(), b)
    ));
    assert!(std::ptr::eq(
        reads(second.model(), b),
        reads(
            second.model(),
            FactKey::Property {
                element: SPORTS,
                property: EFFECTIVE,
            }
        )
    ));
    assert_eq!(second.build_metrics().logical_search_sets, 3);
    assert_eq!(second.build_metrics().logical_search_entries, 7);
}

#[test]
fn twelve_thousand_outputs_retain_one_six_thousand_key_search_population() {
    const OUTPUTS: usize = 12_000;
    const SEARCHES: usize = 6_000;
    let snapshot = vertical();
    let common = Arc::new(
        (0..SEARCHES)
            .map(|index| StructuralSearch::Element(ElementId::from_u128(10_000 + index as u128)))
            .collect::<BTreeSet<_>>(),
    );
    let proof = Arc::new(Explanation {
        rule: RULE,
        dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(VEHICLE))]),
    });
    let mut builder = DerivationBuilder::new(snapshot.clone());
    for index in 0..OUTPUTS {
        builder.element_with_explanation(key(index), PART_DEF, [], proof.clone());
        builder.searches_shared(FactKey::Element(key(index).element_id()), common.clone());
    }
    let overlay = builder.build().unwrap();
    let metrics = overlay.build_metrics();
    assert_eq!(metrics.logical_search_sets, OUTPUTS);
    assert_eq!(metrics.logical_search_entries, OUTPUTS * SEARCHES);
    assert_eq!(metrics.retained_search_sets, 1);
    assert_eq!(metrics.retained_search_entries, SEARCHES);
    assert_eq!(metrics.search_sets_interned, 1);
    assert_eq!(metrics.search_sets_reused, OUTPUTS - 1);
    assert!(
        overlay
            .model()
            .computation_searches()
            .all(|(_, set)| { std::ptr::eq(set, common.as_ref()) && set.len() == SEARCHES })
    );

    let mut next = DerivationBuilder::new(snapshot);
    next.element_with_explanation(key(OUTPUTS), PART_DEF, [], proof);
    next.searches_shared(FactKey::Element(key(OUTPUTS).element_id()), common);
    let next = next.build_on_overlay(overlay).unwrap();
    let metrics = next.build_metrics();
    assert_eq!(metrics.logical_search_entries, (OUTPUTS + 1) * SEARCHES);
    assert_eq!(metrics.retained_search_sets, 1);
    assert_eq!(metrics.retained_search_entries, SEARCHES);
    assert_eq!(metrics.search_sets_interned, 0);
    assert_eq!(metrics.search_sets_reused, 1);
    eprintln!(
        "search storage: {} logical entries; {} retained sets / {} entries",
        metrics.logical_search_entries,
        metrics.retained_search_sets,
        metrics.retained_search_entries
    );
}
