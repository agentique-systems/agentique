use super::*;
use agq_kerml_semantics::{
    LibrarySetIdentity, SearchDependency, StandardLibraryArtifact, StandardRole,
};

#[test]
fn publication_rule_registry_rejects_other_profiles_and_unimplemented_operations() {
    let operational = sysml_producer_rule_ids(SysmlBaselineProfile::OperationalV1);
    let published = sysml_producer_rule_ids(SysmlBaselineProfile::Published);
    assert!(operational.is_disjoint(&published));
    for rule in [
        "checkItemUsageSubitemSpecialization",
        "checkAcceptActionUsageTriggerActionSpecialization",
        "deriveUsageMayTimeVary",
    ] {
        assert!(operational.contains(&SysmlBaselineProfile::OperationalV1.rule_id(rule)));
    }
    assert!(!operational.contains(
        &SysmlBaselineProfile::OperationalV1.rule_id("checkSendActionUsageSpecialization")
    ));
    assert!(!operational.contains(
        &SysmlBaselineProfile::OperationalV1.rule_id("deriveTransitionUsageTriggerAction")
    ));
}

fn item_fixture(composite: bool, with_authored_base: bool) -> Fixture {
    let mut f = Fixture::new();
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    for (n, class, name) in [
        (1, kc::NAMESPACE, "root"),
        (2, kc::LIBRARY_PACKAGE, "Items"),
        (3, sc::ITEM_DEFINITION, "Item"),
        (4, sc::ITEM_USAGE, "items"),
        (5, sc::ITEM_USAGE, "subitems"),
        (6, sc::PART_USAGE, "subparts"),
        (7, kc::LIBRARY_PACKAGE, "Parts"),
        (8, sc::PART_DEFINITION, "Part"),
        (9, sc::PART_USAGE, "parts"),
    ] {
        f.create(n, class, name);
    }
    for (owner, member, class) in [
        (1, 2, kc::OWNING_MEMBERSHIP),
        (2, 3, kc::OWNING_MEMBERSHIP),
        (2, 4, kc::OWNING_MEMBERSHIP),
        (3, 5, kc::FEATURE_MEMBERSHIP),
        (3, 6, kc::FEATURE_MEMBERSHIP),
        (1, 7, kc::OWNING_MEMBERSHIP),
        (7, 8, kc::OWNING_MEMBERSHIP),
        (7, 9, kc::OWNING_MEMBERSHIP),
    ] {
        f.member(owner, member, 100 + member, class);
    }
    f.relation(
        8,
        3,
        208,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f.relation(9, 4, 209, kc::SUBSETTING, kp::SUBSETTING_SUBSETTED_FEATURE);
    f.origin = origin();
    f.create(11, sc::ITEM_DEFINITION, "Container");
    f.create(12, sc::PART_USAGE, "child");
    f.create(13, sc::PART_DEFINITION, "Vehicle");
    f.member(11, 12, 112, kc::FEATURE_MEMBERSHIP);
    f.value(12, kp::FEATURE_IS_COMPOSITE, Value::Boolean(composite));
    if with_authored_base {
        f.relation(
            13,
            8,
            213,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    f
}
fn result<'a>(plan: &'a SysmlProducerPlan, rule: &str) -> &'a SysmlProducerResult {
    plan.results
        .iter()
        .find(|result| result.rule == rule)
        .unwrap()
}

#[test]
fn operational_context_pins_both_manifests_and_preserves_published_target() {
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32]));
    let published = SysmlDependencyContract::checked_in(&bindings).unwrap();
    let operational = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OPERATIONAL_V1,
    )
    .unwrap();
    let operational_v2 = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OPERATIONAL_V2,
    )
    .unwrap();
    assert_ne!(published, operational);
    assert!(published.grammar_compatibility_manifest_digest.is_none());
    assert!(operational.grammar_compatibility_manifest_digest.is_some());
    assert!(published.semantic_correction_manifest_digest.is_none());
    assert!(operational.semantic_correction_manifest_digest.is_some());
    assert_ne!(operational, operational_v2);
    assert_eq!(
        operational.grammar_compatibility_manifest_digest,
        operational_v2.grammar_compatibility_manifest_digest
    );
    assert_ne!(
        operational.semantic_correction_manifest_digest,
        operational_v2.semantic_correction_manifest_digest
    );
    assert_eq!(
        SysmlBaselineProfile::PUBLISHED.composite_item_target(),
        ["Items", "Item", "subitem"]
    );
    assert_eq!(
        SysmlBaselineProfile::OPERATIONAL_V1.composite_item_target(),
        ["Items", "Item", "subitems"]
    );
    assert_eq!(
        published.kerml_publication_digest,
        operational.kerml_publication_digest
    );
    assert_eq!(published.kerml_profile, "agentique-kerml-1.0-operational/9");
}

#[test]
fn positive_bases_use_subclassification_and_subsetting_with_stable_identity() {
    let snapshot = item_fixture(false, false).finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = KerMlQueries::new(context.kerml);
    let bindings = context.bindings;
    let plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V1,
        &bindings,
        &[id(1)],
        id(13),
    );
    let part = result(&plan, "checkPartDefinitionSpecialization");
    assert_eq!(part.evidence.completeness, Completeness::Complete);
    assert_eq!(part.relationships.len(), 1);
    assert_eq!(part.relationships[0].metaclass, kc::SUBCLASSIFICATION);
    assert_eq!(part.relationships[0].general, id(8));
    assert!(
        result(&plan, "checkItemDefinitionSpecialization")
            .relationships
            .is_empty(),
        "Part ancestry establishes Item"
    );
    let repeated = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V1,
        &bindings,
        &[id(1)],
        id(13),
    );
    assert_eq!(
        part.relationships,
        result(&repeated, part.rule).relationships
    );
    let usage = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V1,
        &bindings,
        &[id(1)],
        id(12),
    );
    let parts = result(&usage, "checkPartUsageSpecialization");
    assert_eq!(parts.relationships[0].metaclass, kc::SUBSETTING);
    assert_eq!(parts.relationships[0].general, id(9));
    assert!(
        usage
            .results
            .iter()
            .flat_map(|r| &r.relationships)
            .all(|r| r.metaclass != kc::FEATURE_TYPING)
    );
}

#[test]
fn transitive_existing_base_suppresses_every_redundant_generalization() {
    let snapshot = item_fixture(false, true).finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = SysmlQueries::new(context);
    let plan = queries.producer_plan(&[id(1)], id(13));
    assert_eq!(plan.completeness(), Completeness::Complete);
    assert!(plan.results.iter().all(|r| r.relationships.is_empty()));
}

#[test]
fn composite_item_correction_uses_actual_plural_identity_only_when_applicable() {
    for composite in [false, true] {
        let snapshot = item_fixture(composite, false).finish();
        let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
        let queries = KerMlQueries::new(context.kerml);
        for profile in [
            SysmlBaselineProfile::PUBLISHED,
            SysmlBaselineProfile::OPERATIONAL_V1,
        ] {
            let plan = plan_sysml_producers(&queries, profile, &context.bindings, &[id(1)], id(12));
            let item = result(&plan, "checkItemUsageSubitemSpecialization");
            if !composite {
                assert!(item.relationships.is_empty());
                assert_eq!(item.evidence.completeness, Completeness::Complete);
            } else if profile == SysmlBaselineProfile::PUBLISHED {
                assert!(item.relationships.is_empty());
                assert_eq!(item.evidence.completeness, Completeness::Incomplete);
                assert!(
                    item.evidence
                        .diagnostics
                        .iter()
                        .any(|d| d.code == "SQ_TARGET_MISSING" && d.message.contains("::subitem "))
                );
            } else {
                assert_eq!(item.relationships[0].general, id(5));
                assert_eq!(item.evidence.completeness, Completeness::Complete);
                assert_eq!(
                    result(&plan, "checkPartUsageSubpartSpecialization").relationships[0].general,
                    id(6)
                );
            }
        }
    }
}

#[test]
fn missing_target_retains_empty_namespace_reads_and_pending_inputs_create_no_fact() {
    let snapshot = item_fixture(true, false).finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::from([id(12)]));
    let queries = KerMlQueries::new(context.kerml);
    let plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::PUBLISHED,
        &context.bindings,
        &[id(1)],
        id(12),
    );
    let item = result(&plan, "checkItemUsageSubitemSpecialization");
    assert!(
        item.evidence
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: id(3) })
    );
    assert!(plan.results.iter().all(|r| r.relationships.is_empty()));
    assert_eq!(plan.completeness(), Completeness::Incomplete);
}

#[test]
fn may_time_vary_does_not_read_a_missing_boolean_as_false() {
    let snapshot = item_fixture(false, false).finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = SysmlQueries::new(context);
    let nested = queries.current_may_time_vary(&[id(1)], id(12));
    assert_eq!(nested.value(), &None);
    assert_eq!(nested.completeness(), Completeness::Incomplete);
    assert!(
        nested
            .kerml
            .search_dependencies
            .contains(&SearchDependency::StandardLibraries)
    );
    let root = queries.current_may_time_vary(&[id(1)], id(9));
    assert_eq!(root.value(), &Some(false));
    assert_eq!(root.completeness(), Completeness::Complete);
    let scalar = plan_sysml_may_time_vary(
        queries.kerml(),
        SysmlBaselineProfile::OPERATIONAL_V1,
        &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        &[id(1)],
        id(12),
    );
    assert!(scalar.properties.is_empty());
}

#[test]
fn attribute_usage_formal_target_remains_accepted_kerml_data_values() {
    let mut f = Fixture::new();
    f.create(1, sc::ATTRIBUTE_USAGE, "value");
    let snapshot = f.finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = KerMlQueries::new(context.kerml);
    let plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V1,
        &context.bindings,
        &[],
        id(1),
    );
    let attribute = result(&plan, "checkAttributeUsageSpecialization");
    assert_eq!(attribute.evidence.completeness, Completeness::Incomplete);
    assert!(
        attribute
            .evidence
            .search_dependencies
            .contains(&SearchDependency::StandardLibraries)
    );
    assert_eq!(
        StandardRole::DataValues.specification().0,
        ["Base", "dataValues"]
    );
}

#[test]
fn nested_bindings_validate_definition_prefixes_and_retained_source_gate() {
    let snapshot = item_fixture(false, false).finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = KerMlQueries::new(context.kerml);
    let bindings = StandardSysmlBindings::validate(
        snapshot.model(),
        &queries,
        SystemsLibraryIdentity::pinned(SystemsLibraryIdentity::SOURCE_CONTENT_SET),
        &[id(1)],
        [StandardSysmlRole::Subitems, StandardSysmlRole::Subparts],
    )
    .unwrap();
    assert_eq!(bindings.get(StandardSysmlRole::Subitems), Some(id(5)));
    assert_eq!(bindings.get(StandardSysmlRole::Subparts), Some(id(6)));
    assert!(
        !bindings.sources_verified(),
        "path binding alone is not source acceptance"
    );
    assert_eq!(
        StandardSysmlRole::ViewpointCheck.specification().0,
        ["Views", "ViewpointCheck"]
    );
    assert_eq!(
        StandardSysmlRole::BinaryConnection.specification().0,
        ["Connections", "BinaryConnection"]
    );
}

fn may_time_fixture(
    occurrence_owner: bool,
    composite: bool,
    portion: bool,
    excluded_type: Option<&str>,
) -> (Snapshot, LibrarySetIdentity, Vec<ElementId>) {
    let mut f = Fixture::new();
    let libraries = LibrarySetIdentity {
        artifacts: StandardLibraryArtifact::ALL
            .into_iter()
            .enumerate()
            .map(|(i, artifact)| (artifact, LibraryId::from_u128(900 + i as u128)))
            .collect(),
        pins: BTreeSet::new(),
    };
    let mut next = 20_000u128;
    let mut roots_by_artifact = BTreeMap::new();
    for (&artifact, &library) in &libraries.artifacts {
        f.origin = DeclaredOrigin::StandardLibrary { library };
        f.create(next, kc::NAMESPACE, "root");
        roots_by_artifact.insert(artifact, next);
        next += 1;
    }
    let mut paths: BTreeMap<(StandardLibraryArtifact, Vec<String>), u128> = BTreeMap::new();
    let mut result_owners = Vec::new();
    for role in StandardRole::ALL {
        let artifact = role.library_artifact();
        f.origin = DeclaredOrigin::StandardLibrary {
            library: libraries.artifacts[&artifact],
        };
        let (segments, expected) = role.specification();
        let mut owner = roots_by_artifact[&artifact];
        for index in 0..segments.len() {
            let key = (
                artifact,
                segments[..=index].iter().map(|s| (*s).to_owned()).collect(),
            );
            if let Some(&existing) = paths.get(&key) {
                owner = existing;
                continue;
            }
            let class = if index + 1 == segments.len() {
                expected
            } else {
                match (role, index) {
                    (StandardRole::OccurrenceSnapshots | StandardRole::OccurrenceStartShot, 1) => {
                        kc::CLASS
                    }
                    (StandardRole::ThingsThat, 1) => kc::FEATURE,
                    (StandardRole::FeatureChainSourceTarget, 1) => kc::FUNCTION,
                    (StandardRole::FeatureChainSourceTarget, 2) => kc::FEATURE,
                    _ => kc::LIBRARY_PACKAGE,
                }
            };
            f.create(next, class, segments[index]);
            let membership = if f
                .base
                .model()
                .registry()
                .is_subtype(class, kc::FEATURE)
                .unwrap()
                && index > 1
            {
                kc::FEATURE_MEMBERSHIP
            } else {
                kc::OWNING_MEMBERSHIP
            };
            f.member(owner, next, next + 100_000, membership);
            if f.base
                .model()
                .registry()
                .is_subtype(class, kc::FUNCTION)
                .unwrap()
                || f.base
                    .model()
                    .registry()
                    .is_subtype(class, kc::EXPRESSION)
                    .unwrap()
            {
                result_owners.push((next, libraries.artifacts[&artifact]));
            }
            paths.insert(key, next);
            owner = next;
            next += 1;
        }
    }
    for (owner, library) in result_owners {
        f.origin = DeclaredOrigin::StandardLibrary { library };
        f.create(next, kc::FEATURE, "result");
        f.member(owner, next, next + 100_000, kc::RETURN_PARAMETER_MEMBERSHIP);
        next += 1;
    }
    // Keep formal target authority independent of the 31 algorithmic bindings.
    // The Actions closure fixture exercises these additional exact paths too.
    for rule in agq_kerml_semantics::FormalConstraintId::ALL {
        let artifact = StandardLibraryArtifact::Semantic;
        f.origin = DeclaredOrigin::StandardLibrary {
            library: libraries.artifacts[&artifact],
        };
        let contract = rule.target();
        let segments = rule.effective_path(agq_kerml::BaselineProfile::OPERATIONAL_V9);
        let classes = [
            kc::LIBRARY_PACKAGE,
            contract.owner_metaclass,
            contract.target_metaclass,
        ];
        let mut owner = roots_by_artifact[&artifact];
        for index in 0..segments.len() {
            let key = (
                artifact,
                segments[..=index]
                    .iter()
                    .map(|segment| (*segment).to_owned())
                    .collect(),
            );
            if let Some(&existing) = paths.get(&key) {
                owner = existing;
                continue;
            }
            f.create(next, classes[index], segments[index]);
            f.member(
                owner,
                next,
                next + 100_000,
                if index == 2 {
                    kc::FEATURE_MEMBERSHIP
                } else {
                    kc::OWNING_MEMBERSHIP
                },
            );
            paths.insert(key, next);
            owner = next;
            next += 1;
        }
    }
    let semantic = StandardLibraryArtifact::Semantic;
    f.origin = DeclaredOrigin::StandardLibrary {
        library: libraries.artifacts[&semantic],
    };
    let mut excluded = BTreeMap::new();
    for (package, name) in [("Links", "SelfLink"), ("Occurrences", "HappensLink")] {
        let owner = paths[&(semantic, vec![package.into()])];
        f.create(next, kc::ASSOCIATION, name);
        f.member(owner, next, next + 100_000, kc::OWNING_MEMBERSHIP);
        excluded.insert(name, next);
        next += 1;
    }
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    f.create(30_000, kc::NAMESPACE, "systemsRoot");
    f.create(30_001, kc::LIBRARY_PACKAGE, "Actions");
    f.create(30_002, sc::ACTION_DEFINITION, "Action");
    f.member(30_000, 30_001, 130_001, kc::OWNING_MEMBERSHIP);
    f.member(30_001, 30_002, 130_002, kc::OWNING_MEMBERSHIP);
    excluded.insert("Action", 30_002);
    f.origin = origin();
    f.create(40_000, sc::PART_DEFINITION, "Owner");
    f.create(40_001, sc::ITEM_USAGE, "nested");
    f.member(40_000, 40_001, 140_001, kc::FEATURE_MEMBERSHIP);
    f.value(40_001, kp::FEATURE_IS_COMPOSITE, Value::Boolean(composite));
    f.value(40_001, kp::FEATURE_IS_PORTION, Value::Boolean(portion));
    if occurrence_owner {
        let occurrence = paths[&(semantic, vec!["Occurrences".into(), "Occurrence".into()])];
        f.relation(
            40_000,
            occurrence,
            240_000,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    if let Some(excluded_type) = excluded_type {
        f.relation(
            40_001,
            excluded[excluded_type],
            240_001,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
    }
    let roots = roots_by_artifact
        .values()
        .map(|n| id(*n))
        .chain([id(30_000)])
        .collect();
    (f.finish(), libraries, roots)
}

#[test]
fn may_time_vary_exact_antecedents_and_exclusions_use_canonical_identities() {
    for (occurrence, composite, portion, excluded, expected) in [
        (true, false, false, None, true),
        (false, false, false, None, false),
        (true, false, true, None, false),
        (true, false, false, Some("SelfLink"), false),
        (true, false, false, Some("HappensLink"), false),
        (true, true, false, Some("Action"), false),
        (true, false, false, Some("Action"), true),
    ] {
        let (snapshot, libraries, roots) =
            may_time_fixture(occurrence, composite, portion, excluded);
        let context = SemanticContext::for_snapshot(
            &snapshot,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                exclude_implied: true,
            },
            BTreeSet::new(),
        )
        .unwrap()
        .with_standard_bindings(&roots, &libraries)
        .unwrap();
        let queries = KerMlQueries::new(context);
        let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32]));
        let answer = current_usage_may_time_vary(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V1,
            &bindings,
            &roots,
            id(40_001),
        );
        assert_eq!(
            answer.completeness,
            Completeness::Complete,
            "{excluded:?}: {:?}",
            answer.diagnostics
        );
        assert_eq!(
            answer.value,
            Some(expected),
            "{occurrence} {composite} {portion} {excluded:?}"
        );
        let scalar = plan_sysml_may_time_vary(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V1,
            &bindings,
            &roots,
            id(40_001),
        );
        let negative_type_premise = !occurrence || expected;
        if negative_type_premise {
            assert_eq!(scalar.evidence.completeness, Completeness::Incomplete);
            assert!(scalar.properties.is_empty());
            assert!(
                scalar
                    .evidence
                    .search_dependencies
                    .iter()
                    .any(|dependency| {
                        matches!(
                            dependency,
                            agq_kerml_semantics::SearchDependency::ProducerClosure { .. }
                        )
                    })
            );
            continue;
        }
        // Positive exclusions and the authored portion flag are sufficient;
        // they do not wait on unrelated absent-type closure evidence.
        assert_eq!(scalar.evidence.completeness, Completeness::Complete);
        assert_eq!(scalar.properties.len(), 1);
        assert_eq!(
            scalar.properties[0].property,
            agq_sysml::properties::USAGE_MAY_TIME_VARY
        );
        assert_eq!(
            scalar.properties[0].value,
            SlotValue::Scalar(Value::Boolean(expected))
        );
    }
}

fn corpus_anchor_fixture() -> (Fixture, BTreeMap<StandardSysmlRole, u128>) {
    let mut f = Fixture::new();
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    f.create(1, kc::NAMESPACE, "root");
    let mut paths: BTreeMap<Vec<String>, u128> = BTreeMap::new();
    let mut roles = BTreeMap::new();
    let mut next = 10u128;
    for role in [
        StandardSysmlRole::Action,
        StandardSysmlRole::Actions,
        StandardSysmlRole::Subactions,
        StandardSysmlRole::AssignmentActions,
        StandardSysmlRole::Assignments,
        StandardSysmlRole::WhileLoopActions,
        StandardSysmlRole::WhileLoops,
        StandardSysmlRole::TransitionActions,
        StandardSysmlRole::TransitionAccepter,
        StandardSysmlRole::AcceptActions,
        StandardSysmlRole::AcceptSubactions,
        StandardSysmlRole::PerformedActions,
        StandardSysmlRole::StateTransitions,
        StandardSysmlRole::DecisionTransitions,
        StandardSysmlRole::StateAction,
        StandardSysmlRole::StateActions,
        StandardSysmlRole::Substates,
        StandardSysmlRole::ExclusiveStates,
        StandardSysmlRole::OwnedStates,
        StandardSysmlRole::BinaryInterface,
        StandardSysmlRole::BinaryInterfaces,
        StandardSysmlRole::BinaryConnection,
        StandardSysmlRole::ViewpointCheck,
        StandardSysmlRole::ViewpointChecks,
        StandardSysmlRole::Interfaces,
        StandardSysmlRole::Interface,
        StandardSysmlRole::Messages,
        StandardSysmlRole::Flows,
    ] {
        let (segments, expected) = role.specification();
        let mut owner = 1;
        for index in 0..segments.len() {
            let key: Vec<_> = segments[..=index].iter().map(|s| (*s).to_owned()).collect();
            if let Some(&id) = paths.get(&key) {
                owner = id;
                continue;
            }
            let class = if index + 1 == segments.len() {
                expected
            } else {
                role.prefix_class(index)
            };
            f.create(next, class, segments[index]);
            f.member(
                owner,
                next,
                next + 10_000,
                if index > 1 {
                    kc::FEATURE_MEMBERSHIP
                } else {
                    kc::OWNING_MEMBERSHIP
                },
            );
            paths.insert(key, next);
            owner = next;
            next += 1;
        }
        roles.insert(role, owner);
    }
    f.origin = origin();
    (f, roles)
}

#[test]
fn operational_v2_authority_matrix_corrects_only_the_three_pinned_targets() {
    let (mut f, roles) = corpus_anchor_fixture();
    for (subject, class) in [
        (3000, sc::VIEWPOINT_DEFINITION),
        (3001, sc::VIEWPOINT_USAGE),
        (3002, sc::CONNECTION_DEFINITION),
    ] {
        // These deliberately arbitrary authored names prove selection is by
        // structural rule and canonical target, never a source-library name.
        f.create(subject, class, "arbitraryAuthoredSubject");
    }
    for end in [3003, 3004] {
        f.create(end, sc::REFERENCE_USAGE, "arbitraryEnd");
        f.member(3002, end, end + 10_000, kc::FEATURE_MEMBERSHIP);
        f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
    }
    let snapshot = f.finish();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = KerMlQueries::new(context.kerml);
    for profile in [
        SysmlBaselineProfile::PUBLISHED,
        SysmlBaselineProfile::OPERATIONAL_V1,
    ] {
        for (subject, rule) in [
            (3000, "checkViewpointDefinitionSpecialization"),
            (3001, "checkViewpointUsageSpecialization"),
            (3002, "checkConnectionDefinitionBinarySpecialization"),
        ] {
            let plan =
                plan_sysml_producers(&queries, profile, &context.bindings, &[id(1)], id(subject));
            let outcome = result(&plan, rule);
            assert!(outcome.relationships.is_empty());
            assert_eq!(outcome.evidence.completeness, Completeness::Incomplete);
            assert!(
                outcome
                    .evidence
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "SQ_TARGET_MISSING")
            );
        }
    }
    for (subject, rule, target) in [
        (
            3000,
            "checkViewpointDefinitionSpecialization",
            StandardSysmlRole::ViewpointCheck,
        ),
        (
            3001,
            "checkViewpointUsageSpecialization",
            StandardSysmlRole::ViewpointChecks,
        ),
        (
            3002,
            "checkConnectionDefinitionBinarySpecialization",
            StandardSysmlRole::BinaryConnection,
        ),
    ] {
        let plan = plan_sysml_producers(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V2,
            &context.bindings,
            &[id(1)],
            id(subject),
        );
        let outcome = result(&plan, rule);
        assert_eq!(
            outcome.evidence.completeness,
            Completeness::Complete,
            "{rule}"
        );
        assert_eq!(outcome.relationships.len(), 1, "{rule}");
        assert_eq!(
            outcome.relationships[0].general,
            id(roles[&target]),
            "{rule}"
        );
    }
}

fn set_enum(f: &mut Fixture, subject: u128, property: PropertyId, name: &str) {
    let ValueKind::Enumeration(domain) = f
        .base
        .model()
        .registry()
        .property(property)
        .unwrap()
        .value_kind
    else {
        panic!("enum property");
    };
    let literal = *f
        .base
        .model()
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, candidate)| candidate.as_str() == name)
        .unwrap()
        .0;
    f.value(subject, property, Value::Enumeration(literal));
}

#[test]
fn corpus_action_bases_and_subactions_follow_metaclasses_and_state_membership_kind() {
    let (mut f, roles) = corpus_anchor_fixture();
    f.create(3000, sc::ACTION_DEFINITION, "Owner");
    for (subject, class) in [
        (3001, sc::ASSIGNMENT_ACTION_USAGE),
        (3002, sc::WHILE_LOOP_ACTION_USAGE),
    ] {
        f.create(subject, class, "nested");
        f.value(subject, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(3000, subject, subject + 10_000, kc::FEATURE_MEMBERSHIP);
    }
    f.create(3003, sc::STATE_DEFINITION, "State");
    for (subject, kind) in [(3004, "entry"), (3005, "do"), (3006, "exit")] {
        f.create(subject, sc::PERFORM_ACTION_USAGE, kind);
        f.value(subject, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(
            3003,
            subject,
            subject + 10_000,
            sc::STATE_SUBACTION_MEMBERSHIP,
        );
        set_enum(
            &mut f,
            subject + 10_000,
            agq_sysml::properties::STATE_SUBACTION_MEMBERSHIP_KIND,
            kind,
        );
    }
    let snapshot = f.finish();
    let queries = SysmlQueries::new(crate::context::fixture_context(&snapshot, BTreeSet::new()));
    for (subject, base, base_rule, subset, subset_rule) in [
        (
            3001,
            StandardSysmlRole::AssignmentActions,
            "checkAssignmentActionUsageSpecialization",
            StandardSysmlRole::Assignments,
            "checkAssignmentActionUsageSubactionSpecialization",
        ),
        (
            3002,
            StandardSysmlRole::WhileLoopActions,
            "checkWhileLoopActionUsageSpecialization",
            StandardSysmlRole::WhileLoops,
            "checkWhileLoopActionUsageSubactionSpecialization",
        ),
    ] {
        let plan = queries.producer_plan(&[id(1)], id(subject));
        assert_eq!(
            result(&plan, base_rule).relationships[0].general,
            id(roles[&base])
        );
        assert_eq!(
            result(&plan, subset_rule).relationships[0].general,
            id(roles[&subset])
        );
    }
    for subject in 3004..=3006 {
        let plan = queries.producer_plan(&[id(1)], id(subject));
        let action = result(&plan, "checkActionUsageSubactionSpecialization");
        assert_eq!(action.evidence.completeness, Completeness::Complete);
        assert_eq!(
            !action.relationships.is_empty(),
            subject == 3005,
            "only do is a subaction"
        );
    }
}

#[test]
fn transition_acceptance_and_source_specialization_use_structural_memberships() {
    for trigger in [false, true] {
        let (mut f, roles) = corpus_anchor_fixture();
        f.create(3000, sc::STATE_DEFINITION, "Owner");
        f.create(3001, sc::TRANSITION_USAGE, "transition");
        f.value(3001, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(3000, 3001, 13_001, kc::FEATURE_MEMBERSHIP);
        f.create(3002, sc::STATE_USAGE, "start");
        f.member(3000, 3002, 13_002, kc::FEATURE_MEMBERSHIP);
        f.create(14_001, kc::MEMBERSHIP, "source");
        f.owned
            .entry(id(3001))
            .or_default()
            .push(Value::Reference(id(14_001)));
        f.value(
            14_001,
            kp::MEMBERSHIP_MEMBER_ELEMENT,
            Value::Reference(id(3002)),
        );
        f.create(3003, sc::ACCEPT_ACTION_USAGE, "accept");
        f.member(3001, 3003, 13_003, sc::TRANSITION_FEATURE_MEMBERSHIP);
        set_enum(
            &mut f,
            13_003,
            agq_sysml::properties::TRANSITION_FEATURE_MEMBERSHIP_KIND,
            if trigger { "trigger" } else { "effect" },
        );
        let snapshot = f.finish();
        let queries =
            SysmlQueries::new(crate::context::fixture_context(&snapshot, BTreeSet::new()));
        let accept = queries.producer_plan(&[id(1)], id(3003));
        let (rule, target) = if trigger {
            (
                "checkAcceptActionUsageTriggerActionSpecialization",
                StandardSysmlRole::TransitionAccepter,
            )
        } else {
            (
                "checkAcceptActionUsageSpecialization",
                StandardSysmlRole::AcceptActions,
            )
        };
        // `TransitionAction::accepter` is reached through the semantic
        // specialization of the nested trigger AcceptActionUsage.  This is a
        // canonical relationship, rather than a source-path alias: inherited
        // feature lookup subsequently retains the library accepter identity.
        let accept_specialization = result(&accept, rule);
        assert_eq!(
            accept_specialization.evidence.completeness,
            Completeness::Complete
        );
        assert_eq!(accept_specialization.relationships.len(), 1);
        assert_eq!(accept_specialization.relationships[0].specific, id(3003));
        assert_eq!(
            accept_specialization.relationships[0].general,
            id(roles[&target])
        );
        let inactive_rule = if trigger {
            "checkAcceptActionUsageSpecialization"
        } else {
            "checkAcceptActionUsageTriggerActionSpecialization"
        };
        let inactive = result(&accept, inactive_rule);
        assert_eq!(inactive.evidence.completeness, Completeness::Complete);
        assert!(inactive.relationships.is_empty());
        assert!(!inactive.evidence.search_dependencies.is_empty());
        let transition = queries.producer_plan(&[id(1)], id(3001));
        assert_eq!(
            result(&transition, "checkTransitionUsageSpecialization").relationships[0].general,
            id(roles[&StandardSysmlRole::TransitionActions])
        );
        assert_eq!(
            result(&transition, "checkTransitionUsageStateSpecialization").relationships[0].general,
            id(roles[&StandardSysmlRole::StateTransitions])
        );
        assert!(
            result(&transition, "checkTransitionUsageActionSpecialization")
                .relationships
                .is_empty()
        );
    }
}

#[test]
fn actions_micro_closes_generic_owner_negation_and_preserves_payload_identities() {
    use agq_kerml_semantics::{
        FormalConstraintId, MemberAccess, PublicationOverlayError, SemanticClosureRequirement,
        close_result_structure_with_extension,
    };
    // Synthetic anchors isolate the scheduler/query contract from the corpus.
    // They are immutable dependencies in this fixture, never a production receipt.
    let (mut anchors, roles) = corpus_anchor_fixture();
    let (kernel, libraries, mut roots) = may_time_fixture(false, false, false, None);
    roots.retain(|root| *root != id(30_000));
    roots.push(id(1));
    for record in kernel.model().elements().filter(|record| {
        matches!(record.origin(), agq_kernel::provenance::Origin::Declared(DeclaredOrigin::StandardLibrary { library }) if libraries.artifacts.values().any(|candidate| candidate == library))
    }) {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else { unreachable!() };
        anchors.changes.create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            anchors.changes.set(record.id(), property, slot.value().clone(), origin.clone());
        }
    }
    anchors.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    let accepter = roles[&StandardSysmlRole::TransitionAccepter];
    anchors.create(45_001, sc::REFERENCE_USAGE, "acceptedMessage");
    set_enum(&mut anchors, 45_001, kp::FEATURE_DIRECTION, "in");
    anchors.member(accepter, 45_001, 145_001, kc::PARAMETER_MEMBERSHIP);
    for (source, relation) in [
        (roles[&StandardSysmlRole::Actions], 245_001),
        (accepter, 245_002),
    ] {
        anchors.relation(
            source,
            roles[&StandardSysmlRole::Action],
            relation,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
    }
    let anchors = anchors.finish();
    let mut names = anchors.change_set();
    for membership in anchors.model().instances(kc::MEMBERSHIP, true).unwrap() {
        names.clear(membership.id(), kp::ELEMENT_DECLARED_NAME);
    }
    let dependency = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(anchors.apply(&names).unwrap())
            .build()
            .unwrap(),
    );
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(50_000, sc::TRANSITION_USAGE, "transition");
    for parameter in [50_001, 50_002] {
        f.create(parameter, sc::REFERENCE_USAGE, "");
        f.changes.clear(id(parameter), kp::ELEMENT_DECLARED_NAME);
        set_enum(&mut f, parameter, kp::FEATURE_DIRECTION, "in");
        f.member(
            50_000,
            parameter,
            parameter + 100_000,
            kc::PARAMETER_MEMBERSHIP,
        );
        f.changes
            .clear(id(parameter + 100_000), kp::ELEMENT_DECLARED_NAME);
    }
    f.create(50_003, sc::ACCEPT_ACTION_USAGE, "accepter");
    f.value(50_003, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(50_000, 50_003, 150_003, sc::TRANSITION_FEATURE_MEMBERSHIP);
    f.changes.clear(id(150_003), kp::ELEMENT_DECLARED_NAME);
    set_enum(
        &mut f,
        150_003,
        agq_sysml::properties::TRANSITION_FEATURE_MEMBERSHIP_KIND,
        "trigger",
    );
    f.create(50_004, kc::STEP, "nestedAction");
    f.value(50_004, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(50_003, 50_004, 150_004, kc::FEATURE_MEMBERSHIP);
    f.changes.clear(id(150_004), kp::ELEMENT_DECLARED_NAME);
    let snapshot = f.finish();
    let options = SemanticOptions {
        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
        ..Default::default()
    };
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32]));
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        bindings,
        roots.clone(),
    );
    fn make_context<'m>(
        overlay: &'m agq_kernel::derived::DerivedOverlay,
        options: &SemanticOptions,
        roots: &[ElementId],
        libraries: &LibrarySetIdentity,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        SemanticContext::for_overlay(overlay, options.clone(), BTreeSet::new())
            .unwrap()
            .with_standard_bindings(roots, libraries)
            .unwrap()
            .with_formal_constraint_targets(
                roots,
                libraries.artifacts[&StandardLibraryArtifact::Semantic],
            )
            .with_naming_extension(
                SYSML_SEMANTIC_CONTEXT_DOMAIN,
                [47; 32],
                Arc::new(SysmlNamingExtension),
            )
            .map_err(PublicationOverlayError::Context)
    }
    let initial = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let initial_queries =
        KerMlQueries::new(make_context(&initial, &options, &roots, &libraries).unwrap());
    assert_eq!(
        initial_queries
            .formal_constraint_applies(
                FormalConstraintId::StepOwnedPerformanceSpecialization,
                id(50_004)
            )
            .completeness,
        Completeness::Incomplete
    );
    let closure = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| make_context(overlay, &options, &roots, &libraries),
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closure.converged, "{:?}", closure.stages);
    assert_eq!(
        closure.completeness,
        Completeness::Complete,
        "{:?}",
        closure.stages.last()
    );
    let certificate = closure
        .certificate
        .as_ref()
        .expect("scheduler-issued closure")
        .clone();
    let context = make_context(&closure.overlay, &options, &roots, &libraries)
        .unwrap()
        .with_producer_registry_digest(certificate.registry_digest())
        .unwrap()
        .with_producer_closure(certificate.clone())
        .unwrap();
    let queries = KerMlQueries::new(context);
    let final_plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V2,
        &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        &roots,
        id(50_003),
    );
    assert_eq!(final_plan.completeness(), Completeness::Complete);
    assert!(certificate.is_closed(id(50_003), SemanticClosureRequirement::EffectiveTyping));
    let negative = queries.formal_constraint_applies(
        FormalConstraintId::StepOwnedPerformanceSpecialization,
        id(50_004),
    );
    assert_eq!(
        negative.completeness,
        Completeness::Complete,
        "{negative:?}"
    );
    assert!(!negative.value);
    assert!(
        negative
            .search_dependencies
            .iter()
            .any(|dependency| matches!(dependency, SearchDependency::ProducerClosure { .. }))
    );
    for (owner, name, expected) in [
        (50_000, "accepter", 50_003),
        (50_000, "acceptedMessage", 50_002),
        (50_003, "acceptedMessage", 45_001),
    ] {
        let result = queries.lookup_member(id(owner), name, MemberAccess::All);
        assert_eq!(
            result.completeness,
            Completeness::Complete,
            "{name}: {result:?}"
        );
        assert_eq!(
            result
                .value
                .iter()
                .map(|member| member.element)
                .collect::<Vec<_>>(),
            [id(expected)],
            "{name}"
        );
    }
    assert!(Arc::ptr_eq(
        closure.overlay.declared().immutable_dependency().unwrap(),
        &dependency
    ));
}

#[test]
fn parallel_and_exclusive_state_antecedents_choose_distinct_canonical_anchors() {
    for parallel in [false, true] {
        let (mut f, roles) = corpus_anchor_fixture();
        f.create(3000, sc::STATE_DEFINITION, "Owner");
        f.value(
            3000,
            agq_sysml::properties::STATE_DEFINITION_IS_PARALLEL,
            Value::Boolean(parallel),
        );
        f.create(3001, sc::STATE_USAGE, "nested");
        f.value(3001, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(3000, 3001, 13_001, kc::FEATURE_MEMBERSHIP);
        let snapshot = f.finish();
        let queries =
            SysmlQueries::new(crate::context::fixture_context(&snapshot, BTreeSet::new()));
        let plan = queries.producer_plan(&[id(1)], id(3001));
        let (rule, target, other) = if parallel {
            (
                "checkStateUsageSubstateSpecialization",
                StandardSysmlRole::Substates,
                "checkStateUsageExclusiveStateSpecialization",
            )
        } else {
            (
                "checkStateUsageExclusiveStateSpecialization",
                StandardSysmlRole::ExclusiveStates,
                "checkStateUsageSubstateSpecialization",
            )
        };
        assert_eq!(
            result(&plan, rule).relationships[0].general,
            id(roles[&target])
        );
        assert!(result(&plan, other).relationships.is_empty());
    }
}

#[test]
fn interface_and_flow_end_rules_use_only_owned_end_features() {
    let (mut f, roles) = corpus_anchor_fixture();
    for (subject, class) in [
        (3000, sc::INTERFACE_DEFINITION),
        (3001, sc::INTERFACE_USAGE),
        (3002, sc::FLOW_USAGE),
        (3003, sc::FLOW_USAGE),
    ] {
        f.create(subject, class, "connector");
        if subject == 3003 {
            continue;
        }
        for offset in [100, 200] {
            let end = subject + offset;
            f.create(end, sc::REFERENCE_USAGE, "end");
            f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
            f.member(subject, end, end + 10_000, kc::END_FEATURE_MEMBERSHIP);
        }
    }
    let snapshot = f.finish();
    let queries = SysmlQueries::new(crate::context::fixture_context(&snapshot, BTreeSet::new()));
    for (subject, rule, role) in [
        (
            3000,
            "checkInterfaceDefinitionBinarySpecialization",
            StandardSysmlRole::BinaryInterface,
        ),
        (
            3001,
            "checkInterfaceUsageBinarySpecialization",
            StandardSysmlRole::BinaryInterfaces,
        ),
        (
            3002,
            "checkFlowUsageFlowSpecialization",
            StandardSysmlRole::Flows,
        ),
    ] {
        let plan = queries.producer_plan(&[id(1)], id(subject));
        assert_eq!(
            result(&plan, rule).relationships[0].general,
            id(roles[&role])
        );
    }
    let message = queries.producer_plan(&[id(1)], id(3003));
    assert!(
        result(&message, "checkFlowUsageFlowSpecialization")
            .relationships
            .is_empty()
    );
}
