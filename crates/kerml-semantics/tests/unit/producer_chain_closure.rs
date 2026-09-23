use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};

const CHAIN_TYPING: ProducerFamilyId = ProducerFamilyId::new("Fixture.ChainTyping");

fn chain_fixture() -> Snapshot {
    let mut f = Fixture::new();
    for feature in [1, 2, 3] {
        f.create(feature, c::FEATURE);
    }
    // Relationship IDs intentionally disagree with canonical owned order.
    relation(
        &mut f,
        1,
        2,
        20,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    relation(
        &mut f,
        1,
        3,
        10,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    f.create(4, c::CLASSIFIER);
    relation(&mut f, 3, 4, 30, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(30, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(3)));
    f.finish()
}

fn typing_registry() -> ProducerRegistry {
    let mut descriptor = ProducerDescriptor::new(
        CHAIN_TYPING,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    descriptor.scope = ProducerEffectScope::Subject;
    ProducerRegistry::new([descriptor]).unwrap()
}

fn evaluation_table(
    snapshot: &Snapshot,
    registry: &ProducerRegistry,
    incomplete: Option<ElementId>,
) -> ProducerEvaluationTable {
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), registry);
    }
    for feature in [1, 2, 3] {
        table
            .record(
                &[(
                    id(feature),
                    CHAIN_TYPING,
                    if incomplete == Some(id(feature)) {
                        Completeness::Incomplete
                    } else {
                        Completeness::Complete
                    },
                )],
                registry,
            )
            .unwrap();
        let reads = if feature == 2 {
            // A head producer needs the complete terminal typing of the chain;
            // its own unrelated typing must not create the reverse dependency.
            vec![ProducerRead::Requirement(
                id(1),
                SemanticClosureRequirement::EffectiveTyping,
            )]
        } else {
            Vec::new()
        };
        table.record_reads(&[(id(feature), CHAIN_TYPING, reads.into())], registry);
    }
    table
}

#[test]
fn nonterminal_chain_typing_does_not_reopen_terminal_effective_typing() {
    let snapshot = chain_fixture();
    let registry = typing_registry();
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            exclude_implied: true,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let q = KerMlQueries::new(context.fork());
    let chain = q.chaining_features(id(1));
    assert_eq!(chain.completeness, Completeness::Complete);
    assert_eq!(chain.value, [id(2), id(3)]);
    let target = q.feature_target(id(1));
    assert_eq!(target.completeness, Completeness::Complete);
    assert_eq!(target.value, Some(id(3)));
    let typing = q.typing_features(id(1));
    assert_eq!(typing.completeness, Completeness::Complete);
    assert_eq!(typing.value, [id(3)]);
    let types = q.feature_types(id(1));
    assert_eq!(types.completeness, Completeness::Complete);
    assert_eq!(types.value, [id(4)]);
    let supertypes = q.all_supertypes(id(1));
    assert_eq!(supertypes.completeness, Completeness::Complete);
    assert_eq!(supertypes.value, [id(1), id(3), id(4)]);

    let table = evaluation_table(&snapshot, &registry, Some(id(2)));
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    assert!(!certificate.is_closed(id(2), requirement));
    assert!(certificate.is_closed(id(3), requirement));
    assert_eq!(
        certificate.evaluation(id(1), 0),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
    assert!(
        certificate.is_closed(id(1), requirement),
        "nonterminal typing does not alter the established chain terminal or its supertypes"
    );
}

#[test]
fn pending_terminal_typing_still_reopens_the_chain() {
    let snapshot = chain_fixture();
    let registry = typing_registry();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let table = evaluation_table(&snapshot, &registry, Some(id(3)));
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    assert!(!certificate.is_closed(id(3), requirement));
    assert!(!certificate.is_closed(id(1), requirement));
}
