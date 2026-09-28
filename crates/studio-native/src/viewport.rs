//! The Surface: GPU drawing of the scene, labels, and direct manipulation.
//!
//! Click selects (Shift adds), drag on the background pans (Shift+drag
//! selects an area), drag a card to move it on the Surface (layout only),
//! Alt+drag a card onto a container to move it into that container, drag
//! from a port to a port to connect them, double-click to rename.
use crate::{
    app::StudioApp,
    commands::{self, CommandId},
    gpu::{Batch, Quad, SceneCallback},
    navigation::SurfaceView,
    theme::{self, Theme},
};
use agq_language::Parent;
use agq_studio_scene::{
    DiffMark, EdgeKind, ElementId, LockMark, LodLevel, NodeCategory, Point, PortDirection,
    PortSide, Rect, SceneTarget, Size, VisibleScene,
};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, Vec2};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Instant,
};

/// A drag in progress on the Surface.
#[derive(Clone, Debug)]
pub enum Gesture {
    Pan,
    Marquee {
        start: Point,
        end: Point,
    },
    /// Moving a card: presentation only unless dropped with Alt on a container.
    Move {
        card: ElementId,
        start: Point,
        now: Point,
    },
    /// Dragging from a port to connect it.
    Connect {
        card: ElementId,
        port: ElementId,
        now: Point,
    },
}

impl StudioApp {
    fn to_screen(&self, rect: egui::Rect, point: Point) -> egui::Pos2 {
        let p = self.camera.world_to_screen(point);
        rect.min + Vec2::new(p.x, p.y)
    }

    fn screen_bounds(&self, rect: egui::Rect, bounds: Rect) -> egui::Rect {
        egui::Rect::from_min_max(
            self.to_screen(rect, bounds.min),
            self.to_screen(rect, bounds.max),
        )
    }

    /// Whether an element changed a moment ago and is highlighted.
    fn highlighted(&self, id: ElementId) -> bool {
        self.highlights.contains_key(&id)
    }

    fn port_visible(&self, card: ElementId, port: ElementId) -> bool {
        self.lod.level() >= LodLevel::Summary
            || self.selection.contains(port)
            || self.selection.contains(card)
            || self.highlights.contains_key(&port)
    }

    /// Zoom out, the zoom level (click: 100%), zoom in and fit, in the
    /// Surface's bottom-right corner (§3.2 Surface: zoom controls).
    fn zoom_control(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let theme = self.theme;
        let mut chosen = None;
        egui::Area::new(egui::Id::new("zoom-control"))
            .fixed_pos(rect.right_bottom() - Vec2::new(212.0, 44.0))
            .order(egui::Order::Middle)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(theme.elevated)
                    .stroke(egui::Stroke::new(1.0, theme.border))
                    .corner_radius(crate::tokens::radius::MENU as u8)
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let percent = format!("{:.0}%", self.camera.zoom * 100.0);
                            for (label, command, hover) in [
                                ("−", CommandId::ZoomOut, "Zoom out (-)"),
                                (
                                    percent.as_str(),
                                    CommandId::ZoomReset,
                                    "Zoom to 100% (Shift+0)",
                                ),
                                ("+", CommandId::ZoomIn, "Zoom in (+)"),
                                ("Fit", CommandId::Fit, "Fit to view (Shift+1)"),
                            ] {
                                if ui
                                    .add(
                                        egui::Button::new(label)
                                            .frame(false)
                                            .min_size(Vec2::new(32.0, 24.0)),
                                    )
                                    .on_hover_text(hover)
                                    .clicked()
                                {
                                    chosen = Some(command);
                                }
                            }
                        });
                    });
            });
        if let Some(command) = chosen {
            self.execute(command, ui.ctx());
        }
    }

    pub fn viewport(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(1.0)),
            Sense::click_and_drag(),
        );
        crate::targets::record(ui.ctx(), crate::targets::Target::Viewport, rect);
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("studio-viewport-rect"), rect));
        self.camera.viewport = Size::new(rect.width(), rect.height());
        // Fit once the Surface has kept its size for two frames: a window's
        // first frame can report a size it never shows (with a UI scale).
        let stable = self.surface_size.replace(rect.size()) == Some(rect.size());
        if self.fit_pending && stable {
            self.frame_all();
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
        let world = pointer.map(|p| {
            self.camera
                .screen_to_world(Point::new(p.x - rect.left(), p.y - rect.top()))
        });
        let mut hovered = None;
        if (response.hovered() || response.dragged())
            && let Some(point) = world
        {
            let started = Instant::now();
            hovered = self.spatial.hit_test(point, 6.0 / self.camera.zoom);
            // Hidden ports are represented by their card at low detail.
            if let Some(SceneTarget::Port(card, port)) = hovered
                && !self.port_visible(card, port)
            {
                hovered = Some(SceneTarget::Node(card));
            }
            self.timing.hit(started.elapsed());
        }
        self.gestures(ui, &response, hovered.as_ref(), world);
        let shared = self.shared_definition().and_then(|definition| {
            let tree = self.project.as_ref()?.state().tree();
            Some(crate::edit::display_name(tree, definition))
        });
        response.context_menu(|ui| {
            for id in context_commands(self.selection.primary.as_ref()) {
                if commands::unavailable(*id, &self.context()).is_some() {
                    continue;
                }
                let command = commands::command(*id);
                let label = match (&shared, id) {
                    (Some(definition), CommandId::Rename | CommandId::Delete | CommandId::Lock) => {
                        format!(
                            "{} in {definition} (shared)    {}",
                            command.label, command.shortcut
                        )
                    }
                    _ => format!("{}    {}", command.label, command.shortcut),
                };
                if ui.button(label).clicked() {
                    self.execute(*id, ui.ctx());
                    ui.close();
                }
            }
        });
        self.zoom_control(ui, rect);
        let painter = ui.painter_at(rect);
        let theme = self.theme;
        // A dot grid follows the camera and disappears when too dense.
        let spacing = 80.0 * self.camera.zoom;
        if spacing >= 18.0 {
            let origin = self.camera.world_to_screen(Point::default());
            let mut y = rect.top() + origin.y.rem_euclid(spacing);
            while y < rect.bottom() {
                let mut x = rect.left() + origin.x.rem_euclid(spacing);
                while x < rect.right() {
                    painter.circle_filled(egui::pos2(x, y), 0.8, theme.border.gamma_multiply(0.6));
                    x += spacing;
                }
                y += spacing;
            }
        }
        let visibility = Instant::now();
        let view = self.camera.visible_rect().inflate(20.0 / self.camera.zoom);
        let objects = self.spatial.visible_scene(&self.scene, view);
        // The batch is in world coordinates and depends on the camera only
        // through the level of detail, so it is built for the view with a
        // margin of half its size on each side and reused while the view
        // stays inside: panning and zooming in do not rebuild it (W5.5).
        let mut hasher = DefaultHasher::new();
        self.scene.generation.hash(&mut hasher);
        self.theme.dark.hash(&mut hasher);
        self.theme.contrast.hash(&mut hasher);
        (self.lod.level() as u8).hash(&mut hasher);
        self.selection.targets.hash(&mut hasher);
        for (id, started) in &self.highlights {
            id.hash(&mut hasher);
            started.to_bits().hash(&mut hasher);
        }
        let key = hasher.finish();
        self.timing.visibility(visibility.elapsed());
        let covered = self
            .batch_region
            .is_some_and(|region| region.contains_rect(view));
        if self.batch_key != Some(key) || !covered {
            let started = Instant::now();
            let region = view.inflate(0.5 * view.width().max(view.height()));
            let wide = self.spatial.visible_scene(&self.scene, region);
            self.batch = Arc::new(self.make_batch(key, &wide));
            self.batch_key = Some(key);
            self.batch_region = Some(region);
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
        let labels = Instant::now();
        self.timing.total_nodes = self.scene.nodes.len();
        self.timing.visible_nodes = objects.nodes.len();
        self.node_labels(&painter, rect, &objects);
        self.edge_labels(&painter, rect, &objects, hovered.as_ref());
        self.port_labels(&painter, rect, &objects);
        self.timing.labels(labels.elapsed());
        self.gesture_overlay(&painter, rect);
        if let Some(target) = &hovered
            && self.gesture.is_none()
            && let Some(text) = self.hover_text(target)
        {
            response.clone().on_hover_text(text);
        }
        self.name_for_screen_readers(ui, &response, rect, hovered.as_ref());
        if let Some(comparison) = &self.comparison {
            // A tag above the Surface's content, like a raised label.
            let text = format!(
                "What changed · {} → {}   ·   + new   ~ changed   − deleted",
                comparison.before_label, comparison.after_label
            );
            let galley = painter.layout_no_wrap(text, theme::medium(theme::LABEL), theme.text);
            let tag = egui::Rect::from_center_size(
                rect.center_top() + Vec2::new(0.0, theme::SPACE_XL + galley.size().y * 0.5),
                galley.size() + Vec2::new(2.0 * theme::SPACE_L, 2.0 * theme::SPACE_S + 2.0),
            );
            painter.rect(
                tag,
                theme::RADIUS,
                theme.elevated,
                Stroke::new(theme::HAIRLINE, theme.border_strong),
                egui::StrokeKind::Inside,
            );
            painter.galley(tag.center() - galley.size() * 0.5, galley, theme.text);
        }
        if self.scene.nodes.is_empty() {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                if self.editable() {
                    "Empty model · press P to create a part"
                } else {
                    "Nothing to show in this view"
                },
                theme::regular(theme::HEADING),
                theme.muted,
            );
        }
    }

    fn gestures(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        hovered: Option<&SceneTarget>,
        world: Option<Point>,
    ) {
        let modifiers = ui.input(|i| i.modifiers);
        // A dialog waits for an answer: the Surface takes no input.
        if self.dialog.is_some() {
            return;
        }
        if response.drag_started()
            && let Some(point) = world
        {
            self.camera_target = None;
            // The press position decides what is dragged.
            let press = ui
                .input(|i| i.pointer.press_origin())
                .map(|p| {
                    let rect = response.rect;
                    self.camera
                        .screen_to_world(Point::new(p.x - rect.left(), p.y - rect.top()))
                })
                .unwrap_or(point);
            let target = self.spatial.hit_test(press, 6.0 / self.camera.zoom);
            self.gesture = Some(match target {
                _ if modifiers.shift => Gesture::Marquee {
                    start: press,
                    end: point,
                },
                Some(SceneTarget::Port(card, port))
                    if self.editable() && self.port_visible(card, port) =>
                {
                    Gesture::Connect {
                        card,
                        port,
                        now: point,
                    }
                }
                Some(SceneTarget::Node(card) | SceneTarget::Container(card))
                    if self.view != SurfaceView::Requirements =>
                {
                    Gesture::Move {
                        card,
                        start: press,
                        now: point,
                    }
                }
                _ => Gesture::Pan,
            });
        }
        if response.dragged() {
            self.timing.input(crate::timing::InputKind::Pan);
            let delta = ui.input(|i| i.pointer.delta());
            match (&mut self.gesture, world) {
                (Some(Gesture::Pan), _) | (None, _) => {
                    self.camera.pan_screen(Point::new(delta.x, delta.y));
                }
                (Some(Gesture::Marquee { end, .. }), Some(point)) => *end = point,
                (Some(Gesture::Move { now, .. } | Gesture::Connect { now, .. }), Some(point)) => {
                    *now = point
                }
                _ => {}
            }
            ui.ctx().request_repaint();
        }
        if response.drag_stopped() {
            match self.gesture.take() {
                Some(Gesture::Marquee { start, end }) => {
                    let targets = self.spatial.marquee(Rect::from_points(start, end));
                    self.selection.replace(targets);
                    self.batch_key = None;
                }
                Some(Gesture::Move { card, start, now }) => {
                    self.drop_card(card, start, now, modifiers.alt, hovered)
                }
                Some(Gesture::Connect { card, port, .. }) => match hovered {
                    Some(SceneTarget::Port(other_card, other)) if *other != port => {
                        self.connect((card, Some(port)), (*other_card, Some(*other)))
                    }
                    Some(SceneTarget::Node(other) | SceneTarget::Container(other))
                        if *other != card =>
                    {
                        self.connect((card, Some(port)), (*other, None))
                    }
                    _ => self.status = "Drop on another port to connect".into(),
                },
                _ => {}
            }
        }
        if response.clicked() {
            self.timing.input(crate::timing::InputKind::Selection);
            let double = self.canvas_clicks.click(
                self.scene.generation,
                hovered,
                response
                    .interact_pointer_pos()
                    .map_or([0.0, 0.0], |p| [p.x, p.y]),
                ui.input(|i| i.time),
            );
            match hovered {
                Some(target) => {
                    self.select(target.clone(), modifiers.shift);
                    // A click on an attribute or item line works on that line.
                    if let (SceneTarget::Node(card), Some(point)) = (target, world)
                        && let Some(feature) = self.feature_at(*card, point)
                    {
                        self.inspected = Some((self.selection.primary.clone(), feature));
                    }
                    if double {
                        let id = if self.editable() {
                            CommandId::Rename
                        } else {
                            CommandId::Focus
                        };
                        self.execute(id, ui.ctx());
                    }
                }
                None if !modifiers.shift => {
                    self.selection.clear();
                    self.batch_key = None;
                }
                None => {}
            }
        }
        if response.secondary_clicked() {
            match hovered {
                Some(target) if !self.selection.targets.contains(target) => {
                    self.select(target.clone(), false)
                }
                Some(_) => {}
                None => self.selection.clear(),
            }
            self.batch_key = None;
        }
    }

    /// Ends a card drag: Alt over a container moves the element into it;
    /// otherwise the card keeps its new place on the Surface (layout only).
    fn drop_card(
        &mut self,
        card: ElementId,
        start: Point,
        now: Point,
        alt: bool,
        hovered: Option<&SceneTarget>,
    ) {
        if alt {
            let into = match hovered {
                Some(SceneTarget::Node(id) | SceneTarget::Container(id)) if *id != card => {
                    Some(*id)
                }
                _ => None,
            };
            match into {
                Some(container) if self.editable() => {
                    self.move_to(card, Parent::Element(container))
                }
                _ => self.status = "Hold Alt and drop onto a container to move into it".into(),
            }
            return;
        }
        let dx = now.x - start.x;
        let dy = now.y - start.y;
        if dx.abs() + dy.abs() < 2.0 {
            return;
        }
        let mut moved = vec![card];
        let mut index = 0;
        while index < moved.len() {
            let owner = moved[index];
            moved.extend(
                self.scene
                    .nodes
                    .iter()
                    .filter(|n| n.semantic.owner == Some(owner))
                    .map(|n| n.id()),
            );
            index += 1;
        }
        let memory = self.layouts.entry(self.view).or_default();
        for id in moved {
            if let Some(node) = self.scene.nodes.iter().find(|n| n.id() == id) {
                memory
                    .bounds
                    .insert(id, node.bounds.translate(Point::new(dx, dy)));
                if memory.is_pinned(id) {
                    let _ = memory.pin(id, node.bounds.translate(Point::new(dx, dy)));
                }
            }
        }
        self.rebuild();
    }

    fn gesture_overlay(&self, painter: &egui::Painter, rect: egui::Rect) {
        let theme = self.theme;
        match &self.gesture {
            Some(Gesture::Marquee { start, end }) => {
                let area = egui::Rect::from_two_pos(
                    self.to_screen(rect, *start),
                    self.to_screen(rect, *end),
                );
                painter.rect(
                    area,
                    0.0,
                    theme.accent.gamma_multiply(0.10),
                    Stroke::new(1.0, theme.accent),
                    egui::StrokeKind::Inside,
                );
            }
            Some(Gesture::Move { card, start, now }) => {
                if let Some(node) = self.lookup.node(&self.scene, *card) {
                    let moved = node
                        .bounds
                        .translate(Point::new(now.x - start.x, now.y - start.y));
                    painter.rect(
                        self.screen_bounds(rect, moved),
                        8.0,
                        theme.accent.gamma_multiply(0.08),
                        Stroke::new(1.5, theme.accent),
                        egui::StrokeKind::Inside,
                    );
                }
            }
            Some(Gesture::Connect { card, port, now }) => {
                if let Some(from) = self.lookup.port(&self.scene, *card, *port) {
                    painter.line_segment(
                        [
                            self.to_screen(rect, from.position),
                            self.to_screen(rect, *now),
                        ],
                        Stroke::new(2.0, theme.accent),
                    );
                    painter.circle_filled(self.to_screen(rect, *now), 4.0, theme.accent);
                }
            }
            _ => {}
        }
    }

    /// Screen-reader names (ROADMAP §3.5): the Surface, and as its items the
    /// selected elements and the card under the pointer, which the GPU draws
    /// without widgets.
    fn name_for_screen_readers(
        &self,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: egui::Rect,
        hovered: Option<&SceneTarget>,
    ) {
        use egui::accesskit::Role;
        if !crate::accessibility::enabled(ui) {
            return;
        }
        let selected = self.selection.targets.len();
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(Role::List);
            node.set_label(format!(
                "Surface: {} elements, {selected} selected",
                self.scene.nodes.len()
            ));
        });
        let named = self
            .selection
            .targets
            .iter()
            .chain(hovered.filter(|target| !self.selection.targets.contains(*target)));
        for target in named {
            let Some(bounds) = self.scene.target_bounds(target) else {
                continue;
            };
            let selected = self.selection.targets.contains(target);
            crate::accessibility::name_rect(
                ui,
                response.id.with(("element", target)),
                self.screen_bounds(rect, bounds),
                Role::ListItem,
                Some(selected),
                || {
                    let text = self.hover_text(target).unwrap_or_default();
                    // One line: the hints after the first line are for sighted use.
                    let mut label = text.lines().next().unwrap_or_default().to_string();
                    if selected {
                        label.push_str(", selected");
                    }
                    label
                },
            );
        }
    }

    fn hover_text(&self, target: &SceneTarget) -> Option<String> {
        match target {
            SceneTarget::Node(id) | SceneTarget::Container(id) => {
                self.lookup.node(&self.scene, *id).map(|n| {
                    let mut text = format!("{} {}", n.semantic.keyword, n.semantic.name);
                    if !n.semantic.detail.is_empty() {
                        text.push_str(&format!(" {}", n.semantic.detail));
                    }
                    match n.semantic.lock {
                        LockMark::Own => text.push_str("\nLocked"),
                        LockMark::Covered => text.push_str("\nLocked with its owner"),
                        LockMark::None => {}
                    }
                    if n.semantic.problems > 0 {
                        text.push_str(&format!(
                            "\n{} problem(s); see the Inspector",
                            n.semantic.problems
                        ));
                    }
                    text
                })
            }
            SceneTarget::Port(card, id) => self.lookup.port(&self.scene, *card, *id).map(|p| {
                let shared = p
                    .defined_in
                    .and_then(|d| self.lookup.node(&self.scene, d))
                    .map(|d| format!("\nDefined in {}: changes apply to it", d.semantic.name))
                    .unwrap_or_default();
                format!(
                    "port {} · direction {}{shared}\nDrag to another port to connect",
                    p.name,
                    direction_label(p.direction)
                )
            }),
            SceneTarget::Edge(id) => self
                .scene
                .edges
                .iter()
                .find(|e| e.semantic.id == *id)
                .map(|e| format!("{} · {}", e.semantic.kind.label(), e.semantic.label)),
        }
    }

    /// Card text, laid out top to bottom in screen space so nothing overlaps
    /// at any zoom: a marks row (kind, change, problems, lock), the name on
    /// up to two lines, the type, and the attribute and item lines at the
    /// bottom. Text that does not fit is left out rather than overprinted.
    fn node_labels(&self, painter: &egui::Painter, rect: egui::Rect, objects: &VisibleScene<'_>) {
        let theme = self.theme;
        let scale = self.camera.zoom;
        let inspected = self.inspected_element();
        for node in &objects.nodes {
            let bounds = self.screen_bounds(rect, node.bounds);
            if bounds.width() < 36.0 || bounds.height() < 16.0 {
                continue;
            }
            let pad = (14.0 * scale).clamp(5.0, 20.0);
            let width = (bounds.width() - 2.0 * pad).max(10.0);
            let removed = node.diff == DiffMark::Removed;
            let text = if removed { theme.muted } else { theme.text };
            // No text is drawn below the caption size: rows that would need
            // smaller text are left out at this zoom.
            let caption = (9.5 * scale).clamp(theme::CAPTION, 13.0);
            let name_size =
                ((if node.is_container { 17.0 } else { 16.0 }) * scale).clamp(11.0, 26.0);
            let mut y = bounds.top() + (8.0 * scale).clamp(3.0, 12.0);
            // The marks row: kind on the left; change, problems, lock on the right.
            let marks_row = bounds.height() >= caption + name_size + 2.0 * pad;
            if marks_row {
                let mut right = bounds.right() - pad;
                let middle = y + caption * 0.5;
                if node.diff != DiffMark::Unchanged {
                    let (badge, color) = match node.diff {
                        DiffMark::Added => ("+ NEW", theme.green),
                        DiffMark::Removed => ("− DELETED", theme.muted),
                        _ => ("~ CHANGED", theme.violet),
                    };
                    let placed = painter.text(
                        egui::pos2(right, middle),
                        Align2::RIGHT_CENTER,
                        badge,
                        theme::semibold(caption),
                        color,
                    );
                    right = placed.left() - caption * 0.6;
                }
                if node.semantic.problems > 0 {
                    let center = egui::pos2(right - caption * 0.55, middle);
                    painter.circle_filled(center, caption * 0.62, theme.amber);
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        "!",
                        theme::semibold(caption),
                        theme.canvas,
                    );
                    let count = painter.text(
                        egui::pos2(center.x - caption * 0.9, middle),
                        Align2::RIGHT_CENTER,
                        node.semantic.problems.to_string(),
                        theme::medium(caption),
                        theme.amber,
                    );
                    right = count.left() - caption * 0.6;
                }
                if node.semantic.lock.locked() {
                    let size = (caption * 1.3).max(theme::LOCK_MARK);
                    lock_mark(
                        painter,
                        egui::pos2(right - size * 0.5, middle),
                        size,
                        node.semantic.lock,
                        theme,
                    );
                    right -= size + theme::SPACE_S;
                }
                bounded_label(
                    painter,
                    egui::pos2(bounds.left() + pad, y),
                    &node.semantic.keyword.to_uppercase(),
                    theme::semibold(caption),
                    category_color(node.category, theme),
                    (right - bounds.left() - 2.0 * pad).max(10.0),
                    1,
                );
                y += caption + 5.0 * scale.clamp(0.5, 1.5);
            }
            let label = if node.is_container {
                format!(
                    "{}  {}",
                    if node.collapsed { "▸" } else { "▾" },
                    node.semantic.name
                )
            } else {
                node.semantic.name.clone()
            };
            let room = bounds.bottom() - y - 2.0;
            if room < name_size {
                continue;
            }
            // Names wrap only between words; a single word is elided.
            let rows = if room >= name_size * 2.6 && !node.is_container && label.contains(' ') {
                2
            } else {
                1
            };
            let name = bounded_label(
                painter,
                egui::pos2(bounds.left() + pad, y),
                &label,
                theme::medium(name_size),
                text,
                width,
                rows,
            );
            y = name.bottom() + 4.0 * scale.clamp(0.5, 1.5);
            let features_top = bounds.top()
                + (node.bounds.height()
                    - agq_studio_scene::feature_block(node.semantic.features.len()))
                    * scale;
            let small = (11.0 * scale).clamp(theme::CAPTION, 16.0);
            if self.lod.level() >= LodLevel::Features
                && !node.semantic.detail.is_empty()
                && y + small <= features_top.min(bounds.bottom() - 2.0)
            {
                bounded_label(
                    painter,
                    egui::pos2(bounds.left() + pad, y),
                    &node.semantic.detail,
                    theme::regular(small),
                    theme.muted,
                    width,
                    1,
                );
            }
            let line = agq_studio_scene::FEATURE_LINE * scale;
            if self.lod.level() >= LodLevel::Features
                && !node.is_container
                && line >= theme::CAPTION + 2.0
            {
                let size = (10.5 * scale).min(line * 0.8).clamp(theme::CAPTION, 15.0);
                let shown = node
                    .semantic
                    .features
                    .len()
                    .min(agq_studio_scene::MAX_FEATURE_LINES);
                let more = node.semantic.features.len() - shown;
                for (index, feature) in node.semantic.features.iter().take(shown).enumerate() {
                    let top = features_top + index as f32 * line;
                    if top < y {
                        continue;
                    }
                    let last = index + 1 == shown && more > 0;
                    let text = if last {
                        format!("+{} more", more + 1)
                    } else {
                        feature.text.clone()
                    };
                    let color = if inspected == Some(feature.id) && !last {
                        theme.accent
                    } else if feature.problems > 0 && !last {
                        theme.amber
                    } else {
                        theme.muted
                    };
                    let lock_room = if feature.lock.locked() {
                        size * 1.6
                    } else {
                        0.0
                    };
                    bounded_label(
                        painter,
                        egui::pos2(bounds.left() + pad, top),
                        &text,
                        theme::regular(size),
                        color,
                        (width - lock_room).max(10.0),
                        1,
                    );
                    if feature.lock.locked() && !last {
                        lock_mark(
                            painter,
                            egui::pos2(bounds.right() - pad - 6.0, top + size * 0.6),
                            theme::LOCK_MARK,
                            feature.lock,
                            theme,
                        );
                    }
                }
            }
        }
    }

    /// The attribute or item line of a card under a Surface point, if any.
    pub fn feature_at(&self, card: ElementId, point: Point) -> Option<ElementId> {
        let node = self.lookup.node(&self.scene, card)?;
        if node.is_container {
            return None;
        }
        let top = node.bounds.max.y - agq_studio_scene::feature_block(node.semantic.features.len());
        if point.y < top {
            return None;
        }
        let index = ((point.y - top) / agq_studio_scene::FEATURE_LINE).floor() as usize;
        let shown = node
            .semantic
            .features
            .len()
            .min(agq_studio_scene::MAX_FEATURE_LINES);
        (index < shown).then(|| node.semantic.features[index].id)
    }

    fn edge_labels(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        objects: &VisibleScene<'_>,
        hovered: Option<&SceneTarget>,
    ) {
        let theme = self.theme;
        if self.lod.level() < LodLevel::Features && self.selection.targets.is_empty() {
            return;
        }
        let obstacles: Vec<egui::Rect> = objects
            .nodes
            .iter()
            .map(|node| {
                let mut bounds = self.screen_bounds(rect, node.bounds);
                if node.is_container && !node.collapsed {
                    bounds.max.y = bounds.max.y.min(bounds.min.y + 58.0 * self.camera.zoom);
                }
                bounds
            })
            .collect();
        let mut routes = crate::relationship_labels::RouteObstacles::default();
        for edge in &objects.edges {
            for pair in edge.points.windows(2) {
                routes.insert(
                    self.to_screen(rect, pair[0]),
                    self.to_screen(rect, pair[1]),
                    rect,
                );
            }
        }
        let mut placed = Vec::new();
        let mut automatic = 0;
        for edge in &objects.edges {
            let target = SceneTarget::Edge(edge.semantic.id.clone());
            let explicit = self.selection.targets.contains(&target) || hovered == Some(&target);
            let incident = self.selection.contains(edge.semantic.source.node)
                || self.selection.contains(edge.semantic.target.node)
                || edge
                    .semantic
                    .element
                    .is_some_and(|e| self.highlights.contains_key(&e));
            let shown = explicit
                || edge.semantic.lock.locked()
                || (incident && objects.edges.len() <= 40)
                || (self.lod.level() >= LodLevel::Relationships && objects.edges.len() <= 16);
            if !shown || (!explicit && automatic >= 10) {
                continue;
            }
            let galley = crate::relationship_labels::layout(
                painter,
                edge.semantic.label.clone(),
                (rect.width() * 0.3).clamp(80.0, 280.0),
            );
            let route: Vec<_> = edge
                .points
                .iter()
                .map(|p| self.to_screen(rect, *p))
                .collect();
            let lock_room = if edge.semantic.lock.locked() {
                theme::LOCK_MARK + theme::SPACE_S
            } else {
                0.0
            };
            let Some(placement) = crate::relationship_labels::place(
                &route,
                galley.size()
                    + 2.0 * crate::relationship_labels::PADDING
                    + Vec2::new(lock_room, 0.0),
                rect,
                &obstacles,
                &placed,
                explicit,
                &routes,
            ) else {
                continue;
            };
            let label = placement.bounds;
            crate::relationship_labels::paint(painter, &placement, galley, explicit, theme);
            if edge.semantic.lock.locked() {
                lock_mark(
                    painter,
                    egui::pos2(label.right() - theme::SPACE_S - 6.0, label.center().y),
                    theme::LOCK_MARK,
                    edge.semantic.lock,
                    theme,
                );
            }
            placed.push(label);
            automatic += usize::from(!explicit);
        }
    }

    fn port_labels(&self, painter: &egui::Painter, rect: egui::Rect, objects: &VisibleScene<'_>) {
        let theme = self.theme;
        for port in &objects.ports {
            if !self.port_visible(port.owner, port.id) {
                continue;
            }
            let selected = self
                .selection
                .targets
                .contains(&SceneTarget::Port(port.owner, port.id));
            if !(self.lod.level() >= LodLevel::Relationships
                || selected
                || self.selection.contains(port.owner)
                || self.lod.level() >= LodLevel::Features && objects.ports.len() <= 60)
            {
                continue;
            }
            let position = self.to_screen(rect, port.position);
            let owner = self.lookup.node(&self.scene, port.owner);
            let expanded = owner.is_some_and(|n| n.is_container && !n.collapsed);
            let owner_width = owner.map_or(200.0, |n| n.bounds.width() * self.camera.zoom);
            let color = if selected { theme.accent } else { theme.muted };
            let mut job = egui::text::LayoutJob::simple_singleline(
                port.name.clone(),
                theme::medium(theme::LABEL),
                color,
            );
            job.wrap.max_width = (owner_width * 0.44).max(40.0);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = false;
            job.wrap.overflow_character = Some('…');
            let galley = painter.layout_job(job);
            let header = port.label_in_header && self.camera.zoom * 24.0 >= galley.size().y;
            let origin = port_label_origin(port.side, position, galley.size(), expanded, header);
            let label = egui::Rect::from_min_size(origin, galley.size());
            if expanded && !header {
                painter.rect_filled(label.expand(2.0), 2.0, theme.surface);
            }
            painter.galley(origin, galley, color);
            if port.lock.locked() {
                // The lock mark sits beside the label, away from the port.
                let x = if label.center().x >= position.x {
                    label.right() + 8.0
                } else {
                    label.left() - 8.0
                };
                lock_mark(
                    painter,
                    egui::pos2(x, label.center().y),
                    theme::LOCK_MARK,
                    port.lock,
                    theme,
                );
            }
        }
    }

    fn make_batch(&self, key: u64, objects: &VisibleScene<'_>) -> Batch {
        let mut batch = Batch {
            key,
            ..Default::default()
        };
        let theme = self.theme;
        for node in &objects.nodes {
            let selected = self.selection.contains(node.id());
            let mut border = if selected { theme.accent } else { theme.border };
            match node.diff {
                DiffMark::Added => border = theme.green,
                DiffMark::Changed => border = theme.violet,
                _ => {}
            }
            if node.semantic.lock == LockMark::Own && !selected {
                border = theme.border_strong;
            }
            let mut fill = if node.is_container {
                theme.containment(node.depth)
            } else {
                theme.elevated
            };
            if node.diff == DiffMark::Removed {
                fill = theme.canvas;
                border = theme.error;
            }
            let r = [
                node.bounds.min.x,
                node.bounds.min.y,
                node.bounds.width(),
                node.bounds.height(),
            ];
            let radius = match node.category {
                NodeCategory::Requirement => theme::REQUIREMENT_RADIUS,
                _ if node.is_container => theme::CONTAINER_RADIUS,
                _ => theme::CARD_RADIUS,
            };
            let width = if selected || node.diff != DiffMark::Unchanged {
                theme::STROKE_SELECTED
            } else {
                theme::HAIRLINE
            };
            let mut quad = Quad::rect(r, fill, border, radius, width);
            if node.diff == DiffMark::Removed {
                quad.detail = [10.0, 6.0, 0.0, 0.0];
            }
            // A just-changed card glows; the fade runs on the GPU. Under
            // reduced motion it gets a steady halo until the highlight ends.
            if let Some(started) = self.highlights.get(&node.id()) {
                batch.overlays.push(if self.reduced_motion {
                    Quad::halo(r, radius, theme.changed)
                } else {
                    Quad::changed(r, radius, theme.changed, *started)
                });
            }
            let list = if node.is_container {
                &mut batch.containers
            } else {
                &mut batch.nodes
            };
            if selected {
                list.push(Quad::halo(r, radius, theme.accent));
            } else if node.diff == DiffMark::Added {
                list.push(Quad::halo(r, radius, theme.green));
            }
            list.push(quad);
            // A separator under the title makes the card easy to read.
            let separator_y = if node.is_container { 58.0 } else { 28.0 };
            list.push(Quad::rect(
                [r[0] + 14.0, r[1] + separator_y, (r[2] - 28.0).max(0.0), 0.8],
                theme.border.gamma_multiply(0.6),
                Color32::TRANSPARENT,
                0.0,
                0.0,
            ));
            // A problem bar along the card's left edge.
            if node.semantic.problems > 0 {
                list.push(Quad::rect(
                    [r[0] + 1.0, r[1] + 8.0, 3.0, (r[3] - 16.0).max(0.0)],
                    theme.amber,
                    Color32::TRANSPARENT,
                    1.5,
                    0.0,
                ));
            }
        }
        let selected_edges: Vec<&str> = self
            .selection
            .targets
            .iter()
            .filter_map(|t| match t {
                SceneTarget::Edge(id) => Some(id.as_str()),
                _ => None,
            })
            .collect();
        for edge in &objects.edges {
            let selected = selected_edges.contains(&edge.semantic.id.as_str());
            let incident = self.selection.contains(edge.semantic.source.node)
                || self.selection.contains(edge.semantic.target.node)
                || edge
                    .semantic
                    .source
                    .port
                    .is_some_and(|p| self.selection.contains(p))
                || edge
                    .semantic
                    .target
                    .port
                    .is_some_and(|p| self.selection.contains(p));
            let glow = edge.semantic.element.is_some_and(|e| self.highlighted(e));
            let mut color = match edge.semantic.kind {
                EdgeKind::Satisfy => theme.amber.gamma_multiply(0.8),
                EdgeKind::Typing | EdgeKind::Specialization => theme.muted.gamma_multiply(0.55),
                _ => theme.edge,
            };
            if selected || incident {
                color = theme.accent;
            }
            if edge.semantic.problems > 0 {
                color = theme.amber;
            }
            match edge.diff {
                DiffMark::Added => color = theme.green,
                DiffMark::Changed => color = theme.violet,
                DiffMark::Removed => color = theme.error,
                DiffMark::Unchanged => {}
            }
            if glow {
                color = theme.changed;
            }
            let width = if selected {
                theme::EDGE_WIDTH_SELECTED
            } else if incident || glow {
                theme::EDGE_WIDTH_INCIDENT
            } else {
                theme::EDGE_WIDTH
            };
            let dashed = matches!(
                edge.semantic.kind,
                EdgeKind::Typing | EdgeKind::Specialization | EdgeKind::Satisfy
            ) || edge.diff == DiffMark::Removed;
            for pair in edge.points.windows(2) {
                if let Some(mut quad) =
                    Quad::segment([pair[0].x, pair[0].y], [pair[1].x, pair[1].y], width, color)
                {
                    if dashed {
                        quad.detail = [10.0, 6.0, 0.0, 0.0];
                    }
                    batch.edges.push(quad);
                }
            }
            if edge.semantic.directed && edge.points.len() >= 2 {
                let end = edge.points[edge.points.len() - 1];
                let before = edge.points[edge.points.len() - 2];
                let length = (end.x - before.x).hypot(end.y - before.y).max(0.001);
                let (ux, uy) = ((end.x - before.x) / length, (end.y - before.y) / length);
                for side in [-1.0, 1.0] {
                    if let Some(quad) = Quad::segment(
                        [
                            end.x - ux * 8.0 - uy * 4.5 * side,
                            end.y - uy * 8.0 + ux * 4.5 * side,
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
        let connected: std::collections::BTreeSet<ElementId> = objects
            .edges
            .iter()
            .flat_map(|e| [e.semantic.source.port, e.semantic.target.port])
            .flatten()
            .collect();
        for port in &objects.ports {
            if !self.port_visible(port.owner, port.id) {
                continue;
            }
            let selected = self
                .selection
                .targets
                .contains(&SceneTarget::Port(port.owner, port.id));
            let glow = self.highlighted(port.id);
            let size = if selected {
                theme::PORT_SIZE + 3.0
            } else {
                theme::PORT_SIZE
            };
            let border = if selected {
                theme.accent
            } else if glow {
                theme.changed
            } else {
                match port.diff {
                    DiffMark::Added => theme.green,
                    DiffMark::Changed => theme.violet,
                    DiffMark::Removed => theme.error,
                    _ => theme.muted,
                }
            };
            batch.overlays.push(Quad::rect(
                [
                    port.position.x - size / 2.0,
                    port.position.y - size / 2.0,
                    size,
                    size,
                ],
                theme.canvas,
                border,
                theme::RADIUS_S * 0.6,
                if selected || glow {
                    theme::STROKE_SELECTED
                } else {
                    1.5
                },
            ));
            if connected.contains(&port.id) {
                batch.overlays.push(Quad::rect(
                    [port.position.x - 2.0, port.position.y - 2.0, 4.0, 4.0],
                    if selected { theme.accent } else { theme.green },
                    Color32::TRANSPARENT,
                    1.0,
                    0.0,
                ));
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
    in_header: bool,
) -> egui::Pos2 {
    if expanded_owner {
        let offset = match (side, in_header) {
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
                PortSide::Left => Vec2::new(10.0, -size.y * 0.5),
                PortSide::Right => Vec2::new(-10.0 - size.x, -size.y * 0.5),
                PortSide::Top => Vec2::new(-size.x * 0.5, -10.0 - size.y),
                PortSide::Bottom => Vec2::new(-size.x * 0.5, 10.0),
            }
    }
}

fn direction_label(direction: PortDirection) -> &'static str {
    match direction {
        PortDirection::Unspecified => "not given",
        PortDirection::In => "in",
        PortDirection::Out => "out",
        PortDirection::InOut => "inout",
    }
}

fn context_commands(target: Option<&SceneTarget>) -> &'static [CommandId] {
    use CommandId::*;
    match target {
        None => &[
            CreatePart,
            CreateRequirement,
            Fit,
            Architecture,
            Graph,
            Requirements,
        ],
        Some(SceneTarget::Port(..)) => &[Rename, Connect, Delete, Lock],
        Some(SceneTarget::Edge(_)) => &[Delete],
        Some(SceneTarget::Node(_) | SceneTarget::Container(_)) => &[
            Rename,
            CreatePart,
            CreatePort,
            CreateAttribute,
            Connect,
            MoveTo,
            Lock,
            Delete,
            Focus,
            Collapse,
            Pin,
            Unpin,
        ],
    }
}

fn category_color(category: NodeCategory, theme: crate::theme::Theme) -> Color32 {
    match category {
        NodeCategory::Requirement => theme.amber,
        NodeCategory::Definition => theme.violet,
        NodeCategory::Package => theme.muted,
        _ => theme.accent,
    }
}

/// The lock mark: full for an element that carries the lock, faint for one
/// covered by an owner's lock.
fn lock_mark(painter: &egui::Painter, center: egui::Pos2, size: f32, lock: LockMark, theme: Theme) {
    let size = size.max(theme::LOCK_MARK);
    if lock == LockMark::Own {
        theme.lock_mark(painter, center, size);
    } else {
        let mut faint = painter.clone();
        faint.set_opacity(0.45);
        theme.lock_mark(&faint, center, size);
    }
}

/// Text within a width, on at most `rows` lines; returns where it was drawn.
fn bounded_label(
    painter: &egui::Painter,
    position: egui::Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    rows: usize,
) -> egui::Rect {
    let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font, color);
    job.wrap.max_width = width;
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    let galley = painter.layout_job(job);
    let drawn = egui::Rect::from_min_size(position, galley.size());
    painter.galley(position, galley, color);
    drawn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanded_owner_labels_use_the_header_or_the_outside_not_child_cards() {
        let size = Vec2::new(92.0, 14.0);
        let zoom = 0.708;
        for side in [PortSide::Left, PortSide::Right] {
            let x = if side == PortSide::Left {
                0.0
            } else {
                844.0 * zoom
            };
            let port = egui::pos2(x, 72.0 * zoom);
            let header =
                egui::Rect::from_min_size(port_label_origin(side, port, size, true, true), size);
            assert!(header.min.x >= 0.0 && header.max.x <= 844.0 * zoom);
            let body = egui::pos2(x, 292.0 * zoom);
            let outside =
                egui::Rect::from_min_size(port_label_origin(side, body, size, true, false), size);
            if side == PortSide::Left {
                assert!(outside.max.x < 0.0);
            } else {
                assert!(outside.min.x > 844.0 * zoom);
            }
        }
    }
}
