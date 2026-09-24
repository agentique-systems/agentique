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
    production: Option<syntax::production::Document>,
}
impl ProjectDocument {
    fn parse(
        id: DocumentId,
        language: SourceLanguage,
        source: Arc<str>,
        limits: ParseLimits,
        production_frontend: bool,
        sysml_profile: Option<syntax::production::SysmlSyntaxProfile>,
    ) -> Result<Self, SourceError> {
        if source.len() > limits.max_bytes {
            return Err(SourceError::Limit("byte"));
        }
        let production =
            if let Some(profile) = sysml_profile.filter(|_| language == SourceLanguage::SysMl) {
                Some(syntax::production::parse_sysml_with_profile(
                    profile,
                    id,
                    SourceRevisionId::new(),
                    source.clone(),
                    syntax::production::Limits {
                        source: limits,
                        ..Default::default()
                    },
                )?)
            } else if production_frontend {
                Some(syntax::production::parse_with_dialect(
                    match language {
                        SourceLanguage::KerMl => syntax::production::Dialect::KerMl,
                        SourceLanguage::SysMl => syntax::production::Dialect::SysMl,
                    },
                    id,
                    SourceRevisionId::new(),
                    source.clone(),
                    syntax::production::Limits {
                        source: limits,
                        ..Default::default()
                    },
                )?)
            } else {
                None
            };
        let syntax = match language {
            _ if production_frontend => None,
            SourceLanguage::KerMl => Some(syntax::parse(id, source.clone(), limits)?),
            // Never feed SysML into a KerML parser and assert the resulting partial facts.
            SourceLanguage::SysMl => None,
        };
        Ok(Self {
            id,
            revision: production
                .as_ref()
                .map(syntax::production::Document::revision)
                .or_else(|| syntax.as_ref().map(SyntaxDocument::revision))
                .unwrap_or_default(),
            language,
            source,
            syntax,
            production,
        })
    }
    fn edit(&self, edit: &TextEdit, limits: ParseLimits) -> Result<Self, SourceError> {
        if let Some(production) = &self.production {
            let next = production.edit(
                edit,
                syntax::production::Limits {
                    source: limits,
                    ..Default::default()
                },
            )?;
            return Ok(Self {
                id: self.id,
                revision: next.revision(),
                language: self.language,
                source: next.source().into(),
                syntax: None,
                production: Some(next),
            });
        }
        if let Some(syntax) = &self.syntax {
            let next = syntax.edit(edit, limits)?;
            return Ok(Self {
                id: self.id,
                revision: next.revision(),
                language: self.language,
                source: next.source().into(),
                syntax: Some(next),
                production: None,
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
        Self::parse(self.id, self.language, source.into(), limits, false, None)
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
    /// Full shared production arena for the explicitly enabled mixed frontend.
    pub fn production_syntax(&self) -> Option<&syntax::production::Document> {
        self.production.as_ref()
    }
    pub fn status(&self) -> DocumentStatus {
        if let Some(production) = &self.production {
            return if production.is_complete() {
                DocumentStatus::Parsed
            } else {
                DocumentStatus::Recovered
            };
        }
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
    documents: BTreeMap<String, Arc<ProjectDocument>>,
    model: ProjectModel,
    diagnostics: Vec<ProjectDiagnostic>,
}
#[derive(Debug)]
enum ProjectModel {
    KerMl(lowering::LoweredModel),
    SysMl(crate::sysml::SourceModel),
}
impl ProjectModel {
    fn semantic_model(&self) -> &agq_kernel::ModelView {
        match self {
            Self::KerMl(model) => model.snapshot().model(),
            Self::SysMl(model) => model.semantic_model(),
        }
    }
    fn snapshot(&self) -> &Snapshot {
        match self {
            Self::KerMl(model) => model.snapshot(),
            Self::SysMl(model) => model.snapshot(),
        }
    }
    fn root(&self) -> ElementId {
        match self {
            Self::KerMl(model) => model.root(),
            Self::SysMl(model) => model.root(),
        }
    }
    fn diagnostics(&self) -> &[FrontendDiagnostic] {
        match self {
            Self::KerMl(model) => model.diagnostics(),
            Self::SysMl(model) => model.diagnostics(),
        }
    }
    fn references(&self) -> &[ReferenceAssertion] {
        match self {
            Self::KerMl(model) => model.references(),
            Self::SysMl(model) => model.references(),
        }
    }
    fn queries(&self) -> KerMlQueries<'_> {
        match self {
            Self::KerMl(model) => model.queries(),
            Self::SysMl(model) => model.queries(),
        }
    }
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
        self.documents.iter().map(|(p, d)| (p.as_str(), d.as_ref()))
    }
    pub fn document(&self, id: DocumentId) -> Option<&ProjectDocument> {
        self.documents
            .values()
            .find(|d| d.id == id)
            .map(Arc::as_ref)
    }
    pub fn document_at(&self, path: &str) -> Option<&ProjectDocument> {
        self.documents.get(path).map(Arc::as_ref)
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
    /// Effective SysML queries are available only for the accepted two-publication
    /// constructor. Every answer retains closure evidence and completeness.
    pub fn sysml_queries(&self) -> Option<agq_sysml_semantics::SysmlQueries<'_>> {
        match &self.model {
            ProjectModel::SysMl(model) => model.sysml_queries(),
            _ => None,
        }
    }
    /// Shared canonical semantic model, including the derived overlay when the
    /// accepted authored producer path is enabled. `snapshot` remains declared.
    pub fn semantic_model(&self) -> &agq_kernel::ModelView {
        self.model.semantic_model()
    }
    pub fn producer_status(&self) -> Option<&crate::sysml::AuthoredProducerStatus> {
        match &self.model {
            ProjectModel::SysMl(model) => model.producer_status(),
            _ => None,
        }
    }
    pub fn producer_closure(
        &self,
    ) -> Option<&Arc<agq_kerml_semantics::ProducerClosureCertificate>> {
        match &self.model {
            ProjectModel::SysMl(model) => model.producer_closure(),
            _ => None,
        }
    }
    /// Canonical element/slot/occurrence origins for the production frontend.
    pub fn production_source_map(&self) -> Option<&crate::library::LibrarySourceMap> {
        match &self.model {
            ProjectModel::SysMl(model) => Some(model.source_map()),
            _ => None,
        }
    }
    /// Usability of the implemented KerML slice, not full-language validation.
    pub fn is_complete_slice(&self) -> bool {
        self.diagnostics.is_empty()
            && self.model.diagnostics().is_empty()
            && self.producer_status().is_none_or(|status| {
                status.converged
                    && status.completeness == agq_kerml_semantics::Completeness::Complete
            })
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
    #[error(transparent)]
    Construction(#[from] crate::library::LibraryLoadError),
}

/// Source histories and semantic publications only; no persistence/application contracts.
pub struct SourceProject {
    profile: agq_kerml::BaselineProfile,
    id: ProjectId,
    root: ElementId,
    limits: ParseLimits,
    current: Arc<ProjectRevision>,
    history: BTreeMap<RevisionId, Arc<ProjectRevision>>,
    publication: Option<Arc<crate::library::CanonicalKermlStandardLibraries>>,
    production_frontend: bool,
    accepted_sysml: Option<Arc<crate::sysml::AcceptedSourceDependency>>,
}
impl SourceProject {
    pub fn new() -> Result<Self, ProjectError> {
        Self::with_limits(ParseLimits::default())
    }
    pub fn with_limits(limits: ParseLimits) -> Result<Self, ProjectError> {
        Self::with_limits_and_profile(limits, agq_kerml::BaselineProfile::OPERATIONAL)
    }
    /// Select immutable interpretation authority for this project's entire history.
    pub fn with_profile(profile: agq_kerml::BaselineProfile) -> Result<Self, ProjectError> {
        Self::with_limits_and_profile(ParseLimits::default(), profile)
    }
    pub fn with_limits_and_profile(
        limits: ParseLimits,
        profile: agq_kerml::BaselineProfile,
    ) -> Result<Self, ProjectError> {
        Self::create(limits, profile, None, false, None)
    }
    /// Share an accepted immutable publication across independent authored histories.
    /// Its authority profile and canonical library identities cannot be overridden.
    pub fn with_standard_libraries(
        publication: Arc<crate::library::CanonicalKermlStandardLibraries>,
    ) -> Result<Self, ProjectError> {
        Self::create(
            ParseLimits::default(),
            publication.profile(),
            Some(publication),
            false,
            None,
        )
    }
    /// Enable shared lossless SysML/KerML production construction over an accepted
    /// dependency. Current-graph success does not assert SysML producer closure.
    pub fn with_sysml_standard_libraries(
        publication: Arc<crate::library::CanonicalKermlStandardLibraries>,
    ) -> Result<Self, ProjectError> {
        Self::create(
            ParseLimits::default(),
            publication.profile(),
            Some(publication),
            true,
            None,
        )
    }
    /// Enable Operational v2 authored semantics over exact accepted KerML and
    /// Systems publications. Each revision closes its local combined producers;
    /// immutable standard graphs and their accepted bindings are shared.
    pub fn with_accepted_sysml_standard_libraries(
        publication: Arc<crate::sysml::CanonicalSysmlSystemsLibrary>,
    ) -> Result<Self, ProjectError> {
        let dependency = crate::sysml::AcceptedSourceDependency::new(publication)?;
        Self::create(
            ParseLimits::default(),
            dependency.publication.accepted_kerml().profile(),
            Some(dependency.publication.accepted_kerml().clone()),
            true,
            Some(dependency),
        )
    }
    pub fn accepted_sysml_standard_library(
        &self,
    ) -> Option<&Arc<crate::sysml::CanonicalSysmlSystemsLibrary>> {
        self.accepted_sysml
            .as_ref()
            .map(|dependency| &dependency.publication)
    }
    pub fn standard_libraries(
        &self,
    ) -> Option<&Arc<crate::library::CanonicalKermlStandardLibraries>> {
        self.publication.as_ref()
    }
    fn create(
        limits: ParseLimits,
        profile: agq_kerml::BaselineProfile,
        publication: Option<Arc<crate::library::CanonicalKermlStandardLibraries>>,
        production_frontend: bool,
        accepted_sysml: Option<Arc<crate::sysml::AcceptedSourceDependency>>,
    ) -> Result<Self, ProjectError> {
        let id = ProjectId(GeneratorId::new());
        let root = ElementId::new();
        let model = if let Some(dependency) = &accepted_sysml {
            ProjectModel::SysMl(crate::sysml::lower_accepted_source(
                &[],
                None,
                root,
                agq_kernel::provenance::DeclaredOrigin::Generated { generator: id.0 },
                dependency.clone(),
            )?)
        } else if production_frontend {
            ProjectModel::SysMl(crate::sysml::lower_source(
                &[],
                None,
                root,
                agq_kernel::provenance::DeclaredOrigin::Generated { generator: id.0 },
                publication.clone().expect("required accepted dependency"),
            )?)
        } else {
            ProjectModel::KerMl(lowering::lower_project(
                [],
                None,
                root,
                agq_kernel::provenance::DeclaredOrigin::Generated { generator: id.0 },
                false,
                profile,
                publication.clone(),
            )?)
        };
        let current = Arc::new(ProjectRevision {
            project: id,
            documents: BTreeMap::new(),
            model,
            diagnostics: vec![],
        });
        Ok(Self {
            profile,
            id,
            root,
            limits,
            history: BTreeMap::from([(current.revision(), current.clone())]),
            current,
            publication,
            production_frontend,
            accepted_sysml,
        })
    }
    pub fn baseline_profile(&self) -> agq_kerml::BaselineProfile {
        self.profile
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
                    Arc::new(ProjectDocument::parse(
                        DocumentId::new(),
                        language,
                        source.into(),
                        self.limits,
                        self.production_frontend,
                        self.accepted_sysml
                            .as_ref()
                            .map(|dependency| dependency.syntax_profile()),
                    )?),
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
                    documents.insert(path, Arc::new(next));
                }
                ProjectChange::Replace { source, .. } => {
                    let next = ProjectDocument::parse(
                        id,
                        documents[&path].language,
                        source.into(),
                        self.limits,
                        self.production_frontend,
                        self.accepted_sysml
                            .as_ref()
                            .map(|dependency| dependency.syntax_profile()),
                    )?;
                    documents.insert(path, Arc::new(next));
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
        let model = if self.production_frontend {
            let inputs: Vec<_> = documents
                .values()
                .map(|document| {
                    let syntax = document.production.as_ref().expect("production frontend");
                    if !syntax.is_complete() {
                        return Err(crate::library::LibraryLoadError::Syntax(
                            document.id.to_string(),
                        ));
                    }
                    Ok(crate::library::construction::SourceInput {
                        syntax,
                        library: None,
                        sysml: document.language == SourceLanguage::SysMl,
                    })
                })
                .collect::<Result<_, _>>()?;
            let ProjectModel::SysMl(previous) = &self.current.model else {
                unreachable!("immutable frontend")
            };
            let origin = agq_kernel::provenance::DeclaredOrigin::Generated {
                generator: self.id.0,
            };
            ProjectModel::SysMl(if let Some(dependency) = &self.accepted_sysml {
                crate::sysml::lower_accepted_source(
                    &inputs,
                    Some(previous),
                    self.root,
                    origin,
                    dependency.clone(),
                )?
            } else {
                crate::sysml::lower_source(
                    &inputs,
                    Some(previous),
                    self.root,
                    origin,
                    self.publication
                        .clone()
                        .expect("required accepted dependency"),
                )?
            })
        } else {
            ProjectModel::KerMl(lowering::lower_project(
                documents.values().filter_map(|document| document.syntax()),
                Some(match &self.current.model {
                    ProjectModel::KerMl(model) => model,
                    _ => unreachable!("immutable frontend"),
                }),
                self.root,
                agq_kernel::provenance::DeclaredOrigin::Generated {
                    generator: self.id.0,
                },
                documents
                    .values()
                    .any(|d| d.status() == DocumentStatus::FrontendUnavailable),
                self.profile,
                self.publication.clone(),
            )?)
        };
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
    documents: &BTreeMap<String, Arc<ProjectDocument>>,
    path: &str,
) -> Result<(), ProjectError> {
    if path.is_empty() || documents.contains_key(path) {
        Err(ProjectError::InvalidPath(path.into()))
    } else {
        Ok(())
    }
}
