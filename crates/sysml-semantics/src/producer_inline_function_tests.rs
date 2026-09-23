//! Interfaces::excludingOnce inline body/capture shape; the first range/size
//! argument is reduced to `seq`, preserving `seq->selectOne{in i; seq#(i)==value}`.
use super::*;

fn result(f: &mut Fixture, owner: u128, feature: u128, class: MetaclassId, implied: bool) {
    f.create(feature, class, "");
    set_enum(f, feature, kp::FEATURE_DIRECTION, "out");
    f.member(
        owner,
        feature,
        feature + 100_000,
        kc::RETURN_PARAMETER_MEMBERSHIP,
    );
    if implied {
        f.value(
            feature + 100_000,
            kp::RELATIONSHIP_IS_IMPLIED,
            Value::Boolean(true),
        );
        f.value(owner, kp::ELEMENT_IS_IMPLIED_INCLUDED, Value::Boolean(true));
    }
}

fn input(f: &mut Fixture, owner: u128, feature: u128) {
    f.create(feature, kc::FEATURE, "");
    set_enum(f, feature, kp::FEATURE_DIRECTION, "in");
    f.member(owner, feature, feature + 100_000, kc::PARAMETER_MEMBERSHIP);
}

fn reference_value(f: &mut Fixture, owner: u128, expression: u128, target: u128) {
    f.create(expression, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(owner, expression, expression + 100_000, kc::FEATURE_VALUE);
    reference(f, expression, target, expression + 200_000);
}

#[test]
fn inline_function_body_with_outer_capture_and_index_closes() {
    let (kernel, _, roots) =
        closed_kernel_anchor_fixture_with(true, false, false, |f, bindings| {
            for (function, name, namespace, parameters) in [
                (78_100, "#", StandardRole::BaseFunctions, ["seq", "index"]),
                (78_200, "==", StandardRole::BaseFunctions, ["x", "y"]),
                (
                    78_300,
                    "selectOne",
                    StandardRole::ControlFunctions,
                    ["collection", "selector1"],
                ),
            ] {
                f.create(function, kc::FUNCTION, name);
                f.member(
                    bindings.get(namespace).as_u128(),
                    function,
                    function + 100_000,
                    kc::OWNING_MEMBERSHIP,
                );
                for (offset, name) in parameters.into_iter().enumerate() {
                    let parameter = function + 1 + offset as u128;
                    let selector = function == 78_300 && offset == 1;
                    f.create(
                        parameter,
                        if selector {
                            kc::EXPRESSION
                        } else {
                            kc::FEATURE
                        },
                        name,
                    );
                    set_enum(f, parameter, kp::FEATURE_DIRECTION, "in");
                    f.member(
                        function,
                        parameter,
                        parameter + 100_000,
                        kc::PARAMETER_MEMBERSHIP,
                    );
                    if selector {
                        input(f, parameter, 78_304);
                        result(f, parameter, 78_305, kc::FEATURE, false);
                    }
                }
                result(f, function, function + 3, kc::FEATURE, false);
            }
            for relationships in f.owned.values() {
                for value in relationships {
                    if let Value::Reference(id) = value {
                        f.changes.clear(*id, kp::ELEMENT_DECLARED_NAME);
                    }
                }
            }
        });
    let (dependency, roots) = expression_dependency_from_kernel_with(
        kernel,
        roots,
        "isEmpty",
        &["seq"],
        |f, roles, occurrence| {
            f.relation(
                roles[&StandardSysmlRole::Action],
                occurrence.as_u128(),
                278_400,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            f.relation(
                roles[&StandardSysmlRole::Calculation],
                roles[&StandardSysmlRole::Action],
                278_401,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
        },
    );
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(78_000, sc::CALCULATION_DEFINITION, "excludingOnce");
    for (feature, name) in [(78_001, "seq"), (78_002, "value")] {
        f.create(feature, sc::REFERENCE_USAGE, name);
        set_enum(&mut f, feature, kp::FEATURE_DIRECTION, "in");
        f.member(78_000, feature, feature + 100_000, kc::FEATURE_MEMBERSHIP);
    }
    f.create(78_003, sc::ATTRIBUTE_USAGE, "position");
    f.member(78_000, 78_003, 178_003, kc::FEATURE_MEMBERSHIP);
    f.create(78_004, kc::INVOCATION_EXPRESSION, "");
    f.member(78_003, 78_004, 178_004, kc::FEATURE_VALUE);
    input(&mut f, 78_004, 78_005);
    reference_value(&mut f, 78_005, 78_006, 78_001);
    result(&mut f, 78_006, 78_026, sc::REFERENCE_USAGE, false);
    reference(&mut f, 78_004, 78_300, 278_004);
    input(&mut f, 78_004, 78_007);
    // BodyExpression -> FRE owning its referent Expression through FeatureMembership.
    f.create(78_008, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(78_007, 78_008, 178_008, kc::FEATURE_VALUE);
    f.create(78_009, kc::EXPRESSION, "");
    f.member(78_008, 78_009, 178_009, kc::FEATURE_MEMBERSHIP);
    f.create(78_010, kc::FEATURE, "i");
    set_enum(&mut f, 78_010, kp::FEATURE_DIRECTION, "in");
    f.member(78_009, 78_010, 178_010, kc::FEATURE_MEMBERSHIP);
    f.create(78_011, kc::OPERATOR_EXPRESSION, "");
    f.value(
        78_011,
        kp::OPERATOR_EXPRESSION_OPERATOR,
        Value::String("==".into()),
    );
    f.member(78_009, 78_011, 178_011, kc::RESULT_EXPRESSION_MEMBERSHIP);
    input(&mut f, 78_011, 78_012);
    f.create(78_013, kc::INDEX_EXPRESSION, "");
    f.value(
        78_013,
        kp::INDEX_EXPRESSION_OPERATOR,
        Value::String("#".into()),
    );
    f.member(78_012, 78_013, 178_013, kc::FEATURE_VALUE);
    input(&mut f, 78_013, 78_014);
    reference_value(&mut f, 78_014, 78_015, 78_001);
    result(&mut f, 78_015, 78_027, sc::REFERENCE_USAGE, false);
    f.create(78_016, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(78_013, 78_016, 178_016, kc::FEATURE_MEMBERSHIP);
    reference(&mut f, 78_016, 78_010, 278_016);
    result(&mut f, 78_016, 78_028, sc::REFERENCE_USAGE, false);
    result(&mut f, 78_013, 78_029, kc::FEATURE, true);
    input(&mut f, 78_011, 78_017);
    reference_value(&mut f, 78_017, 78_018, 78_002);
    result(&mut f, 78_018, 78_030, sc::REFERENCE_USAGE, false);
    result(&mut f, 78_011, 78_031, sc::REFERENCE_USAGE, false);
    result(&mut f, 78_009, 78_032, kc::FEATURE, true);
    result(&mut f, 78_008, 78_033, kc::FEATURE, true);
    result(&mut f, 78_004, 78_034, sc::REFERENCE_USAGE, false);
    for subject in (78_004..=78_018)
        .chain(78_026..=78_034)
        .filter(|&id| id != 78_010)
    {
        f.changes.clear(id(subject), kp::ELEMENT_DECLARED_NAME);
    }
    let snapshot = without_membership_names(f.finish());
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots.clone(),
    );
    let closed = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| context(overlay, &dependency, &roots),
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closed.converged);
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages.last()
    );
    assert!(
        closed
            .certificate
            .as_ref()
            .unwrap()
            .is_fully_closed(closed.overlay.model())
    );
    for property in [
        agq_sysml::properties::USAGE_MAY_TIME_VARY,
        kp::FEATURE_IS_VARIABLE,
    ] {
        assert!(
            matches!(closed.overlay.model().property_state(id(78_003), property),
                Ok(agq_kernel::derived::PropertyState::Computed(slot)) if slot.value() == &SlotValue::Scalar(Value::Boolean(true))),
            "the captured-body value must retain the actual variable branch"
        );
    }
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
