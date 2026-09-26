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
            create_base_ready: self.binding.is_some_and(|binding| {
                self.history.as_ref().is_some_and(|history| {
                    history.project.id == binding.project
                        && history.branches.iter().any(|branch| {
                            Some(branch.id) == self.branch && branch.head == binding.revision
                        })
                        && history.revisions.iter().any(|revision| {
                            revision.revision_id == binding.revision
                                && matches!(
                                    revision.validation,
                                    agq_modeling_repository::ValidationState::Validated(_)
                                )
                        })
                })
            }),
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
            busy: !self.pending.is_empty() || self.lifecycle_unknown,
            graph_node: self.world == World::Graph
                && self
                    .selected_element()
                    .is_some_and(|id| self.lookup.node(&self.scene, id).is_some()),
            pinned: self
                .selected_element()
                .is_some_and(|id| self.layout.is_pinned(id)),
            agent_view: self.show_agent,
            diff: self.comparison == ComparisonMode::Diff,
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
        if mode != ComparisonMode::Current && self.fixture.is_none() {
            let definition = self.definition();
            if let Some(id) = self.candidate.as_ref().and_then(|candidate| {
                (candidate.before.view != definition
                    || candidate.after.view != definition
                    || self
                        .binding
                        .is_some_and(|binding| candidate.before.revision_id != binding.revision))
                .then_some(candidate.id)
                .flatten()
            }) {
                if self.request_candidate_pair(id) {
                    self.status = "Refreshing candidate review for this view; choose Candidate or Diff when ready".into();
                }
                return;
            }
        }
        let previous = self.display_snapshot();
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
        if !self.rebuild_immediate() {
            self.restore_display(previous);
            return;
        }
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
        if self.candidate.as_ref().is_some_and(|c| {
            c.id.is_some() && c.phase != Some(agq_studio_platform::CandidatePhase::Committed)
        }) {
            self.status =
                "Review or cancel the retained candidate before changing project or revision"
                    .into();
            return false;
        }
        true
    }
    pub fn invalidate_inspection(&mut self) {
        self.bridge.cancel_reads();
        self.inspector = None;
        self.explanation = None;
        self.source = None;
        self.inspector_request = 0;
        self.explanation_request = 0;
        self.source_request = 0;
    }
    /// An explicit action on the visible revision supersedes queued navigation.
    /// Its worker result cannot later replace this newer operator intent.
    pub fn cancel_revision_navigation(&mut self) {
        if let Some(target) = self.pending_revision.take() {
            if self
                .committed_receipt
                .as_ref()
                .is_some_and(|(project, receipt)| {
                    *project == target.project && receipt.revision_id == target.revision
                })
            {
                self.revision_retry = Some(target);
            }
            self.scene_request = 0;
            self.requested_definition = None;
            self.deferred_definition = None;
            self.restore = None;
        }
    }
    pub fn reconcile_candidate_lifecycle(&mut self) {
        if let Some(id) = self.candidate.as_ref().and_then(|candidate| candidate.id) {
            self.lifecycle_unknown = true;
            let view = self.definition();
            self.lifecycle_request = self.enqueue(Box::new(move |platform| {
                platform
                    .candidate(id, &view)
                    .map(Output::CandidateLifecycle)
            }));
        }
    }
    pub fn refresh_history(&mut self) {
        if let Some(project) = self.project_id() {
            let request = self.enqueue(Box::new(move |platform| {
                platform.history(project).map(Output::HistoryRefresh)
            }));
            self.history_request = (request != 0).then_some((request, project));
        }
    }
    pub fn retry_revision_view(&mut self) {
        let Some(target) = self.revision_retry else {
            return;
        };
        if self.project_id() != Some(target.project) || !self.allow_context_change() {
            return;
        }
        self.pending_revision = Some(target);
        self.restore = None;
        self.focus = None;
        self.expanded = None;
        self.refresh_history();
        self.request_projection();
        self.status = "Retrying revision view · durable history is unchanged".into();
    }
    pub fn select_revision(&mut self, revision: ProjectRevisionId) {
        if !self.allow_context_change() {
            return;
        }
        self.remember_location();
        self.focus = None;
        self.expanded = None;
        self.dependencies = None;
        self.show_agent = false;
        self.agent_activity = None;
        self.agent_return = None;
        self.candidate = None;
        self.compare_before = None;
        self.comparison = ComparisonMode::Current;
        self.invalidate_inspection();
        self.scene_request = 0;
        self.requested_definition = None;
        self.deferred_definition = None;
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
            self.pending_revision = Some(agq_studio_platform::RevisionBinding {
                revision,
                ..binding
            });
            self.status = "Loading requested revision · current revision retained".into();
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
    pub fn return_to_revision(&mut self, revision: ProjectRevisionId) {
        self.remember_location();
        if let Some(location) = self.navigation.latest_for_revision(revision) {
            self.restore_location(location);
        } else {
            self.select_revision(revision);
        }
    }
    pub fn definition(&self) -> ViewDefinition {
        let mut view = match self.world {
            World::System => ViewDefinition {
                // One engineering level includes the actual owned parts and
                // their definitions. Enter a subsystem to reveal the next.
                depth: 1,
                ..ViewDefinition::architecture()
            },
            World::Requirements => ViewDefinition::requirements(),
            World::Graph => ViewDefinition {
                depth: 1,
                ..ViewDefinition::semantic_graph()
            },
            _ => ViewDefinition::semantic_graph(),
        };
        if let Some(saved) = self
            .restore
            .as_ref()
            .and_then(|session| session.presentation.as_ref())
            .filter(|saved| saved.world == self.world)
        {
            view = saved.definition.clone();
        } else if let Some(deferred) = self
            .deferred_definition
            .as_ref()
            .filter(|deferred| deferred.kind == view.kind)
        {
            view = deferred.clone();
        } else if let Some((_, requested)) =
            self.requested_definition
                .as_ref()
                .filter(|(request, requested)| {
                    *request != 0 && *request == self.scene_request && requested.kind == view.kind
                })
        {
            view = requested.clone();
        } else if self.ready && self.active_projection().view.kind == view.kind {
            view.depth = self.active_projection().view.depth;
            view.graph_scope = self.active_projection().view.graph_scope;
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
            self.inspector_request = self.request_panel_read(
                crate::read_lane::PanelRead::Inspector,
                binding,
                candidate,
                element,
            );
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
        if self.pending_revision.is_some() || self.restore.is_some() {
            return;
        }
        self.navigation.push(self.current_location());
    }
    pub fn remember_location(&mut self) {
        if self.pending_revision.is_none() && self.restore.is_none() {
            self.navigation.update_current(self.current_location());
        }
    }
    fn current_location(&self) -> Location {
        Location {
            revision: self.projection.revision_id,
            world: self.world,
            focus: self.focus,
            center: [self.camera.center.x, self.camera.center.y],
            zoom: self.camera.zoom,
            presentation: self.capture_presentation(),
        }
    }
    pub fn restore_location(&mut self, location: Location) {
        self.focus_changes_pending = false;
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
        self.restore_world_filters(location.world);
        self.world = location.world;
        self.focus = location.focus;
        self.expanded = location.presentation.expanded.clone();
        self.collapsed = location.presentation.collapsed.clone();
        self.families = location
            .presentation
            .definition
            .relationship_families
            .iter()
            .copied()
            .collect();
        self.include_standard = location.presentation.definition.include_standard_library;
        let definition = location.presentation.definition.clone();
        if let Some(binding) = self.binding {
            self.pending_revision = (binding.revision != location.revision).then_some(
                agq_studio_platform::RevisionBinding {
                    revision: location.revision,
                    ..binding
                },
            );
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
                layout: if (self.layout_world, self.layout_focus)
                    == (location.world, location.focus)
                {
                    self.layout.clone()
                } else {
                    self.layouts
                        .get(&(location.world, location.focus))
                        .cloned()
                        .unwrap_or_default()
                },
                dark: self.theme.dark,
                high_contrast: self.theme.contrast,
                reduced_motion: self.reduced_motion,
                presentation: Some(location.presentation),
            });
            self.request_projection_definition(definition);
        } else {
            // Fixture Worlds can use different projections; rebuild the visited
            // World before reconciling its revision-scoped saved selection.
            self.projection = if location.world == World::Requirements {
                fixtures::requirements()
            } else {
                fixture_projection(self.fixture.as_deref().unwrap_or("architecture"))
            };
            self.apply_saved_presentation(location.presentation);
            self.rebuild_immediate();
            self.request_inspection();
        }
        let mut target = self.camera;
        target.center = Point::new(location.center[0], location.center[1]);
        target.zoom = location.zoom;
        self.camera_target = Some(target);
        self.fit_pending = false;
    }
    pub fn request_projection(&mut self) {
        self.request_projection_definition(self.definition());
    }
    pub(crate) fn request_projection_definition(&mut self, definition: ViewDefinition) {
        self.focus_changes_pending = false;
        if self
            .restore
            .as_ref()
            .and_then(|session| session.presentation.as_ref())
            .is_some_and(|saved| saved.definition != definition)
        {
            // A newer operator query also supersedes the saved camera/layout.
            // Its result must not be relabeled as the older saved definition.
            self.restore = None;
        }
        self.scene_builder.invalidate();
        self.invalidate_inspection();
        if self.pending_revision.is_none()
            && self.comparison == ComparisonMode::Current
            && self.fixture.is_none()
            && let Some(binding) = self.binding
        {
            self.pin_current_reader();
            match self.bridge.project_read(binding, definition.clone()) {
                Ok(request) => {
                    self.scene_request = request;
                    self.pending.insert(request);
                    self.requested_definition = Some((request, definition));
                    self.deferred_definition = None;
                    return;
                }
                Err(error) if self.bridge.mutation_pending() => {
                    self.status = format!("Current revision view unavailable: {error}");
                    return;
                }
                Err(_) => {} // Initial opening can still acquire its first projection serially.
            }
        }
        if self.bridge.mutation_pending() {
            self.deferred_definition = Some(definition);
            self.status =
                "View update queued after model work; the previous revision view remains open"
                    .into();
            return;
        }
        self.deferred_definition = None;
        if let Some(binding) = self.pending_revision.or(self.binding) {
            let candidate = self
                .pending_revision
                .is_none()
                .then(|| self.visible_candidate_id())
                .flatten();
            let comparison_base = (self.pending_revision.is_none()
                && self.candidate.is_none()
                && self.comparison == ComparisonMode::Diff)
                .then(|| {
                    self.compare_before
                        .as_ref()
                        .map(|before| before.revision_id)
                })
                .flatten();
            let requested = definition.clone();
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
            self.requested_definition =
                (self.scene_request != 0).then_some((self.scene_request, requested));
            if self.scene_request == 0
                && let Some(target) = self.pending_revision.take()
            {
                self.revision_retry = Some(target);
            }
        } else {
            // Fixture queries are presentation only, but still retain the
            // requested lens so ordinary edit readiness checks use the same scope.
            let previous = self.display_snapshot();
            self.projection.view = definition;
            if !self.rebuild() {
                self.restore_display(previous);
            }
        }
    }
    pub fn switch_world(&mut self, world: World) {
        self.remember_location();
        let selected = self.selected_element();
        if self.show_agent && self.agent_return.is_some() {
            self.dismiss_agent_view();
        }
        self.navigation.update_camera(
            [self.camera.center.x, self.camera.center.y],
            self.camera.zoom,
        );
        self.restore_world_filters(world);
        self.world = world;
        self.search.clear();
        self.expanded = None;
        self.dependencies = None;
        self.show_agent = false;
        self.agent_return = None;
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
            let mut definition = self.definition();
            if world == World::Graph {
                // An explicit World command starts an ordinary one-hop view;
                // reprojection of an agent view retains its separate scope.
                definition.depth = 1;
                definition.graph_scope = agq_modeling_view::GraphScope::Neighborhood;
            }
            self.request_projection_definition(definition);
        }
        self.fit_pending = true;
        self.record_location();
    }
    pub(crate) fn restore_world_filters(&mut self, world: World) {
        if self.world != world {
            self.world_filters
                .insert(self.world, (self.families.clone(), self.include_standard));
            let (families, standards) = self
                .world_filters
                .get(&world)
                .cloned()
                .unwrap_or_else(|| (RelationshipFamily::all().into_iter().collect(), false));
            self.families = families;
            self.include_standard = standards;
        }
    }
    pub fn load_fixture(&mut self, name: &str) {
        if !self.allow_context_change() {
            return;
        }
        self.bridge.clear_reader();
        self.fixture = Some(name.into());
        self.binding = None;
        self.pending_revision = None;
        self.revision_retry = None;
        self.history_request = None;
        self.lifecycle_request = 0;
        self.lifecycle_unknown = false;
        self.branch = None;
        self.history = None;
        // Fence all outstanding read responses when entering fixture mode.
        self.scene_request = 0;
        self.requested_definition = None;
        self.deferred_definition = None;
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
        self.world_filters.clear();
        self.families = RelationshipFamily::all().into_iter().collect();
        self.include_standard = false;
        self.search.clear();
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
        if matches!(
            id,
            Focus
                | Fit
                | Home
                | Back
                | Forward
                | Up
                | System
                | Graph
                | Requirements
                | SelectionRequirements
                | History
                | DismissAgent
        ) {
            self.remember_location();
            self.focus_changes_pending = false;
        }
        if self.bridge.mutation_pending() && id == Compare {
            self.status = "Wait for the model operation before changing the view".into();
            return;
        }
        match id {
            Pin | Unpin => {
                if let Some((element, bounds)) = self.selected_element().and_then(|id| {
                    self.lookup
                        .node(&self.scene, id)
                        .map(|node| (id, node.bounds))
                }) {
                    if id == Pin {
                        if let Err(error) = self.layout.pin(element, bounds) {
                            self.status = error.to_string();
                            return;
                        }
                    } else {
                        self.layout.unpin(element);
                    }
                    self.rebuild();
                    self.status = if id == Pin {
                        "Graph position pinned · presentation only"
                    } else {
                        "Graph position unpinned · presentation only"
                    }
                    .into();
                }
            }
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
            FocusChanges => self.focus_changes(),
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
            SelectionRequirements => self.show_requirements_selection(),
            History => self.switch_world(World::History),
            Home => {
                self.focus = None;
                self.expanded = None;
                self.dependencies = None;
                self.collapsed.clear();
                self.switch_world(World::System);
            }
            Fit => {
                self.focus_changes_pending = false;
                self.fit_pending = true;
            }
            Focus => {
                self.cancel_revision_navigation();
                self.navigation.update_camera(
                    [self.camera.center.x, self.camera.center.y],
                    self.camera.zoom,
                );
                if let Some(id) = self.selected_element() {
                    if self.world == World::System
                        && self.scene.node(id).is_some_and(|node| {
                            // Ports alone do not create geometric containment.
                            // A real part still has a focused semantic view.
                            node.is_container
                                || (self.fixture.is_none()
                                    && matches!(
                                        node.category,
                                        agq_studio_scene::NodeCategory::System
                                            | agq_studio_scene::NodeCategory::Part
                                    ))
                        })
                    {
                        self.focus = Some(id);
                        self.search.clear();
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
                    self.explanation_request = self.request_panel_read(
                        crate::read_lane::PanelRead::Explain,
                        binding,
                        candidate,
                        element,
                    );
                }
            }
            Source => {
                self.show_source = true;
                if let Some((binding, candidate, element)) = self.selected_context() {
                    self.source_request = self.request_panel_read(
                        crate::read_lane::PanelRead::Source,
                        binding,
                        candidate,
                        element,
                    );
                }
            }
            DismissAgent => self.dismiss_agent_view(),
            Dependencies => {
                self.cancel_revision_navigation();
                self.remember_agent_return();
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
                    let view =
                        ViewDefinition::dependency_neighborhood(element, families, 2, standards);
                    self.fit_pending = true;
                    if candidate.is_none()
                        && self.binding == Some(binding)
                        && self.comparison == ComparisonMode::Current
                    {
                        self.request_projection_definition(view);
                        self.status = "Querying revision-bound dependency neighborhood".into();
                        return;
                    }
                    let requested = view.clone();
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
                    self.requested_definition =
                        (self.scene_request != 0).then_some((self.scene_request, requested));
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
                self.cancel_revision_navigation();
                if let Some(selected) = self.selected_element() {
                    if self.fixture.is_none()
                        && self.comparison == ComparisonMode::Current
                        && self.world == World::Graph
                        && self.focus == Some(selected)
                        && matches!(id, ExpandBoth | CollapseNeighborhood)
                    {
                        let mut definition = self.definition();
                        let depth = if id == CollapseNeighborhood {
                            1
                        } else {
                            definition.depth.saturating_add(1).min(8)
                        };
                        if depth != definition.depth {
                            definition.depth = depth;
                            self.expanded = None;
                            self.request_projection_definition(definition);
                            self.status = format!(
                                "Loading {depth}-hop semantic neighborhood; previous view retained"
                            );
                            return;
                        }
                        if id == ExpandBoth {
                            self.status =
                                "Eight-hop limit reached; focus an object to explore further"
                                    .into();
                            return;
                        }
                    }
                    let direction = match id {
                        ExpandIncoming => NeighborhoodDirection::Incoming,
                        ExpandOutgoing => NeighborhoodDirection::Outgoing,
                        _ => NeighborhoodDirection::Both,
                    };
                    let seeds = if id == ExpandBoth {
                        self.expanded.clone().unwrap_or_else(|| {
                            self.active_projection()
                                .nodes
                                .iter()
                                .map(|node| node.id)
                                .collect()
                        })
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
            CreatePart | RenamePart => self.open_part_edit(id),
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
        let Some(owner) = self.part_edit_target(CommandId::CreatePart) else {
            return;
        };
        let name = self.new_part_name.trim().to_owned();
        if name.is_empty() {
            self.status = "Enter a part name".into();
            return;
        }
        let owner_name = self
            .scene
            .node(owner)
            .map_or("selected part", |node| node.semantic.name.as_str());
        let intent = format!("Add {name} to {owner_name}");
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
                    intent,
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
                intent,
                actor: "human-operator".into(),
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
        let (mut before, after) = fixtures::revision_diff();
        // Fixture names describe the pictured revision, not a different lens.
        before.view = after.view.clone();
        self.projection = after;
        self.compare_before = Some(before);
        self.comparison = ComparisonMode::Diff;
        self.dependencies = None;
        self.show_agent = false;
        self.expanded = None;
        self.rebuild();
        self.fit_pending = false;
        self.focus_changes_pending = true;
        self.status =
            "Comparing architecture baseline with candidate coordination · visual fixture".into();
    }
    pub fn compare_parent(&mut self) {
        if let Some(reason) = commands::unavailable(CommandId::Compare, &self.context()) {
            self.status = reason.into();
            return;
        }
        self.cancel_revision_navigation();
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
            let requested = view.clone();
            self.scene_request = self.enqueue(Box::new(move |p| {
                p.compare(binding.project, parent, binding.revision, &view)
                    .map(Output::Comparison)
            }));
            self.requested_definition =
                (self.scene_request != 0).then_some((self.scene_request, requested));
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
            egui::Popup::close_all(ctx);
            self.palette = !self.palette;
            self.palette_focus = true;
            return;
        }
        // Menus own dismissal and navigation keys before canvas shortcuts.
        // In particular, leave Escape unconsumed so egui closes the popup
        // without clearing selection or navigating behind it.
        if egui::Popup::is_any_open(ctx) {
            return;
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
        if ctx.input(|i| i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowUp)) {
            let nodes = self.filtered_outliner();
            if nodes.is_empty() {
                return;
            }
            let current = self.selected_element().and_then(|id| {
                nodes
                    .iter()
                    .position(|index| self.scene.nodes[*index].id() == id)
            });
            let next = if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                current.map_or(0, |current| (current + 1) % nodes.len())
            } else {
                current.map_or(nodes.len() - 1, |current| {
                    (current + nodes.len() - 1) % nodes.len()
                })
            };
            self.select(SceneTarget::Node(self.scene.nodes[nodes[next]].id()), false);
        }
    }

    pub fn filtered_outliner(&self) -> Vec<usize> {
        let search = self.search.to_lowercase();
        self.outliner_order
            .iter()
            .copied()
            .filter(|index| {
                search.is_empty()
                    || self.scene.nodes[*index]
                        .semantic
                        .name
                        .to_lowercase()
                        .contains(&search)
            })
            .collect()
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
    use clap::Parser;

    fn application() -> StudioApp {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let args = crate::Args::parse_from([
            "studio",
            "--fixture",
            "architecture",
            "--no-restore",
            "--root",
            root.to_str().unwrap(),
        ]);
        let context = eframe::CreationContext::_new_kittest(egui::Context::default());
        StudioApp::new(&context, args).unwrap()
    }

    #[test]
    fn back_forward_restores_each_visits_camera_selection_filters_and_world() {
        let mut app = application();
        let context = egui::Context::default();
        let target = SceneTarget::Node(app.scene.nodes[0].id);
        app.selection.select(target.clone(), false);
        app.camera.center = Point::new(341.0, 625.0);
        app.camera.zoom = 0.63;
        app.include_standard = true;
        app.families = BTreeSet::from([RelationshipFamily::Ownership]);
        let system = app.capture_presentation();
        app.switch_world(World::Graph);
        app.selection.clear();
        app.camera.center = Point::new(-80.0, 90.0);
        app.camera.zoom = 1.73;
        app.include_standard = false;
        app.families = RelationshipFamily::all().into_iter().collect();
        let graph = app.capture_presentation();
        app.execute(CommandId::Back, &context);
        assert_eq!(app.world, World::System);
        assert_eq!(app.camera.center, system.camera.center);
        assert_eq!(app.camera.zoom, system.camera.zoom);
        assert_eq!(app.selection.primary, Some(target));
        assert!(app.include_standard);
        assert_eq!(
            app.families,
            BTreeSet::from([RelationshipFamily::Ownership])
        );
        // Projection completion records the restored visit. It must not truncate
        // the forward chain just because camera/filter/selection values differ.
        app.record_location();
        app.execute(CommandId::Forward, &context);
        assert_eq!(app.world, World::Graph);
        assert_eq!(app.camera.center, graph.camera.center);
        assert_eq!(app.camera.zoom, graph.camera.zoom);
        assert!(app.selection.primary.is_none());
        assert!(!app.include_standard);
        assert_eq!(
            app.families,
            RelationshipFamily::all().into_iter().collect()
        );
    }

    #[test]
    fn standards_and_relationship_filters_belong_to_each_world_and_reset_with_project() {
        let mut app = application();
        let initial = app.families.clone();
        app.search = "ModelingPlatform".into();
        app.switch_world(World::Graph);
        assert!(app.search.is_empty());
        app.include_standard = true;
        app.families.remove(&RelationshipFamily::Ownership);
        let graph = app.families.clone();
        app.switch_world(World::Requirements);
        assert!(!app.include_standard);
        assert_eq!(app.families, initial);
        app.switch_world(World::System);
        assert!(!app.include_standard);
        assert_eq!(app.families, initial);
        app.switch_world(World::Graph);
        assert!(app.include_standard);
        assert_eq!(app.families, graph);
        app.load_fixture("architecture");
        assert!(app.world_filters.is_empty());
        assert!(!app.include_standard);
        assert_eq!(app.families, initial);
    }

    #[test]
    fn reprojection_retains_explicit_dependency_scope_without_leaking_to_system() {
        let mut app = application();
        app.world = World::Graph;
        app.projection.view = ViewDefinition::semantic_graph();
        app.projection.view.graph_scope = agq_modeling_view::GraphScope::DependencyNeighborhood;
        app.projection.view.depth = 2;
        app.families.remove(&RelationshipFamily::Ownership);
        assert_eq!(
            app.definition().graph_scope,
            agq_modeling_view::GraphScope::DependencyNeighborhood
        );
        assert_eq!(app.definition().depth, 2);
        assert!(
            !app.definition()
                .relationship_families
                .contains(&RelationshipFamily::Ownership)
        );
        app.world = World::System;
        assert_eq!(
            app.definition().graph_scope,
            agq_modeling_view::GraphScope::Neighborhood
        );
        assert_eq!(app.definition().depth, 1);
    }

    #[test]
    fn graph_depth_limit_never_shrinks_the_visible_neighborhood() {
        let mut app = application();
        app.world = World::Graph;
        app.fixture = None;
        app.projection.view = ViewDefinition::semantic_graph();
        app.projection.view.depth = 8;
        let id = app.projection.nodes[0].id;
        app.focus = Some(id);
        app.selection.select(SceneTarget::Node(id), false);
        let generation = app.generation;
        app.execute(CommandId::ExpandBoth, &egui::Context::default());
        assert_eq!(app.generation, generation);
        assert!(app.expanded.is_none());
        assert_eq!(app.definition().depth, 8);
        assert!(app.status.contains("Eight-hop limit"));
    }

    #[test]
    fn explicit_world_exit_restores_operator_filters_before_leaving_agent_view() {
        let mut app = application();
        app.families.remove(&RelationshipFamily::Typing);
        let system = app.families.clone();
        app.remember_agent_return();
        app.world = World::Graph;
        app.show_agent = true;
        app.include_standard = true;
        app.families.remove(&RelationshipFamily::Ownership);
        app.switch_world(World::System);
        assert_eq!(app.families, system);
        assert!(!app.include_standard);
        app.switch_world(World::Graph);
        assert_eq!(
            app.families,
            RelationshipFamily::all().into_iter().collect()
        );
        assert!(!app.include_standard);
    }

    #[test]
    fn diff_retains_removed_objects_outside_temporary_current_neighborhood() {
        let mut app = application();
        app.show_fixture_diff();
        let removed: Vec<_> = app
            .scene
            .nodes
            .iter()
            .filter(|node| node.diff == agq_studio_scene::DiffMark::Removed)
            .map(|node| node.id())
            .collect();
        assert!(!removed.is_empty());
        app.expanded = Some(BTreeSet::from([app.active_projection().nodes[0].id]));
        assert!(app.rebuild_immediate());
        assert!(removed.iter().all(|id| app.scene.node(*id).is_some()));
        assert!(commands::unavailable(CommandId::ExpandBoth, &app.context()).is_some());
        let before = (
            app.expanded.clone(),
            app.focus,
            serde_json::to_value(app.active_projection()).unwrap(),
            app.generation,
        );
        app.execute(CommandId::ExpandBoth, &egui::Context::default());
        assert_eq!(
            before,
            (
                app.expanded.clone(),
                app.focus,
                serde_json::to_value(app.active_projection()).unwrap(),
                app.generation
            )
        );
    }

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
