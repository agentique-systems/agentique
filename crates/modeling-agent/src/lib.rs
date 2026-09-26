//! Agent Fabric boundaries over immutable Gen2 observations and source candidates.
//! Providers may suggest decisions or commands; only explicit authority permits commit.
//! This package supplies no autonomous loop, network provider or execution semantics.
#![forbid(unsafe_code)]

mod commands;
pub mod decision;
use agq_kernel::ElementId;
use agq_modeling_repository::{BranchId, ProjectId, ProjectRevisionId};
use agq_modeling_view::ViewDefinition;
pub use commands::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicit independent capabilities; proposing does not imply validation or commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Authority {
    Read,
    Propose,
    Validate,
    Commit,
}

/// Trusted host policy. Never accept this value from a model/provider response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub actor: String,
    pub permissions: BTreeSet<Authority>,
}
impl AgentPolicy {
    /// Default machine participant may read and prepare changes only.
    pub fn agent(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            permissions: BTreeSet::from([Authority::Read, Authority::Propose]),
        }
    }
    /// Local operator policy supplied by the authenticated Studio host.
    pub fn operator() -> Self {
        Self {
            actor: "human-operator".into(),
            permissions: BTreeSet::from([
                Authority::Read,
                Authority::Propose,
                Authority::Validate,
                Authority::Commit,
            ]),
        }
    }
    pub fn require(&self, permission: Authority) -> Result<(), AgentError> {
        if self.permissions.contains(&permission) {
            Ok(())
        } else {
            Err(AgentError::Denied(permission))
        }
    }
}

/// Exact observation scope retained with every proposal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentContext {
    pub project: ProjectId,
    pub branch: BranchId,
    pub revision: ProjectRevisionId,
    pub selection: Vec<ElementId>,
}

/// An intent is input, never authority to advance a durable branch.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentIntent {
    pub context: AgentContext,
    pub message: String,
}

/// Provider-neutral results can point to a view, decision or reviewed source plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentProposal {
    View {
        revision: ProjectRevisionId,
        definition: ViewDefinition,
    },
    Decision(decision::DecisionResult),
    Commands {
        context: AgentContext,
        commands: Vec<ModelCommand>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("actor lacks {0:?} authority")]
    Denied(Authority),
    #[error("source command: {0}")]
    Invalid(String),
    #[error(transparent)]
    Service(#[from] agq_modeling_service::ServiceError),
}
impl AgentError {
    /// Whether the explicit source task stopped without producing a candidate.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Service(error) if error.is_cancelled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn machine_proposal_cannot_grant_commit_or_validation() {
        let agent = AgentPolicy::agent("architecture-agent");
        assert!(agent.require(Authority::Read).is_ok());
        assert!(agent.require(Authority::Propose).is_ok());
        assert!(agent.require(Authority::Validate).is_err());
        assert!(agent.require(Authority::Commit).is_err());
        assert!(AgentPolicy::operator().require(Authority::Commit).is_ok());
    }
}
