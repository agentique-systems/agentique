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
        if answer.context != self.snapshot.context
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
        // Reuse the ordinary dependency mapping, retaining all canonical fact
        // and association endpoint reads. Only the separately proven closure
        // searches above may differ from ordinary global invalidation.
        for dependency in &answer.canonical_dependencies {
            let (Dependency::Declared(fact) | Dependency::Derived(fact)) = dependency;
            match fact {
                FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                    keys.insert(InvalidationKey::Element(*id));
                }
                FactKey::AssociationOccurrence(id) => {
                    if let Some(occurrence) = self.queries.model().association_occurrence(*id) {
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
