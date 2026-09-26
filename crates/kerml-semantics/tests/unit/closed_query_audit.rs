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
