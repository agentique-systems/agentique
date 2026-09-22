use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::read_dependencies::{query_read_keys, structural_searches};
use agq_kernel::derived::{
    ComputationFailure, DerivationBuilder, IncompleteReason, StructuralSearch,
};

fn context(overlay: &agq_kernel::derived::DerivedOverlay) -> SemanticContext<'_> {
    SemanticContext::for_overlay(overlay, SemanticOptions::default(), BTreeSet::new()).unwrap()
}

#[test]
fn composed_fact_observation_retains_failed_and_absent_read_evidence() {
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    let snapshot = fixture.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, SemanticOptions::default(), BTreeSet::new())
            .unwrap(),
    );
    let property = FactKey::Property {
        element: id(1),
        property: p::TYPE_IS_CONJUGATED,
    };
    assert_eq!(
        q.canonical_fact_evidence(property).completeness,
        Completeness::Incomplete
    );
    let missing = q.canonical_fact_evidence(FactKey::Element(id(999)));
    assert_eq!(missing.completeness, Completeness::Complete);
    assert!(
        missing
            .search_dependencies
            .contains(&SearchDependency::Element(id(999)))
    );
    assert!(missing.fact_origins.is_empty());
    let mut builder = DerivationBuilder::new(snapshot);
    builder
        .failure(
            id(1),
            p::TYPE_IS_CONJUGATED,
            ComputationFailure::Incomplete {
                reason: IncompleteReason::MissingInput,
                explanation: agq_kernel::provenance::Explanation {
                    rule: RuleId::from_u128(987),
                    dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
                },
                searches: BTreeSet::from([StructuralSearch::Incoming(id(999))]),
            },
        )
        .unwrap();
    let overlay = builder.build().unwrap();
    let q = KerMlQueries::new(context(&overlay));
    let observed = q.canonical_fact_evidence(property);
    assert_eq!(observed.completeness, Completeness::Incomplete);
    assert!(observed.fact_origins.contains_key(&FactKey::Element(id(1))));
    assert!(observed.fact_origins.contains_key(&property));
    assert!(
        observed
            .search_dependencies
            .contains(&SearchDependency::Kernel(StructuralSearch::Incoming(id(
                999
            ))))
    );
    assert!(
        observed
            .canonical_dependencies
            .contains(&Dependency::Derived(property))
    );
    assert_eq!(observed.clone().map(|_| 42).map(|_| ()), observed);
}

#[test]
fn a_million_logical_search_entries_stay_shared_through_subquery_merges() {
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    let snapshot = fixture.finish();
    let slots: Vec<_> = snapshot
        .model()
        .element(id(1))
        .unwrap()
        .slots()
        .map(|(property, slot)| (property, slot.value().clone()))
        .collect();
    let searches: Arc<BTreeSet<_>> = Arc::new(
        (10_000..12_048)
            .map(|n| StructuralSearch::Element(id(n)))
            .collect(),
    );
    let mut builder = DerivationBuilder::new(snapshot);
    let mut facts = vec![];
    for index in 0..512 {
        let key = DerivationKey {
            rule: RuleId::from_u128(999),
            subject: id(1),
            output: OutputKey::from_u128(index),
        };
        let fact = FactKey::Element(key.element_id());
        builder.element(key, c::TYPE, slots.clone(), BTreeSet::new());
        builder.searches_shared(fact, searches.clone());
        facts.push(fact);
    }
    let overlay = builder.build().unwrap();
    let input = context(&overlay);
    let eager = KerMlQueries::new(input.fork());
    let producer = KerMlQueries::for_production(input);
    let mut public = eager.result(());
    let mut carried = producer.result(());
    for fact in facts {
        eager.fact(&mut public, fact);
        let mut subquery = producer.result(());
        producer.fact(&mut subquery, fact);
        carried.merge(subquery.clone());
        carried.merge(subquery);
    }
    assert!(carried.search_dependencies.is_empty());
    // Exactly one retained set, despite 512 outputs and two merges per output.
    assert_eq!(carried.shared_search_dependencies.iter().count(), 2048);
    assert_eq!(public.search_dependencies.len(), 2048);
    assert_eq!(structural_searches(&carried), structural_searches(&public));
    assert_eq!(
        query_read_keys(&carried, overlay.model()),
        query_read_keys(&public, overlay.model())
    );
    assert_eq!(
        carried.canonical_dependencies,
        public.canonical_dependencies
    );
    let mut public_receiver = eager.result(());
    public_receiver.merge(carried.clone());
    assert_eq!(
        public_receiver.search_dependencies,
        public.search_dependencies
    );
    assert_eq!(public_receiver.shared_search_dependencies.iter().count(), 0);
    carried.expand_search_dependencies();
    assert_eq!(carried.search_dependencies, public.search_dependencies);
    assert_eq!(carried.shared_search_dependencies.iter().count(), 0);
}

#[test]
fn private_result_equality_and_debug_do_not_depend_on_search_allocation_or_partition() {
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    let overlay = DerivationBuilder::new(fixture.finish()).build().unwrap();
    let q = KerMlQueries::for_production(context(&overlay));
    let searches = BTreeSet::from([
        StructuralSearch::Element(id(999)),
        StructuralSearch::Incoming(id(1)),
    ]);
    let mut a = q.result(());
    let mut b = q.result(());
    a.shared_search_dependencies
        .insert(&Arc::new(searches.clone()));
    for search in searches {
        b.shared_search_dependencies
            .insert(&Arc::new(BTreeSet::from([search])));
    }
    assert_eq!(a, b);
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    // Positional proofs stay producer-visible and must expand carried reads.
    for rule in [
        Rule::ParameterRedefinition,
        Rule::ResultRedefinition,
        Rule::EndRedefinition,
    ] {
        a.prove(QueryKind::RedefinedFeatures, id(1), id(2), rule, []);
    }
    b.expand_search_dependencies();
    let premises: Vec<_> = b
        .search_dependencies
        .iter()
        .cloned()
        .map(Evidence::Search)
        .collect();
    for rule in [
        Rule::ParameterRedefinition,
        Rule::ResultRedefinition,
        Rule::EndRedefinition,
    ] {
        b.prove(
            QueryKind::RedefinedFeatures,
            id(1),
            id(2),
            rule,
            premises.clone(),
        );
    }
    assert_eq!(a, b);
    b.search_dependencies
        .insert(SearchDependency::Kernel(StructuralSearch::Model));
    assert_ne!(a, b);
    a.clear_search_dependencies();
    assert_eq!(a.shared_search_dependencies.iter().count(), 0);
}

#[test]
fn status_read_sets_expand_every_search_and_remain_isolated_between_contexts() {
    let mut fixture = Fixture::new();
    fixture.create(1, c::PACKAGE);
    fixture.member(1, 10, 2, c::CLASS, "Base");
    fixture.member(1, 11, 3, c::CLASS, "Derived");
    relation(
        &mut fixture,
        3,
        2,
        20,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    fixture.value(20, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(3)));
    let snapshot = fixture.finish();
    let key = DerivationKey {
        rule: RuleId::from_u128(993),
        subject: id(3),
        output: OutputKey::from_u128(1),
    };
    let relationship = key.element_id();
    let slots: Vec<_> = snapshot
        .model()
        .element(id(20))
        .unwrap()
        .slots()
        .map(|(property, slot)| (property, slot.value().clone()))
        .collect();
    let searches = BTreeSet::from([
        StructuralSearch::Element(id(991)),
        StructuralSearch::Incoming(id(992)),
        StructuralSearch::Property {
            element: id(3),
            property: p::ELEMENT_DECLARED_NAME,
        },
        StructuralSearch::Association {
            element: id(3),
            association: snapshot
                .model()
                .registry()
                .property(p::SPECIALIZATION_GENERAL)
                .unwrap()
                .association
                .unwrap(),
        },
        StructuralSearch::DescriptorGraph,
    ]);
    let mut builder = DerivationBuilder::new(snapshot);
    builder.element(key, c::SPECIALIZATION, slots, BTreeSet::new());
    builder.extend_ordered_references(
        id(3),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![relationship],
        agq_kernel::provenance::Explanation {
            rule: key.rule,
            dependencies: BTreeSet::new(),
        },
    );
    builder.searches(FactKey::Element(relationship), searches.clone());
    let first = builder.build().unwrap();
    let public = KerMlQueries::new(context(&first));
    let status = public.status_queries();
    let mut builder = DerivationBuilder::from_overlay(first.clone());
    builder.searches(
        FactKey::Element(relationship),
        BTreeSet::from([StructuralSearch::Model]),
    );
    let second = builder.build().unwrap();
    let changed = KerMlStatusQueries::new(context(&second));
    assert_ne!(status.context(), changed.context());
    for name in ["Base", "Missing"] {
        let name = QualifiedName {
            absolute: false,
            segments: vec![name.into()],
        };
        let expected =
            public.lookup_relationship_target(relationship, p::SPECIALIZATION_GENERAL, &name);
        let actual = status.lookup_relationship_target_with_reads(
            relationship,
            p::SPECIALIZATION_GENERAL,
            &name,
        );
        assert_eq!(actual.outcome, QueryOutcome::from(expected.clone()));
        assert_eq!(
            actual.reads.search_dependencies(),
            &expected.search_dependencies
        );
        assert_eq!(
            actual.reads.canonical_dependencies(),
            &expected.canonical_dependencies
        );
        for search in &searches {
            assert!(
                actual
                    .reads
                    .search_dependencies()
                    .contains(&SearchDependency::Kernel(search.clone()))
            );
        }
        assert!(actual.reads.affected_by(&BTreeSet::from([id(991)]), false));
        assert!(actual.reads.affected_by(&BTreeSet::from([id(992)]), false));
        assert!(!actual.reads.reads_entire_model());
        let next = changed.lookup_relationship_target_with_reads(
            relationship,
            p::SPECIALIZATION_GENERAL,
            &name,
        );
        assert!(next.reads.reads_entire_model());
        assert_eq!(
            actual,
            status.lookup_relationship_target_with_reads(
                relationship,
                p::SPECIALIZATION_GENERAL,
                &name
            )
        );
    }
}

#[test]
fn deferred_searches_include_occurrence_navigation_and_failure_dependencies() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::ASSOCIATION);
    for (end, ty, membership, typing) in [(10, 20, 30, 40), (11, 21, 31, 41)] {
        f.create(end, c::FEATURE);
        f.create(ty, c::CLASSIFIER);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, end, membership, c::END_FEATURE_MEMBERSHIP);
        relation(
            &mut f,
            end,
            ty,
            typing,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            typing,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(end)),
        );
    }
    f.create(9, c::FEATURE);
    member(&mut f, 10, 9, 50, c::OWNING_MEMBERSHIP);
    let snapshot = f.finish();
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap(),
    );
    let overlay = q
        .plan_result_structure([id(10), id(9)])
        .materialize(&snapshot)
        .unwrap()
        .overlay;
    let link = overlay
        .model()
        .association_occurrences()
        .next()
        .unwrap()
        .id();
    let mut builder = DerivationBuilder::from_overlay(overlay);
    let missing = StructuralSearch::Element(id(777_001));
    builder.searches(
        FactKey::AssociationOccurrence(link),
        BTreeSet::from([missing.clone()]),
    );
    builder
        .failure(
            id(1),
            p::TYPE_IS_CONJUGATED,
            ComputationFailure::Incomplete {
                reason: IncompleteReason::MissingInput,
                explanation: agq_kernel::provenance::Explanation {
                    rule: RuleId::from_u128(987),
                    dependencies: BTreeSet::from([Dependency::Derived(
                        FactKey::AssociationOccurrence(link),
                    )]),
                },
                searches: BTreeSet::from([StructuralSearch::Incoming(id(777_002))]),
            },
        )
        .unwrap();
    let overlay = builder.build().unwrap();
    let context = SemanticContext::for_overlay(&overlay, options, BTreeSet::new()).unwrap();
    let public = KerMlQueries::new(context.fork());
    let producer = KerMlQueries::for_production(context);
    let relationship = public.owned_cross_subsetting(id(10)).value.unwrap();
    let mut expected = public.result(());
    let mut actual = producer.result(());
    for fact in [
        FactKey::Property {
            element: relationship,
            property: p::CROSS_SUBSETTING_CROSSED_FEATURE,
        },
        FactKey::Property {
            element: id(1),
            property: p::TYPE_IS_CONJUGATED,
        },
    ] {
        public.fact(&mut expected, fact);
        producer.fact(&mut actual, fact);
    }
    assert_eq!(
        query_read_keys(&actual, overlay.model()),
        query_read_keys(&expected, overlay.model())
    );
    assert_eq!(structural_searches(&actual), structural_searches(&expected));
    actual.expand_search_dependencies();
    assert_eq!(actual.search_dependencies, expected.search_dependencies);
    assert!(
        actual
            .search_dependencies
            .contains(&SearchDependency::Kernel(missing))
    );
    assert!(
        actual
            .search_dependencies
            .contains(&SearchDependency::Kernel(StructuralSearch::Incoming(id(
                777_002
            ))))
    );
}
