//! GPU viewport, semantic accessibility proxies and spatial gesture translation.
use crate::{
    app::StudioApp,
    commands::{self, CommandId},
    gpu::{Batch, Quad, SceneCallback},
    panels::category_icon,
};
use agq_modeling_view::{RelationshipFamily, ViewOrigin};
use agq_studio_scene::{DiffMark, LodLevel, NodeCategory, Point, PortSide, SceneTarget, Size};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, Vec2};
use std::{
    collections::{BTreeSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Instant,
};

impl StudioApp {
    pub fn viewport(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        crate::automation::record(ui.ctx(), crate::automation::Target::Viewport, rect);
        self.camera.viewport = Size::new(rect.width(), rect.height());
        if self.fit_pending {
            self.camera.fit(self.scene.bounds(), 42.0);
            self.camera.zoom = self.camera.zoom.min(1.3);
            self.camera_target = None;
            self.fit_pending = false;
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
            // A hidden feature port is represented by its owning node at low LOD.
            if self.lod.level() < LodLevel::Features
                && let Some(SceneTarget::Port(id)) = hovered.as_ref()
                && let Some(port) = self.lookup.port(&self.scene, *id)
            {
                hovered = Some(SceneTarget::Node(port.owner));
            }
            self.timing.hit(started.elapsed());
            let wheel = ui.input(|i| i.smooth_scroll_delta.y);
            if wheel.abs() > 0.01 {
                self.camera_target = None;
                self.camera.zoom_at(local.unwrap(), (wheel * 0.0025).exp());
                self.timing.input(crate::timing::InputKind::Zoom);
                ui.ctx().request_repaint();
            }
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
            if self.lod.level() < LodLevel::Features {
                targets.retain(|target| !matches!(target, SceneTarget::Port(_)));
            }
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
        if response.secondary_clicked()
            && let Some(target) = hovered.clone()
        {
            self.select(target, false);
        }
        response.context_menu(|ui| {
            for id in [
                CommandId::Focus,
                CommandId::Dependencies,
                CommandId::Explain,
                CommandId::Source,
                CommandId::CreatePart,
                CommandId::Neighbors,
                CommandId::Fit,
            ] {
                let command = commands::COMMANDS.iter().find(|c| c.id == id).unwrap();
                let reason = commands::unavailable(id, &self.context());
                let button = ui.add_enabled(reason.is_none(), egui::Button::new(command.label));
                if button.clicked() {
                    self.execute(id, ui.ctx());
                    ui.close();
                }
                if let Some(reason) = reason {
                    button.on_disabled_hover_text(reason);
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
        let visible: BTreeSet<_> = self
            .spatial
            .visible(self.camera.visible_rect().inflate(20.0 / self.camera.zoom))
            .into_iter()
            .collect();
        let mut hasher = DefaultHasher::new();
        self.generation.hash(&mut hasher);
        self.theme.dark.hash(&mut hasher);
        self.theme.contrast.hash(&mut hasher);
        (self.lod.level() as u8).hash(&mut hasher);
        visible.hash(&mut hasher);
        self.selection.targets.hash(&mut hasher);
        if let Some(ids) = &self.dependencies {
            ids.hash(&mut hasher);
        }
        let key = hasher.finish();
        if self.batch_key != Some(key) {
            self.batch = Arc::new(self.make_batch(key, &visible));
            self.batch_key = Some(key);
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
        self.timing.total_nodes = self.scene.nodes.len();
        let objects = self.lookup.visible(&self.scene, &visible);
        for node in &objects.nodes {
            let target = if node.is_container {
                SceneTarget::Container(node.id())
            } else {
                SceneTarget::Node(node.id())
            };
            if !visible.contains(&target) {
                continue;
            }
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
            if self.lod.level() >= LodLevel::Summary || node.is_container {
                let title_size = if node.is_container { 17.0 } else { 16.0 };
                let top = bounds.min
                    + Vec2::new(
                        inset,
                        if node.is_container {
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
                elided(
                    &painter,
                    top,
                    &label,
                    (title_size * scale).clamp(10.0, 26.0),
                    text,
                    (bounds.width() - 2.0 * inset).max(10.0),
                );
                if node.is_container && self.lod.level() >= LodLevel::Summary {
                    painter.text(
                        bounds.min + Vec2::new(inset, 40.0 * scale),
                        Align2::LEFT_TOP,
                        format!("{} components", node.semantic.counts.parts),
                        FontId::proportional((10.0 * scale).clamp(9.0, 14.0)),
                        theme.muted,
                    );
                }
                if !node.is_container && self.lod.level() >= LodLevel::Summary {
                    painter.text(
                        bounds.min + Vec2::new(inset, 15.0 * scale),
                        Align2::LEFT_TOP,
                        format!(
                            "{}   {}",
                            category_icon(node.category),
                            node.category.label()
                        ),
                        FontId::proportional((9.0 * scale).clamp(8.0, 14.0)),
                        if faded {
                            theme.muted.gamma_multiply(0.5)
                        } else {
                            category_color(node.category, theme)
                        },
                    );
                }
                if self.lod.level() >= LodLevel::Features && !node.is_container {
                    let subtitle = if node.semantic.counts.ports > 0 {
                        format!(
                            "{} ports   ·   {} parts",
                            node.semantic.counts.ports, node.semantic.counts.parts
                        )
                    } else {
                        format!(
                            "{:?}  ·  {}",
                            node.semantic.origin,
                            if node.semantic.source_available {
                                "source-backed"
                            } else {
                                "projection"
                            }
                        )
                    };
                    elided(
                        &painter,
                        bounds.min + Vec2::new(inset, 69.0 * scale),
                        &subtitle,
                        (11.0 * scale).clamp(9.0, 16.0),
                        theme.muted,
                        (bounds.width() - 2.0 * inset).max(10.0),
                    );
                    if self.lod.level() >= LodLevel::Relationships {
                        for (index, feature) in node.semantic.features.iter().take(2).enumerate() {
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
                            node.semantic.name, node.semantic.semantic_kind, node.semantic.origin
                        ),
                    )
                });
            }
        }
        for edge in &objects.edges {
            if !self
                .selection
                .targets
                .contains(&SceneTarget::Edge(edge.semantic.id.clone()))
            {
                continue;
            }
            if let Some(segment) = edge.points.windows(2).max_by(|a, b| {
                ((a[1].x - a[0].x).hypot(a[1].y - a[0].y))
                    .total_cmp(&((b[1].x - b[0].x).hypot(b[1].y - b[0].y)))
            }) {
                let center = self.camera.world_to_screen(Point::new(
                    (segment[0].x + segment[1].x) * 0.5,
                    (segment[0].y + segment[1].y) * 0.5,
                ));
                let position = rect.min + Vec2::new(center.x, center.y - 14.0);
                let label = format!(
                    "{}{}",
                    edge.semantic.label,
                    if edge.semantic.origin == ViewOrigin::Derived {
                        " · derived"
                    } else {
                        ""
                    }
                );
                let galley =
                    painter.layout_no_wrap(label, FontId::proportional(12.0), theme.accent);
                let label_rect =
                    egui::Rect::from_center_size(position, galley.size() + Vec2::new(16.0, 8.0));
                painter.rect(
                    label_rect,
                    4.0,
                    theme.canvas,
                    Stroke::new(1.0, theme.border),
                    egui::StrokeKind::Inside,
                );
                painter.galley(label_rect.min + Vec2::new(8.0, 4.0), galley, theme.accent);
            }
        }
        if self.lod.level() >= LodLevel::Features {
            for port in &objects.ports {
                if !visible.contains(&SceneTarget::Port(port.id)) {
                    continue;
                }
                if self.lod.level() >= LodLevel::Relationships || self.selection.contains(port.id) {
                    let point = self.camera.world_to_screen(port.position);
                    let position = rect.min + Vec2::new(point.x, point.y);
                    let (offset, align) = if port.side == PortSide::Left {
                        (Vec2::new(-10.0, -10.0), Align2::RIGHT_BOTTOM)
                    } else {
                        (Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM)
                    };
                    painter.text(
                        position + offset,
                        align,
                        &port.name,
                        FontId::proportional(11.0),
                        theme.muted,
                    );
                }
            }
        }
        if let Some(target) = hovered {
            let label = match &target {
                SceneTarget::Node(id) | SceneTarget::Container(id) => self
                    .scene
                    .node(*id)
                    .map(|n| format!("{} · {}", n.semantic.name, n.semantic.semantic_kind)),
                SceneTarget::Port(id) => self
                    .scene
                    .ports
                    .iter()
                    .find(|p| p.id == *id)
                    .map(|p| format!("{} · Port · {:?}", p.name, p.direction)),
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
        let caption = if self.fixture.is_some() {
            "DETERMINISTIC VISUAL FIXTURE"
        } else {
            "REVISION-BOUND SEMANTIC WORLD"
        };
        painter.text(
            rect.left_bottom() + Vec2::new(24.0, -23.0),
            Align2::LEFT_BOTTOM,
            caption,
            FontId::proportional(10.0),
            theme.muted.gamma_multiply(0.8),
        );
        painter.text(
            rect.right_bottom() + Vec2::new(-24.0, -23.0),
            Align2::RIGHT_BOTTOM,
            "Drag to pan   ·   Wheel to zoom   ·   F to focus",
            FontId::proportional(11.0),
            theme.muted,
        );
        if self.scene.nodes.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "No elements in this view",
                FontId::proportional(20.0),
                theme.muted,
            );
        }
    }
    fn make_batch(&self, key: u64, visible: &BTreeSet<SceneTarget>) -> Batch {
        let mut batch = Batch {
            key,
            ..Default::default()
        };
        let theme = self.theme;
        let objects = self.lookup.visible(&self.scene, visible);
        for node in &objects.nodes {
            let target = if node.is_container {
                SceneTarget::Container(node.id())
            } else {
                SceneTarget::Node(node.id())
            };
            if !visible.contains(&target) {
                continue;
            }
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
                if theme.dark {
                    Color32::from_rgb(21, 28, 37)
                } else {
                    Color32::from_rgb(241, 245, 249)
                }
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
            if !visible.contains(&SceneTarget::Edge(edge.semantic.id.clone())) {
                continue;
            }
            // Ownership is already explicit spatially; graph mode exposes its actual edges.
            if self.world == crate::navigation::World::System
                && edge.semantic.family == RelationshipFamily::Ownership
            {
                continue;
            }
            let selected = self
                .selection
                .targets
                .contains(&SceneTarget::Edge(edge.semantic.id.clone()));
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
                    .gamma_multiply(if theme.contrast { 0.95 } else { 0.50 })
            };
            if edge.diff == DiffMark::Added {
                color = theme.green;
            } else if edge.diff == DiffMark::Removed {
                color = theme.amber.gamma_multiply(0.6);
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
            for port in &objects.ports {
                if !visible.contains(&SceneTarget::Port(port.id)) {
                    continue;
                }
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
            }
        }
        batch
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
fn elided(
    painter: &egui::Painter,
    position: egui::Pos2,
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
) {
    let mut job =
        egui::text::LayoutJob::simple_singleline(text.into(), FontId::proportional(size), color);
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    painter.galley(position, painter.layout_job(job), color);
}
