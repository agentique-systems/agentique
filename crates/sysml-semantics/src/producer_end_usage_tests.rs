//! Connection ends combine positional typing, cross selection and variable snapshots.
use super::*;
#[path = "producer_item_end_usage_tests.rs"]
mod item_end_usage_tests;
use agq_kerml_semantics::{
    ProducerClosedDependency, PublicationOverlayError, SemanticClosureRequirement,
    close_result_structure_with_extension,
};

#[test]
fn connection_end_usages_close_with_redefinition_and_variable_snapshots() {
    connection_end_fixture(|_, _, _| {}, &[]);
}

#[test]
fn anonymous_connection_end_with_owned_cross_multiplicity_closes() {
    connection_end_fixture(
        |f, occurrence, _| {
            f.create(76_000, sc::OCCURRENCE_DEFINITION, "Container");
            f.member(1, 76_000, 176_000, kc::OWNING_MEMBERSHIP);
            f.relation(
                76_000,
                occurrence.as_u128(),
                276_000,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
            for (n, name) in [(76_001, "sourceEvent"), (76_002, "source")] {
                f.create(n, sc::OCCURRENCE_USAGE, name);
                f.member(76_000, n, n + 100_000, kc::FEATURE_MEMBERSHIP);
                f.relation(
                    n,
                    occurrence.as_u128(),
                    n + 200_000,
                    kc::FEATURE_TYPING,
                    kp::FEATURE_TYPING_TYPE,
                );
            }
            f.create(76_010, sc::CONNECTION_USAGE, "");
            f.member(76_000, 76_010, 176_010, kc::FEATURE_MEMBERSHIP);
            f.value(76_010, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
            f.relation(
                76_010,
                75_000,
                276_010,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
            for (end, target) in [(76_011, 76_001), (76_012, 76_002)] {
                f.create(end, sc::REFERENCE_USAGE, "");
                f.member(76_010, end, end + 100_000, kc::END_FEATURE_MEMBERSHIP);
                f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
                let relation = end + 200_000;
                f.create(relation, kc::REFERENCE_SUBSETTING, "");
                f.owned
                    .entry(id(end))
                    .or_default()
                    .push(Value::Reference(id(relation)));
                f.value(
                    relation,
                    kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
                    Value::Reference(id(target)),
                );
            }
            // Operational grammar lowers `[1] source` to a plain owned cross
            // Feature containing the multiplicity, followed by ReferenceSubsetting.
            f.create(76_013, kc::FEATURE, "");
            f.member(76_012, 76_013, 176_013, kc::OWNING_MEMBERSHIP);
            f.owned.get_mut(&id(76_012)).unwrap().swap(0, 1);
            f.create(76_014, kc::MULTIPLICITY_RANGE, "");
            f.member(76_013, 76_014, 176_014, kc::OWNING_MEMBERSHIP);
            f.changes
                .create(id(76_015), kc::LITERAL_INTEGER, f.origin.clone());
            for property in f
                .base
                .model()
                .registry()
                .effective_properties(kc::LITERAL_INTEGER)
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
                    id(76_015),
                    property.id,
                    SlotValue::Scalar(value),
                    f.origin.clone(),
                );
            }
            f.member(76_014, 76_015, 176_015, kc::OWNING_MEMBERSHIP);
            f.create(76_016, kc::FEATURE, "");
            set_enum(f, 76_016, kp::FEATURE_DIRECTION, "out");
            f.member(76_015, 76_016, 176_016, kc::RETURN_PARAMETER_MEMBERSHIP);
            f.value(176_016, kp::RELATIONSHIP_IS_IMPLIED, Value::Boolean(true));
            f.value(
                76_015,
                kp::ELEMENT_IS_IMPLIED_INCLUDED,
                Value::Boolean(true),
            );
            for n in [76_010, 76_011, 76_012, 76_013, 76_014, 76_016] {
                f.changes.clear(id(n), kp::ELEMENT_DECLARED_NAME);
            }
        },
        &[(76_011, None), (76_012, Some(76_013))],
    );
}

fn connection_end_fixture(
    customize: impl FnOnce(&mut Fixture, ElementId, ElementId),
    additional_ends: &[(u128, Option<u128>)],
) {
    let (dependency, _, mut roots) = closed_kernel_anchor_fixture(true, false, false);
    let standard = dependency.context().standard_bindings.as_ref().unwrap();
    let occurrence = standard.get(StandardRole::Occurrence);
    let anything = standard.get(StandardRole::Anything);
    let (source, _) = corpus_anchor_fixture_complete(true);
    let source = source.finish();
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    for record in source.model().elements() {
        let agq_kernel::provenance::Origin::Declared(origin) = record.origin() else {
            unreachable!()
        };
        f.changes
            .create(record.id(), record.metaclass(), origin.clone());
        for (property, slot) in record.slots() {
            f.changes
                .set(record.id(), property, slot.value().clone(), origin.clone());
            if property == kp::ELEMENT_OWNED_RELATIONSHIP {
                let SlotValue::Ordered(values) = slot.value() else {
                    unreachable!()
                };
                f.owned.insert(record.id(), values.clone());
            }
        }
    }
    roots.push(id(1));
    for (connection, name) in [(75_000, "ConnectionBase"), (75_010, "AuthoredConnection")] {
        f.create(connection, sc::CONNECTION_DEFINITION, name);
        f.member(1, connection, connection + 100_000, kc::OWNING_MEMBERSHIP);
        f.relation(
            connection,
            occurrence.as_u128(),
            connection + 200_000,
            kc::SUBCLASSIFICATION,
            kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
        );
        for (offset, name, class) in [
            (1, "source", sc::REFERENCE_USAGE),
            (2, "target", sc::OCCURRENCE_USAGE),
        ] {
            let end = connection + offset;
            f.create(end, class, name);
            f.member(connection, end, end + 100_000, kc::FEATURE_MEMBERSHIP);
            f.value(end, kp::FEATURE_IS_END, Value::Boolean(true));
            f.relation(
                end,
                anything.as_u128(),
                end + 200_000,
                kc::FEATURE_TYPING,
                kp::FEATURE_TYPING_TYPE,
            );
        }
    }
    f.relation(
        75_010,
        75_000,
        275_020,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    // The second local end has an explicit redefinition; the first also exercises
    // the positional producer's corresponding inherited-end path.
    f.relation(
        75_012,
        75_002,
        275_021,
        kc::REDEFINITION,
        kp::REDEFINITION_REDEFINED_FEATURE,
    );
    customize(&mut f, occurrence, anything);
    let snapshot = f.finish();
    let mut names = snapshot.change_set();
    for record in snapshot.model().elements() {
        if !snapshot.is_dependency_element(record.id())
            && snapshot
                .model()
                .registry()
                .is_subtype(record.metaclass(), kc::MEMBERSHIP)
                .unwrap()
        {
            names.clear(record.id(), kp::ELEMENT_DECLARED_NAME);
        }
    }
    let snapshot = snapshot.apply(&names).unwrap();
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32]));
    let extension = SysmlProducerExtension::new(
        SysmlBaselineProfile::OPERATIONAL_V2,
        bindings.clone(),
        roots.clone(),
    );
    fn context<'m>(
        overlay: &'m agq_kernel::derived::DerivedOverlay,
        dependency: &Arc<ProducerClosedDependency>,
        roots: &[ElementId],
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        let context = dependency
            .project_overlay_context(overlay, roots)
            .map_err(PublicationOverlayError::Context)?;
        Ok(crate::context::fixture_overlay_context(
            overlay,
            context,
            SysmlBaselineProfile::OPERATIONAL_V2,
        )
        .unwrap()
        .kerml)
    }
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
    let queries = KerMlQueries::new(
        context(&closed.overlay, &dependency, &roots)
            .unwrap()
            .with_producer_registry_digest(certificate.registry_digest())
            .unwrap()
            .with_producer_closure(certificate.clone())
            .unwrap(),
    );
    for (subject, expected_cross) in [75_001, 75_002, 75_011, 75_012]
        .into_iter()
        .map(|id| (id, None))
        .chain(additional_ends.iter().copied())
    {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(
                certificate.is_closed(id(subject), requirement),
                "{subject}/{requirement:?}"
            );
        }
        let varies = current_usage_may_time_vary(
            &queries,
            SysmlBaselineProfile::OPERATIONAL_V2,
            &bindings,
            &roots,
            id(subject),
        );
        assert_eq!(varies.completeness, Completeness::Complete, "{varies:?}");
        assert_eq!(varies.value, Some(true));
        let agq_kernel::derived::PropertyState::Computed(variable) = closed
            .overlay
            .model()
            .property_state(id(subject), kp::FEATURE_IS_VARIABLE)
            .unwrap()
        else {
            panic!("{subject}: variable value must be computed")
        };
        assert_eq!(variable.value(), &SlotValue::Scalar(Value::Boolean(true)));
        let cross = queries.owned_cross_feature(id(subject));
        assert_eq!(cross.completeness, Completeness::Complete);
        assert_eq!(cross.value, expected_cross.map(id));
    }
    for (subject, inherited) in [(75_011, 75_001), (75_012, 75_002)] {
        let redefined = queries.redefined_features(id(subject));
        assert_eq!(redefined.completeness, Completeness::Complete);
        assert!(redefined.value.contains(&id(inherited)), "{redefined:?}");
    }
    assert!(Arc::ptr_eq(
        closed.overlay.declared().immutable_dependency().unwrap(),
        dependency.overlay()
    ));
}
