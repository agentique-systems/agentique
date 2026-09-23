//! Bounded closure work and timing observations; never acceptance evidence.
use agq_kerml_semantics::{PublicationCounters, PublicationRoundMetrics};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn counters(c: &PublicationCounters) -> Value {
    json!({
        "round": round(&c.round),
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
        "logical_search_sets": c.logical_search_sets,
        "logical_search_entries": c.logical_search_entries,
        "retained_search_sets": c.retained_search_sets,
        "retained_search_entries": c.retained_search_entries,
        "search_sets_interned": c.search_sets_interned,
        "search_sets_reused": c.search_sets_reused,
        "dependency_edges_considered": c.dependency_edges_considered,
        "dirty_subjects_enqueued": c.dirty_subjects_enqueued,
        "dirty_reevaluations": c.dirty_reevaluations,
        "maximum_worklist_size": c.maximum_worklist_size,
        "overlay_materializations": c.overlay_materializations,
        "empty_frontiers_reused": c.empty_frontiers_reused,
        "active_dependency_subjects": c.active_dependency_subjects,
        "active_dependency_keys": c.active_dependency_keys,
        "active_dependency_edges": c.active_dependency_edges,
        "maximum_dependency_keys": c.maximum_dependency_keys,
        "maximum_dependency_edges": c.maximum_dependency_edges,
        "fixed_point_rounds": c.fixed_point_rounds,
    })
}

pub fn round(r: &PublicationRoundMetrics) -> Value {
    json!({
        "subjects_evaluated": r.subjects_evaluated,
        "subjects_skipped": r.subjects_skipped,
        "subjects_reopened": r.subjects_reopened,
        "planned_elements": r.planned_elements,
        "accepted_elements": r.accepted_elements,
        "accepted_occurrences": r.accepted_occurrences,
        "next_dirty_subjects": r.next_dirty_subjects,
        "next_dirty_by_reason": r.next_dirty_by_reason.iter().map(|(reason, count)| (format!("{reason:?}"), count)).collect::<BTreeMap<_, _>>(),
        "families": r.families.iter().map(|(family, metrics)| (family.name(), json!({
            "attempts": metrics.attempts,
            "planning_micros": metrics.planning_micros,
            "shared_planning_micros": metrics.shared_planning_micros,
            "reopened_by_reason": metrics.reopened_by_reason.iter().map(|(reason, count)| (format!("{reason:?}"), count)).collect::<BTreeMap<_, _>>(),
        }))).collect::<BTreeMap<_, _>>(),
        "planning_including_query_cache_micros": r.planning_micros,
        "dependency_index_micros": r.dependency_index_micros,
        "model_materialization_micros": r.model_materialization_micros,
        "certificate_revalidation_micros": r.certificate_revalidation_micros,
        "certificate_build_micros": r.certificate_build_micros,
        "elapsed_micros": r.elapsed_micros,
    })
}
