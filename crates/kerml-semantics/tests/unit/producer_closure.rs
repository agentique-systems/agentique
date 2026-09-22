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
        [ProducerEffect::Scalar(p::TYPE_IS_ABSTRACT)],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::SubjectAndOwners;
    let registry = ProducerRegistry::new([
        writer,
        ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Scalar(p::TYPE_IS_SUFFICIENT)],
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
                vec![ProducerRead::Property(subject, p::TYPE_IS_ABSTRACT)].into(),
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
