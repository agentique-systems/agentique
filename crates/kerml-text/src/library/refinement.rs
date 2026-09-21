//! Candidate reconstruction and dependency-checked reference outcome reuse.
use super::{LibraryDraft, LibraryLoadError, PendingLibraryReference};
use agq_kerml::properties as p;
use agq_kerml_semantics::{
    KerMlStatusQueries, QueryInvalidationSet, QueryReadSet, SemanticContextId,
};
use agq_kernel::{ConstructionView, ElementId, ElementRecord, ModelView, PropertyId, value::Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, Instant},
};

type ReferenceKey = (ElementId, PropertyId);
type Endpoints = BTreeMap<ReferenceKey, ElementId>;

/// Both strategies reconstruct the same ordinary canonical candidate each round.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReferenceRefinementStrategy {
    #[default]
    DependencyDriven,
    /// Re-evaluate every source reference, independent of cache invalidation.
    ReferenceFullScan,
}

/// Observations never participate in reference selection or semantic acceptance.
#[derive(Clone, Debug)]
pub struct ReferenceRefinementRound {
    pub round: usize,
    pub input_endpoints: usize,
    pub selected_endpoints: usize,
    pub structural_obligations: usize,
    pub references_considered: usize,
    pub references_evaluated: usize,
    pub references_reused: usize,
    pub affected_elements: usize,
    pub construction_elapsed: Duration,
    pub context_elapsed: Duration,
    pub change_detection_elapsed: Duration,
    pub resolution_elapsed: Duration,
}

struct CachedReference {
    request: PendingLibraryReference,
    reads: QueryInvalidationSet,
    // The expected-metaclass filter below is outside the semantic query, so its
    // candidate records are additional explicit positive cache dependencies.
    candidates: BTreeSet<ElementId>,
    selected: Option<ElementId>,
}

fn record_participants(record: &ElementRecord, affected: &mut BTreeSet<ElementId>) {
    affected.insert(record.id());
    affected.extend(
        record
            .slots()
            .flat_map(|(_, slot)| slot.value().values())
            .filter_map(|value| {
                if let Value::Reference(id) = value {
                    Some(*id)
                } else {
                    None
                }
            }),
    );
}

/// A merge walk compares canonical before/after records, including slot origins.
/// A retarget or removal invalidates both its old and new navigation participants.
fn changed_population(before: &ConstructionView, after: &ConstructionView) -> BTreeSet<ElementId> {
    let mut affected = BTreeSet::new();
    let mut old = before.model().elements().peekable();
    let mut new = after.model().elements().peekable();
    while let (Some(a), Some(b)) = (old.peek(), new.peek()) {
        match a.id().cmp(&b.id()) {
            std::cmp::Ordering::Less => record_participants(old.next().unwrap(), &mut affected),
            std::cmp::Ordering::Greater => record_participants(new.next().unwrap(), &mut affected),
            std::cmp::Ordering::Equal => {
                if a != b {
                    record_participants(a, &mut affected);
                    record_participants(b, &mut affected);
                }
                old.next();
                new.next();
            }
        }
    }
    for record in old.chain(new) {
        record_participants(record, &mut affected);
    }
    let mut old = before.model().association_occurrences().peekable();
    let mut new = after.model().association_occurrences().peekable();
    while let (Some(a), Some(b)) = (old.peek(), new.peek()) {
        match a.id().cmp(&b.id()) {
            std::cmp::Ordering::Less => {
                affected.extend(old.next().unwrap().ends().values().copied())
            }
            std::cmp::Ordering::Greater => {
                affected.extend(new.next().unwrap().ends().values().copied())
            }
            std::cmp::Ordering::Equal => {
                if a != b {
                    affected.extend(a.ends().values().copied());
                    affected.extend(b.ends().values().copied());
                }
                old.next();
                new.next();
            }
        }
    }
    for occurrence in old.chain(new) {
        affected.extend(occurrence.ends().values().copied());
    }
    let obligations = |candidate: &ConstructionView| {
        candidate
            .obligations()
            .iter()
            .map(|o| (o.element, o.property, o.actual))
            .collect::<BTreeSet<_>>()
    };
    affected.extend(
        obligations(before)
            .symmetric_difference(&obligations(after))
            .map(|(id, _, _)| *id),
    );
    // Direct containment owners are negative population search keys even when
    // the changed endpoint is represented by an AssociationOccurrence. One hop
    // finds the owning relationship; a second finds its namespace/Type owner.
    // Do not propagate arbitrary descendants to every ancestor root: reads of
    // transitive scope already carry their individual bounded search keys.
    let participants: Vec<_> = affected.iter().copied().collect();
    for model in [before.model(), after.model()] {
        for &id in &participants {
            for owner in ownership_sources(model, id) {
                affected.insert(owner);
                affected.extend(ownership_sources(model, owner));
            }
        }
    }
    affected
}

fn ownership_sources(model: &ModelView, id: ElementId) -> impl Iterator<Item = ElementId> + '_ {
    model
        .incoming(id)
        .filter(|r| {
            matches!(
                r.property,
                p::ELEMENT_OWNED_RELATIONSHIP | p::RELATIONSHIP_OWNED_RELATED_ELEMENT
            )
        })
        .map(|r| r.source)
}

fn context_changes(
    previous: &SemanticContextId,
    current: &SemanticContextId,
    affected: &mut BTreeSet<ElementId>,
) -> bool {
    affected.extend(
        previous
            .pending_specialization_scopes
            .symmetric_difference(&current.pending_specialization_scopes)
            .copied(),
    );
    affected.extend(
        previous
            .pending_namespace_scopes
            .symmetric_difference(&current.pending_namespace_scopes)
            .copied(),
    );
    affected.extend(
        previous
            .construction_obligations
            .symmetric_difference(&current.construction_obligations)
            .map(|(id, _)| *id),
    );
    !QueryReadSet::context_compatible(previous, current)
}

pub(super) fn refine(
    mut construct: impl FnMut(&Endpoints) -> Result<LibraryDraft, LibraryLoadError>,
    mut query: impl for<'m> FnMut(&'m LibraryDraft) -> Result<KerMlStatusQueries<'m>, LibraryLoadError>,
    strategy: ReferenceRefinementStrategy,
    mut progress: impl FnMut(&ReferenceRefinementRound),
) -> Result<LibraryDraft, LibraryLoadError> {
    let mut resolved = Endpoints::new();
    let mut previous_resolutions = BTreeSet::new();
    let mut previous_candidate: Option<ConstructionView> = None;
    let mut previous_context: Option<SemanticContextId> = None;
    let mut cache = BTreeMap::<ReferenceKey, CachedReference>::new();
    for round in 0.. {
        if !previous_resolutions.insert(resolved.clone()) {
            return Err(LibraryLoadError::Interpretation(
                "Resolution refinement cycle".into(),
            ));
        }
        let start = Instant::now();
        let draft = construct(&resolved)?;
        let construction_elapsed = start.elapsed();
        let start = Instant::now();
        let mut affected = previous_candidate
            .take()
            .as_ref()
            .map_or_else(BTreeSet::new, |old| {
                changed_population(old, &draft.candidate)
            });
        // Release the old graph before retaining any new query caches.
        let change_detection_elapsed = start.elapsed();
        let start = Instant::now();
        let mut queries = query(&draft)?;
        let context = queries.context().clone();
        let context_elapsed = start.elapsed();
        let contract_changed = previous_context
            .as_ref()
            .is_none_or(|old| context_changes(old, &context, &mut affected));
        let mut report = ReferenceRefinementRound {
            round,
            input_endpoints: resolved.len(),
            selected_endpoints: 0,
            structural_obligations: draft.candidate.obligations().len(),
            references_considered: draft.references.len(),
            references_evaluated: 0,
            references_reused: 0,
            affected_elements: affected.len(),
            construction_elapsed,
            context_elapsed,
            change_detection_elapsed,
            resolution_elapsed: Duration::ZERO,
        };
        let start = Instant::now();
        let mut next = Endpoints::new();
        let mut next_cache = BTreeMap::new();
        for reference in &draft.references {
            let key = (reference.relationship, reference.property);
            let cached = cache.remove(&key).filter(|cached| {
                strategy == ReferenceRefinementStrategy::DependencyDriven
                    && cached.request == *reference
                    && cached.candidates.is_disjoint(&affected)
                    && !cached.reads.affected_by(&affected, contract_changed)
            });
            let cached = if let Some(cached) = cached {
                report.references_reused += 1;
                cached
            } else {
                if report.references_evaluated > 0
                    && report.references_evaluated.is_multiple_of(128)
                {
                    queries = queries.fork();
                    assert_eq!(queries.context(), &context);
                }
                report.references_evaluated += 1;
                let result = queries.lookup_relationship_target_with_reads(
                    reference.relationship,
                    reference.property,
                    &reference.name,
                );
                let candidates = result
                    .outcome
                    .value
                    .iter()
                    .flat_map(|m| [m.element, m.membership])
                    .collect();
                let selected = if let [member] = result.outcome.value.as_slice() {
                    let target = if reference.membership_target {
                        member.membership
                    } else {
                        member.element
                    };
                    let record = draft
                        .candidate
                        .model()
                        .element(target)
                        .expect("query endpoint");
                    draft
                        .candidate
                        .model()
                        .registry()
                        .is_subtype(record.metaclass(), reference.expected)
                        .map_err(agq_kernel::ModelError::from)?
                        .then_some(target)
                } else {
                    None
                };
                // These are provisional canonical-construction candidates. The
                // completeness requirement remains in the separate final audit.
                CachedReference {
                    request: reference.clone(),
                    reads: result.reads.into_invalidation(),
                    candidates,
                    selected,
                }
            };
            if let Some(target) = cached.selected {
                next.insert(key, target);
            }
            next_cache.insert(key, cached);
        }
        report.selected_endpoints = next.len();
        report.resolution_elapsed = start.elapsed();
        progress(&report);
        drop(queries);
        if next == resolved {
            return Ok(draft);
        }
        previous_candidate = Some(draft.candidate);
        previous_context = Some(context);
        cache = next_cache;
        resolved = next;
    }
    unreachable!("unbounded refinement loop exits on stable state, cycle or construction error")
}

#[cfg(test)]
#[path = "refinement_tests.rs"]
mod tests;
