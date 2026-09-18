//! In-memory multi-document construction over the same lowering pipeline as Document.
use crate::{FrontendDiagnostic, ReferenceAssertion, lowering, syntax};
use agq_kerml_semantics::KerMlQueries;
use agq_kernel::{DocumentId, ElementId, GeneratorId, RevisionId, Snapshot, SourceRevisionId};
use std::{collections::BTreeMap, sync::Arc};
use syntax::{ParseLimits, SourceError, SyntaxDocument, SyntaxStatus, TextEdit};

/// Project identity is allocated independently of paths, document and semantic IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectId(GeneratorId);

/// Explicit document language; an extension never determines semantic identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLanguage {
    KerMl,
    SysMl,
}

/// Frontend capability is distinct from successfully parsing an empty document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentStatus {
    Parsed,
    Recovered,
    FrontendUnavailable,
}

/// One immutable document revision. Paths are project-local labels, never IDs.
#[derive(Clone, Debug)]
pub struct ProjectDocument {
    id: DocumentId,
    revision: SourceRevisionId,
    language: SourceLanguage,
    source: Arc<str>,
    syntax: Option<SyntaxDocument>,
}
impl ProjectDocument {
    fn parse(
        id: DocumentId,
        language: SourceLanguage,
        source: Arc<str>,
        limits: ParseLimits,
    ) -> Result<Self, SourceError> {
        if source.len() > limits.max_bytes {
            return Err(SourceError::Limit("byte"));
        }
        let syntax = match language {
            SourceLanguage::KerMl => Some(syntax::parse(id, source.clone(), limits)?),
            // Never feed SysML into a KerML parser and assert the resulting partial facts.
            SourceLanguage::SysMl => None,
        };
        Ok(Self {
            id,
            revision: syntax
                .as_ref()
                .map_or_else(SourceRevisionId::new, SyntaxDocument::revision),
            language,
            source,
            syntax,
        })
    }
    fn edit(&self, edit: &TextEdit, limits: ParseLimits) -> Result<Self, SourceError> {
        if let Some(syntax) = &self.syntax {
            let next = syntax.edit(edit, limits)?;
            return Ok(Self {
                id: self.id,
                revision: next.revision(),
                language: self.language,
                source: next.source().into(),
                syntax: Some(next),
            });
        }
        let range = edit.range.start() as usize..edit.range.end() as usize;
        if self.source.get(range.clone()).is_none() {
            return Err(SourceError::InvalidEdit);
        }
        let length = self.source.len() - range.len();
        if length > limits.max_bytes
            || edit.replacement.len() > limits.max_bytes.saturating_sub(length)
        {
            return Err(SourceError::Limit("byte"));
        }
        let mut source = self.source.to_string();
        source.replace_range(range, &edit.replacement);
        Self::parse(self.id, self.language, source.into(), limits)
    }
    pub fn id(&self) -> DocumentId {
        self.id
    }
    pub fn revision(&self) -> SourceRevisionId {
        self.revision
    }
    pub fn language(&self) -> SourceLanguage {
        self.language
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn syntax(&self) -> Option<&SyntaxDocument> {
        self.syntax.as_ref()
    }
    pub fn status(&self) -> DocumentStatus {
        match self.syntax.as_ref().map(SyntaxDocument::status) {
            Some(SyntaxStatus::Success) => DocumentStatus::Parsed,
            Some(SyntaxStatus::Recovered) => DocumentStatus::Recovered,
            None => DocumentStatus::FrontendUnavailable,
        }
    }
}

/// Project-level failures remain separate from syntax and language diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectDiagnostic {
    UnsupportedFrontend {
        document: DocumentId,
        language: SourceLanguage,
    },
}

/// A single coherent publication, including exact source inputs and one kernel snapshot.
#[derive(Debug)]
pub struct ProjectRevision {
    project: ProjectId,
    documents: BTreeMap<String, ProjectDocument>,
    model: lowering::LoweredModel,
    diagnostics: Vec<ProjectDiagnostic>,
}
impl ProjectRevision {
    pub fn project(&self) -> ProjectId {
        self.project
    }
    pub fn revision(&self) -> RevisionId {
        self.model.snapshot().revision()
    }
    pub fn snapshot(&self) -> &Snapshot {
        self.model.snapshot()
    }
    pub fn root(&self) -> ElementId {
        self.model.root()
    }
    /// Documents are ordered by exact project-local path, independent of insertion order.
    pub fn documents(&self) -> impl Iterator<Item = (&str, &ProjectDocument)> {
        self.documents.iter().map(|(p, d)| (p.as_str(), d))
    }
    pub fn document(&self, id: DocumentId) -> Option<&ProjectDocument> {
        self.documents.values().find(|d| d.id == id)
    }
    pub fn document_at(&self, path: &str) -> Option<&ProjectDocument> {
        self.documents.get(path)
    }
    pub fn diagnostics(&self) -> &[ProjectDiagnostic] {
        &self.diagnostics
    }
    pub fn semantic_diagnostics(&self) -> &[FrontendDiagnostic] {
        self.model.diagnostics()
    }
    pub fn references(&self) -> &[ReferenceAssertion] {
        self.model.references()
    }
    pub fn queries(&self) -> KerMlQueries<'_> {
        self.model.queries()
    }
    /// Usability of the implemented KerML slice, not full-language validation.
    pub fn is_complete_slice(&self) -> bool {
        self.diagnostics.is_empty()
            && self.model.diagnostics().is_empty()
            && self
                .documents
                .values()
                .all(|d| d.status() == DocumentStatus::Parsed)
    }
}

/// An explicit batch of source changes, applied against one exact project revision.
#[derive(Clone, Debug)]
pub enum ProjectChange {
    Add {
        path: String,
        language: SourceLanguage,
        source: String,
    },
    Edit {
        document: DocumentId,
        edit: TextEdit,
    },
    Replace {
        document: DocumentId,
        source: String,
    },
    Remove {
        document: DocumentId,
    },
    /// Rename the document label; this does not move declarations between documents.
    RenameDocument {
        document: DocumentId,
        path: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("project publication changed since revision {0}")]
    StaleRevision(RevisionId),
    #[error("missing project document {0}")]
    MissingDocument(DocumentId),
    #[error("duplicate or empty project path {0:?}")]
    InvalidPath(String),
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Kernel(#[from] agq_kernel::ModelError),
}

/// Source histories and semantic publications only; no persistence/application contracts.
pub struct SourceProject {
    id: ProjectId,
    root: ElementId,
    limits: ParseLimits,
    current: Arc<ProjectRevision>,
    history: BTreeMap<RevisionId, Arc<ProjectRevision>>,
}
impl SourceProject {
    pub fn new() -> Result<Self, ProjectError> {
        Self::with_limits(ParseLimits::default())
    }
    pub fn with_limits(limits: ParseLimits) -> Result<Self, ProjectError> {
        let id = ProjectId(GeneratorId::new());
        let root = ElementId::new();
        let model = lowering::lower_project(
            [],
            None,
            root,
            agq_kernel::provenance::DeclaredOrigin::Generated { generator: id.0 },
            false,
        )?;
        let current = Arc::new(ProjectRevision {
            project: id,
            documents: BTreeMap::new(),
            model,
            diagnostics: vec![],
        });
        Ok(Self {
            id,
            root,
            limits,
            history: BTreeMap::from([(current.revision(), current.clone())]),
            current,
        })
    }
    pub fn current(&self) -> &Arc<ProjectRevision> {
        &self.current
    }
    pub fn revision(&self, id: RevisionId) -> Option<&Arc<ProjectRevision>> {
        self.history.get(&id)
    }
    /// Rebuild all dependencies against the complete candidate input, then publish once.
    /// No failed batch changes document heads, model history or retired identities.
    pub fn apply(
        &mut self,
        base: RevisionId,
        changes: impl IntoIterator<Item = ProjectChange>,
    ) -> Result<Arc<ProjectRevision>, ProjectError> {
        if base != self.current.revision() {
            return Err(ProjectError::StaleRevision(base));
        }
        let mut documents = self.current.documents.clone();
        for change in changes {
            if let ProjectChange::Add {
                path,
                language,
                source,
            } = change
            {
                check_path(&documents, &path)?;
                documents.insert(
                    path,
                    ProjectDocument::parse(
                        DocumentId::new(),
                        language,
                        source.into(),
                        self.limits,
                    )?,
                );
                continue;
            }
            let id = match &change {
                ProjectChange::Edit { document, .. }
                | ProjectChange::Replace { document, .. }
                | ProjectChange::Remove { document }
                | ProjectChange::RenameDocument { document, .. } => *document,
                ProjectChange::Add { .. } => unreachable!(),
            };
            let path = documents
                .iter()
                .find(|(_, d)| d.id == id)
                .map(|(p, _)| p.clone())
                .ok_or(ProjectError::MissingDocument(id))?;
            match change {
                ProjectChange::Edit { edit, .. } => {
                    let next = documents[&path].edit(&edit, self.limits)?;
                    documents.insert(path, next);
                }
                ProjectChange::Replace { source, .. } => {
                    let next = ProjectDocument::parse(
                        id,
                        documents[&path].language,
                        source.into(),
                        self.limits,
                    )?;
                    documents.insert(path, next);
                }
                ProjectChange::Remove { .. } => {
                    documents.remove(&path);
                }
                ProjectChange::RenameDocument { path: next, .. } => {
                    if path != next {
                        check_path(&documents, &next)?;
                        let doc = documents.remove(&path).expect("checked document");
                        documents.insert(next, doc);
                    }
                }
                ProjectChange::Add { .. } => unreachable!(),
            }
        }
        let model = lowering::lower_project(
            documents.values().filter_map(ProjectDocument::syntax),
            Some(&self.current.model),
            self.root,
            agq_kernel::provenance::DeclaredOrigin::Generated {
                generator: self.id.0,
            },
            documents
                .values()
                .any(|d| d.status() == DocumentStatus::FrontendUnavailable),
        )?;
        let diagnostics = documents
            .values()
            .filter(|d| d.status() == DocumentStatus::FrontendUnavailable)
            .map(|d| ProjectDiagnostic::UnsupportedFrontend {
                document: d.id,
                language: d.language,
            })
            .collect();
        let next = Arc::new(ProjectRevision {
            project: self.id,
            documents,
            model,
            diagnostics,
        });
        self.history.insert(next.revision(), next.clone());
        self.current = next.clone();
        Ok(next)
    }
}
fn check_path(
    documents: &BTreeMap<String, ProjectDocument>,
    path: &str,
) -> Result<(), ProjectError> {
    if path.is_empty() || documents.contains_key(path) {
        Err(ProjectError::InvalidPath(path.into()))
    } else {
        Ok(())
    }
}
