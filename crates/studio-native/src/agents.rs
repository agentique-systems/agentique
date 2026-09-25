//! Provider-neutral spatial agent presentation. No model mutation or commit authority.
use agq_kernel::ElementId;
use agq_modeling_agent::decision::{DecisionAnswer, DecisionResult, DecisionState, DecisionValue};
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{OverlayKind, SceneOverlay, SceneTarget};
use std::collections::BTreeSet;

pub struct AgentPresentation {
    pub observation: DecisionState,
    pub decision: DecisionResult,
    pub overlay: SceneOverlay,
}

/// Explicit illustrative distribution; it is never represented as calibrated confidence.
pub fn dependency_view(
    revision: ProjectRevisionId,
    targets: &BTreeSet<ElementId>,
) -> AgentPresentation {
    AgentPresentation {
        observation: DecisionState {
            intent: "Show dependencies".into(),
            revision: revision.to_string(),
            selected_elements: targets.iter().map(ToString::to_string).collect(),
        },
        decision: DecisionResult {
            provider: "native-illustrative-decision".into(),
            revision: revision.to_string(),
            answers: vec![DecisionAnswer {
                question: "lens".into(),
                value: DecisionValue::Choice("Graph".into()),
                probabilities: vec![
                    ("Architecture".into(), 0.17),
                    ("Graph".into(), 0.72),
                    ("Requirements".into(), 0.08),
                    ("History".into(), 0.03),
                ],
                confidence: None,
            }],
        },
        overlay: SceneOverlay {
            id: "dependency-focus".into(),
            revision_id: revision,
            targets: targets.iter().copied().map(SceneTarget::Node).collect(),
            label: "Dependency context".into(),
            kind: OverlayKind::Focus,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_decision_and_overlay_retain_observation_revision_without_commit_capability() {
        let revision = ProjectRevisionId::from_u128(72);
        let presentation = dependency_view(revision, &BTreeSet::from([ElementId::from_u128(3)]));
        assert_eq!(presentation.overlay.revision_id, revision);
        assert_eq!(
            presentation.decision.revision,
            presentation.observation.revision
        );
        assert!(presentation.decision.answers[0].confidence.is_none());
        assert!(
            (presentation.decision.answers[0]
                .probabilities
                .iter()
                .map(|(_, score)| score)
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1e-12
        );
        assert_eq!(presentation.overlay.targets.len(), 1);
    }
}
