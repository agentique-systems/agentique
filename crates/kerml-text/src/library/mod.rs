//! Canonical construction from verified immutable KerML source documents.
//!
//! Construction is explicitly unpublished until structural references resolve.
//! It is not a validated standard-library binding or an alternative model store.
mod construction;
mod vocabulary;

use agq_kerml_semantics::QualifiedName;
use agq_kerml_syntax::production;
use agq_kernel::{
    ElementId, MetaclassId, PropertyId, Snapshot,
    provenance::{FactKey, SourceOrigin},
};
use agq_standard_libraries::{LibraryDocument, LibraryLanguage, VerifiedLibrarySet};
use std::collections::BTreeMap;

/// A reference is an unlowered assertion until its target is established semantically.
#[derive(Clone, Debug)]
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
    candidate: agq_kernel::ConstructionView,
    source_map: LibrarySourceMap,
    roots: Vec<ElementId>,
    references: Vec<PendingLibraryReference>,
}
impl LibraryDraft {
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
        let roots: std::collections::BTreeSet<_> = self.roots.iter().copied().collect();
        let availability = roots.iter().map(|r| (*r, roots.clone())).collect();
        let context = SemanticContext::for_construction(&self.candidate, Default::default(), pins)
            .and_then(|c| c.with_available_roots(availability))
            .map_err(|e| LibraryLoadError::Interpretation(format!("{e:?}")))?
            .with_standard_bindings(&self.roots, semantic)
            .map_err(|e| LibraryLoadError::Interpretation(format!("Binding validation: {e:?}")))?;
        Ok(KerMlQueries::new(context))
    }
    pub fn candidate(&self) -> &agq_kernel::ConstructionView {
        &self.candidate
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
}

struct Input {
    source: LibraryDocument,
    syntax: production::Document,
}

/// Parse the complete verified KerML corpus and lower declarations atomically.
/// Unresolved relationships remain explicit assertions, never fabricated endpoints.
pub fn lower_declarations(sources: &VerifiedLibrarySet) -> Result<LibraryDraft, LibraryLoadError> {
    construction::construct(&parse_sources(sources)?, &BTreeMap::new())
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
    mut progress: impl FnMut(usize, usize, usize),
) -> Result<LibraryDraft, LibraryLoadError> {
    let inputs = parse_sources(sources)?;
    let mut resolved = BTreeMap::new();
    let mut previous = std::collections::BTreeSet::new();
    let mut round = 0;
    loop {
        if !previous.insert(resolved.clone()) {
            return Err(LibraryLoadError::Interpretation(
                "Resolution refinement cycle".into(),
            ));
        }
        let draft = construction::construct(&inputs, &resolved)?;
        progress(round, resolved.len(), draft.candidate.obligations().len());
        let queries = draft.queries(sources)?;
        let mut next = BTreeMap::new();
        for reference in &draft.references {
            let result = queries.lookup_relationship_target(
                reference.relationship,
                reference.property,
                &reference.name,
            );
            if let [member] = result.value.as_slice() {
                let target = if reference.membership_target {
                    member.membership
                } else {
                    member.element
                };
                let class = draft
                    .candidate
                    .model()
                    .element(target)
                    .expect("query endpoint")
                    .metaclass();
                if draft
                    .candidate
                    .model()
                    .registry()
                    .is_subtype(class, reference.expected)
                    .map_err(agq_kernel::ModelError::from)?
                {
                    next.insert((reference.relationship, reference.property), target);
                }
            }
        }
        if next == resolved {
            return Ok(draft);
        }
        resolved = next;
        round += 1;
    }
}
