use super::*;
use agq_sysml::properties as sp;

const RULE: &str = "checkUsageVariationDefinitionSpecialization";

fn color(f: &mut Fixture) {
    f.origin = origin();
    f.create(500_000, sc::ENUMERATION_DEFINITION, "Color");
    f.value(500_000, kp::TYPE_IS_ABSTRACT, Value::Boolean(true));
    f.value(
        500_000,
        sp::ENUMERATION_DEFINITION_IS_VARIATION,
        Value::Boolean(true),
    );
    for (n, name) in [(500_001, "red"), (500_002, "green")] {
        f.create(n, sc::ENUMERATION_USAGE, name);
        f.member(500_000, n, n + 100, sc::VARIANT_MEMBERSHIP);
        f.changes.clear(id(n + 100), kp::ELEMENT_DECLARED_NAME);
    }
}

#[test]
fn enumeration_variants_propose_typing_with_owner_and_membership_evidence() {
    let mut f = Fixture::new();
    color(&mut f);
    f.create(500_003, sc::ENUMERATION_USAGE, "ordinaryOwnedUsage");
    f.member(500_000, 500_003, 500_103, kc::FEATURE_MEMBERSHIP);
    f.create(500_004, sc::ATTRIBUTE_USAGE, "ordinaryVariant");
    f.member(500_000, 500_004, 500_104, sc::VARIANT_MEMBERSHIP);
    f.create(500_005, sc::ATTRIBUTE_DEFINITION, "OrdinaryDefinition");
    f.create(500_006, sc::ATTRIBUTE_USAGE, "notAVariation");
    f.member(500_005, 500_006, 500_106, sc::VARIANT_MEMBERSHIP);
    let snapshot = f.finish();
    let queries = q(&snapshot);
    for n in [500_001, 500_002, 500_004] {
        let plan = queries.producer_plan(&[], id(n));
        let answer = result(&plan, RULE);
        assert_eq!(answer.evidence.completeness, Completeness::Complete);
        assert_eq!(answer.relationships.len(), 1);
        let edge = &answer.relationships[0];
        assert_eq!(edge.metaclass, kc::FEATURE_TYPING);
        assert_eq!(edge.specific, id(n));
        assert_eq!(edge.general, id(500_000));
        for input in [id(n), id(n + 100), id(500_000)] {
            assert!(
                answer
                    .evidence
                    .positive_dependencies
                    .contains(&agq_kernel::provenance::FactKey::Element(input))
            );
        }
    }
    for n in [500_003, 500_006] {
        let plan = queries.producer_plan(&[], id(n));
        let answer = result(&plan, RULE);
        assert_eq!(answer.evidence.completeness, Completeness::Complete);
        assert!(answer.relationships.is_empty());
    }
    let descriptor = sysml_producer_descriptors()
        .into_iter()
        .find(|d| d.id == agq_kerml_semantics::ProducerFamilyId::new(RULE))
        .unwrap();
    assert!(
        descriptor
            .effects
            .contains(&agq_kerml_semantics::ProducerEffect::Typing)
    );
    assert_eq!(
        descriptor.relationship_classes,
        Some(BTreeSet::from([kc::FEATURE_TYPING]))
    );
    assert!(
        sysml_producer_rule_ids(SysmlBaselineProfile::OPERATIONAL_V2)
            .contains(&SysmlBaselineProfile::OPERATIONAL_V2.rule_id(RULE))
    );
}

#[test]
fn enumeration_variation_specialization_does_not_duplicate_indirect_typing() {
    let mut f = Fixture::new();
    color(&mut f);
    f.create(500_010, sc::ATTRIBUTE_USAGE, "intermediate");
    f.relation(
        500_001,
        500_010,
        500_201,
        kc::SUBSETTING,
        kp::SUBSETTING_SUBSETTED_FEATURE,
    );
    f.relation(
        500_010,
        500_000,
        500_202,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    f.relation(
        500_002,
        500_000,
        500_203,
        kc::FEATURE_TYPING,
        kp::FEATURE_TYPING_TYPE,
    );
    let snapshot = f.finish();
    let queries = q(&snapshot);
    for n in [500_001, 500_002] {
        let plan = queries.producer_plan(&[], id(n));
        let answer = result(&plan, RULE);
        assert_eq!(answer.evidence.completeness, Completeness::Complete);
        assert!(answer.relationships.is_empty());
        assert_eq!(queries.current_usage_types(id(n)).value(), &[id(500_000)]);
    }
}

#[test]
fn enumeration_variants_close_through_real_scheduler_and_effective_queries() {
    use agq_kerml_semantics::{PublicationOverlayError, close_result_structure_with_extension};
    let (dependency, _, roots) = closed_kernel_anchor_fixture(false, false, false);
    let base = dependency.project_snapshot();
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    color(&mut f);
    let snapshot = f.finish();
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
                .project_overlay_context(overlay, &[id(500_000)])
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
            .project_overlay_context(&closed.overlay, &[id(500_000)])
            .unwrap(),
        SysmlBaselineProfile::OPERATIONAL_V2,
    )
    .unwrap()
    .with_producer_closure(closed.certificate.unwrap())
    .unwrap();
    let queries = SysmlQueries::new(context);
    let before = closed.overlay.model().len();
    let data_values = dependency
        .context()
        .standard_bindings
        .as_ref()
        .unwrap()
        .get(StandardRole::DataValues);
    for (n, name) in [(500_001, "red"), (500_002, "green")] {
        let types = queries.effective_usage_types(id(n));
        assert_eq!(types.completeness(), Completeness::Complete, "{types:?}");
        assert_eq!(types.value(), &[id(500_000)]);
        let names = queries.effective_names(id(n));
        assert_eq!(names.completeness(), Completeness::Complete, "{names:?}");
        assert!(format!("{:?}", names.value()).contains(name));
        assert!(
            queries
                .kerml()
                .all_supertypes(id(n))
                .value
                .contains(&data_values)
        );
        assert_eq!(
            queries.kerml().owning_relationship(id(n)).value,
            Some(id(n + 100))
        );
        let plan = queries.producer_plan(&[], id(n));
        assert!(result(&plan, RULE).relationships.is_empty());
    }
    assert_eq!(closed.overlay.model().len(), before);
}
