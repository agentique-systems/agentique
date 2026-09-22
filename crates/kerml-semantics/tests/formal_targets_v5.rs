//! Independent canonical witnesses; library spelling is used only during binding.
include!("common/namespace_fixture.rs");
use agq_kerml::BaselineProfile as Profile;

const PROFILES: [Profile; 6] = [
    Profile::PublishedKerMl10,
    Profile::OPERATIONAL_V1,
    Profile::OPERATIONAL_V2,
    Profile::OPERATIONAL_V3,
    Profile::OPERATIONAL_V4,
    Profile::OPERATIONAL_V5,
];

#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    Missing,
    WrongClass,
    WrongLibrary,
    Duplicate,
    Private,
}

fn fixture(
    profile: Profile,
    fault_rule: FormalConstraintId,
    fault: Fault,
    published: bool,
) -> (Snapshot, BTreeMap<FormalConstraintId, ElementId>) {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let changes = base.change_set();
    let mut f = Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    };
    f.create(1, c::NAMESPACE);
    let mut ids = BTreeMap::<Vec<&str>, u128>::new();
    let mut targets = BTreeMap::new();
    let mut next = 10;
    for rule in FormalConstraintId::ALL {
        let spec = rule.target();
        let path = if published {
            spec.published_target
        } else {
            spec.operational_target
        };
        let classes = [
            c::LIBRARY_PACKAGE,
            if published && rule == FormalConstraintId::FeatureSubobjectSpecialization {
                c::CLASS
            } else {
                spec.owner_metaclass
            },
            spec.target_metaclass,
        ];
        let mut owner = 1;
        for depth in 0..3 {
            let key = path[..=depth].to_vec();
            owner = if let Some(&existing) = ids.get(&key) {
                existing
            } else {
                let n = next;
                next += 2;
                f.create(
                    n,
                    if depth == 2 && rule == fault_rule && matches!(fault, Fault::WrongClass) {
                        c::BOOLEAN_EXPRESSION
                    } else {
                        classes[depth]
                    },
                );
                f.value(
                    n,
                    p::ELEMENT_DECLARED_NAME,
                    Value::String(
                        if depth == 2 && rule == fault_rule && matches!(fault, Fault::Missing) {
                            "unrelated"
                        } else {
                            path[depth]
                        }
                        .into(),
                    ),
                );
                f.create(
                    n + 1,
                    if depth == 2 {
                        c::FEATURE_MEMBERSHIP
                    } else {
                        c::OWNING_MEMBERSHIP
                    },
                );
                f.own(owner, n + 1);
                f.changes.set(
                    id(n + 1),
                    p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                    SlotValue::Ordered(vec![Value::Reference(id(n))]),
                    origin(),
                );
                if depth == 2 && rule == fault_rule && matches!(fault, Fault::Private) {
                    f.visibility(n + 1, "private");
                }
                if depth == 2 && rule == fault_rule && matches!(fault, Fault::Duplicate) {
                    f.member(owner, 9001, 9000, c::FEATURE, path[depth]);
                }
                ids.insert(key, n);
                n
            };
        }
        targets.insert(rule, id(owner));
    }
    // Same arbitrary subject name for every rule; behavior depends on metaclass,
    // flag, owning membership and explicit typing, never a declaration name.
    let spec = fault_rule.target();
    f.member(1, 1001, 1000, spec.owner_metaclass, "tamarind");
    f.create(1002, spec.target_metaclass);
    f.value(
        1002,
        p::ELEMENT_DECLARED_NAME,
        Value::String("mango".into()),
    );
    f.create(1003, c::FEATURE_MEMBERSHIP);
    f.own(1000, 1003);
    f.changes.set(
        id(1003),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(1002))]),
        origin(),
    );
    f.value(1002, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.value(1002, p::FEATURE_IS_PORTION, Value::Boolean(true));
    f.create(1004, c::FEATURE_TYPING);
    f.value(
        1004,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(1002)),
    );
    f.value(1004, p::FEATURE_TYPING_TYPE, Value::Reference(id(1000)));
    f.own(1002, 1004);
    if !published {
        let target = targets[&fault_rule].as_u128();
        let target_owner = ids[&spec.operational_target[..2].to_vec()];
        let flag = if fault_rule == FormalConstraintId::FeaturePortionSpecialization {
            p::FEATURE_IS_PORTION
        } else {
            p::FEATURE_IS_COMPOSITE
        };
        if fault_rule != FormalConstraintId::StepEnclosedPerformanceSpecialization {
            f.value(target, flag, Value::Boolean(true));
        }
        f.create(8500, c::FEATURE_TYPING);
        f.value(
            8500,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(target)),
        );
        f.value(
            8500,
            p::FEATURE_TYPING_TYPE,
            Value::Reference(id(target_owner)),
        );
        f.own(target, 8500);
    }
    let input = f.finish();
    let output = Snapshot::new(Arc::new(input.model().registry().clone()));
    let mut changes = output.change_set();
    for record in input.model().elements() {
        let library = if matches!(fault, Fault::WrongLibrary) && record.id() == targets[&fault_rule]
        {
            LibraryId::from_u128(8)
        } else {
            LibraryId::from_u128(7)
        };
        let provenance = DeclaredOrigin::StandardLibrary { library };
        changes.create(record.id(), record.metaclass(), provenance.clone());
        for (property, slot) in record.slots() {
            changes.set(
                record.id(),
                property,
                slot.value().clone(),
                provenance.clone(),
            );
        }
    }
    for link in input.model().association_occurrences() {
        changes.link(
            link.id(),
            link.association(),
            link.ends().clone(),
            link.positions().clone(),
            DeclaredOrigin::StandardLibrary {
                library: LibraryId::from_u128(7),
            },
        );
    }
    (output.apply(&changes).unwrap(), targets)
}

fn queries(snapshot: &Snapshot, profile: Profile) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(
            snapshot,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap()
        .with_formal_constraint_targets(&[id(1)], LibraryId::from_u128(7)),
    )
}

#[test]
fn six_rules_six_profiles_exact_targets_and_arbitrary_names() {
    let mut contexts = BTreeSet::new();
    for profile in PROFILES {
        for rule in FormalConstraintId::ALL {
            let (snapshot, targets) = fixture(profile, rule, Fault::None, false);
            let q = queries(&snapshot, profile);
            contexts.insert((
                q.context().baseline_profile_id,
                q.context().errata_manifest_digest,
            ));
            let bound = q.formal_constraint_target(rule);
            let applies = q.formal_constraint_applies(rule, id(1002));
            assert_eq!(
                applies.completeness,
                Completeness::Complete,
                "{:?}",
                applies.diagnostics
            );
            assert!(applies.value, "{rule:?}");
            if profile == Profile::OPERATIONAL_V5 {
                assert_eq!(
                    bound.completeness,
                    Completeness::Complete,
                    "{:?}",
                    bound.diagnostics
                );
                assert_eq!(bound.value, Some(targets[&rule]));
                let reflexive = q.formal_constraint_applies(rule, targets[&rule]);
                assert!(reflexive.value, "{rule:?}");
                assert_eq!(reflexive.completeness, Completeness::Complete);
                let validation = q.validate_formal_target_constraints(targets[&rule]);
                assert_eq!(
                    validation.completeness,
                    Completeness::Complete,
                    "{:?}",
                    validation.diagnostics
                );
                assert!(validation.diagnostics.iter().all(|d| d.code != rule.name()));
                let specializations = q.all_specializations(id(1002));
                assert!(
                    specializations.value.contains(&targets[&rule]),
                    "{rule:?}: {:?}",
                    specializations.diagnostics
                );
                assert!(
                    specializations
                        .search_dependencies
                        .contains(&SearchDependency::FormalConstraintTarget(rule))
                );
                assert!(
                    bound
                        .explanations
                        .values()
                        .flatten()
                        .any(|e| e.rule == Rule::OperationalFormalConstraintTargetV1)
                );
            } else {
                assert_eq!(bound.value, None);
                assert!(
                    bound
                        .diagnostics
                        .iter()
                        .any(|d| d.code == "KQ_FORMAL_TARGET_MISSING")
                );
                let (published, literal) = fixture(profile, rule, Fault::None, true);
                let q = queries(&published, profile);
                let bound = q.formal_constraint_target(rule);
                assert_eq!(
                    bound.completeness,
                    Completeness::Complete,
                    "{:?}",
                    bound.diagnostics
                );
                assert_eq!(bound.value, Some(literal[&rule]));
            }
            println!("{} {rule:?}: exact target contract checked", profile.id());
        }
    }
    assert_eq!(contexts.len(), 6);
}

#[test]
fn every_rule_rejects_corrupt_bindings_without_fallback() {
    for rule in FormalConstraintId::ALL {
        for (fault, code) in [
            (Fault::Missing, "KQ_FORMAL_TARGET_MISSING"),
            (Fault::WrongClass, "KQ_FORMAL_TARGET_METACLASS"),
            (Fault::WrongLibrary, "KQ_FORMAL_TARGET_LIBRARY"),
            (Fault::Duplicate, "KQ_FORMAL_TARGET_AMBIGUOUS"),
            (Fault::Private, "KQ_FORMAL_TARGET_VISIBILITY"),
        ] {
            let (snapshot, _) = fixture(Profile::OPERATIONAL_V5, rule, fault, false);
            let q = queries(&snapshot, Profile::OPERATIONAL_V5);
            let answer = q.formal_constraint_target(rule);
            assert_eq!(answer.value, None, "{rule:?} {fault:?}");
            assert_eq!(answer.completeness, Completeness::Invalid);
            assert!(
                answer.diagnostics.iter().any(|d| d.code == code),
                "{rule:?} {fault:?}: {:?}",
                answer.diagnostics
            );
        }
        let (snapshot, _) = fixture(Profile::OPERATIONAL_V5, rule, Fault::None, false);
        let v5 = queries(&snapshot, Profile::OPERATIONAL_V5);
        let v4 = queries(&snapshot, Profile::OPERATIONAL_V4);
        assert_ne!(v4.context(), v5.context());
        assert!(v4.formal_constraint_target(rule).value.is_none());
        assert!(v5.formal_constraint_target(rule).value.is_some());
    }
}

#[test]
fn typed_contracts_equal_every_frozen_manifest_row() {
    let manifest: serde_json::Value =
        serde_json::from_str(agq_kerml::OPERATIONAL_FORMAL_TARGET_ERRATA_V5_MANIFEST).unwrap();
    let rows = manifest["entries"].as_array().unwrap();
    assert_eq!(rows.len(), FormalConstraintId::ALL.len());
    for rule in FormalConstraintId::ALL {
        let target = rule.target();
        let row = rows
            .iter()
            .find(|r| r["rule_id"] == target.rule_id)
            .unwrap();
        assert_eq!(row["rule"], rule.name());
        assert_eq!(row["published_target"], target.published_target.join("::"));
        assert_eq!(
            row["operational_target"],
            target.operational_target.join("::")
        );
        assert_eq!(row["issue"], target.authority);
    }
}

#[test]
fn incomplete_owner_typing_cannot_make_an_antecedent_vacuously_complete() {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(Profile::OPERATIONAL_V5).unwrap(),
    ));
    let changes = base.change_set();
    let mut f = Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    };
    f.create(1, c::NAMESPACE);
    f.member(1, 11, 2, c::FEATURE, "outer");
    f.create(3, c::FEATURE);
    f.create(12, c::FEATURE_MEMBERSHIP);
    f.own(2, 12);
    f.changes.set(
        id(12),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(id(3))]),
        origin(),
    );
    f.value(3, p::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    let untyped = f.finish();
    let q = queries(&untyped, Profile::OPERATIONAL_V5);
    let inapplicable =
        q.formal_constraint_applies(FormalConstraintId::FeatureSubobjectSpecialization, id(3));
    assert!(!inapplicable.value);
    assert_eq!(inapplicable.completeness, Completeness::Incomplete);
    assert!(
        inapplicable
            .search_dependencies
            .contains(&SearchDependency::ProducerClosure {
                subject: id(3),
                requirement: SemanticClosureRequirement::EffectiveTyping,
                certificate_digest: None,
            })
    );
    let changes = untyped.change_set();
    let mut f = Fixture {
        base: untyped,
        changes,
        owned: BTreeMap::new(),
    };
    f.create(4, c::STRUCTURE);
    f.create(5, c::FEATURE_TYPING);
    f.own(3, 5);
    f.value(5, p::FEATURE_TYPING_TYPED_FEATURE, Value::Reference(id(3)));
    f.value(5, p::FEATURE_TYPING_TYPE, Value::Reference(id(4)));
    let snapshot = f.finish();
    let q = queries(&snapshot, Profile::OPERATIONAL_V5);
    let result =
        q.formal_constraint_applies(FormalConstraintId::FeatureSubobjectSpecialization, id(3));
    assert_eq!(result.completeness, Completeness::Incomplete);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_FORMAL_ANTECEDENT_TYPE_CLOSURE")
    );
}
