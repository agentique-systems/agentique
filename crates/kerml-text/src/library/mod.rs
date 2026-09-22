//! Canonical construction from verified immutable KerML source documents.
//!
//! Construction is explicitly unpublished until structural references resolve.
//! It is not a validated standard-library binding or an alternative model store.
mod binding_manifest;
pub(crate) mod construction;
pub mod corrections;
mod publication;
pub(crate) mod refinement;
mod vocabulary;
pub use publication::*;
pub use refinement::{ReferenceRefinementRound, ReferenceRefinementStrategy};

use agq_kerml_semantics::QualifiedName;
use agq_kerml_syntax::production;
use agq_kernel::{
    ElementId, MetaclassId, PropertyId, Snapshot,
    provenance::{FactKey, SourceOrigin},
};
use agq_standard_libraries::{LibraryDocument, LibraryLanguage, VerifiedLibrarySet};
use std::collections::BTreeMap;

/// A reference is an unlowered assertion until its target is established semantically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingLibraryReference {
    pub relationship: ElementId,
    pub property: PropertyId,
    pub expected: MetaclassId,
    pub name: QualifiedName,
    pub membership_target: bool,
    pub executable_expression: bool,
    pub origin: SourceOrigin,
}

/// Source evidence is separate from each canonical record/slot's StandardLibrary origin.
pub type LibrarySourceMap = BTreeMap<FactKey, SourceOrigin>;

/// An unpublished construction. Callers must not treat this as validated libraries.
#[derive(Debug)]
pub struct LibraryDraft {
    base: Snapshot,
    candidate: std::sync::Arc<agq_kernel::ConstructionView>,
    semantic_candidate: Option<agq_kernel::derived::ConstructionOverlay>,
    producer_closure: Option<std::sync::Arc<agq_kerml_semantics::ProducerClosureCertificate>>,
    profile: agq_kerml::BaselineProfile,
    source_map: LibrarySourceMap,
    roots: Vec<ElementId>,
    references: Vec<PendingLibraryReference>,
    superseded_references: Vec<PendingLibraryReference>,
}
impl LibraryDraft {
    /// Revalidate every source record and occurrence with ordinary strict kernel
    /// construction. This discharges storage obligations only; it does not assert
    /// semantic producer closure, reference resolution or canonical publication.
    pub fn strict_snapshot(&self) -> Result<Snapshot, LibraryLoadError> {
        use agq_kernel::provenance::Origin;
        let empty = self.base.clone();
        let mut changes = empty.change_set();
        for record in self.candidate.model().elements() {
            if empty.is_dependency_element(record.id()) {
                continue;
            }
            let Origin::Declared(origin) = record.origin() else {
                return Err(LibraryLoadError::Interpretation(
                    "Derived source record".into(),
                ));
            };
            changes.create(record.id(), record.metaclass(), origin.clone());
            for (property, slot) in record.slots() {
                let Origin::Declared(origin) = slot.origin() else {
                    return Err(LibraryLoadError::Interpretation(
                        "Derived source slot".into(),
                    ));
                };
                changes.set(record.id(), property, slot.value().clone(), origin.clone());
            }
        }
        for occurrence in self.candidate.model().association_occurrences() {
            if empty
                .model()
                .association_occurrence(occurrence.id())
                .is_some()
            {
                continue;
            }
            let origin = occurrence.declared_origin().ok_or_else(|| {
                LibraryLoadError::Interpretation("Derived source occurrence".into())
            })?;
            changes.link(
                occurrence.id(),
                occurrence.association(),
                occurrence.ends().clone(),
                occurrence.positions().clone(),
                origin.clone(),
            );
        }
        Ok(empty.apply(&changes)?)
    }

    /// Queries over this unpublished candidate with exact archive pins and
    /// validated canonical anchor declarations. This is not publication acceptance.
    pub fn queries(
        &self,
        sources: &VerifiedLibrarySet,
    ) -> Result<agq_kerml_semantics::KerMlQueries<'_>, LibraryLoadError> {
        use agq_kerml_semantics::{KerMlQueries, LibraryPin, SemanticContext};
        let revisions: BTreeMap<_, _> = sources
            .documents()
            .map(|d| (d.document(), d.revision()))
            .collect();
        if self
            .source_map
            .values()
            .any(|s| revisions.get(&s.document) != Some(&s.revision))
        {
            return Err(LibraryLoadError::Interpretation(
                "Library sources do not identify this construction".into(),
            ));
        }
        let mut pins = std::collections::BTreeSet::new();
        for library in sources.libraries().values().filter(|l| {
            l.documents()
                .iter()
                .any(|d| d.language() == LibraryLanguage::KerMl)
        }) {
            let mut sha256 = [0; 32];
            for (index, byte) in sha256.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&library.archive_sha256()[index * 2..index * 2 + 2], 16)
                    .expect("verified digest");
            }
            pins.insert(LibraryPin {
                name: library.resource().into(),
                sha256,
            });
        }
        let semantic = sources
            .libraries()
            .values()
            .find(|l| {
                l.resource() == "https://www.omg.org/spec/KerML/20250201/Semantic-Library.kpar"
            })
            .ok_or_else(|| {
                LibraryLoadError::Interpretation("Missing exact Semantic Library artifact".into())
            })?
            .id();
        let library_set = agq_kerml_semantics::LibrarySetIdentity {
            artifacts: agq_kerml_semantics::StandardLibraryArtifact::ALL
                .into_iter()
                .map(|artifact| {
                    let library = sources
                        .libraries()
                        .values()
                        .find(|l| l.resource() == artifact.resource())
                        .expect("verified KerML dependency closure");
                    (artifact, library.id())
                })
                .collect(),
            pins: pins.clone(),
        };
        let roots: std::collections::BTreeSet<_> = self.roots.iter().copied().collect();
        let availability = roots.iter().map(|r| (*r, roots.clone())).collect();
        let context = SemanticContext::for_construction(
            &self.candidate,
            agq_kerml_semantics::SemanticOptions {
                baseline_profile: self.profile,
                ..Default::default()
            },
            pins,
        )
        .and_then(|c| c.with_available_roots(availability))
        .map_err(|e| LibraryLoadError::Interpretation(format!("{e:?}")))?
        .with_standard_bindings(&self.roots, &library_set)
        .map_err(|e| LibraryLoadError::Interpretation(format!("Binding validation: {e:?}")))?
        .with_formal_constraint_targets(&self.roots, semantic);
        Ok(KerMlQueries::new(context))
    }
    pub fn baseline_profile(&self) -> agq_kerml::BaselineProfile {
        self.profile
    }
    pub fn candidate(&self) -> &agq_kernel::ConstructionView {
        &self.candidate
    }
    pub(crate) fn candidate_shared(&self) -> &std::sync::Arc<agq_kernel::ConstructionView> {
        &self.candidate
    }
    pub(crate) fn set_semantic_candidate(
        &mut self,
        overlay: agq_kernel::derived::ConstructionOverlay,
    ) {
        assert!(std::sync::Arc::ptr_eq(
            overlay.declared_shared(),
            &self.candidate
        ));
        self.semantic_candidate = Some(overlay);
        self.producer_closure = None;
    }
    /// Unpublished producer facts, when reference refinement required semantic
    /// inheritance. This overlay is not used by strict declared construction.
    pub fn semantic_candidate(&self) -> Option<&agq_kernel::derived::ConstructionOverlay> {
        self.semantic_candidate.as_ref()
    }
    /// Scheduler evidence belongs to the exact immutable semantic candidate,
    /// separately from its canonical records and source construction.
    pub fn producer_closure(
        &self,
    ) -> Option<&std::sync::Arc<agq_kerml_semantics::ProducerClosureCertificate>> {
        self.producer_closure.as_ref()
    }
    pub(crate) fn set_producer_closure(
        &mut self,
        certificate: Option<std::sync::Arc<agq_kerml_semantics::ProducerClosureCertificate>>,
    ) {
        self.producer_closure = certificate;
    }
    pub(crate) fn take_semantic_candidate(
        &mut self,
    ) -> Option<agq_kernel::derived::ConstructionOverlay> {
        self.producer_closure = None;
        self.semantic_candidate.take()
    }
    pub(crate) fn reference_model(&self) -> &agq_kernel::ModelView {
        self.semantic_candidate
            .as_ref()
            .map_or_else(|| self.candidate.model(), |overlay| overlay.model())
    }
    pub fn source_map(&self) -> &LibrarySourceMap {
        &self.source_map
    }
    pub fn roots(&self) -> &[ElementId] {
        &self.roots
    }
    pub fn references(&self) -> &[PendingLibraryReference] {
        &self.references
    }
    /// Original source assertions explicitly replaced by a reviewed correction.
    /// These remain inspectable, but do not assert operational endpoints.
    pub fn superseded_references(&self) -> &[PendingLibraryReference] {
        &self.superseded_references
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LibraryLoadError {
    #[error(transparent)]
    Source(#[from] agq_kerml_syntax::SourceError),
    #[error(transparent)]
    Library(#[from] agq_standard_libraries::LibraryError),
    #[error(transparent)]
    Kernel(#[from] agq_kernel::ModelError),
    #[error("library syntax requires recovery: {0}")]
    Syntax(String),
    #[error("unsupported canonical grammar interpretation: {0}")]
    Interpretation(String),
    #[error("semantic producer closure failed: {0}")]
    ProducerClosure(#[from] agq_kerml_semantics::PublicationOverlayError),
}

struct Input {
    source: LibraryDocument,
    syntax: production::Document,
}

/// Parse the complete verified KerML corpus and lower declarations atomically.
/// Unresolved relationships remain explicit assertions, never fabricated endpoints.
pub fn lower_declarations(sources: &VerifiedLibrarySet) -> Result<LibraryDraft, LibraryLoadError> {
    lower_declarations_with_profile(sources, agq_kerml::BaselineProfile::OPERATIONAL)
}

/// Lower immutable pinned declarations, then apply an explicitly selected model correction.
pub fn lower_declarations_with_profile(
    sources: &VerifiedLibrarySet,
    profile: agq_kerml::BaselineProfile,
) -> Result<LibraryDraft, LibraryLoadError> {
    let draft = construction::construct(&parse_sources(sources)?, &BTreeMap::new(), profile)?;
    apply_profile(draft, sources)
}

fn apply_profile(
    draft: LibraryDraft,
    sources: &VerifiedLibrarySet,
) -> Result<LibraryDraft, LibraryLoadError> {
    if draft.profile.corrects_library_content() {
        corrections::OperationalLibraryPatchSet::reviewed()?.apply(draft, sources)
    } else {
        Ok(draft)
    }
}

fn parse_sources(sources: &VerifiedLibrarySet) -> Result<Vec<Input>, LibraryLoadError> {
    let mut inputs = vec![];
    for source in sources
        .documents()
        .filter(|d| d.language() == LibraryLanguage::KerMl)
    {
        let syntax = production::parse(
            source.document(),
            source.revision(),
            source.source(),
            production::Limits::default(),
        )?;
        if !syntax.is_complete() {
            return Err(LibraryLoadError::Syntax(source.path().into()));
        }
        inputs.push(Input {
            source: source.clone(),
            syntax,
        });
    }
    inputs.sort_by(|a, b| a.source.path().cmp(b.source.path()));
    Ok(inputs)
}

/// Refine pending canonical endpoints using semantic query candidates. The
/// returned view remains unpublished, even when a provisional endpoint exists.
/// Complete publication must revalidate every resolution and structural obligation.
pub fn refine_declarations(
    sources: &VerifiedLibrarySet,
    progress: impl FnMut(usize, usize, usize),
) -> Result<LibraryDraft, LibraryLoadError> {
    refine_declarations_with_profile(sources, agq_kerml::BaselineProfile::OPERATIONAL, progress)
}

/// Reproduce construction under an explicitly selected authority profile.
pub fn refine_declarations_with_profile(
    sources: &VerifiedLibrarySet,
    profile: agq_kerml::BaselineProfile,
    mut progress: impl FnMut(usize, usize, usize),
) -> Result<LibraryDraft, LibraryLoadError> {
    refine_declarations_with_report(
        sources,
        profile,
        ReferenceRefinementStrategy::DependencyDriven,
        |round| {
            progress(
                round.round,
                round.input_endpoints,
                round.structural_obligations,
            )
        },
    )
}

/// Refine unpublished endpoints with explicit per-frontier work and phase timing.
/// The full-scan strategy remains a reference oracle for bounded fixtures. Both
/// strategies preserve provisional candidates; accepted publication independently
/// requires every mandatory reference to resolve completely to its stored target.
pub fn refine_declarations_with_report(
    sources: &VerifiedLibrarySet,
    profile: agq_kerml::BaselineProfile,
    strategy: ReferenceRefinementStrategy,
    progress: impl FnMut(&ReferenceRefinementRound),
) -> Result<LibraryDraft, LibraryLoadError> {
    let inputs = parse_sources(sources)?;
    refinement::refine(
        |resolved| {
            apply_profile(
                construction::construct(&inputs, resolved, profile)?,
                sources,
            )
        },
        |draft| Ok(draft.queries(sources)?.status_queries()),
        strategy,
        progress,
    )
}
