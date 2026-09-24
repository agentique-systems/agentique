//! In-memory authored revisions over authenticated, immutable Gen2 publications.
//!
//! Document edits publish a new Working revision atomically. Validation is a
//! separate checked conversion and never claims full language conformance or
//! execution support. Retained revisions and their borrowed query answers remain
//! independent of later edits. Source checkpoints retain identity for an outer repository.
#![forbid(unsafe_code)]
mod checkpoint;
use agq_kerml_semantics::{Completeness, KerMlQueries, ProducerClosureCertificate, Resolution};
pub use agq_kerml_text::QueryUnavailable;
use agq_kerml_text::library::{CanonicalKermlStandardLibraries, LibraryLoadError};
use agq_kerml_text::sysml::{AuthoredProducerStatus, CanonicalSysmlSystemsLibrary};
use agq_kerml_text::{
    ProjectChange, ProjectDocument, ProjectError, ProjectId, SourceCompilation, SourceDiagnostic,
    SourceEffectiveAudit, SourceInputs, SourceLanguage,
};
use agq_kernel::provenance::{FactKey, SourceOrigin};
use agq_kernel::{
    ConstructionView, DocumentId, ElementId, ElementRecord, ModelView, RevisionId, Snapshot,
};
use agq_sysml_semantics::SysmlQueries;
pub use checkpoint::*;
use std::{collections::BTreeMap, ops::Deref, sync::Arc};

/// Workspace history identity, distinct from kernel and source revision IDs.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct ProjectRevisionId(RevisionId);
impl ProjectRevisionId {
    /// Construct an externally allocated portable project revision identity.
    pub const fn from_u128(value: u128) -> Self {
        Self(RevisionId::from_u128(value))
    }
    /// Obtain the portable identity representation.
    pub const fn as_u128(self) -> u128 {
        self.0.as_u128()
    }
    /// Allocate a workspace-history identity independently of kernel revision identity.
    pub fn new() -> Self {
        Self(RevisionId::new())
    }
}
impl Default for ProjectRevisionId {
    fn default() -> Self {
        Self::new()
    }
}
impl std::fmt::Display for ProjectRevisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::str::FromStr for ProjectRevisionId {
    type Err = uuid::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::parse_str(value).map(|id| Self(RevisionId::from_u128(id.as_u128())))
    }
}

/// One immutable source/declared/derived/evidence binding. Fields cannot be replaced.
#[derive(Debug)]
pub struct ProjectRevision {
    revision: ProjectRevisionId,
    parent: Option<ProjectRevisionId>,
    compilation: SourceCompilation,
}
impl ProjectRevision {
    /// Observable authored work; counts never establish semantic acceptance.
    pub fn compilation_work(&self) -> &agq_kerml_text::CompilationWork {
        self.compilation.work()
    }
    /// Exact source identities and declared fact changes since the parent.
    pub fn edit_frontier(&self) -> &agq_kerml_text::SourceEditFrontier {
        self.compilation.edit_frontier()
    }
    /// Authored project identity shared by this history, independent of its path labels.
    pub fn project(&self) -> ProjectId {
        self.compilation.inputs().project()
    }
    /// Immutable workspace-history identity, distinct from the kernel revision.
    pub fn revision(&self) -> ProjectRevisionId {
        self.revision
    }
    /// Sole parent revision, or none for an initial workspace.
    pub fn parent(&self) -> Option<ProjectRevisionId> {
        self.parent
    }
    /// Kernel identity of the compiled frontier, when available.
    pub fn kernel_revision(&self) -> Option<RevisionId> {
        self.compilation.kernel_revision()
    }
    /// Canonical root of the authored project.
    pub fn root(&self) -> ElementId {
        self.compilation.inputs().root()
    }
    /// Authored documents with their current path labels and stable identities.
    pub fn documents(&self) -> impl Iterator<Item = (&str, &ProjectDocument)> {
        self.compilation.inputs().documents()
    }
    /// Find an authored document by its path label in this revision.
    pub fn document_at(&self, path: &str) -> Option<&ProjectDocument> {
        self.compilation.inputs().document_at(path)
    }
    /// Find an authored document by stable identity.
    pub fn document(&self, document: DocumentId) -> Option<&ProjectDocument> {
        self.compilation.inputs().document(document)
    }
    /// Strict declared snapshot, if construction completed successfully.
    pub fn strict_snapshot(&self) -> Option<&Snapshot> {
        self.compilation.strict_snapshot()
    }
    /// Inspect an unfinished construction frontier when no strict snapshot exists.
    pub fn construction(&self) -> Option<&ConstructionView> {
        self.compilation.construction()
    }
    /// Canonical semantic frontier available for this revision.
    pub fn semantic_model(&self) -> Option<&ModelView> {
        self.compilation.semantic_model()
    }
    /// Source and construction diagnostics bound to this revision.
    pub fn diagnostics(&self) -> &[SourceDiagnostic] {
        self.compilation.diagnostics()
    }
    /// Applicable effective queries over this revision's entire local graph.
    pub fn effective_audit(&self) -> Option<&SourceEffectiveAudit> {
        self.compilation.effective_audit()
    }
    /// Authored reference assertions and their exact resolution evidence.
    pub fn references(&self) -> &[agq_kerml_text::ReferenceAssertion] {
        self.compilation.references()
    }
    /// Observed authored producer convergence and completeness.
    pub fn producer_status(&self) -> Option<&AuthoredProducerStatus> {
        self.compilation.producer_status()
    }
    /// Closure certificate for this exact semantic frontier, when available.
    pub fn producer_closure(&self) -> Option<&Arc<ProducerClosureCertificate>> {
        self.compilation.producer_closure()
    }
    /// Borrow KerML queries bound to this immutable revision and context.
    pub fn kerml_queries(&self) -> Result<KerMlQueries<'_>, QueryUnavailable> {
        self.compilation.kerml_queries()
    }
    /// Borrow SysML queries bound to this immutable revision and context.
    pub fn sysml_queries(&self) -> Result<SysmlQueries<'_>, QueryUnavailable> {
        self.compilation.sysml_queries()
    }
    /// Authored source origin for a canonical fact, when one is recorded.
    pub fn source_for_fact(&self, fact: FactKey) -> Option<&SourceOrigin> {
        self.compilation.source_map().get(&fact)
    }
    /// Look up a canonical record in the available semantic frontier.
    pub fn element(&self, element: ElementId) -> Option<&ElementRecord> {
        self.semantic_model()?.element(element)
    }
    /// Shared immutable accepted Systems publication used by this revision.
    pub fn accepted_sysml(&self) -> &Arc<CanonicalSysmlSystemsLibrary> {
        self.compilation.inputs().accepted_sysml()
    }
    /// Shared immutable accepted KerML publication underlying Systems.
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
        match self.sysml_queries() {
            Ok(queries) => {
                if self.effective_audit().is_none_or(|audit| {
                    audit.context() != queries.context() || !audit.report().findings.is_empty()
                }) {
                    findings.push(ValidationFinding::EffectiveAudit);
                }
            }
            Err(_) => {
                if !findings.contains(&ValidationFinding::Context) {
                    findings.push(ValidationFinding::Context);
                }
            }
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
    /// Exact Working handle that passed the acceptance transition.
    pub fn working(&self) -> &Arc<WorkingProjectRevision> {
        &self.working
    }
    /// Immutable workspace-history identity, distinct from the kernel revision.
    pub fn revision(&self) -> ProjectRevisionId {
        self.working.revision()
    }
    /// Canonical semantic graph guaranteed by successful validation.
    pub fn semantic_model(&self) -> &ModelView {
        self.working.semantic_model().expect("validated graph")
    }
    /// KerML queries over the validated semantic context.
    pub fn kerml_queries(&self) -> KerMlQueries<'_> {
        self.working.kerml_queries().expect("validated context")
    }
    /// SysML queries over the validated semantic context.
    pub fn sysml_queries(&self) -> SysmlQueries<'_> {
        self.working
            .sysml_queries()
            .expect("validated SysML context")
    }
}

/// Versioned authored platform acceptance; this does not assert full language conformance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PlatformAcceptanceContract {
    /// Adopted Phase 1 platform slice: strict construction, closure and effective audit.
    Phase1V1,
}
impl PlatformAcceptanceContract {
    /// Stable external identifier for the adopted acceptance contract.
    pub fn id(self) -> &'static str {
        match self {
            Self::Phase1V1 => "agentique-modeling-workspace-phase1/1",
        }
    }
}

/// A failed obligation of the versioned platform acceptance contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationFinding {
    /// At least one document did not parse completely.
    Syntax,
    /// Source or construction diagnostics remain.
    Diagnostics,
    /// No strict declared snapshot is available.
    Construction,
    /// Authored producers did not converge completely.
    ProducerClosure,
    /// The certificate does not establish full closure of this graph.
    Certificate,
    /// An authored reference is unresolved or incomplete.
    References,
    /// The semantic query context is unavailable or incompatible.
    Context,
    /// Missing, context-mismatched or failing applicable effective query audit.
    EffectiveAudit,
}
/// Failed acceptance of one immutable Working revision.
#[derive(Clone, Debug, thiserror::Error)]
#[error("revision {revision:?} does not satisfy the platform contract: {findings:?}")]
pub struct ValidationFailure {
    /// Revision on which these findings were observed.
    pub revision: ProjectRevisionId,
    /// Every failed platform obligation discovered during validation.
    pub findings: Vec<ValidationFinding>,
}

/// Failures that prevent construction or acknowledgement of a workspace revision.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// The supplied expected head is no longer current.
    #[error("workspace head changed since {0:?}")]
    StaleRevision(ProjectRevisionId),
    /// The path does not select a supported source language.
    #[error("document extension is not .kerml or .sysml: {0}")]
    UnsupportedExtension(String),
    /// Candidate project or parent does not match this workspace head.
    #[error("candidate does not continue this project head")]
    ForeignCandidate,
    /// Source edit or document identity reconciliation failed.
    #[error(transparent)]
    Edit(#[from] ProjectError),
    /// Semantic construction or publication attachment failed.
    #[error(transparent)]
    Build(#[from] LibraryLoadError),
}

/// One mutable head and immutable retained revisions. Mutation requires exclusive access.
pub struct ProjectWorkspace {
    head: Arc<WorkingProjectRevision>,
    revisions: BTreeMap<ProjectRevisionId, Arc<WorkingProjectRevision>>,
}
impl ProjectWorkspace {
    /// Open an independent workspace at an immutable retained revision.
    pub fn from_revision(head: Arc<WorkingProjectRevision>) -> Self {
        Self {
            revisions: BTreeMap::from([(head.revision(), head.clone())]),
            head,
        }
    }
    /// Construct an empty workspace using a shared accepted Systems publication.
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
    /// Current acknowledged in-memory head.
    pub fn head(&self) -> &Arc<WorkingProjectRevision> {
        &self.head
    }
    /// Look up an immutable revision retained by this workspace.
    pub fn revision(&self, revision: ProjectRevisionId) -> Option<&Arc<WorkingProjectRevision>> {
        self.revisions.get(&revision)
    }
    /// Retained immutable revisions in identity order, not chronological order.
    pub fn revisions(&self) -> impl Iterator<Item = &Arc<WorkingProjectRevision>> {
        self.revisions.values()
    }
    /// Construct a candidate without acknowledging or moving the workspace head.
    pub fn prepare(
        &self,
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
        Ok(revision)
    }
    /// Acknowledge a candidate after the caller's durable commit succeeds.
    /// This operation performs no persistence and rejects foreign histories.
    pub fn acknowledge(
        &mut self,
        expected: ProjectRevisionId,
        candidate: Arc<WorkingProjectRevision>,
    ) -> Result<(), WorkspaceError> {
        if expected != self.head.revision() {
            return Err(WorkspaceError::StaleRevision(expected));
        }
        if candidate.project() != self.head.project() || candidate.parent() != Some(expected) {
            return Err(WorkspaceError::ForeignCandidate);
        }
        self.revisions
            .insert(candidate.revision(), candidate.clone());
        self.head = candidate;
        Ok(())
    }
    /// In-memory convenience; durable callers use prepare, commit, acknowledge.
    pub fn apply(
        &mut self,
        expected: ProjectRevisionId,
        changes: impl IntoIterator<Item = ProjectChange>,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        let candidate = self.prepare(expected, changes)?;
        self.acknowledge(expected, candidate.clone())?;
        Ok(candidate)
    }
    /// Construct and acknowledge an in-memory revision adding a KerML document.
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
    /// Construct and acknowledge an in-memory revision adding a SysML document.
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
    /// Select KerML or SysML from the path extension and add an in-memory revision.
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
    /// Apply a source byte-range edit and acknowledge its in-memory revision.
    pub fn edit_document(
        &mut self,
        expected: ProjectRevisionId,
        document: DocumentId,
        edit: agq_kerml_text::syntax::TextEdit,
    ) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
        self.apply(expected, [ProjectChange::Edit { document, edit }])
    }
    /// Remove a document by identity and acknowledge its in-memory revision.
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
