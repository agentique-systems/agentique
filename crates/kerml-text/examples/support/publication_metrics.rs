//! Deterministic closure counters, separate from resource observations.
use agq_kerml_semantics::PublicationCounters;
use serde_json::{Value, json};

pub fn counters(c: &PublicationCounters) -> Value {
    json!({
        "declared_subjects": c.declared_subjects,
        "subjects_considered": c.subjects_considered,
        "subjects_evaluated": c.subjects_evaluated,
        "subjects_skipped_by_applicability": c.subjects_skipped_by_applicability,
        "producer_families_attempted": c.producer_families_attempted,
        "new_elements_proposed": c.new_elements_proposed,
        "new_association_occurrences_proposed": c.new_association_occurrences_proposed,
        "existing_derived_facts_reused": c.existing_derived_facts_reused,
        "new_proof_sets_interned": c.new_proof_sets_interned,
        "existing_proof_sets_reused": c.existing_proof_sets_reused,
        "dependency_edges_considered": c.dependency_edges_considered,
        "dirty_subjects_enqueued": c.dirty_subjects_enqueued,
        "dirty_reevaluations": c.dirty_reevaluations,
        "maximum_worklist_size": c.maximum_worklist_size,
        "overlay_materializations": c.overlay_materializations,
        "fixed_point_rounds": c.fixed_point_rounds,
    })
}
