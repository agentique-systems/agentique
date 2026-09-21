//! Immutable accepted standard libraries. Construction and conformance are separate.
use super::{LibraryDraft, LibraryLoadError, LibrarySourceMap};
use agq_kerml::BaselineProfile;
use agq_kerml_semantics::{
    CanonicalPublicationBuilder, CompletePublicationOverlay, Completeness, KerMlQueries,
    LibrarySetIdentity, PublicationOverlayError, PublicationStage, SemanticContextId,
    StandardKermlBindings,
};
use agq_kernel::{ElementId, Snapshot, derived::DerivedOverlay, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use std::sync::Arc;

#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;

/// A mandatory reference must have one Complete answer matching its stored endpoint.
#[derive(Clone, Debug)]
pub struct PublicationReferenceFailure {
    pub relationship: ElementId,
    pub completeness: Completeness,
    pub candidates: usize,
    pub canonical_endpoint_valid: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CanonicalPublicationError {
    #[error(transparent)]
    Source(#[from] LibraryLoadError),
    #[error("{0}")]
    Overlay(#[from] PublicationOverlayError),
    #[error("{} mandatory references failed canonical publication", .0.len())]
    References(Vec<PublicationReferenceFailure>),
    #[error("canonical publication requires Operational v8")]
    Profile,
}

/// One immutable canonical graph with its declared source and sealed producer closure.
/// No public constructor accepts caller-supplied capability or conformance labels.
pub struct CanonicalKermlStandardLibraries {
    snapshot: Snapshot,
    complete: CompletePublicationOverlay,
    shared_overlay: Arc<DerivedOverlay>,
    source_map: LibrarySourceMap,
    roots: Vec<ElementId>,
    mandatory_references: usize,
    source_content_set: String,
}
impl CanonicalKermlStandardLibraries {
    /// Publish the exact verified source construction. All producers and mandatory
    /// references must close; validator coverage is deliberately not an input.
    pub fn publish(
        draft: LibraryDraft,
        sources: &VerifiedLibrarySet,
        max_stages: usize,
        batch_progress: impl FnMut(usize, usize, usize, usize),
        stage_progress: impl FnMut(&PublicationStage),
        mut reference_progress: impl FnMut(usize, usize),
        capability_progress: impl FnMut(usize, usize, usize),
    ) -> Result<Self, CanonicalPublicationError> {
        if draft.baseline_profile() != BaselineProfile::OPERATIONAL_V8 {
            return Err(CanonicalPublicationError::Profile);
        }
        let original = draft.queries(sources)?.context().clone();
        let bindings = original
            .standard_bindings
            .as_ref()
            .expect("validated library bindings");
        let snapshot = draft.strict_snapshot()?;
        let complete =
            CanonicalPublicationBuilder::new(&snapshot, draft.roots(), bindings.library_set())
                .build_with_progress(
                    max_stages,
                    batch_progress,
                    stage_progress,
                    capability_progress,
                )?;
        let model = complete.overlay().model();
        let mut failures = vec![];
        for (index, batch) in draft.references().chunks(32).enumerate() {
            let q = complete.queries().status_queries();
            for reference in batch {
                let answer = q.lookup_relationship_target(
                    reference.relationship,
                    reference.property,
                    &reference.name,
                );
                let stored: Vec<_> = model
                    .navigation_slot(reference.relationship, reference.property)
                    .into_iter()
                    .flat_map(|slot| slot.value().values())
                    .filter_map(|value| {
                        if let Value::Reference(id) = value {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();
                let valid = answer.value.first().is_some_and(|target| {
                    let target = if reference.membership_target {
                        target.membership
                    } else {
                        target.element
                    };
                    stored == [target]
                        && model.element(target).is_some_and(|record| {
                            model
                                .registry()
                                .is_subtype(record.metaclass(), reference.expected)
                                .unwrap_or(false)
                        })
                });
                if answer.completeness != Completeness::Complete
                    || answer.value.len() != 1
                    || !valid
                {
                    failures.push(PublicationReferenceFailure {
                        relationship: reference.relationship,
                        completeness: answer.completeness,
                        candidates: answer.value.len(),
                        canonical_endpoint_valid: valid,
                    });
                }
            }
            reference_progress(
                ((index + 1) * 32).min(draft.references().len()),
                failures.len(),
            );
        }
        if !failures.is_empty() {
            return Err(CanonicalPublicationError::References(failures));
        }
        Ok(Self {
            shared_overlay: Arc::new(complete.overlay().clone()),
            snapshot,
            complete,
            source_map: draft.source_map,
            roots: draft.roots,
            mandatory_references: draft.references.len(),
            source_content_set: sources.content_set_id().into(),
        })
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn overlay(&self) -> &DerivedOverlay {
        &self.shared_overlay
    }
    pub fn complete_overlay(&self) -> &CompletePublicationOverlay {
        &self.complete
    }
    pub fn queries(&self) -> KerMlQueries<'_> {
        self.complete.queries()
    }
    pub fn context(&self) -> &SemanticContextId {
        self.complete.context()
    }
    pub fn profile(&self) -> BaselineProfile {
        BaselineProfile::OPERATIONAL_V8
    }
    pub fn bindings(&self) -> &StandardKermlBindings {
        self.context()
            .standard_bindings
            .as_ref()
            .expect("accepted bindings")
    }
    pub fn library_set(&self) -> &LibrarySetIdentity {
        self.bindings().library_set()
    }
    pub fn source_map(&self) -> &LibrarySourceMap {
        &self.source_map
    }
    pub fn roots(&self) -> &[ElementId] {
        &self.roots
    }
    pub fn semantic_digest(&self) -> [u8; 32] {
        self.context().model_digest
    }
    pub fn source_content_set(&self) -> &str {
        &self.source_content_set
    }
    pub fn mandatory_reference_count(&self) -> usize {
        self.mandatory_references
    }
    /// Independent authored history sharing the immutable accepted publication.
    pub(crate) fn project_snapshot(&self) -> Snapshot {
        Snapshot::with_immutable_dependency(self.shared_overlay.clone())
    }
}
impl std::fmt::Debug for CanonicalKermlStandardLibraries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalKermlStandardLibraries")
            .field("semantic_digest", &self.semantic_digest())
            .field("source_content_set", &self.source_content_set)
            .finish_non_exhaustive()
    }
}
