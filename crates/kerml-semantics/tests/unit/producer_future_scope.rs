use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.UnrelatedPopulationReader");

#[test]
fn binding_helpers_do_not_activate_future_owner_writers_at_unrelated_types() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::FEATURE_REFERENCE_EXPRESSION);
    f.create(2, c::CLASSIFIER);
    f.create(10, c::CLASSIFIER);
    member(&mut f, 10, 1, 11, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let creator = ProducerFamily::FeatureReferenceExpression.descriptor(profile);
    let future = ProducerFamily::VariableFeaturing.descriptor(profile);
    let registry = ProducerRegistry::new([
        creator.clone(),
        future.clone(),
        ProducerDescriptor::new(
            READER,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        ),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let q = KerMlQueries::for_production(context.fork());
    let answer = q.owned_relationships_of_type(id(2), c::FEATURE_MEMBERSHIP);
    assert_eq!(answer.completeness, Completeness::Complete);
    assert!(answer.value.is_empty());
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
        for descriptor in registry.descriptors() {
            if descriptor
                .applicability
                .applies(snapshot.model(), record.metaclass())
            {
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if record.id() == id(1) && descriptor.id == creator.id {
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
                        if record.id() == id(2) && descriptor.id == READER {
                            producer_reads(&answer, snapshot.model())
                        } else {
                            Vec::new().into()
                        },
                    )],
                    &registry,
                );
            }
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
        certificate.evaluation(id(2), registry.index(READER).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete),
        "binding helpers are owned below expression 1; a hypothetical helper's owner writer cannot add members to unrelated Classifier 2"
    );
}
