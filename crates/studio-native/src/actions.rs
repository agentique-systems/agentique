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
        if self.bridge.mutation_pending() {
            self.status = "Wait for the model operation before navigating".into();
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
            });
            self.request_projection();
        } else {
            self.rebuild();
        }
        self.camera.center = Point::new(location.center[0], location.center[1]);
        self.camera.zoom = location.zoom;
        self.camera_target = None;
        self.fit_pending = false;
    }
    pub fn request_projection(&mut self) {
        self.invalidate_inspection();
        if let Some(binding) = self.binding {
            let definition = self.definition();
            let candidate = self.visible_candidate_id();
            self.scene_request = self.enqueue(Box::new(move |platform| {
                if let Some(id) = candidate {
                    crate::bridge::candidate_view(platform, id, &definition)
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
        if self.bridge.mutation_pending() {
            self.status = "Wait for the model operation before switching worlds".into();
            return;
        }
        self.navigation.update_camera(
            [self.camera.center.x, self.camera.center.y],
            self.camera.zoom,
        );
        self.world = world;
        self.expanded = None;
        self.dependencies = None;
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
            self.rebuild();
        } else if world != World::History {
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
        if self.bridge.mutation_pending()
            && matches!(
                id,
                Home | Back
                    | Forward
                    | Up
                    | System
                    | Graph
                    | Requirements
                    | History
                    | Focus
                    | Compare
                    | Dependencies
                    | ExpandIncoming
                    | ExpandOutgoing
                    | ExpandBoth
                    | CollapseNeighborhood
            )
        {
            self.status = "Wait for the model operation before changing the view".into();
            return;
        }
        match id {
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
            ReducedMotion => self.reduced_motion = !self.reduced_motion,
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
                    self.expanded = None;
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
            self.enqueue_mutation(crate::bridge::nested_part(context, owner, name, view));
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
            self.fit_pending = true;
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
        self.rebuild();
        self.fit_pending = true;
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
