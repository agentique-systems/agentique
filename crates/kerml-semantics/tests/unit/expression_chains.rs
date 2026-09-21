use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn operator_target_source_redefinitions_and_result_chain_close_in_canonical_stages() {
    let profile = agq_kerml::BaselineProfile::OPERATIONAL_V8;
    let options = SemanticOptions {
        baseline_profile: profile,
        ..Default::default()
    };
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
    let mut context =
        SemanticContext::for_snapshot(&snapshot, options.clone(), BTreeSet::new()).unwrap();
    context.id.standard_bindings = Some(bindings.clone());
    let q = KerMlQueries::new(context);
    assert_eq!(q.instantiated_type(id(1)).value, Some(id(500)));
    let mut derived = q
        .plan_result_structure([id(1), id(2)])
        .materialize(&snapshot)
        .unwrap();
    let mut converged = false;
    for _ in 0..6 {
        let mut context =
            SemanticContext::for_overlay(&derived.overlay, options.clone(), BTreeSet::new())
                .unwrap();
        context.id.standard_bindings = Some(bindings.clone());
        let q = KerMlQueries::new(context);
        let subjects = derived
            .overlay
            .model()
            .elements()
            .filter(|r| matches!(r.origin(), Origin::Derived(_)))
            .map(|r| r.id())
            .chain([id(1), id(2)])
            .collect::<BTreeSet<_>>();
        let next = q
            .plan_result_structure(subjects)
            .materialize_on_overlay(&derived.overlay)
            .unwrap();
        let stable = next.overlay.model().len() == derived.overlay.model().len();
        derived = next;
        if stable {
            assert_eq!(
                derived.production.completeness,
                Completeness::Complete,
                "{:?}",
                derived.production.diagnostics
            );
            converged = true;
            break;
        }
    }
    assert!(converged);
    let mut context =
        SemanticContext::for_overlay(&derived.overlay, options, BTreeSet::new()).unwrap();
    context.id.standard_bindings = Some(bindings);
    let q = KerMlQueries::new(context);
    let source_target = q.source_target_feature(id(1)).value.unwrap();
    assert_eq!(q.owning_type(source_target).value, Some(id(2)));
    assert_eq!(
        q.redefined_features(source_target)
            .value
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([id(900), standard_target])
    );
    let result = q.structural_result(id(1)).value.unwrap();
    assert_eq!(q.owning_type(result).value, Some(id(1)));
    assert!(q.feature_types(result).value.contains(&id(901)));
    let chain = q
        .subsetted_features(result)
        .value
        .into_iter()
        .find(|&f| q.chaining_features(f).value == [id(2), source_target])
        .unwrap();
    assert!(derived.overlay.explain(FactKey::Element(chain)).is_some());
    assert_eq!(
        snapshot
            .model()
            .instances(c::RETURN_PARAMETER_MEMBERSHIP, true)
            .unwrap()
            .count(),
        0
    );
}
