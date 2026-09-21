include!("common/namespace_fixture.rs");

#[test]
fn correction_fact_evidence_retains_entry_library_profile_and_output_identity() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V3;
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let changes = base.change_set();
    let mut f = Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    };
    f.create(1, c::NAMESPACE);
    f.member(1, 102, 2, c::FEATURE, "before");
    let correction = DeclaredOrigin::ReviewedCorrection {
        profile: profile.id().into(),
        entry: "reviewed-synthetic-entry".into(),
        authority: BTreeSet::from(["urn:review:synthetic".into()]),
        library: LibraryId::from_u128(40),
        source_key: "sha256:synthetic-content".into(),
        output_key: "feature/name".into(),
    };
    f.changes.set(
        id(2),
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("after".into())),
        correction.clone(),
    );
    let snapshot = f.finish();
    let pin = LibraryPin {
        name: "synthetic-content".into(),
        sha256: [40; 32],
    };
    let context = SemanticContext::for_snapshot(
        &snapshot,
        SemanticOptions {
            baseline_profile: profile,
            ..Default::default()
        },
        BTreeSet::from([pin.clone()]),
    )
    .unwrap();
    let q = KerMlQueries::new(context);
    let answer = q.lookup_member(id(1), "after", MemberAccess::All);
    let fact = FactKey::Property {
        element: id(2),
        property: p::ELEMENT_DECLARED_NAME,
    };
    assert_eq!(*answer.fact_origins[&fact], Origin::Declared(correction));
    assert!(answer.positive_dependencies.contains(&fact));
    assert!(
        answer
            .positive_dependencies
            .contains(&FactKey::Element(id(102)))
    );
    assert!(
        answer
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: id(1) })
    );
    assert_eq!(answer.context.baseline_profile_id, profile.id());
    assert_eq!(
        answer.context.errata_manifest_digest,
        profile.errata_manifest_sha256()
    );
    assert!(answer.context.pinned_libraries.contains(&pin));
    for wrong in [
        agq_kerml::BaselineProfile::OPERATIONAL_V1,
        agq_kerml::BaselineProfile::OPERATIONAL_V2,
    ] {
        assert!(
            matches!(SemanticContext::for_snapshot(&snapshot, SemanticOptions {baseline_profile:wrong,..Default::default()}, Default::default()), Err(ContextError::CorrectionProfileMismatch(key)) if key == fact)
        );
    }
}

fn edge(f: &mut Fixture, relationship: u128, class: MetaclassId, source: u128, target: u128) {
    f.create(relationship, class);
    let (specific, general) = match class {
        c::SUBCLASSIFICATION => (
            p::SUBCLASSIFICATION_SUBCLASSIFIER,
            p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        ),
        c::REDEFINITION => (
            p::REDEFINITION_REDEFINING_FEATURE,
            p::REDEFINITION_REDEFINED_FEATURE,
        ),
        c::FEATURE_TYPING => (p::FEATURE_TYPING_TYPED_FEATURE, p::FEATURE_TYPING_TYPE),
        c::SUBSETTING => (
            p::SUBSETTING_SUBSETTING_FEATURE,
            p::SUBSETTING_SUBSETTED_FEATURE,
        ),
        _ => panic!("unsupported witness edge"),
    };
    f.value(relationship, specific, Value::Reference(id(source)));
    f.value(relationship, general, Value::Reference(id(target)));
    f.own(source, relationship);
}

#[test]
fn two_explicit_type_paths_require_a_common_redefinition_before_publication() {
    for profile in [
        agq_kerml::BaselineProfile::PublishedKerMl10,
        agq_kerml::BaselineProfile::OPERATIONAL_V1,
        agq_kerml::BaselineProfile::OPERATIONAL_V2,
        agq_kerml::BaselineProfile::OPERATIONAL_V3,
    ] {
        for repaired in [false, true] {
            let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
            let changes = base.change_set();
            let mut f = Fixture {
                base,
                changes,
                owned: BTreeMap::new(),
            };
            f.create(1, c::NAMESPACE);
            for (membership, target, name) in
                [(102, 2, "Cobalt"), (103, 3, "Quartz"), (104, 4, "Jasper")]
            {
                f.member(1, membership, target, c::CLASS, name);
            }
            f.member(2, 110, 10, c::FEATURE, "tone");
            f.member(3, 111, 11, c::FEATURE, "tone");
            f.member(4, 112, 12, c::FEATURE, "tone");
            edge(&mut f, 200, c::SUBCLASSIFICATION, 3, 2);
            edge(&mut f, 201, c::SUBCLASSIFICATION, 4, 2);
            edge(&mut f, 202, c::REDEFINITION, 11, 10);
            edge(&mut f, 203, c::REDEFINITION, 12, 10);
            f.member(1, 120, 20, c::FEATURE, "cells");
            edge(&mut f, 204, c::FEATURE_TYPING, 20, 4);
            f.member(1, 121, 21, c::FEATURE, "edges");
            edge(&mut f, 205, c::FEATURE_TYPING, 21, 3);
            edge(&mut f, 206, c::SUBSETTING, 21, 20);
            if repaired {
                f.member(1, 130, 30, c::CLASS, "Combined");
                f.member(30, 131, 31, c::FEATURE, "tone");
                edge(&mut f, 207, c::SUBCLASSIFICATION, 30, 3);
                edge(&mut f, 208, c::SUBCLASSIFICATION, 30, 4);
                edge(&mut f, 209, c::REDEFINITION, 31, 11);
                edge(&mut f, 210, c::REDEFINITION, 31, 12);
                f.value(205, p::FEATURE_TYPING_TYPE, Value::Reference(id(30)));
            }
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(
                    &snapshot,
                    SemanticOptions {
                        baseline_profile: profile,
                        ..Default::default()
                    },
                    Default::default(),
                )
                .unwrap(),
            );
            let lookup = q.lookup_member(id(21), "tone", MemberAccess::All);
            let targets: Vec<_> = lookup
                .value
                .iter()
                .map(|m| m.element)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            assert_eq!(
                targets,
                if repaired {
                    vec![id(31)]
                } else {
                    vec![id(11), id(12)]
                }
            );
            assert_eq!(lookup.completeness, Completeness::Complete);
            let validation = q.validate_namespace_distinguishability(id(21));
            assert_eq!(
                validation.completeness,
                if repaired {
                    Completeness::Complete
                } else {
                    Completeness::Invalid
                }
            );
            assert!(
                !lookup
                    .explanations
                    .values()
                    .flatten()
                    .any(|p| p.rule == Rule::OperationalRedefinitionTargetV1)
            );
            println!(
                "AUTHORITY_WITNESS {}",
                serde_json::json!({"profile":profile.id(),"explicit_common_redefinition":repaired,
                "targets":targets.iter().map(|id|id.to_string()).collect::<Vec<_>>(),
                "lookup_completeness":format!("{:?}",lookup.completeness),
                "validation":format!("{:?}",validation.completeness),
                "positive_dependencies":lookup.positive_dependencies.iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),
                "search_dependencies":lookup.search_dependencies.iter().map(|d|format!("{d:?}")).collect::<Vec<_>>() })
            );
        }
    }
}

#[test]
fn positional_common_ends_and_results_suppress_both_inherited_memberships() {
    // Observation adds the missing second end; the two feature monitors add
    // both ends; the expression monitor adds a common return parameter.
    // Names and identities deliberately bear no resemblance to library names.
    for result in [false, true] {
        for repaired in [false, true] {
            let mut f = Fixture::new();
            f.create(1, c::NAMESPACE);
            let class = if result { c::FUNCTION } else { c::CLASS };
            for (membership, ty, name) in [(102, 2, "Amber"), (103, 3, "Birch"), (104, 4, "Cedar")]
            {
                f.member(1, membership, ty, class, name);
            }
            edge(&mut f, 200, c::SUBCLASSIFICATION, 4, 2);
            edge(&mut f, 201, c::SUBCLASSIFICATION, 4, 3);
            for (owner, membership, feature) in [(2, 110, 10), (3, 111, 11), (4, 112, 12)] {
                if owner == 4 && !repaired {
                    continue;
                }
                f.create(feature, c::FEATURE);
                f.value(
                    feature,
                    p::ELEMENT_DECLARED_NAME,
                    Value::String("sharedFacet".into()),
                );
                if result {
                    f.enumeration(feature, p::FEATURE_DIRECTION, "out");
                } else {
                    f.value(feature, p::FEATURE_IS_END, Value::Boolean(true));
                }
                f.create(
                    membership,
                    if result {
                        c::RETURN_PARAMETER_MEMBERSHIP
                    } else {
                        c::FEATURE_MEMBERSHIP
                    },
                );
                f.own(owner, membership);
                f.changes.set(
                    id(membership),
                    p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                    SlotValue::Ordered(vec![Value::Reference(id(feature))]),
                    origin(),
                );
            }
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                    .unwrap(),
            );
            let lookup = q.lookup_member(id(4), "sharedFacet", MemberAccess::All);
            assert_eq!(lookup.completeness, Completeness::Complete);
            assert_eq!(
                lookup
                    .value
                    .iter()
                    .map(|m| m.element)
                    .collect::<BTreeSet<_>>(),
                if repaired {
                    BTreeSet::from([id(12)])
                } else {
                    BTreeSet::from([id(10), id(11)])
                }
            );
            assert_eq!(
                q.validate_namespace_distinguishability(id(4)).completeness,
                if repaired {
                    Completeness::Complete
                } else {
                    Completeness::Invalid
                }
            );
        }
    }
}
