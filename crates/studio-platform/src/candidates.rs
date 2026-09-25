use crate::*;
use agq_modeling_agent::{AgentCandidate, AgentContext, ModelCommand, SourcePreview};
use agq_modeling_repository::CommitReceipt;

const MAX_ACTIVE_CANDIDATES: usize = 8;
const MAX_RETAINED_CANDIDATES: usize = 32;

/// Process-local review handle. The candidate's semantic revision has its own ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CandidateId(uuid::Uuid);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CandidatePhase {
    Working,
    Validated,
    CommitUnresolved,
    Committed,
}

pub(crate) struct RetainedCandidate {
    candidate: AgentCandidate,
    phase: CandidatePhase,
    receipt: Option<CommitReceipt>,
}

/// Explicit preview is not a durable branch head or a claim of validation.
#[derive(Clone, Debug, Serialize)]
pub struct CandidateProjection {
    pub id: CandidateId,
    pub base: RevisionBinding,
    pub phase: CandidatePhase,
    pub projection: ViewProjection,
    pub changes: RevisionDiff,
    pub source_preview: SourcePreview,
}

impl StudioPlatform {
    pub fn propose(
        &mut self,
        context: AgentContext,
        command: ModelCommand,
        definition: &ViewDefinition,
    ) -> Result<CandidateProjection> {
        if self
            .candidates
            .values()
            .filter(|c| c.phase != CandidatePhase::Committed)
            .count()
            >= MAX_ACTIVE_CANDIDATES
        {
            return Err(PlatformError::Invalid(
                "Review or cancel a retained candidate first".into(),
            ));
        }
        if self.candidates.len() >= MAX_RETAINED_CANDIDATES {
            let expired = self
                .candidates
                .iter()
                .find_map(|(id, c)| (c.phase == CandidatePhase::Committed).then_some(*id));
            if let Some(id) = expired {
                self.candidates.remove(&id);
            }
        }
        let candidate = agq_modeling_agent::propose(&self.service, &self.policy, context, command)?;
        let id = CandidateId(uuid::Uuid::new_v4());
        self.candidates.insert(
            id,
            RetainedCandidate {
                candidate,
                phase: CandidatePhase::Working,
                receipt: None,
            },
        );
        match self.candidate(id, definition) {
            Ok(projection) => Ok(projection),
            Err(error) => {
                self.candidates.remove(&id);
                Err(error)
            }
        }
    }

    fn retained(&self, id: CandidateId) -> Result<&RetainedCandidate> {
        self.policy.require(Authority::Read)?;
        self.candidates
            .get(&id)
            .ok_or_else(|| PlatformError::Invalid("Candidate no longer available".into()))
    }

    pub fn candidate(
        &self,
        id: CandidateId,
        definition: &ViewDefinition,
    ) -> Result<CandidateProjection> {
        let stored = self.retained(id)?;
        let base = RevisionBinding {
            project: stored.candidate.context.project,
            revision: stored.candidate.context.revision,
        };
        let before = self.bound(base)?;
        let after = stored.candidate.prepared().bound_revision();
        Ok(CandidateProjection {
            id,
            base,
            phase: stored.phase,
            projection: agq_modeling_view::project(after.revision(), definition)?,
            changes: agq_modeling_service::revision_diff(&before, &after)?,
            source_preview: stored.candidate.source_preview.clone(),
        })
    }

    pub fn inspect_candidate(
        &self,
        id: CandidateId,
        element: ElementId,
    ) -> Result<ElementInspector> {
        Ok(agq_modeling_view::inspect(
            self.retained(id)?.candidate.prepared().revision(),
            element,
        )?)
    }

    pub fn explain_candidate(
        &self,
        id: CandidateId,
        element: ElementId,
    ) -> Result<ExplanationProjection> {
        Ok(agq_modeling_view::explain(
            self.retained(id)?.candidate.prepared().revision(),
            element,
        )?)
    }

    pub fn validate(
        &mut self,
        id: CandidateId,
        definition: &ViewDefinition,
    ) -> Result<CandidateProjection> {
        self.policy.require(Authority::Validate)?;
        let stored = self
            .candidates
            .get_mut(&id)
            .ok_or_else(|| PlatformError::Invalid("Candidate no longer available".into()))?;
        if !matches!(
            stored.phase,
            CandidatePhase::Working | CandidatePhase::Validated
        ) {
            return Err(PlatformError::Invalid(
                "Commit already attempted; retry the same commit".into(),
            ));
        }
        stored.candidate.validate(&self.policy)?;
        stored.phase = CandidatePhase::Validated;
        self.candidate(id, definition)
    }

    /// Repeated calls reuse the exact service operation; no local head changes precede durability.
    pub fn commit(&mut self, id: CandidateId) -> Result<CommitReceipt> {
        self.policy.require(Authority::Commit)?;
        let stored = self
            .candidates
            .get_mut(&id)
            .ok_or_else(|| PlatformError::Invalid("Candidate no longer available".into()))?;
        if let Some(receipt) = &stored.receipt {
            return Ok(receipt.clone());
        }
        if stored
            .candidate
            .prepared()
            .bound_revision()
            .validated()
            .is_none()
        {
            return Err(PlatformError::Invalid(
                "Validate this candidate before operator commit".into(),
            ));
        }
        stored.phase = CandidatePhase::CommitUnresolved;
        let receipt = stored.candidate.commit(&self.service, &self.policy)?;
        stored.receipt = Some(receipt.clone());
        stored.phase = CandidatePhase::Committed;
        Ok(receipt)
    }

    /// Cancel only an uncommitted candidate. Durable history uses a new revision.
    pub fn cancel(&mut self, id: CandidateId) -> Result<()> {
        self.policy.require(Authority::Propose)?;
        let phase = self.retained(id)?.phase;
        if matches!(
            phase,
            CandidatePhase::CommitUnresolved | CandidatePhase::Committed
        ) {
            return Err(PlatformError::Invalid(
                "Commit has been attempted; reconcile its durable receipt".into(),
            ));
        }
        self.candidates.remove(&id);
        Ok(())
    }
}
