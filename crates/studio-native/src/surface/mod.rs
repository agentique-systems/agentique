//! The Surface (§3.2 Surface): the model drawn spatially, and direct
//! manipulation of it.
//!
//! Click selects (Shift adds), drag on the background pans (Shift+drag
//! selects an area), Space+drag pans from anywhere, drag a card to move it
//! (layout only), Alt+drag a card onto a container to move it into that
//! container, drag from a port to a port or card to connect them,
//! double-click renames in place (or opens a composite's definition; Enter
//! opens it too, Backspace goes back along the breadcrumb). A building block
//! dragged from the Library (C-49) shows where it would go and which ports
//! it would fit; dropped on a card it goes inside, dropped on a fitting port
//! it goes beside that port's part, connected to it. The wheel and a pinch
//! zoom around the pointer; Shift+wheel pans across.
mod minimap;
pub(crate) mod paint;

use crate::{
    commands::{self, CommandId, Deselect, SelectDown, SelectLeft, SelectRight, SelectUp},
    navigation::SurfaceView as View,
    studio::{Dirty, Studio, StudioEvent},
    ui::{self, ActiveTheme, Button, EmptyState, IconName, TextField, r, theme},
    workspace::StudioExt,
};
use agq_language::Parent;
use agq_studio_scene::{ElementId, LockMark, Point, Rect, SceneTarget, Size};
use gpui::{
    App, AppContext, Bounds, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, KeyUpEvent, Modifiers, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PinchEvent, Pixels, Render,
    ScrollDelta, ScrollWheelEvent, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Task, Window, canvas, div, point, prelude::FluentBuilder, px,
};
use gpui_base::input::{InputEvent, InputState};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
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
    /// A building block dragged from the Library: the card it would go
    /// into and whether that card can hold it, a fitting port it would be
    /// connected to, and every port in view one of its ports fits.
    Insert {
        block: agq_library::BlockRef,
        now: Point,
        target: Option<(ElementId, bool)>,
        port: Option<(ElementId, ElementId)>,
        fitting: std::collections::BTreeSet<(ElementId, ElementId)>,
    },
}

/// What the Surface asks of the workspace.
pub enum SurfaceEvent {
    /// Open the context menu for the selection at this window position.
    ContextMenu(gpui::Point<Pixels>),
}

/// A press that has not become a drag yet.
struct Press {
    at: gpui::Point<Pixels>,
    last: gpui::Point<Pixels>,
    world: Point,
    target: Option<SceneTarget>,
    modifiers: Modifiers,
    dragging: bool,
}

/// How far the pointer moves before a press becomes a drag.
const DRAG_THRESHOLD: f32 = 3.0;
/// How long the pointer rests on an element before its details show.
const HOVER_DETAILS: Duration = Duration::from_millis(450);

pub struct SurfaceView {
    studio: Entity<Studio>,
    focus: FocusHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    pointer: Option<gpui::Point<Pixels>>,
    hovered: Option<SceneTarget>,
    hovered_since: Option<Instant>,
    hover_timer: Option<Task<()>>,
    press: Option<Press>,
    space: bool,
    last_frame: Option<Instant>,
    rename: Option<(ElementId, Entity<InputState>, Subscription)>,
    _subscription: Subscription,
}

impl EventEmitter<SurfaceEvent> for SurfaceView {}

impl Focusable for SurfaceView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SurfaceView {
    pub fn new(studio: Entity<Studio>, cx: &mut Context<SurfaceView>) -> SurfaceView {
        let subscription = cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
            if event.0.intersects(
                Dirty::CAMERA
                    | Dirty::SELECTION
                    | Dirty::MODEL
                    | Dirty::APPEARANCE
                    | Dirty::OVERLAY
                    | Dirty::LAYOUT,
            ) {
                cx.notify();
            }
        });
        SurfaceView {
            studio,
            focus: cx.focus_handle(),
            bounds: Rc::new(Cell::new(Bounds::default())),
            pointer: None,
            hovered: None,
            hovered_since: None,
            hover_timer: None,
            press: None,
            space: false,
            last_frame: None,
            rename: None,
            _subscription: subscription,
        }
    }

    /// A window position on the Surface, in world coordinates.
    fn world(&self, position: gpui::Point<Pixels>, cx: &App) -> Point {
        let origin = self.bounds.get().origin;
        self.studio.read(cx).camera.screen_to_world(Point::new(
            f32::from(position.x - origin.x),
            f32::from(position.y - origin.y),
        ))
    }

    fn hit(&self, world: Point, cx: &mut App) -> Option<SceneTarget> {
        let started = Instant::now();
        let studio = self.studio.read(cx);
        let mut hit = studio.spatial.hit_test(world, 6.0 / studio.camera.zoom);
        // Hidden ports are represented by their card at low detail.
        if let Some(SceneTarget::Port(card, port)) = hit
            && !port_visible(studio, card, port)
        {
            hit = Some(SceneTarget::Node(card));
        }
        let elapsed = started.elapsed();
        self.studio
            .update(cx, |studio, _| studio.timing.hit(elapsed));
        hit
    }

    fn set_hovered(&mut self, hovered: Option<SceneTarget>, cx: &mut Context<Self>) {
        if self.hovered != hovered {
            self.hovered = hovered;
            self.hovered_since = self.hovered.as_ref().map(|_| Instant::now());
            // Details show after a rest; a timer draws them then.
            self.hover_timer = self.hovered.as_ref().map(|_| {
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(HOVER_DETAILS).await;
                    let _ = this.update(cx, |_, cx| cx.notify());
                })
            });
            cx.notify();
        }
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        self.studio
            .update(cx, |studio, _| studio.timing.input_received());
        if self.studio.read(cx).dialog.is_some() {
            return;
        }
        let world = self.world(event.position, cx);
        let target = self.hit(world, cx);
        self.press = Some(Press {
            at: event.position,
            last: event.position,
            world,
            target,
            modifiers: event.modifiers,
            dragging: false,
        });
        self.studio.act(cx, |studio| {
            studio.camera_target = None;
            studio.mark(Dirty::CAMERA);
        });
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pointer = Some(event.position);
        let world = self.world(event.position, cx);
        let Some(press) = &mut self.press else {
            let hovered = self.hit(world, cx);
            self.set_hovered(hovered, cx);
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            // The button was released outside the window.
            self.press = None;
            self.studio.act(cx, |studio| studio.gesture = None);
            return;
        }
        let delta = event.position - press.last;
        press.last = event.position;
        if !press.dragging {
            let moved = event.position - press.at;
            if f32::from(moved.x).hypot(f32::from(moved.y)) < DRAG_THRESHOLD {
                return;
            }
            press.dragging = true;
            let (target, start, modifiers) = (press.target.clone(), press.world, press.modifiers);
            let space = self.space;
            self.studio.act(cx, |studio| {
                let editable = studio.editable();
                studio.gesture = Some(match target {
                    _ if space => Gesture::Pan,
                    _ if modifiers.shift => Gesture::Marquee { start, end: world },
                    Some(SceneTarget::Port(card, port))
                        if editable && port_visible(studio, card, port) =>
                    {
                        Gesture::Connect {
                            card,
                            port,
                            now: world,
                        }
                    }
                    Some(SceneTarget::Node(card) | SceneTarget::Container(card))
                        if studio.view != View::Requirements =>
                    {
                        Gesture::Move {
                            card,
                            start,
                            now: world,
                        }
                    }
                    _ => Gesture::Pan,
                });
            });
        }
        let connecting = matches!(self.studio.read(cx).gesture, Some(Gesture::Connect { .. }));
        if connecting {
            let hovered = self.hit(world, cx);
            self.set_hovered(hovered, cx);
        }
        self.studio.act(cx, |studio| {
            studio.timing.input_received();
            studio.timing.input(crate::timing::InputKind::Pan);
            match &mut studio.gesture {
                Some(Gesture::Pan) | None => {
                    studio
                        .camera
                        .pan_screen(Point::new(f32::from(delta.x), f32::from(delta.y)));
                    studio.mark(Dirty::CAMERA);
                }
                Some(Gesture::Marquee { end, .. }) => {
                    *end = world;
                    studio.mark(Dirty::CAMERA);
                }
                Some(Gesture::Move { now, .. } | Gesture::Connect { now, .. }) => {
                    *now = world;
                    studio.mark(Dirty::CAMERA);
                }
                Some(Gesture::Insert { .. }) => {}
            }
        });
    }

    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(press) = self.press.take() else {
            return;
        };
        let world = self.world(event.position, cx);
        let hovered = self.hit(world, cx);
        if press.dragging {
            let alt = event.modifiers.alt;
            self.studio.act(cx, |studio| match studio.gesture.take() {
                Some(Gesture::Marquee { start, end }) => {
                    let targets = studio.spatial.marquee(Rect::from_points(start, end));
                    studio.selection.replace(targets);
                }
                Some(Gesture::Move { card, start, now }) => {
                    drop_card(studio, card, start, now, alt, hovered.as_ref())
                }
                Some(Gesture::Connect { card, port, .. }) => match &hovered {
                    Some(SceneTarget::Port(other_card, other)) if *other != port => {
                        studio.connect((card, Some(port)), (*other_card, Some(*other)))
                    }
                    Some(SceneTarget::Node(other) | SceneTarget::Container(other))
                        if *other != card =>
                    {
                        studio.connect((card, Some(port)), (*other, None))
                    }
                    _ => studio.status = "Drop on another port to connect".into(),
                },
                _ => studio.mark(Dirty::CAMERA),
            });
            return;
        }
        // A click.
        let shift = event.modifiers.shift;
        let position = [f32::from(event.position.x), f32::from(event.position.y)];
        let rename = self.studio.act(cx, |studio| {
            studio.timing.input(crate::timing::InputKind::Selection);
            let double = studio.canvas_clicks.click(
                studio.scene.generation,
                press.target.as_ref(),
                position,
                f64::from(crate::motion::clock()),
            );
            match &press.target {
                Some(target) => {
                    studio.select(target.clone(), shift);
                    // A click on an attribute or item line works on that line.
                    if let SceneTarget::Node(card) = target
                        && let Some(feature) = feature_at(studio, *card, press.world)
                    {
                        studio.inspected = Some((studio.selection.primary.clone(), feature));
                    }
                    studio.panel = crate::studio::Panel::Inspector;
                    if double {
                        // A usage of a composite opens its definition.
                        let composite = studio
                            .project
                            .as_ref()
                            .zip(target.element_id())
                            .is_some_and(|(p, e)| {
                                crate::library::is_composite_usage(p.state().tree(), e)
                            });
                        let id = if composite {
                            CommandId::OpenDefinition
                        } else if studio.editable() {
                            CommandId::Rename
                        } else {
                            CommandId::Focus
                        };
                        studio.execute(id);
                        return id == CommandId::Rename;
                    }
                    false
                }
                None if !shift => {
                    studio.selection.clear();
                    false
                }
                None => false,
            }
        });
        if rename {
            window.focus(&self.focus, cx);
        }
    }

    /// A block from the Library moves over the Surface: where it would go
    /// and what it would fit are drawn.
    fn drag_block(
        &mut self,
        event: &gpui::DragMoveEvent<crate::panels::library::LibraryDrag>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let position = event.event.position;
        if !self.bounds.get().contains(&position) {
            if matches!(self.studio.read(cx).gesture, Some(Gesture::Insert { .. })) {
                self.studio.act(cx, |studio| {
                    studio.gesture = None;
                    studio.mark(Dirty::CAMERA);
                });
            }
            return;
        }
        let drag = event.drag(cx).clone();
        let world = self.world(position, cx);
        let hovered = self.hit(world, cx);
        self.pointer = Some(position);
        self.studio.act(cx, |studio| {
            let fitting = match &studio.gesture {
                Some(Gesture::Insert { block, fitting, .. }) if *block == drag.block => {
                    fitting.clone()
                }
                _ => fitting_on_surface(studio, &drag.block),
            };
            let port = match &hovered {
                Some(SceneTarget::Port(card, port)) if fitting.contains(&(*card, *port)) => {
                    Some((*card, *port))
                }
                _ => None,
            };
            let target = drop_target(studio, hovered.as_ref(), drag.kind);
            studio.gesture = Some(Gesture::Insert {
                block: drag.block.clone(),
                now: world,
                target,
                port,
                fitting,
            });
            studio.mark(Dirty::CAMERA);
        });
    }

    /// A block from the Library is dropped on the Surface.
    fn drop_block(
        &mut self,
        drag: &crate::panels::library::LibraryDrag,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drag = drag.clone();
        self.studio.act(cx, |studio| {
            let gesture = studio.gesture.take();
            let (target, port) = match gesture {
                Some(Gesture::Insert { target, port, .. }) => (target, port),
                _ => (None, None),
            };
            let tree = studio.project.as_ref().map(|p| p.state().tree());
            let (parent, connect) = match (port, target) {
                (Some((card, port)), _) => (
                    tree.and_then(|t| t.get(card))
                        .and_then(|e| e.owner())
                        .map(Parent::Element),
                    Some(agq_library::ConnectTo {
                        card,
                        port,
                        with: None,
                    }),
                ),
                (None, Some((card, true))) => (Some(Parent::Element(card)), None),
                (None, Some((card, false))) => {
                    let name = tree
                        .and_then(|t| t.effective_name(card))
                        .unwrap_or("that element");
                    studio.status = format!(
                        "{} cannot hold a {}: drop it on a part or on the Surface",
                        name,
                        agq_library::usage_kind(drag.kind).map_or("block", |k| k.keyword())
                    );
                    studio.mark(Dirty::CAMERA | Dirty::STATUS);
                    return;
                }
                (None, None) => (Some(studio.model_package()), None),
            };
            studio.insert_block(drag.block.clone(), parent, connect);
        });
        window.focus(&self.focus, cx);
    }

    fn right_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let world = self.world(event.position, cx);
        let target = self.hit(world, cx);
        self.studio.act(cx, |studio| match &target {
            Some(target) if !studio.selection.targets.contains(target) => {
                studio.select(target.clone(), false)
            }
            Some(_) => {}
            None => studio.selection.clear(),
        });
        cx.emit(SurfaceEvent::ContextMenu(event.position));
    }

    fn scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let origin = self.bounds.get().origin;
        let pointer = Point::new(
            f32::from(event.position.x - origin.x),
            f32::from(event.position.y - origin.y),
        );
        let (x, y) = match event.delta {
            ScrollDelta::Pixels(delta) => (f32::from(delta.x), f32::from(delta.y)),
            ScrollDelta::Lines(delta) => (delta.x * 50.0, delta.y * 50.0),
        };
        let shift = event.modifiers.shift;
        self.studio.act(cx, |studio| {
            studio.timing.input_received();
            studio.camera_target = None;
            studio.camera_move = None;
            if shift || (x != 0.0 && y == 0.0) {
                // Across: Shift+wheel, or a horizontal wheel.
                let across = if x != 0.0 { x } else { y };
                studio.camera.pan_screen(Point::new(across, 0.0));
                studio.timing.input(crate::timing::InputKind::Pan);
            } else if y.is_finite() && y != 0.0 {
                studio
                    .camera
                    .zoom_at(pointer, (y * 0.0025).clamp(-20.0, 20.0).exp());
                studio.timing.input(crate::timing::InputKind::Zoom);
            }
            studio.mark(Dirty::CAMERA);
        });
    }

    fn pinch(&mut self, event: &PinchEvent, _: &mut Window, cx: &mut Context<Self>) {
        let origin = self.bounds.get().origin;
        let pointer = Point::new(
            f32::from(event.position.x - origin.x),
            f32::from(event.position.y - origin.y),
        );
        let factor = 1.0 + event.delta;
        if factor.is_finite() && factor > 0.0 {
            self.studio.act(cx, |studio| {
                studio.camera_target = None;
                studio.camera_move = None;
                studio.camera.zoom_at(pointer, factor);
                studio.timing.input(crate::timing::InputKind::Zoom);
                studio.mark(Dirty::CAMERA);
            });
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, _: &mut Context<Self>) {
        if event.keystroke.key == "space" {
            self.space = true;
        }
    }

    fn key_up(&mut self, event: &KeyUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if event.keystroke.key == "space" {
            self.space = false;
        }
    }

    /// Shows or ends the in-place rename field for the dialog the Studio has open.
    fn sync_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let wanted = match &self.studio.read(cx).dialog {
            Some(crate::edit::Dialog::Rename { element, name }) => Some((*element, name.clone())),
            _ => None,
        };
        match (wanted, &self.rename) {
            (Some((element, _)), Some((current, _, _))) if element == *current => {}
            (Some((element, name)), _) => {
                let state = cx.new(|cx| InputState::new(window, cx).default_value(name));
                let studio = self.studio.clone();
                let subscription = cx.subscribe_in(
                    &state,
                    window,
                    move |this, state, event: &InputEvent, window, cx| match event {
                        InputEvent::PressEnter { .. } => {
                            let name = state.read(cx).value().to_string();
                            studio.act(cx, |studio| {
                                studio.dialog = None;
                                studio.rename(element, &name);
                            });
                            this.rename = None;
                            window.focus(&this.focus, cx);
                        }
                        InputEvent::Blur => {
                            studio.act(cx, |studio| {
                                if matches!(studio.dialog, Some(crate::edit::Dialog::Rename { .. }))
                                {
                                    studio.dialog = None;
                                }
                            });
                            this.rename = None;
                        }
                        _ => {}
                    },
                );
                let focus = state.read(cx).focus_handle(cx);
                state.update(cx, |state, cx| state.select_all(window, cx));
                window.defer(cx, move |window, cx| window.focus(&focus, cx));
                self.rename = Some((element, state, subscription));
            }
            (None, Some(_)) => self.rename = None,
            (None, None) => {}
        }
    }

    /// Where an element's name is on the Surface, relative to the Surface.
    fn name_rect(&self, element: ElementId, cx: &App) -> Bounds<Pixels> {
        let studio = self.studio.read(cx);
        let bounds = studio
            .lookup
            .node(&studio.scene, element)
            .map(|n| n.bounds)
            .or_else(|| {
                studio
                    .scene
                    .ports
                    .iter()
                    .find(|p| p.id == element)
                    .map(|p| Rect::new(p.position.x, p.position.y, 180.0, 20.0))
            });
        let size = self.bounds.get().size;
        match bounds {
            Some(bounds) => {
                let a = studio.camera.world_to_screen(bounds.min);
                let b = studio.camera.world_to_screen(bounds.max);
                Bounds::new(
                    point(px(a.x + 6.0), px(a.y + 6.0)),
                    gpui::size(px((b.x - a.x - 12.0).max(200.0)), px(30.0)),
                )
            }
            None => Bounds::new(
                point(size.width * 0.5 - px(120.0), size.height * 0.5 - px(16.0)),
                gpui::size(px(240.0), px(30.0)),
            ),
        }
    }
}

/// Whether a port is shown: from the summary tier on, or when it or its card
/// is selected, or it just changed.
fn port_visible(studio: &Studio, card: ElementId, port: ElementId) -> bool {
    studio.lod.level() >= agq_studio_scene::LodLevel::Summary
        || studio.selection.contains(port)
        || studio.selection.contains(card)
        || studio.highlights.contains_key(&port)
}

/// The attribute or item line of a card under a Surface point, if any.
pub fn feature_at(studio: &Studio, card: ElementId, point: Point) -> Option<ElementId> {
    let node = studio.lookup.node(&studio.scene, card)?;
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

/// Ends a card drag: Alt over a container moves the element into it;
/// otherwise the card keeps its new place on the Surface (layout only).
pub fn drop_card(
    studio: &mut Studio,
    card: ElementId,
    start: Point,
    now: Point,
    alt: bool,
    hovered: Option<&SceneTarget>,
) {
    if alt {
        let into = match hovered {
            Some(SceneTarget::Node(id) | SceneTarget::Container(id)) if *id != card => Some(*id),
            _ => None,
        };
        match into {
            Some(container) if studio.editable() => {
                studio.move_to(card, Parent::Element(container))
            }
            _ => studio.status = "Hold Alt and drop onto a container to move into it".into(),
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
            studio
                .scene
                .nodes
                .iter()
                .filter(|n| n.semantic.owner == Some(owner))
                .map(|n| n.id()),
        );
        index += 1;
    }
    let scene = studio.scene.clone();
    let memory = studio.layouts.entry(studio.view).or_default();
    for id in moved {
        if let Some(node) = scene.nodes.iter().find(|n| n.id() == id) {
            memory
                .bounds
                .insert(id, node.bounds.translate(Point::new(dx, dy)));
            if memory.is_pinned(id) {
                let _ = memory.pin(id, node.bounds.translate(Point::new(dx, dy)));
            }
        }
    }
    studio.rebuild();
}

/// An element's name in the Surface's list for screen readers: its first
/// line of details, and whether it is selected.
pub fn element_name(studio: &Studio, target: &SceneTarget, selected: bool) -> String {
    let text = hover_text(studio, target).unwrap_or_default();
    let mut label = text.lines().next().unwrap_or_default().to_string();
    if selected {
        label.push_str(", selected");
    }
    label
}

/// What an element is, in a few lines: for its details on hover and its
/// name for screen readers.
pub fn hover_text(studio: &Studio, target: &SceneTarget) -> Option<String> {
    match target {
        SceneTarget::Node(id) | SceneTarget::Container(id) => {
            studio.lookup.node(&studio.scene, *id).map(|n| {
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
        SceneTarget::Port(card, id) => studio.lookup.port(&studio.scene, *card, *id).map(|p| {
            let shared = p
                .defined_in
                .and_then(|d| studio.lookup.node(&studio.scene, d))
                .map(|d| format!("\nDefined in {}: changes apply to it", d.semantic.name))
                .unwrap_or_default();
            format!(
                "port {} · direction {}{shared}\nDrag to another port to connect",
                p.name,
                match p.direction {
                    agq_studio_scene::PortDirection::Unspecified => "not given",
                    agq_studio_scene::PortDirection::In => "in",
                    agq_studio_scene::PortDirection::Out => "out",
                    agq_studio_scene::PortDirection::InOut => "inout",
                }
            )
        }),
        SceneTarget::Edge(id) => studio
            .scene
            .edges
            .iter()
            .find(|e| e.semantic.id == *id)
            .map(|e| format!("{} · {}", e.semantic.kind.label(), e.semantic.label)),
    }
}

/// The commands a context menu offers for what is selected.
pub fn context_commands(target: Option<&SceneTarget>) -> &'static [CommandId] {
    use CommandId::*;
    match target {
        None => &[
            CreatePart,
            InsertFromLibrary,
            CreateRequirement,
            Fit,
            Architecture,
            Graph,
            Requirements,
        ],
        Some(SceneTarget::Port(..)) => &[
            ConnectFromLibrary,
            Rename,
            Connect,
            OpenDefinition,
            Delete,
            Lock,
        ],
        Some(SceneTarget::Edge(_)) => &[Delete],
        Some(SceneTarget::Node(_) | SceneTarget::Container(_)) => &[
            Rename,
            CreatePart,
            InsertFromLibrary,
            CreatePort,
            CreateAttribute,
            Connect,
            OpenDefinition,
            FindUsages,
            Specialize,
            CreateBlock,
            SaveToLibrary,
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

/// The ports in view that one of a block's ports would fit, by the model's
/// own rule: marked while the block is dragged (at most 300 ports).
fn fitting_on_surface(
    studio: &mut Studio,
    block: &agq_library::BlockRef,
) -> std::collections::BTreeSet<(ElementId, ElementId)> {
    let view = studio.camera.visible_rect();
    let shown: Vec<(ElementId, ElementId)> = studio
        .scene
        .ports
        .iter()
        .filter(|p| view.contains(p.position))
        .take(300)
        .map(|p| (p.owner, p.id))
        .collect();
    if shown.is_empty() {
        return Default::default();
    }
    studio.library_index();
    let Some(tree) = studio.project.as_ref().map(|p| p.state().tree()) else {
        return Default::default();
    };
    let Some(index) = studio.library.index.position(block) else {
        return Default::default();
    };
    let ids: Vec<ElementId> = shown.iter().map(|(_, id)| *id).collect();
    let fits = studio
        .library
        .source
        .fitting_ports(&studio.library.index, index, tree, &ids);
    shown
        .into_iter()
        .filter(|(_, id)| fits.contains(id))
        .collect()
}

/// The card a dragged block would go into, and whether that card can hold
/// a usage of it.
fn drop_target(
    studio: &Studio,
    hovered: Option<&SceneTarget>,
    kind: agq_language::ElementKind,
) -> Option<(ElementId, bool)> {
    use agq_language::ElementKind::*;
    let card = match hovered? {
        SceneTarget::Node(id) | SceneTarget::Container(id) => *id,
        SceneTarget::Port(card, _) => *card,
        SceneTarget::Edge(_) => return None,
    };
    let tree = studio.project.as_ref()?.state().tree();
    let owner = tree.get(card)?.kind;
    let fits = match kind {
        PartDef => matches!(owner, Package | PartDef | Part),
        PortDef => matches!(owner, PartDef | Part | PortDef | Port),
        ItemDef => matches!(
            owner,
            Package | PartDef | Part | ItemDef | Item | PortDef | Port
        ),
        AttributeDef => owner.is_definition() || owner.is_usage(),
        RequirementDef => matches!(
            owner,
            Package | PartDef | Part | RequirementDef | Requirement
        ),
        _ => false,
    };
    Some((card, fits))
}

impl Render for SurfaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_rename(window, cx);
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let hovered = self.hovered.clone();
        let hover_details = self
            .hovered_since
            .filter(|since| since.elapsed() >= HOVER_DETAILS && self.press.is_none())
            .and(hovered.as_ref())
            .and_then(|target| hover_text(studio, target));
        let pointer = self.pointer;
        let empty = studio.scene.nodes.is_empty();
        let editable = studio.editable();
        let fixture = studio.fixture.is_some();
        let comparison = studio.comparison.as_ref().map(|c| {
            (
                c.before_label.clone(),
                if c.after_is_now {
                    "now".to_string()
                } else {
                    c.after_label.clone()
                },
            )
        });
        let zoom = studio.camera.zoom;
        let selected = studio.selection.targets.len();
        let total = studio.scene.nodes.len();
        let named: Vec<(SceneTarget, bool, String, Bounds<Pixels>)> = studio
            .selection
            .targets
            .iter()
            .map(|t| (t.clone(), true))
            .chain(
                hovered
                    .clone()
                    .filter(|t| !studio.selection.targets.contains(t))
                    .map(|t| (t, false)),
            )
            .filter_map(|(target, selected)| {
                let bounds = studio.scene.target_bounds(&target)?;
                let a = studio.camera.world_to_screen(bounds.min);
                let b = studio.camera.world_to_screen(bounds.max);
                let label = element_name(studio, &target, selected);
                Some((
                    target,
                    selected,
                    label,
                    Bounds::from_corners(point(px(a.x), px(a.y)), point(px(b.x), px(b.y))),
                ))
            })
            .take(24)
            .collect();
        let rename = self
            .rename
            .as_ref()
            .map(|(element, state, _)| (*element, state.clone()));
        let rename_rect = rename
            .as_ref()
            .map(|(element, _)| self.name_rect(*element, cx));
        let bounds_cell = self.bounds.clone();
        let studio_entity = self.studio.clone();
        let studio_after = self.studio.clone();
        let measuring = self.studio.read(cx).args.measuring();
        let last_frame = self.last_frame.replace(Instant::now());
        let dt = last_frame.map_or(1.0 / 60.0, |last| last.elapsed().as_secs_f32());
        let compatible_hovered = hovered.clone();
        let studio_for_minimap = self.studio.clone();
        let crumbs = if studio.drill.is_empty() {
            Vec::new()
        } else {
            studio.breadcrumbs()
        };

        div()
            .id("surface")
            .key_context("Surface")
            .track_focus(&self.focus)
            .role(gpui::Role::List)
            .aria_label(SharedString::from(format!(
                "Surface: {total} elements, {selected} selected"
            )))
            .size_full()
            .relative()
            .overflow_hidden()
            .cursor(if self.space || matches!(self.studio.read(cx).gesture, Some(Gesture::Pan)) {
                gpui::CursorStyle::ClosedHand
            } else {
                gpui::CursorStyle::Arrow
            })
            .on_drag_move(cx.listener(Self::drag_block))
            .on_drop(cx.listener(Self::drop_block))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::right_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_scroll_wheel(cx.listener(Self::scroll))
            .on_pinch(cx.listener(Self::pinch))
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if !*hovered {
                    this.pointer = None;
                    this.set_hovered(None, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &Deselect, _, cx| {
                this.studio.act(cx, |studio| {
                    studio.selection.clear();
                    studio.mark(Dirty::SELECTION);
                })
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _, cx| {
                this.studio.act(cx, |studio| studio.select_neighbour((-1.0, 0.0)))
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _, cx| {
                this.studio.act(cx, |studio| studio.select_neighbour((1.0, 0.0)))
            }))
            .on_action(cx.listener(|this, _: &SelectUp, _, cx| {
                this.studio.act(cx, |studio| studio.select_neighbour((0.0, -1.0)))
            }))
            .on_action(cx.listener(|this, _: &SelectDown, _, cx| {
                this.studio.act(cx, |studio| studio.select_neighbour((0.0, 1.0)))
            }))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        bounds_cell.set(bounds);
                        ui::target::record("Viewport", bounds);
                        let scale = f32::from(window.rem_size()) / 16.0;
                        studio_entity.update(cx, |studio, cx| {
                            studio.frame_number += 1;
                            studio.timing.frame();
                            let size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                            studio.camera.viewport = Size::new(size.0, size.1);
                            // Fit once the Surface has kept its size for two
                            // frames: a window's first frame can report a size
                            // it never shows.
                            let stable = studio.surface_size.replace(size) == Some(size);
                            if studio.fit_pending && stable {
                                studio.frame_all();
                            }
                            let moving = studio.animate(dt);
                            studio.lod.update(studio.camera.zoom);
                            let compatible = compatible_ports(studio, compatible_hovered.as_ref());
                            let frame = paint::Frame {
                                scene: studio.scene.clone(),
                                spatial: studio.spatial.clone(),
                                lookup: studio.lookup.clone(),
                                camera: studio.camera,
                                lod: studio.lod.level(),
                                selection: studio.selection.clone(),
                                hovered: compatible_hovered.clone(),
                                inspected: studio.inspected_element(),
                                highlights: studio.highlights.clone(),
                                gesture: studio.gesture.clone(),
                                compatible,
                                reduced_motion: studio.reduced_motion,
                                theme: cx.theme().clone(),
                                ui_scale: scale,
                                marks: std::rc::Rc::new(studio.surface_marks()),
                            };
                            (frame, moving)
                        })
                    },
                    move |bounds, (frame, moving), window, cx| {
                        let started = Instant::now();
                        let painted = paint::paint(&frame, bounds, window, cx);
                        let elapsed = started.elapsed();
                        if moving || painted.animating || measuring {
                            window.request_animation_frame();
                        }
                        studio_after.update(cx, |studio, _| {
                            studio.timing.surface_paint(elapsed);
                            studio.timing.visibility(painted.visibility);
                            studio.timing.labels(painted.labels);
                            studio.timing.visible_nodes = painted.visible_nodes;
                            studio.timing.total_nodes = total;
                            studio.timing.ui_complete();
                        });
                    },
                )
                .size_full(),
            )
            .children(named.into_iter().enumerate().map(|(index, (_, selected, label, bounds))| {
                div()
                    .id(("surface-element", index))
                    .role(gpui::Role::ListItem)
                    .aria_label(SharedString::from(label))
                    .aria_selected(selected)
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
            }))
            .when(empty, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if editable {
                            EmptyState::new(
                                IconName::Parts,
                                "An empty model",
                                "Create the first part here, or describe the system to the Assistant and watch it appear.",
                            )
                            .hint("P", "part")
                            .hint("R", "requirement")
                            .hint("Ctrl+L", "ask the Assistant")
                            .into_any_element()
                        } else if fixture {
                            EmptyState::new(IconName::Eye, "Nothing to show in this view", "Switch to another view to see this model.")
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }),
                )
            })
            .when_some(comparison, |this, (before, after)| {
                this.child(comparison_tag(before, after, self.studio.clone(), cx))
            })
            .when(!crumbs.is_empty(), |this| {
                this.child(breadcrumb(crumbs, self.studio.clone(), cx))
            })
            .when(!empty, |this| {
                this.child(zoom_control(zoom, self.studio.clone(), cx))
                    .child(minimap::Minimap::new(studio_for_minimap))
            })
            .when_some(rename.zip(rename_rect), |this, ((_, state), rect)| {
                this.child(
                    div()
                        .absolute()
                        .left(rect.origin.x)
                        .top(rect.origin.y)
                        .w(rect.size.width.min(px(420.0)))
                        .occlude()
                        // Escape keeps the name as it was.
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                this.rename = None;
                                this.studio.act(cx, |studio| {
                                    if matches!(studio.dialog, Some(crate::edit::Dialog::Rename { .. })) {
                                        studio.dialog = None;
                                    }
                                    studio.mark(Dirty::OVERLAY);
                                });
                                window.focus(&this.focus, cx);
                            }
                        }))
                        .rounded(r(crate::tokens::radius::CONTROL))
                        .shadow(theme.shadow_overlay())
                        .child(TextField::new(&state).mono()),
                )
            })
            .when_some(hover_details.zip(pointer), |this, (text, pointer)| {
                let origin = self.bounds.get().origin;
                let mut lines = text.lines();
                let title = lines.next().unwrap_or_default().to_string();
                let rest: Vec<String> = lines.map(str::to_string).collect();
                this.child(
                    div()
                        .absolute()
                        .left(pointer.x - origin.x + px(14.0))
                        .top(pointer.y - origin.y + px(18.0))
                        .max_w(r(320.0))
                        .px(r(10.0))
                        .py(r(7.0))
                        .rounded(r(crate::tokens::radius::CONTROL))
                        .bg(theme.overlay)
                        .border_1()
                        .border_color(theme.border)
                        .shadow(theme.shadow_overlay())
                        .text_size(r(theme::text::SM))
                        .flex()
                        .flex_col()
                        .gap(r(3.0))
                        .child(div().text_color(theme.text).font_weight(theme::MEDIUM).child(title))
                        .children(rest.into_iter().map(|line| div().text_color(theme.text_muted).child(line))),
                )
            })
    }
}

/// The ports a drag from a port can connect to, marked while dragging (D3);
/// while a building block is dragged, the ports it would fit.
fn compatible_ports(
    studio: &Studio,
    _hovered: Option<&SceneTarget>,
) -> std::collections::BTreeSet<(ElementId, ElementId)> {
    if let Some(Gesture::Insert { fitting, .. }) = &studio.gesture {
        return fitting.clone();
    }
    let mut ports = std::collections::BTreeSet::new();
    if let Some(Gesture::Connect { card, port, .. }) = &studio.gesture {
        let view = studio.camera.visible_rect();
        for other in &studio.scene.ports {
            if other.id != *port
                && other.owner != *card
                && view.contains(other.position)
                && ports.len() < 400
            {
                ports.insert((other.owner, other.id));
            }
        }
    }
    ports
}

/// The zoom control in the Surface's bottom-right corner: out, the zoom
/// level (click: 100%), in, and fit.
fn zoom_control(zoom: f32, studio: Entity<Studio>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let run = move |id: CommandId| {
        let studio = studio.clone();
        move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
            studio.act(cx, |studio| {
                studio.execute(id);
                studio.mark(Dirty::CAMERA);
            })
        }
    };
    let shortcut = |id| commands::command(id).shortcut;
    div()
        .absolute()
        .right(r(12.0))
        .bottom(r(12.0))
        .flex()
        .items_center()
        .gap(r(2.0))
        .p(r(3.0))
        .rounded(r(crate::tokens::radius::MENU))
        .bg(theme.overlay.opacity(0.94))
        .border_1()
        .border_color(theme.border)
        .shadow(theme.shadow_small())
        .occlude()
        .child(
            Button::icon_only("zoom-out", IconName::ZoomOut, "Zoom out")
                .small()
                .tooltip("Zoom out", Some(shortcut(CommandId::ZoomOut)))
                .on_click(run(CommandId::ZoomOut)),
        )
        .child(
            Button::new("zoom-reset", format!("{:.0}%", zoom * 100.0))
                .small()
                .ghost()
                .tooltip("Zoom to 100%", Some(shortcut(CommandId::ZoomReset)))
                .on_click(run(CommandId::ZoomReset)),
        )
        .child(
            Button::icon_only("zoom-in", IconName::ZoomIn, "Zoom in")
                .small()
                .tooltip("Zoom in", Some(shortcut(CommandId::ZoomIn)))
                .on_click(run(CommandId::ZoomIn)),
        )
        .child(
            div()
                .w(theme::hairline())
                .h(r(16.0))
                .mx(r(2.0))
                .bg(theme.separator),
        )
        .child(
            Button::icon_only("zoom-fit", IconName::Fit, "Fit to view")
                .small()
                .tooltip("Fit to view", Some(shortcut(CommandId::Fit)))
                .on_click(run(CommandId::Fit)),
        )
}

/// Where the Surface is: the whole model, then each definition opened;
/// a crumb goes back to it, and Back one step (Backspace, Alt+Left).
pub(crate) fn breadcrumb(
    crumbs: Vec<String>,
    studio: Entity<Studio>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let last = crumbs.len().saturating_sub(1);
    let back = studio.clone();
    div()
        .id("breadcrumb")
        .absolute()
        .top(r(12.0))
        .left(r(12.0))
        .max_w(gpui::relative(0.7))
        .occlude()
        .role(gpui::Role::Navigation)
        .aria_label("Where you are: the whole model, then each definition opened")
        .flex()
        .items_center()
        .gap(r(2.0))
        .p(r(3.0))
        .rounded(r(crate::tokens::radius::MENU))
        .bg(theme.overlay.opacity(0.94))
        .border_1()
        .border_color(theme.border)
        .shadow(theme.shadow_small())
        .child(
            Button::icon_only("breadcrumb-back", IconName::ChevronLeft, "Back")
                .small()
                .tooltip("Back", Some("Backspace"))
                .on_click(move |_, _, cx| back.act(cx, |studio| studio.back())),
        )
        .children(crumbs.into_iter().enumerate().flat_map(|(depth, crumb)| {
            let studio = studio.clone();
            let separator = (depth > 0).then(|| {
                div()
                    .text_size(r(theme::text::SM))
                    .text_color(theme.text_faint)
                    .child("›")
                    .into_any_element()
            });
            let item = if depth == last {
                div()
                    .px(r(6.0))
                    .text_size(r(theme::text::SM))
                    .font_weight(theme::SEMIBOLD)
                    .font_family(theme::MONO)
                    .text_color(theme.text)
                    .whitespace_nowrap()
                    .child(crumb)
                    .into_any_element()
            } else {
                Button::new(("crumb", depth), crumb)
                    .small()
                    .ghost()
                    .on_click(move |_, _, cx| studio.act(cx, |studio| studio.back_to(depth)))
                    .into_any_element()
            };
            separator.into_iter().chain(std::iter::once(item))
        }))
}

/// A tag above the Surface's content while it shows what changed.
fn comparison_tag(
    before: String,
    after: String,
    studio: Entity<Studio>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .absolute()
        .top(r(12.0))
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .child(
            div()
                .occlude()
                .flex()
                .items_center()
                .gap(r(10.0))
                .pl(r(12.0))
                .pr(r(4.0))
                .h(r(34.0))
                .rounded(r(crate::tokens::radius::MENU))
                .bg(theme.overlay)
                .border_1()
                .border_color(theme.border)
                .shadow(theme.shadow_overlay())
                .text_size(r(theme::text::SM))
                .child(
                    ui::icon(IconName::Compare)
                        .size(14.0)
                        .color(theme.text_muted),
                )
                .child(
                    div()
                        .text_color(theme.text)
                        .font_weight(theme::MEDIUM)
                        .child(format!("What changed · {before} → {after}")),
                )
                .child(ui::Chip::new("new").tone(ui::Tone::Success))
                .child(ui::Chip::new("changed").tone(ui::Tone::Info))
                .child(ui::Chip::new("deleted").tone(ui::Tone::Danger))
                .child(
                    Button::new("close-comparison", "Close")
                        .small()
                        .ghost()
                        .icon(IconName::Close)
                        .on_click(move |_, _, cx| {
                            studio.act(cx, |studio| studio.close_comparison())
                        }),
                ),
        )
}

#[cfg(test)]
mod tests;
