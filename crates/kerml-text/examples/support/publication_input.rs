//! Shared preparation for bounded, explicitly scoped publication preflights.
use agq_kerml_semantics::*;
use agq_kerml_text::library::LibraryDraft;
use agq_kernel::{DocumentId, ElementId, Snapshot, derived::DerivedOverlay, provenance::FactKey};
use agq_standard_libraries::VerifiedLibrarySet;
use std::collections::{BTreeMap, BTreeSet};
#[path = "publication_dependencies.rs"]
mod publication_dependencies;
pub use publication_dependencies::Boundary as PublicationScopeBoundary;
#[path = "publication_refinement.rs"]
mod publication_refinement;

pub struct PublicationInput {
    pub draft: LibraryDraft,
    pub snapshot: Snapshot,
    pub identity: SemanticContextId,
    pub refinement: Vec<serde_json::Value>,
}
impl PublicationInput {
    pub fn load(sources: &VerifiedLibrarySet) -> Result<Self, Box<dyn std::error::Error>> {
        let (draft, refinement) = publication_refinement::prepare(sources)?;
        let identity = draft.queries(sources)?.context().clone();
        let snapshot = draft.strict_snapshot()?;
        Ok(Self {
            draft,
            snapshot,
            identity,
            refinement,
        })
    }

    pub fn context<'m>(
        &self,
        overlay: &'m DerivedOverlay,
    ) -> Result<SemanticContext<'m>, PublicationOverlayError> {
        let bindings = self
            .identity
            .standard_bindings
            .as_ref()
            .expect("validated bindings");
        Ok(SemanticContext::for_overlay(
            overlay,
            self.identity.options.clone(),
            self.identity.pinned_libraries.clone(),
        )
        .and_then(|c| c.with_available_roots((*self.identity.available_roots).clone()))
        .map_err(PublicationOverlayError::Context)?
        .with_standard_bindings(self.draft.roots(), bindings.library_set())
        .map_err(PublicationOverlayError::Bindings)?
        .with_formal_constraint_targets(self.draft.roots(), bindings.library()))
    }

    /// Select real declarations and their structural and semantic owner dependencies.
    /// Imports retain the complete declared namespace environment but do not
    /// demand every unrelated producer in that namespace. Enclosing Types are
    /// selected because their producers can change a referenced child Feature.
    pub fn subjects(
        &self,
        sources: &VerifiedLibrarySet,
        documents: &[&str],
    ) -> Result<BTreeSet<ElementId>, Box<dyn std::error::Error>> {
        for name in documents {
            if sources
                .documents()
                .filter(|d| d.path().ends_with(name))
                .count()
                != 1
            {
                return Err(
                    format!("Slice document must identify one pinned source: {name}").into(),
                );
            }
        }
        let documents: BTreeSet<DocumentId> = sources
            .documents()
            .filter(|d| documents.iter().any(|name| d.path().ends_with(name)))
            .map(|d| d.document())
            .collect();
        let mut selected: BTreeSet<_> = self
            .draft
            .source_map()
            .iter()
            .filter_map(|(fact, source)| match fact {
                FactKey::Element(id) if documents.contains(&source.document) => Some(*id),
                _ => None,
            })
            .collect();
        if selected.is_empty() {
            return Err("Slice has no canonical source subjects".into());
        }
        // These producer inputs are contextual anchors, not authored graph edges.
        selected.extend(
            self.identity
                .standard_bindings
                .as_ref()
                .expect("validated bindings")
                .iter()
                .map(|(_, id)| id),
        );
        Ok(publication_dependencies::subjects(
            self.snapshot.model(),
            selected,
        )?)
    }

    pub fn document_counts(
        &self,
        subjects: &BTreeSet<ElementId>,
        sources: &VerifiedLibrarySet,
    ) -> BTreeMap<String, usize> {
        let paths: BTreeMap<_, _> = sources
            .documents()
            .map(|d| (d.document(), d.path()))
            .collect();
        let mut counts = BTreeMap::new();
        for (fact, source) in self.draft.source_map() {
            if matches!(fact, FactKey::Element(id) if subjects.contains(id)) {
                *counts
                    .entry(paths[&source.document].to_owned())
                    .or_default() += 1;
            }
        }
        counts
    }
}
