use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn variable_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    let mut targets = BTreeMap::new();
    for (index, role) in StandardRole::ALL.into_iter().enumerate() {
        let element = 1000 + index as u128;
        f.create(element, role.specification().1);
        targets.insert(
            role,
            BoundStandardElement {
                element: id(element),
                library: LibraryId::from_u128(1),
            },
        );
    }
    let snapshots = targets[&StandardRole::OccurrenceSnapshots].element;
    let occurrence = targets[&StandardRole::Occurrence].element;
    member(
        &mut f,
        occurrence.as_u128(),
        snapshots.as_u128(),
        500,
        c::FEATURE_MEMBERSHIP,
    );
    relation(
        &mut f,
        snapshots.as_u128(),
        occurrence.as_u128(),
        501,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        501,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(snapshots),
    );
    f.create(1, c::CLASS);
    relation(
        &mut f,
        1,
        occurrence.as_u128(),
        502,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(502, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
    for n in [10, 11] {
        f.create(n, c::FEATURE);
        f.value(n, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
        member(&mut f, 1, n, n + 100, c::FEATURE_MEMBERSHIP);
    }
    let snapshot = f.finish();
    // Algorithm fixture; exact-path, per-artifact binding validation has separate
    // integration coverage. No constructor bypass is exposed to production callers.
    let bindings = Arc::new(StandardKermlBindings {
        targets,
        library_set: LibrarySetIdentity {
            artifacts: BTreeMap::from([(
                StandardLibraryArtifact::Semantic,
                LibraryId::from_u128(1),
            )]),
            pins: BTreeSet::new(),
        },
    });
    (snapshot, bindings)
}

fn nested_variable_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
    let (base, bindings) = variable_fixture();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(9, c::CLASS);
    member(&mut f, 9, 1, 90, c::OWNING_MEMBERSHIP);
    (f.finish(), bindings)
}

#[test]
fn variable_featuring_actual_plan_writes_featuring_only_on_the_variable() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let (snapshot, bindings) = nested_variable_fixture();
    let mut context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap();
    context.id.standard_bindings = Some(bindings);
    let q = KerMlQueries::for_production(context);
    let plan = q.plan_result_structure([id(10)]);
    assert!(plan.producer_evaluations.contains(&(
        id(10),
        ProducerFamily::VariableFeaturing.id(),
        Completeness::Complete,
    )));
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(profile)),
    )
    .unwrap();
    plan.validate_declared_effects(&[id(10)], &registry)
        .unwrap();
    let result = plan.materialize(&snapshot).unwrap();
    let model = result.overlay.model();
    let refs = |element, property| {
        model
            .navigation_slot(element, property)
            .into_iter()
            .flat_map(|slot| slot.value().values())
            .filter_map(|value| match value {
                Value::Reference(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let mut featuring = 0;
    let mut memberships = 0;
    for record in model
        .elements()
        .filter(|record| snapshot.model().element(record.id()).is_none())
    {
        if model
            .registry()
            .is_subtype(record.metaclass(), c::TYPE_FEATURING)
            .unwrap()
        {
            featuring += 1;
            assert_eq!(
                refs(record.id(), p::TYPE_FEATURING_FEATURE_OF_TYPE),
                [id(10)]
            );
        }
        if model
            .registry()
            .is_subtype(record.metaclass(), c::FEATURE_MEMBERSHIP)
            .unwrap()
        {
            memberships += 1;
            assert_eq!(
                model
                    .incoming_for_property(record.id(), p::ELEMENT_OWNED_RELATIONSHIP)
                    .map(|reference| reference.source)
                    .collect::<Vec<_>>(),
                [id(1)]
            );
            assert!(
                refs(record.id(), p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
                    .iter()
                    .all(|id| snapshot.model().element(*id).is_none())
            );
        }
        if model
            .registry()
            .is_subtype(record.metaclass(), c::REDEFINITION)
            .unwrap()
        {
            assert!(
                refs(record.id(), p::REDEFINITION_REDEFINING_FEATURE)
                    .iter()
                    .all(|id| snapshot.model().element(*id).is_none())
            );
        }
    }
    assert_eq!(featuring, 1);
    assert_eq!(memberships, 1);
}

#[test]
fn variable_featuring_pending_child_cannot_change_ancestor_type_featuring() {
    use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
    const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.VariableFeaturingReader");
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let (snapshot, _) = nested_variable_fixture();
    let registry = ProducerRegistry::new([
        ProducerFamily::VariableFeaturing.descriptor(profile),
        ProducerDescriptor::new(READER, [], ProducerApplicability::Subtypes(vec![c::CLASS])),
    ])
    .unwrap();
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let q = KerMlQueries::for_production(context.fork());
    for (target, class, expected) in [
        (
            1,
            c::TYPE_FEATURING,
            ProducerEvaluationState::EvaluatedComplete,
        ),
        (10, c::TYPE_FEATURING, ProducerEvaluationState::Pending),
        (1, c::MEMBERSHIP, ProducerEvaluationState::Pending),
        (
            1,
            c::FEATURE_TYPING,
            ProducerEvaluationState::EvaluatedComplete,
        ),
        (9, c::MEMBERSHIP, ProducerEvaluationState::EvaluatedComplete),
    ] {
        let answer = q.owned_relationships_of_type(id(target), class);
        assert_eq!(answer.completeness, Completeness::Complete);
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            for descriptor in registry.descriptors() {
                if !descriptor
                    .applicability
                    .applies(snapshot.model(), record.metaclass())
                {
                    continue;
                }
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if record.id() == id(10)
                                && descriptor.id == ProducerFamily::VariableFeaturing.id()
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
                        if record.id() == id(1) && descriptor.id == READER {
                            producer_reads(&answer, snapshot.model())
                        } else {
                            Vec::new().into()
                        },
                    )],
                    &registry,
                );
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
            certificate.evaluation(id(1), registry.index(READER).unwrap()),
            Some(expected),
            "target={target}; class={class:?}"
        );
    }
}
#[test]
fn variable_end_snapshot_cannot_reopen_its_own_crossing_or_positional_rule() {
    use crate::producer_closure::ProducerEvaluationTable;
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let (snapshot, bindings) = nested_variable_fixture();
    let mut changes = snapshot.change_set();
    changes.set(
        id(10),
        p::FEATURE_IS_END,
        SlotValue::Scalar(Value::Boolean(true)),
        origin(),
    );
    let snapshot = snapshot.apply(&changes).unwrap();
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(profile)),
    )
    .unwrap();
    let mut context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    context.id.standard_bindings = Some(bindings);
    let q = KerMlQueries::for_production(context.fork());
    let plan = q.plan_result_structure([id(10)]);
    plan.validate_declared_effects(&[id(10)], &registry)
        .unwrap();
    for family in [
        ProducerFamily::OwnedCrossing,
        ProducerFamily::PositionalRedefinition,
    ] {
        assert!(
            plan.producer_evaluations
                .contains(&(id(10), family.id(), Completeness::Complete))
        );
    }
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
                            if record.id() == id(10)
                                && descriptor.id == ProducerFamily::VariableFeaturing.id()
                            {
                                Completeness::Incomplete
                            } else {
                                Completeness::Complete
                            },
                        )],
                        &registry,
                    )
                    .unwrap();
            }
        }
    }
    table.record_reads(&plan.producer_reads, &registry);
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    for family in [
        ProducerFamily::OwnedCrossing,
        ProducerFamily::PositionalRedefinition,
    ] {
        let reads = plan
            .producer_reads
            .iter()
            .find(|(subject, f, _)| *subject == id(10) && *f == family.id())
            .unwrap();
        assert_eq!(
            certificate.evaluation(id(10), registry.index(family.id()).unwrap()),
            Some(ProducerEvaluationState::EvaluatedComplete),
            "{family:?}: {:?}",
            reads.2
        );
    }
}

fn expression_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
    expression_fixture_with_profile(agq_kerml::BaselineProfile::OPERATIONAL_V8)
}

fn expression_fixture_with_profile(
    profile: agq_kerml::BaselineProfile,
) -> (Snapshot, Arc<StandardKermlBindings>) {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    let mut targets = BTreeMap::new();
    for (index, role) in StandardRole::ALL.into_iter().enumerate() {
        let element = 1000 + index as u128;
        f.create(element, role.specification().1);
        targets.insert(
            role,
            BoundStandardElement {
                element: id(element),
                library: LibraryId::from_u128(1),
            },
        );
    }
    let package = targets[&StandardRole::ControlFunctions].element;
    let standard_target = targets[&StandardRole::FeatureChainSourceTarget].element;
    f.create(500, c::FUNCTION);
    f.value(500, p::ELEMENT_DECLARED_NAME, Value::String(".".into()));
    member(&mut f, package.as_u128(), 500, 501, c::OWNING_MEMBERSHIP);
    f.create(502, c::FEATURE);
    f.enumeration(502, p::FEATURE_DIRECTION, "in");
    member(&mut f, 500, 502, 503, c::PARAMETER_MEMBERSHIP);
    member(
        &mut f,
        502,
        standard_target.as_u128(),
        504,
        c::FEATURE_MEMBERSHIP,
    );
    f.create(900, c::FEATURE);
    f.create(901, c::CLASSIFIER);
    relation(
        &mut f,
        900,
        901,
        902,
        c::FEATURE_TYPING,
        p::FEATURE_TYPING_TYPE,
    );
    f.value(
        902,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(900)),
    );
    f.create(1, c::FEATURE_CHAIN_EXPRESSION);
    f.value(
        1,
        p::FEATURE_CHAIN_EXPRESSION_OPERATOR,
        Value::String(".".into()),
    );
    f.create(2, c::FEATURE);
    f.enumeration(2, p::FEATURE_DIRECTION, "in");
    member(&mut f, 1, 2, 3, c::PARAMETER_MEMBERSHIP);
    f.create(4, c::MEMBERSHIP);
    f.own(1, 4);
    f.value(4, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(900)));
    let snapshot = f.finish();
    let bindings = Arc::new(StandardKermlBindings {
        targets,
        library_set: LibrarySetIdentity {
            artifacts: BTreeMap::from([(
                StandardLibraryArtifact::Semantic,
                LibraryId::from_u128(1),
            )]),
            pins: BTreeSet::new(),
        },
    });
    (snapshot, bindings)
}

fn expression_nested_result_fixture(
    existing_source_target: bool,
) -> (Snapshot, Arc<StandardKermlBindings>) {
    let (base, bindings) =
        expression_fixture_with_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9);
    let mut f = Fixture {
        changes: base.change_set(),
        owned: BTreeMap::from([(
            id(1),
            base.model()
                .navigation_slot(id(1), p::ELEMENT_OWNED_RELATIONSHIP)
                .unwrap()
                .value()
                .values()
                .cloned()
                .collect(),
        )]),
        base,
    };
    f.create(5000, c::FEATURE);
    f.enumeration(5000, p::FEATURE_DIRECTION, "out");
    member(&mut f, 1, 5000, 5001, c::RETURN_PARAMETER_MEMBERSHIP);
    // The input's value is a nested reference expression with its own result;
    // that result is not the chain expression's direct structural result.
    f.create(5002, c::FEATURE_REFERENCE_EXPRESSION);
    member(&mut f, 2, 5002, 5003, c::FEATURE_VALUE);
    f.create(5004, c::FEATURE);
    f.enumeration(5004, p::FEATURE_DIRECTION, "out");
    member(&mut f, 5002, 5004, 5005, c::RETURN_PARAMETER_MEMBERSHIP);
    if existing_source_target {
        f.create(5006, c::FEATURE);
        member(&mut f, 2, 5006, 5007, c::FEATURE_MEMBERSHIP);
    }
    (f.finish(), bindings)
}

#[test]
fn pending_feature_chain_cannot_change_nested_input_expression_result_typing() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    for existing_source_target in [false, true] {
        let (snapshot, _) = expression_nested_result_fixture(existing_source_target);
        let registry =
            ProducerRegistry::new([ProducerFamily::FeatureChainExpression.descriptor(profile)])
                .unwrap();
        let context = SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
        let q = KerMlQueries::new(context.fork());
        assert_eq!(q.structural_result(id(1)).value, Some(id(5000)));
        assert_eq!(q.structural_result(id(5002)).value, Some(id(5004)));
        assert_eq!(
            q.source_target_feature(id(1)).value,
            existing_source_target.then_some(id(5006))
        );
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert!(!certificate.is_closed(id(5000), SemanticClosureRequirement::EffectiveTyping));
        if existing_source_target {
            assert!(!certificate.is_closed(id(5006), SemanticClosureRequirement::EffectiveTyping));
        }
        assert!(
            certificate.is_closed(id(5004), SemanticClosureRequirement::EffectiveTyping),
            "the pending chain cannot specialize a nested input expression's result"
        );
    }
}

#[test]
fn feature_chain_planner_specializes_only_direct_result_and_source_target() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    for existing_source_target in [false, true] {
        let (snapshot, bindings) = expression_nested_result_fixture(existing_source_target);
        let mut context = SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap();
        context.id.standard_bindings = Some(bindings);
        let q = KerMlQueries::new(context);
        let plan = q.plan_result_structure([id(1)]);
        assert!(plan.producer_evaluations.contains(&(
            id(1),
            ProducerFamily::FeatureChainExpression.id(),
            Completeness::Complete
        )));
        let registry = ProducerRegistry::new(
            ProducerFamily::ALL
                .into_iter()
                .map(|family| family.descriptor(profile)),
        )
        .unwrap();
        plan.validate_declared_effects(&[id(1)], &registry).unwrap();
        let output = plan.materialize(&snapshot).unwrap();
        let rules: BTreeSet<_> = [
            "checkFeatureChainExpressionResultSpecialization",
            "checkFeatureChainExpressionTargetRedefinition",
            "checkFeatureChainExpressionSourceTargetRedefinition",
        ]
        .into_iter()
        .map(|rule| crate::result_structure::rule_id(profile, rule))
        .collect();
        let model = output.overlay.model();
        let specialized: BTreeSet<_> = model
            .elements()
            .filter(|record| {
                matches!(record.origin(), Origin::Derived(proof) if rules.contains(&proof.rule))
                    && model
                        .registry()
                        .is_subtype(record.metaclass(), c::SPECIALIZATION)
                        .unwrap()
            })
            .map(|record| {
                let specific = model
                    .registry()
                    .resolve_property(record.metaclass(), p::SPECIALIZATION_SPECIFIC)
                    .unwrap()
                    .unwrap()
                    .id;
                match model
                    .navigation_slot(record.id(), specific)
                    .unwrap()
                    .value()
                {
                    SlotValue::Scalar(Value::Reference(target)) => *target,
                    value => panic!("unexpected specialization endpoint {value:?}"),
                }
            })
            .collect();
        assert_eq!(specialized.len(), 2);
        assert!(specialized.contains(&id(5000)));
        assert!(!specialized.contains(&id(5004)));
        if existing_source_target {
            assert_eq!(specialized, BTreeSet::from([id(5000), id(5006)]));
        } else {
            assert_eq!(
                specialized
                    .iter()
                    .filter(|&&target| snapshot.model().element(target).is_some())
                    .copied()
                    .collect::<BTreeSet<_>>(),
                BTreeSet::from([id(5000)])
            );
        }
    }
}

#[test]
fn feature_chain_scope_bounds_direct_and_future_readers_without_losing_unknown_guards() {
    use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};
    const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.ChainScopeReader");
    const MUTATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.ChainScopeMutator");
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    for case in [
        "bounded",
        "unknown_creation",
        "ownership_writer",
        "reference_writer",
        "pending_provider",
        "missing_endpoint",
    ] {
        let (snapshot, _) = expression_nested_result_fixture(true);
        let mut writer = ProducerFamily::FeatureChainExpression.descriptor(profile);
        writer.scoped_fresh_ownership = case != "unknown_creation";
        let mut descriptors = vec![
            writer.clone(),
            ProducerFamily::VariableFeaturing.descriptor(profile),
            ProducerDescriptor::new(
                READER,
                [],
                ProducerApplicability::Subtypes(vec![c::FEATURE]),
            ),
        ];
        if matches!(case, "ownership_writer" | "reference_writer") {
            let mut mutator = ProducerDescriptor::new(
                MUTATOR,
                [if case == "ownership_writer" {
                    ProducerEffect::Ownership
                } else {
                    ProducerEffect::Scalar(p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
                }],
                ProducerApplicability::Any,
            );
            mutator.scope = ProducerEffectScope::Model;
            descriptors.push(mutator);
        }
        let registry = ProducerRegistry::new(descriptors).unwrap();
        let options = SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        };
        let partial = (case == "missing_endpoint").then(|| {
            let mut edit = snapshot.change_set();
            edit.clear(id(5007), p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
            snapshot.preview(&edit).unwrap()
        });
        let context = if let Some(partial) = &partial {
            assert!(
                writer
                    .scope
                    .selected_targets(partial.model(), id(1), false)
                    .unwrap()
                    .is_err()
            );
            SemanticContext::for_construction(partial, options, BTreeSet::new()).unwrap()
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
        let model = context.model();
        let mut table = ProducerEvaluationTable::default();
        for record in model.elements() {
            table.pending(record.id(), model, &registry);
            for descriptor in registry.descriptors() {
                if !descriptor.applicability.applies(model, record.metaclass()) {
                    continue;
                }
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if descriptor.id == writer.id || descriptor.id == MUTATOR {
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
                            vec![ProducerRead::Owned(record.id(), c::FEATURE_MEMBERSHIP)].into()
                        } else {
                            Vec::new().into()
                        },
                    )],
                    &registry,
                );
            }
        }
        let certificate =
            ProducerClosureCertificate::issue(model, context.id(), &registry, &table, |_| None);
        for target in [5002, 5004] {
            assert_eq!(
                certificate.evaluation(id(target), registry.index(READER).unwrap()),
                Some(if case == "bounded" {
                    ProducerEvaluationState::EvaluatedComplete
                } else {
                    ProducerEvaluationState::Pending
                }),
                "nested reader {target}, case {case}"
            );
        }
        for target in [1, 2, 5000, 5006] {
            assert_eq!(
                certificate.evaluation(id(target), registry.index(READER).unwrap()),
                Some(ProducerEvaluationState::Pending),
                "allowed attachment reader {target}, case {case}"
            );
        }
    }
}

#[test]
fn feature_chain_scope_audits_targets_and_rebinds_changed_containment() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    let (snapshot, _) = expression_nested_result_fixture(true);
    let descriptor = ProducerFamily::FeatureChainExpression.descriptor(profile);
    let registry = ProducerRegistry::new([descriptor.clone()]).unwrap();
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let context = SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    assert_eq!(
        descriptor
            .scope
            .selected_targets(snapshot.model(), id(1), false)
            .unwrap()
            .unwrap(),
        BTreeSet::from([id(1), id(2), id(5000), id(5006)])
    );
    let q = KerMlQueries::new(context.fork());
    for target in [1, 2, 5000, 5002, 5004, 5006] {
        let mut plan = q.plan_result_structure([]);
        let output = plan
            .add_derived_element(
                DerivationKey {
                    subject: id(1),
                    rule: RuleId::from_u128(1000),
                    output: OutputKey::from_u128(target),
                },
                c::SUBSETTING,
                BTreeMap::from([
                    (
                        p::SUBSETTING_SUBSETTING_FEATURE,
                        SlotValue::Scalar(Value::Reference(id(target))),
                    ),
                    (
                        p::SUBSETTING_SUBSETTED_FEATURE,
                        SlotValue::Scalar(Value::Reference(id(900))),
                    ),
                ]),
                Some(id(target)),
                &q.canonical_fact_evidence(FactKey::Element(id(target))),
            )
            .unwrap()
            .unwrap();
        plan.attribute_producer_outputs(id(1), descriptor.id, [output]);
        assert_eq!(
            plan.validate_declared_effects(&[id(1)], &registry).is_ok(),
            [1, 2, 5000, 5006].contains(&target),
            "scope audit target {target}"
        );
    }
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(certificate.is_closed(id(5004), SemanticClosureRequirement::EffectiveTyping));
    let mut edit = snapshot.change_set();
    // Move the formerly nested result into the existing source-target membership.
    edit.set(
        id(5007),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(5004))]),
        origin(),
    );
    edit.clear(id(5005), p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
    let changed = snapshot.apply(&edit).unwrap();
    let next = SemanticContext::for_snapshot(&changed, options, BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let rebound = certificate.rebind(&context, &next, &registry).unwrap();
    assert!(
        !rebound
            .certificate
            .is_closed(id(5004), SemanticClosureRequirement::EffectiveTyping)
    );
    assert!(
        rebound
            .certificate
            .is_closed(id(5006), SemanticClosureRequirement::EffectiveTyping)
    );
    assert_eq!(ProducerEffectScope::SubjectAndOwnedResults as u8, 6);
    assert_eq!(ProducerEffectScope::SubjectAndOwnedFeatures as u8, 7);
}

fn assert_chain_query_ignores_result_snapshot_membership(
    query: impl FnOnce(&KerMlQueries<'_>) -> QueryResult<Option<ElementId>>,
    expected: ElementId,
) {
    assert_chain_reads_ignore_result_snapshot_membership(
        |q| {
            let answer = query(q);
            assert_eq!(answer.completeness, Completeness::Complete);
            assert_eq!(answer.value, Some(expected));
            crate::producer_closure::producer_reads(&answer, q.model())
        },
        true,
        false,
    );
}

fn assert_chain_reads_ignore_result_snapshot_membership(
    read_set: impl FnOnce(&KerMlQueries<'_>) -> crate::producer_closure::ProducerReads,
    existing_source_target: bool,
    materialized: bool,
) {
    use crate::producer_closure::ProducerEvaluationTable;
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    let (snapshot, bindings) = expression_nested_result_fixture(existing_source_target);
    let registry = ProducerRegistry::new([
        ProducerFamily::FeatureChainExpression.descriptor(profile),
        ProducerFamily::VariableFeaturing.descriptor(profile),
    ])
    .unwrap();
    let mut context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    context.id.standard_bindings = Some(bindings.clone());
    let first = materialized.then(|| {
        KerMlQueries::for_production(context.fork())
            .plan_result_structure([id(1)])
            .materialize(&snapshot)
            .unwrap()
    });
    let mut context = if let Some(first) = &first {
        SemanticContext::for_overlay(
            &first.overlay,
            context.id().options.clone(),
            BTreeSet::new(),
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
    } else {
        context
    };
    context.id.standard_bindings = Some(bindings);
    let q = KerMlQueries::for_production(context.fork());
    let model = q.model();
    let reads = read_set(&q);
    let mut table = ProducerEvaluationTable::default();
    for record in model.elements() {
        table.pending(record.id(), model, &registry);
        for descriptor in registry.descriptors() {
            if !descriptor.applicability.applies(model, record.metaclass()) {
                continue;
            }
            table
                .record(
                    &[(
                        record.id(),
                        descriptor.id,
                        if record.id() == id(5000)
                            && descriptor.id == ProducerFamily::VariableFeaturing.id()
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
                    if record.id() == id(1)
                        && descriptor.id == ProducerFamily::FeatureChainExpression.id()
                    {
                        reads.clone()
                    } else {
                        Vec::new().into()
                    },
                )],
                &registry,
            );
        }
    }
    let certificate =
        ProducerClosureCertificate::issue(model, context.id(), &registry, &table, |_| None);
    assert_eq!(
        certificate.evaluation(
            id(1),
            registry
                .index(ProducerFamily::FeatureChainExpression.id())
                .unwrap()
        ),
        Some(ProducerEvaluationState::EvaluatedComplete),
        "a result snapshot cannot displace the established chain query answer: {reads:?}"
    );
}

#[test]
fn first_input_population_does_not_wait_for_result_snapshot_membership() {
    assert_chain_query_ignores_result_snapshot_membership(|q| q.first_input(id(1)), id(2));
}

#[test]
fn reference_referent_does_not_wait_for_later_result_snapshot_membership() {
    assert_chain_query_ignores_result_snapshot_membership(|q| q.reference_referent(id(1)), id(900));
}

#[test]
fn first_input_preserves_direction_order_and_incomplete_candidate_meaning() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
    for case in [
        "return_in",
        "return_out",
        "return_inout",
        "no_in",
        "undirected_return",
        "invalid_return_endpoint",
        "pending_provider",
    ] {
        let (base, _) = expression_nested_result_fixture(true);
        let mut f = Fixture {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
        };
        // A structurally admitted in-directed return still participates in
        // ownedFeatures->select(direction='in'); semantic validation is separate.
        f.changes.set(
            id(1),
            p::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered([5001, 3, 4].map(|n| Value::Reference(id(n))).to_vec()),
            origin(),
        );
        f.enumeration(
            5000,
            p::FEATURE_DIRECTION,
            match case {
                "return_in" => "in",
                "return_inout" => "inout",
                _ => "out",
            },
        );
        if case == "no_in" {
            f.enumeration(2, p::FEATURE_DIRECTION, "inout");
        }
        if case == "undirected_return" {
            f.changes.clear(id(5000), p::FEATURE_DIRECTION);
        }
        if case == "invalid_return_endpoint" {
            f.changes
                .clear(id(5001), p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        }
        let snapshot = f.finish();
        let context = SemanticContext::for_project_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
            BTreeSet::new(),
            if case == "pending_provider" {
                BTreeSet::from([id(1)])
            } else {
                BTreeSet::new()
            },
        )
        .unwrap();
        let q = KerMlQueries::for_production(context);
        let result = q.first_input(id(1));
        assert_eq!(
            result.completeness,
            match case {
                "invalid_return_endpoint" => Completeness::Invalid,
                "pending_provider" => Completeness::Incomplete,
                _ => Completeness::Complete,
            },
            "{case}"
        );
        assert_eq!(
            result.value,
            match case {
                "return_in" => Some(id(5000)),
                "no_in" => None,
                _ => Some(id(2)),
            },
            "{case}"
        );
        for kind in [
            FeaturePopulationKind::Parameter,
            FeaturePopulationKind::Result,
        ] {
            assert!(
                result.search_dependencies.contains(
                    &SearchDependency::StructuralFeaturePopulation { owner: id(1), kind }
                )
            );
        }
        if case == "undirected_return" {
            assert!(
                result
                    .search_dependencies
                    .contains(&SearchDependency::PropertySet {
                        element: id(5000),
                        property: p::FEATURE_DIRECTION,
                    })
            );
        }
    }
}

#[test]
fn feature_chain_source_target_declares_membership_on_its_existing_input() {
    let (snapshot, bindings) = expression_fixture();
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
    let mut context =
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap();
    context.id.standard_bindings = Some(bindings.clone());
    let q = KerMlQueries::new(context);
    let first = q
        .plan_result_structure([id(1)])
        .materialize(&snapshot)
        .unwrap();
    let mut context =
        SemanticContext::for_overlay(&first.overlay, options, BTreeSet::new()).unwrap();
    context.id.standard_bindings = Some(bindings);
    let q = KerMlQueries::new(context);
    assert!(q.direct_features(id(2)).value.is_empty());
    let plan = q.plan_result_structure([id(1)]);
    let descriptors: Vec<_> = ProducerFamily::ALL
        .into_iter()
        .map(|family| family.descriptor(profile))
        .collect();
    plan.validate_declared_effects(
        &[id(1)],
        &ProducerRegistry::new(descriptors.clone()).unwrap(),
    )
    .unwrap();
    let deficient = ProducerRegistry::new(descriptors.into_iter().map(|mut descriptor| {
        if descriptor.id == ProducerFamily::FeatureChainExpression.id() {
            descriptor.effects.remove(&ProducerEffect::Membership);
        }
        descriptor
    }))
    .unwrap();
    let error = plan
        .validate_declared_effects(&[id(1)], &deficient)
        .unwrap_err();
    assert!(
        matches!(error, PublicationOverlayError::ProducerEffectViolation(failure) if failure.semantic_target == id(2))
    );
    let second = plan.materialize_on_overlay(&first.overlay).unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(
            &second.overlay,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    assert_eq!(q.direct_features(id(2)).value.len(), 1);
}

fn crossing_fixture() -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V8).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::ASSOCIATION);
    for (end, ty, membership, typing) in [(10, 20, 30, 40), (11, 21, 31, 41)] {
        f.create(end, c::FEATURE);
        f.create(ty, c::CLASSIFIER);
        f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
        member(&mut f, 1, end, membership, c::END_FEATURE_MEMBERSHIP);
        relation(
            &mut f,
            end,
            ty,
            typing,
            c::FEATURE_TYPING,
            p::FEATURE_TYPING_TYPE,
        );
        f.value(
            typing,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(end)),
        );
    }
    f.create(9, c::FEATURE);
    member(&mut f, 10, 9, 50, c::OWNING_MEMBERSHIP);
    f.finish()
}
fn close(
    snapshot: &Snapshot,
    bindings: Option<&Arc<StandardKermlBindings>>,
    options: PublicationClosureOptions,
) -> PublicationClosure {
    close_result_structure(
        snapshot,
        options,
        |overlay| {
            let mut context = SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .map_err(PublicationOverlayError::Context)?;
            context.id.standard_bindings = bindings.cloned();
            Ok(context)
        },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap()
}
fn compare(
    expected: &PublicationClosure,
    actual: &PublicationClosure,
    bindings: Option<&Arc<StandardKermlBindings>>,
) {
    assert!(actual.converged, "{:?}", actual.stages);
    assert_eq!(expected.completeness, actual.completeness);
    assert!(
        expected
            .overlay
            .model()
            .elements()
            .eq(actual.overlay.model().elements()),
        "canonical records and ownership differ"
    );
    assert!(
        expected
            .overlay
            .model()
            .association_occurrences()
            .eq(actual.overlay.model().association_occurrences()),
        "occurrences differ"
    );
    assert!(
        expected.overlay.facts().eq(actual.overlay.facts()),
        "derived explanations differ"
    );
    let query = |overlay| {
        let mut context = SemanticContext::for_overlay(
            overlay,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap();
        context.id.standard_bindings = bindings.cloned();
        KerMlQueries::new(context)
    };
    let expected_q = query(&expected.overlay);
    let actual_q = query(&actual.overlay);
    assert_eq!(
        expected_q.context(),
        actual_q.context(),
        "semantic digest/search metadata differ"
    );
    let population: Vec<_> = actual.overlay.model().elements().map(|r| r.id()).collect();
    let a = expected_q.audit_expanded_capabilities(population.iter().copied());
    let b = actual_q.audit_publication_capabilities(population.iter().copied());
    assert_eq!(a.checked_items, b.checked_items);
    assert_eq!(
        b.checked_items[&PublicationFamily::IdentityProvenance],
        actual.overlay.facts().count()
    );
    if bindings.is_some() {
        assert_eq!(
            b.checked_items[&PublicationFamily::StandardBindings],
            StandardRole::ALL.len()
        );
    }
    assert_eq!(a.failures, b.failures);
    for subject in population {
        if actual_q.is(subject, c::FEATURE) {
            assert_eq!(
                expected_q.feature_types(subject),
                actual_q.feature_types(subject)
            );
            assert_eq!(
                expected_q.featuring_types(subject),
                actual_q.featuring_types(subject)
            );
            assert_eq!(
                expected_q.cross_feature(subject),
                actual_q.cross_feature(subject)
            );
        }
    }
}
fn permutations(
    snapshot: &Snapshot,
    bindings: Option<&Arc<StandardKermlBindings>>,
    subjects: Option<BTreeSet<ElementId>>,
) {
    let options = PublicationClosureOptions {
        initial_subjects: subjects,
        ..Default::default()
    };
    let expected = close(
        snapshot,
        bindings,
        PublicationClosureOptions {
            strategy: PublicationClosureStrategy::ReferenceFullScan,
            ..options.clone()
        },
    );
    assert!(expected.converged, "{:?}", expected.stages);
    assert_eq!(
        expected.completeness,
        Completeness::Complete,
        "{:?}",
        expected.stages.last()
    );
    for order in [
        PublicationWorklistOrder::Fifo,
        PublicationWorklistOrder::Lifo,
        PublicationWorklistOrder::ReversedInitial,
        PublicationWorklistOrder::Partitioned,
    ] {
        for batch_size in [1, 7, 32] {
            let actual = close(
                snapshot,
                bindings,
                PublicationClosureOptions {
                    order,
                    batch_size,
                    ..options.clone()
                },
            );
            compare(&expected, &actual, bindings);
        }
    }
}
#[test]
fn worklist_matches_fullscan_for_crossing_occurrences_and_negative_searches() {
    permutations(&crossing_fixture(), None, None);
}
#[test]
fn worklist_matches_fullscan_for_shared_variable_domains() {
    let (snapshot, bindings) = variable_fixture();
    permutations(
        &snapshot,
        Some(&bindings),
        Some(BTreeSet::from([id(10), id(11)])),
    );
}
#[test]
fn worklist_matches_fullscan_for_multiple_expression_rounds() {
    let (snapshot, bindings) = expression_fixture();
    permutations(
        &snapshot,
        Some(&bindings),
        Some(BTreeSet::from([id(1), id(2)])),
    );
}
#[test]
fn empty_frontier_reuses_overlay_without_recounting_the_previous_build() {
    let snapshot = crossing_fixture();
    let actual = close(&snapshot, None, PublicationClosureOptions::default());
    let expected = close(
        &snapshot,
        None,
        PublicationClosureOptions {
            strategy: PublicationClosureStrategy::ReferenceFullScan,
            ..Default::default()
        },
    );
    compare(&expected, &actual, None);
    let [.., previous, last] = actual.stages.as_slice() else {
        panic!("fixture must derive facts before its empty frontier");
    };
    assert_eq!(last.added_elements, 0);
    assert_eq!(last.added_occurrences, 0);
    assert!(previous.counters.new_proof_sets_interned > 0);
    assert_eq!(
        last.counters.empty_frontiers_reused,
        previous.counters.empty_frontiers_reused + 1
    );
    assert_eq!(
        last.counters.overlay_materializations,
        previous.counters.overlay_materializations
    );
    assert_eq!(
        last.counters.new_proof_sets_interned,
        previous.counters.new_proof_sets_interned
    );
    assert_eq!(
        last.counters.existing_proof_sets_reused,
        previous.counters.existing_proof_sets_reused
    );
    assert_eq!(
        last.counters.search_sets_interned,
        previous.counters.search_sets_interned
    );
    assert_eq!(
        last.counters.search_sets_reused,
        previous.counters.search_sets_reused
    );
    assert_eq!(
        last.counters.logical_search_entries,
        previous.counters.logical_search_entries
    );
    assert_eq!(
        last.counters.retained_search_entries,
        previous.counters.retained_search_entries
    );
    assert_eq!(expected.counters.empty_frontiers_reused, 0);
    assert_eq!(
        expected.counters.overlay_materializations,
        expected.counters.fixed_point_rounds + 1
    );
    assert!(actual.counters.active_dependency_edges > 0);
    assert!(actual.counters.active_dependency_keys > 0);
    assert!(actual.counters.active_dependency_subjects > 0);
}

#[test]
fn inapplicable_population_requires_only_the_initial_materialization() {
    let snapshot = synthetic_publication_fixture(0, 4);
    let actual = close(&snapshot, None, PublicationClosureOptions::default());
    assert!(actual.converged);
    assert_eq!(actual.completeness, Completeness::Complete);
    assert_eq!(actual.counters.overlay_materializations, 1);
    assert_eq!(actual.counters.empty_frontiers_reused, 1);
    assert_eq!(actual.counters.new_proof_sets_interned, 0);
    assert_eq!(actual.counters.existing_proof_sets_reused, 0);
    assert_eq!(actual.counters.active_dependency_subjects, 0);
    assert_eq!(actual.counters.active_dependency_keys, 0);
    assert_eq!(actual.counters.active_dependency_edges, 0);
    assert!(
        snapshot
            .model()
            .elements()
            .eq(actual.overlay.model().elements())
    );
}

#[test]
fn derived_siblings_share_identical_negative_search_evidence() {
    let snapshot = crossing_fixture();
    let result = close(&snapshot, None, PublicationClosureOptions::default());
    let searches: Vec<_> = result.overlay.model().computation_searches().collect();
    let mut shared = 0;
    for (index, (_, left)) in searches.iter().enumerate() {
        for (_, right) in &searches[index + 1..] {
            if !left.is_empty() && left == right {
                assert!(
                    std::ptr::eq(*left, *right),
                    "equal search evidence must remain shared in the published graph"
                );
                shared += 1;
            }
        }
    }
    assert!(shared > 0, "fixture must exercise shared negative searches");
    assert!(result.counters.retained_search_entries < result.counters.logical_search_entries);
    assert!(result.counters.retained_search_sets < result.counters.logical_search_sets);
}

#[test]
fn resource_limit_does_not_claim_closure() {
    let (snapshot, bindings) = expression_fixture();
    let result = close(
        &snapshot,
        Some(&bindings),
        PublicationClosureOptions {
            max_rounds: 1,
            initial_subjects: Some(BTreeSet::from([id(1), id(2)])),
            ..Default::default()
        },
    );
    assert!(!result.converged);
    assert_eq!(result.completeness, Completeness::Incomplete);
}

struct AdditionalSpecialization {
    general: ElementId,
}
impl PublicationProducerExtension for AdditionalSpecialization {
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        model.registry().is_subtype(class, c::CLASS).unwrap()
    }
    fn contribute<'m>(
        &self,
        q: &KerMlQueries<'m>,
        subject: ElementId,
        _: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), agq_kernel::derived::DerivationError> {
        if subject != id(1) {
            return Ok(());
        }
        let supers = q.supertypes(subject);
        let satisfied = supers.value.contains(&self.general);
        let mut evidence = supers.map(|_| ());
        evidence
            .merge_evidence(q.canonical_fact_evidence(FactKey::Element(self.general)))
            .unwrap();
        if satisfied {
            return plan.observe_evidence(evidence);
        }
        plan.add_derived_element(
            DerivationKey {
                rule: RuleId::from_u128(9901),
                subject,
                output: OutputKey::from_u128(9902),
            },
            c::SUBCLASSIFICATION,
            BTreeMap::from([
                (
                    p::SUBCLASSIFICATION_SUBCLASSIFIER,
                    SlotValue::Scalar(Value::Reference(subject)),
                ),
                (
                    p::SUBCLASSIFICATION_SUPERCLASSIFIER,
                    SlotValue::Scalar(Value::Reference(self.general)),
                ),
            ]),
            Some(subject),
            &evidence,
        )?;
        Ok(())
    }
}

#[test]
fn extension_specialization_dirties_kerml_variable_producers_in_every_order() {
    let (original, bindings) = variable_fixture();
    let mut changes = original.change_set();
    changes.remove(id(502));
    changes.set(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(110)), Value::Reference(id(111))]),
        origin(),
    );
    let snapshot = original.apply(&changes).unwrap();
    let extension = AdditionalSpecialization {
        general: bindings.get(StandardRole::Occurrence),
    };
    let run = |options| {
        close_result_structure_with_extension(
            &snapshot,
            options,
            |overlay| {
                let mut context = SemanticContext::for_overlay(
                    overlay,
                    SemanticOptions {
                        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .map_err(PublicationOverlayError::Context)?;
                context.id.standard_bindings = Some(bindings.clone());
                Ok(context)
            },
            &extension,
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap()
    };
    let expected = run(PublicationClosureOptions {
        strategy: PublicationClosureStrategy::ReferenceFullScan,
        ..Default::default()
    });
    assert!(expected.converged);
    assert_eq!(
        expected.completeness,
        Completeness::Complete,
        "{:?}",
        expected.stages.last()
    );
    assert!(
        expected
            .overlay
            .model()
            .instances(c::TYPE_FEATURING, true)
            .unwrap()
            .count()
            >= 2
    );
    for order in [
        PublicationWorklistOrder::Fifo,
        PublicationWorklistOrder::Lifo,
        PublicationWorklistOrder::ReversedInitial,
        PublicationWorklistOrder::Partitioned,
    ] {
        for batch_size in [1, 7] {
            let actual = run(PublicationClosureOptions {
                order,
                batch_size,
                ..Default::default()
            });
            compare(&expected, &actual, Some(&bindings));
            assert!(actual.counters.dirty_reevaluations > 0);
        }
    }
}

#[test]
fn immutable_dependency_is_never_a_producer_subject_even_if_requested() {
    struct Observe(std::cell::RefCell<BTreeSet<ElementId>>);
    impl PublicationProducerExtension for Observe {
        fn applies(&self, _: &ModelView, _: MetaclassId) -> bool {
            true
        }
        fn contribute<'m>(
            &self,
            _: &KerMlQueries<'m>,
            subject: ElementId,
            _: ResultStructureStratum,
            _: &mut ResultStructurePlan<'m>,
        ) -> Result<(), agq_kernel::derived::DerivationError> {
            self.0.borrow_mut().insert(subject);
            Ok(())
        }
    }
    let mut f = Fixture::new();
    f.create(1, c::CLASS);
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(f.finish())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(2, c::CLASS);
    let snapshot = f.finish();
    let extension = Observe(Default::default());
    let result = close_result_structure_with_extension(
        &snapshot,
        PublicationClosureOptions {
            initial_subjects: Some(BTreeSet::from([id(1), id(2)])),
            ..Default::default()
        },
        |overlay| {
            SemanticContext::for_overlay(overlay, Default::default(), BTreeSet::new())
                .map_err(PublicationOverlayError::Context)
        },
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(result.converged);
    assert_eq!(*extension.0.borrow(), BTreeSet::from([id(2)]));
    assert_eq!(result.counters.declared_subjects, 1);
    assert!(Arc::ptr_eq(
        result.overlay.declared().immutable_dependency().unwrap(),
        &dependency
    ));
    assert_eq!(
        dependency.model().element(id(1)),
        result.overlay.model().element(id(1))
    );
}

fn usage_variable_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
    use agq_sysml::{classes as sc, properties as sp};
    let (original, bindings) = variable_fixture();
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V8).unwrap(),
    ));
    let mut changes = base.change_set();
    for record in original.model().elements() {
        let usage = [id(10), id(11)].contains(&record.id());
        changes.create(
            record.id(),
            if usage {
                sc::REFERENCE_USAGE
            } else {
                record.metaclass()
            },
            origin(),
        );
        for (property, slot) in record.slots() {
            if usage && property == p::FEATURE_IS_VARIABLE {
                continue;
            }
            changes.set(record.id(), property, slot.value().clone(), origin());
        }
        if usage {
            changes.set(
                record.id(),
                sp::USAGE_IS_VARIATION,
                SlotValue::Scalar(Value::Boolean(false)),
                origin(),
            );
        }
    }
    for link in original.model().association_occurrences() {
        changes.link(
            link.id(),
            link.association(),
            link.ends().clone(),
            link.positions().clone(),
            origin(),
        );
    }
    let snapshot = base.apply(&changes).unwrap();
    (snapshot, bindings)
}

#[test]
fn stable_usage_property_activates_kerml_after_structural_closure() {
    use agq_sysml::{classes as sc, properties as sp};
    struct UsageProperties;
    impl PublicationProducerExtension for UsageProperties {
        fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
            model.registry().is_subtype(class, sc::USAGE).unwrap()
        }
        fn has_stable_properties(&self) -> bool {
            true
        }
        fn contribute<'m>(
            &self,
            q: &KerMlQueries<'m>,
            subject: ElementId,
            stratum: ResultStructureStratum,
            plan: &mut ResultStructurePlan<'m>,
        ) -> Result<(), agq_kernel::derived::DerivationError> {
            if stratum != ResultStructureStratum::Structural {
                plan.add_derived_property(
                    subject,
                    sp::USAGE_MAY_TIME_VARY,
                    SlotValue::Scalar(Value::Boolean(true)),
                    RuleId::from_u128(9903),
                    &q.canonical_fact_evidence(FactKey::Element(subject)),
                )?;
            }
            Ok(())
        }
    }
    let (snapshot, bindings) = usage_variable_fixture();
    assert!(
        snapshot
            .model()
            .navigation_slot(id(10), p::FEATURE_IS_VARIABLE)
            .is_none()
    );
    let run = |strategy| {
        close_result_structure_with_extension(
            &snapshot,
            PublicationClosureOptions {
                strategy,
                ..Default::default()
            },
            |overlay| {
                let mut context = SemanticContext::for_overlay(
                    overlay,
                    SemanticOptions {
                        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .map_err(PublicationOverlayError::Context)?;
                context.id.standard_bindings = Some(bindings.clone());
                Ok(context)
            },
            &UsageProperties,
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap()
    };
    let worklist = run(PublicationClosureStrategy::Worklist);
    let reference = run(PublicationClosureStrategy::ReferenceFullScan);
    assert_eq!(
        worklist.completeness,
        Completeness::Complete,
        "{:?}",
        worklist.stages.last()
    );
    assert!(
        worklist
            .stages
            .iter()
            .any(|stage| stage.stratum == ResultStructureStratum::StableProperties)
    );
    assert!(
        worklist
            .overlay
            .model()
            .instances(c::TYPE_FEATURING, true)
            .unwrap()
            .count()
            >= 2
    );
    // `Usage::mayTimeVary` redefines KerML `Feature::isVariable`. The stable
    // SysML scalar must dirty the KerML producer and create its canonical
    // snapshot-domain chain, not merely make a later query appear complete.
    let model = worklist.overlay.model();
    let snapshots = bindings.get(StandardRole::OccurrenceSnapshots);
    for usage in [id(10), id(11)] {
        let featuring = model
            .instances(c::TYPE_FEATURING, true)
            .unwrap()
            .find(|relationship| {
                model
                    .navigation_slot(relationship.id(), p::TYPE_FEATURING_FEATURE_OF_TYPE)
                    .is_some_and(|slot| {
                        slot.value().values().any(
                            |value| matches!(value, Value::Reference(feature) if *feature == usage),
                        )
                    })
            })
            .expect("stable mayTimeVary must activate VariableFeaturing");
        let domain = model
            .navigation_slot(featuring.id(), p::TYPE_FEATURING_FEATURING_TYPE)
            .and_then(|slot| {
                slot.value().values().find_map(|value| match value {
                    Value::Reference(id) => Some(*id),
                    _ => None,
                })
            })
            .expect("VariableFeaturing must use a canonical snapshot domain");
        assert!(
            model.incoming(domain).any(|reference| {
                model.element(reference.source).is_some_and(|relationship| {
                    relationship.metaclass() == c::REDEFINITION
                        && model
                            .navigation_slot(
                                relationship.id(),
                                p::REDEFINITION_REDEFINED_FEATURE,
                            )
                            .is_some_and(|slot| {
                                slot.value().values().any(|value| {
                                    matches!(value, Value::Reference(target) if *target == snapshots)
                                })
                            })
                })
            }),
            "snapshot domain must redefine the accepted OccurrenceSnapshots anchor"
        );
    }
    compare(&reference, &worklist, Some(&bindings));
}

#[test]
fn stable_usage_property_retains_contributors_when_variable_featuring_extends_derived_ownership() {
    use agq_sysml::{classes as sc, properties as sp};
    fn specialization_key(subject: ElementId) -> DerivationKey {
        DerivationKey {
            rule: RuleId::from_u128(9904),
            subject,
            output: OutputKey::from_u128(1),
        }
    }
    struct SpecializedUsageProperties {
        general: ElementId,
        ownership_reads: std::cell::Cell<usize>,
    }
    impl PublicationProducerExtension for SpecializedUsageProperties {
        fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
            model.registry().is_subtype(class, sc::USAGE).unwrap()
        }
        fn has_stable_properties(&self) -> bool {
            true
        }
        fn contribute<'m>(
            &self,
            q: &KerMlQueries<'m>,
            subject: ElementId,
            stratum: ResultStructureStratum,
            plan: &mut ResultStructurePlan<'m>,
        ) -> Result<(), agq_kernel::derived::DerivationError> {
            let mut base = q.canonical_fact_evidence(FactKey::Element(subject));
            base.merge_evidence(q.canonical_fact_evidence(FactKey::Element(self.general)))
                .unwrap();
            plan.add_derived_element(
                specialization_key(subject),
                c::SUBSETTING,
                BTreeMap::from([
                    (
                        p::SUBSETTING_SUBSETTING_FEATURE,
                        SlotValue::Scalar(Value::Reference(subject)),
                    ),
                    (
                        p::SUBSETTING_SUBSETTED_FEATURE,
                        SlotValue::Scalar(Value::Reference(self.general)),
                    ),
                ]),
                Some(subject),
                &base,
            )?;
            if stratum != ResultStructureStratum::Structural {
                let owned = q.owned_relationships(subject);
                assert!(
                    owned
                        .value
                        .contains(&specialization_key(subject).element_id())
                );
                assert!(
                    owned.canonical_dependencies.contains(&Dependency::Derived(
                        FactKey::Property {
                            element: subject,
                            property: p::ELEMENT_OWNED_RELATIONSHIP
                        }
                    )),
                    "scalar premise must read the already-derived ownership aggregate"
                );
                self.ownership_reads.set(self.ownership_reads.get() + 1);
                plan.add_derived_property(
                    subject,
                    sp::USAGE_MAY_TIME_VARY,
                    SlotValue::Scalar(Value::Boolean(true)),
                    RuleId::from_u128(9905),
                    &owned.map(|_| ()),
                )?;
            }
            Ok(())
        }
    }
    let (snapshot, bindings) = usage_variable_fixture();
    let run = |strategy, order, batch_size| {
        let extension = SpecializedUsageProperties {
            general: bindings.targets[&StandardRole::DataValues].element,
            ownership_reads: std::cell::Cell::new(0),
        };
        let result = close_result_structure_with_extension(
            &snapshot,
            PublicationClosureOptions {
                strategy,
                order,
                batch_size,
                ..Default::default()
            },
            |overlay| {
                let mut context = SemanticContext::for_overlay(
                    overlay,
                    SemanticOptions {
                        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                        ..Default::default()
                    },
                    BTreeSet::new(),
                )
                .map_err(PublicationOverlayError::Context)?;
                context.id.standard_bindings = Some(bindings.clone());
                Ok(context)
            },
            &extension,
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap();
        assert!(extension.ownership_reads.get() >= 2);
        assert!(result.converged, "{:?}", result.stages);
        assert_eq!(
            result.completeness,
            Completeness::Complete,
            "{:?}",
            result.stages.last()
        );
        for subject in [id(10), id(11)] {
            let fact = FactKey::Property {
                element: subject,
                property: sp::USAGE_MAY_TIME_VARY,
            };
            let proof = result.overlay.explain(fact).expect("derived stable scalar");
            assert!(
                !proof
                    .dependencies
                    .contains(&Dependency::Derived(FactKey::Property {
                        element: subject,
                        property: p::ELEMENT_OWNED_RELATIONSHIP,
                    })),
                "proof must not refer back to an aggregate later extended by VariableFeaturing"
            );
            assert!(
                proof
                    .dependencies
                    .contains(&Dependency::Derived(FactKey::Element(
                        specialization_key(subject).element_id()
                    ))),
                "normalization must preserve the original specialization contributor"
            );
            assert!(
                result
                    .overlay
                    .model()
                    .navigation_slot(subject, p::ELEMENT_OWNED_RELATIONSHIP)
                    .unwrap()
                    .value()
                    .values()
                    .filter_map(|value| match value {
                        Value::Reference(id) => result.overlay.model().element(*id),
                        _ => None,
                    })
                    .any(|record| record.metaclass() == c::TYPE_FEATURING),
                "the stable scalar must activate a later KerML producer on the same owner"
            );
        }
        result
    };
    let expected = run(
        PublicationClosureStrategy::ReferenceFullScan,
        PublicationWorklistOrder::Fifo,
        32,
    );
    for (order, batch_size) in [
        (PublicationWorklistOrder::Fifo, 1),
        (PublicationWorklistOrder::Lifo, 7),
        (PublicationWorklistOrder::ReversedInitial, 16),
        (PublicationWorklistOrder::Partitioned, 32),
    ] {
        let actual = run(PublicationClosureStrategy::Worklist, order, batch_size);
        compare(&expected, &actual, Some(&bindings));
    }
}

/// Explicit workflow scale gate; release runs are invoked under the watchdog.
#[test]
#[ignore = "synthetic publication scale gate; run explicitly in release mode"]
fn publication_scale_sixty_thousand_subjects() {
    run_publication_scale(4000, 60000);
}

/// Same semantic shape as the full scale gate; useful for profiling a bounded
/// probe without weakening or replacing the sixty-thousand-subject gate.
#[test]
#[ignore = "bounded profiling probe; run explicitly in release mode"]
fn publication_scale_probe() {
    let groups = std::env::var("AGQ_PUBLICATION_SCALE_GROUPS")
        .map(|n| n.parse().expect("positive scale group count"))
        .unwrap_or(128);
    run_publication_scale(groups, groups * 15);
}

fn synthetic_publication_fixture(groups: u128, declared: u128) -> Snapshot {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    // Shared Types produce a large incoming-search fan-out. Independent owned
    // crossings require negative searches, occurrence navigation and later
    // derived Feature/FeatureChaining subjects without a global corpus rescan.
    f.create(1, c::CLASSIFIER);
    f.create(2, c::CLASSIFIER);
    for group in 0..groups {
        let n = 100 + group * 16;
        f.create(n, c::ASSOCIATION);
        for (end, ty, membership, typing) in [(n + 1, 1, n + 3, n + 5), (n + 2, 2, n + 4, n + 6)] {
            f.create(end, c::FEATURE);
            f.value(end, p::FEATURE_IS_END, Value::Boolean(true));
            member(&mut f, n, end, membership, c::END_FEATURE_MEMBERSHIP);
            relation(
                &mut f,
                end,
                ty,
                typing,
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPE,
            );
            f.value(
                typing,
                p::FEATURE_TYPING_TYPED_FEATURE,
                Value::Reference(id(end)),
            );
        }
        f.create(n + 7, c::FEATURE);
        member(&mut f, n + 1, n + 7, n + 8, c::OWNING_MEMBERSHIP);
    }
    for n in 0..(declared - (groups * 9 + 2)) {
        f.create(1_000_000 + n, c::PACKAGE);
    }
    let snapshot = f.finish();
    assert_eq!(snapshot.model().len(), declared as usize);
    snapshot
}

fn run_publication_scale(groups: u128, declared: u128) {
    let started = std::time::Instant::now();
    eprintln!("semantic-scale start groups={groups} declared={declared}");
    let snapshot = synthetic_publication_fixture(groups, declared);
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    eprintln!("semantic-scale snapshot elapsed={:?}", started.elapsed());
    let result = close_result_structure(
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
        |round, completed, total, facts| {
            if completed % 1024 == 0 || completed == total {
                eprintln!("semantic-scale round={round} evaluated={completed}/{total} proposed={facts} elapsed={:?}", started.elapsed());
            }
        },
        |stage| eprintln!("semantic-scale frontier={} counters={:?} elapsed={:?}", stage.stage, stage.counters, started.elapsed()),
    )
    .unwrap();
    eprintln!(
        "semantic-scale elapsed={:?} {:?}",
        started.elapsed(),
        result.counters
    );
    assert!(result.converged, "{:?}", result.stages);
    assert_eq!(
        result.completeness,
        Completeness::Complete,
        "{:?}",
        result.stages.last()
    );
    assert!(result.counters.new_elements_proposed >= groups as usize * 5);
    assert!(result.counters.new_association_occurrences_proposed >= groups as usize);
    assert!(
        result.counters.subjects_skipped_by_applicability >= (declared - (groups * 9 + 2)) as usize
    );
    assert!(result.counters.fixed_point_rounds > 1);
    assert!(result.counters.existing_proof_sets_reused > 0);
    assert!(
        result.counters.subjects_evaluated
            < snapshot.model().len() * result.counters.fixed_point_rounds
    );
    assert!(result.overlay.build_metrics().reused_owned_storage);
    // Existing public explanations remain inspectable after compact evaluation.
    for (_, explanation) in result.overlay.facts() {
        assert!(!explanation.dependencies.is_empty());
    }
}

#[test]
fn worklist_matches_fullscan_on_medium_shared_endpoint_population() {
    let snapshot = synthetic_publication_fixture(32, 512);
    let expected = close(
        &snapshot,
        None,
        PublicationClosureOptions {
            strategy: PublicationClosureStrategy::ReferenceFullScan,
            batch_size: 23,
            ..Default::default()
        },
    );
    for (order, batch_size) in [
        (PublicationWorklistOrder::Fifo, 16),
        (PublicationWorklistOrder::Lifo, 47),
    ] {
        let actual = close(
            &snapshot,
            None,
            PublicationClosureOptions {
                order,
                batch_size,
                ..Default::default()
            },
        );
        compare(&expected, &actual, None);
        assert!(!actual.producer_reads.reads_entire_model());
        for shared in [id(1), id(2)] {
            assert!(actual.producer_reads.bounded_elements().contains(&shared));
        }
    }
}

#[test]
fn status_queries_preserve_reference_values_completeness_and_diagnostics() {
    let mut f = Fixture::new();
    f.create(1, c::PACKAGE);
    f.member(1, 10, 2, c::CLASS, "Base");
    f.member(1, 11, 3, c::CLASS, "Derived");
    f.member(1, 12, 4, c::CLASS, "Repeated");
    f.member(1, 13, 5, c::CLASS, "Repeated");
    relation(
        &mut f,
        3,
        2,
        20,
        c::SPECIALIZATION,
        p::SPECIALIZATION_GENERAL,
    );
    f.value(20, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(3)));
    let snapshot = f.finish();
    for pending in [BTreeSet::new(), BTreeSet::from([id(1)])] {
        let q = KerMlQueries::new(
            SemanticContext::for_project_snapshot(
                &snapshot,
                Default::default(),
                BTreeSet::new(),
                BTreeSet::new(),
                pending,
            )
            .unwrap(),
        );
        let status = q.status_queries();
        assert_eq!(status.context(), q.context());
        for name in ["Base", "Missing", "Repeated"] {
            let name = QualifiedName {
                absolute: false,
                segments: vec![name.into()],
            };
            let tracked = status.lookup_relationship_target_with_reads(
                id(20),
                p::SPECIALIZATION_GENERAL,
                &name,
            );
            assert_eq!(
                tracked.outcome,
                status.lookup_relationship_target(id(20), p::SPECIALIZATION_GENERAL, &name)
            );
            assert!(tracked.reads.affected_by(&BTreeSet::from([id(1)]), false));
            assert!(
                !tracked
                    .reads
                    .affected_by(&BTreeSet::from([id(999999)]), false)
            );
            assert!(tracked.reads.affected_by(&BTreeSet::new(), true));
            assert!(!tracked.reads.search_dependencies().is_empty());
            assert!(QueryReadSet::context_compatible(
                q.context(),
                status.context()
            ));
            let mut changed = status.context().clone();
            changed.model_digest = [42; 32];
            changed.pending_namespace_scopes.insert(id(1));
            assert!(QueryReadSet::context_compatible(status.context(), &changed));
            let mut phase = changed.clone();
            phase.derivation_phase = DerivationPhase::CompletePublicationOverlay;
            assert!(!QueryReadSet::context_compatible(status.context(), &phase));
            changed.options.exclude_implied = !changed.options.exclude_implied;
            assert!(!QueryReadSet::context_compatible(
                status.context(),
                &changed
            ));
            assert_eq!(
                status.lookup_relationship_target(id(20), p::SPECIALIZATION_GENERAL, &name),
                QueryOutcome::from(q.lookup_relationship_target(
                    id(20),
                    p::SPECIALIZATION_GENERAL,
                    &name
                ))
            );
            for class in [c::CLASS, c::FEATURE] {
                assert_eq!(
                    status.resolve_name(id(1), &name, class, false),
                    QueryOutcome::from(q.resolve_name(id(1), &name, class, false))
                );
                assert_eq!(
                    status.resolve_reference(id(3), &name, class),
                    QueryOutcome::from(q.resolve_reference(id(3), &name, class))
                );
                assert_eq!(
                    status.fork().resolve_reference(id(3), &name, class),
                    status.resolve_reference(id(3), &name, class)
                );
            }
        }
    }
}

#[test]
fn worklist_matches_fullscan_for_reference_expression_and_feature_values() {
    for directed in [false, true] {
        let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
        let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
        let mut f = Fixture {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
        };
        f.create(1, c::FUNCTION);
        f.create(2, c::FEATURE_REFERENCE_EXPRESSION);
        for n in [3, 4, 5, 6] {
            f.create(n, c::FEATURE);
        }
        member(&mut f, 1, 2, 101, c::RESULT_EXPRESSION_MEMBERSHIP);
        member(&mut f, 2, 3, 102, c::RETURN_PARAMETER_MEMBERSHIP);
        member(&mut f, 1, 4, 103, c::FEATURE_MEMBERSHIP);
        member(&mut f, 1, 5, 104, c::RETURN_PARAMETER_MEMBERSHIP);
        member(&mut f, 1, 6, 105, c::FEATURE_MEMBERSHIP);
        if directed {
            f.enumeration(6, p::FEATURE_DIRECTION, "in");
        }
        f.enumeration(3, p::FEATURE_DIRECTION, "out");
        f.enumeration(5, p::FEATURE_DIRECTION, "out");
        relation(
            &mut f,
            2,
            4,
            106,
            c::MEMBERSHIP,
            p::MEMBERSHIP_MEMBER_ELEMENT,
        );
        type_featuring(&mut f, 2, 1, 107);
        f.create(7, c::EXPRESSION);
        f.create(8, c::FEATURE);
        f.enumeration(8, p::FEATURE_DIRECTION, "out");
        member(&mut f, 7, 8, 108, c::RETURN_PARAMETER_MEMBERSHIP);
        member(&mut f, 6, 7, 109, c::FEATURE_VALUE);
        f.value(109, p::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(false));
        f.value(109, p::FEATURE_VALUE_IS_INITIAL, Value::Boolean(false));
        permutations(&f.finish(), None, None);
    }
}

#[test]
fn missing_scoped_subject_is_invalid_even_when_no_facts_are_added() {
    let snapshot = crossing_fixture();
    for strategy in [
        PublicationClosureStrategy::Worklist,
        PublicationClosureStrategy::ReferenceFullScan,
    ] {
        let result = close(
            &snapshot,
            None,
            PublicationClosureOptions {
                initial_subjects: Some(BTreeSet::from([id(999999)])),
                strategy,
                ..Default::default()
            },
        );
        assert!(result.converged);
        assert_eq!(result.completeness, Completeness::Invalid);
        assert!(
            result
                .stages
                .last()
                .unwrap()
                .diagnostics
                .iter()
                .any(|d| d.code == "KQ_PRODUCER_SUBJECT" && d.subject == id(999999))
        );
    }
}

#[test]
fn deferred_binding_failure_cannot_certify_structural_fixed_point() {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V8).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::FEATURE_REFERENCE_EXPRESSION);
    f.create(2, c::FEATURE);
    f.enumeration(2, p::FEATURE_DIRECTION, "out");
    member(&mut f, 1, 2, 3, c::RETURN_PARAMETER_MEMBERSHIP);
    // Structurally valid records, but no referent exists for the deferred rule.
    let snapshot = f.finish();
    for strategy in [
        PublicationClosureStrategy::Worklist,
        PublicationClosureStrategy::ReferenceFullScan,
    ] {
        let result = close(
            &snapshot,
            None,
            PublicationClosureOptions {
                strategy,
                ..Default::default()
            },
        );
        assert!(result.converged);
        assert_ne!(result.completeness, Completeness::Complete);
        assert_eq!(
            result.stages.first().unwrap().stratum,
            ResultStructureStratum::Structural
        );
        let last = result.stages.last().unwrap();
        assert_eq!(last.stratum, ResultStructureStratum::ContextualBindings);
        assert!(
            last.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "KQ_REFERENCE_REFERENT")
        );
        let limited = close(
            &snapshot,
            None,
            PublicationClosureOptions {
                strategy,
                max_rounds: 1,
                ..Default::default()
            },
        );
        assert!(!limited.converged);
        assert_eq!(limited.completeness, Completeness::Incomplete);
    }
}

#[test]
fn changing_formal_binding_contract_is_rejected_between_frontiers() {
    let snapshot = crossing_fixture();
    let mut calls = 0;
    let result = close_result_structure(
        &snapshot,
        PublicationClosureOptions::default(),
        |overlay| {
            calls += 1;
            let context = SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .map_err(PublicationOverlayError::Context)?;
            Ok(context.with_formal_constraint_targets(&[], LibraryId::from_u128(calls)))
        },
        |_, _, _, _| {},
        |_| {},
    );
    assert!(matches!(
        result,
        Err(PublicationOverlayError::SchedulerContextMismatch { changed_fields })
            if changed_fields == ["formal_constraint_targets"]
    ));
}

#[test]
fn formal_binding_read_metadata_rebinds_to_each_publication_frontier() {
    let snapshot = crossing_fixture();
    let mut digests = BTreeSet::new();
    let result = close_result_structure(
        &snapshot,
        PublicationClosureOptions::default(),
        |overlay| {
            let context = SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .map_err(PublicationOverlayError::Context)?;
            digests.insert(context.id().model_digest);
            Ok(context.with_formal_constraint_targets(&[], LibraryId::from_u128(1)))
        },
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(digests.len() > 1);
    assert!(result.converged);
    assert_eq!(result.completeness, Completeness::Complete);
}

#[test]
fn actual_chain_planner_does_not_wait_for_result_snapshot_membership() {
    for (existing_source_target, materialized) in
        [(false, false), (true, false), (false, true), (true, true)]
    {
        assert_chain_reads_ignore_result_snapshot_membership(
            |q| {
                let plan = q.plan_result_structure([id(1)]);
                assert!(plan.producer_evaluations.contains(&(
                    id(1),
                    ProducerFamily::FeatureChainExpression.id(),
                    Completeness::Complete,
                )));
                plan.producer_reads
                    .iter()
                    .find(|(subject, family, _)| {
                        *subject == id(1) && *family == ProducerFamily::FeatureChainExpression.id()
                    })
                    .expect("actual chain producer read row")
                    .2
                    .clone()
            },
            existing_source_target,
            materialized,
        );
    }
}
