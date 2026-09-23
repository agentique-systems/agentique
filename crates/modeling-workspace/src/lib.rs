//! In-memory authored revisions over authenticated, immutable Gen2 publications.
//!
//! Document edits publish a new Working revision atomically. Validation is a
//! separate checked conversion and never claims full language conformance or
//! execution support. Retained revisions and their borrowed query answers remain
//! independent of later edits. There is no persistence or application adapter.
#![forbid(unsafe_code)]
use agq_kerml_semantics::{Completeness, KerMlQueries, ProducerClosureCertificate, Resolution};
pub use agq_kerml_text::QueryUnavailable;
use agq_kerml_text::library::{CanonicalKermlStandardLibraries, LibraryLoadError};
use agq_kerml_text::sysml::{AuthoredProducerStatus, CanonicalSysmlSystemsLibrary};
use agq_kerml_text::{
    ProjectChange, ProjectDocument, ProjectError, ProjectId, SourceCompilation, SourceDiagnostic,
    SourceInputs, SourceLanguage,
};
use agq_kernel::provenance::{FactKey, SourceOrigin};
use agq_kernel::{
    ConstructionView, DocumentId, ElementId, ElementRecord, ModelView, RevisionId, Snapshot,
};
use agq_sysml_semantics::SysmlQueries;
use std::{collections::BTreeMap, ops::Deref, sync::Arc};

/// Workspace history identity, distinct from kernel and source revision IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectRevisionId(RevisionId);

/// One immutable source/declared/derived/evidence binding. Fields cannot be replaced.
#[derive(Debug)]
pub struct ProjectRevision {
    revision: ProjectRevisionId,
    parent: Option<ProjectRevisionId>,
    compilation: SourceCompilation,
}
impl ProjectRevision {
    /// Authored project identity shared by this history, independent of its path labels.
    pub fn project(&self) -> ProjectId {
        self.compilation.inputs().project()
    }
    pub fn revision(&self) -> ProjectRevisionId {
        self.revision
    }
    pub fn parent(&self) -> Option<ProjectRevisionId> {
        self.parent
    }
    pub fn kernel_revision(&self) -> Option<RevisionId> {
        self.compilation.kernel_revision()
    }
    pub fn root(&self) -> ElementId {
        self.compilation.inputs().root()
    }
    pub fn documents(&self) -> impl Iterator<Item = (&str, &ProjectDocument)> {
        self.compilation.inputs().documents()
    }
    pub fn document_at(&self, path: &str) -> Option<&ProjectDocument> {
        self.compilation.inputs().document_at(path)
    }
    pub fn document(&self, document: DocumentId) -> Option<&ProjectDocument> {
        self.compilation.inputs().document(document)
    }
    pub fn strict_snapshot(&self) -> Option<&Snapshot> {
        self.compilation.strict_snapshot()
    }
    pub fn construction(&self) -> Option<&ConstructionView> {
        self.compilation.construction()
    }
    pub fn semantic_model(&self) -> Option<&ModelView> {
        self.compilation.semantic_model()
    }
    pub fn diagnostics(&self) -> &[SourceDiagnostic] {
        self.compilation.diagnostics()
    }
    pub fn references(&self) -> &[agq_kerml_text::ReferenceAssertion] {
        self.compilation.references()
    }
    pub fn producer_status(&self) -> Option<&AuthoredProducerStatus> {
        self.compilation.producer_status()
    }
    pub fn producer_closure(&self) -> Option<&Arc<ProducerClosureCertificate>> {
        self.compilation.producer_closure()
    }
    pub fn kerml_queries(&self) -> Result<KerMlQueries<'_>, QueryUnavailable> {
        self.compilation.kerml_queries()
    }
    pub fn sysml_queries(&self) -> Result<SysmlQueries<'_>, QueryUnavailable> {
        self.compilation.sysml_queries()
    }
    pub fn source_for_fact(&self, fact: FactKey) -> Option<&SourceOrigin> {
        self.compilation.source_map().get(&fact)
    }
    pub fn element(&self, element: ElementId) -> Option<&ElementRecord> {
        self.semantic_model()?.element(element)
    }
    pub fn accepted_sysml(&self) -> &Arc<CanonicalSysmlSystemsLibrary> {
        self.compilation.inputs().accepted_sysml()
    }
    pub fn accepted_kerml(&self) -> &Arc<CanonicalKermlStandardLibraries> {
        self.accepted_sysml().accepted_kerml()
    }
}

/// Published authored inputs; may retain recovery, unresolved references or partial closure.
#[derive(Debug)]
pub struct WorkingProjectRevision {
    revision: ProjectRevision,
}
impl Deref for WorkingProjectRevision {
    type Target = ProjectRevision;
    fn deref(&self) -> &Self::Target {
        &self.revision
    }
}
impl WorkingProjectRevision {
    /// Check the phase-1 platform contract on this exact immutable revision.
    pub fn validate(self: &Arc<Self>) -> Result<ValidatedProjectRevision, ValidationFailure> {
        let mut findings = Vec::new();
        if !self.diagnostics().is_empty() {
            findings.push(ValidationFinding::Diagnostics);
        }
        if self.strict_snapshot().is_none() {
            findings.push(ValidationFinding::Construction);
        }
        if self
            .documents()
            .any(|(_, document)| document.status() != agq_kerml_text::DocumentStatus::Parsed)
        {
            findings.push(ValidationFinding::Syntax);
        }
        if self
            .producer_status()
            .is_none_or(|status| !status.converged || status.completeness != Completeness::Complete)
        {
            findings.push(ValidationFinding::ProducerClosure);
        }
        if self.semantic_model().is_none_or(|model| {
            self.producer_closure()
                .is_none_or(|certificate| !certificate.is_fully_closed(model))
        }) {
            findings.push(ValidationFinding::Certificate);
        }
        if self.references().iter().any(|reference| {
            reference.resolution.completeness != Completeness::Complete
                || !matches!(reference.resolution.value, Resolution::Resolved(_))
        }) {
            findings.push(ValidationFinding::References);
        }
        match self.kerml_queries() {
            Ok(queries)
                if self.producer_closure().is_some_and(|certificate| {
                    certificate.compatible_context(queries.context())
                }) => {}
            _ => findings.push(ValidationFinding::Context),
        }
        if self.sysml_queries().is_err() && !findings.contains(&ValidationFinding::Context) {
            findings.push(ValidationFinding::Context);
        }
        if findings.is_empty() {
            Ok(ValidatedProjectRevision {
                working: self.clone(),
                contract: PlatformAcceptanceContract::Phase1V1,
            })
        } else {
            Err(ValidationFailure {
                revision: self.revision(),
                findings,
            })
        }
    }
}

/// Accepted platform slice, retaining the exact Working handle that passed all gates.
#[derive(Clone, Debug)]
pub struct ValidatedProjectRevision {
    working: Arc<WorkingProjectRevision>,
    contract: PlatformAcceptanceContract,
}
impl ValidatedProjectRevision {
    /// Contract actually checked when this immutable handle was validated.
    pub fn acceptance_contract(&self) -> PlatformAcceptanceContract {
        self.contract
    }
    pub fn working(&self) -> &Arc<WorkingProjectRevision> {
        &self.working
    }
    pub fn revision(&self) -> ProjectRevisionId {
        self.working.revision()
    }
    pub fn semantic_model(&self) -> &ModelView {
        self.working.semantic_model().expect("validated graph")
    }
    pub fn kerml_queries(&self) -> KerMlQueries<'_> {
        self.working.kerml_queries().expect("validated context")
    }
    pub fn sysml_queries(&self) -> SysmlQueries<'_> {
        self.working
            .sysml_queries()
            .expect("validated SysML context")
    }
}

/// Versioned authored platform acceptance; this does not assert full language conformance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformAcceptanceContract {
    Phase1V1,
}
impl PlatformAcceptanceContract {
    pub fn id(self) -> &'static str {
        match self {
            Self::Phase1V1 => "agentique-modeling-workspace-phase1/1",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationFinding {
    Syntax,
    Diagnostics,
    Construction,
    ProducerClosure,
    Certificate,
    References,
    Context,
}
#[derive(Clone, Debug, thiserror::Error)]
#[error("revision {revision:?} does not satisfy the platform contract: {findings:?}")]
pub struct ValidationFailure {
    pub revision: ProjectRevisionId,
    pub findings: Vec<ValidationFinding>,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace head changed since {0:?}")]
    StaleRevision(ProjectRevisionId),
    #[error("document extension is not .kerml or .sysml: {0}")]
    UnsupportedExtension(String),
    #[error(transparent)]
    Edit(#[from] ProjectError),
    #[error(transparent)]
    Build(#[from] LibraryLoadError),
}

/// One mutable head and immutable retained revisions. Mutation requires exclusive access.
pub struct ProjectWorkspace {
    head: Arc<WorkingProjectRevision>,
    revisions: BTreeMap<ProjectRevisionId, Arc<WorkingProjectRevision>>,
}
impl ProjectWorkspace {
    pub fn open(publication: Arc<CanonicalSysmlSystemsLibrary>) -> Result<Self, WorkspaceError> {
        let inputs = Arc::new(SourceInputs::with_accepted_sysml(publication)?);
        let compilation = inputs.compile(None)?;
        let head = Arc::new(WorkingProjectRevision {
            revision: ProjectRevision {
                revision: ProjectRevisionId(RevisionId::new()),
                parent: None,
                compilation,
            },
        });
        Ok(Self {
            revisions: BTreeMap::from([(head.revision(), head.clone())]),
            head,
        })
    }
    pub fn head(&self) -> &Arc<WorkingProjectRevision> {
        &self.head
    }
    pub fn revision(&self, revision: ProjectRevisionId) -> Option<&Arc<WorkingProjectRevision>> {
        self.revisions.get(&revision)
    }
    pub fn revisions(&self) -> impl Iterator<Item = &Arc<WorkingProjectRevision>> {
        self.revisions.values()
    }
    /// All preparation and compilation precede head/history mutation.
    pub fn apply(
        &mut self,
        expected: ProjectRevisionId,
        changes: impl IntoIterator<Item = ProjectChange>,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        if expected != self.head.revision() {
            return Err(WorkspaceError::StaleRevision(expected));
        }
        let prior = &self.head.compilation;
        let inputs = Arc::new(prior.inputs().apply(changes)?);
        let compilation = inputs.compile(Some(prior))?;
        let revision = Arc::new(WorkingProjectRevision {
            revision: ProjectRevision {
                revision: ProjectRevisionId(RevisionId::new()),
                parent: Some(expected),
                compilation,
            },
        });
        self.revisions.insert(revision.revision(), revision.clone());
        self.head = revision.clone();
        Ok(revision)
    }
    pub fn add_kerml(
        &mut self,
        expected: ProjectRevisionId,
        path: &str,
        source: &str,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        self.apply(
            expected,
            [ProjectChange::Add {
                path: path.into(),
                language: SourceLanguage::KerMl,
                source: source.into(),
            }],
        )
    }
    pub fn add_sysml(
        &mut self,
        expected: ProjectRevisionId,
        path: &str,
        source: &str,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        self.apply(
            expected,
            [ProjectChange::Add {
                path: path.into(),
                language: SourceLanguage::SysMl,
                source: source.into(),
            }],
        )
    }
    pub fn add_document(
        &mut self,
        expected: ProjectRevisionId,
        path: &str,
        source: &str,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        if path.ends_with(".kerml") {
            self.add_kerml(expected, path, source)
        } else if path.ends_with(".sysml") {
            self.add_sysml(expected, path, source)
        } else {
            Err(WorkspaceError::UnsupportedExtension(path.into()))
        }
    }
    pub fn edit_document(
        &mut self,
        expected: ProjectRevisionId,
        document: DocumentId,
        edit: agq_kerml_text::syntax::TextEdit,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        self.apply(expected, [ProjectChange::Edit { document, edit }])
    }
    pub fn remove_document(
        &mut self,
        expected: ProjectRevisionId,
        document: DocumentId,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        self.apply(expected, [ProjectChange::Remove { document }])
    }
}

#[cfg(feature = "verification")]
pub mod testing;
