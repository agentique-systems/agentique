use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn closed(snapshot: &Snapshot) -> KerMlQueries<'_> {
    let registry = ProducerRegistry::new([]).unwrap();
    let context = SemanticContext::for_snapshot(snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(certificate.is_fully_closed(snapshot.model()));
    KerMlQueries::new(
        context
            .with_producer_closure(Arc::new(certificate))
            .unwrap(),
    )
}

fn fixture() -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    f.finish()
}

#[test]
fn closed_audit_retains_unrelated_changes_but_rejects_new_negative_search_matches() {
    let snapshot = fixture();
    let q = closed(&snapshot);
    let audit = ClosedAuditContext::new(&q).unwrap();
    let answer = q.direct_feature_types(id(1));
    assert!(answer.value.is_empty());
    assert_eq!(answer.completeness, Completeness::Complete);
    let mut reads = ClosedAuditReads::default();
    reads.subject(id(1));
    audit.observe(&mut reads, &answer);
    let old = audit.into_snapshot();
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    f.create(4, c::CLASSIFIER);
    let unrelated = f.finish();
    let q2 = closed(&unrelated);
    let audit2 = ClosedAuditContext::new(&q2).unwrap();
    let delta = audit2.delta(&old).unwrap();
    let reads2 = audit2
        .transport(&reads, &delta)
        .expect("unrelated addition preserves exact reads");
    let old2 = audit2.into_snapshot();
    let mut f = Fixture {
        changes: unrelated.change_set(),
        base: unrelated.clone(),
        owned: BTreeMap::new(),
    };
    f.create(5, c::FEATURE_TYPING);
    f.value(5, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    f.value(5, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    let changed = f.finish();
    assert_eq!(
        snapshot.model().element(id(1)),
        changed.model().element(id(1))
    );
    let q3 = closed(&changed);
    let audit3 = ClosedAuditContext::new(&q3).unwrap();
    assert!(!audit3.preserves(&reads2, &audit3.delta(&old2).unwrap()));
    assert!(
        !audit3.preserves(&reads, &delta),
        "delta must bind the receiving context"
    );
    assert_eq!(q3.direct_feature_types(id(1)).value, vec![id(2)]);
}

#[test]
fn closed_audit_reproves_closure_witnesses_and_preserves_real_global_reads() {
    let snapshot = fixture();
    let q = closed(&snapshot);
    let audit = ClosedAuditContext::new(&q).unwrap();
    let answer = q.producer_closure(id(1), SemanticClosureRequirement::EffectiveTyping);
    let mut witness = ClosedAuditReads::default();
    audit.observe(&mut witness, &answer);
    let mut global_answer = answer.clone();
    global_answer
        .search_dependencies
        .insert(SearchDependency::Kernel(
            agq_kernel::derived::StructuralSearch::Model,
        ));
    let mut global = ClosedAuditReads::default();
    audit.observe(&mut global, &global_answer);
    let mut incomplete = answer.clone();
    incomplete.completeness = Completeness::Incomplete;
    let mut failed = ClosedAuditReads::default();
    audit.observe(&mut failed, &incomplete);
    let old = audit.into_snapshot();
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    f.create(4, c::CLASSIFIER);
    let next = f.finish();
    let q2 = closed(&next);
    let audit2 = ClosedAuditContext::new(&q2).unwrap();
    let delta = audit2.delta(&old).unwrap();
    assert!(audit2.preserves(&witness, &delta));
    assert!(!audit2.preserves(&global, &delta));
    assert!(!audit2.preserves(&failed, &delta));
    let mut foreign = ClosedAuditReads::default();
    audit2.observe(&mut foreign, &answer);
    assert!(!audit2.preserves(&foreign, &delta));
}

#[test]
fn closed_audit_refuses_open_or_missing_writer_certificates() {
    let snapshot = fixture();
    let context =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    assert!(ClosedAuditContext::new(&KerMlQueries::new(context)).is_none());
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        ProducerFamilyId::new("Audit.FutureWriter"),
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    let q = KerMlQueries::new(
        context
            .with_producer_closure(Arc::new(certificate))
            .unwrap(),
    );
    assert!(ClosedAuditContext::new(&q).is_none());
}

#[test]
fn closed_audit_shared_dependency_signatures_keep_local_incoming_changes() {
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(fixture())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(10, c::FEATURE);
    let before = f.finish();
    let mut f = Fixture {
        changes: before.change_set(),
        base: before.clone(),
        owned: BTreeMap::new(),
    };
    f.create(11, c::FEATURE_TYPING);
    f.value(
        11,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(10)),
    );
    f.value(11, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    let after = f.finish();
    assert!(std::ptr::eq(
        before.model().element(id(2)).unwrap(),
        after.model().element(id(2)).unwrap()
    ));
    let changed = |dependency| {
        let left = crate::producer_closure::audit_subject_signatures(before.model(), dependency);
        let right = crate::producer_closure::audit_subject_signatures(after.model(), dependency);
        left.keys()
            .chain(right.keys())
            .copied()
            .filter(|id| left.get(id) != right.get(id))
            .collect::<BTreeSet<_>>()
    };
    let full = changed(None);
    let bounded = changed(Some(dependency.model()));
    assert!(
        bounded.contains(&id(2)),
        "new incoming carrier affects its shared standard endpoint"
    );
    assert_eq!(
        full, bounded,
        "factoring exact immutable bytes preserves this invalidation frontier"
    );
}

#[test]
fn closed_audit_negative_name_search_reopens_when_an_existing_member_is_renamed() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    f.value(2, p::ELEMENT_DECLARED_NAME, Value::String("before".into()));
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    let before = f.finish();
    let q = closed(&before);
    let answer = q.lookup_member(id(1), "after", MemberAccess::All);
    assert_eq!(answer.completeness, Completeness::Complete);
    assert!(answer.value.is_empty());
    let collector = ClosedAuditContext::new(&q).unwrap();
    let mut reads = ClosedAuditReads::default();
    reads.subject(id(1));
    collector.observe(&mut reads, &answer);
    let prior = collector.into_snapshot();

    let mut f = Fixture {
        changes: before.change_set(),
        base: before.clone(),
        owned: BTreeMap::new(),
    };
    f.value(2, p::ELEMENT_DECLARED_NAME, Value::String("after".into()));
    let after = f.finish();
    assert_eq!(
        before.model().element(id(1)),
        after.model().element(id(1)),
        "the lookup owner and membership population did not change"
    );
    let q = closed(&after);
    let answer = q.lookup_member(id(1), "after", MemberAccess::All);
    assert_eq!(answer.completeness, Completeness::Complete);
    assert_eq!(
        answer
            .value
            .iter()
            .map(|entry| entry.element)
            .collect::<Vec<_>>(),
        vec![id(2)]
    );
    let collector = ClosedAuditContext::new(&q).unwrap();
    assert!(
        collector
            .transport(&reads, &collector.delta(&prior).unwrap())
            .is_none(),
        "an earlier negative match cannot survive a consulted member's rename"
    );
}

#[test]
fn closed_audit_rejects_a_separate_equal_dependency_mount() {
    let registry = ProducerRegistry::new([]).unwrap();
    let overlay = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(fixture())
            .build()
            .unwrap(),
    );
    let context = SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    let context = context
        .with_producer_closure(Arc::new(certificate))
        .unwrap();
    let first = ProducerClosedDependency::new(overlay.clone(), &context, &registry).unwrap();
    let separate = ProducerClosedDependency::new(overlay.clone(), &context, &registry).unwrap();
    assert!(!Arc::ptr_eq(&first, &separate));
    assert_eq!(first.context(), separate.context());
    assert!(Arc::ptr_eq(first.overlay(), separate.overlay()));

    fn project<'a>(
        mount: &Arc<ProducerClosedDependency>,
        snapshot: &'a Snapshot,
        registry: &ProducerRegistry,
    ) -> KerMlQueries<'a> {
        let context = mount
            .project_context(snapshot, &[], BTreeSet::new(), BTreeSet::new())
            .unwrap();
        let certificate = ProducerClosureCertificate::initial(&context, registry).unwrap();
        KerMlQueries::new(
            context
                .with_producer_closure(Arc::new(certificate))
                .unwrap(),
        )
    }
    let snapshot = first.project_snapshot();
    let q = project(&first, &snapshot, &registry);
    let collector = ClosedAuditContext::new(&q).unwrap();
    let answer = q.direct_feature_types(id(1));
    assert_eq!(answer.completeness, Completeness::Complete);
    let mut reads = ClosedAuditReads::default();
    collector.observe(&mut reads, &answer);
    let prior = collector.into_snapshot();
    let same = ClosedAuditContext::new(&q).unwrap();
    assert!(
        same.transport(&reads, &same.delta(&prior).unwrap())
            .is_some()
    );

    let other_snapshot = separate.project_snapshot();
    let other = project(&separate, &other_snapshot, &registry);
    assert_eq!(answer.value, other.direct_feature_types(id(1)).value);
    let other = ClosedAuditContext::new(&other).unwrap();
    assert!(
        other.delta(&prior).is_none(),
        "equal publication/model identities cannot replace the exact factored mount"
    );
}

#[test]
fn closed_audit_rejects_static_query_and_writer_contract_changes_on_equal_records() {
    let snapshot = fixture();
    let q = closed(&snapshot);
    let prior = ClosedAuditContext::new(&q).unwrap().into_snapshot();
    let empty = ProducerRegistry::new([]).unwrap();
    let different = ProducerRegistry::new([ProducerDescriptor::new(
        ProducerFamilyId::new("Audit.DifferentContract"),
        [ProducerEffect::Typing],
        ProducerApplicability::Never,
    )])
    .unwrap();
    for (options, registry) in [
        (
            SemanticOptions {
                exclude_implied: true,
                ..Default::default()
            },
            &empty,
        ),
        (SemanticOptions::default(), &different),
    ] {
        let context = SemanticContext::for_snapshot(&snapshot, options, BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let certificate = ProducerClosureCertificate::initial(&context, registry).unwrap();
        assert!(certificate.is_fully_closed(snapshot.model()));
        let other = KerMlQueries::new(
            context
                .with_producer_closure(Arc::new(certificate))
                .unwrap(),
        );
        assert!(
            ClosedAuditContext::new(&other)
                .unwrap()
                .delta(&prior)
                .is_none()
        );
    }
}

#[test]
fn closed_audit_failed_subject_cannot_be_reused_after_later_successful_answers() {
    let snapshot = fixture();
    let q = closed(&snapshot);
    let collector = ClosedAuditContext::new(&q).unwrap();
    let complete = q.direct_feature_types(id(1));
    let failed = q.direct_feature_types(id(999));
    assert_eq!(complete.completeness, Completeness::Complete);
    assert_eq!(failed.completeness, Completeness::Invalid);
    let mut reads = ClosedAuditReads::default();
    reads.subject(id(1));
    collector.observe(&mut reads, &complete);
    collector.observe(&mut reads, &failed);
    collector.observe(&mut reads, &complete);
    let prior = collector.into_snapshot();
    let same = ClosedAuditContext::new(&q).unwrap();
    assert!(
        same.transport(&reads, &same.delta(&prior).unwrap())
            .is_none(),
        "a later successful query cannot erase an earlier failed audit check"
    );
}
