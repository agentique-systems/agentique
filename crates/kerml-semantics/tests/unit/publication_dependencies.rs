use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::read_dependencies::InvalidationKey;

fn plan(
    snapshot: &Snapshot,
    descriptors: Vec<ProducerDescriptor>,
    edges: Vec<PublicationDependency>,
    reads: Vec<(ElementId, QueryReadSet)>,
) -> PublicationDependencyPlan {
    let registry = ProducerRegistry::new(descriptors).unwrap();
    let context =
        SemanticContext::for_snapshot(snapshot, SemanticOptions::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    PublicationDependencyPlan::build(
        &context,
        &registry,
        snapshot
            .model()
            .elements()
            .map(|record| record.id())
            .collect(),
        edges,
        reads,
    )
}

fn nodes(count: u128) -> Snapshot {
    let mut f = Fixture::new();
    for n in 0..count {
        f.create(n, c::CLASSIFIER);
    }
    f.finish()
}

fn edge(consumer: u128, provider: u128) -> PublicationDependency {
    PublicationDependency {
        consumer: id(consumer),
        provider: id(provider),
        reason: PublicationDependencyReason::MandatoryReference,
    }
}

fn reads(keys: impl IntoIterator<Item = InvalidationKey>) -> QueryReadSet {
    QueryReadSet {
        canonical_dependencies: BTreeSet::new(),
        search_dependencies: BTreeSet::new(),
        keys: keys.into_iter().collect(),
    }
}

#[test]
fn linear_diamond_and_mutual_dependencies_have_provider_first_components() {
    let snapshot = nodes(5);
    let plan = plan(
        &snapshot,
        vec![],
        vec![
            edge(1, 0),
            edge(2, 0),
            edge(3, 1),
            edge(3, 2),
            edge(4, 3),
            edge(3, 4),
        ],
        vec![],
    );
    assert_eq!(plan.components().len(), 4);
    assert_eq!(plan.components()[0].subjects(), &BTreeSet::from([id(0)]));
    assert_eq!(
        plan.components()[3].subjects(),
        &BTreeSet::from([id(3), id(4)])
    );
    assert_eq!(plan.components()[3].dependencies(), &BTreeSet::from([1, 2]));
    assert!(plan.diagnostics().is_empty());
    let legal = vec![
        BTreeSet::from([id(0)]),
        BTreeSet::from([id(2)]),
        BTreeSet::from([id(1)]),
        BTreeSet::from([id(3), id(4)]),
    ];
    assert_eq!(plan.validate_partition(&legal), Ok(()));
    let illegal = vec![
        BTreeSet::from([id(1)]),
        BTreeSet::from([id(0)]),
        BTreeSet::from([id(2)]),
        BTreeSet::from([id(3), id(4)]),
    ];
    assert!(matches!(
        plan.validate_partition(&illegal),
        Err(PublicationPartitionError::ProviderAfterConsumer { .. })
    ));
}

#[test]
fn graph_input_permutations_preserve_plan_identity() {
    let snapshot = nodes(8);
    let original = vec![
        edge(1, 0),
        edge(2, 0),
        edge(3, 1),
        edge(3, 2),
        edge(5, 4),
        edge(4, 5),
        edge(6, 3),
        edge(7, 6),
    ];
    let baseline = plan(&snapshot, vec![], original.clone(), vec![]);
    // Deterministic pseudo-random input permutations, with duplicate edges too.
    let mut state = 31_u64;
    for _ in 0..32 {
        let mut edges = original.clone();
        for i in (1..edges.len()).rev() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            edges.swap(i, (state as usize) % (i + 1));
        }
        edges.push(edges[0].clone());
        let permuted = plan(&snapshot, vec![], edges, vec![]);
        assert_eq!(baseline.digest(), permuted.digest());
        assert_eq!(baseline.components(), permuted.components());
    }
}

#[test]
fn canonical_typing_and_subsetting_keep_relationship_carriers() {
    let mut f = Fixture::new();
    f.create(0, c::FEATURE);
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    subset(&mut f, 0, 1, 3);
    relation(&mut f, 0, 2, 4, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(4, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(0)));
    let snapshot = f.finish();
    let plan = plan(&snapshot, vec![], vec![], vec![]);
    assert!(
        plan.dependencies()
            .iter()
            .any(|edge| edge.consumer == id(3) && edge.provider == id(1))
    );
    assert!(
        plan.dependencies()
            .iter()
            .any(|edge| edge.consumer == id(4) && edge.provider == id(2))
    );
    // The general subjects do not gain a write permission or reverse read edge
    // merely because a locally owned relationship names them.
    assert!(
        !plan
            .dependencies()
            .iter()
            .any(|edge| [id(1), id(2)].contains(&edge.consumer) && edge.provider == id(0))
    );
}

#[test]
fn later_relationship_writer_cannot_split_the_required_carrier() {
    let mut f = Fixture::new();
    f.create(0, c::CLASSIFIER);
    f.create(1, c::FEATURE);
    member(&mut f, 0, 1, 2, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let mut writer = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.OwningType"),
        [ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::SubjectAndOwningType;
    writer.scoped_fresh_ownership = true;
    let plan = plan(&snapshot, vec![writer], vec![], vec![(id(1), reads([]))]);
    assert_eq!(plan.component_of(id(0)), plan.component_of(id(1)));
    assert!(plan.dependencies().iter().any(|edge| edge.consumer == id(0)
        && edge.provider == id(1)
        && matches!(edge.reason, PublicationDependencyReason::Writer { .. })));
    assert!(matches!(
        plan.validate_partition(&[BTreeSet::from([id(0), id(2)]), BTreeSet::from([id(1)])]),
        Err(PublicationPartitionError::SplitSemanticCycle { .. })
    ));
}

#[test]
fn missing_and_negative_provider_reads_remain_conservative() {
    let snapshot = nodes(3);
    let writer = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.Writer"),
        [ProducerEffect::Membership],
        ProducerApplicability::Any,
    );
    let missing = plan(&snapshot, vec![writer.clone()], vec![], vec![]);
    assert!(
        missing
            .diagnostics()
            .contains(&PublicationPlanDiagnostic::MissingProviderEvidence(id(0)))
    );
    assert_eq!(missing.components().len(), 1);
    let negative = plan(
        &snapshot,
        vec![writer],
        vec![],
        vec![
            (id(0), reads([InvalidationKey::Incoming(id(1))])),
            (id(1), reads([])),
            (id(2), reads([])),
        ],
    );
    assert!(
        negative
            .diagnostics()
            .contains(&PublicationPlanDiagnostic::GlobalProviderRead(id(0)))
    );
    assert!(negative.writers().iter().all(|writer| {
        writer
            .global_requirements
            .contains(&SemanticClosureRequirement::EffectiveMembership)
    }));
    assert_eq!(negative.components().len(), 1);
}

#[test]
fn future_subject_can_activate_a_previously_inapplicable_writer() {
    let snapshot = nodes(2);
    let mut creator = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.Creator"),
        [],
        ProducerApplicability::Any,
    );
    creator.fresh_effects.insert(ProducerEffect::Membership);
    creator.scope = ProducerEffectScope::Subject;
    // An unbounded fresh attachment must never be narrowed to observed output.
    let mut late = ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.Late"),
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
    );
    late.scope = ProducerEffectScope::SubjectAndOwners;
    let plan = plan(
        &snapshot,
        vec![creator, late],
        vec![],
        vec![(id(0), reads([])), (id(1), reads([]))],
    );
    assert_eq!(plan.applicable_pairs(), 2);
    assert!(plan.writers().iter().any(|writer| writer.future_subject
        && writer.family.name() == "Fixture.Late"
        && writer.targets.is_none()));
    assert_eq!(plan.components().len(), 1);
}

#[test]
fn unknown_provider_and_omitted_candidate_are_explicit_findings() {
    let snapshot = nodes(2);
    let unknown = plan(&snapshot, vec![], vec![edge(0, 99)], vec![]);
    assert!(
        unknown
            .diagnostics()
            .contains(&PublicationPlanDiagnostic::UnknownProvider {
                consumer: id(0),
                provider: id(99)
            })
    );
    let registry = ProducerRegistry::new([]).unwrap();
    let context =
        SemanticContext::for_snapshot(&snapshot, SemanticOptions::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    let missing =
        PublicationDependencyPlan::build(&context, &registry, BTreeSet::from([id(0)]), [], []);
    assert!(
        missing
            .diagnostics()
            .contains(&PublicationPlanDiagnostic::MissingCandidateSubject(id(1)))
    );
}

#[test]
fn deep_dependency_graph_uses_no_recursive_traversal() {
    let ids: Vec<_> = (0..12_000).map(id).collect();
    let mut adjacency = vec![BTreeSet::new(); ids.len()];
    for (i, neighbors) in adjacency.iter_mut().enumerate().skip(1) {
        neighbors.insert(i - 1);
    }
    let components = super::components(&ids, &adjacency);
    assert_eq!(components.len(), ids.len());
    assert_eq!(
        components.last().unwrap().subjects(),
        &BTreeSet::from([id(11_999)])
    );
}

#[test]
fn immutable_dependency_requires_authenticated_closure() {
    let overlay = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(nodes(2))
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(overlay.clone());
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(10, c::CLASSIFIER);
    let snapshot = f.finish();
    let registry = ProducerRegistry::new([]).unwrap();
    let mut context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    // Neither physical protection nor a caller-supplied digest is authority.
    context.id.publication_dependency_digest = Some([7; 32]);
    let unproven = PublicationDependencyPlan::build(
        &context,
        &registry,
        BTreeSet::from([id(10)]),
        [edge(10, 0)],
        [],
    );
    assert!(
        unproven
            .diagnostics()
            .contains(&PublicationPlanDiagnostic::MissingCandidateSubject(id(0)))
    );
    assert!(
        unproven
            .diagnostics()
            .contains(&PublicationPlanDiagnostic::UnknownProvider {
                consumer: id(10),
                provider: id(0)
            })
    );

    let context = SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    let context = context.with_producer_closure(certificate).unwrap();
    let dependency = ProducerClosedDependency::new(overlay.clone(), &context, &registry).unwrap();
    let context = dependency
        .project_context(&snapshot, &[id(10)], BTreeSet::new(), BTreeSet::new())
        .unwrap();
    let proven = PublicationDependencyPlan::build(
        &context,
        &registry,
        BTreeSet::from([id(10)]),
        [edge(10, 0)],
        [],
    );
    assert!(proven.diagnostics().is_empty());
    assert_eq!(proven.components().len(), 1);
}
