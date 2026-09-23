use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.UnrelatedPopulationReader");

#[test]
fn binding_helpers_do_not_activate_future_owner_writers_at_unrelated_types() {
    for case in [
        "bounded",
        "second_bounded_root",
        "unknown_creation",
        "unknown_transitive",
        "model_writer",
        "ownership_writer",
        "reference_scalar",
        "reparented",
        "pending_provider",
    ] {
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
        member(
            &mut f,
            if case == "reparented" { 2 } else { 10 },
            1,
            11,
            c::FEATURE_MEMBERSHIP,
        );
        if case == "second_bounded_root" {
            f.create(3, c::FEATURE_REFERENCE_EXPRESSION);
            member(&mut f, 2, 3, 12, c::FEATURE_MEMBERSHIP);
        }
        let snapshot = f.finish();
        let mut creator = ProducerFamily::FeatureReferenceExpression.descriptor(profile);
        let mut future = ProducerFamily::VariableFeaturing.descriptor(profile);
        match case {
            "unknown_creation" => creator.scoped_fresh_ownership = false,
            "model_writer" => future.scope = ProducerEffectScope::Model,
            "ownership_writer" => {
                future.effects.insert(ProducerEffect::Ownership);
            }
            "reference_scalar" => {
                future
                    .effects
                    .insert(ProducerEffect::Scalar(p::FEATURE_FEATURE_TARGET));
            }
            _ => {}
        }
        let mut reader = ProducerDescriptor::new(
            READER,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        );
        reader.scoped_fresh_ownership = case != "unknown_transitive";
        let registry = ProducerRegistry::new([creator.clone(), future.clone(), reader]).unwrap();
        let context = SemanticContext::for_project_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
            BTreeSet::new(),
            if case == "pending_provider" {
                BTreeSet::from([id(10)])
            } else {
                BTreeSet::new()
            },
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
        let q = KerMlQueries::for_production(context.fork());
        let answer = q.owned_relationships_of_type(id(2), c::FEATURE_MEMBERSHIP);
        assert_eq!(answer.completeness, Completeness::Complete);
        if !matches!(case, "reparented" | "second_bounded_root") {
            assert!(answer.value.is_empty());
        }
        let owner_answer = q.owned_relationships_of_type(id(10), c::FEATURE_MEMBERSHIP);
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
                                if (record.id() == id(1)
                                    || case == "second_bounded_root" && record.id() == id(3))
                                    && descriptor.id == creator.id
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
                            if record.id() == id(2) && descriptor.id == READER {
                                producer_reads(&answer, snapshot.model())
                            } else if record.id() == id(10) && descriptor.id == READER {
                                producer_reads(&owner_answer, snapshot.model())
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
            Some(if case == "bounded" {
                ProducerEvaluationState::EvaluatedComplete
            } else {
                ProducerEvaluationState::Pending
            }),
            "binding helper ownership case={case}"
        );
        if case != "reparented" {
            assert_eq!(
                certificate.evaluation(id(10), registry.index(READER).unwrap()),
                Some(ProducerEvaluationState::Pending),
                "the actual existing ancestor remains open: {case}"
            );
        }
        if case == "bounded" {
            let mut edit = snapshot.change_set();
            edit.set(
                id(10),
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(vec![]),
                origin(),
            );
            edit.set(
                id(2),
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(vec![Value::Reference(id(11))]),
                origin(),
            );
            let changed = snapshot.apply(&edit).unwrap();
            let changed_context = SemanticContext::for_snapshot(
                &changed,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
            let rebound = certificate
                .rebind(&context, &changed_context, &registry)
                .unwrap();
            assert_eq!(
                rebound
                    .certificate
                    .evaluation(id(2), registry.index(READER).unwrap()),
                Some(ProducerEvaluationState::Pending),
                "a reconstructed attachment root must reopen its former absence"
            );
        }
    }
}
