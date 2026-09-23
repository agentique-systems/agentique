use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::ProducerEvaluationTable;

fn fixture(default: bool) -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    for offset in [0, 10] {
        f.create(2 + offset, c::EXPRESSION);
        f.create(3 + offset, c::FEATURE);
        member(
            &mut f,
            2 + offset,
            3 + offset,
            5 + offset,
            c::RETURN_PARAMETER_MEMBERSHIP,
        );
        member(&mut f, 1, 2 + offset, 4 + offset, c::FEATURE_VALUE);
        f.value(
            4 + offset,
            p::FEATURE_VALUE_IS_DEFAULT,
            Value::Boolean(default && offset == 0),
        );
    }
    f.finish()
}

#[test]
fn deferred_value_binding_does_not_close_early_or_reopen_structural_valuation() {
    let registry = ProducerRegistry::new([
        ProducerFamily::FeatureValuation.descriptor(Default::default()),
        ProducerFamily::FeatureValue.descriptor(Default::default()),
    ])
    .unwrap();
    let snapshot = fixture(false);
    let input =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let q = KerMlQueries::new(input);
    let structural =
        q.plan_result_structure_in_stratum([id(1)], ResultStructureStratum::Structural);
    assert!(structural.deferred_bindings.contains(&id(1)));
    assert!(
        structural
            .producer_evaluations
            .iter()
            .any(|&(s, f, state)| s == id(1)
                && f == ProducerFamily::FeatureValuation.id()
                && state == Completeness::Complete)
    );
    assert!(
        !structural
            .producer_evaluations
            .iter()
            .any(|&(s, f, _)| s == id(1) && f == ProducerFamily::FeatureValue.id())
    );
    let overlay = structural.materialize(&snapshot).unwrap().overlay;
    let context = SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let queries = KerMlQueries::new(context.fork());
    let plan = queries.plan_result_structure_in_stratum(
        overlay.model().elements().map(|r| r.id()),
        ResultStructureStratum::Structural,
    );
    let mut table = ProducerEvaluationTable::default();
    for record in overlay.model().elements() {
        table.pending(record.id(), overlay.model(), &registry);
    }
    let evaluations: Vec<_> = plan
        .producer_evaluations
        .iter()
        .copied()
        .filter(|(_, family, _)| registry.index(*family).is_some())
        .collect();
    let reads: Vec<_> = plan
        .producer_reads
        .iter()
        .filter(|(_, family, _)| registry.index(*family).is_some())
        .cloned()
        .collect();
    table.record(&evaluations, &registry).unwrap();
    table.record_reads(&reads, &registry);
    let certificate =
        ProducerClosureCertificate::issue(overlay.model(), context.id(), &registry, &table, |_| {
            None
        });
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveMembership));
    assert_eq!(
        certificate.evaluation(
            id(1),
            registry.index(ProducerFamily::FeatureValue.id()).unwrap()
        ),
        Some(ProducerEvaluationState::Pending)
    );
    let checkpoint = certificate.checkpoint(&context).unwrap();
    let changed = fixture(true);
    let changed_context =
        SemanticContext::for_snapshot(&changed, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    let rebound = checkpoint.rebind(&changed_context, &registry).unwrap();
    assert_eq!(
        rebound.certificate.evaluation(
            id(1),
            registry
                .index(ProducerFamily::FeatureValuation.id())
                .unwrap()
        ),
        Some(ProducerEvaluationState::Pending)
    );
    // Both FeatureValues contribute structural reads; a later value must not
    // overwrite the first value's flags/endpoints in the retained evaluation.
    let valuation_reads = reads
        .iter()
        .find(|(s, f, _)| *s == id(1) && *f == ProducerFamily::FeatureValuation.id())
        .unwrap();
    for value in [4, 14] {
        assert!(
            valuation_reads
                .2
                .contains(&crate::producer_closure::ProducerRead::Property(
                    id(value),
                    p::FEATURE_VALUE_IS_DEFAULT
                ))
        );
    }
}
