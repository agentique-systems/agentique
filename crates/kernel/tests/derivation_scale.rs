//! Generic publication-shaped workloads, without any language rule evaluator.
mod common;

use agq_kernel::derived::{DerivationBuilder, DerivationError, DerivedOverlay, StructuralSearch};
use agq_kernel::metamodel::*;
use agq_kernel::provenance::{Dependency, Explanation, ExplanationPool, FactKey, Origin};
use agq_kernel::*;
use common::{ELEMENT, MM, authored, class, model_descriptor, property};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const ASSOCIATION: AssociationId = AssociationId::from_u128(900);
const LEFT: PropertyId = PropertyId::from_u128(901);
const RIGHT: PropertyId = PropertyId::from_u128(902);
const RULE: RuleId = RuleId::from_u128(903);
const ROOT: ElementId = ElementId::from_u128(1);

fn base(count: usize) -> Snapshot {
    let mut properties = vec![];
    for (id, opposite, name) in [(LEFT, RIGHT, "left"), (RIGHT, LEFT, "right")] {
        let mut end = property(
            id,
            name,
            ELEMENT,
            ValueKind::Reference(ELEMENT),
            Multiplicity::MANY,
        );
        end.owner = PropertyOwner::Association(ASSOCIATION);
        end.association = Some(ASSOCIATION);
        end.opposite_ends.insert(opposite);
        properties.push(end);
    }
    let registry = MetamodelRegistry::from_descriptors(DescriptorSet {
        models: vec![model_descriptor()],
        classes: vec![class(ELEMENT, "Node", &[])],
        associations: vec![AssociationDescriptor {
            id: ASSOCIATION,
            name: "Link".into(),
            package: vec![],
            metamodel: MM,
            member_ends: vec![LEFT, RIGHT],
            navigable_owned_ends: BTreeSet::from([LEFT, RIGHT]),
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
        }],
        properties,
        ..Default::default()
    })
    .unwrap();
    let base = Snapshot::new(Arc::new(registry));
    let mut changes = base.change_set();
    for index in 1..=count {
        changes.create(ElementId::from_u128(index as u128), ELEMENT, authored());
    }
    base.apply(&changes).unwrap()
}

fn key(stage: u128, index: usize) -> DerivationKey {
    DerivationKey {
        subject: ROOT,
        rule: RULE,
        output: OutputKey::from_u128((stage << 64) | index as u128),
    }
}

fn proof(dependencies: impl IntoIterator<Item = Dependency>) -> Arc<Explanation> {
    Arc::new(Explanation {
        rule: RULE,
        dependencies: dependencies.into_iter().collect(),
    })
}

#[test]
fn sixty_thousand_subjects_share_proofs_across_three_additive_frontiers() {
    let declared_count = 60_000;
    let first_count = 20_000;
    let second_count = 10_000;
    let base = base(declared_count);
    let first_proof = proof(
        base.model()
            .elements()
            .map(|record| Dependency::Declared(FactKey::Element(record.id()))),
    );
    let mut first = DerivationBuilder::new(base.clone());
    for index in 0..first_count {
        let output = key(1, index);
        first.element_with_explanation(output, ELEMENT, [], first_proof.clone());
        first.searches(
            FactKey::Element(output.element_id()),
            BTreeSet::from([
                StructuralSearch::Incoming(output.element_id()),
                StructuralSearch::Element(key(99, index).element_id()),
            ]),
        );
    }
    let first = first.build().unwrap();
    assert_eq!(first.build_metrics().proof_sets_interned, 1);
    assert_eq!(first.build_metrics().proof_sets_reused, first_count - 1);
    assert_eq!(
        first.build_metrics().dependency_edges_considered,
        declared_count
    );
    let second_proof = proof(
        std::iter::once(Dependency::Declared(FactKey::Element(ROOT))).chain(
            (0..first_count)
                .map(|index| Dependency::Derived(FactKey::Element(key(1, index).element_id()))),
        ),
    );
    // The caller prepares against a borrowed immutable frontier and releases it
    // before handing the entire input into the next atomic materialization.
    let mut second = DerivationBuilder::new(first.declared().clone());
    for index in 0..second_count {
        second.element_with_explanation(key(2, index), ELEMENT, [], second_proof.clone());
    }
    let second = second.build_on_overlay(first).unwrap();
    assert!(second.build_metrics().reused_owned_storage);
    assert_eq!(second.build_metrics().existing_facts_reused, first_count);
    assert_eq!(second.build_metrics().proof_sets_interned, 1);
    assert_eq!(second.build_metrics().proof_sets_reused, second_count - 1);
    assert_eq!(
        second.build_metrics().dependency_edges_considered,
        first_count + 1
    );
    let link_proof = proof(
        std::iter::once(Dependency::Declared(FactKey::Element(ROOT)))
            .chain(second.facts().map(|(fact, _)| Dependency::Derived(fact))),
    );
    let mut third = DerivationBuilder::from_overlay(second);
    for index in 0..first_count {
        third.association_occurrence_with_explanation(
            key(3, index),
            ASSOCIATION,
            BTreeMap::from([
                (LEFT, key(1, index).element_id()),
                (RIGHT, key(2, index % second_count).element_id()),
            ]),
            BTreeMap::new(),
            link_proof.clone(),
        );
    }
    let third = third.build().unwrap();
    assert!(third.build_metrics().reused_owned_storage);
    assert_eq!(
        third.build_metrics().new_association_occurrences,
        first_count
    );
    assert_eq!(third.build_metrics().proof_sets_interned, 1);
    assert_eq!(third.build_metrics().proof_sets_reused, first_count - 1);
    assert_eq!(
        third.build_metrics().dependency_edges_considered,
        first_count + second_count + 1
    );
    assert_eq!(
        third.model().elements().count(),
        declared_count + first_count + second_count
    );
    assert_eq!(third.model().association_occurrences().count(), first_count);
    assert_eq!(third.model().computation_searches().count(), first_count);
    assert_eq!(base.model().elements().count(), declared_count);
    assert_eq!(base.model().association_occurrences().count(), 0);
    for record in third.model().association_occurrences() {
        let Origin::Derived(stored) = record.origin() else {
            panic!("derived occurrence lost its evidence")
        };
        assert!(Arc::ptr_eq(stored, &link_proof));
    }
    eprintln!(
        "scale: {declared_count} declared, {} derived Elements, {first_count} occurrences, 3 frontiers, 3 shared proofs, {} checked dependency edges",
        first_count + second_count,
        declared_count + 2 * first_count + second_count + 2,
    );
}

fn compare(left: &DerivedOverlay, right: &DerivedOverlay) {
    assert!(left.model().elements().eq(right.model().elements()));
    assert!(
        left.model()
            .association_occurrences()
            .eq(right.model().association_occurrences())
    );
    assert!(left.facts().eq(right.facts()));
    assert!(
        left.model()
            .computation_searches()
            .eq(right.model().computation_searches())
    );
    for record in left.model().elements() {
        let id = record.id();
        assert!(left.model().incoming(id).eq(right.model().incoming(id)));
        assert!(left.model().outgoing(id).eq(right.model().outgoing(id)));
        assert_eq!(
            left.model().navigation_slot(id, LEFT),
            right.model().navigation_slot(id, LEFT)
        );
        assert_eq!(
            left.model().navigation_slot(id, RIGHT),
            right.model().navigation_slot(id, RIGHT)
        );
    }
}

#[test]
fn owned_shared_and_one_batch_construction_have_identical_semantic_graphs() {
    let unrelated = base(1);
    let base = base(8);
    let enqueue = |builder: &mut DerivationBuilder, index: usize| {
        let key = key(1, index);
        builder.element(key, ELEMENT, [], BTreeSet::new());
        builder.association_occurrence(
            key,
            ASSOCIATION,
            BTreeMap::from([(LEFT, ROOT), (RIGHT, key.element_id())]),
            BTreeMap::new(),
            BTreeSet::new(),
        );
        builder.searches(
            FactKey::Element(key.element_id()),
            BTreeSet::from([StructuralSearch::Incoming(ROOT)]),
        );
    };
    let mut all = DerivationBuilder::new(base.clone());
    for index in 0..8 {
        enqueue(&mut all, index);
    }
    let all = all.build().unwrap();
    let mut staged = DerivationBuilder::new(base.clone()).build().unwrap();
    let mut retained = vec![];
    for index in (0..8).rev() {
        if index % 2 == 0 {
            retained.push(staged.clone());
        }
        let mut next = DerivationBuilder::new(base.clone());
        enqueue(&mut next, index);
        staged = next.build_on_overlay(staged).unwrap();
        assert_eq!(staged.build_metrics().reused_owned_storage, index % 2 != 0);
    }
    compare(&all, &staged);
    for (index, old) in retained.iter().enumerate() {
        assert_eq!(old.model().association_occurrences().count(), 2 * index + 1);
    }
    assert!(matches!(
        DerivationBuilder::new(unrelated).build_on_overlay(staged),
        Err(DerivationError::InputContextMismatch)
    ));
}

#[test]
fn proof_pool_counts_content_reuse_and_preserves_canonical_allocation_after_clone() {
    let evidence = proof([Dependency::Declared(FactKey::Element(ROOT))]);
    let mut pool = ExplanationPool::default();
    let canonical = pool.intern_shared(evidence.clone());
    assert!(Arc::ptr_eq(
        &pool.intern_shared(evidence.clone()),
        &canonical
    ));
    let equal = Arc::new(evidence.as_ref().clone());
    assert!(Arc::ptr_eq(&pool.intern_shared(equal), &canonical));
    assert_eq!(pool.statistics().interned, 1);
    assert_eq!(pool.statistics().reused, 2);
    let mut copied = pool.clone();
    drop(pool);
    drop(evidence);
    assert!(Arc::ptr_eq(
        &copied.intern_shared(canonical.clone()),
        &canonical
    ));
    assert_eq!(copied.statistics().reused, 3);
}
