use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

const ACTIVATE: ProducerFamilyId = ProducerFamilyId::new("Fixture.Activate");
const TYPE: ProducerFamilyId = ProducerFamilyId::new("Fixture.Type");

struct PendingScalar;
impl PublicationProducerExtension for PendingScalar {
    fn descriptors(&self) -> Vec<ProducerDescriptor> {
        vec![
            ProducerDescriptor::new(
                ACTIVATE,
                [ProducerEffect::Scalar(p::FEATURE_FEATURE_TARGET)],
                ProducerApplicability::Subtypes(vec![c::FEATURE]),
            ),
            ProducerDescriptor::new(
                TYPE,
                [ProducerEffect::Typing],
                ProducerApplicability::Subtypes(vec![c::FEATURE]),
            ),
        ]
    }
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        model.registry().is_subtype(class, c::FEATURE).unwrap()
    }
    fn contribute<'m>(
        &self,
        q: &KerMlQueries<'m>,
        subject: ElementId,
        _: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), agq_kernel::derived::DerivationError> {
        let mut scalar = q.canonical_fact_evidence(FactKey::Element(subject));
        scalar.problem(
            Completeness::Incomplete,
            "FIXTURE_SCALAR",
            subject,
            "A future scalar may activate typing",
        );
        plan.record_producer_evaluation_evidence(subject, ACTIVATE, &scalar);
        plan.observe_evidence(scalar)?;
        let mut typing = q.canonical_fact_evidence(FactKey::Element(subject));
        let _flag = q.read_reference(&mut typing, subject, p::FEATURE_FEATURE_TARGET);
        assert_eq!(typing.completeness, Completeness::Complete);
        plan.record_producer_evaluation_evidence(subject, TYPE, &typing);
        plan.observe_evidence(typing)
    }
}

struct DelayedTyping {
    incomplete: bool,
}
impl PublicationProducerExtension for DelayedTyping {
    fn descriptors(&self) -> Vec<ProducerDescriptor> {
        vec![
            ProducerDescriptor::new(
                ACTIVATE,
                [ProducerEffect::Featuring],
                ProducerApplicability::Subtypes(vec![c::FEATURE]),
            ),
            ProducerDescriptor::new(
                TYPE,
                [ProducerEffect::Typing],
                ProducerApplicability::Subtypes(vec![c::FEATURE]),
            ),
        ]
    }
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        model.registry().is_subtype(class, c::FEATURE).unwrap()
    }
    fn contribute<'m>(
        &self,
        q: &KerMlQueries<'m>,
        subject: ElementId,
        _: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), agq_kernel::derived::DerivationError> {
        let mut evidence = q.canonical_fact_evidence(FactKey::Element(subject));
        plan.record_producer_evaluation_evidence(subject, ACTIVATE, &evidence);
        plan.add_derived_element(
            DerivationKey {
                rule: RuleId::from_u128(99001),
                subject,
                output: OutputKey::from_u128(99004),
            },
            c::TYPE_FEATURING,
            BTreeMap::from([
                (
                    p::TYPE_FEATURING_FEATURE_OF_TYPE,
                    SlotValue::Scalar(Value::Reference(subject)),
                ),
                (
                    p::TYPE_FEATURING_FEATURING_TYPE,
                    SlotValue::Scalar(Value::Reference(id(3))),
                ),
            ]),
            Some(subject),
            &evidence,
        )?;
        let relationships = q.owned_relationships(subject);
        let active = relationships.value.iter().any(|&relationship| {
            q.model()
                .element(relationship)
                .is_some_and(|record| record.metaclass() == c::TYPE_FEATURING)
        });
        evidence.merge(relationships);
        if !active || self.incomplete {
            evidence.problem(
                Completeness::Incomplete,
                "FIXTURE_PENDING",
                subject,
                "Delayed typing family is not yet complete",
            );
        } else {
            plan.add_derived_element(
                DerivationKey {
                    rule: RuleId::from_u128(99002),
                    subject,
                    output: OutputKey::from_u128(99003),
                },
                c::FEATURE_TYPING,
                BTreeMap::from([
                    (
                        p::FEATURE_TYPING_TYPED_FEATURE,
                        SlotValue::Scalar(Value::Reference(subject)),
                    ),
                    (
                        p::FEATURE_TYPING_TYPE,
                        SlotValue::Scalar(Value::Reference(id(2))),
                    ),
                ]),
                Some(subject),
                &evidence,
            )?;
        }
        plan.record_producer_evaluation_evidence(subject, TYPE, &evidence);
        plan.observe_evidence(evidence)
    }
}
fn fixture() -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    f.create(3, c::CLASSIFIER);
    f.finish()
}
fn run(
    snapshot: &Snapshot,
    incomplete: bool,
    options: PublicationClosureOptions,
) -> PublicationClosure {
    close_result_structure_with_extension(
        snapshot,
        options,
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .map_err(PublicationOverlayError::Context)
        },
        &DelayedTyping { incomplete },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap()
}

#[test]
fn delayed_producer_requires_witness_and_closes_after_dependency_activation() {
    let snapshot = fixture();
    let initial = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    assert!(initial.feature_types(id(1)).value.is_empty());
    assert_eq!(
        initial
            .producer_closure(id(1), SemanticClosureRequirement::EffectiveTyping)
            .completeness,
        Completeness::Incomplete
    );
    let partial = run(
        &snapshot,
        false,
        PublicationClosureOptions {
            max_rounds: 1,
            ..Default::default()
        },
    );
    assert!(!partial.converged);
    assert!(partial.certificate.is_none());
    let result = run(&snapshot, false, Default::default());
    assert!(result.converged);
    assert_eq!(
        result.completeness,
        Completeness::Complete,
        "{:?}",
        result.stages
    );
    let certificate = result.certificate.unwrap();
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    let context =
        SemanticContext::for_overlay(&result.overlay, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap()
            .with_producer_closure(certificate.clone())
            .unwrap();
    let q = KerMlQueries::new(context);
    assert_eq!(q.feature_types(id(1)).value, vec![id(2)]);
    assert_eq!(
        q.producer_closure(id(1), SemanticClosureRequirement::EffectiveTyping)
            .completeness,
        Completeness::Complete
    );
    assert!(!q.feature_types(id(1)).value.contains(&id(3)));
}

#[test]
fn incomplete_typing_family_blocks_its_subject_but_inapplicability_does_not() {
    let result = run(&fixture(), true, Default::default());
    assert_eq!(result.completeness, Completeness::Incomplete);
    let certificate = result.certificate.unwrap();
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
    assert!(certificate.incomplete_pairs() > 0);
}

#[test]
fn complete_empty_typing_evaluation_is_not_closed_while_scalar_dependency_is_pending() {
    let snapshot = fixture();
    let result = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .map_err(PublicationOverlayError::Context)
        },
        &PendingScalar,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(result.converged);
    assert_eq!(result.completeness, Completeness::Incomplete);
    let certificate = result.certificate.unwrap();
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
}

#[test]
fn certificate_is_identical_across_frontier_orders_batches_and_reference_scan() {
    let snapshot = fixture();
    let expected = run(&snapshot, false, Default::default())
        .certificate
        .unwrap();
    for order in [
        PublicationWorklistOrder::Fifo,
        PublicationWorklistOrder::Lifo,
        PublicationWorklistOrder::ReversedInitial,
        PublicationWorklistOrder::Partitioned,
    ] {
        for batch_size in [1, 7] {
            for strategy in [
                PublicationClosureStrategy::Worklist,
                PublicationClosureStrategy::ReferenceFullScan,
            ] {
                let actual = run(
                    &snapshot,
                    false,
                    PublicationClosureOptions {
                        order,
                        batch_size,
                        strategy,
                        ..Default::default()
                    },
                )
                .certificate
                .unwrap();
                assert_eq!(
                    actual.digest(),
                    expected.digest(),
                    "{order:?}, {batch_size}, {strategy:?}"
                );
            }
        }
    }
}

struct NegativeOutput;
impl PublicationProducerExtension for NegativeOutput {
    fn descriptors(&self) -> Vec<ProducerDescriptor> {
        [ACTIVATE, TYPE]
            .into_iter()
            .map(|family| {
                let mut descriptor = ProducerDescriptor::new(
                    family,
                    [],
                    ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
                );
                descriptor
                    .fresh_effects
                    .insert(ProducerEffect::ResultStructure);
                descriptor
            })
            .collect()
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
        let positive = q.canonical_fact_evidence(FactKey::Element(subject));
        plan.record_producer_evaluation_evidence(subject, ACTIVATE, &positive);
        if subject == id(2) {
            plan.add_derived_element(
                DerivationKey {
                    subject,
                    rule: RuleId::from_u128(99501),
                    output: OutputKey::from_u128(1),
                },
                c::COMMENT,
                BTreeMap::from([(
                    p::COMMENT_BODY,
                    SlotValue::Scalar(Value::String("unrelated".into())),
                )]),
                None,
                &positive,
            )?;
        }
        let negative = if subject == id(1) {
            q.producer_closure(subject, SemanticClosureRequirement::EffectiveTyping)
                .map(|_| ())
        } else {
            positive
        };
        plan.record_producer_evaluation_evidence(subject, TYPE, &negative);
        if subject == id(1) {
            plan.add_derived_element(
                DerivationKey {
                    subject,
                    rule: RuleId::from_u128(99502),
                    output: OutputKey::from_u128(1),
                },
                c::COMMENT,
                BTreeMap::from([(
                    p::COMMENT_BODY,
                    SlotValue::Scalar(Value::String("negative".into())),
                )]),
                None,
                &negative,
            )?;
        }
        plan.observe_evidence(negative)
    }
}

#[test]
fn negative_closure_output_is_deterministic_across_work_orders_and_batches() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::CLASSIFIER);
    let snapshot = f.finish();
    let mut expected = None;
    for order in [
        PublicationWorklistOrder::Fifo,
        PublicationWorklistOrder::Lifo,
        PublicationWorklistOrder::ReversedInitial,
        PublicationWorklistOrder::Partitioned,
    ] {
        for batch_size in [1, 7] {
            let result = close_result_structure_with_extension(
                &snapshot,
                PublicationClosureOptions {
                    order,
                    batch_size,
                    ..Default::default()
                },
                |overlay| {
                    SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                        .map_err(PublicationOverlayError::Context)
                },
                &NegativeOutput,
                |_, _, _, _| {},
                |_| {},
            )
            .unwrap();
            assert!(result.converged);
            assert_eq!(result.completeness, Completeness::Complete);
            let certificate = result.certificate.unwrap();
            assert_eq!(
                result
                    .overlay
                    .model()
                    .elements()
                    .filter(|record| record.metaclass() == c::COMMENT)
                    .count(),
                2
            );
            let actual = (certificate.model_digest(), certificate.digest());
            assert_eq!(
                *expected.get_or_insert(actual),
                actual,
                "{order:?}, batch={batch_size}"
            );
        }
    }
}

#[test]
fn registry_and_graph_changes_reject_previous_certificate() {
    let snapshot = fixture();
    let result = run(&snapshot, false, Default::default());
    let certificate = result.certificate.unwrap();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        ProducerFamilyId::new("Fixture.Added"),
        [ProducerEffect::Typing],
        ProducerApplicability::Any,
    )])
    .unwrap();
    let changed_registry =
        SemanticContext::for_overlay(&result.overlay, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    assert!(matches!(
        changed_registry.with_producer_closure(certificate.clone()),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let changed_graph =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap();
    assert!(matches!(
        changed_graph.with_producer_closure(certificate),
        Err(ContextError::ProducerClosureMismatch)
    ));
}

#[test]
fn requirements_name_exact_typing_effect_dependencies() {
    use ProducerEffect as E;
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    for effect in [
        E::Typing,
        E::Subsetting,
        E::Redefinition,
        E::Specialization,
        E::Conjugation,
        E::FeatureChain,
    ] {
        assert!(requirement.requires(effect));
    }
    for effect in [
        E::ValueBinding,
        E::Scalar(p::FEATURE_IS_VARIABLE),
        E::Featuring,
    ] {
        assert!(!requirement.requires(effect));
    }
}

#[test]
fn fresh_relationship_does_not_exempt_an_existing_semantic_source() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut plan = q.plan_result_structure([]);
    let evidence = q.canonical_fact_evidence(FactKey::Element(id(1)));
    plan.add_derived_element(
        DerivationKey {
            rule: RuleId::from_u128(17),
            subject: id(2),
            output: OutputKey::from_u128(18),
        },
        c::FEATURE_TYPING,
        BTreeMap::from([
            (
                p::FEATURE_TYPING_TYPED_FEATURE,
                SlotValue::Scalar(Value::Reference(id(1))),
            ),
            (
                p::FEATURE_TYPING_TYPE,
                SlotValue::Scalar(Value::Reference(id(2))),
            ),
        ]),
        Some(id(1)),
        &evidence,
    )
    .unwrap();
    let mut descriptor = ProducerDescriptor::new(TYPE, [], ProducerApplicability::Any);
    descriptor.fresh_effects.insert(ProducerEffect::Typing);
    let registry = ProducerRegistry::new([descriptor.clone()]).unwrap();
    assert!(plan.validate_declared_effects(&[id(2)], &registry).is_err());
    descriptor.effects.insert(ProducerEffect::Typing);
    let registry = ProducerRegistry::new([descriptor.clone()]).unwrap();
    assert!(
        plan.validate_declared_effects(&[id(2)], &registry).is_err(),
        "a different existing source exceeds SubjectAndOwned"
    );
    descriptor.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    plan.validate_declared_effects(&[id(2)], &registry).unwrap();
}

#[test]
fn fresh_membership_cannot_silently_reown_an_existing_subject() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut plan = q.plan_result_structure([]);
    plan.add_derived_element(
        DerivationKey {
            rule: RuleId::from_u128(99400),
            subject: id(2),
            output: OutputKey::from_u128(1),
        },
        c::OWNING_MEMBERSHIP,
        BTreeMap::from([(
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(3))]),
        )]),
        Some(id(2)),
        &q.canonical_fact_evidence(FactKey::Element(id(2))),
    )
    .unwrap();
    let mut descriptor = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Membership],
        ProducerApplicability::Any,
    );
    descriptor.fresh_effects.insert(ProducerEffect::Ownership);
    assert!(
        plan.validate_declared_effects(
            &[id(2)],
            &ProducerRegistry::new([descriptor.clone()]).unwrap()
        )
        .is_err()
    );
    descriptor.effects.insert(ProducerEffect::Ownership);
    descriptor.scope = ProducerEffectScope::Model;
    plan.validate_declared_effects(&[id(2)], &ProducerRegistry::new([descriptor]).unwrap())
        .unwrap();
}

#[test]
fn typed_population_retains_broad_reads_only_when_actually_observed() {
    use crate::producer_closure::{ProducerRead, producer_reads};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut evidence = q.canonical_fact_evidence(FactKey::Property {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    });
    evidence.search_dependencies.clear();
    evidence
        .search_dependencies
        .insert(SearchDependency::OwnedRelationships {
            owner: id(1),
            class: c::FEATURE_TYPING,
        });
    let reads = producer_reads(&evidence, q.model());
    assert!(reads.contains(&ProducerRead::Owned(id(1), c::FEATURE_TYPING)));
    assert!(!reads.contains(&ProducerRead::Property(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP
    )));
    evidence
        .search_dependencies
        .insert(SearchDependency::PropertySet {
            element: id(1),
            property: p::ELEMENT_OWNED_RELATIONSHIP,
        });
    assert!(
        producer_reads(&evidence, q.model()).contains(&ProducerRead::Property(
            id(1),
            p::ELEMENT_OWNED_RELATIONSHIP
        ))
    );
}

#[test]
fn owner_scoped_effects_reach_parent_reads_without_tainting_unrelated_subjects() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    f.create(3, c::CLASSIFIER);
    member(&mut f, 1, 2, 4, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let mut writer = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Scalar(p::ELEMENT_DECLARED_NAME)],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::SubjectAndOwners;
    let registry = ProducerRegistry::new([
        writer,
        ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Scalar(p::ELEMENT_DECLARED_SHORT_NAME)],
            ProducerApplicability::Any,
        ),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    for subject in [id(1), id(2), id(3), id(4)] {
        table.pending(subject, snapshot.model(), &registry);
        table
            .record(&[(subject, TYPE, Completeness::Complete)], &registry)
            .unwrap();
        table.record_reads(
            &[(
                subject,
                TYPE,
                vec![ProducerRead::Property(subject, p::ELEMENT_DECLARED_NAME)].into(),
            )],
            &registry,
        );
    }
    table
        .record(&[(id(2), ACTIVATE, Completeness::Incomplete)], &registry)
        .unwrap();
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    assert_eq!(
        certificate.evaluation(id(1), registry.index(TYPE).unwrap()),
        Some(ProducerEvaluationState::Pending)
    );
    assert_eq!(
        certificate.evaluation(id(3), registry.index(TYPE).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
}

#[test]
fn generic_effects_cover_subtype_populations_but_membership_does_not_reown() {
    use crate::producer_closure::{ProducerRead, effect_changes_read};
    let snapshot = fixture();
    for effect in [
        ProducerEffect::Featuring,
        ProducerEffect::Membership,
        ProducerEffect::ResultStructure,
    ] {
        for class in [c::SPECIALIZATION, c::FEATURE_TYPING] {
            assert!(
                !effect_changes_read(effect, &ProducerRead::Owned(id(1), class), snapshot.model()),
                "{effect:?} changes {class:?}"
            );
        }
    }
    assert!(effect_changes_read(
        ProducerEffect::Membership,
        &ProducerRead::Owned(id(1), c::FEATURE_VALUE),
        snapshot.model()
    ));
    assert!(effect_changes_read(
        ProducerEffect::Typing,
        &ProducerRead::Owned(id(1), c::SPECIALIZATION),
        snapshot.model()
    ));
    let ownership = ProducerRead::Source(
        id(1),
        c::RELATIONSHIP,
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
    );
    assert!(!effect_changes_read(
        ProducerEffect::Membership,
        &ownership,
        snapshot.model()
    ));
    assert!(effect_changes_read(
        ProducerEffect::Ownership,
        &ownership,
        snapshot.model()
    ));
}

#[test]
fn pending_creator_cannot_hide_a_future_cross_subject_typing_family() {
    let snapshot = fixture();
    let mut creator = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    creator
        .fresh_effects
        .insert(ProducerEffect::ResultStructure);
    let mut future = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE_VALUE]),
    );
    future.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([creator, future]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = crate::producer_closure::ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
    }
    table
        .record(&[(id(1), ACTIVATE, Completeness::Incomplete)], &registry)
        .unwrap();
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    assert!(
        !certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping),
        "an absent family may activate on a future generated FeatureValue and type this existing Classifier"
    );
}

#[test]
fn immutable_record_reads_are_fixed_but_external_source_relationships_stay_open() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let mut writer = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([
        writer,
        ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        ),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    for (read, expected) in [
        (
            ProducerRead::Any(id(2)),
            ProducerEvaluationState::EvaluatedComplete,
        ),
        (
            ProducerRead::Property(id(2), p::ELEMENT_OWNED_RELATIONSHIP),
            ProducerEvaluationState::EvaluatedComplete,
        ),
        (
            ProducerRead::Source(id(2), c::FEATURE_TYPING, p::FEATURE_TYPING_TYPED_FEATURE),
            ProducerEvaluationState::Pending,
        ),
    ] {
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
        }
        table
            .record(
                &[
                    (id(1), ACTIVATE, Completeness::Incomplete),
                    (id(3), TYPE, Completeness::Complete),
                ],
                &registry,
            )
            .unwrap();
        table.record_reads(&[(id(3), TYPE, vec![read].into())], &registry);
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |subject| subject == id(2),
        );
        assert_eq!(
            certificate.evaluation(id(3), registry.index(TYPE).unwrap()),
            Some(expected)
        );
        assert!(
            !certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping),
            "a new local nonowned typing may still reference the dependency source"
        );
    }
}

#[test]
fn additive_scalar_facts_are_fixed_while_absence_and_collection_reads_remain_open() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    f.create(3, c::FEATURE_TYPING);
    f.own(1, 3);
    f.value(3, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    f.value(3, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    f.value(1, p::ELEMENT_DECLARED_NAME, Value::String("fixed".into()));
    let snapshot = f.finish();
    let registry = ProducerRegistry::new([
        ProducerDescriptor::new(
            ACTIVATE,
            [
                ProducerEffect::Scalar(p::ELEMENT_DECLARED_NAME),
                ProducerEffect::Scalar(p::ELEMENT_DECLARED_SHORT_NAME),
                ProducerEffect::Membership,
            ],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        ),
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    for (property, expected) in [
        (
            p::ELEMENT_DECLARED_NAME,
            ProducerEvaluationState::EvaluatedComplete,
        ),
        (
            p::ELEMENT_DECLARED_SHORT_NAME,
            ProducerEvaluationState::Pending,
        ),
        (
            p::ELEMENT_OWNED_RELATIONSHIP,
            ProducerEvaluationState::Pending,
        ),
    ] {
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            table
                .record(&[(record.id(), TYPE, Completeness::Complete)], &registry)
                .unwrap();
            table.record_reads(
                &[(
                    record.id(),
                    TYPE,
                    if record.id() == id(2) {
                        vec![ProducerRead::Property(id(1), property)].into()
                    } else {
                        Vec::new().into()
                    },
                )],
                &registry,
            );
        }
        table
            .record(&[(id(1), ACTIVATE, Completeness::Incomplete)], &registry)
            .unwrap();
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |_| false,
        );
        assert_eq!(
            certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
            Some(expected)
        );
    }
}

#[test]
fn declared_relationship_classes_bound_current_and_future_population_writes() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let mut previous_registry = None;
    for owner in [id(1), id(3)] {
        for (bound, class, expected) in [
            (false, c::FEATURE_VALUE, ProducerEvaluationState::Pending),
            (
                true,
                c::FEATURE_VALUE,
                ProducerEvaluationState::EvaluatedComplete,
            ),
            (true, c::MEMBERSHIP, ProducerEvaluationState::Pending),
        ] {
            let mut writer = ProducerDescriptor::new(
                ACTIVATE,
                [ProducerEffect::Membership],
                ProducerApplicability::Subtypes(vec![c::FEATURE]),
            );
            writer.scope = ProducerEffectScope::SubjectAndOwners;
            if bound {
                writer.relationship_classes = Some(BTreeSet::from([c::FEATURE_MEMBERSHIP]));
            }
            let registry = ProducerRegistry::new([
                writer,
                ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any),
            ])
            .unwrap();
            if !bound {
                previous_registry = Some(registry.digest());
            } else {
                assert_ne!(previous_registry, Some(registry.digest()));
            }
            let context =
                SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
                    .unwrap()
                    .with_producer_registry_digest(registry.digest())
                    .unwrap();
            let mut table = ProducerEvaluationTable::default();
            for record in snapshot.model().elements() {
                table.pending(record.id(), snapshot.model(), &registry);
                table
                    .record(&[(record.id(), TYPE, Completeness::Complete)], &registry)
                    .unwrap();
                table.record_reads(
                    &[(
                        record.id(),
                        TYPE,
                        if record.id() == id(2) {
                            vec![ProducerRead::Owned(owner, class)].into()
                        } else {
                            Vec::new().into()
                        },
                    )],
                    &registry,
                );
            }
            table
                .record(&[(id(1), ACTIVATE, Completeness::Incomplete)], &registry)
                .unwrap();
            let certificate = ProducerClosureCertificate::issue(
                snapshot.model(),
                context.id(),
                &registry,
                &table,
                |_| false,
            );
            assert_eq!(
                certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
                Some(expected),
                "owner={owner:?}, class={class:?}, bounded={bound}"
            );
        }
    }
}

#[test]
fn relationship_class_bound_rejects_undeclared_subtype_output() {
    let snapshot = fixture();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut plan = q.plan_result_structure([]);
    plan.add_derived_element(
        DerivationKey {
            subject: id(1),
            rule: RuleId::from_u128(99700),
            output: OutputKey::from_u128(1),
        },
        c::FEATURE_VALUE,
        BTreeMap::new(),
        Some(id(1)),
        &q.canonical_fact_evidence(FactKey::Element(id(1))),
    )
    .unwrap();
    let mut descriptor = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Membership],
        ProducerApplicability::Any,
    );
    plan.validate_declared_effects(
        &[id(1)],
        &ProducerRegistry::new([descriptor.clone()]).unwrap(),
    )
    .unwrap();
    descriptor.relationship_classes = Some(BTreeSet::from([c::FEATURE_MEMBERSHIP]));
    assert!(
        plan.validate_declared_effects(&[id(1)], &ProducerRegistry::new([descriptor]).unwrap())
            .is_err()
    );
}

#[test]
fn owned_descendant_effects_do_not_retype_or_invalidate_the_producer_subject() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let mut f = Fixture::new();
    for subject in [1, 2, 3] {
        f.create(subject, c::FEATURE);
    }
    member(&mut f, 1, 2, 4, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let mut writer = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Subsetting],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::OwnedDescendants;
    let registry = ProducerRegistry::new([
        writer.clone(),
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
        table
            .record(&[(record.id(), TYPE, Completeness::Complete)], &registry)
            .unwrap();
        table.record_reads(
            &[(
                record.id(),
                TYPE,
                vec![ProducerRead::Owned(record.id(), c::SUBSETTING)].into(),
            )],
            &registry,
        );
        if record.metaclass() == c::FEATURE {
            table
                .record(
                    &[(
                        record.id(),
                        ACTIVATE,
                        if record.id() == id(1) {
                            Completeness::Incomplete
                        } else {
                            Completeness::Complete
                        },
                    )],
                    &registry,
                )
                .unwrap();
            table.record_reads(&[(record.id(), ACTIVATE, Vec::new().into())], &registry);
        }
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(!certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
    assert!(certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
    assert_eq!(
        certificate.evaluation(id(1), registry.index(TYPE).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
    assert_eq!(
        certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
        Some(ProducerEvaluationState::Pending)
    );

    let q = KerMlQueries::new(context);
    for source in [id(1), id(2)] {
        let mut plan = q.plan_result_structure([]);
        plan.add_derived_element(
            DerivationKey {
                subject: id(1),
                rule: RuleId::from_u128(99800),
                output: OutputKey::from_u128(1),
            },
            c::SUBSETTING,
            BTreeMap::from([
                (
                    p::SUBSETTING_SUBSETTING_FEATURE,
                    SlotValue::Scalar(Value::Reference(source)),
                ),
                (
                    p::SUBSETTING_SUBSETTED_FEATURE,
                    SlotValue::Scalar(Value::Reference(id(3))),
                ),
            ]),
            Some(source),
            &q.canonical_fact_evidence(FactKey::Element(source)),
        )
        .unwrap();
        assert_eq!(
            plan.validate_declared_effects(
                &[id(1)],
                &ProducerRegistry::new([writer.clone()]).unwrap()
            )
            .is_ok(),
            source == id(2)
        );
    }
}

#[test]
fn pending_ownership_can_activate_typing_on_a_currently_unowned_subject() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let mut attachment = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Ownership, ProducerEffect::Membership],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    attachment.scope = ProducerEffectScope::Model;
    for scope in [
        ProducerEffectScope::SubjectAndOwned,
        ProducerEffectScope::OwnedDescendants,
    ] {
        let mut typing =
            ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any);
        typing.scope = scope;
        let registry = ProducerRegistry::new([attachment.clone(), typing]).unwrap();
        let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            table
                .record(&[(record.id(), TYPE, Completeness::Complete)], &registry)
                .unwrap();
            table.record_reads(
                &[(
                    record.id(),
                    TYPE,
                    vec![ProducerRead::Owned(record.id(), c::MEMBERSHIP)].into(),
                )],
                &registry,
            );
        }
        table
            .record(&[(id(1), ACTIVATE, Completeness::Incomplete)], &registry)
            .unwrap();
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |_| false,
        );
        assert!(
            !certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping),
            "pending ownership may place the current orphan in a typing producer's future {scope:?} scope"
        );
    }
}

#[test]
fn positional_bounds_remain_open_to_future_scalar_producers() {
    use crate::FeaturePopulationKind as K;
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    use agq_kernel::metamodel::{MetamodelRegistry, PropertyOwner};
    let direction_alias = PropertyId::from_u128(0xfee100);
    let end_alias = PropertyId::from_u128(0xfee101);
    let unknown_property = PropertyId::from_u128(0xfee102);
    let custom_class = MetaclassId::from_u128(0xfee103);
    let mut descriptors = agq_kerml::descriptors();
    let mut class = descriptors
        .classes
        .iter()
        .find(|c| c.id == c::FEATURE)
        .unwrap()
        .clone();
    class.id = custom_class;
    class.name = "FixtureFeature".into();
    class.direct_supertypes = BTreeSet::from([c::FEATURE]);
    descriptors.classes.push(class);
    for (original, alias) in [
        (p::FEATURE_DIRECTION, direction_alias),
        (p::FEATURE_IS_END, end_alias),
    ] {
        let mut property = descriptors
            .properties
            .iter()
            .find(|p| p.id == original)
            .unwrap()
            .clone();
        property.id = alias;
        property.name = format!("fixture{}", property.name);
        property.owner = PropertyOwner::Class(custom_class);
        property.redefines = BTreeSet::from([original]);
        property.association = None;
        property.opposite_ends.clear();
        descriptors.properties.push(property);
    }
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::CLASSIFIER);
    f.create(2, c::CLASSIFIER);
    let snapshot = f.finish();
    for kind in [K::Parameter, K::End, K::Result] {
        for bounded in [false, true] {
            for future_property in [
                None,
                Some(p::FEATURE_DIRECTION),
                Some(p::FEATURE_IS_END),
                Some(direction_alias),
                Some(end_alias),
                Some(unknown_property),
            ] {
                let mut writer = ProducerDescriptor::new(
                    ACTIVATE,
                    [ProducerEffect::Membership],
                    ProducerApplicability::Any,
                );
                writer.scope = ProducerEffectScope::SubjectAndOwners;
                if bounded {
                    writer.feature_populations = Some(BTreeSet::new());
                }
                let mut descriptors = vec![
                    writer,
                    ProducerDescriptor::new(
                        TYPE,
                        [ProducerEffect::Typing],
                        ProducerApplicability::Any,
                    ),
                ];
                if let Some(property) = future_property {
                    descriptors.push(ProducerDescriptor::new(
                        ProducerFamilyId::new("Fixture.FutureScalar"),
                        [ProducerEffect::Scalar(property)],
                        ProducerApplicability::Subtypes(vec![c::FEATURE]),
                    ));
                }
                let registry = ProducerRegistry::new(descriptors).unwrap();
                let context =
                    SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
                        .unwrap()
                        .with_producer_registry_digest(registry.digest())
                        .unwrap();
                let mut table = ProducerEvaluationTable::default();
                for record in snapshot.model().elements() {
                    table.pending(record.id(), snapshot.model(), &registry);
                    table
                        .record(
                            &[
                                (
                                    record.id(),
                                    ACTIVATE,
                                    if record.id() == id(1) {
                                        Completeness::Incomplete
                                    } else {
                                        Completeness::Complete
                                    },
                                ),
                                (record.id(), TYPE, Completeness::Complete),
                            ],
                            &registry,
                        )
                        .unwrap();
                    table.record_reads(
                        &[
                            (record.id(), ACTIVATE, Vec::new().into()),
                            (
                                record.id(),
                                TYPE,
                                vec![ProducerRead::FeaturePopulation(id(1), kind)].into(),
                            ),
                        ],
                        &registry,
                    );
                }
                let certificate = ProducerClosureCertificate::issue(
                    snapshot.model(),
                    context.id(),
                    &registry,
                    &table,
                    |_| false,
                );
                let remains_open = !bounded
                    || matches!((kind, future_property), (K::Parameter, Some(property)) if [p::FEATURE_DIRECTION, direction_alias, unknown_property].contains(&property))
                    || matches!((kind, future_property), (K::End, Some(property)) if [p::FEATURE_IS_END, end_alias, unknown_property].contains(&property));
                assert_eq!(
                    certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
                    Some(if remains_open {
                        ProducerEvaluationState::Pending
                    } else {
                        ProducerEvaluationState::EvaluatedComplete
                    }),
                    "kind={kind:?}, bounded={bounded}, future={future_property:?}"
                );
            }
        }
    }
}

#[test]
fn empty_positional_bound_rejects_qualifying_or_unknown_members() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    let snapshot = f.finish();
    let direction = snapshot
        .model()
        .navigation_slot(id(2), p::FEATURE_DIRECTION)
        .unwrap()
        .value()
        .clone();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut descriptor = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Membership],
        ProducerApplicability::Any,
    );
    descriptor.feature_populations = Some(BTreeSet::new());
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    for case in 0..5 {
        let mut plan = q.plan_result_structure([]);
        let key = |n| DerivationKey {
            subject: id(1),
            rule: RuleId::from_u128(99900),
            output: OutputKey::from_u128(n),
        };
        let mut slots = BTreeMap::new();
        if case == 1 {
            slots.insert(p::FEATURE_DIRECTION, direction.clone());
        }
        if case == 2 {
            slots.insert(p::FEATURE_IS_END, SlotValue::Scalar(Value::Boolean(true)));
        }
        if case != 4 {
            plan.add_derived_element(
                key(1),
                c::FEATURE,
                slots,
                None,
                &q.canonical_fact_evidence(FactKey::Element(id(1))),
            )
            .unwrap();
        }
        plan.add_derived_element(
            key(2),
            if case == 3 {
                c::RETURN_PARAMETER_MEMBERSHIP
            } else {
                c::FEATURE_MEMBERSHIP
            },
            BTreeMap::from([(
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(key(1).element_id())]),
            )]),
            Some(id(1)),
            &q.canonical_fact_evidence(FactKey::Element(id(1))),
        )
        .unwrap();
        assert_eq!(
            plan.validate_declared_effects(&[id(1)], &registry).is_ok(),
            case == 0,
            "case={case}"
        );
    }
}

#[test]
fn unknown_family_evaluation_and_duplicate_registry_identity_are_rejected() {
    let descriptor =
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any);
    assert!(ProducerRegistry::new([descriptor.clone(), descriptor.clone()]).is_err());
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    let snapshot = fixture();
    let mut table = crate::producer_closure::ProducerEvaluationTable::default();
    table.pending(id(1), snapshot.model(), &registry);
    assert!(
        table
            .record(&[(id(1), ACTIVATE, Completeness::Complete)], &registry)
            .is_err()
    );
}

#[test]
fn populated_overlay_revalidation_keeps_graph_and_certificate_identity() {
    let original = run(&fixture(), false, Default::default());
    let expected = original.certificate.unwrap();
    let repeated = close_result_structure_on_overlay_with_extension(
        original.overlay,
        Default::default(),
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .map_err(PublicationOverlayError::Context)
        },
        &DelayedTyping { incomplete: false },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(repeated.converged);
    assert_eq!(repeated.completeness, Completeness::Complete);
    assert_eq!(repeated.certificate.unwrap().digest(), expected.digest());
    assert_eq!(repeated.counters.new_elements_proposed, 0);
    assert_eq!(repeated.counters.fixed_point_rounds, 1);
}

#[test]
fn populated_overlay_revalidates_under_changed_interpretation_contract() {
    let original = run(&fixture(), false, Default::default());
    let old = original.certificate.unwrap();
    let repeated = close_result_structure_on_overlay_with_extension(
        original.overlay,
        Default::default(),
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .and_then(|context| {
                    context.with_semantic_extension_identity("fixture-naming/1", [7; 32])
                })
                .map_err(PublicationOverlayError::Context)
        },
        &DelayedTyping { incomplete: false },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(repeated.converged);
    assert_eq!(repeated.completeness, Completeness::Complete);
    let renewed = repeated.certificate.unwrap();
    assert_ne!(
        renewed.context_contract_digest(),
        old.context_contract_digest()
    );
    assert_ne!(renewed.digest(), old.digest());
    assert_eq!(repeated.counters.new_elements_proposed, 0);
    let context =
        SemanticContext::for_overlay(&repeated.overlay, Default::default(), BTreeSet::new())
            .unwrap()
            .with_semantic_extension_identity("fixture-naming/1", [7; 32])
            .unwrap()
            .with_producer_registry_digest(renewed.registry_digest())
            .unwrap();
    assert!(!old.compatible_context(context.id()));
    let q = KerMlQueries::new(context.with_producer_closure(renewed).unwrap());
    assert_eq!(q.feature_types(id(1)).value, vec![id(2)]);
}

#[test]
fn incomplete_target_and_owning_producer_block_dependent_subject_typing() {
    let mut f = Fixture::new();
    for subject in [1, 2, 3, 4] {
        f.create(subject, c::FEATURE);
    }
    subset(&mut f, 1, 2, 20);
    member(&mut f, 2, 3, 30, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = crate::producer_closure::ProducerEvaluationTable::default();
    for subject in [1, 2, 3, 4] {
        table.pending(id(subject), snapshot.model(), &registry);
        table.record_reads(&[(id(subject), TYPE, Vec::new().into())], &registry);
        table
            .record(
                &[(
                    id(subject),
                    TYPE,
                    if subject == 2 {
                        Completeness::Incomplete
                    } else {
                        Completeness::Complete
                    },
                )],
                &registry,
            )
            .unwrap();
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    for subject in [1, 2, 3] {
        assert!(
            !certificate.is_closed(id(subject), SemanticClosureRequirement::EffectiveTyping),
            "{subject}"
        );
    }
    assert!(certificate.is_closed(id(4), SemanticClosureRequirement::EffectiveTyping));
}

#[test]
fn ownership_derived_chain_and_reference_sources_propagate_target_incompleteness() {
    let mut f = Fixture::new();
    for subject in [1, 2, 3] {
        f.create(subject, c::FEATURE);
    }
    relation(
        &mut f,
        1,
        2,
        10,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    relation(
        &mut f,
        3,
        2,
        11,
        c::REFERENCE_SUBSETTING,
        p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
    );
    let snapshot = f.finish();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = crate::producer_closure::ProducerEvaluationTable::default();
    for subject in [1, 2, 3] {
        table.pending(id(subject), snapshot.model(), &registry);
        table
            .record(
                &[(
                    id(subject),
                    TYPE,
                    if subject == 2 {
                        Completeness::Incomplete
                    } else {
                        Completeness::Complete
                    },
                )],
                &registry,
            )
            .unwrap();
        table.record_reads(&[(id(subject), TYPE, Vec::new().into())], &registry);
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    for subject in [1, 2, 3] {
        assert!(
            !certificate.is_closed(id(subject), SemanticClosureRequirement::EffectiveTyping),
            "{subject}"
        );
    }
}

#[test]
fn arbitrary_inverse_read_depends_on_producers_at_other_sources() {
    let snapshot = fixture();
    let registry = ProducerRegistry::new([
        ProducerDescriptor::new(
            ACTIVATE,
            [ProducerEffect::Subsetting],
            ProducerApplicability::Any,
        ),
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = crate::producer_closure::ProducerEvaluationTable::default();
    for subject in [1, 2, 3] {
        table.pending(id(subject), snapshot.model(), &registry);
        for family in [ACTIVATE, TYPE] {
            table
                .record(
                    &[(
                        id(subject),
                        family,
                        if subject == 2 && family == ACTIVATE {
                            Completeness::Incomplete
                        } else {
                            Completeness::Complete
                        },
                    )],
                    &registry,
                )
                .unwrap();
            let reads = if subject == 1 && family == TYPE {
                vec![crate::producer_closure::ProducerRead::Inverse]
            } else {
                vec![]
            };
            table.record_reads(&[(id(subject), family, reads.into())], &registry);
        }
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
}

#[test]
fn certificate_scale_sixty_thousand_subjects_has_compact_pair_storage() {
    use crate::producer_closure::ProducerEvaluationTable;
    let mut f = Fixture::new();
    for n in 1..=60_000 {
        f.create(n, c::CLASSIFIER);
    }
    let snapshot = f.finish();
    let families = [
        "f00", "f01", "f02", "f03", "f04", "f05", "f06", "f07", "f08", "f09", "f10", "f11", "f12",
        "f13", "f14", "f15", "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23", "f24", "f25",
        "f26", "f27", "f28", "f29", "f30", "f31", "f32", "f33", "f34", "f35", "f36", "f37", "f38",
        "f39", "f40", "f41", "f42", "f43", "f44", "f45", "f46", "f47", "f48", "f49", "f50", "f51",
        "f52", "f53", "f54", "f55", "f56", "f57", "f58", "f59", "f60", "f61", "f62", "f63", "f64",
        "f65", "f66", "f67", "f68", "f69", "f70", "f71", "f72", "f73", "f74", "f75", "f76", "f77",
        "f78", "f79",
    ];
    let registry = ProducerRegistry::new(families.into_iter().enumerate().map(|(index, name)| {
        ProducerDescriptor::new(
            ProducerFamilyId::new(name),
            [if index == 0 {
                ProducerEffect::Membership
            } else {
                ProducerEffect::Typing
            }],
            if index < 5 {
                ProducerApplicability::Any
            } else {
                ProducerApplicability::Never
            },
        )
    }))
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    for n in 1..=60_000 {
        table.pending(id(n), snapshot.model(), &registry);
        let evaluations: Vec<_> = registry
            .descriptors()
            .iter()
            .take(5)
            .enumerate()
            .map(|(index, d)| {
                (
                    id(n),
                    d.id,
                    if n == 1 && index == 0 {
                        Completeness::Incomplete
                    } else {
                        Completeness::Complete
                    },
                )
            })
            .collect();
        table.record(&evaluations, &registry).unwrap();
        let reads: crate::producer_closure::ProducerReads =
            vec![crate::producer_closure::ProducerRead::Structural(id(n))].into();
        table.record_reads(
            &evaluations
                .iter()
                .map(|(subject, family, _)| (*subject, *family, reads.clone()))
                .collect::<Vec<_>>(),
            &registry,
        );
    }
    let started = std::time::Instant::now();
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| false,
    );
    eprintln!(
        "closure certificate: subjects=60000 families=80 applicable={} closed={} incomplete={} bytes={} build_ms={}",
        certificate.applicable_pairs(),
        certificate.closed_pairs(),
        certificate.incomplete_pairs(),
        certificate.storage_bytes(),
        started.elapsed().as_millis()
    );
    assert_eq!(certificate.applicable_pairs(), 300_000);
    assert_eq!(certificate.closed_pairs(), 299_995);
    assert_eq!(certificate.incomplete_pairs(), 1);
    assert!(certificate.storage_bytes() < 2_300_000);
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(certificate.is_closed(id(60_000), SemanticClosureRequirement::EffectiveTyping));
}
