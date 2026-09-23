use super::*;
use crate::{KerMlQueries, QueryReadSet};

fn snapshot() -> Snapshot {
    Snapshot::new(Arc::new(agq_kerml::registry().unwrap()))
}

#[test]
fn producer_source_identity_is_opt_in_and_registry_reattachment_is_idempotent() {
    let snapshot = snapshot();
    let original =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let historical_digest = crate::context_digest::model_digest(snapshot.model());
    assert_eq!(original.id().model_digest, historical_digest);
    let registry = crate::ProducerRegistry::new([]).unwrap();
    let attached = original
        .fork()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    assert_ne!(attached.id().model_digest, historical_digest);
    let repeated = attached
        .fork()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    assert_eq!(attached.id(), repeated.id());
    assert_eq!(original.id().model_digest, historical_digest);
    assert!(std::ptr::eq(original.model, attached.model));
}

#[test]
fn context_mismatch_diagnostics_name_only_changed_identity_fields() {
    let snapshot = snapshot();
    let context =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let original = context.id();
    assert!(original.differing_fields(original).is_empty());
    let mut changed = original.clone();
    changed.options.exclude_implied = !changed.options.exclude_implied;
    changed.producer_registry_digest = Some([17; 32]);
    assert_eq!(
        changed.differing_fields(original),
        ["options", "producer_registry_digest"]
    );
}

#[test]
fn composed_identity_is_frozen_without_mutating_the_graph_or_original_context() {
    let snapshot = snapshot();
    let original =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    assert!(original.id().semantic_extensions.is_empty());
    let composed = original
        .fork()
        .with_semantic_extension_identity("fixture-language/1", [1; 32])
        .unwrap();
    assert!(original.id().semantic_extensions.is_empty());
    assert_eq!(original.id().model_digest, composed.id().model_digest);
    assert_eq!(
        original.id().descriptor_digest,
        composed.id().descriptor_digest
    );
    assert!(std::ptr::eq(original.model, composed.model));
    let idempotent = composed
        .fork()
        .with_semantic_extension_identity("fixture-language/1", [1; 32])
        .unwrap();
    assert_eq!(composed.id(), idempotent.id());
    assert!(matches!(
        composed
            .fork()
            .with_semantic_extension_identity("fixture-language/1", [2; 32]),
        Err(ContextError::SemanticExtensionIdentityMismatch(
            "fixture-language/1"
        ))
    ));
    let independent = composed
        .fork()
        .with_semantic_extension_identity("other-language/1", [2; 32])
        .unwrap();
    assert_eq!(independent.id().semantic_extensions.len(), 2);
    let answer = KerMlQueries::new(composed.fork()).owned_relationships(ElementId::from_u128(1));
    assert_eq!(answer.context, *composed.id());
}

#[test]
fn changed_language_interpretation_invalidates_cached_outcomes_on_unchanged_records() {
    let snapshot = snapshot();
    let base =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let first = base
        .fork()
        .with_semantic_extension_identity("fixture-language/1", [1; 32])
        .unwrap();
    let second = base
        .fork()
        .with_semantic_extension_identity("fixture-language/1", [2; 32])
        .unwrap();
    assert!(!QueryReadSet::context_compatible(base.id(), first.id()));
    assert!(!QueryReadSet::context_compatible(first.id(), second.id()));
    let mut graph_changed = first.id().clone();
    graph_changed.model_digest[0] ^= 1;
    graph_changed.revision = agq_kernel::RevisionId::new();
    assert!(QueryReadSet::context_compatible(first.id(), &graph_changed));
    graph_changed
        .semantic_extensions
        .insert("fixture-language/1", [3; 32]);
    assert!(!QueryReadSet::context_compatible(
        first.id(),
        &graph_changed
    ));
}

#[test]
fn producer_registry_and_closure_change_query_identity_without_model_changes() {
    let snapshot = snapshot();
    let base =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let first = base.fork().with_producer_registry_digest([1; 32]).unwrap();
    assert!(std::ptr::eq(base.model, first.model));
    assert_ne!(base.id().model_digest, first.id().model_digest);
    assert!(!QueryReadSet::context_compatible(base.id(), first.id()));
    assert_eq!(
        first.id(),
        first
            .fork()
            .with_producer_registry_digest([1; 32])
            .unwrap()
            .id()
    );
    assert!(matches!(
        first.fork().with_producer_registry_digest([2; 32]),
        Err(ContextError::ProducerRegistryIdentityMismatch)
    ));
    let mut attached = first.id().clone();
    attached.producer_closure_digest = Some([3; 32]);
    assert!(!QueryReadSet::context_compatible(first.id(), &attached));
    assert_eq!(
        first.id().closure_contract_digest(),
        attached.closure_contract_digest()
    );
    attached.producer_registry_digest = Some([2; 32]);
    assert_ne!(
        first.id().closure_contract_digest(),
        attached.closure_contract_digest()
    );
}

#[test]
fn closure_contract_is_independent_of_revision_label_and_evidence_attachment() {
    let snapshot = snapshot();
    let context =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let mut alternative = context.id().clone();
    alternative.revision = RevisionId::new();
    alternative.model_digest = [5; 32];
    alternative.derivation_phase = crate::DerivationPhase::PartialDerivationOverlay;
    alternative.producer_closure_digest = Some([6; 32]);
    assert_eq!(
        context.id().closure_contract_digest(),
        alternative.closure_contract_digest()
    );
    alternative
        .pending_specialization_scopes
        .insert(ElementId::from_u128(77));
    assert_ne!(
        context.id().closure_contract_digest(),
        alternative.closure_contract_digest()
    );
}

#[test]
fn missing_closure_is_a_typed_dependency_without_a_fake_canonical_fact() {
    use crate::{Completeness, SearchDependency, SemanticClosureRequirement};
    let snapshot = snapshot();
    let context =
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
    let queries = KerMlQueries::new(context);
    let subject = ElementId::from_u128(5);
    let answer = queries.producer_closure(subject, SemanticClosureRequirement::EffectiveTyping);
    assert!(!answer.value);
    assert_eq!(answer.completeness, Completeness::Incomplete);
    assert!(answer.positive_dependencies.is_empty());
    assert!(answer.canonical_dependencies.is_empty());
    assert!(
        answer
            .search_dependencies
            .contains(&SearchDependency::ProducerClosure {
                subject,
                requirement: SemanticClosureRequirement::EffectiveTyping,
                certificate_digest: None,
                source: None,
            })
    );
}
