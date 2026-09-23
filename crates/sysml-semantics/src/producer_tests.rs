use super::*;
#[path = "producer_end_usage_tests.rs"]
mod end_usage_tests;
#[path = "producer_expression_usage_tests.rs"]
mod expression_usage_tests;
#[path = "producer_trigger_tests.rs"]
mod trigger_tests;
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
    // This positive obligation may retain its ordinary edge when removing it
    // would require an unproved reverse-path absence to choose between bases.
    assert_eq!(
        result(&plan, "checkItemDefinitionSpecialization").relationships[0].general,
        id(3)
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
fn suppressed_base_retains_the_more_specific_proposals_owner_antecedent() {
    use agq_kernel::provenance::FactKey;
    let mut f = item_fixture(true, false);
    f.relation(6, 9, 206, kc::SUBSETTING, kp::SUBSETTING_SUBSETTED_FEATURE);
    f.create(14, kc::CLASSIFIER, "nonItemOwner");
    let snapshot = f.finish();
    let queries = SysmlQueries::new(crate::context::fixture_context(&snapshot, BTreeSet::new()));
    let plan = queries.producer_plan(&[id(1)], id(12));
    let broad = result(&plan, "checkPartUsageSpecialization");
    let narrow = result(&plan, "checkPartUsageSubpartSpecialization");
    assert!(broad.relationships.is_empty());
    assert_eq!(narrow.relationships[0].general, id(6));
    assert!(
        broad
            .evidence
            .positive_dependencies
            .contains(&FactKey::Element(id(11)))
    );
    assert!(broad.evidence.canonical_dependencies.contains(
        &agq_kernel::provenance::Dependency::Declared(FactKey::Element(id(11)))
    ));

    let mut changes = snapshot.change_set();
    changes.set(
        id(11),
        kp::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![]),
        origin(),
    );
    changes.set(
        id(14),
        kp::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(112))]),
        origin(),
    );
    let edited = snapshot.apply(&changes).unwrap();
    let queries = SysmlQueries::new(crate::context::fixture_context(&edited, BTreeSet::new()));
    let plan = queries.producer_plan(&[id(1)], id(12));
    assert!(
        result(&plan, "checkPartUsageSubpartSpecialization")
            .relationships
            .is_empty()
    );
    assert_eq!(
        result(&plan, "checkPartUsageSpecialization").relationships[0].general,
        id(9)
    );
}

#[test]
fn proposal_suppression_uses_the_retained_smallest_witness_across_a_chain() {
    use agq_kernel::provenance::{Dependency, FactKey};
    let mut f = item_fixture(true, false);
    f.relation(5, 4, 205, kc::SUBSETTING, kp::SUBSETTING_SUBSETTED_FEATURE);
    f.relation(4, 9, 204, kc::SUBSETTING, kp::SUBSETTING_SUBSETTED_FEATURE);
    let source = f.finish();
    // Planning order is B, C, A while canonical target order is A < B < C.
    // C must use retained A's antecedents, not the already suppressed B proposal.
    let rename = |element| {
        if element == id(4) {
            id(20)
        } else if element == id(9) {
            id(30)
        } else {
            element
        }
    };
    let base = Fixture::new().base;
    let mut changes = base.change_set();
    for record in source.model().elements() {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        changes.create(rename(record.id()), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            let value = |value: &Value| match value {
                Value::Reference(element) => Value::Reference(rename(*element)),
                other => other.clone(),
            };
            let slot = match slot.value() {
                SlotValue::Scalar(v) => SlotValue::Scalar(value(v)),
                SlotValue::Ordered(v) => SlotValue::Ordered(v.iter().map(value).collect()),
                SlotValue::Set(v) => SlotValue::Set(v.iter().map(value).collect()),
                SlotValue::Bag(v) => SlotValue::Bag(v.iter().map(value).collect()),
            };
            changes.set(rename(record.id()), property, slot, origin.clone());
        }
    }
    let snapshot = base.apply(&changes).unwrap();
    let context = crate::context::fixture_context(&snapshot, BTreeSet::new());
    let queries = KerMlQueries::new(context.kerml);
    let plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V2,
        &context.bindings,
        &[id(1)],
        id(12),
    );
    assert_eq!(
        result(&plan, "checkItemUsageSubitemSpecialization").relationships[0].general,
        id(5)
    );
    for rule in [
        "checkItemUsageSpecialization",
        "checkPartUsageSpecialization",
    ] {
        let broad = result(&plan, rule);
        assert!(broad.relationships.is_empty());
        assert!(
            broad
                .evidence
                .canonical_dependencies
                .contains(&Dependency::Declared(FactKey::Element(id(11))))
        );
        assert!(
            broad
                .evidence
                .positive_dependencies
                .contains(&FactKey::Element(id(205)))
        );
    }
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
fn missing_target_retains_pending_reads_without_suppressing_proven_positive_bases() {
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
            .contains(&SearchDependency::Kernel(
                agq_kernel::derived::StructuralSearch::DeclaredProperty {
                    element: id(3),
                    property: kp::ELEMENT_OWNED_RELATIONSHIP,
                }
            ))
    );
    assert!(item.relationships.is_empty());
    assert_eq!(item.evidence.completeness, Completeness::Incomplete);
    // The pending authored specialization can change ancestry exhaustiveness,
    // but cannot change the existing PartUsage metaclass or its standard base.
    let positive = result(&plan, "checkPartUsageSpecialization");
    assert_eq!(positive.evidence.completeness, Completeness::Complete);
    assert_eq!(positive.relationships.len(), 1);
    assert_eq!(positive.relationships[0].general, id(9));
    assert_eq!(
        queries.all_supertypes(id(12)).completeness,
        Completeness::Incomplete
    );
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
fn may_time_vary_positive_owner_uses_only_selected_occurrence_path() {
    use agq_kernel::{
        derived::{DerivationBuilder, StructuralSearch},
        provenance::{Dependency, FactKey},
    };
    let (snapshot, libraries, roots) = may_time_fixture(true, false, false, None);
    let mut f = Fixture {
        changes: snapshot.change_set(),
        base: snapshot,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(40_002, kc::CLASSIFIER, "UnrelatedAncestor");
    let snapshot = f.finish();
    let slots = snapshot
        .model()
        .element(id(240_000))
        .unwrap()
        .slots()
        .map(|(property, slot)| {
            (
                property,
                if property == kp::SUBCLASSIFICATION_SUPERCLASSIFIER {
                    SlotValue::Scalar(Value::Reference(id(40_002)))
                } else {
                    slot.value().clone()
                },
            )
        })
        .collect::<Vec<_>>();
    let key = DerivationKey {
        rule: RuleId::from_u128(98_223),
        subject: id(40_000),
        output: OutputKey::from_u128(1),
    };
    let mut builder = DerivationBuilder::new(snapshot);
    builder.element(
        key,
        kc::SUBCLASSIFICATION,
        slots,
        BTreeSet::from([Dependency::Declared(FactKey::Element(id(40_001)))]),
    );
    builder.searches(
        FactKey::Element(key.element_id()),
        BTreeSet::from([StructuralSearch::ProducerClosure {
            subject: id(40_001),
            requirement: agq_kerml_semantics::SemanticClosureRequirement::EffectiveTyping
                .contract_id()
                .into(),
        }]),
    );
    let overlay = builder.build().unwrap();
    let context = SemanticContext::for_overlay(
        &overlay,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_standard_bindings(&roots, &libraries)
    .unwrap();
    {
        let queries = KerMlQueries::new(context);
        let all = queries.all_supertypes(id(40_000));
        assert!(
            all.canonical_dependencies
                .contains(&Dependency::Derived(FactKey::Element(key.element_id())))
        );
        let answer = current_usage_may_time_vary(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V2,
            &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
            &roots,
            id(40_001),
        );
        assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
        assert_eq!(answer.value, Some(true));
        assert!(answer.positive_dependencies.contains(&FactKey::Property {
            element: id(240_000),
            property: kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        }));
        assert!(
            !answer
                .canonical_dependencies
                .contains(&Dependency::Derived(FactKey::Element(key.element_id()))),
            "unrelated ancestor imported its child-typing requirement: {answer:?}"
        );
        assert!(
            !answer
                .search_dependencies
                .contains(&SearchDependency::Kernel(
                    StructuralSearch::ProducerClosure {
                        subject: id(40_001),
                        requirement:
                            agq_kerml_semantics::SemanticClosureRequirement::EffectiveTyping
                                .contract_id()
                                .into(),
                    }
                ))
        );
        let final_plan = plan_sysml_may_time_vary(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V2,
            &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
            &roots,
            id(40_001),
        );
        assert_eq!(final_plan.evidence.completeness, Completeness::Incomplete);
        assert!(final_plan.properties.is_empty());
        assert!(final_plan.evidence.search_dependencies.iter().any(|search| matches!(search,
            SearchDependency::ProducerClosure { subject, requirement: agq_kerml_semantics::SemanticClosureRequirement::EffectiveTyping, .. }
                if *subject == id(40_001)
        )), "the child's negative excluded-type premise still requires closure");
    }
}

#[test]
fn may_time_vary_positive_exclusion_uses_only_selected_type_path() {
    use agq_kernel::{
        derived::{DerivationBuilder, StructuralSearch},
        provenance::{Dependency, FactKey},
    };
    for excluded in ["Action", "SelfLink", "HappensLink"] {
        let (snapshot, libraries, roots) =
            may_time_fixture(true, excluded == "Action", false, Some(excluded));
        let mut f = Fixture {
            changes: snapshot.change_set(),
            base: snapshot,
            owned: BTreeMap::new(),
            origin: origin(),
        };
        f.create(40_002, kc::CLASSIFIER, "UnrelatedSubjectAncestor");
        let snapshot = f.finish();
        let slots = snapshot
            .model()
            .element(id(240_001))
            .unwrap()
            .slots()
            .map(|(property, slot)| {
                (
                    property,
                    if property == kp::FEATURE_TYPING_TYPE {
                        SlotValue::Scalar(Value::Reference(id(40_002)))
                    } else {
                        slot.value().clone()
                    },
                )
            })
            .collect::<Vec<_>>();
        let key = DerivationKey {
            rule: RuleId::from_u128(98_224),
            subject: id(40_001),
            output: OutputKey::from_u128(1),
        };
        let self_read = StructuralSearch::Property {
            element: id(40_001),
            property: agq_sysml::properties::USAGE_MAY_TIME_VARY,
        };
        let mut builder = DerivationBuilder::new(snapshot);
        builder.element(key, kc::FEATURE_TYPING, slots, BTreeSet::new());
        builder.searches(
            FactKey::Element(key.element_id()),
            BTreeSet::from([self_read.clone()]),
        );
        let overlay = builder.build().unwrap();
        {
            let context = SemanticContext::for_overlay(
                &overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap()
            .with_standard_bindings(&roots, &libraries)
            .unwrap();
            let queries = KerMlQueries::new(context);
            let all = queries.all_supertypes(id(40_001));
            assert!(
                all.search_dependencies
                    .contains(&SearchDependency::Kernel(self_read.clone()))
            );
            let plan = plan_sysml_may_time_vary(
                &queries,
                SysmlBaselineProfile::OPERATIONAL_V2,
                &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
                &roots,
                id(40_001),
            );
            assert_eq!(
                plan.evidence.completeness,
                Completeness::Complete,
                "{excluded}: {:?}",
                plan.evidence
            );
            assert_eq!(plan.properties.len(), 1);
            assert_eq!(
                plan.properties[0].value,
                SlotValue::Scalar(Value::Boolean(false))
            );
            assert!(
                plan.evidence
                    .canonical_dependencies
                    .contains(&Dependency::Declared(FactKey::Property {
                        element: id(240_001),
                        property: kp::FEATURE_TYPING_TYPE
                    })),
                "the actual positive exclusion edge remains required"
            );
            assert!(
                !plan
                    .evidence
                    .canonical_dependencies
                    .contains(&Dependency::Derived(FactKey::Element(key.element_id()))),
                "{excluded}: an unrelated ancestor is not a premise of a positive exclusion"
            );
            assert!(
                !plan
                    .evidence
                    .search_dependencies
                    .contains(&SearchDependency::Kernel(self_read.clone())),
                "{excluded}: positive exclusion must not acquire its own mayTimeVary prerequisite"
            );
        }
    }
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
    corpus_anchor_fixture_complete(false)
}
fn corpus_anchor_fixture_complete(all: bool) -> (Fixture, BTreeMap<StandardSysmlRole, u128>) {
    let mut f = Fixture::new();
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    f.create(1, kc::NAMESPACE, "root");
    let mut paths: BTreeMap<Vec<String>, u128> = BTreeMap::new();
    let mut roles = BTreeMap::new();
    let mut next = 10u128;
    let selected = [
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
    ];
    for &role in if all {
        &StandardSysmlRole::ALL[..]
    } else {
        &selected[..]
    } {
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
    actions_micro(ActionsMicro::Basic);
}

#[test]
fn producer_closure_does_not_erase_unsupported_variation_semantics() {
    actions_micro(ActionsMicro::Variation);
}

#[test]
fn certified_usage_scalar_activates_shared_snapshot_and_value_context_producers() {
    actions_micro(ActionsMicro::VariableValue);
}

#[test]
fn root_usage_absent_owner_closes_with_combined_producers() {
    actions_micro(ActionsMicro::RootUsage);
}

#[test]
fn actions_nested_state_transition_closes_owner_typing_and_payload_structure() {
    actions_micro(ActionsMicro::NestedState);
}

#[test]
fn directed_usage_value_closes_without_adopting_an_existing_contextual_feature() {
    actions_micro(ActionsMicro::DirectedValue);
}

#[test]
fn unnamed_constraint_and_connection_usages_close_under_occurrence_owner() {
    actions_micro(ActionsMicro::UnnamedOccurrenceUsages);
}

#[test]
fn unnamed_assertion_body_closes_with_inherited_result_and_expression_binding() {
    actions_micro(ActionsMicro::AssertionBody);
}

#[test]
fn input_action_body_closes_under_while_loop_with_nested_actions() {
    actions_micro(ActionsMicro::WhileLoopBody);
}

#[derive(Clone, Copy)]
enum ActionsMicro {
    Basic,
    Variation,
    VariableValue,
    RootUsage,
    NestedState,
    DirectedValue,
    UnnamedOccurrenceUsages,
    AssertionBody,
    WhileLoopBody,
}

fn actions_micro(variant: ActionsMicro) {
    let with_variable_value = matches!(
        variant,
        ActionsMicro::VariableValue | ActionsMicro::NestedState | ActionsMicro::DirectedValue
    );
    let with_root_usage = matches!(variant, ActionsMicro::RootUsage);
    let nested_state = matches!(variant, ActionsMicro::NestedState);
    let unnamed_occurrence_usages = matches!(
        variant,
        ActionsMicro::UnnamedOccurrenceUsages | ActionsMicro::AssertionBody
    );
    let assertion_body = matches!(variant, ActionsMicro::AssertionBody);
    let while_loop_body = matches!(variant, ActionsMicro::WhileLoopBody);
    use agq_kerml_semantics::{
        FormalConstraintId, MemberAccess, PublicationOverlayError, SemanticClosureRequirement,
        close_result_structure_with_extension,
    };
    // Synthetic anchors isolate the scheduler/query contract from the corpus.
    // They are immutable dependencies in this fixture, never a production receipt.
    let (kernel_dependency, libraries, mut roots) = closed_kernel_anchor_fixture(
        with_variable_value || unnamed_occurrence_usages || while_loop_body,
        while_loop_body,
        assertion_body,
    );
    let occurrence = kernel_dependency
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::Occurrence);
    let (source, roles) = corpus_anchor_fixture_complete(true);
    let owned = source.owned.clone();
    let source = source.finish();
    let base = kernel_dependency.project_snapshot();
    let mut anchors = Fixture {
        changes: base.change_set(),
        base,
        owned,
        origin: origin(),
    };
    for record in source.model().elements() {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        anchors
            .changes
            .create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            anchors
                .changes
                .set(record.id(), property, slot.value().clone(), origin.clone());
        }
    }
    roots.push(id(1));
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
    if with_variable_value || while_loop_body {
        anchors.relation(
            roles[&StandardSysmlRole::Action],
            occurrence.as_u128(),
            245_003,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    if while_loop_body {
        anchors.create(45_010, sc::ACTION_DEFINITION, "WhileLoopActionDefinition");
        anchors.relation(
            45_010,
            roles[&StandardSysmlRole::Action],
            245_010,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
        anchors.relation(
            roles[&StandardSysmlRole::WhileLoopActions],
            45_010,
            245_011,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
        for (parameter, class, name) in [
            (45_011, sc::REFERENCE_USAGE, "whileTest"),
            (45_012, sc::ACTION_USAGE, "body"),
            (45_013, sc::REFERENCE_USAGE, "untilTest"),
        ] {
            anchors.create(parameter, class, name);
            set_enum(&mut anchors, parameter, kp::FEATURE_DIRECTION, "in");
            anchors.member(
                45_010,
                parameter,
                parameter + 100_000,
                kc::PARAMETER_MEMBERSHIP,
            );
        }
    }
    let anchors = anchors.finish();
    let mut names = anchors.change_set();
    for membership in anchors.model().instances(kc::MEMBERSHIP, true).unwrap() {
        if matches!(membership.origin(), agq_kernel::provenance::Origin::Declared(DeclaredOrigin::StandardLibrary { library }) if *library == SystemsLibraryIdentity::LIBRARY)
        {
            names.clear(membership.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    let anchor_snapshot = anchors.apply(&names).unwrap();
    let anchor_options = SemanticOptions {
        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
        ..Default::default()
    };
    let anchor_extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots.clone(),
    );
    let anchor_closure = close_result_structure_with_extension(
        &anchor_snapshot,
        Default::default(),
        |overlay| {
            make_context(
                overlay,
                &anchor_options,
                &roots,
                &libraries,
                Some(&kernel_dependency),
            )
        },
        &anchor_extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(
        anchor_closure.completeness,
        Completeness::Complete,
        "anchor closure {:?}",
        anchor_closure.stages.last()
    );
    let registry = agq_kerml_semantics::ProducerRegistry::new(
        agq_kerml_semantics::ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(anchor_options.baseline_profile))
            .chain(crate::sysml_producer_descriptors()),
    )
    .unwrap();
    let anchor_certificate = anchor_closure.certificate.unwrap();
    let anchor_overlay = Arc::new(anchor_closure.overlay);
    let anchor_context = make_context(
        &anchor_overlay,
        &anchor_options,
        &roots,
        &libraries,
        Some(&kernel_dependency),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap()
    .with_producer_closure(anchor_certificate)
    .unwrap();
    let dependency = agq_kerml_semantics::ProducerClosedDependency::new(
        anchor_overlay.clone(),
        &anchor_context,
        &registry,
    )
    .unwrap();
    drop(anchor_context);
    let dependency = if while_loop_body {
        kernel_dependency
    } else {
        dependency
    };
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    if while_loop_body {
        for record in anchor_snapshot
            .model()
            .elements()
            .filter(|record| f.base.model().element(record.id()).is_none())
        {
            let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
                unreachable!()
            };
            f.changes
                .create(record.id(), record.metaclass(), origin.clone());
            for (property, slot) in record.slots() {
                f.changes
                    .set(record.id(), property, slot.value().clone(), origin.clone());
                if property == kp::ELEMENT_OWNED_RELATIONSHIP {
                    let SlotValue::Ordered(relationships) = slot.value() else {
                        unreachable!()
                    };
                    f.owned.insert(record.id(), relationships.clone());
                }
            }
        }
    }
    f.create(50_000, sc::TRANSITION_USAGE, "transition");
    if nested_state {
        // Actions::AcceptAction owns aState, whose transition owns an accepter.
        // All three local levels must close; none is an immutable anchor.
        f.create(50_010, sc::ACTION_DEFINITION, "AcceptActionDefinition");
        f.create(50_011, sc::STATE_USAGE, "aState");
        f.value(50_011, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(50_010, 50_011, 150_011, kc::FEATURE_MEMBERSHIP);
        f.value(50_000, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(50_011, 50_000, 150_010, kc::FEATURE_MEMBERSHIP);
        for (feature, membership, name) in [(50_012, 150_012, "start"), (50_013, 150_013, "done")] {
            f.create(feature, sc::STATE_USAGE, name);
            f.value(feature, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
            f.member(50_011, feature, membership, kc::FEATURE_MEMBERSHIP);
        }
        f.create(150_014, kc::MEMBERSHIP, "source");
        f.owned
            .entry(id(50_000))
            .or_default()
            .push(Value::Reference(id(150_014)));
        f.value(
            150_014,
            kp::MEMBERSHIP_MEMBER_ELEMENT,
            Value::Reference(id(50_012)),
        );
    }
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
    if matches!(variant, ActionsMicro::Variation) {
        f.value(
            50_003,
            agq_sysml::properties::USAGE_IS_VARIATION,
            Value::Boolean(true),
        );
    }
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
    if with_root_usage {
        f.create(50_008, sc::REFERENCE_USAGE, "rootReference");
    }
    if unnamed_occurrence_usages {
        f.create(50_020, sc::OCCURRENCE_DEFINITION, "Owner");
        f.relation(
            50_020,
            occurrence.as_u128(),
            250_020,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
        for (subject, metaclass) in [
            (50_021, sc::CONSTRAINT_USAGE),
            (50_022, sc::ASSERT_CONSTRAINT_USAGE),
            (50_023, sc::CONNECTION_USAGE),
        ] {
            f.create(subject, metaclass, "");
            f.changes.clear(id(subject), kp::ELEMENT_DECLARED_NAME);
            f.value(subject, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
            f.member(50_020, subject, subject + 100_000, kc::FEATURE_MEMBERSHIP);
            f.changes
                .clear(id(subject + 100_000), kp::ELEMENT_DECLARED_NAME);
        }
    }
    if assertion_body {
        f.create(50_024, kc::LITERAL_BOOLEAN, "");
        f.changes.clear(id(50_024), kp::ELEMENT_DECLARED_NAME);
        f.value(50_024, kp::LITERAL_BOOLEAN_VALUE, Value::Boolean(true));
        f.member(50_022, 50_024, 150_024, kc::RESULT_EXPRESSION_MEMBERSHIP);
        f.create(50_025, kc::FEATURE, "");
        f.changes.clear(id(50_025), kp::ELEMENT_DECLARED_NAME);
        set_enum(&mut f, 50_025, kp::FEATURE_DIRECTION, "out");
        f.member(50_024, 50_025, 150_025, kc::RETURN_PARAMETER_MEMBERSHIP);
        for membership in [150_024, 150_025] {
            f.changes.clear(id(membership), kp::ELEMENT_DECLARED_NAME);
        }
    }
    if while_loop_body {
        f.create(50_030, sc::ACTION_DEFINITION, "LoopOwner");
        f.create(50_031, sc::WHILE_LOOP_ACTION_USAGE, "whileLoop");
        f.value(50_031, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
        f.member(50_030, 50_031, 150_031, kc::FEATURE_MEMBERSHIP);
        f.create(50_036, sc::REFERENCE_USAGE, "whileTest");
        set_enum(&mut f, 50_036, kp::FEATURE_DIRECTION, "in");
        f.member(50_031, 50_036, 150_036, kc::PARAMETER_MEMBERSHIP);
        f.create(50_032, sc::ACTION_USAGE, "");
        f.changes.clear(id(50_032), kp::ELEMENT_DECLARED_NAME);
        set_enum(&mut f, 50_032, kp::FEATURE_DIRECTION, "in");
        f.member(50_031, 50_032, 150_032, kc::PARAMETER_MEMBERSHIP);
        // A while-only node declares its test and body; untilTest is inherited.
        for (subject, class) in [
            (50_033, sc::ASSIGNMENT_ACTION_USAGE),
            (50_034, sc::PERFORM_ACTION_USAGE),
            (50_035, sc::ASSIGNMENT_ACTION_USAGE),
        ] {
            f.create(subject, class, "");
            f.changes.clear(id(subject), kp::ELEMENT_DECLARED_NAME);
            f.value(subject, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
            f.member(50_032, subject, subject + 100_000, kc::FEATURE_MEMBERSHIP);
        }
        for membership in [150_031, 150_032, 150_033, 150_034, 150_035, 150_036] {
            f.changes.clear(id(membership), kp::ELEMENT_DECLARED_NAME);
        }
    }
    if with_variable_value {
        f.create(50_005, sc::REFERENCE_USAGE, "variableValue");
        if matches!(variant, ActionsMicro::DirectedValue) {
            set_enum(&mut f, 50_005, kp::FEATURE_DIRECTION, "in");
        }
        f.member(50_000, 50_005, 150_005, kc::FEATURE_MEMBERSHIP);
        f.changes.clear(id(150_005), kp::ELEMENT_DECLARED_NAME);
        f.create(50_006, kc::EXPRESSION, "valueExpression");
        f.create(50_007, kc::FEATURE, "valueResult");
        set_enum(&mut f, 50_007, kp::FEATURE_DIRECTION, "out");
        f.member(50_006, 50_007, 150_007, kc::RETURN_PARAMETER_MEMBERSHIP);
        f.member(50_005, 50_006, 150_006, kc::FEATURE_VALUE);
        f.value(150_006, kp::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(false));
        f.value(150_006, kp::FEATURE_VALUE_IS_INITIAL, Value::Boolean(false));
    }
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
        dependency: Option<&Arc<agq_kerml_semantics::ProducerClosedDependency>>,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        let kerml = if let Some(dependency) = dependency {
            dependency
                .project_overlay_context(overlay, roots)
                .map_err(PublicationOverlayError::Context)?
        } else {
            SemanticContext::for_overlay(overlay, options.clone(), BTreeSet::new())
                .unwrap()
                .with_standard_bindings(roots, libraries)
                .unwrap()
                .with_formal_constraint_targets(
                    roots,
                    libraries.artifacts[&StandardLibraryArtifact::Semantic],
                )
        };
        crate::context::fixture_overlay_context(
            overlay,
            kerml,
            SysmlBaselineProfile::OPERATIONAL_V2,
        )
        .map(|context| context.kerml)
        .map_err(|error| match error {
            SysmlContextError::KerMl(error) => PublicationOverlayError::Context(error),
            _ => panic!("synthetic context: {error}"),
        })
    }
    let initial = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let initial_queries = KerMlQueries::new(
        make_context(&initial, &options, &roots, &libraries, Some(&dependency)).unwrap(),
    );
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
        |overlay| make_context(overlay, &options, &roots, &libraries, Some(&dependency)),
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
    let context = make_context(
        &closure.overlay,
        &options,
        &roots,
        &libraries,
        Some(&dependency),
    )
    .unwrap()
    .with_producer_registry_digest(certificate.registry_digest())
    .unwrap()
    .with_producer_closure(certificate.clone())
    .unwrap();
    let queries = KerMlQueries::new(context);
    let composed_context = crate::context::fixture_overlay_context(
        &closure.overlay,
        dependency
            .project_overlay_context(&closure.overlay, &roots)
            .unwrap(),
        SysmlBaselineProfile::OPERATIONAL_V2,
    )
    .unwrap()
    .with_producer_closure(certificate.clone())
    .unwrap();
    let composed = SysmlQueries::new(composed_context);
    if assertion_body {
        let result = queries.result_parameters(id(50_022));
        assert_eq!(result.completeness, Completeness::Complete, "{result:?}");
        assert_eq!(result.value.len(), 1);
        assert!(
            dependency
                .overlay()
                .model()
                .element(result.value[0])
                .is_some()
        );
        assert!(closure.overlay.model().elements().any(|record| {
            queries.implied_binding_role(record.id())
                == Some(agq_kerml_semantics::ImpliedBindingRole::ExpressionResult)
        }));
    }
    if unnamed_occurrence_usages {
        for subject in [50_021, 50_022, 50_023] {
            assert!(
                certificate.is_closed(id(subject), SemanticClosureRequirement::EffectiveTyping)
            );
            let scalar = plan_sysml_may_time_vary(
                &queries,
                SysmlBaselineProfile::OPERATIONAL_V2,
                &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
                &roots,
                id(subject),
            );
            assert_eq!(
                scalar.evidence.completeness,
                Completeness::Complete,
                "{scalar:?}"
            );
            assert_eq!(
                queries
                    .model()
                    .navigation_slot(id(subject), agq_sysml::properties::USAGE_MAY_TIME_VARY)
                    .unwrap()
                    .value(),
                &SlotValue::Scalar(Value::Boolean(true)),
            );
        }
    }
    let trigger = composed.transition_features(id(50_000), TransitionFeatureKind::Trigger);
    let expected_completeness = if matches!(variant, ActionsMicro::Variation) {
        Completeness::Incomplete
    } else {
        Completeness::Complete
    };
    assert_eq!(trigger.completeness(), expected_completeness, "{trigger:?}");
    assert_eq!(trigger.value(), &[id(50_003)]);
    let payload = composed.accept_action_payload_parameter(id(50_003));
    assert_eq!(payload.completeness(), expected_completeness, "{payload:?}");
    assert_eq!(payload.value(), &[id(45_001)]);
    let effective = composed.effective_usages(id(50_000));
    if matches!(variant, ActionsMicro::Variation) {
        assert_eq!(
            effective.completeness(),
            Completeness::Incomplete,
            "{effective:?}"
        );
        assert!(
            effective
                .pending
                .contains(&(id(50_003), PendingSysmlRule::Variation))
        );
        assert!(
            !effective
                .pending
                .contains(&(id(50_003), PendingSysmlRule::ProducerClosure))
        );
    } else {
        assert_eq!(
            effective.completeness(),
            Completeness::Complete,
            "{effective:?}"
        );
    }
    assert!(effective.value().contains(&id(50_003)));
    assert!(effective.supporting_queries.iter().any(|proof| {
        proof
            .search_dependencies
            .iter()
            .any(|search| matches!(search, SearchDependency::ProducerClosure { .. }))
    }));
    if nested_state {
        for subject in [50_000, 50_003, 50_010, 50_011, 50_012, 50_013] {
            assert!(
                certificate.is_closed(id(subject), SemanticClosureRequirement::EffectiveTyping)
            );
        }
        assert_eq!(queries.owning_type(id(50_000)).value, Some(id(50_011)));
        assert_eq!(queries.owning_type(id(50_011)).value, Some(id(50_010)));
        let owner = queries.lookup_path(
            id(50_010),
            &agq_kerml_semantics::QualifiedName {
                absolute: false,
                segments: ["aState", "transition", "accepter", "acceptedMessage"]
                    .map(str::to_owned)
                    .to_vec(),
            },
        );
        assert_eq!(owner.completeness, Completeness::Complete, "{owner:?}");
        assert_eq!(
            owner
                .value
                .iter()
                .map(|member| member.element)
                .collect::<Vec<_>>(),
            [id(45_001)]
        );
    }
    if with_root_usage {
        let owner = queries.owning_type(id(50_008));
        assert_eq!(owner.completeness, Completeness::Complete, "{owner:?}");
        assert_eq!(owner.value, None);
        let scalar = plan_sysml_may_time_vary(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V2,
            &StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
            &roots,
            id(50_008),
        );
        assert_eq!(
            scalar.evidence.completeness,
            Completeness::Complete,
            "{scalar:?}"
        );
        assert_eq!(
            queries
                .model()
                .navigation_slot(id(50_008), agq_sysml::properties::USAGE_MAY_TIME_VARY)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(false))
        );
        assert!(certificate.is_closed(id(50_008), SemanticClosureRequirement::EffectiveOwnership));
        assert!(scalar.evidence.search_dependencies.iter().any(|dependency| matches!(dependency,
            SearchDependency::ProducerClosure { subject, requirement: SemanticClosureRequirement::EffectiveOwnership, .. } if *subject == id(50_008)
        )));
    }
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
    if with_variable_value {
        assert_eq!(
            queries
                .model()
                .navigation_slot(id(50_005), agq_sysml::properties::USAGE_MAY_TIME_VARY)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(true))
        );
        let domains = queries.featuring_types(id(50_005));
        assert_eq!(domains.completeness, Completeness::Complete, "{domains:?}");
        assert_eq!(domains.value.len(), 1);
        assert_ne!(
            domains.value[0],
            id(50_000),
            "variable uses a shared snapshot domain"
        );
        assert!(certificate.is_closed(id(50_005), SemanticClosureRequirement::ValueContext));
        let bindings: Vec<_> = queries
            .model()
            .instances(kc::BINDING_CONNECTOR, true)
            .unwrap()
            .filter(|record| {
                queries.implied_binding_role(record.id())
                    == Some(agq_kerml_semantics::ImpliedBindingRole::FeatureValue)
            })
            .map(|record| record.id())
            .collect();
        assert_eq!(bindings.len(), 1);
        let binding_domains = queries.featuring_types(bindings[0]);
        assert_eq!(
            binding_domains.completeness,
            Completeness::Complete,
            "{binding_domains:?}"
        );
        assert_eq!(binding_domains.value, domains.value);
    }
    if while_loop_body {
        assert_eq!(
            queries.structural_parameter_features(id(50_031)).value,
            [id(50_036), id(50_032), id(45_013)],
        );
        assert_eq!(
            queries
                .model()
                .navigation_slot(id(50_032), agq_sysml::properties::USAGE_MAY_TIME_VARY)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(true)),
            "the noncomposite input body varies in its occurrence-typed owner",
        );
        let domains = queries.featuring_types(id(50_032));
        assert_eq!(domains.completeness, Completeness::Complete, "{domains:?}");
        assert_eq!(domains.value.len(), 1, "{domains:?}");
        assert_ne!(
            domains.value[0],
            id(50_031),
            "the body uses a snapshot domain"
        );
        assert!(certificate.is_closed(id(50_032), SemanticClosureRequirement::EffectiveTyping));
    }
    assert!(Arc::ptr_eq(
        closure.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
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

#[test]
fn standard_anchor_path_requires_original_declared_ownership() {
    use agq_kernel::derived::{DerivationBuilder, StructuralSearch};
    use agq_kernel::provenance::FactKey;
    let mut f = item_fixture(false, false);
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    f.create(19, sc::PART_DEFINITION, "Part");
    f.member(7, 19, 119, kc::OWNING_MEMBERSHIP);
    f.owned
        .get_mut(&id(7))
        .unwrap()
        .retain(|value| *value != Value::Reference(id(119)));
    let snapshot = f.finish();
    let mut builder = DerivationBuilder::new(snapshot);
    builder.extend_ordered_references(
        id(7),
        kp::ELEMENT_OWNED_RELATIONSHIP,
        vec![id(119)],
        agq_kernel::provenance::Explanation {
            rule: RuleId::from_u128(98211),
            dependencies: BTreeSet::new(),
        },
    );
    let overlay = builder.build().unwrap();
    let context = crate::context::fixture_overlay_context(
        &overlay,
        SemanticContext::for_overlay(
            &overlay,
            agq_kerml_semantics::SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                exclude_implied: true,
            },
            BTreeSet::new(),
        )
        .unwrap(),
        SysmlBaselineProfile::OPERATIONAL_V2,
    )
    .unwrap();
    let queries = KerMlQueries::new(context.kerml);
    let plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V2,
        &context.bindings,
        &[id(1)],
        id(13),
    );
    let part = result(&plan, "checkPartDefinitionSpecialization");
    assert_eq!(part.evidence.completeness, Completeness::Complete);
    assert_eq!(part.relationships[0].general, id(8));
    assert!(
        part.evidence
            .search_dependencies
            .contains(&SearchDependency::Kernel(
                StructuralSearch::DeclaredProperty {
                    element: id(7),
                    property: kp::ELEMENT_OWNED_RELATIONSHIP,
                }
            ))
    );
    assert!(
        !part
            .evidence
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: id(7) })
    );
    for element in [id(8), id(108)] {
        assert!(
            part.evidence
                .positive_dependencies
                .contains(&FactKey::Element(element))
        );
    }
    // The same carrier becomes an original declaration in a new source graph.
    // It must now participate and expose the duplicate anchor, not be ignored.
    let mut changes = overlay.declared().change_set();
    changes.set(
        id(7),
        kp::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![
            Value::Reference(id(108)),
            Value::Reference(id(109)),
            Value::Reference(id(119)),
        ]),
        DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        },
    );
    let edited = overlay.declared().apply(&changes).unwrap();
    let context = crate::context::fixture_context(&edited, BTreeSet::new());
    let queries = KerMlQueries::new(context.kerml);
    let plan = plan_sysml_producers(
        &queries,
        SysmlBaselineProfile::OPERATIONAL_V2,
        &context.bindings,
        &[id(1)],
        id(13),
    );
    let part = result(&plan, "checkPartDefinitionSpecialization");
    assert_eq!(part.evidence.completeness, Completeness::Invalid);
    assert!(
        part.evidence
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SQ_TARGET_AMBIGUOUS")
    );
}

fn closed_kernel_anchor_fixture(
    with_snapshot_typing: bool,
    with_time_enclosed_occurrences: bool,
    with_boolean_inheritance: bool,
) -> (
    Arc<agq_kerml_semantics::ProducerClosedDependency>,
    LibrarySetIdentity,
    Vec<ElementId>,
) {
    closed_kernel_anchor_fixture_with(
        with_snapshot_typing,
        with_time_enclosed_occurrences,
        with_boolean_inheritance,
        |_, _| {},
    )
}

fn closed_kernel_anchor_fixture_with(
    with_snapshot_typing: bool,
    with_time_enclosed_occurrences: bool,
    with_boolean_inheritance: bool,
    customize: impl FnOnce(&mut Fixture, &agq_kerml_semantics::StandardKermlBindings),
) -> (
    Arc<agq_kerml_semantics::ProducerClosedDependency>,
    LibrarySetIdentity,
    Vec<ElementId>,
) {
    use agq_kerml_semantics::{
        ProducerClosedDependency, ProducerFamily, ProducerRegistry, PublicationOverlayError,
        close_result_structure_with_extension,
    };
    fn kernel_context<'m>(
        overlay: &'m agq_kernel::derived::DerivedOverlay,
        roots: &[ElementId],
        libraries: &LibrarySetIdentity,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        let context = SemanticContext::for_overlay(
            overlay,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .unwrap()
        .with_standard_bindings(roots, libraries)
        .unwrap()
        .with_formal_constraint_targets(
            roots,
            libraries.artifacts[&StandardLibraryArtifact::Semantic],
        );
        Ok(crate::context::fixture_overlay_context(
            overlay,
            context,
            SysmlBaselineProfile::OPERATIONAL_V2,
        )
        .unwrap()
        .kerml)
    }
    let registry = ProducerRegistry::new(
        ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9))
            .chain(sysml_producer_descriptors()),
    )
    .unwrap();
    let (all_kernel, libraries, mut kernel_roots) = may_time_fixture(false, false, false, None);
    kernel_roots.retain(|root| *root != id(30_000));
    let mut kernel = Fixture::new();
    for record in all_kernel.model().elements().filter(|record| {
        matches!(record.origin(),
        agq_kernel::provenance::Origin::Declared(DeclaredOrigin::StandardLibrary { library })
        if libraries.artifacts.values().any(|candidate| candidate == library))
    }) {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        kernel
            .changes
            .create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            kernel
                .changes
                .set(record.id(), property, slot.value().clone(), origin.clone());
            if property == kp::ELEMENT_OWNED_RELATIONSHIP {
                let SlotValue::Ordered(relationships) = slot.value() else {
                    unreachable!()
                };
                kernel.owned.insert(record.id(), relationships.clone());
            }
        }
        if all_kernel
            .model()
            .registry()
            .is_subtype(record.metaclass(), kc::MEMBERSHIP)
            .unwrap()
        {
            kernel.changes.clear(record.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    if with_snapshot_typing {
        let context = SemanticContext::for_snapshot(
            &all_kernel,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                exclude_implied: true,
            },
            BTreeSet::new(),
        )
        .unwrap()
        .with_standard_bindings(&kernel_roots, &libraries)
        .unwrap();
        let bindings = context.id().standard_bindings.as_ref().unwrap();
        if with_boolean_inheritance {
            // The real Performances anchors form this chain. Independent
            // placeholder results otherwise make assertion arity ambiguous.
            for (index, specific, general) in [
                (
                    0,
                    StandardRole::BooleanEvaluations,
                    StandardRole::Evaluations,
                ),
                (
                    1,
                    StandardRole::TrueEvaluations,
                    StandardRole::BooleanEvaluations,
                ),
                (
                    2,
                    StandardRole::FalseEvaluations,
                    StandardRole::BooleanEvaluations,
                ),
            ] {
                kernel.relation(
                    bindings.get(specific).as_u128(),
                    bindings.get(general).as_u128(),
                    245_020 + index,
                    kc::SUBSETTING,
                    kp::SUBSETTING_SUBSETTED_FEATURE,
                );
            }
        }
        kernel.relation(
            bindings.get(StandardRole::OccurrenceSnapshots).as_u128(),
            bindings.get(StandardRole::Occurrence).as_u128(),
            245_004,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
        if with_time_enclosed_occurrences {
            kernel.origin = DeclaredOrigin::StandardLibrary {
                library: libraries.artifacts[&StandardLibraryArtifact::Semantic],
            };
            kernel.create(245_005, kc::FEATURE, "timeEnclosedOccurrences");
            kernel.member(
                bindings.get(StandardRole::Occurrence).as_u128(),
                245_005,
                245_006,
                kc::FEATURE_MEMBERSHIP,
            );
            kernel.changes.clear(id(245_006), kp::ELEMENT_DECLARED_NAME);
        }
    }
    let context = SemanticContext::for_snapshot(
        &all_kernel,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_standard_bindings(&kernel_roots, &libraries)
    .unwrap();
    customize(
        &mut kernel,
        context.id().standard_bindings.as_ref().unwrap(),
    );
    let kernel = kernel.finish();
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        kernel_roots.clone(),
    );
    let closed = close_result_structure_with_extension(
        &kernel,
        Default::default(),
        |overlay| kernel_context(overlay, &kernel_roots, &libraries),
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "kernel {:?}",
        closed.stages.last()
    );
    let certificate = closed.certificate.unwrap();
    let overlay = Arc::new(closed.overlay);
    let context = kernel_context(&overlay, &kernel_roots, &libraries)
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
        .with_producer_closure(certificate)
        .unwrap();
    let dependency = ProducerClosedDependency::new(overlay.clone(), &context, &registry).unwrap();
    drop(context);
    (dependency, libraries, kernel_roots)
}

#[test]
fn untyped_sysml_anchor_population_closes_over_a_genuinely_closed_kernel_layer() {
    use agq_kerml_semantics::{PublicationOverlayError, close_result_structure_with_extension};
    let (dependency, _, kernel_roots) = closed_kernel_anchor_fixture(false, false, false);
    let source = corpus_anchor_fixture_complete(true).0.finish();
    let base = dependency.project_snapshot();
    let mut changes = base.change_set();
    for record in source.model().elements() {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        changes.create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            changes.set(record.id(), property, slot.value().clone(), origin.clone());
        }
        if source
            .model()
            .registry()
            .is_subtype(record.metaclass(), kc::MEMBERSHIP)
            .unwrap()
        {
            changes.clear(record.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    let snapshot = base.apply(&changes).unwrap();
    let mut roots = kernel_roots.clone();
    roots.push(id(1));
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots.clone(),
    );
    let closed = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            let context = dependency
                .project_overlay_context(overlay, &[id(1)])
                .map_err(PublicationOverlayError::Context)?;
            Ok(crate::context::fixture_overlay_context(
                overlay,
                context,
                SysmlBaselineProfile::OPERATIONAL_V2,
            )
            .unwrap()
            .kerml)
        },
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "systems {:?}",
        closed.stages.last()
    );
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
    let certificate = closed.certificate.unwrap();
    for record in closed.overlay.model().elements() {
        for requirement in agq_kerml_semantics::SemanticClosureRequirement::ALL {
            assert!(
                certificate.is_closed(record.id(), requirement),
                "{:?} {requirement:?}",
                record.id()
            );
        }
    }
}
