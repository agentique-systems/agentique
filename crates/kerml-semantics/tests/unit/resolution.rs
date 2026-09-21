use crate as agq_kerml_semantics;
include!("../common/namespace_fixture.rs");

#[test]
fn required_general_scope_retains_an_independently_named_redefined_member() {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V2).unwrap(),
    ));
    let changes = base.change_set();
    let mut f = Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    };
    let mut bindings = BTreeMap::new();
    for (i, role) in StandardRole::ALL.into_iter().enumerate() {
        let n = 1000 + i as u128;
        f.create(n, role.specification().1);
        bindings.insert(role, id(n));
    }
    let binary = bindings[&StandardRole::BinaryLink];
    let performances = bindings[&StandardRole::Performances];
    let performance = bindings[&StandardRole::Performance];
    let occurrence = bindings[&StandardRole::Occurrence];
    f.create(1, c::NAMESPACE);
    f.create(500, c::STEP);
    f.create(501, c::FEATURE_TYPING);
    f.own(performances.as_u128(), 501);
    f.value(
        501,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(performances),
    );
    f.value(501, p::FEATURE_TYPING_TYPE, Value::Reference(performance));
    f.member(1, 151, 50, c::PACKAGE, "Occurrences");
    f.value(
        occurrence.as_u128(),
        p::ELEMENT_DECLARED_NAME,
        Value::String("Occurrence".into()),
    );
    f.create(152, c::OWNING_MEMBERSHIP);
    f.own(50, 152);
    f.changes.set(
        id(152),
        p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        SlotValue::Ordered(vec![Value::Reference(occurrence)]),
        origin(),
    );
    f.member(occurrence.as_u128(), 153, 502, c::FEATURE, "snapshots");
    bindings.insert(StandardRole::OccurrenceSnapshots, id(502));
    f.member(occurrence.as_u128(), 154, 503, c::FEATURE, "variable");
    f.value(503, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
    f.member(1, 101, 2, c::ASSOCIATION, "Renamed");
    f.member(1, 102, 3, c::ASSOCIATION, "Specific");
    for (owner, m, n, name) in [
        (binary.as_u128(), 201, 4, "first"),
        (binary.as_u128(), 202, 5, "second"),
        (2, 203, 6, "renamedFirst"),
        (2, 204, 7, "renamedSecond"),
        (3, 205, 8, "narrowFirst"),
        (3, 206, 9, "narrowSecond"),
    ] {
        f.member(owner, m, n, c::FEATURE, name);
        f.value(n, p::FEATURE_IS_END, Value::Boolean(true));
    }
    for (r, s, g) in [(300, 2, binary), (301, 3, id(2))] {
        f.create(r, c::SPECIALIZATION);
        f.own(s, r);
        f.value(r, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(s)));
        f.value(r, p::SPECIALIZATION_GENERAL, Value::Reference(g));
    }
    for (r, s, g) in [(302, 6, 4), (303, 7, 5)] {
        f.create(r, c::REDEFINITION);
        f.own(s, r);
        f.value(
            r,
            p::REDEFINITION_REDEFINING_FEATURE,
            Value::Reference(id(s)),
        );
        f.value(
            r,
            p::REDEFINITION_REDEFINED_FEATURE,
            Value::Reference(id(g)),
        );
    }
    f.create(304, c::REDEFINITION);
    f.own(8, 304);
    f.value(
        304,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(8)),
    );
    f.create(800, c::CLASS);
    f.member(800, 801, 802, c::FEATURE, "renamedSnapshots");
    f.member(800, 803, 804, c::FEATURE, "variableState");
    f.member(800, 805, 806, c::FEATURE, "snapshots");
    f.value(804, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
    f.create(807, c::REDEFINITION);
    f.own(802, 807);
    f.value(
        807,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(802)),
    );
    f.value(
        807,
        p::REDEFINITION_REDEFINED_FEATURE,
        Value::Reference(id(502)),
    );
    f.create(808, c::TYPE_FEATURING);
    f.own(804, 808);
    f.value(
        808,
        p::TYPE_FEATURING_FEATURE_OF_TYPE,
        Value::Reference(id(804)),
    );
    f.value(
        808,
        p::TYPE_FEATURING_FEATURING_TYPE,
        Value::Reference(id(802)),
    );
    f.member(800, 809, 810, c::FEATURE, "anotherVariable");
    f.value(810, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
    let candidate = f.construction();
    let mut context = SemanticContext::for_construction(
        &candidate,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V2,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap();
    // Test the algorithm independently of binding-path validation (which
    // has its own production tests); no test-only binding API is exposed.
    context.id.standard_bindings = Some(Arc::new(StandardKermlBindings {
        targets: bindings
            .into_iter()
            .map(|(role, element)| {
                (
                    role,
                    BoundStandardElement {
                        element,
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
    let q = KerMlQueries::new(context);
    let implicit_type = q.feature_types(id(500));
    assert_eq!(implicit_type.completeness, Completeness::Complete);
    assert!(implicit_type.value.contains(&performance));
    let domain = q.featuring_types(id(503));
    assert_eq!(
        domain.completeness,
        Completeness::Complete,
        "{:?}",
        domain.diagnostics
    );
    assert_eq!(domain.value, vec![id(502)]);
    let domain = q.featuring_types(id(804));
    assert_eq!(
        domain.completeness,
        Completeness::Complete,
        "{:?}",
        domain.diagnostics
    );
    assert_eq!(domain.value, vec![id(802)]);
    assert!(q.is_featuring_type(id(810), id(802)).value);
    let missing_domain = q.featuring_types(id(810));
    assert_eq!(missing_domain.completeness, Completeness::Incomplete);
    assert!(
        missing_domain.value.is_empty(),
        "An eligible candidate does not assert a domain relationship"
    );
    assert!(
        !q.is_featuring_type(id(804), id(806)).value,
        "Spelling is not a standard role"
    );
    assert!(
        !q.is_featuring_type(id(804), id(502)).value,
        "Inherited global snapshots do not have this owner domain"
    );
    assert!(
        q.lookup_member(id(2), "first", MemberAccess::NonPrivate)
            .value
            .is_empty()
    );
    let found = q.lookup_relationship_target(
        id(304),
        p::REDEFINITION_REDEFINED_FEATURE,
        &QualifiedName {
            absolute: false,
            segments: vec!["first".into()],
        },
    );
    assert_eq!(
        found.value.iter().map(|m| m.element).collect::<Vec<_>>(),
        vec![id(4)]
    );
    assert_eq!(
        found.completeness,
        Completeness::Complete,
        "{:?}",
        found.diagnostics
    );
}
