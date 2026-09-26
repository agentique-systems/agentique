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
    lifecycle: CandidateLifecycle,
}
impl RetainedCandidate {
    pub(crate) fn phase(&self) -> CandidatePhase {
        self.lifecycle.phase
    }
}

/// Review state is separate from the semantic candidate so failure/acknowledgement
/// transitions can be checked independently of an expensive accepted publication.
struct CandidateLifecycle {
    phase: CandidatePhase,
    receipt: Option<CommitReceipt>,
}

impl CandidateLifecycle {
    fn working() -> Self {
        Self {
            phase: CandidatePhase::Working,
            receipt: None,
        }
    }

    fn validate_with(
        &mut self,
        policy: &AgentPolicy,
        validate: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        policy.require(Authority::Validate)?;
        if !matches!(
            self.phase,
            CandidatePhase::Working | CandidatePhase::Validated
        ) {
            return Err(PlatformError::Invalid(
                "Commit already attempted; retry the same commit".into(),
            ));
        }
        validate()?;
        self.phase = CandidatePhase::Validated;
        Ok(())
    }

    fn commit_with(
        &mut self,
        policy: &AgentPolicy,
        validated: bool,
        commit: impl FnOnce() -> Result<CommitReceipt>,
    ) -> Result<CommitReceipt> {
        policy.require(Authority::Commit)?;
        if let Some(receipt) = &self.receipt {
            return Ok(receipt.clone());
        }
        if !validated {
            return Err(PlatformError::Invalid(
                "Validate this candidate before operator commit".into(),
            ));
        }
        self.phase = CandidatePhase::CommitUnresolved;
        match commit() {
            Ok(receipt) => {
                self.receipt = Some(receipt.clone());
                self.phase = CandidatePhase::Committed;
                Ok(receipt)
            }
            Err(error) => {
                // An explicit atomic CAS refusal proves this candidate was not
                // committed. It can be cancelled/reprepared. Other errors keep
                // the exact operation for acknowledgement reconciliation.
                if matches!(
                    &error,
                    PlatformError::Agent(agq_modeling_agent::AgentError::Service(
                        agq_modeling_service::ServiceError::Repository(
                            agq_modeling_repository::RepositoryError::Conflict { .. }
                        )
                    ))
                ) {
                    self.phase = CandidatePhase::Validated;
                }
                Err(error)
            }
        }
    }

    fn cancel(&self, policy: &AgentPolicy) -> Result<()> {
        policy.require(Authority::Read)?;
        policy.require(Authority::Propose)?;
        if matches!(
            self.phase,
            CandidatePhase::CommitUnresolved | CandidatePhase::Committed
        ) {
            return Err(PlatformError::Invalid(
                "Commit has been attempted; reconcile its durable receipt".into(),
            ));
        }
        Ok(())
    }
}

/// Plan eviction without mutating review handles before source preparation succeeds.
fn admission(
    entries: impl Iterator<Item = (CandidateId, CandidatePhase)>,
) -> Result<Option<CandidateId>> {
    let mut total = 0;
    let mut active = 0;
    let mut completed = None;
    for (id, phase) in entries {
        total += 1;
        if phase == CandidatePhase::Committed {
            completed.get_or_insert(id);
        } else {
            active += 1;
        }
    }
    if active >= MAX_ACTIVE_CANDIDATES {
        return Err(PlatformError::Invalid(
            "Review or cancel a retained candidate first".into(),
        ));
    }
    Ok((total >= MAX_RETAINED_CANDIDATES)
        .then_some(completed)
        .flatten())
}

/// Explicit preview is not a durable branch head or a claim of validation.
#[derive(Clone, Debug, Serialize)]
pub struct CandidateProjection {
    pub id: CandidateId,
    pub base: RevisionBinding,
    pub phase: CandidatePhase,
    pub intent: String,
    pub actor: String,
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
        self.propose_controlled(context, command, definition, &CompilationControl::default())
    }

    /// Prepare an unpublished edit with explicit cooperative cancellation.
    /// Cancellation never advances a durable branch or retains a partial candidate.
    pub fn propose_controlled(
        &mut self,
        context: AgentContext,
        command: ModelCommand,
        definition: &ViewDefinition,
        control: &CompilationControl,
    ) -> Result<CandidateProjection> {
        self.policy.require(Authority::Read)?;
        self.policy.require(Authority::Propose)?;
        control
            .check()
            .map_err(agq_modeling_service::ServiceError::from)?;
        let expired = admission(
            self.candidates
                .iter()
                .map(|(id, c)| (*id, c.lifecycle.phase)),
        )?;
        let candidate = agq_modeling_agent::propose_controlled(
            &self.service,
            &self.policy,
            context,
            command,
            control,
        )?;
        control
            .enter(CompilationStage::PreparingReview)
            .map_err(agq_modeling_service::ServiceError::from)?;
        let id = CandidateId(uuid::Uuid::new_v4());
        self.candidates.insert(
            id,
            RetainedCandidate {
                candidate,
                lifecycle: CandidateLifecycle::working(),
            },
        );
        match self.candidate(id, definition) {
            Ok(projection) => {
                if let Err(cancelled) = control.check() {
                    self.candidates.remove(&id);
                    return Err(agq_modeling_service::ServiceError::from(cancelled).into());
                }
                if let Some(id) = expired {
                    self.candidates.remove(&id);
                }
                Ok(projection)
            }
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
            phase: stored.lifecycle.phase,
            intent: stored
                .candidate
                .prepared()
                .request()
                .candidate
                .manifest
                .metadata
                .name
                .clone()
                .unwrap_or_else(|| "Model change".into()),
            actor: stored.candidate.actor.clone(),
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

    /// Source provenance is read from the exact candidate, including added elements.
    pub fn source_candidate(
        &self,
        id: CandidateId,
        element: ElementId,
    ) -> Result<SourceProjection> {
        let stored = self.retained(id)?;
        let revision = stored.candidate.prepared().bound_revision();
        source_projection(
            RevisionBinding {
                project: stored.candidate.context.project,
                revision: revision.revision().revision(),
            },
            &revision,
            element,
        )
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
        stored.lifecycle.validate_with(&self.policy, || {
            Ok(stored.candidate.validate(&self.policy)?)
        })?;
        self.candidate(id, definition)
    }

    /// Repeated calls reuse the exact service operation; no local head changes precede durability.
    pub fn commit(&mut self, id: CandidateId) -> Result<CommitReceipt> {
        self.policy.require(Authority::Commit)?;
        let stored = self
            .candidates
            .get_mut(&id)
            .ok_or_else(|| PlatformError::Invalid("Candidate no longer available".into()))?;
        let validated = stored
            .candidate
            .prepared()
            .bound_revision()
            .validated()
            .is_some();
        stored.lifecycle.commit_with(&self.policy, validated, || {
            Ok(stored.candidate.commit(&self.service, &self.policy)?)
        })
    }

    /// Cancel only an uncommitted candidate. Durable history uses a new revision.
    pub fn cancel(&mut self, id: CandidateId) -> Result<()> {
        self.retained(id)?.lifecycle.cancel(&self.policy)?;
        self.candidates.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_modeling_repository::{BranchId, OperationId, RepositoryError};

    #[test]
    fn candidate_pressure_never_evicts_an_unresolved_commit_or_unreviewed_preview() {
        let unknown = CandidateId(uuid::Uuid::new_v4());
        let mut entries = vec![(unknown, CandidatePhase::CommitUnresolved)];
        entries
            .extend((0..6).map(|_| (CandidateId(uuid::Uuid::new_v4()), CandidatePhase::Working)));
        assert_eq!(admission(entries.iter().copied()).unwrap(), None);
        let completed = CandidateId(uuid::Uuid::new_v4());
        entries.push((completed, CandidatePhase::Committed));
        entries.extend(
            (0..24).map(|_| (CandidateId(uuid::Uuid::new_v4()), CandidatePhase::Committed)),
        );
        assert_eq!(entries.len(), MAX_RETAINED_CANDIDATES);
        assert_eq!(admission(entries.iter().copied()).unwrap(), Some(completed));
        entries.push((CandidateId(uuid::Uuid::new_v4()), CandidatePhase::Validated));
        assert!(admission(entries.iter().copied()).is_err());
        assert_eq!(
            entries[0].0, unknown,
            "admission never mutates existing review state"
        );
    }

    fn receipt() -> CommitReceipt {
        CommitReceipt {
            operation_id: OperationId::new(),
            branch_id: BranchId::new(),
            revision_id: ProjectRevisionId::new(),
            replayed: false,
        }
    }

    fn storage_error(error: RepositoryError) -> PlatformError {
        agq_modeling_agent::AgentError::Service(agq_modeling_service::ServiceError::Repository(
            error,
        ))
        .into()
    }

    #[test]
    fn machine_policy_never_calls_validation_or_commit_even_for_cached_receipts() {
        let operator = AgentPolicy::operator();
        let agent = AgentPolicy::agent("embedded-decision-agent");
        let mut state = CandidateLifecycle::working();
        assert!(
            state
                .validate_with(&agent, || panic!("unauthorized validator ran"))
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::Working);
        state.validate_with(&operator, || Ok(())).unwrap();
        assert!(
            state
                .commit_with(&agent, true, || panic!("unauthorized persistence ran"))
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::Validated);
        state
            .commit_with(&operator, true, || Ok(receipt()))
            .unwrap();
        assert!(
            state
                .commit_with(&agent, true, || panic!("cached receipt bypassed policy"))
                .is_err()
        );
    }

    #[test]
    fn validation_failure_retains_working_preview_and_allows_cancel() {
        let operator = AgentPolicy::operator();
        let mut state = CandidateLifecycle::working();
        assert!(
            state
                .validate_with(&operator, || Err(PlatformError::Invalid(
                    "validation findings".into()
                )))
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::Working);
        assert!(state.cancel(&operator).is_ok());
        assert!(
            state
                .commit_with(&operator, false, || panic!("Working preview persisted"))
                .is_err()
        );
    }

    #[test]
    fn presentation_validated_state_cannot_replace_a_validated_revision() {
        let operator = AgentPolicy::operator();
        let mut state = CandidateLifecycle::working();
        state.validate_with(&operator, || Ok(())).unwrap();
        assert!(
            state
                .commit_with(&operator, false, || panic!(
                    "missing validation receipt accepted"
                ))
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::Validated);
    }

    #[test]
    fn lost_acknowledgement_requires_exact_retry_then_replays_without_persistence() {
        let operator = AgentPolicy::operator();
        let mut state = CandidateLifecycle::working();
        state.validate_with(&operator, || Ok(())).unwrap();
        let mut expected = receipt();
        let operation = expected.operation_id;
        let mut attempts = 0;
        assert!(
            state
                .commit_with(&operator, true, || {
                    attempts += 1;
                    Err(storage_error(RepositoryError::OutcomeUnknown(operation)))
                })
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::CommitUnresolved);
        assert!(state.cancel(&operator).is_err());
        assert!(
            state
                .validate_with(&operator, || panic!("unresolved candidate replaced"))
                .is_err()
        );
        expected.replayed = true;
        let restored = state
            .commit_with(&operator, true, || {
                attempts += 1;
                Ok(expected.clone())
            })
            .unwrap();
        assert_eq!(restored.operation_id, operation);
        assert_eq!(state.phase, CandidatePhase::Committed);
        assert_eq!(
            state
                .commit_with(&operator, true, || panic!(
                    "acknowledged operation persisted again"
                ))
                .unwrap(),
            restored
        );
        assert_eq!(attempts, 2);
        assert!(state.cancel(&operator).is_err());
    }

    #[test]
    fn explicit_cas_refusal_allows_cancel_but_unknown_storage_failure_does_not() {
        let operator = AgentPolicy::operator();
        let mut state = CandidateLifecycle::working();
        state.validate_with(&operator, || Ok(())).unwrap();
        assert!(
            state
                .commit_with(&operator, true, || Err(storage_error(
                    RepositoryError::Conflict {
                        expected: ProjectRevisionId::new(),
                        actual: ProjectRevisionId::new(),
                    }
                )))
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::Validated);
        assert!(state.cancel(&operator).is_ok());
        assert!(
            state
                .commit_with(&operator, true, || Err(storage_error(
                    RepositoryError::Storage("connection lost".into())
                )))
                .is_err()
        );
        assert_eq!(state.phase, CandidatePhase::CommitUnresolved);
        assert!(state.cancel(&operator).is_err());
    }
}
