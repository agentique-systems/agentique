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

fn closed_overlay(overlay: &agq_kernel::derived::DerivedOverlay) -> KerMlQueries<'_> {
    let registry = ProducerRegistry::new([]).unwrap();
    let context = SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(certificate.is_fully_closed(overlay.model()));
    KerMlQueries::new(
        context
            .with_producer_closure(Arc::new(certificate))
            .unwrap(),
    )
}

#[test]
fn closed_audit_ignores_unread_outputs_but_rejects_changed_read_output_evidence() {
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    fixture.create(2, c::TYPE);
    let snapshot = fixture.finish();
    let key = DerivationKey {
        rule: RuleId::from_u128(44101),
        subject: id(1),
        output: OutputKey::from_u128(1),
    };
    let output = FactKey::Element(key.element_id());
    let derive = |name: Option<&str>, extra_proof: bool, search: u128| {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        if let Some(name) = name {
            let mut slots: Vec<_> = snapshot
                .model()
                .element(id(2))
                .unwrap()
                .slots()
                .filter(|(property, _)| *property != p::ELEMENT_DECLARED_NAME)
                .map(|(property, slot)| (property, slot.value().clone()))
                .collect();
            slots.push((
                p::ELEMENT_DECLARED_NAME,
                SlotValue::Scalar(Value::String(name.into())),
            ));
            let mut proof = BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]);
            if extra_proof {
                proof.insert(Dependency::Declared(FactKey::Element(id(2))));
            }
            builder.element(key, c::TYPE, slots, proof);
            builder.searches_shared(
                output,
                Arc::new(BTreeSet::from([StructuralSearch::ElementIdentity(id(
                    search,
                ))])),
            );
        }
        builder.build().unwrap()
    };
    let old = derive(Some("old output"), false, 99);
    let old_queries = closed_overlay(&old);
    let collector = ClosedAuditContext::new(&old_queries).unwrap();
    let anchor_answer = old_queries.canonical_fact_evidence(FactKey::Element(id(1)));
    let output_answer = old_queries.canonical_fact_evidence(output);
    assert_eq!(anchor_answer.completeness, Completeness::Complete);
    assert_eq!(output_answer.completeness, Completeness::Complete);
    assert!(
        output_answer
            .canonical_dependencies
            .contains(&Dependency::Derived(output))
    );
    let mut anchor_reads = ClosedAuditReads::default();
    collector.observe(&mut anchor_reads, &anchor_answer);
    let mut output_reads = ClosedAuditReads::default();
    collector.observe(&mut output_reads, &output_answer);
    let previous = collector.into_snapshot();
    for (name, extra_proof, search, change) in [
        (None, false, 99, "lost output"),
        (Some("new output"), false, 99, "changed output value"),
        (Some("old output"), true, 99, "changed output proof"),
        (Some("old output"), false, 100, "changed output search"),
    ] {
        let next = derive(name, extra_proof, search);
        let queries = closed_overlay(&next);
        let collector = ClosedAuditContext::new(&queries).unwrap();
        let delta = collector.delta(&previous).unwrap();
        assert!(
            collector.preserves(&anchor_reads, &delta),
            "{change} cannot invalidate a read of an unchanged proof input alone"
        );
        assert!(
            !collector.preserves(&output_reads, &delta),
            "{change} must invalidate a query that actually read the output"
        );
        let actual = queries.canonical_fact_evidence(FactKey::Element(id(1)));
        let mut expected = anchor_answer.clone();
        expected.context = actual.context.clone();
        assert_eq!(actual, expected, "{change}: anchor evidence stays exact");
    }
    // In the reverse direction, a newly produced detached consumer of this
    // input also must not invalidate the input's unrelated successful audit.
    let empty = derive(None, false, 99);
    let queries = closed_overlay(&empty);
    let collector = ClosedAuditContext::new(&queries).unwrap();
    let mut reads = ClosedAuditReads::default();
    collector.observe(
        &mut reads,
        &queries.canonical_fact_evidence(FactKey::Element(id(1))),
    );
    let previous = collector.into_snapshot();
    let collector = ClosedAuditContext::new(&old_queries).unwrap();
    assert!(collector.preserves(&reads, &collector.delta(&previous).unwrap()));
}

#[test]
fn closed_audit_tracks_transitive_positive_inputs_beneath_equal_derived_roots() {
    use agq_kernel::derived::DerivationBuilder;
    let mut fixture = Fixture::new();
    fixture.create(1, c::TYPE);
    fixture.create(2, c::TYPE);
    fixture.value(1, p::ELEMENT_DECLARED_NAME, Value::String("before".into()));
    let before = fixture.finish();
    let mut fixture = Fixture {
        changes: before.change_set(),
        base: before.clone(),
        owned: BTreeMap::new(),
    };
    fixture.value(1, p::ELEMENT_DECLARED_NAME, Value::String("after".into()));
    let after = fixture.finish();
    let slots: Vec<_> = before
        .model()
        .element(id(2))
        .unwrap()
        .slots()
        .map(|(property, slot)| (property, slot.value().clone()))
        .collect();
    let inner = DerivationKey {
        rule: RuleId::from_u128(44102),
        subject: id(1),
        output: OutputKey::from_u128(1),
    };
    let outer = DerivationKey {
        output: OutputKey::from_u128(2),
        ..inner
    };
    let root = FactKey::Element(outer.element_id());
    let derive = |snapshot: Snapshot| {
        let mut builder = DerivationBuilder::new(snapshot);
        builder.element(
            inner,
            c::TYPE,
            slots.clone(),
            BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
        );
        builder.element(
            outer,
            c::TYPE,
            slots.clone(),
            BTreeSet::from([Dependency::Derived(FactKey::Element(inner.element_id()))]),
        );
        builder.build().unwrap()
    };
    let old = derive(before);
    let next = derive(after);
    assert_eq!(
        old.model().element(outer.element_id()),
        next.model().element(outer.element_id()),
        "the immediate derived root and its provenance IDs are exactly equal"
    );
    let old_queries = closed_overlay(&old);
    let answer = old_queries.canonical_fact_evidence(root);
    assert_eq!(answer.completeness, Completeness::Complete);
    assert_eq!(
        answer.canonical_dependencies,
        BTreeSet::from([Dependency::Derived(root)])
    );
    assert!(
        answer
            .positive_dependencies
            .contains(&FactKey::Element(id(1)))
    );
    let collector = ClosedAuditContext::new(&old_queries).unwrap();
    let mut reads = ClosedAuditReads::default();
    collector.observe(&mut reads, &answer);
    let previous = collector.into_snapshot();
    let queries = closed_overlay(&next);
    let collector = ClosedAuditContext::new(&queries).unwrap();
    assert!(
        !collector.preserves(&reads, &collector.delta(&previous).unwrap()),
        "the transitive positive input changed even though the root is equal"
    );
    let producer = KerMlQueries::for_production(old_queries.context.fork());
    let mut mixed = old_queries.canonical_fact_evidence(FactKey::Element(id(2)));
    mixed
        .merge_evidence(producer.canonical_fact_evidence(root))
        .unwrap();
    assert!(
        !mixed.producer_evidence,
        "ordinary materialization mode remains unchanged"
    );
    assert!(
        mixed.contains_compact_evidence,
        "compact evidence taint must survive public merge"
    );
    assert!(
        !mixed
            .positive_dependencies
            .contains(&FactKey::Element(id(1)))
    );
    let old_collector = ClosedAuditContext::new(&old_queries).unwrap();
    let mut mixed_reads = ClosedAuditReads::default();
    old_collector.observe(&mut mixed_reads, &mixed);
    assert!(
        !collector.preserves(&mixed_reads, &collector.delta(&previous).unwrap()),
        "an ordinary wrapper cannot authenticate a compact proof missing a changed transitive input"
    );
}

#[test]
fn closed_audit_refuses_compact_producer_proofs_without_full_positive_expansion() {
    let snapshot = fixture();
    let ordinary = closed(&snapshot);
    let producer = KerMlQueries::for_production(ordinary.context.fork());
    let answer = producer.canonical_fact_evidence(FactKey::Element(id(1)));
    assert_eq!(answer.completeness, Completeness::Complete);
    assert!(answer.producer_evidence);
    let collector = ClosedAuditContext::new(&ordinary).unwrap();
    let mut reads = ClosedAuditReads::default();
    collector.observe(&mut reads, &answer);
    let previous = collector.into_snapshot();
    let collector = ClosedAuditContext::new(&ordinary).unwrap();
    assert!(
        !collector.preserves(&reads, &collector.delta(&previous).unwrap()),
        "producer-mode proof edges cannot establish all positive audit reads"
    );
}

#[cfg(feature = "verification")]
#[test]
fn closed_audit_verification_trace_matches_rejections_and_caps_samples() {
    let snapshot = fixture();
    let q = closed(&snapshot);
    let previous = ClosedAuditContext::new(&q).unwrap().into_snapshot();
    let collector = ClosedAuditContext::new(&q).unwrap();
    let mut delta = collector.delta(&previous).unwrap();
    let mut reads = ClosedAuditReads::default();
    reads.subject(id(1));
    let trace = |reads: &ClosedAuditReads, delta: &ClosedAuditDelta| {
        let diagnostic = collector.verification_reuse_trace(&previous, Some(reads), Some(delta));
        assert_eq!(
            diagnostic["reason"] == "preserved",
            collector.preserves(reads, delta)
        );
        diagnostic
    };
    assert_eq!(trace(&reads, &delta)["reason"], "preserved");
    delta.affected.insert(id(1));
    assert_eq!(trace(&reads, &delta)["reason"], "bounded_read_changed");
    reads.global = true;
    assert_eq!(trace(&reads, &delta)["reason"], "global_search_changed");
    reads.invalid = true;
    assert_eq!(trace(&reads, &delta)["reason"], "invalid_receipt");
    reads.invalid = false;
    reads.global = false;
    for n in 1000..1100 {
        reads.subject(id(n));
        delta.affected.insert(id(n));
    }
    let diagnostic = trace(&reads, &delta);
    assert_eq!(diagnostic["changed_reads"], 101);
    assert_eq!(
        diagnostic["changed_read_sample"].as_array().unwrap().len(),
        8
    );
    assert_eq!(
        collector.verification_reuse_trace(&previous, None, Some(&delta))["reason"],
        "no_successful_prior_subject"
    );
    let mut wrong_mount = previous.clone();
    wrong_mount.dependency = Some(std::sync::Weak::new());
    assert!(collector.delta(&wrong_mount).is_none());
    assert_eq!(
        collector.verification_reuse_trace(&wrong_mount, Some(&reads), None)["reason"],
        "dependency_mount_mismatch"
    );
    let mut wrong_contract = previous;
    wrong_contract.context.options.exclude_implied = true;
    assert!(collector.delta(&wrong_contract).is_none());
    assert_eq!(
        collector.verification_reuse_trace(&wrong_contract, Some(&reads), None)["reason"],
        "kerml_static_contract_mismatch"
    );
}
