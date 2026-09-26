use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

const ACTIVATE: ProducerFamilyId = ProducerFamilyId::new("Fixture.Activate");
const TYPE: ProducerFamilyId = ProducerFamilyId::new("Fixture.Type");

#[test]
fn positioned_features_keep_exclusion_guards_without_nonmember_creation_proofs() {
    use crate::producer_closure::{
        ProducerEvaluationTable, ProducerRead, effect_changes_read, producer_reads,
    };
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    let mut f = Fixture::new();
    f.create(1, c::BEHAVIOR);
    f.create(2, c::FEATURE);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    f.value(2, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    f.create(4, c::FEATURE);
    f.create(5, c::FEATURE_MEMBERSHIP);
    f.create(6, c::FEATURE);
    f.enumeration(6, p::FEATURE_DIRECTION, "in");
    f.value(6, p::FEATURE_IS_END, Value::Boolean(true));
    f.create(8, c::CLASSIFIER);
    let snapshot = f.finish();
    let feature = DerivationKey {
        rule: RuleId::from_u128(99401),
        subject: id(1),
        output: OutputKey::from_u128(1),
    };
    let membership = DerivationKey {
        output: OutputKey::from_u128(2),
        ..feature
    };
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    let overlay = |selected: bool, retarget: bool, present: bool| {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        if present {
            let mut slots: BTreeMap<_, _> = snapshot
                .model()
                .element(id(4))
                .unwrap()
                .slots()
                .map(|(p, s)| (p, s.value().clone()))
                .collect();
            if selected {
                slots.insert(
                    p::FEATURE_DIRECTION,
                    snapshot
                        .model()
                        .navigation_slot(id(2), p::FEATURE_DIRECTION)
                        .unwrap()
                        .value()
                        .clone(),
                );
                slots.insert(p::FEATURE_IS_END, SlotValue::Scalar(Value::Boolean(true)));
            }
            builder.element(feature, c::FEATURE, slots, BTreeSet::new());
            let mut slots: BTreeMap<_, _> = snapshot
                .model()
                .element(id(5))
                .unwrap()
                .slots()
                .map(|(p, s)| (p, s.value().clone()))
                .collect();
            slots.insert(
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(if retarget {
                    id(6)
                } else {
                    feature.element_id()
                })]),
            );
            builder.element(membership, c::FEATURE_MEMBERSHIP, slots, BTreeSet::new());
            builder.extend_ordered_references(
                id(1),
                p::ELEMENT_OWNED_RELATIONSHIP,
                vec![membership.element_id()],
                agq_kernel::provenance::Explanation {
                    rule: feature.rule,
                    dependencies: BTreeSet::new(),
                },
            );
            for fact in [
                FactKey::Element(feature.element_id()),
                FactKey::Element(membership.element_id()),
                FactKey::Property {
                    element: id(1),
                    property: p::ELEMENT_OWNED_RELATIONSHIP,
                },
            ] {
                builder.searches(
                    fact,
                    BTreeSet::from([StructuralSearch::ProducerClosure {
                        subject: id(8),
                        requirement: requirement.contract_id().into(),
                    }]),
                );
            }
        }
        builder.build().unwrap()
    };
    let original = overlay(false, false, true);
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::BEHAVIOR]),
    )])
    .unwrap();
    for production in [false, true] {
        let context = SemanticContext::for_overlay(&original, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let q = if production {
            KerMlQueries::for_production(context.fork())
        } else {
            KerMlQueries::new(context.fork())
        };
        for (projection, scalar) in [
            (q.owned_parameter_features(id(1)), p::FEATURE_DIRECTION),
            (q.owned_end_features(id(1)), p::FEATURE_IS_END),
        ] {
            assert_eq!(projection.value, [id(2)]);
            assert_eq!(projection.completeness, Completeness::Complete);
            let reads = producer_reads(&projection, original.model());
            assert!(!reads.contains(&ProducerRead::Requirement(id(8), requirement)));
            let guard = ProducerRead::Property(feature.element_id(), scalar);
            assert!(reads.contains(&guard));
            assert!(effect_changes_read(
                ProducerEffect::Scalar(scalar),
                &guard,
                original.model()
            ));
            assert!(reads.contains(&ProducerRead::Identity(membership.element_id())));
            let mut table = ProducerEvaluationTable::default();
            for record in original.model().elements() {
                table.pending(record.id(), original.model(), &registry);
            }
            table
                .record(&[(id(1), TYPE, Completeness::Complete)], &registry)
                .unwrap();
            table.record_reads(&[(id(1), TYPE, reads)], &registry);
            let certificate = ProducerClosureCertificate::issue(
                original.model(),
                context.id(),
                &registry,
                &table,
                |_| None,
            );
            assert!(certificate.is_closed(id(1), requirement));
            let checkpoint = certificate.checkpoint(&context).unwrap();
            for (selected, retarget, present) in [
                (true, false, true),
                (false, true, true),
                (false, false, false),
            ] {
                let edited = overlay(selected, retarget, present);
                let context =
                    SemanticContext::for_overlay(&edited, Default::default(), BTreeSet::new())
                        .unwrap()
                        .with_producer_registry_digest(registry.digest())
                        .unwrap();
                let next = KerMlQueries::new(context.fork());
                let answer = if scalar == p::FEATURE_DIRECTION {
                    next.owned_parameter_features(id(1))
                } else {
                    next.owned_end_features(id(1))
                };
                assert_eq!(answer.value.len(), if present { 2 } else { 1 });
                if present {
                    assert!(
                        producer_reads(&answer, edited.model())
                            .contains(&ProducerRead::Requirement(id(8), requirement))
                    );
                    assert!(
                        !checkpoint
                            .rebind(&context, &registry)
                            .unwrap()
                            .certificate
                            .is_closed(id(1), requirement)
                    );
                }
            }
            for broad_first in [false, true] {
                let fact = FactKey::Property {
                    element: id(1),
                    property: p::ELEMENT_OWNED_RELATIONSHIP,
                };
                let mut combined = q.result(());
                if broad_first {
                    combined.merge(q.canonical_fact_evidence(fact));
                }
                combined.merge(projection.clone());
                if !broad_first {
                    combined.merge(q.canonical_fact_evidence(fact));
                }
                assert!(
                    producer_reads(&combined, original.model())
                        .contains(&ProducerRead::Requirement(id(8), requirement))
                );
            }
        }
    }
}

#[test]
fn owned_end_population_ignores_only_proven_non_end_writers() {
    use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    f.create(3, c::FEATURE);
    f.value(3, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 1, 2, 4, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 3, 5, c::FEATURE_MEMBERSHIP);
    f.create(6, c::CLASSIFIER);
    f.create(7, c::FEATURE);
    f.value(7, p::FEATURE_IS_END, Value::Boolean(true));
    member(&mut f, 6, 7, 8, c::FEATURE_MEMBERSHIP);
    f.create(9, c::SUBCLASSIFICATION);
    f.value(
        9,
        p::SUBCLASSIFICATION_SUBCLASSIFIER,
        Value::Reference(id(1)),
    );
    f.value(
        9,
        p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        Value::Reference(id(6)),
    );
    f.own(1, 9);
    let snapshot = f.finish();
    let pending = SemanticContext::for_project_snapshot(
        &snapshot,
        Default::default(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::from([id(1)]),
    )
    .unwrap();
    assert_eq!(
        KerMlQueries::new(pending)
            .owned_end_features(id(1))
            .completeness,
        Completeness::Incomplete
    );
    for (effect, populations, expected_closed) in [
        (ProducerEffect::Membership, Some(BTreeSet::new()), true),
        (
            ProducerEffect::Membership,
            Some(BTreeSet::from([crate::FeaturePopulationKind::End])),
            false,
        ),
        (ProducerEffect::Scalar(p::FEATURE_IS_END), None, false),
    ] {
        let mut writer = ProducerDescriptor::new(
            ACTIVATE,
            [effect],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        );
        writer.feature_populations = populations;
        writer.relationship_classes = Some(BTreeSet::from([c::FEATURE_MEMBERSHIP]));
        let reader = ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
        );
        let registry = ProducerRegistry::new([writer, reader]).unwrap();
        let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let q = KerMlQueries::for_production(context.fork());
        let ends = q.owned_end_features(id(1));
        assert_eq!(ends.value, vec![id(3)]);
        assert_eq!(ends.completeness, Completeness::Complete);
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
        }
        table
            .record(&[(id(1), TYPE, Completeness::Complete)], &registry)
            .unwrap();
        table
            .record(&[(id(6), TYPE, Completeness::Complete)], &registry)
            .unwrap();
        table.record_reads(&[(id(6), TYPE, Vec::new().into())], &registry);
        table.record_reads(
            &[(id(1), TYPE, producer_reads(&ends, snapshot.model()))],
            &registry,
        );
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |_| None,
        );
        assert_eq!(
            certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping),
            expected_closed,
            "{effect:?}"
        );
    }
}

#[test]
fn inverse_ownership_uses_selected_edge_proof_and_keeps_derived_and_broad_reads() {
    use crate::producer_closure::{ProducerRead, effect_changes_read, producer_reads};
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    for subject in [2, 5, 7, 8] {
        f.create(subject, c::FEATURE);
    }
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 5, 4, c::FEATURE_MEMBERSHIP);
    f.owned
        .get_mut(&id(1))
        .unwrap()
        .retain(|v| *v != Value::Reference(id(4)));
    f.create(9, c::FEATURE_TYPING);
    f.value(9, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(2)));
    f.value(9, p::FEATURE_TYPING_TYPE, Value::Reference(id(1)));
    f.changes.set(
        id(9),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(7))]),
        origin(),
    );
    let snapshot = f.finish();
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    let mut builder = DerivationBuilder::new(snapshot);
    for (owner, property, target) in [
        (1, p::ELEMENT_OWNED_RELATIONSHIP, 4),
        (9, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, 8),
    ] {
        builder.extend_ordered_references(
            id(owner),
            property,
            vec![id(target)],
            agq_kernel::provenance::Explanation {
                rule: RuleId::from_u128(99201),
                dependencies: BTreeSet::new(),
            },
        );
        builder.searches(
            FactKey::Property {
                element: id(owner),
                property,
            },
            BTreeSet::from([StructuralSearch::ProducerClosure {
                subject: id(5),
                requirement: requirement.contract_id().into(),
            }]),
        );
    }
    let overlay = builder.build().unwrap();
    for production in [false, true] {
        let context =
            SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap();
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        for (selected, appended, owner, property, class, child, appended_child) in [
            (
                q.owning_related_element(id(3)),
                q.owning_related_element(id(4)),
                1,
                p::ELEMENT_OWNED_RELATIONSHIP,
                c::ELEMENT,
                3,
                4,
            ),
            (
                q.owning_relationship(id(7)),
                q.owning_relationship(id(8)),
                9,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                c::RELATIONSHIP,
                7,
                8,
            ),
        ] {
            let fact = FactKey::Property {
                element: id(owner),
                property,
            };
            assert_eq!(selected.value, Some(id(owner)));
            assert_eq!(selected.completeness, Completeness::Complete);
            assert!(
                selected
                    .canonical_dependencies
                    .contains(&Dependency::Declared(fact))
            );
            let reads = producer_reads(&selected, overlay.model());
            assert!(!reads.contains(&ProducerRead::Requirement(id(5), requirement)));
            let inverse = ProducerRead::Source(id(child), class, property);
            assert!(reads.contains(&inverse));
            assert!(effect_changes_read(
                ProducerEffect::Ownership,
                &inverse,
                overlay.model()
            ));
            assert_eq!(appended.value, Some(id(owner)));
            assert!(
                appended
                    .search_dependencies
                    .contains(&SearchDependency::Kernel(
                        StructuralSearch::OrderedReferenceContribution {
                            element: id(owner),
                            property,
                            target: id(appended_child),
                        }
                    ))
            );
            assert!(
                producer_reads(&appended, overlay.model())
                    .contains(&ProducerRead::Requirement(id(5), requirement))
            );
            for broad_first in [false, true] {
                let mut combined = q.result(());
                if broad_first {
                    combined.merge(q.canonical_fact_evidence(fact));
                }
                combined.merge(selected.clone());
                if !broad_first {
                    combined.merge(q.canonical_fact_evidence(fact));
                }
                let reads = producer_reads(&combined, overlay.model());
                assert!(reads.contains(&ProducerRead::Property(id(owner), property)));
                assert!(reads.contains(&ProducerRead::Requirement(id(5), requirement)));
                assert!(
                    crate::read_dependencies::structural_searches(&combined).contains(
                        &StructuralSearch::ProducerClosure {
                            subject: id(5),
                            requirement: requirement.contract_id().into(),
                        }
                    )
                );
            }
        }
    }
}

struct PendingScalar;
impl PublicationProducerExtension for PendingScalar {
    fn descriptors(&self) -> Vec<ProducerDescriptor> {
        vec![
            ProducerDescriptor::new(
                ACTIVATE,
                [ProducerEffect::Scalar(p::ELEMENT_DECLARED_SHORT_NAME)],
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
        let _flag = q.read_value(&mut typing, subject, p::ELEMENT_DECLARED_SHORT_NAME);
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
    assert!(
        !partial
            .certificate
            .as_ref()
            .unwrap()
            .is_closed(id(1), SemanticClosureRequirement::EffectiveTyping)
    );
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
fn primitive_scalar_writers_keep_naming_and_membership_requirements_open() {
    use crate::producer_closure::ProducerEvaluationTable;
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    member(&mut f, 2, 1, 3, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    for (property, class, subject, requirement) in [
        (
            p::ELEMENT_DECLARED_NAME,
            c::FEATURE,
            id(1),
            SemanticClosureRequirement::EffectiveNaming,
        ),
        (
            p::MEMBERSHIP_VISIBILITY,
            c::MEMBERSHIP,
            id(3),
            SemanticClosureRequirement::EffectiveMembership,
        ),
    ] {
        let registry = ProducerRegistry::new([ProducerDescriptor::new(
            ACTIVATE,
            [ProducerEffect::Scalar(property)],
            ProducerApplicability::Subtypes(vec![class]),
        )])
        .unwrap();
        let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        for completeness in [
            None,
            Some(Completeness::Incomplete),
            Some(Completeness::Complete),
        ] {
            let mut table = ProducerEvaluationTable::default();
            for record in snapshot.model().elements() {
                table.pending(record.id(), snapshot.model(), &registry);
            }
            if let Some(completeness) = completeness {
                table
                    .record(&[(subject, ACTIVATE, completeness)], &registry)
                    .unwrap();
                table.record_reads(&[(subject, ACTIVATE, Vec::new().into())], &registry);
            }
            let certificate = ProducerClosureCertificate::issue(
                snapshot.model(),
                context.id(),
                &registry,
                &table,
                |_| None,
            );
            assert_eq!(
                certificate.is_closed(id(2), requirement),
                completeness == Some(Completeness::Complete),
                "{requirement:?} may depend on primitive property {property}"
            );
            assert!(certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
            assert!(certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveOwnership));
        }
    }
}

#[test]
fn ownership_attachment_changes_queries_and_keeps_all_requirements_open() {
    use crate::producer_closure::ProducerEvaluationTable;
    let mut f = Fixture::new();
    for (element, class) in [
        (1, c::FEATURE),
        (2, c::CLASSIFIER),
        (3, c::FEATURE_MEMBERSHIP),
        (4, c::FEATURE),
        (5, c::REFERENCE_SUBSETTING),
        (6, c::CLASSIFIER),
        (7, c::FEATURE_TYPING),
        (9, c::FEATURE),
        (10, c::REDEFINITION),
    ] {
        f.create(element, class);
    }
    f.changes.set(
        id(3),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(1))]),
        origin(),
    );
    f.value(4, p::ELEMENT_DECLARED_NAME, Value::String("target".into()));
    f.value(
        5,
        p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        Value::Reference(id(4)),
    );
    f.value(7, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(4)));
    f.value(7, p::FEATURE_TYPING_TYPE, Value::Reference(id(6)));
    f.own(4, 7);
    f.value(
        10,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(9)),
    );
    f.value(
        10,
        p::REDEFINITION_REDEFINED_FEATURE,
        Value::Reference(id(4)),
    );
    let snapshot = f.finish();
    let options = SemanticOptions {
        exclude_implied: true,
        ..Default::default()
    };
    let before = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap(),
    );
    let mut builder = agq_kernel::derived::DerivationBuilder::new(snapshot.clone());
    for (owner, relationship) in [(id(2), id(3)), (id(1), id(5)), (id(9), id(10))] {
        builder.extend_ordered_references(
            owner,
            p::ELEMENT_OWNED_RELATIONSHIP,
            vec![relationship],
            agq_kernel::provenance::Explanation {
                rule: RuleId::from_u128(99873),
                dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(
                    relationship,
                ))]),
            },
        );
    }
    let overlay = builder.build().unwrap();
    let after = KerMlQueries::new(
        SemanticContext::for_overlay(&overlay, options, BTreeSet::new()).unwrap(),
    );
    assert!(before.feature_types(id(1)).value.is_empty());
    assert_eq!(after.feature_types(id(1)).value, vec![id(6)]);
    assert_eq!(before.owning_type(id(1)).value, None);
    assert_eq!(after.owning_type(id(1)).value, Some(id(2)));
    assert!(before.featuring_types(id(1)).value.is_empty());
    assert_eq!(after.featuring_types(id(1)).value, vec![id(2)]);
    assert!(before.effective_features(id(2)).value.is_empty());
    assert_eq!(after.effective_features(id(2)).value, vec![id(1)]);
    assert_eq!(
        before.effective_names(id(9)).value,
        EffectiveNames::Determinate(BTreeSet::new())
    );
    assert_eq!(
        after.effective_names(id(9)).value,
        EffectiveNames::Determinate(BTreeSet::from(["target".into()]))
    );
    assert_eq!(before.common_connector_context(&[id(1)]).value, None);
    assert_eq!(after.common_connector_context(&[id(1)]).value, Some(id(2)));
    for q in [&before, &after] {
        for completeness in [
            q.feature_types(id(1)).completeness,
            q.owning_type(id(1)).completeness,
            q.featuring_types(id(1)).completeness,
            q.effective_features(id(2)).completeness,
            q.effective_names(id(9)).completeness,
            q.common_connector_context(&[id(1)]).completeness,
        ] {
            assert_eq!(completeness, Completeness::Complete);
        }
    }

    let mut writer = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Ownership],
        ProducerApplicability::Any,
    );
    writer.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let context = before
        .context
        .fork()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    for completeness in [
        None,
        Some(Completeness::Incomplete),
        Some(Completeness::Complete),
    ] {
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            if let Some(completeness) = completeness {
                table
                    .record(&[(record.id(), ACTIVATE, completeness)], &registry)
                    .unwrap();
            }
            table.record_reads(&[(record.id(), ACTIVATE, Vec::new().into())], &registry);
        }
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |_| None,
        );
        for requirement in SemanticClosureRequirement::ALL {
            assert!(requirement.requires(ProducerEffect::Ownership));
            assert_eq!(
                certificate.is_closed(id(1), requirement),
                completeness == Some(Completeness::Complete),
                "{requirement:?} must account for ownership-dependent query inputs"
            );
        }
    }
}

#[test]
fn formal_owner_absence_requires_a_witness_until_delayed_attachment() {
    use crate::producer_closure::ProducerEvaluationTable;
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.value(1, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.create(2, c::BEHAVIOR);
    f.create(3, c::FEATURE_MEMBERSHIP);
    f.changes.set(
        id(3),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(1))]),
        origin(),
    );
    let snapshot = f.finish();
    let rule = FormalConstraintId::StepSubperformanceSpecialization;
    let mut writer = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Ownership],
        ProducerApplicability::Any,
    );
    writer.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let before = KerMlQueries::new(context.fork());
    let absent = before.formal_constraint_applies(rule, id(1));
    assert!(!absent.value);
    assert_eq!(absent.completeness, Completeness::Incomplete);
    assert!(
        absent
            .search_dependencies
            .contains(&SearchDependency::ProducerClosure {
                subject: id(1),
                requirement: SemanticClosureRequirement::EffectiveOwnership,
                certificate_digest: None,
                source: None,
            })
    );

    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
    }
    let issue = |table: &ProducerEvaluationTable| {
        Arc::new(ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            table,
            |_| None,
        ))
    };
    let pending = issue(&table);
    let q = KerMlQueries::new(
        context
            .fork()
            .with_producer_closure(pending.clone())
            .unwrap(),
    );
    let absent = q.formal_constraint_applies(rule, id(1));
    assert!(!absent.value);
    assert_eq!(absent.completeness, Completeness::Incomplete);
    assert!(
        absent
            .search_dependencies
            .contains(&SearchDependency::ProducerClosure {
                subject: id(1),
                requirement: SemanticClosureRequirement::EffectiveOwnership,
                certificate_digest: Some(pending.digest()),
                source: None,
            })
    );

    for record in snapshot.model().elements() {
        table
            .record(
                &[(record.id(), ACTIVATE, Completeness::Complete)],
                &registry,
            )
            .unwrap();
        table.record_reads(&[(record.id(), ACTIVATE, Vec::new().into())], &registry);
    }
    let closed = issue(&table);
    let q = KerMlQueries::new(
        context
            .fork()
            .with_producer_closure(closed.clone())
            .unwrap(),
    );
    let absent = q.formal_constraint_applies(rule, id(1));
    assert!(!absent.value);
    assert_eq!(absent.completeness, Completeness::Complete);
    assert_eq!(q.negative_queries_certified(), 1);

    let mut builder = agq_kernel::derived::DerivationBuilder::new(snapshot.clone());
    builder.extend_ordered_references(
        id(2),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![id(3)],
        agq_kernel::provenance::Explanation {
            rule: RuleId::from_u128(99874),
            dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(3)))]),
        },
    );
    let attached = builder.build().unwrap();
    let after_context =
        SemanticContext::for_overlay(&attached, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    assert!(matches!(
        after_context.fork().with_producer_closure(closed),
        Err(ContextError::ProducerClosureMismatch)
    ));
    let positive = KerMlQueries::new(after_context).formal_constraint_applies(rule, id(1));
    assert!(positive.value);
    assert_eq!(positive.completeness, Completeness::Complete);
    assert!(
        !positive
            .search_dependencies
            .iter()
            .any(|search| matches!(search, SearchDependency::ProducerClosure { .. }))
    );

    let mut edit = snapshot.change_set();
    edit.set(
        id(1),
        p::FEATURE_IS_COMPOSITE,
        SlotValue::Scalar(Value::Boolean(false)),
        origin(),
    );
    let noncomposite = snapshot.apply(&edit).unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&noncomposite, Default::default(), BTreeSet::new()).unwrap(),
    );
    let false_flag = q.formal_constraint_applies(rule, id(1));
    assert!(!false_flag.value);
    assert_eq!(false_flag.completeness, Completeness::Complete);
    assert!(
        !false_flag
            .search_dependencies
            .iter()
            .any(|search| matches!(search, SearchDependency::ProducerClosure { .. }))
    );
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
    let mut evidence = q
        .owned_relationships_of_type(id(1), c::FEATURE_TYPING)
        .map(|_| ());
    let reads = producer_reads(&evidence, q.model());
    assert!(reads.contains(&ProducerRead::Owned(id(1), c::FEATURE_TYPING)));
    assert!(!reads.contains(&ProducerRead::Property(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP
    )));
    evidence.merge(q.canonical_fact_evidence(FactKey::Property {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    }));
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
        |_| None,
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
fn expression_result_effects_preserve_subject_typing_and_restrict_value_carriers() {
    use crate::producer_closure::{ProducerRead, descriptor_changes_read};
    let mut f = Fixture::new();
    f.create(1, c::EXPRESSION);
    f.create(2, c::FEATURE);
    let snapshot = f.finish();
    let descriptor =
        ProducerFamily::ExpressionResult.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9);
    let registry = ProducerRegistry::new([descriptor.clone()]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveMembership));
    for (class, changes) in [
        (c::FEATURE_VALUE, false),
        (c::FEATURE_MEMBERSHIP, true),
        (c::OWNING_MEMBERSHIP, true),
    ] {
        assert_eq!(
            descriptor
                .effects
                .iter()
                .any(|&effect| descriptor_changes_read(
                    &descriptor,
                    effect,
                    &ProducerRead::Owned(id(1), class),
                    snapshot.model(),
                    &BTreeSet::new(),
                )),
            changes
        );
    }
    // Actual effect validation must still reject a chaining on an existing
    // subject while allowing exactly the same write on a generated helper.
    let q = KerMlQueries::new(context);
    for fresh_source in [false, true] {
        let mut plan = q.plan_result_structure([]);
        let evidence = q.canonical_fact_evidence(FactKey::Element(id(1)));
        let helper = DerivationKey {
            rule: RuleId::from_u128(98224),
            subject: id(1),
            output: OutputKey::from_u128(1),
        };
        if fresh_source {
            plan.add_derived_element(helper, c::FEATURE, BTreeMap::new(), None, &evidence)
                .unwrap();
        }
        plan.add_derived_element(
            DerivationKey {
                output: OutputKey::from_u128(2),
                ..helper
            },
            c::FEATURE_CHAINING,
            BTreeMap::from([(
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                SlotValue::Scalar(Value::Reference(id(2))),
            )]),
            Some(if fresh_source {
                helper.element_id()
            } else {
                id(1)
            }),
            &evidence,
        )
        .unwrap();
        let outputs = plan.planned_elements().collect::<Vec<_>>();
        plan.attribute_producer_outputs(id(1), ProducerFamily::ExpressionResult.id(), outputs);
        let audited = plan.validate_declared_effects(&[id(1)], &registry);
        assert_eq!(audited.is_ok(), fresh_source, "{audited:?}");
    }
}

#[test]
fn expression_and_function_result_emissions_fit_exact_relationship_contract() {
    use agq_kerml::BaselineProfile;
    for profile in [
        BaselineProfile::PublishedKerMl10,
        BaselineProfile::OPERATIONAL_V9,
    ] {
        for (class, role) in [
            (c::EXPRESSION, ImpliedBindingRole::ExpressionResult),
            (c::FUNCTION, ImpliedBindingRole::FunctionResult),
        ] {
            let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
            let mut f = Fixture {
                changes: base.change_set(),
                base,
                owned: BTreeMap::new(),
            };
            f.create(1, class);
            f.create(2, c::FEATURE);
            f.create(3, c::EXPRESSION);
            f.create(4, c::FEATURE);
            member(&mut f, 1, 2, 10, c::RETURN_PARAMETER_MEMBERSHIP);
            member(&mut f, 1, 3, 11, c::RESULT_EXPRESSION_MEMBERSHIP);
            member(&mut f, 3, 4, 12, c::RETURN_PARAMETER_MEMBERSHIP);
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(
                    &snapshot,
                    SemanticOptions {
                        baseline_profile: profile,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .unwrap(),
            );
            let descriptor = ProducerFamily::ExpressionResult.descriptor(profile);
            let registry = ProducerRegistry::new([descriptor.clone()]).unwrap();
            q.plan_result_structure([id(1)])
                .validate_declared_effects(&[id(1)], &registry)
                .unwrap();
            let expanded = q.derive_result_structure(&snapshot).unwrap();
            let model = expanded.overlay.model();
            let emitted: BTreeSet<_> = model
                .elements()
                .filter_map(|record| {
                    let Origin::Derived(proof) = record.origin() else {
                        return None;
                    };
                    (proof.rule == role.rule_id(profile)
                        && model
                            .registry()
                            .is_subtype(record.metaclass(), c::RELATIONSHIP)
                            .unwrap())
                    .then_some(record.metaclass())
                })
                .collect();
            assert!(emitted.contains(&c::BINDING_CONNECTOR));
            assert!(
                emitted.is_subset(descriptor.relationship_classes.as_ref().unwrap()),
                "{profile:?} {role:?}: {emitted:?}"
            );
        }
    }
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
        |_| None,
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
            |subject| (subject == id(2)).then_some(ClosureSource::LocalProducerClosure),
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
fn immutable_noncomposite_inverse_remains_open_to_local_carriers() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::FEATURE_TYPING);
    f.create(3, c::CLASSIFIER);
    f.value(2, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    f.value(2, p::FEATURE_TYPING_TYPE, Value::Reference(id(3)));
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(f.finish())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency);
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(4, c::CLASSIFIER);
    f.create(5, c::CLASSIFIER);
    let snapshot = f.finish();
    // This noncomposite canonical carrier may be added locally. The immutable
    // record stays untouched, but its inverse navigation changes.
    let mut edit = snapshot.change_set();
    edit.set(
        id(4),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(2))]),
        origin(),
    );
    let changed = snapshot.apply(&edit).unwrap();
    assert!(
        snapshot
            .model()
            .navigation_slot(id(2), p::RELATIONSHIP_OWNING_RELATED_ELEMENT)
            .is_none()
    );
    assert_eq!(
        changed
            .model()
            .navigation_slot(id(2), p::RELATIONSHIP_OWNING_RELATED_ELEMENT)
            .unwrap()
            .value(),
        &SlotValue::Scalar(Value::Reference(id(4)))
    );

    let mut writer = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Ownership],
        ProducerApplicability::Any,
    );
    writer.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([
        writer,
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    for read in [
        ProducerRead::Source(id(2), c::ELEMENT, p::ELEMENT_OWNED_RELATIONSHIP),
        ProducerRead::Property(id(2), p::RELATIONSHIP_OWNING_RELATED_ELEMENT),
    ] {
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            table
                .record(
                    &[
                        (record.id(), TYPE, Completeness::Complete),
                        (
                            record.id(),
                            ACTIVATE,
                            if record.id() == id(4) {
                                Completeness::Incomplete
                            } else {
                                Completeness::Complete
                            },
                        ),
                    ],
                    &registry,
                )
                .unwrap();
            table.record_reads(
                &[
                    (record.id(), TYPE, Vec::new().into()),
                    (record.id(), ACTIVATE, Vec::new().into()),
                ],
                &registry,
            );
        }
        table.record_reads(&[(id(5), TYPE, vec![read].into())], &registry);
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |id| {
                snapshot
                    .is_dependency_element(id)
                    .then_some(ClosureSource::LocalProducerClosure)
            },
        );
        assert_eq!(
            certificate.evaluation(id(5), registry.index(TYPE).unwrap()),
            Some(ProducerEvaluationState::Pending)
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
            |_| None,
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
                |_| None,
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
        |_| None,
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
            |_| None,
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
                    |_| None,
                );
                let remains_open = !bounded
                    || future_property == Some(unknown_property)
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
    assert!(matches!(
        table.record(&[(id(1), ACTIVATE, Completeness::Complete)], &registry),
        Err(PublicationOverlayError::ProducerEvaluationMismatch { subject, family: ACTIVATE, reason: "unregistered producer family", state: None, .. }) if subject == id(1)
    ));
    assert!(matches!(
        table.record(&[(id(99), TYPE, Completeness::Complete)], &registry),
        Err(PublicationOverlayError::ProducerEvaluationMismatch { subject, family: TYPE, reason: "subject has no scheduled evaluation row", state: None, .. }) if subject == id(99)
    ));
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Never,
    )])
    .unwrap();
    table.pending(id(1), snapshot.model(), &registry);
    assert!(matches!(
        table.record(&[(id(1), TYPE, Completeness::Complete)], &registry),
        Err(PublicationOverlayError::ProducerEvaluationMismatch { subject, family: TYPE, state: Some(ProducerEvaluationState::Inapplicable), .. }) if subject == id(1)
    ));
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
        |_| None,
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
        |_| None,
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
        |_| None,
    );
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
}

#[test]
fn reference_scalar_writes_reopen_cross_subject_queries_and_requirement_masks() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    for property in [p::FEATURE_FEATURE_TARGET, p::FEATURE_IS_VARIABLE] {
        let mut writer = ProducerDescriptor::new(
            ACTIVATE,
            [ProducerEffect::Scalar(property)],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        writer.scope = ProducerEffectScope::Subject;
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
        for read in [
            ProducerRead::Inverse,
            ProducerRead::Source(id(3), c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE),
            ProducerRead::Owned(id(3), c::FEATURE_TYPING),
            ProducerRead::Requirement(id(3), SemanticClosureRequirement::EffectiveTyping),
        ] {
            let mut table = ProducerEvaluationTable::default();
            for record in snapshot.model().elements() {
                table.pending(record.id(), snapshot.model(), &registry);
                if record.id() == id(1) {
                    table
                        .record(&[(id(1), ACTIVATE, Completeness::Incomplete)], &registry)
                        .unwrap();
                } else {
                    table
                        .record(&[(record.id(), TYPE, Completeness::Complete)], &registry)
                        .unwrap();
                    table.record_reads(&[(record.id(), TYPE, Vec::new().into())], &registry);
                }
            }
            table.record_reads(&[(id(2), TYPE, vec![read].into())], &registry);
            let certificate = ProducerClosureCertificate::issue(
                snapshot.model(),
                context.id(),
                &registry,
                &table,
                |_| None,
            );
            let pending = property == p::FEATURE_FEATURE_TARGET;
            assert_eq!(
                certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
                Some(if pending {
                    ProducerEvaluationState::Pending
                } else {
                    ProducerEvaluationState::EvaluatedComplete
                })
            );
            assert_eq!(
                certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping),
                !pending
            );
        }
    }
}

#[test]
fn semantic_target_bounds_exclude_carriers_from_current_and_future_effects() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    member(&mut f, 2, 1, 3, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    for bounded in [false, true] {
        let mut writer = ProducerDescriptor::new(
            ACTIVATE,
            [ProducerEffect::Membership],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        writer.scope = ProducerEffectScope::SubjectAndOwners;
        if bounded {
            writer.effect_targets = Some(BTreeSet::from([c::TYPE]));
        }
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
        for (read, carrier) in [
            (
                ProducerRead::Property(id(3), p::RELATIONSHIP_OWNED_RELATED_ELEMENT),
                true,
            ),
            (
                ProducerRead::Property(id(2), p::ELEMENT_OWNED_RELATIONSHIP),
                false,
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
                        (id(2), TYPE, Completeness::Complete),
                    ],
                    &registry,
                )
                .unwrap();
            table.record_reads(&[(id(2), TYPE, vec![read].into())], &registry);
            let certificate = ProducerClosureCertificate::issue(
                snapshot.model(),
                context.id(),
                &registry,
                &table,
                |_| None,
            );
            assert_eq!(
                certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
                Some(if bounded && carrier {
                    ProducerEvaluationState::EvaluatedComplete
                } else {
                    ProducerEvaluationState::Pending
                })
            );
        }
    }
}

#[test]
fn semantic_target_bound_audits_existing_source_not_new_relationship_class() {
    for class in [c::CLASSIFIER, c::PACKAGE] {
        let mut f = Fixture::new();
        f.create(1, class);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
        );
        let mut descriptor = ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Membership],
            ProducerApplicability::Any,
        );
        descriptor.effect_targets = Some(BTreeSet::from([c::TYPE]));
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = q.plan_result_structure([]);
        let key = |n| DerivationKey {
            subject: id(1),
            rule: RuleId::from_u128(99870),
            output: OutputKey::from_u128(n),
        };
        let evidence = q.canonical_fact_evidence(FactKey::Element(id(1)));
        plan.add_derived_element(key(1), c::FEATURE, BTreeMap::new(), None, &evidence)
            .unwrap();
        plan.add_derived_element(
            key(2),
            c::FEATURE_MEMBERSHIP,
            BTreeMap::from([(
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                SlotValue::Ordered(vec![Value::Reference(key(1).element_id())]),
            )]),
            Some(id(1)),
            &evidence,
        )
        .unwrap();
        assert_eq!(
            plan.validate_declared_effects(&[id(1)], &registry).is_ok(),
            class == c::CLASSIFIER
        );
    }
}

#[test]
fn semantic_target_bound_audits_existing_scalar_contribution() {
    for class in [c::CLASSIFIER, c::PACKAGE] {
        let mut fixture = Fixture::new();
        fixture.create(1, class);
        let snapshot = fixture.finish();
        let queries = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
        );
        let property = p::ELEMENT_QUALIFIED_NAME;
        assert!(
            snapshot
                .model()
                .registry()
                .property(property)
                .unwrap()
                .derived
        );
        let mut descriptor = ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Scalar(property)],
            ProducerApplicability::Any,
        );
        descriptor.effect_targets = Some(BTreeSet::from([c::TYPE]));
        let registry = ProducerRegistry::new([descriptor]).unwrap();
        let mut plan = queries.plan_result_structure([]);
        let evidence = queries.canonical_fact_evidence(FactKey::Element(id(1)));
        plan.add_derived_property(
            id(1),
            property,
            SlotValue::Scalar(Value::String("Mobility::fixture".into())),
            RuleId::from_u128(99871),
            &evidence,
        )
        .unwrap();
        let audited = plan.validate_declared_effects(&[id(1)], &registry);
        if class == c::CLASSIFIER {
            audited.unwrap();
        } else {
            let Err(PublicationOverlayError::ProducerEffectViolation(failure)) = audited else {
                panic!("the existing scalar subject must satisfy the declared target bound");
            };
            assert_eq!(failure.operation, "existing property contribution");
            assert_eq!(
                failure.fact,
                FactKey::Property {
                    element: id(1),
                    property
                }
            );
            assert_eq!(failure.semantic_target, id(1));
            assert_eq!(failure.origin_rule, RuleId::from_u128(99871));
            assert!(failure.detail.contains(TYPE.name()));
        }
    }
}

#[test]
fn scalar_producer_base_property_matches_effective_read_alias() {
    use crate::producer_closure::{
        ProducerEvaluationTable, ProducerRead, effect_changes_read, producer_reads,
    };
    use agq_kernel::metamodel::{MetamodelRegistry, PropertyOwner};
    let custom_class = MetaclassId::from_u128(0xfee104);
    let alias = PropertyId::from_u128(0xfee105);
    let mut descriptors = agq_kerml::descriptors();
    let mut class = descriptors
        .classes
        .iter()
        .find(|class| class.id == c::FEATURE)
        .unwrap()
        .clone();
    class.id = custom_class;
    class.name = "FixtureNamedFeature".into();
    class.direct_supertypes = BTreeSet::from([c::FEATURE]);
    descriptors.classes.push(class);
    let mut property = descriptors
        .properties
        .iter()
        .find(|property| property.id == p::ELEMENT_QUALIFIED_NAME)
        .unwrap()
        .clone();
    property.id = alias;
    property.name = "fixtureQualifiedName".into();
    property.owner = PropertyOwner::Class(custom_class);
    property.redefines = BTreeSet::from([p::ELEMENT_QUALIFIED_NAME]);
    descriptors.properties.push(property);
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    let mut fixture = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    fixture.create(1, custom_class);
    let snapshot = fixture.finish();
    for (written, read) in [
        (p::ELEMENT_QUALIFIED_NAME, alias),
        (alias, p::ELEMENT_QUALIFIED_NAME),
    ] {
        assert!(
            effect_changes_read(
                ProducerEffect::Scalar(written),
                &ProducerRead::Property(id(1), read),
                snapshot.model(),
            ),
            "base and effective property identity describe the same pending primitive scalar write"
        );
        let mut writer = ProducerDescriptor::new(
            ACTIVATE,
            [ProducerEffect::Scalar(written)],
            ProducerApplicability::Any,
        );
        writer.effect_targets = Some(BTreeSet::from([c::TYPE]));
        let registry = ProducerRegistry::new([
            writer,
            ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any),
        ])
        .unwrap();
        let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let q = KerMlQueries::new(context);
        let mut evidence = q.canonical_fact_evidence(FactKey::Element(id(1)));
        assert!(q.read_value(&mut evidence, id(1), read).is_none());
        let mut table = ProducerEvaluationTable::default();
        table.pending(id(1), snapshot.model(), &registry);
        table
            .record(
                &[
                    (id(1), ACTIVATE, Completeness::Incomplete),
                    (id(1), TYPE, Completeness::Complete),
                ],
                &registry,
            )
            .unwrap();
        table.record_reads(
            &[(id(1), TYPE, producer_reads(&evidence, snapshot.model()))],
            &registry,
        );
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            q.context(),
            &registry,
            &table,
            |_| None,
        );
        assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));

        let mut plan = q.plan_result_structure([]);
        plan.add_derived_property(
            id(1),
            alias,
            SlotValue::Scalar(Value::String("fixture".into())),
            RuleId::from_u128(99872),
            &evidence,
        )
        .unwrap();
        plan.validate_declared_effects(&[id(1)], &registry).unwrap();
        let applied = plan.materialize(&snapshot).unwrap();
        assert_eq!(
            applied
                .overlay
                .model()
                .navigation_slot(id(1), alias)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::String("fixture".into()))
        );
    }
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
        |_| None,
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

#[test]
fn directed_feature_value_closes_without_reowning_an_earlier_context_feature() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    f.create(4, c::EXPRESSION);
    f.create(5, c::FEATURE);
    f.enumeration(5, p::FEATURE_DIRECTION, "out");
    member(&mut f, 4, 5, 6, c::RETURN_PARAMETER_MEMBERSHIP);
    member(&mut f, 2, 4, 7, c::FEATURE_VALUE);
    f.value(7, p::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(false));
    f.value(7, p::FEATURE_VALUE_IS_INITIAL, Value::Boolean(false));
    let snapshot = f.finish();
    let closed = close_result_structure(
        &snapshot,
        PublicationClosureOptions::default(),
        |overlay| {
            SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .map_err(PublicationOverlayError::Context)
        },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closed.converged);
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages.last()
    );
    let queries = KerMlQueries::new(
        SemanticContext::for_overlay(
            &closed.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    let bindings: Vec<_> = closed
        .overlay
        .model()
        .elements()
        .filter(|record| {
            record.metaclass() == c::BINDING_CONNECTOR
                && queries.implied_binding_role(record.id())
                    == Some(ImpliedBindingRole::FeatureValue)
        })
        .map(|record| record.id())
        .collect();
    assert_eq!(bindings.len(), 1);
    assert!(
        queries
            .validate_connector_featuring(bindings[0])
            .value
            .valid
    );
    let current = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    let direct = current
        .plan_result_structure(snapshot.model().elements().map(|record| record.id()))
        .materialize(&snapshot)
        .unwrap();
    assert_eq!(direct.production.completeness, Completeness::Complete);
    assert!(
        closed
            .overlay
            .model()
            .elements()
            .eq(direct.overlay.model().elements()),
        "staging retains the direct contextual plan's identities, records and provenance"
    );
}

#[test]
fn excluded_relationship_populations_keep_unknown_writers_and_broad_reads_open() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead, producer_reads};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    for (effect, output, can_change) in [
        (ProducerEffect::Membership, None, true),
        (
            ProducerEffect::Membership,
            Some(c::FEATURE_MEMBERSHIP),
            false,
        ),
        (
            ProducerEffect::Membership,
            Some(c::RETURN_PARAMETER_MEMBERSHIP),
            false,
        ),
        (ProducerEffect::Membership, Some(c::OWNING_MEMBERSHIP), true),
        (
            ProducerEffect::Membership,
            Some(MetaclassId::from_u128(99890)),
            true,
        ),
        (ProducerEffect::Ownership, Some(c::FEATURE_MEMBERSHIP), true),
    ] {
        let mut writer = ProducerDescriptor::new(
            ACTIVATE,
            [effect],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        );
        writer.scope = ProducerEffectScope::SubjectAndOwners;
        writer.relationship_classes = output.map(|class| BTreeSet::from([class]));
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
        let q = KerMlQueries::new(context);
        for persisted in [false, true] {
            for broad in [false, true] {
                let mut evidence = q.canonical_fact_evidence(FactKey::Property {
                    element: id(1),
                    property: p::ELEMENT_OWNED_RELATIONSHIP,
                });
                evidence.clear_search_dependencies();
                let excluded = BTreeSet::from([c::FEATURE_MEMBERSHIP]);
                evidence.search_dependencies.insert(if persisted {
                    SearchDependency::Kernel(
                        agq_kernel::derived::StructuralSearch::OwnedRelationshipsExcluding {
                            owner: id(1),
                            class: c::MEMBERSHIP,
                            excluded,
                        },
                    )
                } else {
                    SearchDependency::OwnedRelationshipsExcluding {
                        owner: id(1),
                        class: c::MEMBERSHIP,
                        excluded,
                    }
                });
                if broad {
                    evidence
                        .search_dependencies
                        .insert(SearchDependency::PropertySet {
                            element: id(1),
                            property: p::ELEMENT_OWNED_RELATIONSHIP,
                        });
                }
                let reads = producer_reads(&evidence, snapshot.model());
                assert!(evidence.positive_dependencies.contains(&FactKey::Property {
                    element: id(1),
                    property: p::ELEMENT_OWNED_RELATIONSHIP
                }));
                assert_eq!(
                    reads.contains(&ProducerRead::Property(
                        id(1),
                        p::ELEMENT_OWNED_RELATIONSHIP
                    )),
                    broad
                );
                let mut table = ProducerEvaluationTable::default();
                for record in snapshot.model().elements() {
                    table.pending(record.id(), snapshot.model(), &registry);
                }
                table
                    .record(
                        &[
                            (id(2), ACTIVATE, Completeness::Incomplete),
                            (id(1), TYPE, Completeness::Complete),
                        ],
                        &registry,
                    )
                    .unwrap();
                table.record_reads(&[(id(1), TYPE, reads)], &registry);
                let certificate = ProducerClosureCertificate::issue(
                    snapshot.model(),
                    q.context(),
                    &registry,
                    &table,
                    |_| None,
                );
                assert_eq!(
                    certificate.evaluation(id(1), registry.index(TYPE).unwrap()),
                    Some(if broad || can_change {
                        ProducerEvaluationState::Pending
                    } else {
                        ProducerEvaluationState::EvaluatedComplete
                    }),
                    "output={output:?}, persisted={persisted}, broad={broad}"
                );
            }
        }
    }
}

#[test]
fn namespace_proof_transport_retains_names_visibility_aliases_and_import_flags() {
    use crate::producer_closure::{ProducerRead, effect_changes_read, producer_reads};
    use crate::read_dependencies::{query_read_keys, structural_searches};
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 2, 3, c::PACKAGE, "local");
    f.create(10, c::PACKAGE);
    f.member(10, 11, 12, c::PACKAGE, "imported");
    f.import(1, 4, 10, false);
    f.create(5, c::MEMBERSHIP);
    f.own(1, 5);
    f.value(5, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(12)));
    f.value(5, p::MEMBERSHIP_MEMBER_NAME, Value::String("alias".into()));
    let snapshot = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let native = q.namespace_members(id(1), MemberAccess::Public);
    assert_eq!(
        native.completeness,
        Completeness::Complete,
        "{:?}",
        native.diagnostics
    );
    assert!(native.value.iter().any(|member| member.membership == id(5)));
    assert!(
        native
            .search_dependencies
            .contains(&SearchDependency::ImportedNamespace {
                import: id(4),
                namespace: id(10)
            })
    );
    let mut persisted = native.clone();
    persisted.clear_search_dependencies();
    persisted.search_dependencies.extend(
        structural_searches(&native)
            .into_iter()
            .map(SearchDependency::Kernel),
    );
    let before = producer_reads(&native, snapshot.model());
    let after = producer_reads(&persisted, snapshot.model());
    assert_eq!(
        before, after,
        "persistent proof must retain the exact native causal projection"
    );
    assert_eq!(
        native.positive_dependencies,
        persisted.positive_dependencies
    );
    assert_eq!(
        query_read_keys(&native, snapshot.model()),
        query_read_keys(&persisted, snapshot.model())
    );
    for (element, property) in [
        (3, p::ELEMENT_DECLARED_NAME),
        (2, p::MEMBERSHIP_VISIBILITY),
        (5, p::MEMBERSHIP_MEMBER_NAME),
        (5, p::MEMBERSHIP_MEMBER_SHORT_NAME),
        (4, p::IMPORT_VISIBILITY),
        (4, p::IMPORT_IS_RECURSIVE),
        (4, p::IMPORT_IS_IMPORT_ALL),
    ] {
        let read = ProducerRead::Property(id(element), property);
        assert!(after.contains(&read), "missing {element:?}/{property:?}");
        assert!(effect_changes_read(
            ProducerEffect::Scalar(property),
            &read,
            snapshot.model()
        ));
    }
    assert!(!after.contains(&ProducerRead::Any(id(1))));
    assert!(!after.iter().any(|read| effect_changes_read(
        ProducerEffect::Scalar(p::FEATURE_IS_VARIABLE),
        read,
        snapshot.model()
    )));
}

#[test]
fn filtered_declared_ownership_keeps_original_proof_after_unrelated_append() {
    use crate::producer_closure::{ProducerRead, producer_reads};
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::CLASSIFIER);
    f.create(3, c::SUBCLASSIFICATION);
    f.value(
        3,
        p::SUBCLASSIFICATION_SUBCLASSIFIER,
        Value::Reference(id(1)),
    );
    f.value(
        3,
        p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        Value::Reference(id(2)),
    );
    f.own(1, 3);
    f.create(5, c::FEATURE);
    member(&mut f, 1, 5, 4, c::FEATURE_MEMBERSHIP);
    // The later membership already exists, but is not originally owned by 1.
    f.owned
        .get_mut(&id(1))
        .unwrap()
        .retain(|value| *value != Value::Reference(id(4)));
    let snapshot = f.finish();
    let fact = FactKey::Property {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    };
    let requirement = SemanticClosureRequirement::EffectiveTyping;
    let search = StructuralSearch::ProducerClosure {
        subject: id(2),
        requirement: requirement.contract_id().into(),
    };
    let mut builder = DerivationBuilder::new(snapshot);
    builder.extend_ordered_references(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![id(4)],
        agq_kernel::provenance::Explanation {
            rule: RuleId::from_u128(99721),
            dependencies: BTreeSet::new(),
        },
    );
    builder.searches(fact, BTreeSet::from([search]));
    let overlay = builder.build().unwrap();
    for production in [false, true] {
        let context =
            SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap();
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        let selected = q.owned_relationships_of_type(id(1), c::SPECIALIZATION);
        assert_eq!(selected.value, vec![id(3)]);
        assert_eq!(selected.completeness, Completeness::Complete);
        assert!(selected.positive_dependencies.contains(&fact));
        assert!(
            selected
                .canonical_dependencies
                .contains(&Dependency::Declared(fact))
        );
        assert!(
            !producer_reads(&selected, overlay.model())
                .contains(&ProducerRead::Requirement(id(2), requirement))
        );
        if !production {
            assert!(selected.declared_fact_origins.contains_key(&fact));
            assert!(matches!(
                selected.fact_origins[&fact].as_ref(),
                Origin::Declared(_)
            ));
        }
        for (later, broad) in [
            (
                q.owned_relationships_of_type(id(1), c::FEATURE_MEMBERSHIP),
                false,
            ),
            (q.owned_relationships_of_type(id(1), c::RELATIONSHIP), false),
            (q.owned_relationships(id(1)), true),
        ] {
            assert!(later.value.contains(&id(4)));
            if broad {
                assert!(
                    later
                        .canonical_dependencies
                        .contains(&Dependency::Derived(fact))
                );
            } else {
                assert!(
                    later
                        .search_dependencies
                        .contains(&SearchDependency::Kernel(
                            StructuralSearch::OrderedReferenceContribution {
                                element: id(1),
                                property: p::ELEMENT_OWNED_RELATIONSHIP,
                                target: id(4),
                            }
                        ))
                );
            }
            assert!(
                producer_reads(&later, overlay.model())
                    .contains(&ProducerRead::Requirement(id(2), requirement))
            );
        }
        for broad_first in [false, true] {
            let mut combined = q.result(());
            if broad_first {
                q.property(&mut combined, id(1), p::ELEMENT_OWNED_RELATIONSHIP);
            }
            q.selected_reference_fact(
                &mut combined,
                id(1),
                p::ELEMENT_OWNED_RELATIONSHIP,
                &[id(3)],
            );
            if !broad_first {
                q.property(&mut combined, id(1), p::ELEMENT_OWNED_RELATIONSHIP);
            }
            assert!(
                combined
                    .canonical_dependencies
                    .contains(&Dependency::Declared(fact))
            );
            assert!(
                combined
                    .canonical_dependencies
                    .contains(&Dependency::Derived(fact))
            );
            assert!(
                producer_reads(&combined, overlay.model())
                    .contains(&ProducerRead::Requirement(id(2), requirement))
            );
            if !production {
                assert!(matches!(
                    combined.fact_origins[&fact].as_ref(),
                    Origin::Derived(_)
                ));
            }
        }
        for empty in [
            q.owned_relationships_of_type(id(1), c::FEATURE_TYPING),
            q.owned_relationships_of_type(id(2), c::FEATURE_TYPING),
        ] {
            assert!(empty.value.is_empty());
            assert!(!empty.positive_dependencies.contains(&fact));
            assert!(!empty.positive_dependencies.contains(&FactKey::Property {
                element: id(2),
                property: p::ELEMENT_OWNED_RELATIONSHIP
            }));
        }
        let output = DerivationKey {
            rule: RuleId::from_u128(99722),
            subject: id(1),
            output: OutputKey::from_u128(1),
        };
        let mut materialize = DerivationBuilder::from_overlay(overlay.clone());
        materialize.element(
            output,
            c::SUBCLASSIFICATION,
            overlay
                .model()
                .element(id(3))
                .unwrap()
                .slots()
                .map(|(property, slot)| (property, slot.value().clone())),
            selected.canonical_dependencies.clone(),
        );
        materialize.searches(
            FactKey::Element(output.element_id()),
            crate::read_dependencies::structural_searches(&selected),
        );
        let materialized = materialize.build().unwrap();
        let context =
            SemanticContext::for_overlay(&materialized, Default::default(), BTreeSet::new())
                .unwrap();
        let reread = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        let output_evidence = reread.canonical_fact_evidence(FactKey::Element(output.element_id()));
        assert!(
            !producer_reads(&output_evidence, materialized.model())
                .contains(&ProducerRead::Requirement(id(2), requirement)),
            "a stored Declared dependency must not expand the current aggregate proof on reread"
        );
    }
}

#[test]
fn initial_pending_table_can_certify_only_producer_independent_requirements() {
    use crate::producer_closure::ProducerEvaluationTable;
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.value(1, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    let snapshot = f.finish();
    let base =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(base.id().options.baseline_profile)),
    )
    .unwrap();
    let context = base
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let empty = ProducerEvaluationTable::default();
    let mut pending = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        pending.pending(record.id(), snapshot.model(), &registry);
    }
    let issue = |table: &ProducerEvaluationTable| {
        ProducerClosureCertificate::issue(snapshot.model(), context.id(), &registry, table, |_| {
            None
        })
    };
    let certificate = issue(&empty);
    assert_eq!(certificate.digest(), issue(&pending).digest());
    assert_eq!(certificate.closed_pairs(), 0);
    assert!(certificate.applicable_pairs() > 0);
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveOwnership));
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    let q = KerMlQueries::new(
        context
            .with_producer_closure(Arc::new(certificate))
            .unwrap(),
    );
    let answer =
        q.formal_constraint_applies(FormalConstraintId::StepSubperformanceSpecialization, id(1));
    assert!(!answer.value);
    assert_eq!(answer.completeness, Completeness::Complete);

    for future in [false, true] {
        let mut writer = ProducerDescriptor::new(
            ACTIVATE,
            [ProducerEffect::Ownership],
            if future {
                ProducerApplicability::Subtypes(vec![c::FUNCTION])
            } else {
                ProducerApplicability::Any
            },
        );
        writer.scope = ProducerEffectScope::Model;
        let mut creator = ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::ResultStructure],
            ProducerApplicability::Any,
        );
        creator.fresh_effects.insert(ProducerEffect::Membership);
        let registry = ProducerRegistry::new([writer, creator]).unwrap();
        let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &empty,
            |_| None,
        );
        assert!(
            !certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveOwnership),
            "future={future}"
        );
        let q = KerMlQueries::new(
            context
                .with_producer_closure(Arc::new(certificate))
                .unwrap(),
        );
        assert_eq!(
            q.formal_constraint_applies(
                FormalConstraintId::StepSubperformanceSpecialization,
                id(1)
            )
            .completeness,
            Completeness::Incomplete
        );
    }
}

#[test]
fn initial_zero_writer_proof_closes_absent_owner_without_a_scheduler_round() {
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.value(1, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    let snapshot = f.finish();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Any,
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveOwnership));
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    let queries = KerMlQueries::new(context.with_producer_closure(certificate).unwrap());
    let absent = queries.formal_constraint_applies(
        FormalConstraintId::StepOwnedPerformanceSpecialization,
        id(1),
    );
    assert_eq!(absent.completeness, Completeness::Complete, "{absent:?}");
    assert!(!absent.value);
    assert!(absent.search_dependencies.iter().any(|search| matches!(
        search,
        SearchDependency::ProducerClosure {
            source: Some(ClosureSource::LocalProducerClosure),
            requirement: SemanticClosureRequirement::EffectiveOwnership,
            ..
        }
    )));

    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Ownership],
        ProducerApplicability::Any,
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    let queries = KerMlQueries::new(context.with_producer_closure(certificate).unwrap());
    let open = queries.formal_constraint_applies(
        FormalConstraintId::StepOwnedPerformanceSpecialization,
        id(1),
    );
    assert_eq!(open.completeness, Completeness::Incomplete);
    assert!(!open.value);
}

#[test]
fn unaccepted_immutable_dependency_cannot_mint_accepted_closure() {
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(fixture())
            .build()
            .unwrap(),
    );
    let snapshot = Snapshot::with_immutable_dependency(dependency);
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Any,
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert_ne!(
        certificate.closure_source(id(1), SemanticClosureRequirement::EffectiveOwnership),
        Some(ClosureSource::AcceptedDependency)
    );
}

#[test]
fn accepted_dependency_population_is_closed_without_replaying_local_subject_writers() {
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(fixture())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency);
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(4, c::FEATURE);
    let snapshot = f.finish();
    let mut writer = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing, ProducerEffect::Membership],
        ProducerApplicability::Any,
    );
    writer.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let mut context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    // Exercise the private accepted-boundary marker independently of corpus restoration.
    // Production code only installs this marker after the accepted overlay pointer check.
    context.id.publication_dependency_digest = Some([7; 32]);
    context.accepted_dependency = snapshot.immutable_dependency().cloned();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    for requirement in [
        SemanticClosureRequirement::EffectiveTyping,
        SemanticClosureRequirement::EffectiveMembership,
    ] {
        assert_eq!(
            certificate.closure_source(id(1), requirement),
            Some(ClosureSource::AcceptedDependency)
        );
        assert!(!certificate.is_closed(id(4), requirement));
    }
}

#[test]
fn immutable_family_shortcut_preserves_full_certificate_with_cross_subject_blockers() {
    use crate::producer_closure::ProducerEvaluationTable;
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(fixture())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency);
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(4, c::FEATURE);
    let snapshot = f.finish();
    for accepted in [false, true] {
        for scope in [ProducerEffectScope::Subject, ProducerEffectScope::Model] {
            // Three families intentionally cross packed-byte subject boundaries.
            let registry = ProducerRegistry::new(
                [
                    (ACTIVATE, ProducerEffect::Ownership),
                    (TYPE, ProducerEffect::Typing),
                    (
                        ProducerFamilyId::new("Fixture.Membership"),
                        ProducerEffect::Membership,
                    ),
                ]
                .map(|(family, effect)| {
                    let mut writer =
                        ProducerDescriptor::new(family, [effect], ProducerApplicability::Any);
                    writer.scope = scope;
                    writer
                }),
            )
            .unwrap();
            for pending_provider in [false, true] {
                let mut context =
                    SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
                        .unwrap()
                        .with_producer_registry_digest(registry.digest())
                        .unwrap();
                if accepted {
                    context.id.publication_dependency_digest = Some([7; 32]);
                    context.accepted_dependency = snapshot.immutable_dependency().cloned();
                }
                if pending_provider {
                    context.id.pending_namespace_scopes.insert(id(1));
                }
                for result in [
                    None,
                    Some(Completeness::Complete),
                    Some(Completeness::Incomplete),
                ] {
                    let mut table = ProducerEvaluationTable::default();
                    for record in snapshot.model().elements() {
                        table.pending(record.id(), snapshot.model(), &registry);
                        if let Some(result) = result {
                            for descriptor in registry.descriptors() {
                                table
                                    .record(&[(record.id(), descriptor.id, result)], &registry)
                                    .unwrap();
                            }
                        }
                    }
                    let fast = ProducerClosureCertificate::issue(
                        snapshot.model(),
                        context.id(),
                        &registry,
                        &table,
                        |subject| context.dependency_closure_source(subject),
                    );
                    let full = ProducerClosureCertificate::issue_full_family_rows(
                        snapshot.model(),
                        context.id(),
                        &registry,
                        &table,
                        |subject| context.dependency_closure_source(subject),
                    );
                    fast.assert_exact(&full);
                    if accepted {
                        assert!((0..3).all(|family| fast.evaluation(id(1), family)
                            == Some(ProducerEvaluationState::Inapplicable)));
                        if scope == ProducerEffectScope::Model && result.is_none() {
                            assert!(
                                !fast.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping),
                                "immutable dependency still receives the local model-scope blocker"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn immutable_rebind_shortcut_preserves_evaluations_reads_and_negative_search_reopening() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(fixture())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency);
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(4, c::FEATURE);
    let snapshot = f.finish();
    let mut writer =
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any);
    writer.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([writer]).unwrap();
    fn context<'a>(snapshot: &'a Snapshot, registry: &ProducerRegistry) -> SemanticContext<'a> {
        let mut context =
            SemanticContext::for_snapshot(snapshot, Default::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        context.id.publication_dependency_digest = Some([7; 32]);
        context.accepted_dependency = snapshot.immutable_dependency().cloned();
        context
    }
    let old = context(&snapshot, &registry);
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(4), snapshot.model(), &registry);
    table
        .record(&[(id(4), TYPE, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(
            id(4),
            TYPE,
            vec![ProducerRead::Source(
                id(4),
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPED_FEATURE,
            )]
            .into(),
        )],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        old.id(),
        &registry,
        &table,
        |subject| old.dependency_closure_source(subject),
    );
    let checkpoint = certificate.checkpoint(&old).unwrap();
    for changes_read in [false, true] {
        let mut f = Fixture {
            changes: snapshot.change_set(),
            base: snapshot.clone(),
            owned: BTreeMap::new(),
        };
        if changes_read {
            f.create(5, c::FEATURE_TYPING);
            f.value(5, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(4)));
            f.value(5, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
        } else {
            f.create(5, c::CLASSIFIER);
        }
        let next = f.finish();
        let next = context(&next, &registry);
        let fast = checkpoint.rebind(&next, &registry).unwrap();
        let full = checkpoint
            .rebind_full_family_rows(&next, &registry)
            .unwrap();
        fast.certificate.assert_exact(&full.certificate);
        assert_eq!(fast.affected_subjects, full.affected_subjects);
        assert_eq!(fast.retained_evaluations, full.retained_evaluations);
        assert_eq!(fast.reopened_evaluations, full.reopened_evaluations);
        assert_eq!(fast.reopened_evaluations, usize::from(changes_read));
        assert_eq!(fast.retained_evaluations, usize::from(!changes_read));
    }
}

#[test]
fn closure_checkpoint_retains_unrelated_evaluations_and_reopens_changed_negative_searches() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let mut writer = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(1), snapshot.model(), &registry);
    table
        .record(&[(id(1), TYPE, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(
            id(1),
            TYPE,
            vec![ProducerRead::Source(
                id(1),
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPED_FEATURE,
            )]
            .into(),
        )],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    let checkpoint = certificate.checkpoint(&context).unwrap();
    assert!(checkpoint.fingerprint_storage_bytes() <= snapshot.model().len() * 48);
    drop(context);
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot.clone(),
        owned: BTreeMap::new(),
    };
    f.create(4, c::CLASSIFIER);
    let unrelated = f.finish();
    let next = SemanticContext::for_snapshot(&unrelated, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(rebound.retained_evaluations, 1);
    assert_eq!(rebound.reopened_evaluations, 0);
    assert_ne!(
        certificate.model_digest(),
        rebound.certificate.model_digest()
    );
    assert!(
        rebound
            .certificate
            .is_closed(id(1), SemanticClosureRequirement::EffectiveTyping)
    );
    assert!(next.with_producer_closure(rebound.certificate).is_ok());

    // A fresh source relationship invalidates an earlier empty source search,
    // although the Feature record itself is byte-for-byte unchanged.
    let mut f = Fixture {
        changes: unrelated.change_set(),
        base: unrelated,
        owned: BTreeMap::new(),
    };
    f.create(5, c::FEATURE_TYPING);
    f.value(5, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
    f.value(5, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    let changed = f.finish();
    assert_eq!(
        snapshot.model().element(id(1)),
        changed.model().element(id(1))
    );
    let next = SemanticContext::for_snapshot(&changed, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(rebound.reopened_evaluations, 1);
    assert!(
        !rebound
            .certificate
            .is_closed(id(1), SemanticClosureRequirement::EffectiveTyping)
    );
    let changed_registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Ownership],
        ProducerApplicability::Any,
    )])
    .unwrap();
    assert_eq!(
        checkpoint.rebind(&next, &changed_registry).unwrap_err(),
        ContextError::ProducerClosureMismatch
    );
}

#[test]
fn new_writer_opportunities_reopen_equal_looking_conclusions() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let mut writer = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::Model;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(1), snapshot.model(), &registry);
    table
        .record(&[(id(1), TYPE, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(1), TYPE, vec![ProducerRead::Structural(id(1))].into())],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert!(certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
    let checkpoint = certificate.checkpoint(&context).unwrap();
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot,
        owned: BTreeMap::new(),
    };
    f.create(4, c::FEATURE);
    let changed = f.finish();
    let next = SemanticContext::for_snapshot(&changed, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = checkpoint.rebind(&next, &registry).unwrap();
    assert_eq!(rebound.retained_evaluations, 1);
    assert!(
        !rebound
            .certificate
            .is_closed(id(2), SemanticClosureRequirement::EffectiveTyping)
    );
}

#[test]
fn reconstruction_reopens_producer_when_detached_output_disappears() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let output = DerivationKey {
        rule: RuleId::from_u128(99191),
        subject: id(1),
        output: OutputKey::from_u128(99192),
    };
    let mut builder = agq_kernel::derived::DerivationBuilder::new(snapshot.clone());
    builder.element(
        output,
        c::CLASSIFIER,
        snapshot
            .model()
            .element(id(2))
            .unwrap()
            .slots()
            .map(|(property, slot)| (property, slot.value().clone())),
        BTreeSet::new(),
    );
    let overlay = builder.build().unwrap();
    assert_eq!(
        snapshot.model().element(id(1)),
        overlay.model().element(id(1))
    );
    let mut writer = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    writer.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([writer]).unwrap();
    let context = SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    table.pending(id(1), overlay.model(), &registry);
    table
        .record(&[(id(1), TYPE, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(1), TYPE, vec![ProducerRead::Structural(id(1))].into())],
        &registry,
    );
    let certificate =
        ProducerClosureCertificate::issue(overlay.model(), context.id(), &registry, &table, |_| {
            None
        });
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    let checkpoint = certificate.checkpoint(&context).unwrap();
    drop(context);
    drop(overlay);
    let new = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = checkpoint.rebind(&new, &registry).unwrap();
    assert_eq!(rebound.reopened_evaluations, 1);
    assert!(rebound.affected_subjects.contains(&id(1)));
    assert!(
        !rebound
            .certificate
            .is_closed(id(1), SemanticClosureRequirement::EffectiveTyping)
    );
}

#[test]
fn full_certificate_coverage_requires_closed_provider_masks_not_only_complete_evaluations() {
    let mut descriptor =
        ProducerDescriptor::new(TYPE, [ProducerEffect::Typing], ProducerApplicability::Any);
    descriptor.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    for linked in [false, true] {
        let mut fixture = Fixture::new();
        fixture.create(1, c::FEATURE);
        fixture.create(2, c::CLASSIFIER);
        fixture.create(3, c::FEATURE_TYPING);
        fixture.value(3, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
        if linked {
            fixture.value(3, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(1)));
        }
        let construction = fixture.construction();
        let context =
            SemanticContext::for_construction(&construction, Default::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        let mut table = ProducerEvaluationTable::default();
        for record in construction.model().elements() {
            table.pending(record.id(), construction.model(), &registry);
            table
                .record(&[(record.id(), TYPE, Completeness::Complete)], &registry)
                .unwrap();
            table.record_reads(&[(record.id(), TYPE, Vec::new().into())], &registry);
        }
        let certificate = Arc::new(ProducerClosureCertificate::issue(
            construction.model(),
            context.id(),
            &registry,
            &table,
            |_| None,
        ));
        assert_eq!(certificate.applicable_pairs(), 3);
        assert_eq!(certificate.closed_pairs(), certificate.applicable_pairs());
        for record in construction.model().elements() {
            assert_eq!(
                certificate.evaluation(record.id(), 0),
                Some(ProducerEvaluationState::EvaluatedComplete),
            );
        }
        // Exact context attachment and Complete evaluation rows do not certify
        // a source-linking provider that still has an unidentified endpoint.
        let context = context.with_producer_closure(certificate.clone()).unwrap();
        assert_eq!(
            certificate.is_fully_closed(context.model),
            linked,
            "publication acceptance must reject an open provider mask",
        );
        assert_eq!(
            certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping),
            linked,
        );
    }
}

#[test]
fn full_certificate_coverage_rejects_missing_model_subjects() {
    let registry = ProducerRegistry::new([]).unwrap();
    let mut fixture = Fixture::new();
    fixture.create(1, c::PACKAGE);
    let snapshot = fixture.finish();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(certificate.is_fully_closed(snapshot.model()));
    let mut fixture = Fixture::new();
    fixture.create(1, c::PACKAGE);
    fixture.create(2, c::PACKAGE);
    assert!(!certificate.is_fully_closed(fixture.finish().model()));
}

#[test]
fn initial_closure_must_not_close_an_unidentified_pending_typing_provider() {
    let mut f = Fixture::new();
    f.create(1, c::FEATURE);
    f.create(2, c::CLASSIFIER);
    f.create(3, c::FEATURE_TYPING);
    f.value(3, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    // The relationship exists but its required source has not yet linked.
    // Reconstruction can fill this endpoint with the existing Feature 1.
    let construction = f.construction();
    assert!(!construction.obligations().is_empty());
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Never,
    )])
    .unwrap();
    let context =
        SemanticContext::for_construction(&construction, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    let queries = KerMlQueries::new(context.with_producer_closure(certificate).unwrap());
    let types = queries.feature_types(id(1));
    assert!(types.value.is_empty());
    let absent = queries.producer_closure(id(1), SemanticClosureRequirement::EffectiveTyping);
    assert_eq!(
        absent.completeness,
        Completeness::Incomplete,
        "unidentified provider obligations {:?} must keep typing absence open; types={types:?}; closure={absent:?}",
        construction.obligations(),
    );
}

#[test]
fn initial_closure_keeps_pending_relationship_provider_populations_open() {
    for (label, class, target_property, requirement) in [
        (
            "redefinition",
            c::REDEFINITION,
            Some(p::REDEFINITION_REDEFINED_FEATURE),
            SemanticClosureRequirement::EffectiveTyping,
        ),
        (
            "subsetting",
            c::SUBSETTING,
            Some(p::SUBSETTING_SUBSETTED_FEATURE),
            SemanticClosureRequirement::EffectiveTyping,
        ),
        (
            "chaining",
            c::FEATURE_CHAINING,
            None,
            SemanticClosureRequirement::EffectiveTyping,
        ),
        (
            "membership",
            c::MEMBERSHIP,
            None,
            SemanticClosureRequirement::EffectiveMembership,
        ),
    ] {
        let mut f = Fixture::new();
        f.create(1, c::FEATURE);
        f.create(2, c::FEATURE);
        f.create(3, class);
        if let Some(property) = target_property {
            f.value(3, property, Value::Reference(id(2)));
        } else {
            f.own(1, 3);
        }
        let construction = f.construction();
        assert!(!construction.obligations().is_empty(), "{label}");
        let registry = ProducerRegistry::new([ProducerDescriptor::new(
            TYPE,
            [ProducerEffect::Typing],
            ProducerApplicability::Never,
        )])
        .unwrap();
        let context =
            SemanticContext::for_construction(&construction, Default::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert!(
            !certificate.is_closed(id(1), requirement),
            "{label} pending provider {:?} cannot close {requirement:?}",
            construction.obligations()
        );
    }
}

#[test]
fn initial_closure_requires_a_closed_pending_namespace_owner_provider() {
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.value(1, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.create(2, c::BEHAVIOR);
    f.create(3, c::FEATURE_MEMBERSHIP);
    f.own(2, 3);
    // OwningMembership endpoints are derived from their owned-related collection.
    // This missing source population is marked by the pending namespace contract,
    // even though it is not a required stored-slot construction obligation.
    let construction = f.construction();
    assert!(construction.obligations().is_empty());
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Never,
    )])
    .unwrap();
    let context = SemanticContext::for_project_construction(
        &construction,
        Default::default(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::from([id(2)]),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    let queries = KerMlQueries::new(context.with_producer_closure(certificate).unwrap());
    let absent = queries
        .formal_constraint_applies(FormalConstraintId::StepSubperformanceSpecialization, id(1));
    assert_eq!(
        absent.completeness,
        Completeness::Incomplete,
        "source provider 2 may still attach FeatureMembership 3 to Step 1: {absent:?}"
    );
}

#[test]
fn rebound_pending_provider_reopens_transitive_requirement_readers() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    let snapshot = fixture();
    let mut descriptor = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Specialization],
        ProducerApplicability::Any,
    );
    descriptor.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    for subject in [id(1), id(2), id(3)] {
        table.pending(subject, snapshot.model(), &registry);
        table
            .record(&[(subject, TYPE, Completeness::Complete)], &registry)
            .unwrap();
        let reads = if subject == id(3) {
            vec![ProducerRead::Requirement(
                id(1),
                SemanticClosureRequirement::EffectiveTyping,
            )]
        } else {
            vec![ProducerRead::Structural(subject)]
        };
        table.record_reads(&[(subject, TYPE, reads.into())], &registry);
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert!(certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
    let checkpoint = certificate.checkpoint(&context).unwrap();
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot,
        owned: BTreeMap::new(),
    };
    f.create(4, c::FEATURE_TYPING);
    f.value(4, p::FEATURE_TYPING_TYPE, Value::Reference(id(2)));
    let construction = f.construction();
    let new = SemanticContext::for_construction(&construction, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = checkpoint.rebind(&new, &registry).unwrap();
    assert!(
        !rebound
            .certificate
            .is_closed(id(1), SemanticClosureRequirement::EffectiveTyping)
    );
    assert!(
        !rebound
            .certificate
            .is_closed(id(3), SemanticClosureRequirement::EffectiveTyping),
        "a retained producer depends on newly open provider requirement 1: {rebound:?}"
    );
}

#[test]
fn initial_pending_namespace_keeps_unattached_owning_membership_open() {
    let mut f = Fixture::new();
    f.create(1, c::STEP);
    f.value(1, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.create(2, c::BEHAVIOR);
    f.create(3, c::FEATURE_MEMBERSHIP);
    f.changes.set(
        id(3),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(1))]),
        origin(),
    );
    let construction = f.construction();
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Never,
    )])
    .unwrap();
    let context = SemanticContext::for_project_construction(
        &construction,
        Default::default(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::from([id(2)]),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    let queries = KerMlQueries::new(context.with_producer_closure(certificate).unwrap());
    assert_eq!(queries.owning_relationship(id(1)).value, Some(id(3)));
    let absent = queries
        .formal_constraint_applies(FormalConstraintId::StepSubperformanceSpecialization, id(1));
    assert_eq!(
        absent.completeness,
        Completeness::Incomplete,
        "pending namespace 2 can attach existing membership 3 and give Step 1 an owning Type: {absent:?}"
    );
}

#[test]
fn declared_source_population_ignores_derived_adoptions_and_preserves_transport() {
    use crate::producer_closure::{ProducerRead, producer_reads};
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    f.create(4, c::MEMBERSHIP);
    f.value(4, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(2)));
    let snapshot = f.finish();
    let fresh_slots: Vec<_> = snapshot
        .model()
        .element(id(4))
        .unwrap()
        .slots()
        .map(|(property, slot)| (property, slot.value().clone()))
        .collect();
    let mut builder = DerivationBuilder::new(snapshot);
    let fresh = DerivationKey {
        subject: id(1),
        rule: RuleId::from_u128(99881),
        output: OutputKey::from_u128(1),
    };
    builder.element(fresh, c::MEMBERSHIP, fresh_slots, BTreeSet::new());
    builder.extend_ordered_references(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![id(4), fresh.element_id()],
        agq_kernel::provenance::Explanation {
            rule: fresh.rule,
            dependencies: BTreeSet::new(),
        },
    );
    let overlay = builder.build().unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new()).unwrap(),
    );
    let selected = q.declared_owned_relationships(id(1));
    assert_eq!(selected.value, vec![id(3)]);
    assert_eq!(selected.completeness, Completeness::Complete);
    assert_eq!(
        q.owned_relationships(id(1)).value,
        vec![id(3), id(4), fresh.element_id()]
    );
    let original = ProducerRead::DeclaredProperty(id(1), p::ELEMENT_OWNED_RELATIONSHIP);
    let reads = producer_reads(&selected, overlay.model());
    assert!(reads.contains(&original));
    assert!(!reads.contains(&ProducerRead::Property(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP
    )));
    for effect in [
        ProducerEffect::Membership,
        ProducerEffect::Ownership,
        ProducerEffect::Naming,
        ProducerEffect::Scalar(p::ELEMENT_OWNED_RELATIONSHIP),
    ] {
        assert!(!crate::producer_closure::effect_changes_read(
            effect,
            &original,
            overlay.model()
        ));
    }
    let persisted = crate::read_dependencies::structural_searches(&selected);
    assert!(persisted.contains(&StructuralSearch::DeclaredProperty {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP
    }));
    let roundtrip: BTreeSet<StructuralSearch> =
        serde_json::from_str(&serde_json::to_string(&persisted).unwrap()).unwrap();
    let mut reread = q.result(());
    reread
        .search_dependencies
        .extend(roundtrip.into_iter().map(SearchDependency::Kernel));
    assert!(producer_reads(&reread, overlay.model()).contains(&original));
    // Broad observation remains broad in either evaluation order, even a
    // persisted canonical fact without an explicit current Property search.
    let fact = FactKey::Property {
        element: id(1),
        property: p::ELEMENT_OWNED_RELATIONSHIP,
    };
    let mut current_fact = q.result(());
    q.fact(&mut current_fact, fact);
    for broad_first in [false, true] {
        let mut combined = q.result(());
        if broad_first {
            combined.merge(q.owned_relationships(id(1)));
        }
        combined.merge(selected.clone());
        if !broad_first {
            combined.merge(q.owned_relationships(id(1)));
        }
        assert!(
            producer_reads(&combined, overlay.model()).contains(&ProducerRead::Property(
                id(1),
                p::ELEMENT_OWNED_RELATIONSHIP
            ))
        );
        for broad in [q.canonical_fact_evidence(fact), current_fact.clone()] {
            let mut combined = q.result(());
            if broad_first {
                combined.merge(broad.clone());
            }
            combined.merge(selected.clone());
            if !broad_first {
                combined.merge(broad);
            }
            assert!(
                producer_reads(&combined, overlay.model()).contains(&ProducerRead::Property(
                    id(1),
                    p::ELEMENT_OWNED_RELATIONSHIP
                ))
            );
        }
    }
}

#[test]
fn declared_source_population_closes_against_producers_but_reopens_on_source_edit() {
    use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    f.create(3, c::MEMBERSHIP);
    f.value(3, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(2)));
    let snapshot = f.finish();
    let mut creator = ProducerDescriptor::new(
        ACTIVATE,
        [ProducerEffect::Membership],
        ProducerApplicability::Any,
    );
    creator.scope = ProducerEffectScope::Model;
    let mut typing = ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    typing.scope = ProducerEffectScope::Subject;
    let registry = ProducerRegistry::new([creator, typing]).unwrap();
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let q = KerMlQueries::new(context.fork());
    let empty = q.declared_owned_relationships(id(1));
    assert!(empty.value.is_empty());
    assert!(!empty.search_dependencies.is_empty());
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
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
    table
        .record(&[(id(2), TYPE, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(2), TYPE, producer_reads(&empty, snapshot.model()))],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    assert_eq!(
        certificate.evaluation(id(2), registry.index(TYPE).unwrap()),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
    let checkpoint = certificate.checkpoint(&context).unwrap();
    let mut changes = snapshot.change_set();
    changes.set(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(3))]),
        origin(),
    );
    let changed = snapshot.apply(&changes).unwrap();
    let next = SemanticContext::for_snapshot(&changed, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = checkpoint.rebind(&next, &registry).unwrap();
    assert!(rebound.reopened_evaluations > 0);
    assert_eq!(
        rebound
            .certificate
            .evaluation(id(2), registry.index(TYPE).unwrap()),
        Some(ProducerEvaluationState::Pending)
    );
    assert_eq!(
        KerMlQueries::new(next)
            .declared_owned_relationships(id(1))
            .value,
        vec![id(3)]
    );
}

#[test]
fn declared_source_population_does_not_close_pending_namespace_or_member() {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.create(2, c::FEATURE);
    f.create(3, c::MEMBERSHIP);
    f.own(1, 3);
    let candidate = f.construction();
    assert!(!candidate.obligations().is_empty());
    let context = SemanticContext::for_project_construction(
        &candidate,
        Default::default(),
        BTreeSet::new(),
        BTreeSet::new(),
        BTreeSet::from([id(1)]),
    )
    .unwrap();
    let q = KerMlQueries::new(context);
    let mut result = q.declared_owned_relationships(id(1));
    assert_eq!(result.value, vec![id(3)]);
    assert_eq!(result.completeness, Completeness::Incomplete);
    let member = q.member(id(3));
    assert_eq!(member.completeness, Completeness::Incomplete);
    result.merge(member);
    assert_eq!(result.completeness, Completeness::Incomplete);
}

fn declared_population_alias_pair(
    mut f: Fixture,
) -> (
    agq_kernel::derived::DerivedOverlay,
    agq_kernel::derived::DerivedOverlay,
) {
    use agq_kernel::derived::DerivationBuilder;
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    for member in [3, 4, 5] {
        f.create(member, c::MEMBERSHIP);
        f.value(
            member,
            p::MEMBERSHIP_MEMBER_ELEMENT,
            Value::Reference(id(2)),
        );
    }
    f.own(1, 3);
    let before = f.finish();
    let mut changes = before.change_set();
    changes.set(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(3)), Value::Reference(id(4))]),
        origin(),
    );
    let after = before.apply(&changes).unwrap();
    let overlay = |snapshot: Snapshot, additions: Vec<ElementId>| {
        let mut builder = DerivationBuilder::new(snapshot);
        builder.extend_ordered_references(
            id(1),
            p::ELEMENT_OWNED_RELATIONSHIP,
            additions,
            agq_kernel::provenance::Explanation {
                rule: RuleId::from_u128(8829),
                dependencies: BTreeSet::from([
                    Dependency::Declared(FactKey::Element(id(4))),
                    Dependency::Declared(FactKey::Element(id(5))),
                ]),
            },
        );
        builder.build().unwrap()
    };
    let before = overlay(before, vec![id(4), id(5)]);
    let after = overlay(after, vec![id(5)]);
    assert_eq!(before.model().element(id(1)), after.model().element(id(1)));
    (before, after)
}

#[test]
fn declared_population_reconstruction_reopens_equal_aggregate_frontier() {
    use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
    let (before, after) = declared_population_alias_pair(Fixture::new());
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    )])
    .unwrap();
    let before_context = SemanticContext::for_overlay(&before, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let after_context = SemanticContext::for_overlay(&after, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let before_query = KerMlQueries::new(before_context.fork()).declared_owned_relationships(id(1));
    let after_query = KerMlQueries::new(after_context.fork()).declared_owned_relationships(id(1));
    assert_eq!(before_query.value, vec![id(3)]);
    assert_eq!(after_query.value, vec![id(3), id(4)]);
    let mut table = ProducerEvaluationTable::default();
    for record in before.model().elements() {
        table.pending(record.id(), before.model(), &registry);
    }
    table
        .record(&[(id(2), TYPE, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(2), TYPE, producer_reads(&before_query, before.model()))],
        &registry,
    );
    let certificate = ProducerClosureCertificate::issue(
        before.model(),
        before_context.id(),
        &registry,
        &table,
        |_| None,
    );
    let direct_attachment_rejected = after_context
        .fork()
        .with_producer_closure(Arc::new(certificate.clone()))
        .is_err();
    let rebound = certificate
        .checkpoint(&before_context)
        .unwrap()
        .rebind(&after_context, &registry)
        .unwrap();
    assert_eq!(
        (
            direct_attachment_rejected,
            rebound
                .certificate
                .evaluation(id(2), registry.index(TYPE).unwrap())
        ),
        (true, Some(ProducerEvaluationState::Pending)),
        "original declaration population changed beneath an identical aggregate; digest_equal={}, affected={:?}, retained={}",
        before_context.id().model_digest == after_context.id().model_digest,
        rebound.affected_subjects,
        rebound.retained_evaluations
    );
}

#[test]
fn mixed_declared_and_current_fact_reads_remain_current_before_first_append() {
    use crate::producer_closure::{ProducerRead, producer_reads};
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    f.create(2, c::FEATURE);
    member(&mut f, 1, 2, 3, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    let context =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    for q in [
        KerMlQueries::new(context.fork()),
        KerMlQueries::for_production(context),
    ] {
        let source = q.declared_owned_relationships(id(1));
        let mut current = q.result(());
        q.fact(
            &mut current,
            FactKey::Property {
                element: id(1),
                property: p::ELEMENT_OWNED_RELATIONSHIP,
            },
        );
        for source_first in [false, true] {
            let mut combined = q.result(());
            if source_first {
                combined.merge(source.clone());
            }
            combined.merge(current.clone());
            if !source_first {
                combined.merge(source.clone());
            }
            assert!(
                producer_reads(&combined, snapshot.model()).contains(&ProducerRead::Property(
                    id(1),
                    p::ELEMENT_OWNED_RELATIONSHIP
                )),
                "a broad current fact read must still reopen on the first derived append; source_first={source_first}"
            );
            assert!(
                crate::read_dependencies::structural_searches(&combined).contains(
                    &agq_kernel::derived::StructuralSearch::Property {
                        element: id(1),
                        property: p::ELEMENT_OWNED_RELATIONSHIP,
                    }
                )
            );
        }
    }
}

#[test]
fn dependent_archive_preserves_original_population_and_producer_identity() {
    use agq_kernel::{
        archive::{read_dependent_overlay, write_dependent_overlay},
        derived::DerivationBuilder,
    };
    use std::io::Cursor;

    let mut dependency = Fixture::new();
    dependency.create(99, c::PACKAGE);
    let dependency = Arc::new(DerivationBuilder::new(dependency.finish()).build().unwrap());
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let changes = base.change_set();
    let (before, after) = declared_population_alias_pair(Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    });
    let producer_registry = ProducerRegistry::new([ProducerDescriptor::new(
        TYPE,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    )])
    .unwrap();
    let mut restored_identities = vec![];
    for (original, expected) in [(&before, vec![id(3)]), (&after, vec![id(3), id(4)])] {
        let mut bytes = vec![];
        write_dependent_overlay(original, &mut bytes).unwrap();
        let restored = read_dependent_overlay(
            Cursor::new(&bytes),
            Arc::new(agq_kerml::registry().unwrap()),
            dependency.clone(),
        )
        .unwrap();
        assert!(Arc::ptr_eq(
            restored.declared().immutable_dependency().unwrap(),
            &dependency
        ));
        assert!(restored.model().elements().eq(original.model().elements()));
        assert_eq!(
            restored
                .model()
                .declared_slot(id(1), p::ELEMENT_OWNED_RELATIONSHIP),
            original
                .model()
                .declared_slot(id(1), p::ELEMENT_OWNED_RELATIONSHIP)
        );
        let original_context =
            SemanticContext::for_overlay(original, Default::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(producer_registry.digest())
                .unwrap();
        let restored_context =
            SemanticContext::for_overlay(&restored, Default::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(producer_registry.digest())
                .unwrap();
        assert_eq!(original_context.id(), restored_context.id());
        let restored_query = KerMlQueries::new(restored_context.fork());
        assert_eq!(
            restored_query.declared_owned_relationships(id(1)).value,
            expected
        );
        restored_identities.push(restored_context.id().clone());
        let mut rewrite = vec![];
        write_dependent_overlay(&restored, &mut rewrite).unwrap();
        assert_eq!(bytes, rewrite);
    }
    assert_ne!(
        restored_identities[0].model_digest, restored_identities[1].model_digest,
        "producer graph identity must authenticate distinct original populations after restoration"
    );
}
