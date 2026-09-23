use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;

fn fixture(class: MetaclassId, array: bool, inherited_result: bool) -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, class);
    f.create(2, c::FEATURE);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    member(&mut f, 1, 2, 11, c::PARAMETER_MEMBERSHIP);
    f.create(3, c::FEATURE);
    f.enumeration(3, p::FEATURE_DIRECTION, "out");
    if inherited_result {
        f.create(20, c::FUNCTION);
        member(&mut f, 20, 3, 12, c::RETURN_PARAMETER_MEMBERSHIP);
        relation(
            &mut f,
            1,
            20,
            21,
            c::SPECIALIZATION,
            p::SPECIALIZATION_GENERAL,
        );
        f.value(21, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
    } else {
        member(&mut f, 1, 3, 12, c::RETURN_PARAMETER_MEMBERSHIP);
    }
    f.create(4, c::EXPRESSION);
    member(&mut f, 2, 4, 13, c::FEATURE_VALUE);
    f.create(5, c::FEATURE);
    f.enumeration(5, p::FEATURE_DIRECTION, "out");
    member(&mut f, 4, 5, 14, c::RETURN_PARAMETER_MEMBERSHIP);
    f.create(6, c::DATA_TYPE);
    for (index, role) in StandardRole::ALL.into_iter().enumerate() {
        if role != StandardRole::CollectionsArray {
            f.create(1000 + index as u128, role.specification().1);
        }
    }
    if array {
        relation(&mut f, 5, 6, 15, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
        f.value(15, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(5)));
    }
    f.finish()
}

fn registry() -> ProducerRegistry {
    ProducerRegistry::new([ProducerFamily::IndexSelectResult.descriptor(PROFILE)]).unwrap()
}

fn context(snapshot: &Snapshot) -> SemanticContext<'_> {
    let mut context = SemanticContext::for_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: PROFILE,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap();
    // Ordinary algorithm fixture for the roles read by the planner. Accepted
    // standard binding validation has independent publication tests.
    context.id.standard_bindings = Some(Arc::new(StandardKermlBindings {
        targets: StandardRole::ALL
            .into_iter()
            .enumerate()
            .map(|(index, role)| {
                (
                    role,
                    BoundStandardElement {
                        element: id(if role == StandardRole::CollectionsArray {
                            6
                        } else {
                            1000 + index as u128
                        }),
                        library: LibraryId::from_u128(1),
                    },
                )
            })
            .collect(),
        library_set: LibrarySetIdentity {
            artifacts: BTreeMap::from([(
                StandardLibraryArtifact::Semantic,
                LibraryId::from_u128(1),
            )]),
            pins: BTreeSet::new(),
        },
    }));
    context
        .with_producer_registry_digest(registry().digest())
        .unwrap()
}

#[test]
fn pending_index_select_result_cannot_change_nested_argument_result_typing() {
    for class in [c::INDEX_EXPRESSION, c::SELECT_EXPRESSION] {
        let snapshot = fixture(class, false, false);
        assert!(
            snapshot
                .model()
                .registry()
                .is_subtype(class, c::INSTANTIATION_EXPRESSION)
                .unwrap()
        );
        let context = context(&snapshot);
        let q = KerMlQueries::new(context.fork());
        assert_eq!(q.structural_result(id(1)).value, Some(id(3)));
        assert_eq!(q.structural_result(id(4)).value, Some(id(5)));
        assert_eq!(q.owning_type(id(3)).value, Some(id(1)));
        let certificate = ProducerClosureCertificate::initial(&context, &registry()).unwrap();
        assert!(!certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
        for unchanged in [2, 4, 5] {
            assert!(
                certificate.is_closed(id(unchanged), SemanticClosureRequirement::EffectiveTyping),
                "{class:?}: enclosing Index/Select cannot type nested argument subject {unchanged}"
            );
        }
    }
}

#[test]
fn index_select_planner_writes_only_its_direct_result_and_fresh_helpers() {
    for (class, domain) in [
        (c::INDEX_EXPRESSION, ResultDomainRule::IndexResult),
        (c::SELECT_EXPRESSION, ResultDomainRule::SelectResult),
    ] {
        for array in [false, true] {
            let snapshot = fixture(class, array, false);
            let q = KerMlQueries::new(context(&snapshot));
            let plan = q.plan_result_structure([id(1)]);
            assert!(
                plan.producer_evaluations.contains(&(
                    id(1),
                    ProducerFamily::IndexSelectResult.id(),
                    Completeness::Complete
                )),
                "{:?}",
                plan.production
            );
            plan.validate_declared_effects(&[id(1)], &registry())
                .unwrap();
            let result = plan.materialize(&snapshot).unwrap();
            let rule = crate::result_structure::rule_id(PROFILE, domain.constraint());
            let sources: Vec<_> = result
                .overlay
                .model()
                .elements()
                .filter(|record| {
                    matches!(record.origin(), Origin::Derived(origin) if origin.rule == rule)
                        && record.metaclass() == c::SUBSETTING
                })
                .map(|record| {
                    record
                        .slot(p::SUBSETTING_SUBSETTING_FEATURE)
                        .unwrap()
                        .value()
                        .clone()
                })
                .collect();
            assert_eq!(
                sources,
                if class == c::INDEX_EXPRESSION && array {
                    vec![]
                } else {
                    vec![SlotValue::Scalar(Value::Reference(id(3)))]
                }
            );
            let expanded = KerMlQueries::new(
                SemanticContext::for_overlay(
                    &result.overlay,
                    SemanticOptions {
                        baseline_profile: PROFILE,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .unwrap(),
            );
            assert!(expanded.subsetted_features(id(5)).value.is_empty());
        }
    }
}

#[test]
fn index_select_inherited_result_is_not_a_writable_direct_result() {
    for class in [c::INDEX_EXPRESSION, c::SELECT_EXPRESSION] {
        let snapshot = fixture(class, false, true);
        let q = KerMlQueries::new(context(&snapshot));
        assert_eq!(q.structural_result(id(1)).value, Some(id(3)));
        assert_eq!(q.owning_type(id(3)).value, Some(id(20)));
        let plan = q.plan_result_structure([id(1)]);
        assert!(
            !plan
                .producer_evaluations
                .iter()
                .any(|(subject, family, _)| *subject == id(1)
                    && *family == ProducerFamily::IndexSelectResult.id())
        );
        assert!(plan.producer_evaluations.contains(&(
            id(1),
            ProducerFamily::OwnedInstantiationResult.id(),
            Completeness::Incomplete
        )));
        assert!(
            plan.production
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "KQ_RESULT_STAGE")
        );
        // The incomplete operation must not attach a new specialization onto
        // the inherited canonical result shared by other expressions.
        plan.validate_declared_effects(
            &[id(1)],
            &ProducerRegistry::new(
                ProducerFamily::ALL
                    .into_iter()
                    .map(|family| family.descriptor(PROFILE)),
            )
            .unwrap(),
        )
        .unwrap();
        let output = plan.materialize(&snapshot).unwrap();
        let inherited = KerMlQueries::new(
            SemanticContext::for_overlay(
                &output.overlay,
                SemanticOptions {
                    baseline_profile: PROFILE,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap(),
        )
        .subsetted_features(id(3));
        assert!(inherited.value.is_empty());
    }
}

#[test]
fn index_select_future_owner_writers_respect_results_and_keep_unknown_guards() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.IndexSelectReader");
    for class in [c::INDEX_EXPRESSION, c::SELECT_EXPRESSION] {
        for case in [
            "bounded",
            "unknown_creation",
            "ownership_writer",
            "reference_writer",
            "pending_provider",
            "missing_endpoint",
        ] {
            let snapshot = fixture(class, false, false);
            let mut creator = ProducerFamily::IndexSelectResult.descriptor(PROFILE);
            creator.scoped_fresh_ownership = case != "unknown_creation";
            let mut future = ProducerFamily::VariableFeaturing.descriptor(PROFILE);
            if case == "ownership_writer" {
                future.effects.insert(ProducerEffect::Ownership);
            }
            if case == "reference_writer" {
                future.effects.insert(ProducerEffect::Scalar(
                    p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                ));
            }
            let registry = ProducerRegistry::new([
                creator.clone(),
                future,
                ProducerDescriptor::new(
                    READER,
                    [],
                    ProducerApplicability::Subtypes(vec![c::FEATURE]),
                ),
            ])
            .unwrap();
            let options = SemanticOptions {
                baseline_profile: PROFILE,
                ..Default::default()
            };
            let construction = (case == "missing_endpoint").then(|| {
                let mut edit = snapshot.change_set();
                edit.clear(id(12), p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
                snapshot.preview(&edit).unwrap()
            });
            let context = if let Some(construction) = &construction {
                SemanticContext::for_construction(construction, options, BTreeSet::new()).unwrap()
            } else {
                SemanticContext::for_project_snapshot(
                    &snapshot,
                    options,
                    BTreeSet::new(),
                    BTreeSet::new(),
                    if case == "pending_provider" {
                        BTreeSet::from([id(1)])
                    } else {
                        BTreeSet::new()
                    },
                )
                .unwrap()
            }
            .with_producer_registry_digest(registry.digest())
            .unwrap();
            let mut table = ProducerEvaluationTable::default();
            for record in context.model().elements() {
                table.pending(record.id(), context.model(), &registry);
                for descriptor in registry.descriptors() {
                    if descriptor
                        .applicability
                        .applies(context.model(), record.metaclass())
                    {
                        table
                            .record(
                                &[(
                                    record.id(),
                                    descriptor.id,
                                    if descriptor.id == creator.id {
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
                                if descriptor.id == READER {
                                    vec![ProducerRead::Owned(record.id(), c::FEATURE_MEMBERSHIP)]
                                        .into()
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
                context.model(),
                context.id(),
                &registry,
                &table,
                |_| None,
            );
            for target in [2, 4, 5] {
                assert_eq!(
                    certificate.evaluation(id(target), registry.index(READER).unwrap()),
                    Some(if case == "bounded" {
                        ProducerEvaluationState::EvaluatedComplete
                    } else {
                        ProducerEvaluationState::Pending
                    }),
                    "{class:?} {case} nested reader {target}"
                );
            }
            for target in [1, 3] {
                assert_eq!(
                    certificate.evaluation(id(target), registry.index(READER).unwrap()),
                    Some(ProducerEvaluationState::Pending),
                    "{class:?} {case} actual root {target}"
                );
            }
        }
    }
}
