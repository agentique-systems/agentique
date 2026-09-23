use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

fn invocation_fixture(function: bool) -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::INVOCATION_EXPRESSION);
    f.create(2, if function { c::FUNCTION } else { c::BEHAVIOR });
    f.create(3, c::FEATURE);
    member(&mut f, 1, 3, 13, c::RETURN_PARAMETER_MEMBERSHIP);
    f.enumeration(3, p::FEATURE_DIRECTION, "out");
    f.create(12, c::MEMBERSHIP);
    f.own(1, 12);
    f.value(12, p::MEMBERSHIP_MEMBER_ELEMENT, Value::Reference(id(2)));

    // Invocation argument -> FeatureValue -> reference expression -> its own result.
    // This follows the ownership distinction of a nested source expression;
    // the inner result is never the outer invocation's structural result.
    f.create(4, c::FEATURE);
    f.enumeration(4, p::FEATURE_DIRECTION, "in");
    member(&mut f, 1, 4, 14, c::PARAMETER_MEMBERSHIP);
    f.create(5, c::FEATURE_REFERENCE_EXPRESSION);
    member(&mut f, 4, 5, 45, c::FEATURE_VALUE);
    f.create(6, c::FEATURE);
    f.enumeration(6, p::FEATURE_DIRECTION, "out");
    member(&mut f, 5, 6, 56, c::RETURN_PARAMETER_MEMBERSHIP);
    f.finish()
}

fn invocation_context(snapshot: &Snapshot) -> SemanticContext<'_> {
    SemanticContext::for_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
}

#[test]
fn pending_invocation_cannot_change_nested_argument_result_typing() {
    let snapshot = invocation_fixture(false);
    let registry = ProducerRegistry::new([
        ProducerFamily::Invocation.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9)
    ])
    .unwrap();
    let context = invocation_context(&snapshot)
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let q = KerMlQueries::new(context.fork());
    assert_eq!(q.structural_result(id(1)).value, Some(id(3)));
    assert_eq!(q.structural_result(id(5)).value, Some(id(6)));
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(!certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    assert!(!certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
    assert!(
        certificate.is_closed(id(6), SemanticClosureRequirement::EffectiveTyping),
        "an enclosing invocation cannot type its nested argument expression's return"
    );
}

#[test]
fn invocation_planner_writes_only_its_subject_and_direct_behavior_result() {
    for function in [false, true] {
        let snapshot = invocation_fixture(function);
        let q = KerMlQueries::new(invocation_context(&snapshot));
        let plan = q.plan_result_structure([id(1)]);
        assert!(plan.producer_evaluations.contains(&(
            id(1),
            ProducerFamily::Invocation.id(),
            Completeness::Complete
        )));
        let registry = ProducerRegistry::new([
            ProducerFamily::Invocation.descriptor(agq_kerml::BaselineProfile::OPERATIONAL_V9)
        ])
        .unwrap();
        plan.validate_declared_effects(&[id(1)], &registry).unwrap();
        let output = plan.materialize(&snapshot).unwrap();
        let model = output.overlay.model();
        let specialized: BTreeSet<_> = model
            .elements()
            .filter(|record| {
                matches!(record.origin(), Origin::Derived(_))
                    && model
                        .registry()
                        .is_subtype(record.metaclass(), c::SPECIALIZATION)
                        .unwrap()
            })
            .filter_map(|record| {
                let specific = model
                    .registry()
                    .resolve_property(record.metaclass(), p::SPECIALIZATION_SPECIFIC)
                    .unwrap()
                    .unwrap()
                    .id;
                model
                    .navigation_slot(record.id(), specific)
                    .and_then(|slot| {
                        slot.value().values().find_map(|value| match value {
                            Value::Reference(target) => Some(*target),
                            _ => None,
                        })
                    })
            })
            .collect();
        assert_eq!(
            specialized,
            if function {
                BTreeSet::from([id(1)])
            } else {
                BTreeSet::from([id(1), id(3)])
            }
        );
        assert_eq!(output.production.value.is_empty(), function);
        assert!(!specialized.contains(&id(6)));
    }
}
