use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;

fn options() -> SemanticOptions {
    SemanticOptions {
        baseline_profile: PROFILE,
        ..Default::default()
    }
}

fn fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (element, class) in [
        (1, c::FEATURE),
        (2, c::FEATURE),
        (10, c::FEATURE),
        (50, c::FEATURE_REFERENCE_EXPRESSION),
    ] {
        f.create(element, class);
    }
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    f.finish()
}

#[test]
fn compatibility_nonempty_owner_does_not_wait_for_child_snapshot_membership() {
    let snapshot = fixture();
    let registry = ProducerRegistry::new([
        ProducerFamily::FeatureReferenceExpression.descriptor(PROFILE),
        ProducerFamily::VariableFeaturing.descriptor(PROFILE),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let q = KerMlQueries::for_production(context.fork());
    let mut answer = q.result(false);
    answer.value = q.compatible(&mut answer, id(1), id(10), &mut BTreeSet::new());
    assert!(!answer.value);
    assert_eq!(answer.completeness, Completeness::Complete);
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
        for descriptor in registry.descriptors() {
            if !descriptor
                .applicability
                .applies(snapshot.model(), record.metaclass())
            {
                continue;
            }
            table
                .record(
                    &[(
                        record.id(),
                        descriptor.id,
                        if record.id() == id(2)
                            && descriptor.id == ProducerFamily::VariableFeaturing.id()
                        {
                            Completeness::Incomplete
                        } else {
                            Completeness::Complete
                        },
                    )],
                    &registry,
                )
                .unwrap();
            table.record_reads(
                &[(
                    record.id(),
                    descriptor.id,
                    if record.id() == id(50)
                        && descriptor.id == ProducerFamily::FeatureReferenceExpression.id()
                    {
                        producer_reads(&answer, snapshot.model())
                    } else {
                        Vec::new().into()
                    },
                )],
                &registry,
            );
        }
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert_eq!(
        certificate.evaluation(
            id(50),
            registry
                .index(ProducerFamily::FeatureReferenceExpression.id())
                .unwrap()
        ),
        Some(ProducerEvaluationState::EvaluatedComplete),
        "an additional snapshot membership cannot make an already nonempty direct Feature population empty"
    );
}

#[test]
fn compatibility_nonempty_witness_preserves_specialization_and_empty_unknown_fallbacks() {
    let snapshot = fixture();
    let context = SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new()).unwrap();
    for production in [false, true] {
        let q = if production {
            KerMlQueries::for_production(context.fork())
        } else {
            KerMlQueries::new(context.fork())
        };
        for (left, right) in [(1, 10), (10, 1)] {
            let mut answer = q.result(false);
            answer.value = q.compatible(&mut answer, id(left), id(right), &mut BTreeSet::new());
            assert!(!answer.value);
            assert_eq!(answer.completeness, Completeness::Complete);
        }
        assert!(q.owned_feature_witness(id(1)).is_some());
        assert!(q.owned_feature_witness(id(10)).is_none());
    }

    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    subset(&mut f, 1, 10, 20);
    // Preserve the nonempty population while adding the specialization witness.
    f.owned.insert(
        id(1),
        vec![Value::Reference(id(3)), Value::Reference(id(20))],
    );
    let specialized = f.finish();
    let q = KerMlQueries::for_production(
        SemanticContext::for_snapshot(&specialized, options(), BTreeSet::new()).unwrap(),
    );
    assert!(q.owned_feature_witness(id(1)).is_some());
    let mut answer = q.result(false);
    answer.value = q.compatible(&mut answer, id(1), id(10), &mut BTreeSet::new());
    assert!(answer.value, "the earlier specialization branch still wins");
    assert_eq!(answer.completeness, Completeness::Complete);

    let mut edit = snapshot.change_set();
    edit.clear(id(1), p::ELEMENT_OWNED_RELATIONSHIP);
    let empty = snapshot.apply(&edit).unwrap();
    let q = KerMlQueries::for_production(
        SemanticContext::for_snapshot(&empty, options(), BTreeSet::new()).unwrap(),
    );
    assert!(q.owned_feature_witness(id(1)).is_none());
    let mut answer = q.result(false);
    answer.value = q.compatible(&mut answer, id(1), id(10), &mut BTreeSet::new());
    assert!(!answer.value);
    assert_eq!(answer.completeness, Completeness::Complete);
    assert!(producer_reads(&answer, empty.model()).contains(
        &crate::producer_closure::ProducerRead::Owned(id(1), c::MEMBERSHIP)
    ));

    let mut edit = snapshot.change_set();
    edit.clear(id(3), p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
    let unknown = snapshot.preview(&edit).unwrap();
    let q = KerMlQueries::for_production(
        SemanticContext::for_construction(&unknown, options(), BTreeSet::new()).unwrap(),
    );
    assert!(q.owned_feature_witness(id(1)).is_none());
    let mut answer = q.result(false);
    answer.value = q.compatible(&mut answer, id(1), id(10), &mut BTreeSet::new());
    assert_ne!(answer.completeness, Completeness::Complete);
}

#[test]
fn compatibility_selected_ownership_reopens_on_removal_or_member_retarget() {
    const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.CompatibilityReader");
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        READER,
        [],
        ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
    )])
    .unwrap();
    let snapshot = fixture();
    let context = SemanticContext::for_snapshot(&snapshot, options(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let q = KerMlQueries::for_production(context.fork());
    let mut answer = q.result(false);
    answer.value = q.compatible(&mut answer, id(1), id(10), &mut BTreeSet::new());
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(50), snapshot.model(), &registry);
    table
        .record(&[(id(50), READER, answer.completeness)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(50), READER, producer_reads(&answer, snapshot.model()))],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert_eq!(
        certificate.evaluation(id(50), registry.index(READER).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
    for remove in [false, true] {
        let mut edit = snapshot.change_set();
        if remove {
            edit.clear(id(1), p::ELEMENT_OWNED_RELATIONSHIP);
        } else {
            edit.set(
                id(3),
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(id(10))]),
                origin(),
            );
        }
        let changed = snapshot.apply(&edit).unwrap();
        let next = SemanticContext::for_snapshot(&changed, options(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let rebound = certificate.rebind(&context, &next, &registry).unwrap();
        assert_eq!(
            rebound
                .certificate
                .evaluation(id(50), registry.index(READER).unwrap()),
            Some(ProducerEvaluationState::Pending)
        );
    }
}
