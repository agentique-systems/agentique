use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.CrossSubsetReader");
const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.CrossSubsetWriter");
const CREATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.CrossSubsetCreator");

fn fixture() -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.finish()
}

fn registry(class: Option<MetaclassId>, future: bool) -> ProducerRegistry {
    let mut writer = ProducerDescriptor::new(
        WRITER,
        [ProducerEffect::Membership, ProducerEffect::Subsetting],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = if future {
        ProducerEffectScope::SubjectAndOwningType
    } else {
        ProducerEffectScope::Subject
    };
    writer.relationship_classes = class.map(|class| BTreeSet::from([class]));
    writer.scoped_fresh_ownership = true;
    let mut descriptors = vec![
        writer,
        ProducerDescriptor::new(
            READER,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        ),
    ];
    if future {
        let mut creator = ProducerDescriptor::new(
            CREATOR,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        creator.fresh_effects.insert(ProducerEffect::Membership);
        creator.scoped_fresh_ownership = true;
        descriptors.push(creator);
    }
    ProducerRegistry::new(descriptors).unwrap()
}

fn issue<T>(
    q: &KerMlQueries<'_>,
    registry: &ProducerRegistry,
    answer: &QueryResult<T>,
    future: bool,
) -> ProducerEvaluationState {
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(1), q.model(), registry);
    for descriptor in registry.descriptors() {
        let state = if descriptor.id == READER {
            answer.completeness
        } else if descriptor.id == if future { CREATOR } else { WRITER } {
            Completeness::Incomplete
        } else {
            Completeness::Complete
        };
        table
            .record(&[(id(1), descriptor.id, state)], registry)
            .unwrap();
    }
    table.record_reads(
        &[(id(1), READER, producer_reads(answer, q.model()))],
        registry,
    );
    ProducerClosureCertificate::issue(q.model(), q.context(), registry, &table, |_| None)
        .evaluation(id(1), registry.index(READER).unwrap())
        .unwrap()
}

#[test]
fn cross_subsetting_population_retains_eligible_direct_and_future_writers() {
    let snapshot = fixture();
    for future in [false, true] {
        for (class, expected) in [
            (Some(c::CROSS_SUBSETTING), ProducerEvaluationState::Pending),
            (None, ProducerEvaluationState::Pending),
            (
                Some(c::REDEFINITION),
                ProducerEvaluationState::EvaluatedComplete,
            ),
            (
                Some(c::FEATURE_MEMBERSHIP),
                ProducerEvaluationState::EvaluatedComplete,
            ),
        ] {
            let registry = registry(class, future);
            let context =
                SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
                    .unwrap()
                    .with_producer_registry_digest(registry.digest())
                    .unwrap();
            let q = KerMlQueries::for_production(context);
            let answer = q.owned_cross_subsetting(id(1));
            assert_eq!(answer.value, None);
            assert_eq!(answer.completeness, Completeness::Complete);
            assert_eq!(
                issue(&q, &registry, &answer, future),
                expected,
                "class={class:?}; future={future}"
            );
        }
    }
}

#[test]
fn cross_subsetting_population_preserves_providers_and_independent_broad_reads() {
    let snapshot = fixture();
    let registry = registry(Some(c::FEATURE_MEMBERSHIP), false);
    for specialization in [false, true] {
        let pending = BTreeSet::from([id(1)]);
        let context = SemanticContext::for_project_snapshot(
            &snapshot,
            Default::default(),
            BTreeSet::new(),
            if specialization {
                pending.clone()
            } else {
                BTreeSet::new()
            },
            if specialization {
                BTreeSet::new()
            } else {
                pending
            },
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
        let q = KerMlQueries::for_production(context);
        let answer = q.owned_cross_subsetting(id(1));
        if specialization {
            assert_eq!(answer.completeness, Completeness::Incomplete);
        }
        assert_ne!(
            issue(&q, &registry, &answer, false),
            ProducerEvaluationState::EvaluatedComplete
        );
    }
    for production in [false, true] {
        let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        for reverse in [false, true] {
            let selected = q.owned_cross_subsetting(id(1));
            let broad = q.canonical_fact_evidence(FactKey::Property {
                element: id(1),
                property: p::ELEMENT_OWNED_RELATIONSHIP,
            });
            let answer = if reverse {
                let mut out = broad;
                out.merge(selected);
                out
            } else {
                let mut out = selected.map(|_| ());
                out.merge(broad);
                out
            };
            assert_eq!(
                issue(&q, &registry, &answer, false),
                ProducerEvaluationState::Pending
            );
        }
    }
}
