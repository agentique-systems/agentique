//! VerificationCases objective subject valuation and referenced requirement chain.
use super::*;

fn result(f: &mut Fixture, expression: u128, result: u128, class: MetaclassId) {
    f.create(result, class, "");
    set_enum(f, result, kp::FEATURE_DIRECTION, "out");
    f.member(
        expression,
        result,
        result + 100_000,
        kc::RETURN_PARAMETER_MEMBERSHIP,
    );
    if class == kc::FEATURE {
        f.value(
            result + 100_000,
            kp::RELATIONSHIP_IS_IMPLIED,
            Value::Boolean(true),
        );
        f.value(
            expression,
            kp::ELEMENT_IS_IMPLIED_INCLUDED,
            Value::Boolean(true),
        );
    }
}

fn reference_value(f: &mut Fixture, owner: u128, expression: u128, target: u128, default: bool) {
    f.create(expression, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(owner, expression, expression + 100_000, kc::FEATURE_VALUE);
    f.value(
        expression + 100_000,
        kp::FEATURE_VALUE_IS_DEFAULT,
        Value::Boolean(default),
    );
    reference(f, expression, target, expression + 200_000);
}

#[test]
fn verification_case_objective_subject_and_requirement_chain_close() {
    let (kernel, _, roots) = closed_kernel_anchor_fixture_with(true, true, false, |f, bindings| {
        for (index, role) in [
            StandardRole::Performance,
            StandardRole::Evaluation,
            StandardRole::BooleanEvaluation,
        ]
        .into_iter()
        .enumerate()
        {
            f.relation(
                bindings.get(role).as_u128(),
                bindings.get(StandardRole::Occurrence).as_u128(),
                279_800 + index as u128,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
        }
    });
    let bindings = kernel.context().standard_bindings.as_ref().unwrap();
    let performance = bindings.get(StandardRole::Performance);
    let evaluation = bindings.get(StandardRole::Evaluation);
    let boolean = bindings.get(StandardRole::BooleanEvaluation);
    let (dependency, roots) = expression_dependency_from_kernel_with(
        kernel,
        roots,
        "unusedFunction",
        &[],
        |f, roles, _| {
            for (index, specific, general) in [
                (0, roles[&StandardSysmlRole::Action], performance.as_u128()),
                (
                    1,
                    roles[&StandardSysmlRole::Calculation],
                    roles[&StandardSysmlRole::Action],
                ),
                (
                    2,
                    roles[&StandardSysmlRole::Calculation],
                    evaluation.as_u128(),
                ),
                (
                    3,
                    roles[&StandardSysmlRole::Case],
                    roles[&StandardSysmlRole::Calculation],
                ),
                (
                    4,
                    roles[&StandardSysmlRole::VerificationCase],
                    roles[&StandardSysmlRole::Case],
                ),
                (
                    5,
                    roles[&StandardSysmlRole::RequirementCheck],
                    roles[&StandardSysmlRole::ConstraintCheck],
                ),
                (
                    6,
                    roles[&StandardSysmlRole::ConstraintCheck],
                    boolean.as_u128(),
                ),
            ] {
                f.relation(
                    specific,
                    general,
                    279_810 + index,
                    kc::SUBCLASSIFICATION,
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
            }
            for (index, feature, ty) in [
                (
                    0,
                    StandardSysmlRole::RequirementChecks,
                    StandardSysmlRole::RequirementCheck,
                ),
                (1, StandardSysmlRole::Cases, StandardSysmlRole::Case),
                (
                    2,
                    StandardSysmlRole::VerificationCases,
                    StandardSysmlRole::VerificationCase,
                ),
            ] {
                f.relation(
                    roles[&feature],
                    roles[&ty],
                    279_830 + index,
                    kc::FEATURE_TYPING,
                    kp::FEATURE_TYPING_TYPE,
                );
            }
            f.relation(
                roles[&StandardSysmlRole::VerificationCases],
                roles[&StandardSysmlRole::Cases],
                279_840,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            f.create(79_900, sc::REFERENCE_USAGE, "subj");
            set_enum(f, 79_900, kp::FEATURE_DIRECTION, "in");
            f.member(
                roles[&StandardSysmlRole::RequirementCheck],
                79_900,
                179_900,
                sc::SUBJECT_MEMBERSHIP,
            );
            for (feature, class, name, membership) in [
                (79_901, sc::REFERENCE_USAGE, "subj", sc::SUBJECT_MEMBERSHIP),
                (
                    79_902,
                    sc::REQUIREMENT_USAGE,
                    "obj",
                    sc::OBJECTIVE_MEMBERSHIP,
                ),
                (
                    79_903,
                    sc::REFERENCE_USAGE,
                    "result",
                    kc::RETURN_PARAMETER_MEMBERSHIP,
                ),
            ] {
                f.create(feature, class, name);
                f.member(
                    roles[&StandardSysmlRole::Case],
                    feature,
                    feature + 100_000,
                    membership,
                );
            }
            set_enum(f, 79_901, kp::FEATURE_DIRECTION, "in");
            set_enum(f, 79_903, kp::FEATURE_DIRECTION, "out");
            f.value(79_902, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
            f.relation(
                79_902,
                roles[&StandardSysmlRole::RequirementCheck],
                279_902,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            f.create(79_904, sc::REFERENCE_USAGE, "subj");
            set_enum(f, 79_904, kp::FEATURE_DIRECTION, "in");
            f.member(79_902, 79_904, 179_904, sc::SUBJECT_MEMBERSHIP);
            // Cases::Case::obj::subj defaults to Case::result.
            reference_value(f, 79_904, 79_905, 79_903, true);
            result(f, 79_905, 79_906, sc::REFERENCE_USAGE);
            f.create(79_910, sc::REQUIREMENT_USAGE, "subrequirements");
            f.value(79_910, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
            f.member(
                roles[&StandardSysmlRole::RequirementCheck],
                79_910,
                179_910,
                kc::FEATURE_MEMBERSHIP,
            );
            f.relation(
                79_910,
                roles[&StandardSysmlRole::RequirementChecks],
                279_910,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
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
    f.create(
        79_000,
        sc::VERIFICATION_CASE_DEFINITION,
        "AuthoredVerification",
    );
    for (feature, class, name, membership) in [
        (79_001, sc::REFERENCE_USAGE, "subj", sc::SUBJECT_MEMBERSHIP),
        (
            79_002,
            sc::REFERENCE_USAGE,
            "verdict",
            kc::RETURN_PARAMETER_MEMBERSHIP,
        ),
        (
            79_003,
            sc::REQUIREMENT_USAGE,
            "obj",
            sc::OBJECTIVE_MEMBERSHIP,
        ),
        (
            79_008,
            sc::REQUIREMENT_USAGE,
            "requirementVerifications",
            kc::FEATURE_MEMBERSHIP,
        ),
    ] {
        f.create(feature, class, name);
        f.member(79_000, feature, feature + 100_000, membership);
    }
    set_enum(&mut f, 79_001, kp::FEATURE_DIRECTION, "in");
    set_enum(&mut f, 79_002, kp::FEATURE_DIRECTION, "out");
    f.value(79_003, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    for (specific, general) in [(79_001, 79_901), (79_002, 79_903), (79_003, 79_902)] {
        f.relation(
            specific,
            general,
            specific + 200_000,
            kc::REDEFINITION,
            kp::REDEFINITION_REDEFINED_FEATURE,
        );
    }
    f.create(79_004, sc::REFERENCE_USAGE, "subj");
    set_enum(&mut f, 79_004, kp::FEATURE_DIRECTION, "in");
    f.member(79_003, 79_004, 179_004, sc::SUBJECT_MEMBERSHIP);
    reference_value(&mut f, 79_004, 79_005, 79_001, false);
    result(&mut f, 79_005, 79_006, sc::REFERENCE_USAGE);
    f.create(79_007, sc::REQUIREMENT_USAGE, "requirementVerifications");
    f.value(79_007, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(79_003, 79_007, 179_007, kc::FEATURE_MEMBERSHIP);
    f.relation(
        79_007,
        79_910,
        279_007,
        kc::SUBSETTING,
        kp::SUBSETTING_SUBSETTED_FEATURE,
    );
    // ref requirement requirementVerifications = obj.requirementVerifications.
    f.create(79_009, kc::FEATURE_CHAIN_EXPRESSION, "");
    f.value(
        79_009,
        kp::FEATURE_CHAIN_EXPRESSION_OPERATOR,
        Value::String(".".into()),
    );
    f.member(79_008, 79_009, 179_009, kc::FEATURE_VALUE);
    f.create(79_010, kc::FEATURE, "");
    set_enum(&mut f, 79_010, kp::FEATURE_DIRECTION, "in");
    f.member(79_009, 79_010, 179_010, kc::PARAMETER_MEMBERSHIP);
    reference_value(&mut f, 79_010, 79_011, 79_003, false);
    result(&mut f, 79_011, 79_012, sc::REFERENCE_USAGE);
    result(&mut f, 79_009, 79_013, kc::FEATURE);
    reference(&mut f, 79_009, 79_007, 279_009);
    for subject in [79_005, 79_006, 79_009, 79_010, 79_011, 79_012, 79_013] {
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
    let certificate = closed.certificate.unwrap();
    assert!(certificate.is_fully_closed(closed.overlay.model()));
    for subject in 79_000..=79_013 {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(
                certificate.is_closed(id(subject), requirement),
                "{subject}/{requirement:?}"
            );
        }
    }
    let q = KerMlQueries::new(
        context(&closed.overlay, &dependency, &roots)
            .unwrap()
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap()
            .with_producer_closure(certificate)
            .unwrap(),
    );
    for (feature, expected) in [(79_004, 79_904), (79_003, 79_902)] {
        let answer = q.redefined_features(id(feature));
        assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
        assert!(answer.value.contains(&id(expected)), "{answer:?}");
    }
    for (expression, result) in [(79_005, 79_006), (79_009, 79_013), (79_011, 79_012)] {
        let answer = q.structural_result(id(expression));
        assert_eq!(answer.completeness, Completeness::Complete, "{answer:?}");
        assert_eq!(answer.value, Some(id(result)));
    }
    let reference = q.reference_referent(id(79_009));
    assert_eq!(
        reference.completeness,
        Completeness::Complete,
        "{reference:?}"
    );
    assert_eq!(reference.value, Some(id(79_007)));
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
