use super::*;
use crate::{KerMlQueries, QueryReadSet};

fn snapshot() -> Snapshot {
    Snapshot::new(Arc::new(agq_kerml::registry().unwrap()))
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
