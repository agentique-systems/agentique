//! Finalizer APIs over a real combined producer closure, without copied inheritance.
use super::*;

#[test]
fn finalizer_structural_apis_close_cycles_and_inherited_interface_candidates() {
    use agq_kerml_semantics::{PublicationOverlayError, close_result_structure_with_extension};
    let (dependency, _, mut roots) = closed_kernel_anchor_fixture(false, false, false);
    let (mut source, roles) = corpus_anchor_fixture_complete(true);
    source.origin = origin();
    for owner in [550_001, 550_002, 550_003, 550_004] {
        source.create(
            owner,
            sc::ATTRIBUTE_DEFINITION,
            &format!("Attribute{owner}"),
        );
        source.member(1, owner, owner + 100, kc::OWNING_MEMBERSHIP);
    }
    for (a, b, edge) in [
        (550_001, 550_002, 551_001),
        (550_002, 550_001, 551_002),
        (550_003, 550_004, 551_003),
        (550_004, 550_003, 551_004),
    ] {
        source.relation(
            a,
            b,
            edge,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    for owner in [590_001, 590_002, 590_003] {
        source.create(
            owner,
            sc::OCCURRENCE_DEFINITION,
            &format!("ParameterOwner{owner}"),
        );
        source.member(1, owner, owner + 100, kc::OWNING_MEMBERSHIP);
    }
    for (a, b, edge) in [
        (590_001, 590_002, 591_001),
        (590_002, 590_001, 591_002),
        (590_001, 590_003, 591_003),
        (590_002, 590_003, 591_004),
    ] {
        source.relation(
            a,
            b,
            edge,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    let ValueKind::Enumeration(direction) = source
        .base
        .model()
        .registry()
        .property(kp::FEATURE_DIRECTION)
        .unwrap()
        .value_kind
    else {
        unreachable!()
    };
    let input = *source
        .base
        .model()
        .registry()
        .enumeration(direction)
        .unwrap()
        .literals
        .iter()
        .find(|(_, name)| name.as_str() == "in")
        .unwrap()
        .0;
    for feature in [592_001, 592_002] {
        source.create(feature, sc::REFERENCE_USAGE, &format!("parameter{feature}"));
        source.value(feature, kp::FEATURE_DIRECTION, Value::Enumeration(input));
        source.member(590_003, feature, feature + 100, kc::FEATURE_MEMBERSHIP);
    }
    source.create(550_031, sc::ATTRIBUTE_USAGE, "result");
    source.member(550_003, 550_031, 550_131, kc::RETURN_PARAMETER_MEMBERSHIP);
    for (owner, class) in [
        (560_001, sc::CONNECTION_DEFINITION),
        (560_002, sc::CONNECTION_DEFINITION),
        (560_003, sc::INTERFACE_DEFINITION),
        (560_004, sc::INTERFACE_DEFINITION),
    ] {
        source.create(owner, class, &format!("Interface{owner}"));
        source.member(1, owner, owner + 100, kc::OWNING_MEMBERSHIP);
    }
    for (owner, class, ends) in [
        (560_001, kc::FEATURE, [561_001, 561_002]),
        (560_002, sc::REFERENCE_USAGE, [562_001, 562_002]),
        (560_003, sc::PORT_USAGE, [563_001, 563_002]),
    ] {
        for end in ends {
            source.create(end, class, &format!("end{end}"));
            source.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
            source.member(owner, end, end + 100, kc::END_FEATURE_MEMBERSHIP);
            if class == sc::PORT_USAGE {
                source.relation(
                    end,
                    roles[&StandardSysmlRole::Port],
                    end + 200,
                    kc::FEATURE_TYPING,
                    kp::FEATURE_TYPING_TYPE,
                );
            }
        }
    }
    for (a, b, edge) in [
        (563_001, 562_001, 570_001),
        (563_002, 562_002, 570_002),
        (562_001, 561_001, 570_003),
        (562_002, 561_002, 570_004),
    ] {
        source.relation(
            a,
            b,
            edge,
            kc::REDEFINITION,
            kp::REDEFINITION_REDEFINED_FEATURE,
        );
    }
    for (a, b, edge) in [
        (560_002, 560_001, 580_001),
        (560_003, 560_002, 580_002),
        (560_004, 560_001, 580_003),
        (560_004, 560_002, 580_004),
        (560_004, 560_003, 580_005),
    ] {
        source.relation(
            a,
            b,
            edge,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
    }
    // These are independent authored fixture records. The accepted dependency
    // is shared exactly as in production; inherited records are never cloned.
    let source = source.finish();
    let base = dependency.project_snapshot();
    let mut changes = base.change_set();
    for record in source.model().elements() {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        changes.create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            changes.set(record.id(), property, slot.value().clone(), origin.clone());
        }
        if source
            .model()
            .registry()
            .is_subtype(record.metaclass(), kc::MEMBERSHIP)
            .unwrap()
        {
            changes.clear(record.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    let snapshot = base.apply(&changes).unwrap();
    roots.push(id(1));
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
        roots,
    );
    let closed = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            let context = dependency
                .project_overlay_context(overlay, &[id(1)])
                .map_err(PublicationOverlayError::Context)?;
            Ok(crate::context::fixture_overlay_context(
                overlay,
                context,
                SysmlBaselineProfile::OPERATIONAL_V2,
            )
            .unwrap()
            .kerml)
        },
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages.last()
    );
    let context = crate::context::fixture_overlay_context(
        &closed.overlay,
        dependency
            .project_overlay_context(&closed.overlay, &[id(1)])
            .unwrap(),
        SysmlBaselineProfile::OPERATIONAL_V2,
    )
    .unwrap()
    .with_producer_closure(closed.certificate.unwrap())
    .unwrap();
    let q = SysmlQueries::new(context);
    let count = q.model().len();
    for owner in [550_001, 550_002] {
        let answer = q.effective_return_parameters(id(owner));
        assert_eq!(answer.completeness(), Completeness::Complete, "{answer:?}");
        assert!(answer.value().is_empty());
    }
    for owner in [550_003, 550_004] {
        let answer = q.effective_return_parameters(id(owner));
        assert_eq!(answer.completeness(), Completeness::Complete, "{answer:?}");
        assert_eq!(answer.value(), &vec![id(550_031)]);
    }
    let ends = q.effective_interface_ends(id(560_004));
    assert_eq!(ends.completeness(), Completeness::Complete, "{ends:?}");
    assert_eq!(ends.value(), &vec![id(563_001), id(563_002)]);
    for owner in [590_001, 590_002] {
        let parameters = q.effective_parameters(id(owner));
        assert_eq!(
            parameters.completeness(),
            Completeness::Complete,
            "{parameters:?}"
        );
        assert_eq!(parameters.value(), &vec![id(592_001), id(592_002)]);
    }
    assert_eq!(q.model().len(), count);
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
