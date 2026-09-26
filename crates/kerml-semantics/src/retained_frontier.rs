//! Verification-only experiment in checked, retractable derived-state reuse.
use super::*;
use crate::{ContextError, PublicationOverlayError, SemanticContext};
use agq_kernel::{
    Snapshot,
    derived::{DerivedOverlay, StructuralSearch},
    provenance::{Dependency, FactKey},
};
use std::collections::HashMap;

#[cfg(test)]
#[path = "../tests/unit/retained_frontier.rs"]
mod tests;

/// An unpublished semantic frontier. Ordinary producers and the effective audit
/// must still complete before the caller may accept the reconstructed revision.
pub struct VerificationRetainedSemanticFrontier {
    pub overlay: DerivedOverlay,
    pub certificate: Arc<ProducerClosureCertificate>,
    pub original_facts: usize,
    pub retained_facts: usize,
    pub retraction_rounds: usize,
    pub retained_evaluations: usize,
    pub reopened_evaluations: usize,
}

/// Retain exact old facts only after positive, negative and potential-writer
/// checks stabilize. This experimental path is absent from production builds.
/// It accepts no caller-provided affected set and never mints acceptance.
pub fn verification_retain_semantic_frontier<F>(
    previous: &DerivedOverlay,
    old: &SemanticContext<'_>,
    declared: Snapshot,
    registry: &ProducerRegistry,
    context: F,
) -> Result<VerificationRetainedSemanticFrontier, PublicationOverlayError>
where
    F: for<'m> Fn(&'m DerivedOverlay) -> Result<SemanticContext<'m>, ContextError>,
{
    let certificate = old
        .producer_closure()
        .ok_or(PublicationOverlayError::Context(
            ContextError::ProducerClosureMismatch,
        ))?;
    if !std::ptr::eq(old.model, previous.model()) || !certificate.is_fully_closed(old.model) {
        return Err(PublicationOverlayError::Context(
            ContextError::ProducerClosureMismatch,
        ));
    }
    let checkpoint = certificate
        .checkpoint_sharing_dependency(old)
        .map_err(PublicationOverlayError::Context)?;
    let dependency = old
        .closed_dependency
        .as_ref()
        .map(|base| base.overlay().model());
    let original_signatures = audit_subject_signatures(old.model, dependency);
    let mut requested = previous.verification_local_facts();
    let original_facts = requested.len();
    let mut rounds = 0;
    loop {
        rounds += 1;
        let rebuilt = previous.verification_retain_on(declared.clone(), &requested)?;
        let current = context(&rebuilt.overlay).map_err(PublicationOverlayError::Context)?;
        let rebound = checkpoint
            .rebind(&current, registry)
            .map_err(PublicationOverlayError::Context)?;
        let current_signatures = audit_subject_signatures(current.model, dependency);
        let affected: BTreeSet<_> = original_signatures
            .keys()
            .chain(current_signatures.keys())
            .copied()
            .filter(|id| original_signatures.get(id) != current_signatures.get(id))
            .collect();
        let stable = |subject| {
            SemanticClosureRequirement::ALL
                .into_iter()
                .all(|requirement| rebound.certificate.is_closed(subject, requirement))
        };
        let mut proofs = HashMap::new();
        let mut searches = HashMap::new();
        let retained: BTreeSet<_> = rebuilt
            .retained
            .iter()
            .copied()
            .filter(|fact| {
                let proof = previous
                    .explain(*fact)
                    .expect("kernel retained exact old fact");
                let proof_stable = *proofs.entry(proof as *const _ as usize).or_insert_with(|| {
                    proof.dependencies.iter().all(|read| {
                        let (Dependency::Declared(key) | Dependency::Derived(key)) = read;
                        positive_unchanged(old.model, current.model, *key)
                            && fact_subjects(old.model, *key).into_iter().all(&stable)
                    })
                });
                if !proof_stable {
                    return false;
                }
                previous
                    .model()
                    .computation_searches_shared(*fact)
                    .is_none_or(|recorded| {
                        *searches
                            .entry(Arc::as_ptr(recorded) as usize)
                            .or_insert_with(|| {
                                recorded.iter().all(|search| {
                                    search_unchanged(
                                        search,
                                        old.model,
                                        current.model,
                                        &affected,
                                        &rebound.certificate,
                                    )
                                })
                            })
                    })
            })
            .collect();
        drop(current);
        if retained == rebuilt.retained {
            return Ok(VerificationRetainedSemanticFrontier {
                overlay: rebuilt.overlay,
                certificate: rebound.certificate,
                original_facts,
                retained_facts: retained.len(),
                retraction_rounds: rounds,
                retained_evaluations: rebound.retained_evaluations,
                reopened_evaluations: rebound.reopened_evaluations,
            });
        }
        // Strictly decreasing, so this reaches a fixed point without a timeout
        // masquerading as completeness. The next kernel pass retracts dependent
        // facts and incomplete created records before the next causal check.
        requested = retained;
    }
}

fn positive_unchanged(old: &ModelView, new: &ModelView, fact: FactKey) -> bool {
    match fact {
        FactKey::Element(id) => old.element(id).is_some() && old.element(id) == new.element(id),
        FactKey::Property { element, property } => {
            old.navigation_slot(element, property).is_some()
                && old.navigation_slot(element, property) == new.navigation_slot(element, property)
        }
        FactKey::AssociationOccurrence(id) => {
            old.association_occurrence(id).is_some()
                && old.association_occurrence(id) == new.association_occurrence(id)
        }
    }
}

fn fact_subjects(model: &ModelView, fact: FactKey) -> Vec<ElementId> {
    match fact {
        FactKey::Element(id) | FactKey::Property { element: id, .. } => vec![id],
        FactKey::AssociationOccurrence(id) => model
            .association_occurrence(id)
            .map_or_else(Vec::new, |occurrence| {
                occurrence.ends().values().copied().collect()
            }),
    }
}

fn search_unchanged(
    search: &StructuralSearch,
    old: &ModelView,
    new: &ModelView,
    affected: &BTreeSet<ElementId>,
    certificate: &ProducerClosureCertificate,
) -> bool {
    let stable = |id| {
        SemanticClosureRequirement::ALL
            .into_iter()
            .all(|requirement| certificate.is_closed(id, requirement))
    };
    let bounded = |id| !affected.contains(&id) && stable(id);
    match search {
        StructuralSearch::DescriptorGraph => true, // exact registry allocation checked by kernel
        StructuralSearch::ElementIdentity(id) => {
            // Existing identities are immutable under additive producers. An
            // absent identity cannot rule out a newly applicable future writer.
            old.element(*id).is_some()
                && old.element(*id).map(|record| record.metaclass())
                    == new.element(*id).map(|record| record.metaclass())
        }
        StructuralSearch::DeclaredProperty { element, property } => {
            old.declared_slot(*element, *property) == new.declared_slot(*element, *property)
        }
        StructuralSearch::Property { element, property } => {
            old.navigation_slot(*element, *property) == new.navigation_slot(*element, *property)
                && stable(*element)
        }
        StructuralSearch::OrderedReferenceContribution {
            element,
            property,
            target,
        } => {
            old.ordered_reference_contribution(*element, *property, *target)
                == new.ordered_reference_contribution(*element, *property, *target)
                && stable(*element)
        }
        StructuralSearch::ProducerClosure {
            subject,
            requirement,
        } => SemanticClosureRequirement::from_contract_id(requirement)
            .is_some_and(|requirement| certificate.is_closed(*subject, requirement)),
        StructuralSearch::Model => affected.is_empty() && certificate.is_fully_closed(new),
        StructuralSearch::OwnedMemberProjection { owner, contract } => {
            crate::FeaturePopulationKind::from_contract_id(contract).is_some() && bounded(*owner)
        }
        StructuralSearch::Element(id)
        | StructuralSearch::Incoming(id)
        | StructuralSearch::Association { element: id, .. }
        | StructuralSearch::RelationshipStructure { element: id }
        | StructuralSearch::SourceRelationships { source: id, .. }
        | StructuralSearch::OwnedRelationships { owner: id, .. }
        | StructuralSearch::OwnedRelationshipsExcluding { owner: id, .. } => bounded(*id),
    }
}
