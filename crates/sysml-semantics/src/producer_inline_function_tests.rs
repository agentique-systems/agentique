//! Interfaces::excludingOnce, including the isolated inline-body control and
//! its complete range/size, multiplicity, input-subsetting and final-result shape.
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
    excluding_once(false);
}

#[test]
fn full_excluding_once_range_capture_and_final_result_closes() {
    excluding_once(true);
}

fn literal(f: &mut Fixture, element: u128, infinity: bool) {
    let class = if infinity {
        kc::LITERAL_INFINITY
    } else {
        kc::LITERAL_INTEGER
    };
    f.changes.create(id(element), class, f.origin.clone());
    for property in f
        .base
        .model()
        .registry()
        .effective_properties(class)
        .unwrap()
    {
        if property.derived || property.multiplicity.lower == 0 {
            continue;
        }
        let value = match f
            .base
            .model()
            .registry()
            .storage_kind(property.value_kind)
            .unwrap()
        {
            ValueKind::Boolean => Value::Boolean(false),
            ValueKind::String => Value::String(String::new()),
            ValueKind::Integer => Value::Integer(1.into()),
            ValueKind::Reference(_) => continue,
            other => panic!("literal fixture domain {other:?}"),
        };
        f.changes.set(
            id(element),
            property.id,
            SlotValue::Scalar(value),
            f.origin.clone(),
        );
    }
    result(f, element, element + 1000, kc::FEATURE, true);
}

fn multiplicity(f: &mut Fixture, owner: u128, range: u128, unbounded: bool) {
    f.create(range, kc::MULTIPLICITY_RANGE, "");
    f.member(owner, range, range + 100_000, kc::OWNING_MEMBERSHIP);
    for (offset, infinity) in std::iter::once((1, false)).chain(unbounded.then_some((2, true))) {
        let element = range + offset;
        literal(f, element, infinity);
        f.member(range, element, element + 100_000, kc::OWNING_MEMBERSHIP);
    }
}

fn excluding_once(full: bool) {
    let (kernel, _, roots) =
        closed_kernel_anchor_fixture_with(true, false, false, |f, bindings| {
            let mut functions = vec![
                (
                    78_100,
                    "#",
                    bindings.get(StandardRole::BaseFunctions).as_u128(),
                    vec!["seq", "index"],
                ),
                (
                    78_200,
                    "==",
                    bindings.get(StandardRole::BaseFunctions).as_u128(),
                    vec!["x", "y"],
                ),
                (
                    78_300,
                    "selectOne",
                    bindings.get(StandardRole::ControlFunctions).as_u128(),
                    vec!["collection", "selector1"],
                ),
            ];
            if full {
                // Preserve directed parameter/result arity and the defaulted
                // third excludingAt parameter. Function bodies are not replayed.
                // The dependency must close under the same combined registry.
                let base_membership = Value::Reference(id(bindings
                    .get(StandardRole::BaseFunctions)
                    .as_u128()
                    + 100_000));
                let function_root = f
                    .owned
                    .iter()
                    .find(|(_, owned)| owned.contains(&base_membership))
                    .unwrap()
                    .0
                    .as_u128();
                f.create(78_800, kc::LIBRARY_PACKAGE, "SequenceFunctions");
                f.member(function_root, 78_800, 178_800, kc::OWNING_MEMBERSHIP);
                functions.extend([
                    (
                        78_400,
                        "..",
                        bindings.get(StandardRole::DataFunctions).as_u128(),
                        vec!["lower", "upper"],
                    ),
                    (78_500, "size", 78_800, vec!["seq"]),
                    (
                        78_600,
                        "excludingAt",
                        78_800,
                        vec!["seq", "startIndex", "endIndex"],
                    ),
                ]);
                for (offset, specific, general) in [
                    (0, StandardRole::Performance, StandardRole::Occurrence),
                    (1, StandardRole::Evaluation, StandardRole::Performance),
                ] {
                    f.relation(
                        bindings.get(specific).as_u128(),
                        bindings.get(general).as_u128(),
                        278_700 + offset,
                        kc::SUBCLASSIFICATION,
                        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                    );
                }
                f.create(78_900, kc::DATA_TYPE, "Natural");
                f.relation(
                    78_900,
                    bindings.get(StandardRole::DataValue).as_u128(),
                    278_900,
                    kc::SUBCLASSIFICATION,
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
            }
            for (function, name, namespace, parameters) in functions {
                f.create(function, kc::FUNCTION, name);
                f.member(
                    namespace,
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
                result(f, function, function + 90, kc::FEATURE, false);
                if full && function == 78_600 {
                    reference_value(f, 78_603, 78_610, 78_602);
                    f.value(178_610, kp::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(true));
                    result(f, 78_610, 78_611, kc::FEATURE, true);
                }
            }
            for relationships in f.owned.values() {
                for value in relationships {
                    if let Value::Reference(id) = value {
                        f.changes.clear(*id, kp::ELEMENT_DECLARED_NAME);
                    }
                }
            }
        });
    let performance = kernel
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::Performance);
    let evaluation = kernel
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::Evaluation);
    let (dependency, roots) = expression_dependency_from_kernel_with(
        kernel,
        roots,
        "isEmpty",
        &["seq"],
        |f, roles, occurrence| {
            f.relation(
                roles[&StandardSysmlRole::Action],
                if full {
                    performance.as_u128()
                } else {
                    occurrence.as_u128()
                },
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
            if full {
                f.relation(
                    roles[&StandardSysmlRole::Calculation],
                    evaluation.as_u128(),
                    278_402,
                    kc::SUBCLASSIFICATION,
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
            }
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
    if full {
        f.value(78_001, kp::FEATURE_IS_ORDERED, Value::Boolean(true));
        f.value(78_001, kp::FEATURE_IS_UNIQUE, Value::Boolean(false));
        f.value(78_002, kp::FEATURE_IS_UNIQUE, Value::Boolean(true));
        multiplicity(&mut f, 78_001, 81_000, true);
        multiplicity(&mut f, 78_002, 81_010, false);
        f.relation(
            78_002,
            78_001,
            278_002,
            kc::SUBSETTING,
            kp::SUBSETTING_SUBSETTED_FEATURE,
        );
    }
    f.create(78_003, sc::ATTRIBUTE_USAGE, "position");
    f.member(78_000, 78_003, 178_003, kc::FEATURE_MEMBERSHIP);
    f.create(78_004, kc::INVOCATION_EXPRESSION, "");
    f.member(78_003, 78_004, 178_004, kc::FEATURE_VALUE);
    input(&mut f, 78_004, 78_005);
    if full {
        multiplicity(&mut f, 78_003, 81_020, false);
        f.relation(
            78_003,
            78_900,
            278_003,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
        f.create(78_040, kc::OPERATOR_EXPRESSION, "");
        f.value(
            78_040,
            kp::OPERATOR_EXPRESSION_OPERATOR,
            Value::String("..".into()),
        );
        f.member(78_005, 78_040, 178_040, kc::FEATURE_VALUE);
        input(&mut f, 78_040, 78_041);
        literal(&mut f, 78_042, false);
        f.member(78_041, 78_042, 178_042, kc::FEATURE_VALUE);
        input(&mut f, 78_040, 78_044);
        f.create(78_045, kc::INVOCATION_EXPRESSION, "");
        f.member(78_044, 78_045, 178_045, kc::FEATURE_VALUE);
        reference(&mut f, 78_045, 78_500, 278_045);
        input(&mut f, 78_045, 78_046);
        reference_value(&mut f, 78_046, 78_047, 78_001);
        result(&mut f, 78_047, 78_048, sc::REFERENCE_USAGE, false);
        result(&mut f, 78_045, 78_049, sc::REFERENCE_USAGE, false);
        result(&mut f, 78_040, 78_050, sc::REFERENCE_USAGE, false);
    } else {
        reference_value(&mut f, 78_005, 78_006, 78_001);
        result(&mut f, 78_006, 78_026, sc::REFERENCE_USAGE, false);
    }
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
    if full {
        // The calculation inherits Evaluation's result; only its final body
        // invocation owns the explicit EmptyResultMember/ReferenceUsage result.
        f.create(78_060, kc::INVOCATION_EXPRESSION, "");
        f.member(78_000, 78_060, 178_060, kc::RESULT_EXPRESSION_MEMBERSHIP);
        reference(&mut f, 78_060, 78_600, 278_060);
        input(&mut f, 78_060, 78_061);
        reference_value(&mut f, 78_061, 78_062, 78_001);
        result(&mut f, 78_062, 78_063, sc::REFERENCE_USAGE, false);
        input(&mut f, 78_060, 78_064);
        reference_value(&mut f, 78_064, 78_065, 78_003);
        result(&mut f, 78_065, 78_066, sc::REFERENCE_USAGE, false);
        result(&mut f, 78_060, 78_067, sc::REFERENCE_USAGE, false);
    }
    for subject in (78_004..=78_018)
        .chain(78_026..=78_034)
        .filter(|&id| id != 78_010)
    {
        if !full || ![78_006, 78_026].contains(&subject) {
            f.changes.clear(id(subject), kp::ELEMENT_DECLARED_NAME);
        }
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
    eprintln!(
        "excludingOnce full={full}: rounds={}, converged={}, completeness={:?}",
        closed.counters.fixed_point_rounds, closed.converged, closed.completeness
    );
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
    for subject in if full {
        &[78_001, 78_002, 78_003][..]
    } else {
        &[78_003][..]
    } {
        for property in [
            agq_sysml::properties::USAGE_MAY_TIME_VARY,
            kp::FEATURE_IS_VARIABLE,
        ] {
            assert!(
                matches!(closed.overlay.model().property_state(id(*subject), property),
                    Ok(agq_kernel::derived::PropertyState::Computed(slot)) if slot.value() == &SlotValue::Scalar(Value::Boolean(true))),
                "{subject}: the inputs and captured-body value must retain the actual variable branch"
            );
        }
    }
    if full {
        let queries = KerMlQueries::new(context(&closed.overlay, &dependency, &roots).unwrap());
        let actual_result = queries.result_parameters(id(78_000));
        assert_eq!(actual_result.completeness, Completeness::Complete);
        assert_eq!(actual_result.value.len(), 1);
        assert!(
            dependency
                .overlay()
                .model()
                .element(actual_result.value[0])
                .is_some(),
            "the result must retain the inherited Evaluation identity"
        );
    }
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
