//! Interfaces::Interface::participant::otherParticipants, with a closed callee.
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
    f.changes.clear(id(feature), kp::ELEMENT_DECLARED_NAME);
}

fn multiplicity(f: &mut Fixture, owner: u128, base: u128, lower: i64, unlimited: bool) {
    f.create(base, kc::MULTIPLICITY_RANGE, "");
    f.member(owner, base, base + 100_000, kc::OWNING_MEMBERSHIP);
    for (literal, class) in std::iter::once((base + 1, kc::LITERAL_INTEGER))
        .chain(unlimited.then_some((base + 2, kc::LITERAL_INFINITY)))
    {
        f.changes.create(id(literal), class, f.origin.clone());
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
                ValueKind::Integer => Value::Integer(lower.into()),
                ValueKind::Reference(_) => continue,
                other => panic!("literal fixture domain {other:?}"),
            };
            f.changes.set(
                id(literal),
                property.id,
                SlotValue::Scalar(value),
                f.origin.clone(),
            );
        }
        f.member(base, literal, literal + 100_000, kc::OWNING_MEMBERSHIP);
        result(f, literal, literal + 10, kc::FEATURE, true);
    }
    f.changes.clear(id(base), kp::ELEMENT_DECLARED_NAME);
}

#[test]
fn interface_other_participants_default_invocation_with_closed_callee_closes() {
    let (kernel, _, roots) = closed_kernel_anchor_fixture_with(true, true, false, |f, bindings| {
        f.relation(
            bindings.get(StandardRole::Object).as_u128(),
            bindings.get(StandardRole::Occurrence).as_u128(),
            282_900,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
        f.create(82_900, kc::FEATURE, "participant");
        f.member(
            bindings.get(StandardRole::Link).as_u128(),
            82_900,
            182_900,
            kc::FEATURE_MEMBERSHIP,
        );
        f.relation(
            82_900,
            bindings.get(StandardRole::Anything).as_u128(),
            282_901,
            kc::FEATURE_TYPING,
            kp::FEATURE_TYPING_TYPE,
        );
        f.value(82_900, kp::FEATURE_IS_ORDERED, Value::Boolean(true));
        f.value(82_900, kp::FEATURE_IS_UNIQUE, Value::Boolean(false));
        multiplicity(f, 82_900, 83_900, 2, true);
    });
    let bindings = kernel.context().standard_bindings.as_ref().unwrap();
    let object = bindings.get(StandardRole::Object);
    let link = bindings.get(StandardRole::Link);
    let occurrence = bindings.get(StandardRole::Occurrence);
    let (dependency, roots) = expression_dependency_from_kernel_with(
        kernel,
        roots,
        "unusedFunction",
        &[],
        |f, roles, _| {
            for (index, specific, general) in [
                (0, roles[&StandardSysmlRole::Port], object.as_u128()),
                (
                    1,
                    roles[&StandardSysmlRole::Part],
                    roles[&StandardSysmlRole::Item],
                ),
                (
                    2,
                    roles[&StandardSysmlRole::Connection],
                    roles[&StandardSysmlRole::Part],
                ),
                (3, roles[&StandardSysmlRole::Connection], link.as_u128()),
                (
                    4,
                    roles[&StandardSysmlRole::Interface],
                    roles[&StandardSysmlRole::Connection],
                ),
                (
                    5,
                    roles[&StandardSysmlRole::Calculation],
                    roles[&StandardSysmlRole::Action],
                ),
                (6, roles[&StandardSysmlRole::Action], occurrence.as_u128()),
            ] {
                f.relation(
                    specific,
                    general,
                    282_920 + index,
                    kc::SUBCLASSIFICATION,
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
                );
            }
            f.relation(
                roles[&StandardSysmlRole::Ports],
                roles[&StandardSysmlRole::Port],
                282_930,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            f.create(82_901, sc::REFERENCE_USAGE, "self");
            f.member(
                roles[&StandardSysmlRole::Port],
                82_901,
                182_901,
                kc::FEATURE_MEMBERSHIP,
            );
            f.relation(
                82_901,
                roles[&StandardSysmlRole::Port],
                282_931,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            f.create(82_902, sc::PORT_USAGE, "interfacingPorts");
            f.member(
                roles[&StandardSysmlRole::Port],
                82_902,
                182_902,
                kc::FEATURE_MEMBERSHIP,
            );
            f.relation(
                82_902,
                roles[&StandardSysmlRole::Ports],
                282_932,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            f.relation(
                82_902,
                roles[&StandardSysmlRole::Port],
                282_933,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            f.value(82_902, kp::FEATURE_IS_UNIQUE, Value::Boolean(false));
            multiplicity(f, 82_902, 83_920, 0, true);
            // The body is independently tested. This genuine closed signature
            // retains Calculation ancestry, parameter directions and value :> seq.
            f.create(82_910, sc::CALCULATION_DEFINITION, "excludingOnce");
            f.member(1, 82_910, 182_910, kc::OWNING_MEMBERSHIP);
            for (feature, name) in [(82_911, "seq"), (82_912, "value")] {
                f.create(feature, sc::REFERENCE_USAGE, name);
                set_enum(f, feature, kp::FEATURE_DIRECTION, "in");
                f.member(82_910, feature, feature + 100_000, kc::FEATURE_MEMBERSHIP);
            }
            f.value(82_911, kp::FEATURE_IS_ORDERED, Value::Boolean(true));
            f.value(82_911, kp::FEATURE_IS_UNIQUE, Value::Boolean(false));
            f.relation(
                82_912,
                82_911,
                282_912,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            multiplicity(f, 82_911, 83_940, 1, true);
            multiplicity(f, 82_912, 83_960, 1, false);
            result(f, 82_910, 82_913, sc::REFERENCE_USAGE, false);
        },
    );
    let (_, roles) = corpus_anchor_fixture_complete(true);
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(82_000, sc::INTERFACE_DEFINITION, "AuthoredInterface");
    f.create(82_001, sc::PORT_USAGE, "");
    f.member(82_000, 82_001, 182_001, kc::FEATURE_MEMBERSHIP);
    f.relation(
        82_001,
        82_900,
        282_001,
        kc::REDEFINITION,
        kp::REDEFINITION_REDEFINED_FEATURE,
    );
    f.relation(
        82_001,
        roles[&StandardSysmlRole::Port],
        282_002,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    f.value(82_001, kp::FEATURE_IS_ORDERED, Value::Boolean(true));
    f.value(82_001, kp::FEATURE_IS_UNIQUE, Value::Boolean(false));
    multiplicity(&mut f, 82_001, 83_000, 2, true);
    for (feature, name) in [(82_002, "thisParticipant"), (82_003, "otherParticipants")] {
        f.create(feature, sc::REFERENCE_USAGE, name);
        f.member(82_001, feature, feature + 100_000, kc::FEATURE_MEMBERSHIP);
        set_enum(
            &mut f,
            feature + 100_000,
            kp::MEMBERSHIP_VISIBILITY,
            "protected",
        );
    }
    f.relation(
        82_002,
        82_901,
        282_003,
        kc::REDEFINITION,
        kp::REDEFINITION_REDEFINED_FEATURE,
    );
    f.relation(
        82_003,
        82_902,
        282_004,
        kc::SUBSETTING,
        kp::SUBSETTING_SUBSETTED_FEATURE,
    );
    f.relation(
        82_003,
        roles[&StandardSysmlRole::Port],
        282_005,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    f.value(82_003, kp::FEATURE_IS_UNIQUE, Value::Boolean(false));
    multiplicity(&mut f, 82_003, 83_020, 1, true);
    f.create(82_004, kc::INVOCATION_EXPRESSION, "");
    f.member(82_003, 82_004, 182_004, kc::FEATURE_VALUE);
    f.value(182_004, kp::FEATURE_VALUE_IS_DEFAULT, Value::Boolean(true));
    reference(&mut f, 82_004, 82_910, 284_004);
    for (input, expression, referent, result_id) in [
        (82_005, 82_006, 82_001, 82_007),
        (82_008, 82_009, 82_002, 82_010),
    ] {
        f.create(input, kc::FEATURE, "");
        set_enum(&mut f, input, kp::FEATURE_DIRECTION, "in");
        f.member(82_004, input, input + 100_000, kc::PARAMETER_MEMBERSHIP);
        f.create(expression, kc::FEATURE_REFERENCE_EXPRESSION, "");
        f.member(input, expression, expression + 100_000, kc::FEATURE_VALUE);
        reference(&mut f, expression, referent, expression + 200_000);
        result(&mut f, expression, result_id, sc::REFERENCE_USAGE, false);
    }
    result(&mut f, 82_004, 82_011, sc::REFERENCE_USAGE, false);
    for subject in [
        82_001, 82_004, 82_005, 82_006, 82_007, 82_008, 82_009, 82_010, 82_011,
    ] {
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
    assert!(closed.converged, "{:?}", closed.stages.last());
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages.last()
    );
    let certificate = closed.certificate.unwrap();
    assert!(certificate.is_fully_closed(closed.overlay.model()));
    for subject in 82_000..=82_011 {
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
    for subject in [82_001, 82_002, 82_003] {
        let cross = q.cross_feature(id(subject));
        assert_eq!(cross.completeness, Completeness::Complete, "{cross:?}");
        assert_eq!(cross.value, None, "{cross:?}");
    }
    for (subject, expected) in [(82_006, 82_001), (82_009, 82_002)] {
        let referent = q.reference_referent(id(subject));
        assert_eq!(
            referent.completeness,
            Completeness::Complete,
            "{referent:?}"
        );
        assert_eq!(referent.value, Some(id(expected)));
    }
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
