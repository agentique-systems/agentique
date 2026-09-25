//! Toolkit-independent, in-process Studio application boundary.
//!
//! Run expensive calls on a worker. The shell receives revision-bound view DTOs,
//! never kernel storage or SQLite access. Candidates retain the service's exact
//! durable operation for compare-and-swap and acknowledgement retries.
#![forbid(unsafe_code)]

mod bootstrap;
mod candidates;
mod reader;
mod seed;

pub use bootstrap::*;
pub use candidates::*;
pub use reader::StudioRevisionReader;
pub use seed::seed_agentique;

use agq_kernel::ElementId;
use agq_modeling_agent::{AgentPolicy, Authority};
use agq_modeling_repository::{Branch, Project, ProjectId, ProjectRevisionId, RevisionManifest};
use agq_modeling_service::{BoundRevision, ModelingService, RevisionDiff, RevisionSelector};
use agq_modeling_view::{ElementInspector, ExplanationProjection, ViewDefinition, ViewProjection};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

/// Every observation names an immutable revision, never an implicit branch head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionBinding {
    pub project: ProjectId,
    pub revision: ProjectRevisionId,
}

/// Durable history; switching selection establishes a new [`RevisionBinding`].
#[derive(Clone, Debug, Serialize)]
pub struct ProjectHistory {
    pub project: Project,
    pub branches: Vec<Branch>,
    pub revisions: Vec<RevisionManifest>,
}

/// Exact authored source location, useful to open an auxiliary source surface.
#[derive(Clone, Debug, Serialize)]
pub struct SourceProjection {
    pub binding: RevisionBinding,
    pub element: ElementId,
    pub path: String,
    pub source: String,
    pub start: u64,
    pub end: u64,
}

/// Both sides keep their revision identity and are independently projected.
#[derive(Clone, Debug, Serialize)]
pub struct ComparisonProjection {
    pub project: ProjectId,
    pub before: ViewProjection,
    pub after: ViewProjection,
    pub changes: RevisionDiff,
}

/// An unavailable answer stays unavailable; no fixture or empty success is substituted.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error(transparent)]
    Service(#[from] agq_modeling_service::ServiceError),
    #[error(transparent)]
    Repository(#[from] agq_modeling_repository::RepositoryError),
    #[error(transparent)]
    Agent(#[from] agq_modeling_agent::AgentError),
    #[error(transparent)]
    View(#[from] agq_modeling_view::ViewError),
    #[error(transparent)]
    Runtime(#[from] agq_runtime_publications::RuntimeError),
    #[error("Studio platform: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// Trusted local host capability. Provider responses cannot replace its policy.
/// Own this facade on a background worker and move only projections to the UI.
pub struct StudioPlatform {
    service: Arc<ModelingService>,
    policy: AgentPolicy,
    candidates: BTreeMap<CandidateId, RetainedCandidate>,
}

impl StudioPlatform {
    pub fn new(service: Arc<ModelingService>, policy: AgentPolicy) -> Self {
        Self {
            service,
            policy,
            candidates: BTreeMap::new(),
        }
    }

    pub fn projects(&self) -> Result<Vec<Project>> {
        self.policy.require(Authority::Read)?;
        Ok(self.service.repository().list_projects()?)
    }

    pub fn history(&self, project: ProjectId) -> Result<ProjectHistory> {
        self.policy.require(Authority::Read)?;
        let project = self.service.repository().get_project(project)?;
        Ok(ProjectHistory {
            branches: self.service.repository().list_branches(project.id)?,
            revisions: self.service.repository().list_revisions(project.id)?,
            project,
        })
    }

    fn bound(&self, binding: RevisionBinding) -> Result<BoundRevision> {
        self.policy.require(Authority::Read)?;
        Ok(self.service.resolve(
            binding.project,
            RevisionSelector::Revision(binding.revision),
        )?)
    }

    pub fn project(
        &self,
        binding: RevisionBinding,
        definition: &ViewDefinition,
    ) -> Result<ViewProjection> {
        Ok(agq_modeling_view::project(
            self.bound(binding)?.revision(),
            definition,
        )?)
    }

    pub fn inspect(
        &self,
        binding: RevisionBinding,
        element: ElementId,
    ) -> Result<ElementInspector> {
        Ok(agq_modeling_view::inspect(
            self.bound(binding)?.revision(),
            element,
        )?)
    }

    pub fn explain(
        &self,
        binding: RevisionBinding,
        element: ElementId,
    ) -> Result<ExplanationProjection> {
        Ok(agq_modeling_view::explain(
            self.bound(binding)?.revision(),
            element,
        )?)
    }

    pub fn source(&self, binding: RevisionBinding, element: ElementId) -> Result<SourceProjection> {
        let revision = self.bound(binding)?;
        source_projection(binding, &revision, element)
    }

    pub fn compare(
        &self,
        project: ProjectId,
        from: ProjectRevisionId,
        to: ProjectRevisionId,
        definition: &ViewDefinition,
    ) -> Result<ComparisonProjection> {
        let before = self.bound(RevisionBinding {
            project,
            revision: from,
        })?;
        let after = self.bound(RevisionBinding {
            project,
            revision: to,
        })?;
        Ok(ComparisonProjection {
            project,
            before: agq_modeling_view::project(before.revision(), definition)?,
            after: agq_modeling_view::project(after.revision(), definition)?,
            changes: agq_modeling_service::revision_diff(&before, &after)?,
        })
    }

    /// Bounded dependency neighborhood. Reached package owners are context
    /// anchors, not expansion through unrelated siblings; this is not an impact proof.
    pub fn dependencies(
        &self,
        binding: RevisionBinding,
        selection: ElementId,
        families: Vec<agq_modeling_view::RelationshipFamily>,
        depth: u8,
        include_standard_library: bool,
    ) -> Result<ViewProjection> {
        self.project(
            binding,
            &ViewDefinition::dependency_neighborhood(
                selection,
                families,
                depth,
                include_standard_library,
            ),
        )
    }
}

fn source_projection(
    binding: RevisionBinding,
    revision: &BoundRevision,
    element: ElementId,
) -> Result<SourceProjection> {
    let origin = revision
        .source_origin(element)
        .ok_or_else(|| PlatformError::Invalid("Element has no authored source".into()))?;
    let (path, document) = revision
        .revision()
        .documents()
        .find(|(_, doc)| doc.id() == origin.document)
        .ok_or_else(|| PlatformError::Invalid("Source is outside the authored project".into()))?;
    Ok(SourceProjection {
        binding,
        element,
        path: path.into(),
        source: document.source().into(),
        start: origin.range.start(),
        end: origin.range.end(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_and_results_can_cross_the_native_worker_boundary() {
        fn send<T: Send>() {}
        send::<StudioPlatform>();
        send::<ViewProjection>();
        send::<ElementInspector>();
        send::<ExplanationProjection>();
        send::<ProjectHistory>();
        send::<ComparisonProjection>();
    }
}
