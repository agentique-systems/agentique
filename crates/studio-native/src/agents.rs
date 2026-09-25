//! Provider-neutral spatial agent presentation. No model mutation or commit authority.
use agq_kernel::ElementId;
use agq_modeling_agent::decision::{DecisionAnswer, DecisionResult, DecisionState, DecisionValue};
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{OverlayKind, SceneOverlay, SceneTarget};
use std::collections::BTreeSet;

/// A temporary query has a return address in the same immutable model context.
/// Candidate phase and authority are deliberately never copied back from this.
pub struct AgentReturn {
    context: crate::bridge::WorkContext,
    projection: agq_modeling_view::ViewProjection,
    candidate_views: Option<(
        agq_modeling_view::ViewProjection,
        agq_modeling_view::ViewProjection,
    )>,
    compare_before: Option<agq_modeling_view::ViewProjection>,
    comparison: crate::app::ComparisonMode,
    pub world: crate::navigation::World,
    pub focus: Option<ElementId>,
    pub camera: agq_studio_scene::Camera2D,
    pub selection: crate::selection::Selection,
    layout: agq_studio_scene::LayoutMemory,
    collapsed: BTreeSet<ElementId>,
    expanded: Option<BTreeSet<ElementId>>,
    families: BTreeSet<agq_modeling_view::RelationshipFamily>,
    include_standard: bool,
}

/// A query observation retains the requested subject even when selection changes.
pub struct DependencyActivity {
    pub revision: ProjectRevisionId,
    pub root: ElementId,
    pub root_name: String,
    pub complete: bool,
    pub result_count: usize,
    pub error: Option<String>,
    pub fixture: bool,
}

impl crate::app::StudioApp {
    pub fn remember_agent_return(&mut self) {
        if self.show_agent && self.agent_return.is_some() {
            return;
        }
        self.agent_return = Some(AgentReturn {
            context: self.work_context(),
            projection: self.projection.clone(),
            candidate_views: self
                .candidate
                .as_ref()
                .map(|candidate| (candidate.before.clone(), candidate.after.clone())),
            compare_before: self.compare_before.clone(),
            comparison: self.comparison,
            world: self.world,
            focus: self.focus,
            camera: self.camera,
            selection: self.selection.clone(),
            layout: self.layout.clone(),
            collapsed: self.collapsed.clone(),
            expanded: self.expanded.clone(),
            families: self.families.clone(),
            include_standard: self.include_standard,
        });
    }

    pub fn dismiss_agent_view(&mut self) {
        self.cancel_revision_navigation();
        self.show_agent = false;
        self.dependencies = None;
        self.agent_activity = None;
        // Fence a query which may still be queued on the semantic worker.
        self.scene_request = 0;
        if let Some(previous) = self.agent_return.take().filter(|previous| {
            previous.context.matches(&self.work_context())
                && previous.projection.revision_id == self.projection.revision_id
                && previous
                    .candidate_views
                    .as_ref()
                    .map(|(_, after)| after.revision_id)
                    == self
                        .candidate
                        .as_ref()
                        .map(|candidate| candidate.after.revision_id)
        }) {
            self.projection = previous.projection;
            if let Some((before, after)) = previous.candidate_views
                && let Some(candidate) = &mut self.candidate
            {
                candidate.before = before;
                candidate.after = after;
            }
            self.compare_before = previous.compare_before;
            self.comparison = previous.comparison;
            self.world = previous.world;
            self.focus = previous.focus;
            self.camera_target = Some(previous.camera);
            self.selection = previous.selection;
            self.layouts.insert(self.layout_world, self.layout.clone());
            self.layout = previous.layout;
            self.layout_world = self.world;
            self.collapsed = previous.collapsed;
            self.expanded = previous.expanded;
            self.families = previous.families;
            self.include_standard = previous.include_standard;
            self.invalidate_inspection();
            self.rebuild();
            self.fit_pending = false;
            self.request_inspection();
            self.status = "Returned to your view · model unchanged".into();
        } else {
            // A restored session may retain an overlay without a return address.
            // Keep that view intact and dismiss only its agent presentation.
            self.batch_key = None;
            self.status = "Agent overlay dismissed · current view retained".into();
        }
    }

    pub fn finish_agent_projection(&mut self) {
        if !self.show_agent {
            return;
        }
        let projection = self.active_projection();
        let revision = projection.revision_id;
        let targets: BTreeSet<_> = projection.nodes.iter().map(|node| node.id).collect();
        if let Some(activity) = &mut self.agent_activity {
            if activity.revision != revision {
                self.show_agent = false;
                self.dependencies = None;
                return;
            }
            activity.complete = true;
            activity.result_count = targets.len();
            activity.error = None;
        }
        self.dependencies = Some(targets);
    }
}

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
