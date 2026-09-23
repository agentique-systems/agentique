use super::*;
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.ComponentReader");
const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.ComponentWriter");

struct ReadOnlyFixture {
    provider: ElementId,
    pending_writer: bool,
    writer_scope: ProducerEffectScope,
}
impl PublicationProducerExtension for ReadOnlyFixture {
    fn descriptors(&self) -> Vec<ProducerDescriptor> {
        let reader = ProducerDescriptor::new(
            READER,
            [],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        );
        let mut writer = ProducerDescriptor::new(
            WRITER,
            [ProducerEffect::Membership],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        );
        writer.scope = self.writer_scope;
        writer.scoped_fresh_ownership = true;
        vec![reader, writer]
    }
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        model.registry().is_subtype(class, c::CLASSIFIER).unwrap()
    }
    fn contribute<'m>(
        &self,
        q: &KerMlQueries<'m>,
        subject: ElementId,
        _: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), agq_kernel::derived::DerivationError> {
        let negative = q.owned_relationships_of_type(self.provider, c::FEATURE_MEMBERSHIP);
        assert!(negative.value.is_empty());
        plan.record_producer_evaluation_evidence(subject, READER, &negative);
        plan.observe_evidence(negative.map(|_| ()))?;
        let mut writer = q.canonical_fact_evidence(FactKey::Element(subject));
        if self.pending_writer && subject == id(2) {
            writer.problem(
                Completeness::Incomplete,
                "FIXTURE_LATE_MEMBERSHIP",
                subject,
                "A later producer may write the earlier owned-member population",
            );
        }
        plan.record_producer_evaluation_evidence(subject, WRITER, &writer);
        plan.observe_evidence(writer)
    }
}

fn fixture() -> Snapshot {
    let mut fixture = Fixture::new();
    fixture.create(1, c::CLASSIFIER);
    fixture.create(2, c::CLASSIFIER);
    fixture.finish()
}

fn registry(extension: &ReadOnlyFixture) -> ProducerRegistry {
    ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(Default::default()))
            .chain(extension.descriptors()),
    )
    .unwrap()
}

fn run(
    snapshot: &Snapshot,
    extension: &ReadOnlyFixture,
    order: PublicationWorklistOrder,
) -> PublicationClosure {
    close_result_structure_with_extension(
        snapshot,
        PublicationClosureOptions {
            order,
            ..Default::default()
        },
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .map_err(PublicationOverlayError::Context)
        },
        extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap()
}

fn context<'m>(
    closure: &'m PublicationClosure,
    registry: &ProducerRegistry,
) -> SemanticContext<'m> {
    SemanticContext::for_overlay(&closure.overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
}

#[test]
fn invalid_split_keeps_future_writer_and_negative_search_obligations_open() {
    let snapshot = fixture();
    let extension = ReadOnlyFixture {
        provider: id(1),
        pending_writer: true,
        writer_scope: ProducerEffectScope::Model,
    };
    let registry = registry(&extension);
    let closure = run(&snapshot, &extension, PublicationWorklistOrder::Fifo);
    assert!(closure.converged);
    assert_eq!(closure.completeness, Completeness::Incomplete);
    let context = context(&closure, &registry);
    let certificate = closure.certificate.as_ref().unwrap();
    let local = BTreeSet::from([id(1)]);
    let audit = certificate
        .audit_component(&context, &registry, &local)
        .unwrap();
    assert!(!audit.local_obligations_closed());
    assert!(
        audit
            .findings()
            .contains(&ComponentClosureFinding::OpenRequirement {
                subject: id(1),
                requirement: SemanticClosureRequirement::EffectiveMembership,
            })
    );
    assert!(
        audit
            .findings()
            .contains(&ComponentClosureFinding::PendingProducer {
                subject: id(1),
                family: READER,
            })
    );
    assert!(audit.findings().iter().all(|finding| !matches!(
        finding, ComponentClosureFinding::IncompleteProducer { subject, .. } if *subject == id(1)
    )), "a locally Complete evaluation must still retain the later writer's causal block");
    let later = certificate
        .audit_component(&context, &registry, &BTreeSet::from([id(2)]))
        .unwrap();
    assert!(
        later
            .findings()
            .contains(&ComponentClosureFinding::IncompleteProducer {
                subject: id(2),
                family: WRITER,
            })
    );
}

#[test]
fn complete_zero_output_searches_are_bound_even_when_flat_receipt_is_identical() {
    let snapshot = fixture();
    let extension = ReadOnlyFixture {
        provider: id(1),
        pending_writer: false,
        writer_scope: ProducerEffectScope::Model,
    };
    let changed = ReadOnlyFixture {
        provider: id(2),
        pending_writer: false,
        writer_scope: ProducerEffectScope::Model,
    };
    let registry = registry(&extension);
    let original = run(&snapshot, &extension, PublicationWorklistOrder::Fifo);
    let changed = run(&snapshot, &changed, PublicationWorklistOrder::Fifo);
    assert_eq!(original.completeness, Completeness::Complete);
    assert_eq!(changed.completeness, Completeness::Complete);
    let first = original.certificate.as_ref().unwrap();
    let second = changed.certificate.as_ref().unwrap();
    assert_eq!(first.receipt_value(), second.receipt_value());
    assert_eq!(
        original.overlay.model().elements().count(),
        snapshot.model().elements().count()
    );
    let subjects = BTreeSet::from([id(1)]);
    let first = first
        .audit_component(&context(&original, &registry), &registry, &subjects)
        .unwrap();
    let second = second
        .audit_component(&context(&changed, &registry), &registry, &subjects)
        .unwrap();
    assert!(first.local_obligations_closed());
    assert!(second.local_obligations_closed());
    assert_ne!(first.read_evidence_digest(), second.read_evidence_digest());
    assert_ne!(first.digest(), second.digest());
    assert_eq!(
        first.closed_requirements(),
        SemanticClosureRequirement::ALL.len()
    );
    assert_eq!(first.applicable_pairs(), first.closed_pairs());
}

#[test]
fn audit_identity_is_independent_of_scheduler_order_and_allocation() {
    let snapshot = fixture();
    let extension = ReadOnlyFixture {
        provider: id(1),
        pending_writer: false,
        writer_scope: ProducerEffectScope::Model,
    };
    let registry = registry(&extension);
    let mut expected = None;
    for order in [
        PublicationWorklistOrder::Fifo,
        PublicationWorklistOrder::Lifo,
        PublicationWorklistOrder::ReversedInitial,
        PublicationWorklistOrder::Partitioned,
    ] {
        let closure = run(&snapshot, &extension, order);
        let certificate = closure.certificate.as_ref().unwrap();
        let audit = certificate
            .audit_component(
                &context(&closure, &registry),
                &registry,
                &BTreeSet::from([id(2), id(1)]),
            )
            .unwrap();
        assert!(audit.local_obligations_closed());
        assert_eq!(audit.subjects(), &[id(1), id(2)]);
        assert_eq!(audit.source_certificate_digest(), certificate.digest());
        assert_eq!(*expected.get_or_insert(audit.digest()), audit.digest());
    }
}

#[test]
fn restored_flat_receipt_does_not_supply_new_component_search_evidence() {
    let snapshot = fixture();
    let extension = ReadOnlyFixture {
        provider: id(1),
        pending_writer: false,
        writer_scope: ProducerEffectScope::Model,
    };
    let registry = registry(&extension);
    let closure = run(&snapshot, &extension, PublicationWorklistOrder::Fifo);
    let context = context(&closure, &registry);
    let original = closure.certificate.as_ref().unwrap();
    let restored = ProducerClosureCertificate::from_trusted_receipt(
        &original.receipt_value(),
        context.id(),
        &registry,
        context.model,
    )
    .unwrap();
    assert_eq!(restored.digest(), original.digest());
    assert!(restored.is_fully_closed(context.model));
    let audit = restored
        .audit_component(&context, &registry, &BTreeSet::from([id(1)]))
        .unwrap();
    assert!(!audit.local_obligations_closed());
    assert!(
        audit
            .findings()
            .contains(&ComponentClosureFinding::UnavailableReadEvidence {
                subject: id(1),
                family: READER,
            })
    );
}

#[test]
fn audit_rejects_missing_population_wrong_registry_and_wrong_graph() {
    let snapshot = fixture();
    let extension = ReadOnlyFixture {
        provider: id(1),
        pending_writer: false,
        writer_scope: ProducerEffectScope::Model,
    };
    let registry = registry(&extension);
    let closure = run(&snapshot, &extension, PublicationWorklistOrder::Fifo);
    let context = context(&closure, &registry);
    let certificate = closure.certificate.as_ref().unwrap();
    for (subjects, finding) in [
        (BTreeSet::new(), ComponentClosureFinding::EmptyPopulation),
        (
            BTreeSet::from([id(99)]),
            ComponentClosureFinding::MissingSubject(id(99)),
        ),
    ] {
        let audit = certificate
            .audit_component(&context, &registry, &subjects)
            .unwrap();
        assert_eq!(audit.findings(), &[finding]);
    }
    let subjects = BTreeSet::from([id(1)]);
    assert!(matches!(
        certificate.audit_component(&context, &ProducerRegistry::new([]).unwrap(), &subjects),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let empty = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
    let wrong_context = SemanticContext::for_snapshot(&empty, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    assert!(matches!(
        certificate.audit_component(&wrong_context, &registry, &subjects),
        Err(ContextError::ProducerClosureMismatch)
    ));
}

#[test]
fn disjoint_subject_writer_still_encounters_the_global_membership_proof_guard() {
    let snapshot = fixture();
    let extension = ReadOnlyFixture {
        provider: id(1),
        pending_writer: true,
        writer_scope: ProducerEffectScope::Subject,
    };
    let registry = registry(&extension);
    let closure = run(&snapshot, &extension, PublicationWorklistOrder::Fifo);
    assert!(closure.converged);
    assert_eq!(closure.completeness, Completeness::Incomplete);
    let context = context(&closure, &registry);
    let certificate = closure.certificate.as_ref().unwrap();
    for family in [READER, WRITER] {
        assert_eq!(
            certificate.evaluation(id(1), registry.index(family).unwrap()),
            Some(ProducerEvaluationState::EvaluatedComplete),
            "the precise causal analysis does not connect the disjoint later writer to subject 1"
        );
    }
    let audit = certificate
        .audit_component(&context, &registry, &BTreeSet::from([id(1)]))
        .unwrap();
    assert_eq!(audit.applicable_pairs(), audit.closed_pairs());
    assert!(!audit.local_obligations_closed());
    assert!(
        audit
            .findings()
            .contains(&ComponentClosureFinding::OpenRequirement {
                subject: id(1),
                requirement: SemanticClosureRequirement::EffectiveMembership,
            })
    );
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| matches!(finding, ComponentClosureFinding::OpenRequirement { .. })),
        "only the conservative requirement-footprint guard blocks the earlier component"
    );
}
