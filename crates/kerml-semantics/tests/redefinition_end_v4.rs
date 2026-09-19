include!("common/namespace_fixture.rs");

fn fixture(profile: agq_kerml::BaselineProfile) -> Fixture {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
    let changes = base.change_set();
    Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    }
}

#[test]
fn all_profiles_end_owner_matrix_preserves_flags_and_explains_disposition() {
    use agq_kerml::BaselineProfile as P;
    let mut identities = BTreeSet::new();
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
    ] {
        for (case, class, restricted) in [
            ("classifier", c::CLASS, false),
            ("association", c::ASSOCIATION, true),
            ("connector", c::CONNECTOR, true),
            ("association_subtype", c::ASSOCIATION_STRUCTURE, true),
            ("connector_subtype", c::BINDING_CONNECTOR, true),
            ("no_owning_type", c::NAMESPACE, false),
            ("plain_membership_in_type", c::CLASS, false),
            ("nested_expression", c::EXPRESSION, false),
            ("feature_chain_target", c::FEATURE, false),
            ("inherited_owner", c::CLASS, false),
            ("correction_feature", c::CLASS, false),
        ] {
            for target_end in [false, true] {
                for source_end in [false, true] {
                    let mut f = fixture(profile);
                    f.create(1, class);
                    f.member(1, 102, 2, c::FEATURE, "mango");
                    f.member(1, 103, 3, c::FEATURE, "tamarind");
                    if case == "no_owning_type" || case == "plain_membership_in_type" {
                        // OwningMembership alone does not establish Feature::owningType.
                        f.changes.remove(id(102));
                        f.owned
                            .get_mut(&id(1))
                            .unwrap()
                            .retain(|v| v != &Value::Reference(id(102)));
                        f.create(104, c::OWNING_MEMBERSHIP);
                        f.changes.set(
                            id(104),
                            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                            SlotValue::Ordered(vec![Value::Reference(id(2))]),
                            origin(),
                        );
                        f.own(1, 104);
                    }
                    if case == "nested_expression" || case == "feature_chain_target" {
                        // An outer Association does not replace the immediate owning type.
                        f.create(10, c::ASSOCIATION);
                        f.create(110, c::FEATURE_MEMBERSHIP);
                        f.changes.set(
                            id(110),
                            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                            SlotValue::Ordered(vec![Value::Reference(id(1))]),
                            origin(),
                        );
                        f.own(10, 110);
                    }
                    if case == "inherited_owner" {
                        f.create(10, c::ASSOCIATION);
                        f.create(210, c::SPECIALIZATION);
                        f.value(210, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(10)));
                        f.value(210, p::SPECIALIZATION_GENERAL, Value::Reference(id(1)));
                        f.own(10, 210);
                    }
                    let flag_origin =
                        if case == "correction_feature" && profile.corrects_library_content() {
                            DeclaredOrigin::ReviewedCorrection {
                                profile: P::OPERATIONAL_V3.id().into(),
                                entry: "arbitrary-reviewed-witness".into(),
                                authority: BTreeSet::from(["synthetic-test".into()]),
                                library: LibraryId::from_u128(900),
                                source_key: "synthetic".into(),
                                output_key: "end-flag".into(),
                            }
                        } else {
                            origin()
                        };
                    f.changes.set(
                        id(2),
                        p::FEATURE_IS_END,
                        SlotValue::Scalar(Value::Boolean(source_end)),
                        flag_origin.clone(),
                    );
                    f.value(3, p::FEATURE_IS_END, Value::Boolean(target_end));
                    f.create(200, c::REDEFINITION);
                    f.value(
                        200,
                        p::REDEFINITION_REDEFINING_FEATURE,
                        Value::Reference(id(2)),
                    );
                    f.value(
                        200,
                        p::REDEFINITION_REDEFINED_FEATURE,
                        Value::Reference(id(3)),
                    );
                    f.own(2, 200);
                    // Some isolated Association subclasses have unrelated mandatory
                    // participant obligations. The end-conformance query reads neither.
                    let candidate = f.construction();
                    let q = KerMlQueries::new(
                        SemanticContext::for_construction(
                            &candidate,
                            SemanticOptions {
                                baseline_profile: profile,
                                ..Default::default()
                            },
                            Default::default(),
                        )
                        .unwrap(),
                    );
                    let answer = q.validate_redefinition_end_conformance(id(200));
                    let expected =
                        !target_end || source_end || (profile == P::OPERATIONAL_V4 && !restricted);
                    assert_eq!(
                        answer.completeness,
                        if expected {
                            Completeness::Complete
                        } else {
                            Completeness::Invalid
                        },
                        "{profile:?} {case} {source_end} -> {target_end}: {:?}",
                        answer.diagnostics
                    );
                    let disposition = answer.value.as_ref().unwrap();
                    assert_eq!(disposition.valid, expected);
                    assert_eq!(disposition.redefining_is_end, source_end);
                    assert_eq!(disposition.redefined_is_end, target_end);
                    assert_eq!(
                        disposition.owning_type,
                        if case == "no_owning_type" || case == "plain_membership_in_type" {
                            None
                        } else {
                            Some(id(1))
                        }
                    );
                    assert_eq!(
                        disposition.rule,
                        if profile == P::OPERATIONAL_V4 {
                            Rule::OperationalRedefinitionEndConformanceV1
                        } else {
                            Rule::PublishedRedefinitionEndConformance
                        }
                    );
                    for element in [2, 3] {
                        assert!(answer.positive_dependencies.contains(&FactKey::Property {
                            element: id(element),
                            property: p::FEATURE_IS_END
                        }));
                    }
                    assert_eq!(
                        answer.fact_origins[&FactKey::Property {
                            element: id(2),
                            property: p::FEATURE_IS_END
                        }],
                        Origin::Declared(flag_origin)
                    );
                    assert!(
                        answer
                            .explanations
                            .values()
                            .flatten()
                            .any(|e| e.rule == disposition.rule)
                    );
                    identities.insert((
                        answer.context.baseline_profile_id,
                        answer.context.errata_manifest_digest,
                    ));
                    println!(
                        "profile={} case={case} redefined_end={target_end} redefining_end={source_end} owner={:?} rule={:?} valid={expected}",
                        profile.id(),
                        disposition.owner_metaclass,
                        disposition.rule
                    );
                }
            }
        }
    }
    assert_eq!(identities.len(), 5);
}

#[test]
fn unresolved_endpoint_does_not_pass_end_conformance() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V4;
    let mut f = fixture(profile);
    f.create(1, c::FEATURE);
    f.create(2, c::REDEFINITION);
    f.own(1, 2);
    f.value(
        2,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(1)),
    );
    let candidate = f.construction();
    let q = KerMlQueries::new(
        SemanticContext::for_construction(
            &candidate,
            SemanticOptions {
                baseline_profile: profile,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap(),
    );
    let answer = q.validate_redefinition_end_conformance(id(2));
    assert_eq!(answer.completeness, Completeness::Incomplete);
    assert!(answer.value.is_none());
}
