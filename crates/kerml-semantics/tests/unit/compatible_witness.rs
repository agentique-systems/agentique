use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;

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
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: PROFILE,
            ..Default::default()
        },
        BTreeSet::new(),
    )
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
