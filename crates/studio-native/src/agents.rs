//! Provider-neutral spatial agent presentation. No model mutation or commit authority.
use agq_kernel::ElementId;
use agq_modeling_agent::decision::{
    DecisionModel, DecisionQuestion, DecisionResult, DecisionState, DecisionValue, MockViewDecision,
};
use agq_modeling_workspace::ProjectRevisionId;
use std::collections::BTreeSet;

/// A temporary query has a return address in the same immutable model context.
/// Candidate phase and authority are deliberately never copied back from this.
#[derive(Clone)]
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
#[derive(Clone)]
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
        let previous_display = self.display_snapshot();
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
            self.layouts
                .insert((self.layout_world, self.layout_focus), self.layout.clone());
            self.layout = previous.layout;
            self.layout_world = self.world;
            self.layout_focus = self.focus;
            self.collapsed = previous.collapsed;
            self.expanded = previous.expanded;
            self.families = previous.families;
            self.include_standard = previous.include_standard;
            self.invalidate_inspection();
            if !self.rebuild_immediate() {
                self.restore_display(previous_display);
                return;
            }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestedWorld {
    System,
    Graph,
    Requirements,
    History,
}
impl SuggestedWorld {
    const ALL: [Self; 4] = [Self::System, Self::Graph, Self::Requirements, Self::History];
    fn choice(self) -> &'static str {
        match self {
            Self::System => "Architecture",
            Self::Graph => "Graph",
            Self::Requirements => "Requirements",
            Self::History => "History",
        }
    }
    fn parse(choice: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|world| world.choice() == choice)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System World",
            Self::Graph => "Graph World",
            Self::Requirements => "Requirements World",
            Self::History => "History",
        }
    }
    pub fn world(self) -> crate::navigation::World {
        match self {
            Self::System => crate::navigation::World::System,
            Self::Graph => crate::navigation::World::Graph,
            Self::Requirements => crate::navigation::World::Requirements,
            Self::History => crate::navigation::World::History,
        }
    }
    pub fn command(self) -> crate::commands::CommandId {
        match self {
            Self::System => crate::commands::CommandId::System,
            Self::Graph => crate::commands::CommandId::Graph,
            Self::Requirements => crate::commands::CommandId::Requirements,
            Self::History => crate::commands::CommandId::History,
        }
    }
}

#[derive(Clone)]
pub struct NativeDecision {
    pub observation: DecisionState,
    pub result: DecisionResult,
    pub choice: SuggestedWorld,
    pub weights: Vec<(SuggestedWorld, f64)>,
}

/// Interpret only the explicitly offered view choice. The selected local mock
/// runs synchronously; this is not a network-provider execution facility.
fn recommend_view(
    model: &dyn DecisionModel,
    observation: DecisionState,
) -> Result<NativeDecision, String> {
    let result = model
        .decide(
            &observation,
            &[DecisionQuestion::Choice {
                id: "next-view".into(),
                options: SuggestedWorld::ALL
                    .into_iter()
                    .map(|world| world.choice().into())
                    .collect(),
            }],
        )
        .map_err(|error| error.to_string())?;
    if result.revision != observation.revision || result.provider.trim().is_empty() {
        return Err("Decision response has a different revision or no provider identity".into());
    }
    let [answer] = result.answers.as_slice() else {
        return Err("Decision provider must answer exactly the offered view question".into());
    };
    if answer.question != "next-view" {
        return Err("Decision provider answered another question".into());
    }
    let DecisionValue::Choice(choice) = &answer.value else {
        return Err("Decision provider did not return a view choice".into());
    };
    let choice = SuggestedWorld::parse(choice).ok_or("Decision choice was not offered")?;
    let mut seen = BTreeSet::new();
    let mut weights = vec![];
    for (option, weight) in &answer.probabilities {
        let world =
            SuggestedWorld::parse(option).ok_or("Decision weight names an unoffered view")?;
        if !seen.insert(option) || !weight.is_finite() || !(0.0..=1.0).contains(weight) {
            return Err("Decision weights are duplicated or outside their finite range".into());
        }
        weights.push((world, *weight));
    }
    if answer
        .confidence
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err("Decision confidence is outside its finite range".into());
    }
    // Missing weights stay missing. Do not normalize or invent a distribution.
    Ok(NativeDecision {
        observation,
        result,
        choice,
        weights,
    })
}

#[derive(Clone, PartialEq, Eq)]
struct DecisionBinding {
    epoch: u64,
    binding: Option<agq_studio_platform::RevisionBinding>,
    revision: ProjectRevisionId,
    root: ElementId,
    root_name: String,
    fixture: bool,
}
#[derive(Clone)]
struct CachedDecision {
    binding: DecisionBinding,
    value: Result<NativeDecision, String>,
}

impl crate::app::StudioApp {
    /// Cache a bounded local-provider result for the retained inquiry subject,
    /// not the changing set of dependency results or a later pointer selection.
    pub fn agent_view_decision(
        &self,
        ctx: &eframe::egui::Context,
    ) -> Result<NativeDecision, String> {
        let activity = self
            .agent_activity
            .as_ref()
            .ok_or("No retained query observation")?;
        if !self.show_agent
            || activity.revision != self.scene.revision_id
            || activity.revision != self.active_projection().revision_id
            || activity.fixture != self.fixture.is_some()
            || (!activity.fixture && self.binding.is_none())
            || !self
                .active_projection()
                .nodes
                .iter()
                .any(|node| node.id == activity.root)
        {
            return Err("Decision observation is outside the current revision or view".into());
        }
        let binding = DecisionBinding {
            epoch: self.bridge.epoch(),
            binding: self.binding,
            revision: activity.revision,
            root: activity.root,
            root_name: activity.root_name.clone(),
            fixture: activity.fixture,
        };
        let id = eframe::egui::Id::new("native-view-decision-provider-result");
        if let Some(cached) = ctx.data(|data| data.get_temp::<CachedDecision>(id))
            && cached.binding == binding
        {
            return cached.value;
        }
        let observation = DecisionState {
            intent: "Show dependencies".into(),
            revision: activity.revision.to_string(),
            selected_elements: vec![activity.root.to_string()],
        };
        let value = recommend_view(&MockViewDecision, observation);
        ctx.data_mut(|data| {
            data.insert_temp(
                id,
                CachedDecision {
                    binding,
                    value: value.clone(),
                },
            )
        });
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_mock_retains_observation_and_changes_choice_without_fabricated_weights() {
        let revision = ProjectRevisionId::from_u128(72);
        for (intent, expected) in [
            ("Show dependencies", SuggestedWorld::Graph),
            ("Review requirements", SuggestedWorld::Requirements),
            ("Compare this change", SuggestedWorld::History),
            ("Understand architecture", SuggestedWorld::System),
        ] {
            let observation = DecisionState {
                intent: intent.into(),
                revision: revision.to_string(),
                selected_elements: vec![ElementId::from_u128(3).to_string()],
            };
            let decision = recommend_view(&MockViewDecision, observation).unwrap();
            assert_eq!(decision.choice, expected);
            assert_eq!(decision.result.provider, "deterministic-view-demo");
            assert_eq!(decision.result.revision, decision.observation.revision);
            assert_eq!(decision.observation.selected_elements.len(), 1);
            assert!(decision.result.answers[0].confidence.is_none());
            assert!(decision.weights.is_empty());
        }
    }

    struct ResponseProbe(u8);
    impl DecisionModel for ResponseProbe {
        fn decide(
            &self,
            state: &DecisionState,
            questions: &[DecisionQuestion],
        ) -> Result<DecisionResult, agq_modeling_agent::decision::DecisionError> {
            let mut result = MockViewDecision.decide(state, questions)?;
            match self.0 {
                0 => result.revision = "another revision".into(),
                1 => result.answers[0].question = "unasked question".into(),
                2 => result.answers[0].value = DecisionValue::Choice("Commit".into()),
                3 => result.answers[0].value = DecisionValue::Boolean(true),
                4 => result.answers.push(result.answers[0].clone()),
                5 => result.answers[0].probabilities = vec![("Graph".into(), f64::NAN)],
                6 => result.answers[0].probabilities = vec![("Commit".into(), 0.5)],
                7 => {
                    result.answers[0].probabilities =
                        vec![("Graph".into(), 0.3), ("Graph".into(), 0.2)]
                }
                8 => result.answers[0].confidence = Some(f64::INFINITY),
                9 => result.provider.clear(),
                _ => {
                    result.answers[0].probabilities =
                        vec![("Graph".into(), 0.4), ("Architecture".into(), 0.2)]
                }
            }
            Ok(result)
        }
    }

    #[test]
    fn native_decision_rejects_stale_or_unoffered_results_and_preserves_missing_weights() {
        let state = DecisionState {
            intent: "Show dependencies".into(),
            revision: "revision-a".into(),
            selected_elements: vec!["original-subject".into()],
        };
        for invalid in 0..10 {
            assert!(
                recommend_view(&ResponseProbe(invalid), state.clone()).is_err(),
                "Accepted invalid provider response {invalid}"
            );
        }
        let weighted = recommend_view(&ResponseProbe(10), state).unwrap();
        assert_eq!(
            weighted.weights,
            vec![(SuggestedWorld::Graph, 0.4), (SuggestedWorld::System, 0.2)]
        );
        // The host must not fill missing options or normalize these returned values.
        assert_eq!(weighted.result.answers[0].probabilities.len(), 2);
    }

    #[test]
    fn decision_card_retains_inquiry_subject_and_refuses_stale_observation() {
        use clap::Parser;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let args = crate::Args::parse_from([
            "agq-studio-native",
            "--fixture",
            "architecture",
            "--no-restore",
            "--root",
            root.to_str().unwrap(),
        ]);
        let ctx = eframe::egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = crate::app::StudioApp::new(&creation, args).unwrap();
        let original = app.projection.nodes[0].id;
        let another = app.projection.nodes[1].id;
        app.show_agent = true;
        app.agent_activity = Some(DependencyActivity {
            revision: app.scene.revision_id,
            root: original,
            root_name: "Original inquiry".into(),
            complete: true,
            result_count: app.projection.nodes.len(),
            error: None,
            fixture: true,
        });
        let first = app.agent_view_decision(&ctx).unwrap();
        app.selection
            .select(agq_studio_scene::SceneTarget::Node(another), false);
        assert_eq!(
            app.agent_view_decision(&ctx)
                .unwrap()
                .observation
                .selected_elements,
            first.observation.selected_elements
        );
        app.agent_activity.as_mut().unwrap().root = another;
        assert_eq!(
            app.agent_view_decision(&ctx)
                .unwrap()
                .observation
                .selected_elements,
            vec![another.to_string()]
        );
        app.agent_activity.as_mut().unwrap().revision = ProjectRevisionId::new();
        assert!(app.agent_view_decision(&ctx).is_err());
        app.agent_activity.as_mut().unwrap().revision = app.scene.revision_id;
        app.fixture = None;
        assert!(app.agent_view_decision(&ctx).is_err());
    }
}
