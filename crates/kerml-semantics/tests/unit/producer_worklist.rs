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
fn expression_fixture() -> (Snapshot, Arc<StandardKermlBindings>) {
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
        Err(PublicationOverlayError::Derivation(
            agq_kernel::derived::DerivationError::InputContextMismatch
        ))
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
