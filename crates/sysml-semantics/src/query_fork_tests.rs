use super::*;
use agq_kerml_semantics::{
    PublicationOverlayError, QueryResult, close_result_structure_with_extension,
};
use std::fmt::Debug;

fn same_kernel_answer<T: Debug + Eq>(left: &QueryResult<T>, right: &QueryResult<T>) {
    assert_eq!(left.context, right.context);
    assert_eq!(left.value, right.value);
    assert_eq!(left.completeness, right.completeness);
    assert_eq!(left.diagnostics, right.diagnostics);
    assert_eq!(left.positive_dependencies, right.positive_dependencies);
    assert_eq!(left.search_dependencies, right.search_dependencies);
    assert_eq!(left.explanations, right.explanations);
    assert_eq!(left.fact_origins, right.fact_origins);
    assert_eq!(left.declared_fact_origins, right.declared_fact_origins);
    assert_eq!(left.canonical_dependencies, right.canonical_dependencies);
}

fn same_answer<T: Debug + Eq>(left: &SysmlQueryResult<T>, right: &SysmlQueryResult<T>) {
    assert_eq!(left.context, right.context);
    assert_eq!(left.completeness(), right.completeness());
    same_kernel_answer(&left.kerml, &right.kerml);
    assert_eq!(left.pending, right.pending);
    assert_eq!(left.diagnostics, right.diagnostics);
    assert_eq!(left.rejected_targets, right.rejected_targets);
    assert_eq!(left.filtered_targets, right.filtered_targets);
    assert_eq!(
        left.supporting_queries.len(),
        right.supporting_queries.len()
    );
    for (left, right) in left
        .supporting_queries
        .iter()
        .zip(&right.supporting_queries)
    {
        same_kernel_answer(left, right);
    }
    assert_eq!(left.supporting_names.len(), right.supporting_names.len());
    for (left, right) in left.supporting_names.iter().zip(&right.supporting_names) {
        same_kernel_answer(left, right);
    }
    assert_eq!(
        left.observations.keys().collect::<Vec<_>>(),
        right.observations.keys().collect::<Vec<_>>()
    );
    for (fact, left) in &left.observations {
        same_kernel_answer(left, &right.observations[fact]);
    }
}

#[test]
fn query_fork_preserves_closed_and_pending_answers_with_independent_evaluators() {
    let (dependency, _, roots) = closed_kernel_anchor_fixture(false, false, false);
    let base = dependency.project_snapshot();
    let mut fixture = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
        origin: origin(),
    };
    fixture.create(500_000, sc::ENUMERATION_DEFINITION, "Palette");
    fixture.value(500_000, kp::TYPE_IS_ABSTRACT, Value::Boolean(true));
    fixture.value(
        500_000,
        agq_sysml::properties::ENUMERATION_DEFINITION_IS_VARIATION,
        Value::Boolean(true),
    );
    fixture.create(500_001, sc::ENUMERATION_USAGE, "amber");
    fixture.member(500_000, 500_001, 500_101, sc::VARIANT_MEMBERSHIP);
    fixture
        .changes
        .clear(id(500_101), kp::ELEMENT_DECLARED_NAME);
    let snapshot = fixture.finish();

    // A fork must retain missing producer evidence, not turn a warm current
    // graph answer into a closure certificate.
    let pending = q(&snapshot);
    let pending_answer = pending.effective_usage_types(id(500_001));
    assert_eq!(pending_answer.completeness(), Completeness::Incomplete);
    assert!(!pending_answer.pending.is_empty());
    let pending_fork = pending.fork();
    same_answer(
        &pending_answer,
        &pending_fork.effective_usage_types(id(500_001)),
    );

    // The synthetic dependency helper is pinned to Operational v2. Forking is
    // profile-preserving; it must not rebind that dependency to another profile.
    let profile = SysmlBaselineProfile::OPERATIONAL_V2;
    let closed = close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            let context = dependency
                .project_overlay_context(overlay, &[id(500_000)])
                .map_err(PublicationOverlayError::Context)?;
            Ok(
                crate::context::fixture_overlay_context(overlay, context, profile)
                    .unwrap()
                    .kerml,
            )
        },
        &SysmlProducerExtension::new(
            profile,
            StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32])),
            roots,
        ),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(closed.completeness, Completeness::Complete);
    let context = crate::context::fixture_overlay_context(
        &closed.overlay,
        dependency
            .project_overlay_context(&closed.overlay, &[id(500_000)])
            .unwrap(),
        profile,
    )
    .unwrap()
    .with_producer_closure(closed.certificate.unwrap())
    .unwrap();
    let original = SysmlQueries::new(context);
    let types = original.effective_usage_types(id(500_001));
    let names = original.effective_names(id(500_001));
    assert_eq!(types.completeness(), Completeness::Complete);
    assert_eq!(types.value(), &[id(500_000)]);
    assert_eq!(names.completeness(), Completeness::Complete);
    let warmed_count = original.kerml().negative_queries_certified();
    assert!(warmed_count > 0);
    let fork = original.fork();
    assert!(std::ptr::eq(original.model(), fork.model()));
    assert_eq!(original.context(), fork.context());
    assert_eq!(original.kerml().context(), fork.kerml().context());
    assert!(original.kerml().context().standard_bindings.is_some());
    assert_eq!(fork.kerml().negative_queries_certified(), 0);
    same_answer(&types, &fork.effective_usage_types(id(500_001)));
    same_answer(&names, &fork.effective_names(id(500_001)));
    assert!(fork.kerml().negative_queries_certified() > 0);
    assert_eq!(original.kerml().negative_queries_certified(), warmed_count);
    drop(original);
    same_answer(&types, &fork.effective_usage_types(id(500_001)));
}
