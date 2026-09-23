//! Independent control for atom sharing: storage reuse must preserve each
//! producer's own reads and the reconstruction guards retained by its proof.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{
    ProducerEvaluationTable, ProducerRead, ProducerReads, producer_reads,
};

const FIRST: ProducerFamilyId = ProducerFamilyId::new("Fixture.SharedReads.First");
const SECOND: ProducerFamilyId = ProducerFamilyId::new("Fixture.SharedReads.Second");

fn row_reads(
    table: &ProducerEvaluationTable,
    registry: &ProducerRegistry,
    subject: ElementId,
    family: ProducerFamilyId,
) -> Vec<ProducerRead> {
    table.reads[&subject]
        .iter()
        .find(|(index, _)| Some(*index) == registry.index(family))
        .unwrap()
        .1
        .iter()
        .cloned()
        .collect()
}

fn atom_address(
    table: &ProducerEvaluationTable,
    registry: &ProducerRegistry,
    subject: ElementId,
    family: ProducerFamilyId,
    atom: &ProducerRead,
) -> *const ProducerRead {
    let reads = &table.reads[&subject]
        .iter()
        .find(|(index, _)| Some(*index) == registry.index(family))
        .unwrap()
        .1;
    std::ptr::from_ref(reads.iter().find(|read| *read == atom).unwrap())
}

#[test]
fn shared_atoms_keep_row_local_broad_negative_and_provider_rebind_guards() {
    let mut f = Fixture::new();
    for subject in [1, 3] {
        f.create(subject, c::BEHAVIOR);
    }
    f.create(10, c::FEATURE);
    f.create(20, c::CLASSIFIER);
    f.create(30, c::CLASSIFIER);
    let snapshot = f.finish();
    let registry = ProducerRegistry::new([FIRST, SECOND].map(|family| {
        let mut descriptor = ProducerDescriptor::new(
            family,
            [ProducerEffect::Membership],
            ProducerApplicability::Subtypes(vec![c::BEHAVIOR]),
        );
        descriptor.scope = ProducerEffectScope::Subject;
        descriptor
    }))
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let queries = KerMlQueries::new(context.fork());
    let negative = queries.direct_feature_types(id(10));
    assert_eq!(negative.completeness, Completeness::Complete);
    assert!(negative.value.is_empty());
    let negative_reads = producer_reads(&negative, snapshot.model());
    let source_read =
        ProducerRead::Source(id(10), c::FEATURE_TYPING, p::FEATURE_TYPING_TYPED_FEATURE);
    assert!(negative_reads.contains(&source_read), "{negative_reads:?}");
    let requirement =
        ProducerRead::Requirement(id(10), SemanticClosureRequirement::EffectiveTyping);
    let identity = ProducerRead::Identity(id(30));
    let common = vec![requirement.clone(), identity.clone()];
    let mut table = ProducerEvaluationTable::default();
    for subject in [id(1), id(3)] {
        table.pending(subject, snapshot.model(), &registry);
        for family in [FIRST, SECOND] {
            table
                .record(&[(subject, family, Completeness::Complete)], &registry)
                .unwrap();
            let mut reads = common.clone();
            if subject == id(3) {
                reads.extend(negative_reads.iter().cloned());
            }
            if subject == id(1) && family == SECOND {
                reads.push(ProducerRead::Global);
            }
            let reads: ProducerReads = reads.into();
            table.record_reads(&[(subject, family, reads)], &registry);
        }
    }
    for atom in [&requirement, &identity] {
        let first = atom_address(&table, &registry, id(1), FIRST, atom);
        for (subject, family) in [(id(1), SECOND), (id(3), FIRST), (id(3), SECOND)] {
            assert_eq!(
                first,
                atom_address(&table, &registry, subject, family, atom),
                "equal read values share one immutable atom"
            );
        }
    }
    assert_eq!(
        atom_address(&table, &registry, id(3), FIRST, &source_read),
        atom_address(&table, &registry, id(3), SECOND, &source_read)
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert!(certificate.is_fully_closed(snapshot.model()));
    let checkpoint = certificate.checkpoint(&context).unwrap();
    let before = row_reads(&table, &registry, id(3), FIRST);
    let untouched = row_reads(&table, &registry, id(3), SECOND);
    let extra = ProducerRead::Property(id(10), p::FEATURE_DIRECTION);
    table.record_reads(&[(id(3), FIRST, vec![extra.clone()].into())], &registry);
    let extended = row_reads(&table, &registry, id(3), FIRST);
    assert!(before.iter().all(|read| extended.contains(read)));
    assert!(extended.contains(&extra));
    assert_eq!(row_reads(&table, &registry, id(3), SECOND), untouched);
    assert!(!untouched.contains(&extra));

    // Resetting subject 1 replaces both of its evaluation rows. Subject 3 and
    // the previously issued certificate must retain their original guards.
    table.pending(id(1), snapshot.model(), &registry);
    for family in [FIRST, SECOND] {
        table
            .record(&[(id(1), family, Completeness::Complete)], &registry)
            .unwrap();
        table.record_reads(&[(id(1), family, common.clone().into())], &registry);
    }
    assert_eq!(row_reads(&table, &registry, id(3), FIRST), extended);
    assert_eq!(row_reads(&table, &registry, id(3), SECOND), untouched);
    let revised = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert!(revised.is_fully_closed(snapshot.model()));
    assert_eq!(certificate.receipt_value(), revised.receipt_value());
    let revised_checkpoint = revised.checkpoint(&context).unwrap();

    let mut unrelated = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    unrelated.create(99, c::CLASSIFIER);
    let unrelated = unrelated.finish();
    let next = SemanticContext::for_snapshot(&unrelated, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let original_rebound = checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(original_rebound.reopened_evaluations, 1);
    assert_eq!(original_rebound.retained_evaluations, 3);
    assert_eq!(
        original_rebound
            .certificate
            .evaluation(id(1), registry.index(SECOND).unwrap()),
        Some(ProducerEvaluationState::Pending),
        "old broad read remains in the immutable checkpoint"
    );
    let replaced_rebound = revised_checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(replaced_rebound.reopened_evaluations, 0);
    assert_eq!(replaced_rebound.retained_evaluations, 4);
    assert!(
        replaced_rebound
            .certificate
            .is_fully_closed(unrelated.model())
    );

    let mut typed = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    typed.create(40, c::FEATURE_TYPING);
    typed.value(
        40,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(10)),
    );
    typed.value(40, p::FEATURE_TYPING_TYPE, Value::Reference(id(20)));
    let typed = typed.finish();
    assert_eq!(
        snapshot.model().element(id(10)),
        typed.model().element(id(10))
    );
    let next = SemanticContext::for_snapshot(&typed, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = revised_checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(rebound.reopened_evaluations, 4);
    assert!(!rebound.certificate.is_fully_closed(typed.model()));

    // Unknown source may target feature 10 even though none of its stored
    // records changed. Requirement-only rows must still depend on that provider.
    let mut provider = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    provider.create(41, c::FEATURE_TYPING);
    provider.value(41, p::FEATURE_TYPING_TYPE, Value::Reference(id(20)));
    let provider = provider.construction();
    assert_eq!(
        snapshot.model().element(id(10)),
        provider.model().element(id(10))
    );
    assert_eq!(
        snapshot.model().element(id(30)),
        provider.model().element(id(30))
    );
    let next = SemanticContext::for_construction(&provider, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = revised_checkpoint.rebind(&next, &registry).unwrap();
    assert!(
        !rebound
            .certificate
            .is_closed(id(10), SemanticClosureRequirement::EffectiveTyping)
    );
    for subject in [id(1), id(3)] {
        for family in [FIRST, SECOND] {
            assert_eq!(
                rebound
                    .certificate
                    .evaluation(subject, registry.index(family).unwrap()),
                Some(ProducerEvaluationState::Pending),
                "shared requirement retains each row's provider dependency"
            );
        }
    }
}
