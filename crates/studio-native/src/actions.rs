use crate::{
    app::{Candidate, ComparisonMode, StudioApp, fixture_projection},
    bridge::{Output, Work},
    commands::{self, CommandContext, CommandId},
    navigation::{Location, World},
};
use agq_kernel::ElementId;
use agq_modeling_view::{
    FeatureCounts, RelationshipFamily, ViewDefinition, ViewEdge, ViewNode, ViewOrigin,
};
use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::{NeighborhoodDirection, Point, SceneTarget, fixtures};
use eframe::egui::{self, Key, Modifiers};
use std::collections::BTreeSet;

impl StudioApp {
    pub fn context(&self) -> CommandContext {
        let can_create = self
            .selected_element()
            .and_then(|id| self.scene.node(id))
            .is_some_and(|node| {
                commands::can_create_part(
                    &node.semantic,
                    self.fixture.is_some(),
                    self.projection.revision_id,
                )
            });
        CommandContext {
            selected: self.selection.primary.is_some(),
            can_create,
            candidate: self.candidate.as_ref().map_or(
                commands::CandidateReview::None,
                |candidate| {
                    candidate.phase.map_or(
                        commands::CandidateReview::Visual,
                        commands::CandidateReview::Semantic,
                    )
                },
            ),
            live: self.binding.is_some() && self.fixture.is_none(),
            busy: !self.pending.is_empty(),
        }
    }
    pub fn change_comparison(&mut self, mode: ComparisonMode) {
        if self.candidate.is_none() || self.comparison == mode {
            return;
        }
        if self.bridge.mutation_pending() {
            self.status = "Wait for the model operation before changing the candidate view".into();
            return;
        }
        if let Some(candidate) = &mut self.candidate {
            if self.comparison != ComparisonMode::Current {
                candidate.review_selection = Some(self.selection.clone());
            } else if mode != ComparisonMode::Current
                && let Some(selection) = &candidate.review_selection
            {
                self.selection = selection.clone();
            }
        }
        self.comparison = mode;
        self.invalidate_inspection();
        self.rebuild();
        if self.fixture.is_none() {
            self.request_projection();
        }
    }
    pub fn work_context(&self) -> crate::bridge::WorkContext {
        crate::bridge::WorkContext {
            binding: self.binding,
            candidate: self.candidate.as_ref().and_then(|c| c.id),
            fixture: self.fixture.clone(),
        }
    }
    pub fn enqueue(&mut self, work: Work) -> u64 {
        self.enqueue_kind(work, false)
    }
    pub fn enqueue_mutation(&mut self, work: Work) -> u64 {
        self.enqueue_kind(work, true)
    }
    fn enqueue_kind(&mut self, work: Work, mutation: bool) -> u64 {
        let context = self.work_context();
        match self.bridge.work(work, context, mutation) {
            Ok(id) => {
                self.pending.insert(id);
                id
            }
            Err(e) => {
                self.status = e;
                0
            }
        }
    }
    pub fn allow_context_change(&mut self) -> bool {
        if self.bridge.mutation_pending() {
            self.status = "Wait for the model operation before changing project or revision".into();
            return false;
        }
        if self.candidate.as_ref().is_some_and(|c| c.id.is_some()) {
            self.status =
                "Review or cancel the retained candidate before changing project or revision"
                    .into();
            return false;
        }
        true
    }
    pub fn invalidate_inspection(&mut self) {
        self.inspector = None;
        self.explanation = None;
        self.source = None;
        self.inspector_request = 0;
        self.explanation_request = 0;
        self.source_request = 0;
    }
    pub fn select_revision(&mut self, revision: ProjectRevisionId) {
        if !self.allow_context_change() {
            return;
        }
        self.focus = None;
        self.expanded = None;
        self.dependencies = None;
        self.candidate = None;
        self.compare_before = None;
        self.comparison = ComparisonMode::Current;
        self.invalidate_inspection();
        self.scene_request = 0;
        self.selection.clear();
        self.fit_pending = true;
        if let Some(binding) = self.binding {
            if !self
                .history
                .as_ref()
                .is_some_and(|history| history.revisions.iter().any(|r| r.revision_id == revision))
            {
                self.status = "Revision is outside this project".into();
                return;
            }
            self.binding = Some(agq_studio_platform::RevisionBinding {
                revision,
                ..binding
            });
            self.request_projection();
        } else {
            let (before, after) = fixtures::revision_diff();
            if revision == before.revision_id {
                self.projection = before;
            } else if revision == after.revision_id {
                self.projection = after;
            } else {
                return;
            }
            self.rebuild();
        }
        self.record_location();
    }
    pub fn definition(&self) -> ViewDefinition {
        let mut view = match self.world {
            World::System => ViewDefinition::architecture(),
            World::Requirements => ViewDefinition::requirements(),
            _ => ViewDefinition::semantic_graph(),
        };
        if let Some(saved) = self
            .restore
            .as_ref()
            .and_then(|session| session.presentation.as_ref())
            .filter(|saved| saved.world == self.world)
        {
            view = saved.definition.clone();
        } else if self.active_projection().view.kind == view.kind {
            view.depth = self.active_projection().view.depth;
            view.hidden_elements = self.active_projection().view.hidden_elements.clone();
        }
        view.focus = self.focus;
        view.include_standard_library = self.include_standard;
        view.relationship_families = self.families.iter().copied().collect();
        view
    }
    pub fn selected_element(&self) -> Option<ElementId> {
        self.selection.element(&self.scene)
    }
    pub fn select(&mut self, target: SceneTarget, extend: bool) {
        self.selection.select(target, extend);
        self.batch_key = None;
        self.invalidate_inspection();
        self.request_inspection();
    }
    /// Refresh the exact primary object after a projection without changing
    /// the current multi-selection or promoting a ghost to the active revision.
    pub fn request_inspection(&mut self) {
        self.inspector = None;
        self.inspector_request = 0;
        if let Some((binding, candidate, element)) = self.selected_context() {
            self.inspector_request = self.enqueue(Box::new(move |platform| {
                if let Some(id) = candidate {
                    platform
                        .inspect_candidate(id, element)
                        .map(Output::Inspector)
                } else {
                    platform.inspect(binding, element).map(Output::Inspector)
                }
            }));
        }
    }
    pub fn selected_context(
        &self,
    ) -> Option<(
        agq_studio_platform::RevisionBinding,
        Option<agq_studio_platform::CandidateId>,
        ElementId,
    )> {
        let binding = self.binding?;
        let element = self.selected_element()?;
        let revision = match self.selection.primary.as_ref()? {
            SceneTarget::Node(id) | SceneTarget::Container(id) => {
                self.scene.node(*id)?.semantic.revision_id
            }
            SceneTarget::Port(id) => self.scene.ports.iter().find(|p| p.id == *id)?.revision_id,
            SceneTarget::Edge(id) => {
                self.scene
                    .edges
                    .iter()
                    .find(|e| e.semantic.id == *id)?
                    .semantic
                    .revision_id
            }
        };
        let candidate = self.visible_candidate_id().filter(|_| {
            self.candidate
                .as_ref()
                .is_some_and(|c| c.after.revision_id == revision)
        });
        Some((
            agq_studio_platform::RevisionBinding {
                revision,
                ..binding
            },
            candidate,
            element,
        ))
    }
    pub fn visible_candidate_id(&self) -> Option<agq_studio_platform::CandidateId> {
        if self.comparison == ComparisonMode::Current {
            None
        } else {
            self.candidate.as_ref().and_then(|c| c.id)
        }
    }
    pub fn record_location(&mut self) {
        self.navigation.push(Location {
            revision: self.projection.revision_id,
            world: self.world,
            focus: self.focus,
            center: [self.camera.center.x, self.camera.center.y],
            zoom: self.camera.zoom,
        });
    }
    pub fn restore_location(&mut self, location: Location) {
        if self.bridge.mutation_pending() && location.revision != self.projection.revision_id {
            self.status = "The current revision remains explorable while model work runs".into();
            return;
        }
        if self
            .binding
            .is_some_and(|binding| binding.revision != location.revision)
            && !self.allow_context_change()
        {
            return;
        }
        // Model revisions are immutable, but may require a worker restoration.
        self.world = location.world;
        self.focus = location.focus;
        self.expanded = None;
        if let Some(binding) = self.binding {
            self.binding = Some(agq_studio_platform::RevisionBinding {
                revision: location.revision,
                ..binding
            });
            // Apply the navigation camera after the asynchronous lens query.
            // This is the same disposable presentation restoration used at startup.
            let mut camera = self.camera;
            camera.center = Point::new(location.center[0], location.center[1]);
            camera.zoom = location.zoom;
            self.restore = Some(crate::session::Session {
                version: 1,
                project: Some(binding.project),
                revision: location.revision,
                fixture: None,
                world: location.world,
                focus: location.focus,
                camera,
                layout: if self.layout_world == location.world {
                    self.layout.clone()
                } else {
                    self.layouts
                        .get(&location.world)
                        .cloned()
                        .unwrap_or_default()
                },
                dark: self.theme.dark,
                high_contrast: self.theme.contrast,
                reduced_motion: self.reduced_motion,
                presentation: None,
            });
            self.request_projection();
        } else {
            self.rebuild();
        }
        let mut target = self.camera;
        target.center = Point::new(location.center[0], location.center[1]);
        target.zoom = location.zoom;
        self.camera_target = Some(target);
        self.fit_pending = false;
    }
    pub fn request_projection(&mut self) {
        self.scene_builder.invalidate();
        self.invalidate_inspection();
        if self.bridge.mutation_pending() {
            // Present only already loaded semantic data while the serialized
            // worker reconstructs a candidate. No model operation or query is
            // guessed by this local change of lens.
            self.rebuild();
            self.status = "Model operation running · exploring the loaded revision".into();
            return;
        }
        if let Some(binding) = self.binding {
            let definition = self.definition();
            let candidate = self.visible_candidate_id();
            let comparison_base = (self.candidate.is_none()
                && self.comparison == ComparisonMode::Diff)
                .then(|| {
                    self.compare_before
                        .as_ref()
                        .map(|before| before.revision_id)
                })
                .flatten();
            self.scene_request = self.enqueue(Box::new(move |platform| {
                if let Some(id) = candidate {
                    crate::bridge::candidate_view(platform, id, &definition)
                } else if let Some(before) = comparison_base {
                    platform
                        .compare(binding.project, before, binding.revision, &definition)
                        .map(Output::Comparison)
                } else {
                    platform
                        .project(binding, &definition)
                        .map(Output::Projection)
                }
            }));
        } else {
            self.rebuild();
        }
    }
    pub fn switch_world(&mut self, world: World) {
        let selected = self.selected_element();
        self.navigation.update_camera(
            [self.camera.center.x, self.camera.center.y],
            self.camera.zoom,
        );
        self.world = world;
        self.expanded = None;
        self.dependencies = None;
        self.show_agent = false;
        self.status = format!("{} · selected revision", world.title());
        if world != World::System {
            self.focus = None;
        }
        if self.fixture.is_some() {
            if world == World::Requirements {
                self.projection = fixtures::requirements();
            } else if self.projection.view.kind == agq_modeling_view::ViewKind::Requirements {
                self.projection =
                    fixture_projection(self.fixture.as_deref().unwrap_or("architecture"));
            }
            if world == World::Graph
                && let Some(selected) =
                    selected.filter(|id| self.projection.nodes.iter().any(|node| node.id == *id))
            {
                let mut seeds = BTreeSet::from([selected]);
                seeds.extend(
                    self.projection
                        .nodes
                        .iter()
                        .filter(|node| node.owner == Some(selected))
                        .map(|node| node.id),
                );
                self.expanded = Some(
                    agq_studio_scene::expand_neighborhood(
                        &self.projection,
                        &seeds,
                        &self.families,
                        NeighborhoodDirection::Both,
                    )
                    .elements,
                );
            }
            self.rebuild();
        } else if world != World::History {
            if world == World::Graph {
                self.focus = selected;
            }
            self.request_projection();
        }
        self.fit_pending = true;
        self.record_location();
    }
    pub fn load_fixture(&mut self, name: &str) {
        if !self.allow_context_change() {
            return;
        }
        self.fixture = Some(name.into());
        self.binding = None;
        self.branch = None;
        self.history = None;
        // Fence all outstanding read responses when entering fixture mode.
        self.scene_request = 0;
        self.project_request = 0;
        self.inspector_request = 0;
        self.explanation_request = 0;
        self.source_request = 0;
        self.navigation = Default::default();
        self.projection = fixture_projection(name);
        self.world = if name == "requirements" {
            World::Requirements
        } else if matches!(name, "ports" | "stress1000" | "stress10000") {
            World::Graph
        } else {
            World::System
        };
        self.focus = None;
        self.expanded = None;
        self.dependencies = None;
        self.candidate = None;
        self.compare_before = None;
        self.comparison = ComparisonMode::Current;
        self.layout = Default::default();
        self.layouts.clear();
        self.selection.clear();
        self.rebuild();
        self.fit_pending = true;
        self.ready = true;
        if name == "diff" {
            self.show_fixture_diff();
        }
        self.status = "Visual fixture · semantic acceptance is unavailable".into();
        self.record_location();
    }
    pub fn execute(&mut self, id: CommandId, ctx: &egui::Context) {
        if let Some(reason) = commands::unavailable(id, &self.context()) {
            self.status = reason.into();
            return;
        }
        use CommandId::*;
        if self.bridge.mutation_pending() && matches!(id, Compare | Dependencies) {
            self.status = "Wait for the model operation before changing the view".into();
            return;
        }
        match id {
            ShowLoadedGraph => {
                self.expanded = None;
                self.focus = None;
                self.dependencies = None;
                self.show_agent = false;
                self.request_projection();
                self.fit_pending = true;
                self.status =
                    "Graph overview · focus a selection to inspect its neighborhood".into();
            }
            ReviewCurrent => self.change_comparison(ComparisonMode::Current),
            ReviewCandidate => self.change_comparison(ComparisonMode::Candidate),
            ReviewDiff => self.change_comparison(ComparisonMode::Diff),
            FocusChanges => {
                let leaves = self.scene.nodes.iter().any(|node| {
                    !node.is_container && node.diff != agq_studio_scene::DiffMark::Unchanged
                });
                let bounds = self
                    .scene
                    .nodes
                    .iter()
                    .filter(|node| {
                        node.diff != agq_studio_scene::DiffMark::Unchanged
                            && (!leaves || !node.is_container)
                    })
                    .map(|node| node.bounds)
                    .reduce(|a, b| a.union(b));
                if let Some(bounds) = bounds {
                    let mut target = self.camera;
                    target.fit(bounds, 90.0);
                    target.zoom = target.zoom.min(1.3);
                    self.camera_target = Some(target);
                }
            }
            Theme => {
                self.theme = crate::theme::Theme::new(!self.theme.dark, self.theme.contrast);
                self.theme.install(ctx);
                self.batch_key = None;
            }
            Contrast => {
                self.theme = crate::theme::Theme::new(self.theme.dark, !self.theme.contrast);
                self.theme.install(ctx);
                self.batch_key = None;
            }
            ReducedMotion => {
                self.reduced_motion = !self.reduced_motion;
                ctx.style_mut(|style| {
                    style.animation_time = if self.reduced_motion { 0.0 } else { 0.12 }
                });
            }
            System => self.switch_world(World::System),
            Graph => self.switch_world(World::Graph),
            Requirements => self.switch_world(World::Requirements),
            History => self.switch_world(World::History),
            Home => {
                self.focus = None;
                self.expanded = None;
                self.dependencies = None;
                self.collapsed.clear();
                self.switch_world(World::System);
            }
            Fit => self.fit_pending = true,
            Focus => {
                self.navigation.update_camera(
                    [self.camera.center.x, self.camera.center.y],
                    self.camera.zoom,
                );
                if let Some(id) = self.selected_element() {
                    if self.world == World::System
                        && self.scene.node(id).is_some_and(|n| n.is_container)
                    {
                        self.focus = Some(id);
                        self.request_projection();
                        self.fit_pending = true;
                    } else if let Some(bounds) = self
                        .selection
                        .primary
                        .as_ref()
                        .and_then(|t| self.scene.target_bounds(t))
                    {
                        let mut target = self.camera;
                        target.fit(bounds, 140.0);
                        target.zoom = target.zoom.min(2.0);
                        self.camera_target = Some(target);
                    }
                    self.record_location();
                }
            }
            Up => {
                self.focus = self.focus.and_then(|id| {
                    self.projection
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .and_then(|n| n.owner)
                });
                self.request_projection();
                self.fit_pending = true;
                self.record_location();
            }
            Back => {
                if let Some(location) = self.navigation.back() {
                    self.restore_location(location);
                }
            }
            Forward => {
                if let Some(location) = self.navigation.forward() {
                    self.restore_location(location);
                }
            }
            Explain => {
                self.show_explain = true;
                self.explanation = None;
                if let Some((binding, candidate, element)) = self.selected_context() {
                    self.explanation_request = self.enqueue(Box::new(move |p| {
                        if let Some(id) = candidate {
                            p.explain_candidate(id, element).map(Output::Explanation)
                        } else {
                            p.explain(binding, element).map(Output::Explanation)
                        }
                    }));
                }
            }
            Source => {
                self.show_source = true;
                if let Some((binding, candidate, element)) = self.selected_context() {
                    self.source_request = self.enqueue(Box::new(move |p| {
                        if let Some(id) = candidate {
                            p.source_candidate(id, element).map(Output::Source)
                        } else {
                            p.source(binding, element).map(Output::Source)
                        }
                    }));
                }
            }
            Dependencies => {
                if let Some(root) = self.selected_element() {
                    self.agent_activity = Some(crate::agents::DependencyActivity {
                        revision: self.scene.revision_id,
                        root,
                        root_name: self
                            .active_projection()
                            .nodes
                            .iter()
                            .find(|node| node.id == root)
                            .map_or_else(|| "Selected element".into(), |node| node.name.clone()),
                        complete: false,
                        result_count: 0,
                        error: None,
                        fixture: self.fixture.is_some(),
                    });
                }
                if let Some((binding, candidate, element)) = self.selected_context() {
                    self.world = World::Graph;
                    self.focus = Some(element);
                    self.show_agent = true;
                    self.expanded = None;
                    self.dependencies = None;
                    self.invalidate_inspection();
                    let families = self.families.iter().copied().collect();
                    let standards = self.include_standard;
                    let mut view = ViewDefinition::semantic_graph();
                    view.focus = Some(element);
                    view.depth = 2;
                    view.include_standard_library = standards;
                    view.relationship_families = families;
                    self.fit_pending = true;
                    self.scene_request = self.enqueue(Box::new(move |p| {
                        if let Some(id) = candidate {
                            crate::bridge::candidate_view(p, id, &view)
                        } else {
                            p.dependencies(
                                binding,
                                element,
                                view.relationship_families,
                                2,
                                standards,
                            )
                            .map(Output::Projection)
                        }
                    }));
                    self.status = "Querying revision-bound dependency neighborhood".into();
                    return;
                }
                if let Some(id) = self.selected_element() {
                    self.world = World::Graph;
                    self.focus = None;
                    self.show_agent = true;
                    self.dependencies = Some(
                        agq_studio_scene::expand_neighborhood(
                            self.active_projection(),
                            &BTreeSet::from([id]),
                            &self.families,
                            NeighborhoodDirection::Both,
                        )
                        .elements,
                    );
                    self.expanded = self.dependencies.clone();
                    if let Some(activity) = &mut self.agent_activity {
                        activity.complete = true;
                        activity.result_count =
                            self.dependencies.as_ref().map_or(0, |ids| ids.len());
                    }
                    self.rebuild();
                    self.fit_pending = true;
                    self.status = "Temporary dependency view · model unchanged".into();
                }
            }
            ExpandIncoming | ExpandOutgoing | ExpandBoth | CollapseNeighborhood | Neighbors => {
                if let Some(selected) = self.selected_element() {
                    let direction = match id {
                        ExpandIncoming => NeighborhoodDirection::Incoming,
                        ExpandOutgoing => NeighborhoodDirection::Outgoing,
                        _ => NeighborhoodDirection::Both,
                    };
                    let seeds = if id == ExpandBoth {
                        self.expanded
                            .clone()
                            .unwrap_or_else(|| BTreeSet::from([selected]))
                    } else {
                        BTreeSet::from([selected])
                    };
                    let ids = agq_studio_scene::expand_neighborhood(
                        self.active_projection(),
                        &seeds,
                        &self.families,
                        direction,
                    )
                    .elements;
                    if id == Neighbors {
                        let targets = neighborhood_selection(&self.scene, &self.lookup, ids);
                        self.selection.replace(targets);
                        self.invalidate_inspection();
                        self.request_inspection();
                        self.batch_key = None;
                    } else if id == CollapseNeighborhood {
                        self.expanded = Some(ids);
                        self.rebuild();
                        self.status = "One-hop neighborhood in the loaded projection".into();
                    } else {
                        self.expanded.get_or_insert_with(BTreeSet::new).extend(ids);
                        self.rebuild();
                        self.status = "Neighborhood expanded within the loaded projection".into();
                    }
                }
            }
            CreatePart => {
                self.create_dialog = true;
                self.new_part_name = "newPart".into();
            }
            Compare => self.compare_parent(),
            Validate => {
                if let Some(id) = self.candidate.as_ref().and_then(|c| c.id) {
                    let view = self.definition();
                    self.scene_request = self.enqueue_mutation(Box::new(move |p| {
                        p.validate(id, &view)?;
                        crate::bridge::candidate_view(p, id, &view)
                    }));
                }
            }
            Commit => {
                if let Some(id) = self.candidate.as_ref().and_then(|c| c.id) {
                    self.enqueue_mutation(Box::new(move |p| p.commit(id).map(Output::Committed)));
                }
            }
            Cancel => {
                if let Some(id) = self.candidate.as_ref().and_then(|c| c.id) {
                    self.enqueue_mutation(Box::new(move |p| {
                        p.cancel(id)?;
                        Ok(Output::Cancelled)
                    }));
                } else {
                    self.candidate = None;
                    self.comparison = ComparisonMode::Current;
                    self.rebuild();
                    self.status = "Visual candidate cancelled · current revision restored".into();
                }
            }
        }
    }
    pub fn prepare_part(&mut self) {
        if let Some(reason) = commands::unavailable(CommandId::CreatePart, &self.context()) {
            self.status = reason.into();
            return;
        }
        let Some(owner) = self.selected_element() else {
            return;
        };
        let name = self.new_part_name.trim().to_owned();
        if name.is_empty() {
            self.status = "Enter a part name".into();
            return;
        }
        if let (Some(binding), Some(branch)) = (self.binding, self.branch) {
            let context = agq_modeling_agent::AgentContext {
                project: binding.project,
                branch,
                revision: binding.revision,
                selection: vec![owner],
            };
            let view = self.definition();
            let request =
                self.enqueue_mutation(crate::bridge::nested_part(context, owner, name, view));
            if request != 0 {
                self.preparation = Some(crate::app::PendingPreparation {
                    request,
                    started: std::time::Instant::now(),
                    cancelled: false,
                });
                self.status = "Constructing a Working candidate in the background".into();
            }
        } else {
            // Deterministic visual response to typed intent; no semantic reconstruction exists here.
            let _intent = agq_modeling_agent::ModelCommand::CreatePartUsage {
                owner,
                name: name.clone(),
                definition: None,
            };
            let before = self.projection.clone();
            let mut after = before.clone();
            after.revision_id = ProjectRevisionId::from_u128(before.revision_id.as_u128() + 100);
            for node in &mut after.nodes {
                node.revision_id = after.revision_id;
            }
            for edge in &mut after.edges {
                edge.revision_id = after.revision_id;
            }
            let id = ElementId::from_u128(0xfa11000000000000000000000000ffff);
            after.nodes.push(ViewNode {
                id,
                revision_id: after.revision_id,
                semantic_kind: "PartUsage".into(),
                name: name.clone(),
                qualified_name: None,
                owner: Some(owner),
                origin: ViewOrigin::Authored,
                source_available: false,
                features: vec![],
                counts: FeatureCounts::default(),
                badges: vec!["Visual preview".into()],
            });
            if let Some(parent) = after.nodes.iter_mut().find(|node| node.id == owner) {
                parent.counts.parts += 1;
            }
            if let Some(group) = after
                .groups
                .iter_mut()
                .find(|group| group.element_id == owner)
            {
                group.children.push(id);
            } else {
                after.groups.push(agq_modeling_view::ViewGroup {
                    element_id: owner,
                    children: vec![id],
                });
            }
            after.metadata.local_element_count = after.nodes.len();
            after.edges.push(ViewEdge {
                id: "fixture-candidate-ownership".into(),
                relationship_id: None,
                revision_id: after.revision_id,
                family: RelationshipFamily::Ownership,
                semantic_kind: "OwningMembership".into(),
                source: owner,
                target: id,
                origin: ViewOrigin::Authored,
                rule_id: None,
                label: "owns".into(),
                directed: true,
                order: 0,
            });
            self.candidate = Some(Candidate {
                review_selection: None,
                id: None,
                phase: None,
                before,
                after,
                source: format!(
                    "VISUAL FIXTURE — illustrative intent only\npart {name};\n\nNo source reconstruction, validation or commit has occurred."
                ),
            });
            self.comparison = ComparisonMode::Diff;
            self.rebuild();
            self.status =
                "Candidate visual preview · install runtime for semantic reconstruction".into();
        }
        self.create_dialog = false;
    }
    pub fn show_fixture_diff(&mut self) {
        let (before, after) = fixtures::revision_diff();
        self.projection = after;
        self.compare_before = Some(before);
        self.comparison = ComparisonMode::Diff;
        self.dependencies = None;
        self.show_agent = false;
        self.expanded = None;
        self.rebuild();
        self.fit_pending = true;
        self.status =
            "Comparing architecture baseline with candidate coordination · visual fixture".into();
    }
    pub fn compare_parent(&mut self) {
        if let Some(reason) = commands::unavailable(CommandId::Compare, &self.context()) {
            self.status = reason.into();
            return;
        }
        if self.fixture.is_some() {
            self.show_fixture_diff();
            return;
        }
        if let Some(binding) = self.binding
            && let Some(parent) = self
                .history
                .as_ref()
                .and_then(|h| {
                    h.revisions
                        .iter()
                        .find(|r| r.revision_id == binding.revision)
                })
                .and_then(|r| r.parent_revision_id)
        {
            let view = self.definition();
            self.scene_request = self.enqueue(Box::new(move |p| {
                p.compare(binding.project, parent, binding.revision, &view)
                    .map(Output::Comparison)
            }));
        } else {
            self.status = "This revision has no parent".into();
        }
    }
    pub fn keyboard(&mut self, ctx: &egui::Context) {
        // The input method owns Escape/Enter while composing text.
        if self.ime_composing {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
            self.palette = !self.palette;
            self.palette_focus = true;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.palette {
                self.palette = false;
            } else if self.create_dialog {
                self.create_dialog = false;
            } else if self.show_explain || self.show_source {
                self.show_explain = false;
                self.show_source = false;
            } else if !self.selection.targets.is_empty() {
                self.selection.clear();
                self.inspector = None;
                self.batch_key = None;
            } else {
                self.execute(CommandId::Back, ctx);
            }
            return;
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        for (key, modifiers, command) in [
            (Key::F, Modifiers::NONE, CommandId::Focus),
            (Key::Home, Modifiers::NONE, CommandId::Fit),
            (Key::D, Modifiers::NONE, CommandId::Dependencies),
            (Key::E, Modifiers::NONE, CommandId::Explain),
            (Key::N, Modifiers::NONE, CommandId::Neighbors),
            (Key::Num1, Modifiers::NONE, CommandId::System),
            (Key::Num2, Modifiers::NONE, CommandId::Graph),
            (Key::Num3, Modifiers::NONE, CommandId::Requirements),
            (Key::Num4, Modifiers::NONE, CommandId::History),
            (Key::ArrowLeft, Modifiers::ALT, CommandId::Back),
            (Key::Z, Modifiers::COMMAND, CommandId::Back),
            (Key::ArrowRight, Modifiers::ALT, CommandId::Forward),
            (Key::ArrowUp, Modifiers::ALT, CommandId::Up),
            (Key::Backspace, Modifiers::NONE, CommandId::Up),
            (Key::Enter, Modifiers::NONE, CommandId::Focus),
        ] {
            if ctx.input_mut(|i| i.consume_key(modifiers, key)) {
                self.execute(command, ctx);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowUp))
            && !self.outliner_order.is_empty()
        {
            let current = self
                .selected_element()
                .and_then(|id| {
                    self.outliner_order
                        .iter()
                        .position(|index| self.scene.nodes[*index].id() == id)
                })
                .unwrap_or(0);
            let next = if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                (current + 1) % self.outliner_order.len()
            } else {
                (current + self.outliner_order.len() - 1) % self.outliner_order.len()
            };
            self.select(
                SceneTarget::Node(self.scene.nodes[self.outliner_order[next]].id()),
                false,
            );
        }
    }
}

fn neighborhood_selection(
    scene: &agq_studio_scene::SemanticScene,
    lookup: &agq_studio_scene::SceneLookup,
    elements: BTreeSet<ElementId>,
) -> Vec<SceneTarget> {
    elements
        .into_iter()
        .filter_map(|id| {
            if lookup.port(scene, id).is_some() {
                Some(SceneTarget::Port(id))
            } else {
                lookup.node(scene, id).map(|node| {
                    if node.is_container {
                        SceneTarget::Container(id)
                    } else {
                        SceneTarget::Node(id)
                    }
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbor_selection_preserves_real_ports_and_visible_endpoint_owners() {
        let projection = fixtures::architecture();
        let scene = agq_studio_scene::SemanticScene::from_projection(
            &projection,
            &agq_studio_scene::SceneOptions::default(),
            None,
        )
        .unwrap();
        let lookup = agq_studio_scene::SceneLookup::build(&scene);
        let owner = projection
            .nodes
            .iter()
            .find(|n| n.name == "ModelRepository")
            .unwrap()
            .id;
        let neighborhood = agq_studio_scene::expand_neighborhood(
            &projection,
            &BTreeSet::from([owner]),
            &RelationshipFamily::all().into_iter().collect(),
            NeighborhoodDirection::Both,
        );
        let targets = neighborhood_selection(&scene, &lookup, neighborhood.elements);
        assert!(
            targets
                .iter()
                .any(|target| matches!(target, SceneTarget::Port(_)))
        );
        assert!(
            targets
                .iter()
                .all(|target| scene.target_bounds(target).is_some())
        );
        for target in &targets {
            if let SceneTarget::Port(id) = target {
                let port = lookup.port(&scene, *id).unwrap();
                assert!(
                    targets
                        .iter()
                        .any(|target| target.element_id() == Some(port.owner))
                );
            }
        }
    }
}
