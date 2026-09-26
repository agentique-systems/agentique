//! GPU viewport, semantic accessibility proxies and spatial gesture translation.
use crate::{
    app::StudioApp,
    commands::{self, CommandId},
    gpu::{Batch, Quad, SceneCallback},
};
use agq_modeling_view::{RelationshipFamily, ViewOrigin};
use agq_studio_scene::{
    DiffMark, LodLevel, NodeCategory, Point, PortSide, SceneTarget, Size, VisibleScene,
};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, Vec2};
use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Instant,
};

impl StudioApp {
    fn port_visible(&self, port: &agq_studio_scene::ScenePort) -> bool {
        self.lod.level() >= LodLevel::Features
            || self.selection.contains(port.id)
            || (self.world == crate::navigation::World::System && self.focus == Some(port.owner))
    }
    pub fn viewport(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        crate::automation::record(ui.ctx(), crate::automation::Target::Viewport, rect);
        self.camera.viewport = Size::new(rect.width(), rect.height());
        if self.focus_changes_pending && !self.scene_builder.busy {
            self.focus_changes_pending = false;
            if self.comparison == crate::app::ComparisonMode::Diff {
                self.fit_pending = false;
                if self.candidate.is_none() {
                    self.focus_durable_change_overview();
                } else {
                    self.focus_changes();
                }
            }
        }
        if self.fit_pending && !self.scene_builder.busy {
            let mut target = self.camera;
            target.fit(self.scene.bounds(), 42.0);
            target.zoom = target.zoom.min(1.3);
            if self.frame_number > 2 && !self.reduced_motion {
                self.camera_target = Some(target);
            } else {
                self.camera = target;
                self.camera_target = None;
            }
            self.fit_pending = false;
        }
        if crate::zoom_input::apply(ui, &response, &mut self.camera) {
            self.camera_target = None;
            self.timing.input(crate::timing::InputKind::Zoom);
            ui.ctx().request_repaint();
        }
        self.lod.update(self.camera.zoom);
        let pointer = response
            .hover_pos()
            .or_else(|| ui.input(|i| i.pointer.interact_pos()));
        let local = pointer.map(|p| Point::new(p.x - rect.left(), p.y - rect.top()));
        let world = local.map(|p| self.camera.screen_to_world(p));
        let mut hovered = None;
        if response.hovered()
            && let Some(point) = world
        {
            let started = Instant::now();
            hovered = self.spatial.hit_test(point, 6.0 / self.camera.zoom);
            if self.comparison == crate::app::ComparisonMode::Diff
                && let Some(SceneTarget::Edge(id)) = &hovered
                && !self
                    .selection
                    .targets
                    .contains(&SceneTarget::Edge(id.clone()))
                && self.lookup.edge(&self.scene, id).is_some_and(|edge| {
                    !crate::history::diff_mode(ui.ctx()).includes_edge(edge.semantic.family)
                })
            {
                hovered = None;
            }
            // A hidden feature port is represented by its owning node at low LOD.
            if self.lod.level() < LodLevel::Features
                && let Some(SceneTarget::Port(id)) = hovered.as_ref()
                && let Some(port) = self.lookup.port(&self.scene, *id)
                && !self.port_visible(port)
            {
                hovered = Some(SceneTarget::Node(port.owner));
            }
            self.timing.hit(started.elapsed());
        }
        if response.drag_started() && ui.input(|i| i.modifiers.shift) {
            self.marquee_start = world;
            self.marquee_end = world;
        }
        if response.dragged() {
            self.timing.input(crate::timing::InputKind::Pan);
            self.camera_target = None;
            if self.marquee_start.is_some() {
                self.marquee_end = world;
            } else {
                let delta = ui.input(|i| i.pointer.delta());
                self.camera.pan_screen(Point::new(delta.x, delta.y));
            }
        }
        if response.drag_stopped()
            && let (Some(a), Some(b)) = (self.marquee_start.take(), self.marquee_end.take())
        {
            let mut targets = self
                .spatial
                .marquee(agq_studio_scene::Rect::from_points(a, b));
            targets.retain(|target| match target {
                SceneTarget::Port(id) => self
                    .lookup
                    .port(&self.scene, *id)
                    .is_some_and(|port| self.port_visible(port)),
                _ => true,
            });
            self.selection.replace(targets);
            self.batch_key = None;
            self.inspector = None;
        }
        let mut focus_click = false;
        if response.clicked() {
            self.timing.input(crate::timing::InputKind::Selection);
            focus_click = self.canvas_clicks.click(
                self.generation,
                hovered.as_ref(),
                pointer.map_or([0.0, 0.0], |point| [point.x, point.y]),
                ui.input(|input| input.time),
            );
            if let Some(target) = hovered.clone() {
                self.select(target, ui.input(|i| i.modifiers.shift));
            } else if !ui.input(|i| i.modifiers.shift) {
                self.selection.clear();
                self.inspector = None;
                self.batch_key = None;
            }
        }
        if response.double_clicked() && focus_click && self.selection.primary.is_some() {
            self.execute(CommandId::Focus, ui.ctx());
        }
        if response.secondary_clicked() {
            if let Some(target) = hovered.clone() {
                self.select(target, false);
            } else {
                self.selection.clear();
                self.invalidate_inspection();
                self.batch_key = None;
            }
        }
        response.context_menu(|ui| {
            for id in context_commands(self.selection.primary.as_ref()) {
                let id = *id;
                let command = commands::COMMANDS.iter().find(|c| c.id == id).unwrap();
                let reason = commands::unavailable(id, &self.context());
                if reason.is_some() {
                    continue;
                }
                let button = ui.button(command.label);
                if button.clicked() {
                    self.execute(id, ui.ctx());
                    ui.close();
                }
            }
        });
        let painter = ui.painter_at(rect);
        let theme = self.theme;
        // A presentation grid follows the camera and disappears when too dense.
        let spacing = 80.0 * self.camera.zoom;
        if spacing >= 18.0 {
            let origin = self.camera.world_to_screen(Point::default());
            let mut y = rect.top() + origin.y.rem_euclid(spacing);
            while y < rect.bottom() {
                let mut x = rect.left() + origin.x.rem_euclid(spacing);
                while x < rect.right() {
                    painter.circle_filled(egui::pos2(x, y), 0.8, theme.border.gamma_multiply(0.60));
                    x += spacing;
                }
                y += spacing;
            }
        }
        let visibility_started = Instant::now();
        let objects = self.spatial.visible_scene(
            &self.scene,
            self.camera.visible_rect().inflate(20.0 / self.camera.zoom),
        );
        let mut hasher = DefaultHasher::new();
        let diff_mode = if self.comparison == crate::app::ComparisonMode::Diff {
            crate::history::diff_mode(ui.ctx())
        } else {
            crate::history::DiffMode::All
        };
        diff_mode.hash(&mut hasher);
        self.generation.hash(&mut hasher);
        self.theme.dark.hash(&mut hasher);
        self.theme.contrast.hash(&mut hasher);
        (self.lod.level() as u8).hash(&mut hasher);
        objects.hash(&mut hasher);
        self.selection.targets.hash(&mut hasher);
        if let Some(ids) = &self.dependencies {
            ids.hash(&mut hasher);
        }
        let key = hasher.finish();
        // The index checks revision and identity before borrowing objects in
        // draw order; no per-frame target strings or reverse lookup are needed.
        self.timing.visibility(visibility_started.elapsed());
        let selected_edges: BTreeSet<&str> = self
            .selection
            .targets
            .iter()
            .filter_map(|target| match target {
                SceneTarget::Edge(id) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        if self.batch_key != Some(key) {
            let started = Instant::now();
            self.batch = Arc::new(self.make_batch(key, &objects, &selected_edges, diff_mode));
            self.batch_key = Some(key);
            self.timing.batch(started.elapsed());
        }
        painter.add(egui_wgpu::Callback::new_paint_callback(
            rect,
            SceneCallback {
                batch: self.batch.clone(),
                camera: [
                    self.camera.center.x,
                    self.camera.center.y,
                    rect.width(),
                    rect.height(),
                    self.camera.zoom,
                    ui.ctx().pixels_per_point(),
                    0.0,
                    0.0,
                ],
                stats: self.gpu_stats.clone(),
            },
        ));
        self.timing.visible_nodes = 0;
        self.requirement_lane_labels(&painter, rect);
        let labels_started = Instant::now();
        self.timing.total_nodes = self.scene.nodes.len();
        let mut keyboard_selection = None;
        let mut keyboard_focus = false;
        let mut port_command = None;
        for node in &objects.nodes {
            self.timing.visible_nodes += 1;
            let a = self.camera.world_to_screen(node.bounds.min);
            let b = self.camera.world_to_screen(node.bounds.max);
            let bounds = egui::Rect::from_min_max(
                rect.min + Vec2::new(a.x, a.y),
                rect.min + Vec2::new(b.x, b.y),
            );
            let faded = self.dependencies.as_ref().is_some_and(|ids| {
                !ids.contains(&node.id())
                    && !node.semantic.features.iter().any(|f| ids.contains(&f.id))
            });
            let text = if faded {
                theme.muted.gamma_multiply(0.48)
            } else if node.diff == DiffMark::Removed {
                theme.muted
            } else {
                theme.text
            };
            let scale = self.camera.zoom;
            let inset = 14.0 * scale;
            if self.lod.level() >= LodLevel::Summary
                || (bounds.width() >= 55.0 && bounds.height() >= 24.0)
            {
                let title_size = if node.is_container { 17.0 } else { 16.0 };
                let top = bounds.min
                    + Vec2::new(
                        inset,
                        if self.lod.level() < LodLevel::Summary {
                            5.0
                        } else if node.is_container {
                            15.0 * scale
                        } else {
                            37.0 * scale
                        },
                    );
                let label = if node.is_container {
                    format!(
                        "{}  {}",
                        if node.collapsed { "▸" } else { "▾" },
                        node.semantic.name
                    )
                } else {
                    node.semantic.name.clone()
                };
                let requirement_title = node.category == NodeCategory::Requirement;
                bounded_label(
                    &painter,
                    top,
                    &label,
                    (title_size * scale).clamp(12.0, 26.0),
                    text,
                    (bounds.width() - 2.0 * inset).max(10.0),
                    if requirement_title { 2 } else { 1 },
                );
                if node.is_container && self.lod.level() >= LodLevel::Summary {
                    painter.text(
                        bounds.min + Vec2::new(inset, 40.0 * scale),
                        Align2::LEFT_TOP,
                        format!(
                            "{} part{}{}",
                            node.semantic.counts.parts,
                            if node.semantic.counts.parts == 1 {
                                ""
                            } else {
                                "s"
                            },
                            if node.collapsed { " · collapsed" } else { "" }
                        ),
                        FontId::proportional((10.0 * scale).clamp(9.0, 14.0)),
                        theme.muted,
                    );
                }
                if !node.is_container && self.lod.level() >= LodLevel::Summary {
                    category_mark(
                        &painter,
                        bounds.min + Vec2::new(inset + 4.0 * scale, 20.0 * scale),
                        node.category,
                        (7.0 * scale).clamp(5.0, 12.0),
                        category_color(node.category, theme),
                    );
                    painter.text(
                        bounds.min + Vec2::new(inset + 15.0 * scale, 15.0 * scale),
                        Align2::LEFT_TOP,
                        node.category.label(),
                        FontId::proportional((9.0 * scale).clamp(8.0, 14.0)),
                        if faded {
                            theme.muted.gamma_multiply(0.5)
                        } else {
                            category_color(node.category, theme)
                        },
                    );
                }
                if self.world == crate::navigation::World::Graph && self.layout.is_pinned(node.id())
                {
                    painter.text(
                        bounds.right_top() + Vec2::new(-inset, 15.0 * scale),
                        Align2::RIGHT_TOP,
                        "Pinned",
                        FontId::proportional(10.0),
                        theme.text,
                    );
                }
                if self.lod.level() >= LodLevel::Features && !node.is_container {
                    let subtitle = if node.semantic.counts.ports > 0 {
                        let parts = node.semantic.counts.parts;
                        format!(
                            "{} port{}{}",
                            node.semantic.counts.ports,
                            if node.semantic.counts.ports == 1 {
                                ""
                            } else {
                                "s"
                            },
                            if parts > 0 {
                                format!(" · {parts} part{}", if parts == 1 { "" } else { "s" })
                            } else {
                                String::new()
                            }
                        )
                    } else {
                        match node.semantic.origin {
                            ViewOrigin::Authored => "Authored".into(),
                            ViewOrigin::Derived => "Derived · Explain available".into(),
                            ViewOrigin::Standard => "Standard library".into(),
                            ViewOrigin::Generated => "Generated".into(),
                        }
                    };
                    elided(
                        &painter,
                        bounds.min
                            + Vec2::new(inset, if requirement_title { 87.0 } else { 69.0 } * scale),
                        &subtitle,
                        (11.0 * scale).clamp(9.0, 16.0),
                        theme.muted,
                        (bounds.width() - 2.0 * inset).max(10.0),
                    );
                    if self.lod.level() >= LodLevel::Relationships && !requirement_title {
                        for (index, feature) in node
                            .semantic
                            .features
                            .iter()
                            .filter(|f| {
                                NodeCategory::from_semantic_kind(&f.semantic_kind)
                                    != NodeCategory::Port
                            })
                            .take(1)
                            .enumerate()
                        {
                            elided(
                                &painter,
                                bounds.min + Vec2::new(inset, (90.0 + index as f32 * 15.0) * scale),
                                &feature.name,
                                (10.0 * scale).clamp(9.0, 15.0),
                                theme.muted,
                                (bounds.width() - 2.0 * inset).max(10.0),
                            );
                        }
                    }
                }
                if node.diff != DiffMark::Unchanged {
                    let text = match node.diff {
                        DiffMark::Added => "+ NEW",
                        DiffMark::Removed => "− REMOVED",
                        DiffMark::Changed => "~ CHANGED",
                        _ => "",
                    };
                    painter.text(
                        bounds.right_bottom() - Vec2::new(10.0, 10.0),
                        Align2::RIGHT_BOTTOM,
                        text,
                        FontId::proportional(9.0),
                        theme.amber,
                    );
                }
            }
            // Keyboard/screen-reader names expose semantic objects independently of raster text.
            // Spatial selection still uses the index, rather than traversing widget geometry.
            if self.lod.level() >= LodLevel::Summary && rect.intersects(bounds) {
                let accessibility = ui.interact(
                    bounds.intersect(rect),
                    egui::Id::new(("semantic", self.scene.revision_id, node.id())),
                    Sense::focusable_noninteractive(),
                );
                accessibility.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Label,
                        true,
                        format!(
                            "{}, {}, {:?}",
                            node.semantic.name,
                            node.category.label().to_lowercase(),
                            node.semantic.origin
                        ),
                    )
                });
                if accessibility.has_focus() {
                    painter.rect_stroke(
                        bounds.expand(5.0),
                        7.0,
                        Stroke::new(2.0, theme.accent),
                        egui::StrokeKind::Outside,
                    );
                    if accessibility.gained_focus() {
                        keyboard_selection = Some(SceneTarget::Node(node.id()));
                    }
                    if ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    }) {
                        keyboard_selection = Some(SceneTarget::Node(node.id()));
                        keyboard_focus = true;
                    }
                }
            }
        }
        let mut label_edges = Vec::new();
        for edge in &objects.edges {
            if !diff_mode.includes_edge(edge.semantic.family)
                && !selected_edges.contains(edge.semantic.id.as_str())
            {
                continue;
            }
            let incident = self
                .selection
                .contains(self.lookup.endpoint_owner(edge.semantic.source))
                || self
                    .selection
                    .contains(self.lookup.endpoint_owner(edge.semantic.target));
            let hovered_edge = hovered.as_ref().is_some_and(
                |target| matches!(target, SceneTarget::Edge(id) if id == &edge.semantic.id),
            );
            let requirement_context = self.world == crate::navigation::World::Requirements
                && objects.edges.len() <= 24
                && self.lod.level() >= LodLevel::Summary;
            let selected_edge = selected_edges.contains(edge.semantic.id.as_str());
            let graph_context = self.world == crate::navigation::World::Graph
                && self.lod.level() >= LodLevel::Summary;
            if !(selected_edge
                || hovered_edge
                || requirement_context
                || (incident
                    && (graph_context || self.lod.level() >= LodLevel::Features)
                    && objects.edges.len() <= 24))
            {
                continue;
            }
            label_edges.push((
                if selected_edge {
                    0
                } else if hovered_edge {
                    1
                } else {
                    2
                },
                *edge,
            ));
        }
        // Give explicit inspection priority. Automatic neighborhood labels have
        // a small screen-space budget even when many nodes are selected.
        label_edges.sort_by_key(|(priority, _)| *priority);
        let mut header_bottoms = std::collections::HashMap::<_, f32>::new();
        if !label_edges.is_empty() {
            for port in objects.ports.iter().filter(|port| port.label_in_header) {
                let bottom = rect.top() + self.camera.world_to_screen(port.position).y + 12.0;
                header_bottoms
                    .entry(port.owner)
                    .and_modify(|current| *current = current.max(bottom))
                    .or_insert(bottom);
            }
        }
        let mut obstacles: Vec<_> = if label_edges.is_empty() {
            Vec::new()
        } else {
            objects
                .nodes
                .iter()
                .map(|node| {
                    let a = self.camera.world_to_screen(node.bounds.min);
                    let b = self.camera.world_to_screen(node.bounds.max);
                    let mut bounds = egui::Rect::from_min_max(
                        rect.min + Vec2::new(a.x, a.y),
                        rect.min + Vec2::new(b.x, b.y),
                    );
                    if node.is_container && !node.collapsed {
                        let title_bottom = bounds.min.y + 58.0 * self.camera.zoom;
                        let header_bottom = header_bottoms
                            .get(&node.id())
                            .copied()
                            .unwrap_or(title_bottom)
                            .max(title_bottom);
                        bounds.max.y = bounds.max.y.min(header_bottom);
                    }
                    bounds
                })
                .collect()
        };
        let mut route_obstacles = crate::relationship_labels::RouteObstacles::default();
        if !label_edges.is_empty() {
            for port in &objects.ports {
                let screen = self.camera.world_to_screen(port.position);
                obstacles.push(egui::Rect::from_center_size(
                    rect.min + Vec2::new(screen.x, screen.y),
                    Vec2::splat(12.0),
                ));
            }
            for edge in &objects.edges {
                if diff_mode.includes_edge(edge.semantic.family)
                    || selected_edges.contains(edge.semantic.id.as_str())
                {
                    for pair in edge.points.windows(2) {
                        let a = self.camera.world_to_screen(pair[0]);
                        let b = self.camera.world_to_screen(pair[1]);
                        route_obstacles.insert(
                            rect.min + Vec2::new(a.x, a.y),
                            rect.min + Vec2::new(b.x, b.y),
                            rect,
                        );
                    }
                }
            }
        }
        let mut placed_labels = Vec::new();
        let mut automatic_labels = 0;
        for (priority, edge) in label_edges {
            if priority == 2 && automatic_labels >= 8 {
                continue;
            }
            let label = format!(
                "{}{}",
                edge.semantic.label,
                if edge.semantic.origin == ViewOrigin::Derived {
                    " · derived"
                } else {
                    ""
                }
            );
            let mut label_job = egui::text::LayoutJob::simple_singleline(
                label,
                FontId::proportional(12.0),
                theme.accent,
            );
            label_job.wrap.max_width = (rect.width() * 0.35).clamp(80.0, 320.0);
            label_job.wrap.max_rows = 2;
            label_job.wrap.break_anywhere = true;
            let galley = painter.layout_job(label_job);
            let route: Vec<_> = edge
                .points
                .iter()
                .map(|point| {
                    let screen = self.camera.world_to_screen(*point);
                    rect.min + Vec2::new(screen.x, screen.y)
                })
                .collect();
            let Some(placement) = crate::relationship_labels::place(
                &route,
                galley.size() + Vec2::new(16.0, 8.0),
                rect,
                &obstacles,
                &placed_labels,
                priority < 2,
                &route_obstacles,
            ) else {
                continue;
            };
            let label_rect = placement.bounds;
            let leader_end = egui::pos2(
                placement
                    .anchor
                    .x
                    .clamp(label_rect.left(), label_rect.right()),
                placement
                    .anchor
                    .y
                    .clamp(label_rect.top(), label_rect.bottom()),
            );
            painter.line_segment(
                [placement.anchor, leader_end],
                Stroke::new(1.0, theme.muted),
            );
            painter.rect(
                label_rect,
                4.0,
                theme.canvas,
                Stroke::new(1.0, theme.border),
                egui::StrokeKind::Inside,
            );
            painter.galley(label_rect.min + Vec2::new(8.0, 4.0), galley, theme.accent);
            placed_labels.push(label_rect);
            automatic_labels += usize::from(priority == 2);
        }
        if self.lod.level() >= LodLevel::Features
            || (self.world == crate::navigation::World::System && self.focus.is_some())
            || self
                .selection
                .targets
                .iter()
                .any(|target| matches!(target, SceneTarget::Port(_)))
        {
            for port in &objects.ports {
                let selected_port = self.selection.contains(port.id);
                let focused_boundary = self.world == crate::navigation::World::System
                    && self.focus == Some(port.owner);
                if !self.port_visible(port) {
                    continue;
                }
                let point = self.camera.world_to_screen(port.position);
                let position = rect.min + Vec2::new(point.x, point.y);
                let port_rect =
                    egui::Rect::from_center_size(position, Vec2::splat(16.0)).intersect(rect);
                if !port_rect.is_positive() {
                    continue;
                }
                // Semantic selection remains visible even when ordinary port
                // detail is suppressed at overview/summary zoom. This marker
                // stays a legible screen size without rebuilding GPU batches
                // on every zoom tick.
                if selected_port || (focused_boundary && self.lod.level() < LodLevel::Features) {
                    painter.rect(
                        egui::Rect::from_center_size(
                            position,
                            Vec2::splat(if selected_port { 12.0 } else { 9.0 }),
                        ),
                        2.0,
                        theme.canvas,
                        Stroke::new(
                            2.0,
                            if selected_port {
                                theme.accent
                            } else {
                                theme.muted
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                }
                if selected_port {
                    painter.rect_stroke(
                        egui::Rect::from_center_size(position, Vec2::splat(22.0)),
                        4.0,
                        Stroke::new(1.0, theme.accent.gamma_multiply(0.6)),
                        egui::StrokeKind::Outside,
                    );
                }
                let port_response = ui.interact(
                    port_rect,
                    egui::Id::new(("semantic-port", self.scene.revision_id, port.id)),
                    Sense::click(),
                );
                port_response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!(
                            "{}, Port in {}, direction {}",
                            port.name,
                            self.active_projection()
                                .nodes
                                .iter()
                                .find(|node| node.id == port.proxy_for_owner.unwrap_or(port.owner))
                                .map_or("owner outside this view", |node| node.name.as_str()),
                            port_direction_label(port.direction)
                        ),
                    )
                });
                if port_response.clicked() || port_response.gained_focus() {
                    keyboard_selection = Some(SceneTarget::Port(port.id));
                }
                if port_response.double_clicked() {
                    keyboard_selection = Some(SceneTarget::Port(port.id));
                    keyboard_focus = true;
                }
                if port_response.secondary_clicked() {
                    keyboard_selection = Some(SceneTarget::Port(port.id));
                }
                port_response.context_menu(|ui| {
                    let mut context = self.context();
                    context.selected = true;
                    for id in context_commands(Some(&SceneTarget::Port(port.id))) {
                        if commands::unavailable(*id, &context).is_some() {
                            continue;
                        }
                        let command = commands::COMMANDS
                            .iter()
                            .find(|command| command.id == *id)
                            .expect("registered command");
                        if ui.button(command.label).clicked() {
                            keyboard_selection = Some(SceneTarget::Port(port.id));
                            port_command = Some(*id);
                            ui.close();
                        }
                    }
                });
                if port_response.has_focus() {
                    painter.rect_stroke(
                        port_rect.expand(3.0),
                        3.0,
                        Stroke::new(2.0, theme.accent),
                        egui::StrokeKind::Outside,
                    );
                }
                if self.lod.level() >= LodLevel::Relationships
                    || self.selection.contains(port.id)
                    || self.selection.contains(port.owner)
                    || port_response.hovered()
                    || (self.world == crate::navigation::World::System
                        && self.focus.is_some()
                        && objects.ports.len() <= 24)
                {
                    let owner = self.lookup.node(&self.scene, port.owner);
                    let expanded_owner =
                        owner.is_some_and(|node| node.is_container && !node.collapsed);
                    let owner_width =
                        owner.map_or(100.0 / 0.44, |node| node.bounds.width() * self.camera.zoom);
                    let width = if port.label_in_header {
                        (owner_width * 0.44).min((owner_width * 0.5 - 20.0).max(10.0))
                    } else {
                        owner_width * 0.44
                    };
                    let label_color = if selected_port {
                        theme.accent
                    } else {
                        theme.muted
                    };
                    let mut job = egui::text::LayoutJob::simple_singleline(
                        port.name.clone(),
                        FontId::proportional(12.0),
                        label_color,
                    );
                    job.wrap.max_width = width;
                    job.wrap.max_rows = 1;
                    job.wrap.break_anywhere = true;
                    job.wrap.overflow_character = Some('…');
                    let galley = painter.layout_job(job);
                    // At low zoom a fixed-size name cannot fit the reserved
                    // row. Keep it outside the body with an explicit leader.
                    let header_label = port.label_in_header
                        && self.camera.zoom * 24.0 >= galley.size().y
                        && owner_width >= 80.0;
                    let external = expanded_owner && !header_label;
                    let label_origin = port_label_origin(
                        port.side,
                        position,
                        galley.size(),
                        expanded_owner,
                        header_label,
                    );
                    if external {
                        // A distributed boundary label belongs outside an
                        // expanded owner, never over a child card. A short
                        // leader ties this disposable label to the exact port.
                        let label_rect = egui::Rect::from_min_size(label_origin, galley.size());
                        let end = match port.side {
                            PortSide::Left => label_rect.right_center(),
                            PortSide::Right => label_rect.left_center(),
                            PortSide::Top => label_rect.center_bottom(),
                            PortSide::Bottom => label_rect.center_top(),
                        };
                        painter.line_segment([position, end], Stroke::new(1.0, label_color));
                        painter.rect_filled(label_rect.expand(2.0), 2.0, theme.surface);
                    }
                    painter.galley(label_origin, galley, label_color);
                }
                if port_response.hovered() {
                    let connections = self
                        .active_projection()
                        .edges
                        .iter()
                        .filter(|edge| {
                            edge.family == RelationshipFamily::Connection
                                && (edge.source == port.id || edge.target == port.id)
                        })
                        .count();
                    port_response.on_hover_text(format!(
                        "{} · Port\nDirection {}\n{} connections in this view{}",
                        port.name,
                        port_direction_label(port.direction),
                        connections,
                        if port.proxy_for_owner.is_some() {
                            "\nExposed at collapsed subsystem boundary"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }
        self.timing.labels(labels_started.elapsed());
        if let Some(target) = keyboard_selection {
            self.select(target, false);
            if keyboard_focus {
                self.execute(CommandId::Focus, ui.ctx());
            }
        }
        if let Some(command) = port_command {
            self.execute(command, ui.ctx());
        }
        if let Some(target) = hovered {
            let label = match &target {
                SceneTarget::Node(id) | SceneTarget::Container(id) => self
                    .scene
                    .node(*id)
                    .map(|n| format!("{} · {}", n.semantic.name, n.semantic.semantic_kind)),
                SceneTarget::Port(id) => self.scene.ports.iter().find(|p| p.id == *id).map(|p| {
                    format!(
                        "{} · Port · direction {}{}",
                        p.name,
                        port_direction_label(p.direction),
                        if p.proxy_for_owner.is_some() {
                            " · collapsed boundary proxy; original port identity"
                        } else {
                            ""
                        }
                    )
                }),
                SceneTarget::Edge(id) => self
                    .scene
                    .edges
                    .iter()
                    .find(|e| e.semantic.id == *id)
                    .map(|e| {
                        format!(
                            "{} · {:?} · {:?}",
                            e.semantic.label, e.semantic.family, e.semantic.origin
                        )
                    }),
            };
            if let Some(label) = label {
                response.clone().on_hover_text(label);
            }
        }
        if let (Some(a), Some(b)) = (self.marquee_start, self.marquee_end) {
            let a = self.camera.world_to_screen(a);
            let b = self.camera.world_to_screen(b);
            let box_rect = egui::Rect::from_two_pos(
                rect.min + Vec2::new(a.x, a.y),
                rect.min + Vec2::new(b.x, b.y),
            );
            painter.rect(
                box_rect,
                0.0,
                theme.accent.gamma_multiply(0.10),
                Stroke::new(1.0, theme.accent),
                egui::StrokeKind::Inside,
            );
        }
        if self.fixture.is_some() {
            painter.text(
                rect.left_bottom() + Vec2::new(24.0, -23.0),
                Align2::LEFT_BOTTOM,
                "DETERMINISTIC VISUAL FIXTURE",
                FontId::proportional(10.0),
                theme.muted.gamma_multiply(0.8),
            );
        }
        painter.text(
            rect.right_bottom() + Vec2::new(-24.0, -23.0),
            Align2::RIGHT_BOTTOM,
            "Drag to pan   ·   Wheel to zoom   ·   F to focus",
            FontId::proportional(11.0),
            theme.muted,
        );
        if self.scene.nodes.is_empty() {
            let empty_project = self.fixture.is_none()
                && self.history.as_ref().is_some_and(|history| {
                    history.revisions.iter().any(|revision| {
                        revision.revision_id == self.projection.revision_id
                            && revision.documents.is_empty()
                    })
                });
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                if empty_project {
                    "Empty Working project · use the project menu to Add source document"
                } else {
                    "No elements in this view"
                },
                FontId::proportional(20.0),
                theme.muted,
            );
        }
    }
    fn make_batch(
        &self,
        key: u64,
        objects: &VisibleScene<'_>,
        selected_edges: &BTreeSet<&str>,
        diff_mode: crate::history::DiffMode,
    ) -> Batch {
        let mut batch = Batch {
            key,
            ..Default::default()
        };
        let theme = self.theme;
        for node in &objects.nodes {
            let selected = self.selection.contains(node.id());
            let faded = self.dependencies.as_ref().is_some_and(|ids| {
                !ids.contains(&node.id())
                    && !node.semantic.features.iter().any(|f| ids.contains(&f.id))
            });
            let mut border = if selected { theme.accent } else { theme.border };
            if node.diff == DiffMark::Added {
                border = theme.green;
            } else if node.diff == DiffMark::Changed {
                border = theme.amber;
            }
            let mut fill = if node.is_container {
                theme.containment(node.depth)
            } else {
                theme.elevated
            };
            if faded {
                fill = theme.canvas;
                border = border.gamma_multiply(0.35);
            }
            if node.diff == DiffMark::Removed {
                fill = theme.canvas;
                border = theme.muted.gamma_multiply(0.6);
            }
            let rect = [
                node.bounds.min.x,
                node.bounds.min.y,
                node.bounds.width(),
                node.bounds.height(),
            ];
            let radius = match node.category {
                NodeCategory::Requirement => 2.0,
                NodeCategory::Action | NodeCategory::State => 22.0,
                _ => 8.0,
            };
            let quad = Quad::rect(rect, fill, border, radius, if selected { 2.0 } else { 1.0 });
            if node.is_container {
                batch.containers.push(quad);
                // A header separator makes containment readable without another bright card.
                batch.containers.push(Quad::rect(
                    [rect[0] + 14.0, rect[1] + 58.0, rect[2] - 28.0, 0.8],
                    theme.border.gamma_multiply(0.65),
                    Color32::TRANSPARENT,
                    0.0,
                    0.0,
                ));
            } else {
                if selected || node.diff == DiffMark::Added {
                    batch.nodes.push(Quad::rect(
                        [rect[0] - 4.0, rect[1] - 4.0, rect[2] + 8.0, rect[3] + 8.0],
                        Color32::TRANSPARENT,
                        border.gamma_multiply(0.22),
                        radius + 4.0,
                        3.0,
                    ));
                }
                batch.nodes.push(quad);
                batch.nodes.push(Quad::rect(
                    [rect[0] + 14.0, rect[1] + 30.0, rect[2] - 28.0, 0.7],
                    theme.border.gamma_multiply(0.55),
                    Color32::TRANSPARENT,
                    0.0,
                    0.0,
                ));
            }
        }
        for edge in &objects.edges {
            if !diff_mode.includes_edge(edge.semantic.family)
                && !selected_edges.contains(edge.semantic.id.as_str())
            {
                continue;
            }
            // Ownership is already explicit spatially; graph mode exposes its actual edges.
            if self.world == crate::navigation::World::System
                && edge.semantic.family == RelationshipFamily::Ownership
            {
                continue;
            }
            let selected = selected_edges.contains(edge.semantic.id.as_str());
            let incident = self.selection.contains(edge.semantic.source)
                || self.selection.contains(edge.semantic.target)
                || self
                    .selection
                    .contains(self.lookup.endpoint_owner(edge.semantic.source))
                || self
                    .selection
                    .contains(self.lookup.endpoint_owner(edge.semantic.target));
            let mut color = if selected || incident {
                theme.accent
            } else {
                theme
                    .muted
                    .gamma_multiply(if theme.contrast { 0.95 } else { 0.65 })
            };
            // In System World a selected container establishes the surrounding
            // engineering context; its children's connections remain readable.
            // Graph World uses deliberate path emphasis for semantic reasoning.
            if self.world == crate::navigation::World::Graph
                && !self.selection.targets.is_empty()
                && !selected
                && !incident
            {
                color = color.gamma_multiply(if theme.contrast { 0.70 } else { 0.44 });
            }
            if edge.diff == DiffMark::Added {
                color = theme.green;
            } else if edge.diff == DiffMark::Removed {
                color = theme.amber.gamma_multiply(0.6);
            }
            // Change coloring must respect the same explicit-selection priority
            // as ordinary edges, otherwise every added relation shouts at once.
            if self.comparison == crate::app::ComparisonMode::Diff
                && !self.selection.targets.is_empty()
                && !selected
                && !incident
            {
                color = color.gamma_multiply(0.35);
            }
            if self.dependencies.as_ref().is_some_and(|ids| {
                !ids.contains(&edge.semantic.source) || !ids.contains(&edge.semantic.target)
            }) {
                color = color.gamma_multiply(0.22);
            }
            let width = if selected {
                2.8
            } else if incident {
                1.8
            } else {
                1.1
            };
            let dashed =
                edge.semantic.origin == ViewOrigin::Derived || edge.diff == DiffMark::Removed;
            for points in edge.points.windows(2) {
                let a = points[0];
                let b = points[1];
                if let Some(mut quad) = Quad::segment([a.x, a.y], [b.x, b.y], width, color) {
                    if dashed {
                        quad.detail = [12.0, 7.0, 0.0, 0.0];
                    }
                    batch.edges.push(quad);
                }
            }
            if edge.semantic.directed && edge.points.len() >= 2 {
                let end = edge.points[edge.points.len() - 1];
                let before = edge.points[edge.points.len() - 2];
                let dx = end.x - before.x;
                let dy = end.y - before.y;
                let length = dx.hypot(dy).max(0.001);
                let ux = dx / length;
                let uy = dy / length;
                for side in [-1.0, 1.0] {
                    if let Some(quad) = Quad::segment(
                        [
                            end.x - ux * 7.0 - uy * 4.0 * side,
                            end.y - uy * 7.0 + ux * 4.0 * side,
                        ],
                        [end.x, end.y],
                        width,
                        color,
                    ) {
                        batch.edges.push(quad);
                    }
                }
            }
        }
        if self.lod.level() >= LodLevel::Features {
            let connected: BTreeSet<_> = objects
                .edges
                .iter()
                .filter(|edge| edge.semantic.family == RelationshipFamily::Connection)
                .flat_map(|edge| [edge.semantic.source, edge.semantic.target])
                .collect();
            for port in &objects.ports {
                let selected = self.selection.contains(port.id);
                let size = if selected { 12.0 } else { 9.0 };
                batch.overlays.push(Quad::rect(
                    [
                        port.position.x - size / 2.0,
                        port.position.y - size / 2.0,
                        size,
                        size,
                    ],
                    theme.canvas,
                    if selected { theme.accent } else { theme.muted },
                    2.0,
                    if selected { 2.5 } else { 1.5 },
                ));
                if connected.contains(&port.id) {
                    batch.overlays.push(Quad::rect(
                        [port.position.x - 1.5, port.position.y - 1.5, 3.0, 3.0],
                        if selected { theme.accent } else { theme.green },
                        Color32::TRANSPARENT,
                        0.0,
                        0.0,
                    ));
                }
            }
        }
        batch
    }
}
fn port_label_origin(
    side: PortSide,
    position: egui::Pos2,
    size: Vec2,
    expanded_owner: bool,
    label_in_header: bool,
) -> egui::Pos2 {
    if expanded_owner {
        let offset = match (side, label_in_header) {
            (PortSide::Left, true) => Vec2::new(10.0, -size.y * 0.5),
            (PortSide::Right, true) => Vec2::new(-10.0 - size.x, -size.y * 0.5),
            (PortSide::Left, false) => Vec2::new(-10.0 - size.x, -size.y * 0.5),
            (PortSide::Right, false) => Vec2::new(10.0, -size.y * 0.5),
            (PortSide::Top, _) => Vec2::new(-size.x * 0.5, -10.0 - size.y),
            (PortSide::Bottom, _) => Vec2::new(-size.x * 0.5, 10.0),
        };
        position + offset
    } else {
        position
            + match side {
                PortSide::Left => Vec2::new(10.0, 10.0),
                PortSide::Right => Vec2::new(-10.0 - size.x, 10.0),
                PortSide::Top => Vec2::new(-size.x * 0.5, -10.0 - size.y),
                PortSide::Bottom => Vec2::new(-size.x * 0.5, 10.0),
            }
    }
}

pub(crate) fn port_direction_label(direction: agq_studio_scene::PortDirection) -> &'static str {
    match direction {
        agq_studio_scene::PortDirection::Unspecified => "not specified",
        agq_studio_scene::PortDirection::In => "in",
        agq_studio_scene::PortDirection::Out => "out",
        agq_studio_scene::PortDirection::InOut => "in/out",
    }
}
fn context_commands(target: Option<&SceneTarget>) -> &'static [CommandId] {
    use CommandId::*;
    match target {
        None => &[Fit, Home, System, Graph, Requirements],
        Some(SceneTarget::Port(_)) => {
            &[Focus, Explain, Source, Dependencies, SelectionRequirements]
        }
        Some(SceneTarget::Edge(_)) => &[Focus, Explain, Source],
        Some(SceneTarget::Node(_) | SceneTarget::Container(_)) => &[
            Focus,
            SelectionRequirements,
            Dependencies,
            Explain,
            Source,
            CreatePart,
            RenamePart,
            Pin,
            Unpin,
            Neighbors,
        ],
    }
}
fn category_color(category: NodeCategory, theme: crate::theme::Theme) -> Color32 {
    match category {
        NodeCategory::Requirement => theme.amber,
        NodeCategory::Agent | NodeCategory::Action | NodeCategory::State => theme.violet,
        NodeCategory::Interface | NodeCategory::Port => theme.green,
        _ => theme.accent,
    }
}
/// Font-independent marks. The adjacent text always supplies the category name.
fn category_mark(
    painter: &egui::Painter,
    center: egui::Pos2,
    category: NodeCategory,
    size: f32,
    color: Color32,
) {
    let bounds = egui::Rect::from_center_size(center, Vec2::splat(size));
    let stroke = Stroke::new(1.0, color);
    match category {
        NodeCategory::Interface | NodeCategory::Port => {
            for y in [-2.0, 2.0] {
                painter.line_segment(
                    [
                        center + Vec2::new(-size * 0.5, y),
                        center + Vec2::new(size * 0.5, y),
                    ],
                    stroke,
                );
            }
        }
        NodeCategory::Agent => {
            let r = size * 0.6;
            painter.add(egui::Shape::closed_line(
                vec![
                    center + Vec2::new(0.0, -r),
                    center + Vec2::new(r, 0.0),
                    center + Vec2::new(0.0, r),
                    center + Vec2::new(-r, 0.0),
                ],
                stroke,
            ));
        }
        NodeCategory::Action | NodeCategory::State => {
            painter.circle_stroke(center, size * 0.5, stroke);
        }
        _ => {
            painter.rect_stroke(bounds, 0.0, stroke, egui::StrokeKind::Inside);
            if category == NodeCategory::System {
                painter.rect_stroke(bounds.shrink(2.0), 0.0, stroke, egui::StrokeKind::Inside);
            } else if category == NodeCategory::Requirement {
                painter.line_segment(
                    [
                        center - Vec2::new(size * 0.25, 0.0),
                        center + Vec2::new(size * 0.25, 0.0),
                    ],
                    stroke,
                );
            }
        }
    }
}
fn elided(
    painter: &egui::Painter,
    position: egui::Pos2,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
) {
    bounded_label(painter, position, text, size, color, width, 1);
}
fn bounded_label(
    painter: &egui::Painter,
    position: egui::Pos2,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
    rows: usize,
) {
    let mut job =
        egui::text::LayoutJob::simple_singleline(text.into(), FontId::proportional(size), color);
    job.wrap.max_width = width;
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = rows == 1;
    job.wrap.overflow_character = Some('…');
    painter.galley(position, painter.layout_job(job), color);
}

#[cfg(test)]
mod port_label_tests {
    use super::*;

    #[test]
    fn expanded_owner_labels_use_the_clear_header_or_the_exterior_not_child_cards() {
        let size = Vec2::new(92.0, 14.0);
        // Real-run04's focused container is shown near 71% zoom. The new
        // reserved row at world y=72 stays above children at world y=92.
        let zoom = 0.708;
        for side in [PortSide::Left, PortSide::Right] {
            let x = if side == PortSide::Left {
                0.0
            } else {
                844.0 * zoom
            };
            let port = egui::pos2(x, 72.0 * zoom);
            let rect =
                egui::Rect::from_min_size(port_label_origin(side, port, size, true, true), size);
            assert!(rect.min.y > 58.0 * zoom);
            assert!(rect.max.y < 92.0 * zoom);
            assert!(rect.min.x >= 0.0 && rect.max.x <= 844.0 * zoom);

            let body_port = egui::pos2(x, 292.0 * zoom);
            let exterior = egui::Rect::from_min_size(
                port_label_origin(side, body_port, size, true, false),
                size,
            );
            if side == PortSide::Left {
                assert!(exterior.max.x < 0.0);
            } else {
                assert!(exterior.min.x > 844.0 * zoom);
            }
            assert_eq!(exterior.center().y, body_port.y);
        }
        let top = egui::pos2(140.0, 0.0);
        let top_label = egui::Rect::from_min_size(
            port_label_origin(PortSide::Top, top, size, true, false),
            size,
        );
        assert!(top_label.max.y < top.y);
        let bottom = egui::pos2(140.0, 500.0);
        let bottom_label = egui::Rect::from_min_size(
            port_label_origin(PortSide::Bottom, bottom, size, true, false),
            size,
        );
        assert!(bottom_label.min.y > bottom.y);
    }
}
