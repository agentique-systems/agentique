//! Frontend ownership shape of Actions::ForLoopAction `assign var := seq#(index)`.
use super::*;

fn assignment_index_value(index_result_class: MetaclassId) {
    let (kernel, _, roots) =
        closed_kernel_anchor_fixture_with(true, false, false, |f, bindings| {
            // Pinned BaseFunctions::'#' supplies the standard operator signature.
            // Add it before real dependency closure, never to a mounted closed graph.
            f.create(74_100, kc::FUNCTION, "#");
            f.member(
                bindings.get(StandardRole::BaseFunctions).as_u128(),
                74_100,
                174_100,
                kc::OWNING_MEMBERSHIP,
            );
            for (parameter, name, direction, membership) in [
                (74_101, "seq", "in", kc::PARAMETER_MEMBERSHIP),
                (74_102, "index", "in", kc::PARAMETER_MEMBERSHIP),
                (74_103, "result", "out", kc::RETURN_PARAMETER_MEMBERSHIP),
            ] {
                f.create(parameter, kc::FEATURE, name);
                set_enum(f, parameter, kp::FEATURE_DIRECTION, direction);
                f.member(74_100, parameter, parameter + 100_000, membership);
            }
            for membership in 174_100..=174_103 {
                f.changes.clear(id(membership), kp::ELEMENT_DECLARED_NAME);
            }
        });
    let (dependency, roots) = expression_dependency_from_kernel_with(
        kernel,
        roots,
        "isEmpty",
        &["seq"],
        |f, roles, occurrence| {
            let action = roles[&StandardSysmlRole::Action];
            f.relation(
                action,
                occurrence.as_u128(),
                274_110,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            f.relation(
                roles[&StandardSysmlRole::Actions],
                action,
                274_111,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            f.relation(
                roles[&StandardSysmlRole::Subactions],
                roles[&StandardSysmlRole::Actions],
                274_112,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            f.create(74_110, sc::ACTION_DEFINITION, "AssignmentAction");
            f.relation(
                74_110,
                action,
                274_113,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            f.relation(
                roles[&StandardSysmlRole::AssignmentActions],
                74_110,
                274_114,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            f.relation(
                roles[&StandardSysmlRole::Assignments],
                roles[&StandardSysmlRole::AssignmentActions],
                274_115,
                kc::SUBSETTING,
                kp::SUBSETTING_SUBSETTED_FEATURE,
            );
            for (subject, name, direction) in [
                (74_111, "target", "in"),
                (74_112, "replacementValues", "inout"),
            ] {
                f.create(subject, sc::REFERENCE_USAGE, name);
                set_enum(f, subject, kp::FEATURE_DIRECTION, direction);
                f.member(74_110, subject, subject + 100_000, kc::PARAMETER_MEMBERSHIP);
            }
        },
    );
    let occurrence = dependency
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::Occurrence);
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    f.create(74_000, sc::ACTION_DEFINITION, "LoopOwner");
    f.relation(
        74_000,
        occurrence.as_u128(),
        274_000,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    for (feature, class, name, direction) in [
        (74_001, sc::REFERENCE_USAGE, "seq", Some("in")),
        (74_002, sc::ATTRIBUTE_USAGE, "index", None),
        (74_003, sc::REFERENCE_USAGE, "var", None),
    ] {
        f.create(feature, class, name);
        if let Some(direction) = direction {
            set_enum(&mut f, feature, kp::FEATURE_DIRECTION, direction);
        }
        f.member(74_000, feature, feature + 100_000, kc::FEATURE_MEMBERSHIP);
    }
    f.create(74_004, sc::ASSIGNMENT_ACTION_USAGE, "");
    f.value(74_004, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    f.member(74_000, 74_004, 174_004, kc::FEATURE_MEMBERSHIP);
    // AssignmentTargetParameter and NodeParameter both lower to input ReferenceUsage.
    for feature in [74_005, 74_006] {
        f.create(feature, sc::REFERENCE_USAGE, "");
        set_enum(&mut f, feature, kp::FEATURE_DIRECTION, "in");
        f.member(74_004, feature, feature + 100_000, kc::PARAMETER_MEMBERSHIP);
    }
    reference(&mut f, 74_004, 74_003, 174_020);
    f.create(74_007, kc::INDEX_EXPRESSION, "");
    f.value(
        74_007,
        kp::INDEX_EXPRESSION_OPERATOR,
        Value::String("#".into()),
    );
    f.member(74_006, 74_007, 174_007, kc::FEATURE_VALUE);
    // PrimaryArgumentMember -> PrimaryArgument -> FeatureValue -> `seq`.
    f.create(74_008, kc::FEATURE, "");
    set_enum(&mut f, 74_008, kp::FEATURE_DIRECTION, "in");
    f.member(74_007, 74_008, 174_008, kc::PARAMETER_MEMBERSHIP);
    f.create(74_009, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(74_008, 74_009, 174_009, kc::FEATURE_VALUE);
    reference(&mut f, 74_009, 74_001, 174_021);
    // SequenceExpressionListMember directly owns the `index` expression.
    f.create(74_010, kc::FEATURE_REFERENCE_EXPRESSION, "");
    f.member(74_007, 74_010, 174_010, kc::FEATURE_MEMBERSHIP);
    reference(&mut f, 74_010, 74_002, 174_022);
    for (owner, result) in [(74_007, 74_011), (74_009, 74_012), (74_010, 74_013)] {
        f.create(
            result,
            if owner == 74_007 {
                index_result_class
            } else {
                sc::REFERENCE_USAGE
            },
            "",
        );
        set_enum(&mut f, result, kp::FEATURE_DIRECTION, "out");
        f.member(
            owner,
            result,
            result + 100_000,
            kc::RETURN_PARAMETER_MEMBERSHIP,
        );
    }
    if index_result_class == kc::FEATURE {
        // Canonical abbreviated IndexExpression construction inserts a plain
        // Feature result; only the FRE EmptyFeature results are ReferenceUsages.
        f.value(174_011, kp::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true));
        f.value(
            74_007,
            kp::ELEMENT_IS_IMPLIED_INCLUDED,
            Value::Boolean(true),
        );
    }
    for subject in 74_004..=74_013 {
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
    if closed.completeness != Completeness::Complete {
        let registry = ProducerRegistry::new(
            ProducerFamily::ALL
                .into_iter()
                .map(|family| family.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9))
                .chain(sysml_producer_descriptors()),
        )
        .unwrap();
        let certificate = closed.certificate.as_ref().unwrap();
        for record in closed.overlay.model().elements() {
            for (index, descriptor) in registry.descriptors().iter().enumerate() {
                if certificate.evaluation(record.id(), index)
                    == Some(agq_kerml_semantics::ProducerEvaluationState::EvaluatedIncomplete)
                {
                    eprintln!("incomplete {} / {}", record.id(), descriptor.id.name());
                }
            }
        }
    }
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages.last()
    );
    assert!(
        closed
            .certificate
            .unwrap()
            .is_fully_closed(closed.overlay.model())
    );
    for property in [
        agq_sysml::properties::USAGE_MAY_TIME_VARY,
        kp::FEATURE_IS_VARIABLE,
    ] {
        assert!(
            matches!(closed.overlay.model().property_state(id(74_006),property),
                Ok(agq_kernel::derived::PropertyState::Computed(slot)) if slot.value()==&SlotValue::Scalar(Value::Boolean(true))),
            "assignment input must exercise the real variable branch"
        );
    }
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}

#[test]
fn assignment_index_value_with_frontend_result_classes_closes() {
    assignment_index_value(kc::FEATURE);
}

#[test]
fn assignment_index_value_with_explicit_reference_usage_result_closes() {
    assignment_index_value(sc::REFERENCE_USAGE);
}
