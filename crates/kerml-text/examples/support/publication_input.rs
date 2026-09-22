//! Shared preparation for bounded, explicitly scoped publication preflights.
use agq_kerml_semantics::*;
use agq_kerml_text::library::LibraryDraft;
use agq_kernel::{ElementId, Snapshot, derived::DerivedOverlay, provenance::FactKey};
use agq_standard_libraries::VerifiedLibrarySet;
use std::collections::{BTreeMap, BTreeSet};
#[path = "publication_documents.rs"]
mod publication_documents;
pub use publication_documents::{SLICES, slice_documents, validate_all_documents};
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
    pub fn expanded_subjects(
        &self,
        population: &BTreeSet<ElementId>,
        missing: &BTreeSet<ElementId>,
        maximum: usize,
    ) -> Result<BTreeSet<ElementId>, String> {
        publication_dependencies::expanded_subjects(
            self.snapshot.model(),
            population,
            missing,
            maximum,
        )
    }

    /// Explicit fixture seeds use semantic qualified lookup, never hardcoded IDs.
    pub fn fixture_subject(
        &self,
        sources: &VerifiedLibrarySet,
        segments: &[&str],
    ) -> Result<ElementId, Box<dyn std::error::Error>> {
        let q = self.draft.queries(sources)?;
        let mut candidates = BTreeSet::new();
        for &root in self.draft.roots() {
            let result = q.lookup_path(
                root,
                &QualifiedName {
                    absolute: true,
                    segments: segments
                        .iter()
                        .map(|segment| (*segment).to_owned())
                        .collect(),
                },
            );
            if result.completeness != Completeness::Complete {
                return Err(format!(
                    "Slice fixture lookup {segments:?} is {:?}",
                    result.completeness
                )
                .into());
            }
            candidates.extend(result.value.iter().map(|member| member.element));
        }
        if candidates.len() != 1 {
            return Err(format!("Slice fixture lookup {segments:?} requires one target").into());
        }
        Ok(*candidates.first().expect("one fixture target"))
    }

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
        let documents = publication_documents::select(sources, documents)?;
        let selected: BTreeSet<_> = self
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
        // Every binding stays validated in the context. Package anchors provide
        // lookup environments, without selecting all their unrelated producers.
        Ok(publication_dependencies::subjects_with_context_anchors(
            self.snapshot.model(),
            selected,
            self.identity
                .standard_bindings
                .as_ref()
                .expect("validated bindings")
                .iter()
                .map(|(_, id)| id),
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
