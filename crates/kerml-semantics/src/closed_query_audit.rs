//! Reuse of successful audit outcomes between fully producer-closed revisions.
//!
//! This transports a bounded acceptance outcome, never a QueryResult or its
//! revision-bound explanation. Ordinary query invalidation remains conservative.
use crate::read_dependencies::{InvalidationKey, search_keys, structural_search_key};
use crate::*;
use agq_kernel::{
    ElementId,
    derived::StructuralSearch,
    provenance::{Dependency, FactKey},
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Weak};

/// Exact immutable graph signatures and context behind a successful audit.
/// Construction requires an authenticated, fully closed semantic context.
#[derive(Clone, Debug)]
pub struct ClosedAuditSnapshot {
    context: SemanticContextId,
    signatures: BTreeMap<ElementId, [u8; 32]>,
    // The factored immutable rows must belong to the same actual mount, including
    // optional proof/contribution tables that semantic publication IDs may omit.
    dependency: Option<Weak<ProducerClosedDependency>>,
}

/// Positive reads and negative/provider searches of an audit outcome. Closure
/// witnesses are recorded separately and re-proved on the receiving revision.
#[derive(Clone, Debug, Default)]
pub struct ClosedAuditReads {
    context: Option<SemanticContextId>,
    elements: BTreeSet<ElementId>,
    global: bool,
    witnesses: BTreeSet<(ElementId, SemanticClosureRequirement)>,
    invalid: bool,
}
impl ClosedAuditReads {
    /// Include direct model reads such as audit dispatch on the subject's class.
    pub fn subject(&mut self, subject: ElementId) {
        self.elements.insert(subject);
    }
}

/// Read collector bound to one authenticated, fully closed evaluator. The
/// snapshot and all deltas are computed internally, never supplied by a caller.
pub struct ClosedAuditContext<'a, 'm> {
    queries: &'a KerMlQueries<'m>,
    snapshot: ClosedAuditSnapshot,
}
impl<'a, 'm> ClosedAuditContext<'a, 'm> {
    /// A Working/open producer frontier cannot supply audit reuse evidence.
    pub fn new(queries: &'a KerMlQueries<'m>) -> Option<Self> {
        let certificate = queries.context.producer_closure()?;
        if !certificate.compatible_context(queries.context())
            || !certificate.is_fully_closed(queries.model())
        {
            return None;
        }
        Some(Self {
            queries,
            snapshot: ClosedAuditSnapshot {
                context: queries.context().clone(),
                dependency: queries
                    .context
                    .closed_dependency
                    .as_ref()
                    .map(Arc::downgrade),
                signatures: crate::producer_closure::audit_subject_signatures(
                    queries.model(),
                    queries
                        .context
                        .closed_dependency
                        .as_ref()
                        .map(|base| base.overlay().model()),
                ),
            },
        })
    }

    /// Record every read from a Complete answer over this exact evaluator.
    /// Failed or mismatched answers make the containing outcome non-reusable.
    pub fn observe<T>(&self, reads: &mut ClosedAuditReads, answer: &QueryResult<T>) {
        if answer.producer_evidence
            || answer.contains_compact_evidence
            || answer.context != self.snapshot.context
            || answer.completeness != Completeness::Complete
            || reads
                .context
                .as_ref()
                .is_some_and(|context| context != &answer.context)
        {
            reads.invalid = true;
            return;
        }
        reads.context = Some(answer.context.clone());
        let mut keys = BTreeSet::new();
        for search in &answer.search_dependencies {
            match search {
                SearchDependency::ProducerClosure {
                    subject,
                    requirement,
                    certificate_digest,
                    source,
                } => {
                    let certificate = self
                        .queries
                        .context
                        .producer_closure()
                        .expect("checked closed context");
                    if *certificate_digest != Some(certificate.digest())
                        || source.is_none()
                        || !certificate.is_closed(*subject, *requirement)
                    {
                        reads.invalid = true;
                    }
                    reads.witnesses.insert((*subject, *requirement));
                }
                SearchDependency::Kernel(search) => self.structural(reads, &mut keys, search),
                _ => keys.extend(search_keys(search)),
            }
        }
        for search in answer.shared_search_dependencies.iter() {
            self.structural(reads, &mut keys, search);
        }
        // Retain immediate facts and every expanded positive proof input. A
        // derived root's own bytes can stay equal while a transitive input changes;
        // canonical_dependencies intentionally retains only the immediate DAG.
        // Sealed publication proofs stop at their exact immutable dependency root.
        // Negative/provider searches above remain independently conservative.
        let facts = answer
            .canonical_dependencies
            .iter()
            .map(|dependency| {
                let (Dependency::Declared(fact) | Dependency::Derived(fact)) = dependency;
                *fact
            })
            .chain(answer.positive_dependencies.iter().copied());
        for fact in facts {
            match fact {
                FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                    keys.insert(InvalidationKey::Element(id));
                }
                FactKey::AssociationOccurrence(id) => {
                    if let Some(occurrence) = self.queries.model().association_occurrence(id) {
                        keys.extend(
                            occurrence
                                .ends()
                                .values()
                                .copied()
                                .map(InvalidationKey::Element),
                        );
                    } else {
                        reads.invalid = true;
                    }
                }
            }
        }
        for key in keys {
            match key {
                InvalidationKey::Element(id) | InvalidationKey::Incoming(id) => {
                    reads.elements.insert(id);
                }
                InvalidationKey::Global => reads.global = true,
            }
        }
    }

    fn structural(
        &self,
        reads: &mut ClosedAuditReads,
        keys: &mut BTreeSet<InvalidationKey>,
        search: &StructuralSearch,
    ) {
        if let StructuralSearch::ProducerClosure {
            subject,
            requirement,
        } = search
            && let Some(requirement) = SemanticClosureRequirement::ALL
                .into_iter()
                .find(|candidate| format!("agq-semantic-closure/{candidate:?}/1") == *requirement)
        {
            if !self
                .queries
                .context
                .producer_closure()
                .expect("checked closed context")
                .is_closed(*subject, requirement)
            {
                reads.invalid = true;
            }
            reads.witnesses.insert((*subject, requirement));
        } else if let Some(key) = structural_search_key(search) {
            keys.insert(key);
        }
    }

    /// Compute the exact checked frontier once for all subjects in an audit.
    pub fn delta(&self, previous: &ClosedAuditSnapshot) -> Option<ClosedAuditDelta> {
        match (&previous.dependency, &self.snapshot.dependency) {
            (Some(previous), Some(current)) if Weak::ptr_eq(previous, current) => {}
            (None, None) => {}
            _ => return None,
        }
        let mut current = self.snapshot.context.clone();
        // Both snapshots were created only after full producer closure. The
        // requested witnesses below must still exist and be closed in the new
        // certificate; its old graph-bound identity is never copied forward.
        current.producer_closure_digest = previous.context.producer_closure_digest;
        if !QueryReadSet::context_compatible(&previous.context, &current) {
            return None;
        }
        let mut affected: BTreeSet<_> = previous
            .signatures
            .keys()
            .chain(self.snapshot.signatures.keys())
            .copied()
            .filter(|id| previous.signatures.get(id) != self.snapshot.signatures.get(id))
            .collect();
        for (old, new) in [
            (
                &previous.context.pending_specialization_scopes,
                &current.pending_specialization_scopes,
            ),
            (
                &previous.context.pending_namespace_scopes,
                &current.pending_namespace_scopes,
            ),
        ] {
            affected.extend(old.symmetric_difference(new).copied());
        }
        affected.extend(
            previous
                .context
                .construction_obligations
                .symmetric_difference(&current.construction_obligations)
                .map(|(id, _)| *id),
        );
        Some(ClosedAuditDelta {
            affected,
            previous: previous.context.clone(),
            current: self.snapshot.context.clone(),
        })
    }

    /// Check positive/negative reads and re-prove every used closure requirement.
    pub fn preserves(&self, reads: &ClosedAuditReads, delta: &ClosedAuditDelta) -> bool {
        !reads.invalid
            && self.snapshot.context == delta.current
            && reads
                .context
                .as_ref()
                .is_none_or(|context| context == &delta.previous)
            && (!reads.global || delta.affected.is_empty())
            && reads.elements.is_disjoint(&delta.affected)
            && reads.witnesses.iter().all(|&(id, requirement)| {
                self.queries
                    .context
                    .producer_closure()
                    .expect("checked closed context")
                    .is_closed(id, requirement)
            })
    }

    /// Rebind only the audit read receipt after all checks pass, allowing a
    /// sequence of edits to reuse the same proven outcome. No query evidence is
    /// relabeled by this operation.
    pub fn transport(
        &self,
        reads: &ClosedAuditReads,
        delta: &ClosedAuditDelta,
    ) -> Option<ClosedAuditReads> {
        self.preserves(reads, delta).then(|| {
            let mut reads = reads.clone();
            reads.context = Some(self.snapshot.context.clone());
            reads
        })
    }

    /// Verification-only rejection evidence. This observes the existing decision;
    /// it does not construct a delta, transport a receipt, or grant reuse.
    #[cfg(feature = "verification")]
    pub fn verification_reuse_trace(
        &self,
        previous: &ClosedAuditSnapshot,
        reads: Option<&ClosedAuditReads>,
        delta: Option<&ClosedAuditDelta>,
    ) -> serde_json::Value {
        let mount_matches = match (&previous.dependency, &self.snapshot.dependency) {
            (Some(previous), Some(current)) => Weak::ptr_eq(previous, current),
            (None, None) => true,
            _ => false,
        };
        let mut current = self.snapshot.context.clone();
        current.producer_closure_digest = previous.context.producer_closure_digest;
        let contract_matches = QueryReadSet::context_compatible(&previous.context, &current);
        let (Some(reads), Some(delta)) = (reads, delta) else {
            return serde_json::json!({ "reason": if !mount_matches {
                "dependency_mount_mismatch"
            } else if !contract_matches {
                "kerml_static_contract_mismatch"
            } else if reads.is_none() {
                "no_successful_prior_subject"
            } else {
                "no_checked_delta"
            }});
        };
        let changed_count = reads.elements.intersection(&delta.affected).count();
        let reason = if reads.invalid {
            "invalid_receipt"
        } else if self.snapshot.context != delta.current {
            "foreign_delta_context"
        } else if reads
            .context
            .as_ref()
            .is_some_and(|context| context != &delta.previous)
        {
            "foreign_receipt_context"
        } else if reads.global && !delta.affected.is_empty() {
            "global_search_changed"
        } else if changed_count != 0 {
            "bounded_read_changed"
        } else if reads.witnesses.iter().any(|&(id, requirement)| {
            !self
                .queries
                .context
                .producer_closure()
                .expect("checked closed context")
                .is_closed(id, requirement)
        }) {
            "closure_witness_missing"
        } else {
            "preserved"
        };
        let sample: Vec<_> = reads.elements.intersection(&delta.affected).take(8).map(|id| {
            let record = self.queries.model().element(*id);
            serde_json::json!({
                "element": id.to_string(),
                "name": record.and_then(|record| record.slot(agq_kerml::properties::ELEMENT_DECLARED_NAME))
                    .map(|slot| format!("{:?}", slot.value()).chars().take(160).collect::<String>()),
                "accepted_dependency": self.queries.context.closed_dependency.as_ref()
                    .is_some_and(|base| base.overlay().model().element(*id).is_some()),
            })
        }).collect();
        serde_json::json!({
            "reason": reason,
            "affected_subjects": delta.affected.len(),
            "recorded_elements": reads.elements.len(),
            "global_search": reads.global,
            "changed_reads": changed_count,
            "changed_read_sample": sample,
            "sample_limit": 8,
        })
    }

    /// Retain only immutable context/signature evidence after the audit finishes.
    pub fn into_snapshot(self) -> ClosedAuditSnapshot {
        self.snapshot
    }
}

/// Internally computed change frontier; includes incoming endpoints, derivation
/// proof/search support, property contributions and absent/new subjects.
pub struct ClosedAuditDelta {
    affected: BTreeSet<ElementId>,
    previous: SemanticContextId,
    current: SemanticContextId,
}

#[cfg(test)]
#[path = "../tests/unit/closed_query_audit.rs"]
mod tests;
