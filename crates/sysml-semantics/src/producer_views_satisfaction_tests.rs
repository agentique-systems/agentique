//! Bounded Views::View requirement satisfaction, with the actual frontend shape.
use super::*;
use agq_sysml::properties as sp;

#[test]
fn view_requirement_satisfaction_subject_value_and_required_constraint_close() {
    let (kernel, _, roots) = closed_kernel_anchor_fixture_with(true, true, false, |f, bindings| {
        f.relation(
            bindings.get(StandardRole::BooleanEvaluation).as_u128(),
            bindings.get(StandardRole::Occurrence).as_u128(),
            278_800,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    });
    let boolean = kernel
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::BooleanEvaluation);
    let (dependency, roots) = expression_dependency_from_kernel_with(
        kernel,
        roots,
        "fixtureFunction",
        &[],
        |f, roles, _| {
            for (index, specific, general) in [
                (0, StandardSysmlRole::View, StandardSysmlRole::Part),
                (1, StandardSysmlRole::Part, StandardSysmlRole::Item),
                (
                    2,
                    StandardSysmlRole::ViewpointCheck,
                    StandardSysmlRole::RequirementCheck,
                ),
                (
                    3,
                    StandardSysmlRole::RequirementCheck,
                    StandardSysmlRole::ConstraintCheck,
                ),
            ] {
                f.relation(
                    roles[&specific],
                    roles[&general],
                    278_810 + index,
                    kc::SUBCLASSIFICATION,
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
            }
            f.relation(
                roles[&StandardSysmlRole::ConstraintCheck],
                boolean.as_u128(),
                278_820,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            for (index, feature, ty) in [
                (
                    0,
                    StandardSysmlRole::ConstraintChecks,
                    StandardSysmlRole::ConstraintCheck,
                ),
                (
                    1,
                    StandardSysmlRole::RequirementChecks,
                    StandardSysmlRole::RequirementCheck,
                ),
                (
                    2,
                    StandardSysmlRole::ViewpointChecks,
                    StandardSysmlRole::ViewpointCheck,
                ),
            ] {
                f.relation(
                    roles[&feature],
                    roles[&ty],
                    278_830 + index,
                    kc::FEATURE_TYPING,
                    kp::FEATURE_TYPING_TYPE,
                );
            }
            f.relation(
                roles[&StandardSysmlRole::RequirementChecks],
                roles[&StandardSysmlRole::ConstraintChecks],
                278_840,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            f.relation(
                roles[&StandardSysmlRole::ViewpointChecks],
                roles[&StandardSysmlRole::RequirementChecks],
                278_841,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            f.create(78_900, sc::REFERENCE_USAGE, "subj");
            set_enum(f, 78_900, kp::FEATURE_DIRECTION, "in");
            f.member(
                roles[&StandardSysmlRole::RequirementCheck],
                78_900,
                178_900,
                sc::SUBJECT_MEMBERSHIP,
            );
        },
    );
    let that = dependency
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::ThingsThat);
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(78_000, sc::VIEW_DEFINITION, "AuthoredView");
    f.create(78_001, sc::VIEWPOINT_USAGE, "viewpointSatisfactions");
    f.member(78_000, 78_001, 178_001, kc::FEATURE_MEMBERSHIP);
    f.value(78_001, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.create(
        78_002,
        sc::SATISFY_REQUIREMENT_USAGE,
        "viewpointConformance",
    );
    f.member(78_000, 78_002, 178_002, kc::FEATURE_MEMBERSHIP);
    f.value(78_002, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.value(78_002, kp::INVARIANT_IS_NEGATED, Value::Boolean(false));
    f.create(78_003, sc::REFERENCE_USAGE, "");
    f.member(78_002, 78_003, 178_003, sc::SUBJECT_MEMBERSHIP);
    set_enum(&mut f, 78_003, kp::FEATURE_DIRECTION, "in");
    f.create(78_004, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(78_003, 78_004, 178_004, kc::FEATURE_VALUE);
    reference(&mut f, 78_004, that.as_u128(), 178_005);
    f.create(78_005, kc::FEATURE, "");
    set_enum(&mut f, 78_005, kp::FEATURE_DIRECTION, "out");
    f.member(78_004, 78_005, 178_006, kc::RETURN_PARAMETER_MEMBERSHIP);
    f.value(178_006, kp::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true));
    f.value(
        78_004,
        kp::ELEMENT_IS_IMPLIED_INCLUDED,
        Value::Boolean(true),
    );
    f.create(78_006, sc::CONSTRAINT_USAGE, "");
    f.member(
        78_002,
        78_006,
        178_007,
        sc::REQUIREMENT_CONSTRAINT_MEMBERSHIP,
    );
    set_enum(
        &mut f,
        178_007,
        sp::REQUIREMENT_CONSTRAINT_MEMBERSHIP_KIND,
        "requirement",
    );
    f.value(78_006, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.create(278_006, kc::REFERENCE_SUBSETTING, "");
    f.owned
        .entry(id(78_006))
        .or_default()
        .push(Value::Reference(id(278_006)));
    f.value(
        278_006,
        kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
        Value::Reference(id(78_001)),
    );
    for element in 78_003..=78_006 {
        f.changes.clear(id(element), kp::ELEMENT_DECLARED_NAME);
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
    for subject in 78_000..=78_006 {
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
    assert_eq!(q.feature_with_value(id(178_004)).value, Some(id(78_003)));
    assert_eq!(q.structural_result(id(78_004)).value, Some(id(78_005)));
    assert_eq!(q.owning_type(id(78_003)).value, Some(id(78_002)));
    assert!(q.redefined_features(id(78_003)).value.contains(&id(78_900)));
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
