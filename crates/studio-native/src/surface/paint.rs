//! Drawing one frame of the Surface with GPUI's own primitives (C-48): quads
//! for cards, containers, ports and the orthogonal segments of edges, paths
//! for arrowheads and direction marks, shaped text for labels. Each z-group
//! is a paint layer, so tens of thousands of primitives are cheap to submit;
//! only what is in view (with a margin) is drawn, and levels of detail
//! (named tiers, §3.2) decide what each card shows at a zoom.
use super::Gesture;
use crate::{
    motion,
    selection::Selection,
    ui::{Theme, theme},
};
use agq_studio_scene::{
    Camera2D, DiffMark, EdgeKind, ElementId, LockMark, LodLevel, NodeCategory, Point,
    PortDirection, PortSide, Rect, Scene, SceneLookup, SceneNode, SceneTarget, SpatialIndex,
    VisibleScene,
};
use agq_system_state::Actor;
use gpui::{
    App, BorderStyle, Bounds, Corners, FontWeight, Hsla, PathBuilder, Pixels, SharedString,
    TextAlign, TextRun, TruncateFrom, Window, fill, font, point, px, quad, size,
};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Everything one frame draws, copied or shared from the Studio when the
/// view is built.
pub struct Frame {
    pub scene: Rc<Scene>,
    pub spatial: Rc<SpatialIndex>,
    pub lookup: Rc<SceneLookup>,
    pub camera: Camera2D,
    pub lod: LodLevel,
    pub selection: Selection,
    pub hovered: Option<SceneTarget>,
    /// Card, feature line: the element the Inspector shows.
    pub inspected: Option<ElementId>,
    pub highlights: BTreeMap<ElementId, (f32, Actor)>,
    pub gesture: Option<Gesture>,
    /// Ports a drag from a port may connect to.
    pub compatible: BTreeSet<(ElementId, ElementId)>,
    pub reduced_motion: bool,
    pub theme: Theme,
    /// Points per UI point: labels keep a legible minimum at any UI scale.
    pub ui_scale: f32,
}

/// What the paint measured, for the metrics report.
pub struct Painted {
    pub visible_nodes: usize,
    /// Finding what is in view.
    pub visibility: std::time::Duration,
    /// Laying out and drawing the text (labels; their accessible names are
    /// the Surface's overlay nodes).
    pub labels: std::time::Duration,
    /// Highlights still fading: the Surface asks for another frame.
    pub animating: bool,
}

struct Screen {
    origin: gpui::Point<Pixels>,
    camera: Camera2D,
}

impl Screen {
    fn point(&self, p: Point) -> gpui::Point<Pixels> {
        let s = self.camera.world_to_screen(p);
        point(self.origin.x + px(s.x), self.origin.y + px(s.y))
    }
    fn rect(&self, r: Rect) -> Bounds<Pixels> {
        Bounds::from_corners(self.point(r.min), self.point(r.max))
    }
}

pub(super) fn category_colour(category: NodeCategory, theme: &Theme) -> Hsla {
    match category {
        NodeCategory::Requirement => theme.warning.text,
        NodeCategory::Definition => theme.info.text,
        NodeCategory::Package => theme.text_muted,
        _ => theme.accent.text,
    }
}

/// A whole-point size for Surface text: GPUI rasterizes glyphs per size, so
/// sizes that change every zoom step would be shaped and rasterized again.
fn text_size(points: f32) -> Pixels {
    px(points.round().max(1.0))
}

/// One line of text within `width`, elided with "…" when it does not fit.
#[allow(clippy::too_many_arguments)]
fn label(
    window: &mut Window,
    cx: &mut App,
    text: &str,
    origin: gpui::Point<Pixels>,
    size: Pixels,
    weight: FontWeight,
    family: &'static str,
    colour: Hsla,
    width: Pixels,
) -> Pixels {
    if text.is_empty() || width <= px(4.0) {
        return px(0.0);
    }
    let mut run_font = font(family);
    run_font.weight = weight;
    let runs = [TextRun {
        len: text.len(),
        font: run_font.clone(),
        color: colour,
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let text_system = window.text_system().clone();
    let shaped = text_system.shape_line(SharedString::from(text.to_string()), size, &runs, None);
    let shaped = if shaped.width > width {
        let mut wrapper = text_system.line_wrapper(run_font, size);
        let (text, runs) = wrapper.truncate_line(
            SharedString::from(text.to_string()),
            width,
            "…",
            &runs,
            TruncateFrom::End,
        );
        text_system.shape_line(text, size, &runs, None)
    } else {
        shaped
    };
    let drawn = shaped.width;
    let _ = shaped.paint(origin, size * 1.25, TextAlign::Left, None, window, cx);
    drawn
}

/// The dot grid follows the camera; a coarser grid takes over as dots crowd
/// together, and it fades out as the model gets small (§3.2 Surface).
fn grid(frame: &Frame, bounds: Bounds<Pixels>, window: &mut Window) {
    let zoom = frame.camera.zoom;
    let mut spacing = 24.0 * zoom;
    while spacing < 14.0 {
        spacing *= 4.0;
    }
    let fade = ((spacing - 14.0) / 18.0).clamp(0.0, 1.0) * (zoom * 4.0).clamp(0.0, 1.0);
    if fade <= 0.02 {
        return;
    }
    let dot = frame
        .theme
        .dots
        .opacity(frame.theme.dots.a * (0.35 + 0.65 * fade));
    let origin = frame.camera.world_to_screen(Point::default());
    let size_px = px(1.5);
    window.paint_layer(bounds, |window| {
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        let mut y = origin.y.rem_euclid(spacing);
        while y < height {
            let mut x = origin.x.rem_euclid(spacing);
            while x < width {
                window.paint_quad(fill(
                    Bounds::new(
                        point(
                            bounds.origin.x + px(x) - size_px * 0.5,
                            bounds.origin.y + px(y) - size_px * 0.5,
                        ),
                        size(size_px, size_px),
                    ),
                    dot,
                ));
                x += spacing;
            }
            y += spacing;
        }
    });
}

/// Draws the frame; returns what it measured.
pub fn paint(frame: &Frame, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Painted {
    let theme = &frame.theme;
    let screen = Screen {
        origin: bounds.origin,
        camera: frame.camera,
    };
    let zoom = frame.camera.zoom;
    window.paint_quad(fill(bounds, theme.canvas));
    grid(frame, bounds, window);
    let view = frame.camera.visible_rect().inflate(24.0 / zoom);
    let looked_up = std::time::Instant::now();
    let objects = frame.spatial.visible_scene(&frame.scene, view);
    let visibility = looked_up.elapsed();
    let mut labels = std::time::Duration::ZERO;
    let now = motion::clock();
    let mut animating = false;
    let highlight = |id: ElementId| -> Option<(f32, Hsla)> {
        let (started, actor) = frame.highlights.get(&id)?;
        let strength = if frame.reduced_motion {
            1.0
        } else {
            motion::highlight(now - started)
        };
        (strength > 0.0).then(|| (strength, theme.actor(*actor)))
    };
    let selected_card = |id: ElementId| frame.selection.contains(id);
    let hovered_card = |id: ElementId| matches!(&frame.hovered, Some(SceneTarget::Node(h) | SceneTarget::Container(h)) if *h == id);
    let radius = |node: &SceneNode| -> Pixels {
        let base = match node.category {
            NodeCategory::Requirement => 4.0,
            _ if node.is_container => 10.0,
            _ => crate::tokens::radius::CARD,
        };
        px((base * zoom).clamp(2.0, base))
    };

    // Containers, deepest last so nested ones sit on their owners.
    window.paint_layer(bounds, |window| {
        let mut containers: Vec<_> = objects.nodes.iter().filter(|n| n.is_container).collect();
        containers.sort_by_key(|n| n.depth);
        for node in containers {
            let rect = screen.rect(node.bounds);
            let depth = node.depth.min(4) as f32;
            let fill_colour = if theme.dark {
                theme.chrome.opacity(0.55 + 0.1 * depth)
            } else {
                theme.chrome.opacity(0.7 + 0.06 * depth)
            };
            let (border, style) = match node.diff {
                DiffMark::Removed => (theme.danger.solid, BorderStyle::Dashed),
                DiffMark::Added => (theme.success.solid, BorderStyle::Solid),
                DiffMark::Changed => (theme.info.solid, BorderStyle::Solid),
                DiffMark::Unchanged if selected_card(node.id()) => {
                    (theme.accent.solid, BorderStyle::Solid)
                }
                DiffMark::Unchanged if hovered_card(node.id()) => {
                    (theme.border_strong, BorderStyle::Solid)
                }
                DiffMark::Unchanged => (theme.border, BorderStyle::Solid),
            };
            window.paint_quad(quad(
                rect,
                radius(node),
                fill_colour,
                px(1.0),
                border,
                style,
            ));
            // The title strip.
            let header = (58.0 * zoom).min(f32::from(rect.size.height));
            if header >= 6.0 {
                window.paint_quad(fill(
                    Bounds::new(
                        point(rect.origin.x + px(1.0), rect.origin.y + px(header)),
                        size((rect.size.width - px(2.0)).max(px(0.0)), px(1.0)),
                    ),
                    theme.separator,
                ));
            }
        }
    });

    // Edges: orthogonal segments as thin quads, dashed where the kind says
    // so; arrowheads by kind when they are large enough to read.
    let selected_edges: BTreeSet<&str> = frame
        .selection
        .targets
        .iter()
        .filter_map(|t| match t {
            SceneTarget::Edge(id) => Some(id.as_str()),
            _ => None,
        })
        .collect();
    let crowded = objects.edges.len() > 6000 && frame.selection.targets.is_empty();
    window.paint_layer(bounds, |window| {
        for edge in &objects.edges {
            let semantic = &edge.semantic;
            let selected = selected_edges.contains(semantic.id.as_str())
                || matches!(&frame.hovered, Some(SceneTarget::Edge(h)) if *h == semantic.id);
            let incident = frame.selection.contains(semantic.source.node)
                || frame.selection.contains(semantic.target.node)
                || semantic
                    .source
                    .port
                    .is_some_and(|p| frame.selection.contains(p))
                || semantic
                    .target
                    .port
                    .is_some_and(|p| frame.selection.contains(p));
            let glow = semantic.element.and_then(highlight);
            let mut colour = match semantic.kind {
                EdgeKind::Satisfy => theme.warning.solid.opacity(0.75),
                EdgeKind::Typing | EdgeKind::Specialization => theme.text_faint.opacity(0.7),
                _ => theme.edge.opacity(if crowded { 0.45 } else { 0.85 }),
            };
            if selected || incident {
                colour = theme.accent.solid;
            }
            if semantic.problems > 0 {
                colour = theme.warning.solid;
            }
            match edge.diff {
                DiffMark::Added => colour = theme.success.solid,
                DiffMark::Changed => colour = theme.info.solid,
                DiffMark::Removed => colour = theme.danger.solid,
                DiffMark::Unchanged => {}
            }
            if let Some((strength, actor)) = glow {
                colour = actor.opacity(0.35 + 0.65 * strength);
                animating = true;
            }
            let width = if selected {
                2.25
            } else if incident || glow.is_some() {
                1.75
            } else {
                1.25
            };
            let dashed = matches!(
                semantic.kind,
                EdgeKind::Typing | EdgeKind::Specialization | EdgeKind::Satisfy
            ) || edge.diff == DiffMark::Removed;
            // Each point is placed once, with no list per edge: at 10k
            // cards there are 20k edges a frame.
            let mut last: Option<gpui::Point<Pixels>> = None;
            let mut before_last = None;
            // In a crowded view, a jog under a pixel is not drawn.
            let least = if crowded && !(selected || incident) {
                1.0
            } else {
                0.35
            };
            for p in &edge.points {
                let point = screen.point(*p);
                if let Some(previous) = last {
                    let (dx, dy) = (
                        f32::from(point.x - previous.x).abs(),
                        f32::from(point.y - previous.y).abs(),
                    );
                    if dx >= least || dy >= least {
                        segment(
                            window,
                            previous,
                            point,
                            px(width),
                            colour,
                            dashed && !crowded,
                        );
                    }
                }
                before_last = last;
                last = Some(point);
            }
            if let (Some(a), Some(b)) = (before_last, last)
                && semantic.directed
                && zoom >= 0.35
                && !crowded
            {
                arrowhead(window, a, b, colour, zoom);
            }
        }
    });

    // Cards.
    let lod = frame.lod;
    window.paint_layer(bounds, |window| {
        for node in objects.nodes.iter().filter(|n| !n.is_container) {
            let rect = screen.rect(node.bounds);
            let id = node.id();
            let hovered = hovered_card(id);
            let radius = radius(node);
            if let Some((strength, colour)) = highlight(id) {
                animating = true;
                window.paint_drop_shadows(
                    rect,
                    Corners::all(radius),
                    &[gpui::BoxShadow {
                        color: colour.opacity(0.45 * strength),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(14.0),
                        spread_radius: px(2.0),
                        inset: false,
                    }],
                );
            }
            let (fill_colour, border, style) = match node.diff {
                DiffMark::Removed => (theme.canvas, theme.danger.solid, BorderStyle::Dashed),
                DiffMark::Added => (theme.raised, theme.success.solid, BorderStyle::Solid),
                DiffMark::Changed => (theme.raised, theme.info.solid, BorderStyle::Solid),
                DiffMark::Unchanged => (
                    theme.raised,
                    if hovered || node.semantic.lock == LockMark::Own {
                        theme.border_strong
                    } else {
                        theme.border
                    },
                    BorderStyle::Solid,
                ),
            };
            let tiny = rect.size.width < px(6.0);
            window.paint_quad(quad(
                rect,
                radius,
                fill_colour,
                if tiny { px(0.0) } else { px(1.0) },
                border,
                style,
            ));
            if tiny {
                continue;
            }
            // A slim category mark along the top edge.
            let mark = px((3.0 * zoom).clamp(1.5, 3.0));
            window.paint_quad(quad(
                Bounds::new(
                    point(rect.origin.x + radius, rect.origin.y),
                    size((rect.size.width - radius * 2.0).max(px(0.0)), mark),
                ),
                Corners {
                    top_left: px(0.0),
                    top_right: px(0.0),
                    bottom_left: mark,
                    bottom_right: mark,
                },
                category_colour(node.category, theme).opacity(0.65),
                px(0.0),
                gpui::transparent_black(),
                BorderStyle::Solid,
            ));
            // Far out, the name is a bar the size of the name ("greeked"):
            // the model's shape reads without text too small to read.
            if lod == LodLevel::Overview && frame.selection.targets.is_empty() {
                let pad = (12.0 * zoom).max(2.0);
                let h = (14.5 * zoom * 0.5).max(1.5);
                let w = (node.semantic.name.chars().count() as f32 * 7.6 * zoom)
                    .min(f32::from(rect.size.width) - 2.0 * pad);
                if w > 2.0 && f32::from(rect.size.height) > 12.0 * zoom + h {
                    greek(
                        window,
                        point(rect.origin.x + px(pad), rect.origin.y + px(12.0 * zoom)),
                        w,
                        h,
                        theme.text_faint.opacity(0.55),
                    );
                }
            }
            // A problem bar along the card's left edge.
            if node.semantic.problems > 0 {
                window.paint_quad(quad(
                    Bounds::new(
                        point(rect.origin.x, rect.origin.y + radius),
                        size(px(3.0), (rect.size.height - radius * 2.0).max(px(0.0))),
                    ),
                    Corners::all(px(1.5)),
                    theme.warning.solid,
                    px(0.0),
                    gpui::transparent_black(),
                    BorderStyle::Solid,
                ));
            }
            // A hairline under the title, above the attribute and item lines.
            let features = node.semantic.features.len();
            if lod >= LodLevel::Features && features > 0 {
                let top = node.bounds.max.y - agq_studio_scene::feature_block(features);
                let y = screen.point(Point::new(node.bounds.min.x, top)).y;
                window.paint_quad(fill(
                    Bounds::new(
                        point(rect.origin.x + px(1.0), y - px(4.0 * zoom)),
                        size(rect.size.width - px(2.0), px(1.0)),
                    ),
                    theme.separator,
                ));
            }
        }
    });

    // Text: cards, containers, ports, edge labels.
    let text_layer = lod >= LodLevel::Summary || !frame.selection.targets.is_empty();
    if text_layer {
        let started = std::time::Instant::now();
        window.paint_layer(bounds, |window| {
            for node in &objects.nodes {
                card_text(frame, &screen, node, window, cx);
            }
        });
        edge_labels(frame, &screen, &objects, bounds, window, cx);
        labels += started.elapsed();
    }

    // Ports, above the cards they sit on.
    window.paint_layer(bounds, |window| {
        let connected: BTreeSet<ElementId> = objects
            .edges
            .iter()
            .flat_map(|e| [e.semantic.source.port, e.semantic.target.port])
            .flatten()
            .collect();
        for port in &objects.ports {
            if !port_visible(frame, port.owner, port.id) {
                continue;
            }
            let selected = frame
                .selection
                .targets
                .contains(&SceneTarget::Port(port.owner, port.id));
            let hovered = frame.hovered == Some(SceneTarget::Port(port.owner, port.id));
            let compatible = frame.compatible.contains(&(port.owner, port.id));
            let glow = highlight(port.id);
            let diameter =
                (9.0 * zoom).clamp(6.0, 11.0) + if selected || hovered { 2.0 } else { 0.0 };
            let centre = screen.point(port.position);
            let bounds = Bounds::new(
                point(centre.x - px(diameter * 0.5), centre.y - px(diameter * 0.5)),
                size(px(diameter), px(diameter)),
            );
            let ring = if selected || compatible {
                theme.accent.solid
            } else if let Some((_, colour)) = glow {
                colour
            } else {
                match port.diff {
                    DiffMark::Added => theme.success.solid,
                    DiffMark::Changed => theme.info.solid,
                    DiffMark::Removed => theme.danger.solid,
                    DiffMark::Unchanged if hovered => theme.text_secondary,
                    DiffMark::Unchanged => theme.edge,
                }
            };
            // Hollow when unconnected, filled when connected (§3.2).
            let filled = connected.contains(&port.id);
            if compatible {
                window.paint_drop_shadows(
                    bounds,
                    Corners::all(px(diameter)),
                    &[gpui::BoxShadow {
                        color: theme.accent.solid.opacity(0.35),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(6.0),
                        spread_radius: px(2.0),
                        inset: false,
                    }],
                );
            }
            window.paint_quad(quad(
                bounds,
                px(diameter),
                if filled { ring } else { theme.canvas },
                px(1.5),
                ring,
                BorderStyle::Solid,
            ));
            if zoom >= 0.8 && diameter >= 8.0 {
                direction_mark(
                    window,
                    centre,
                    port.side,
                    port.direction,
                    diameter,
                    ring,
                    filled,
                    theme,
                );
            }
        }
    });
    if text_layer {
        let started = std::time::Instant::now();
        window.paint_layer(bounds, |window| {
            port_labels(frame, &screen, &objects, window, cx)
        });
        labels += started.elapsed();
    }

    // Selection rings, outside the card (§3.2), and the gesture.
    window.paint_layer(bounds, |window| {
        for target in &frame.selection.targets {
            let Some(bounds) = frame.scene.target_bounds(target) else {
                continue;
            };
            if let SceneTarget::Node(id) | SceneTarget::Container(id) = target
                && let Some(node) = frame.lookup.node(&frame.scene, *id)
            {
                let rect = screen.rect(bounds);
                let gap = px(3.0);
                let r = radius(node) + gap;
                window.paint_quad(quad(
                    Bounds::new(
                        point(rect.origin.x - gap, rect.origin.y - gap),
                        size(rect.size.width + gap * 2.0, rect.size.height + gap * 2.0),
                    ),
                    r,
                    gpui::transparent_black(),
                    px(crate::tokens::stroke::FOCUS),
                    theme.accent.solid,
                    BorderStyle::Solid,
                ));
            }
        }
        gesture(frame, &screen, window);
    });
    Painted {
        visible_nodes: objects.nodes.len(),
        visibility,
        labels,
        animating,
    }
}

/// Whether a port is drawn: from the summary tier on, and always when it or
/// its card is selected, or it just changed.
pub fn port_visible(frame: &Frame, card: ElementId, port: ElementId) -> bool {
    frame.lod >= LodLevel::Summary
        || frame.selection.contains(port)
        || frame.selection.contains(card)
        || frame.highlights.contains_key(&port)
}

/// One segment of a route: axis-aligned segments are quads; any other is a
/// stroked path.
fn segment(
    window: &mut Window,
    a: gpui::Point<Pixels>,
    b: gpui::Point<Pixels>,
    width: Pixels,
    colour: Hsla,
    dashed: bool,
) {
    // Shorter than a third of a pixel: nothing to see.
    if (a.x - b.x).abs() < px(0.35) && (a.y - b.y).abs() < px(0.35) {
        return;
    }
    let axis = (a.x - b.x).abs() < px(0.5) || (a.y - b.y).abs() < px(0.5);
    if !axis {
        let mut path = PathBuilder::stroke(width);
        path.move_to(a);
        path.line_to(b);
        if let Ok(path) = path.build() {
            window.paint_path(path, colour);
        }
        return;
    }
    let half = width * 0.5;
    let rect = |from: gpui::Point<Pixels>, to: gpui::Point<Pixels>| {
        Bounds::from_corners(
            point(from.x.min(to.x) - half, from.y.min(to.y) - half),
            point(from.x.max(to.x) + half, from.y.max(to.y) + half),
        )
    };
    if !dashed {
        window.paint_quad(fill(rect(a, b), colour));
        return;
    }
    let length = f32::from((b.x - a.x).abs() + (b.y - a.y).abs());
    let (dash, gap) = (6.0, 4.0);
    let mut at = 0.0;
    while at < length {
        let end = (at + dash).min(length);
        let from = point(
            a.x + (b.x - a.x) * (at / length),
            a.y + (b.y - a.y) * (at / length),
        );
        let to = point(
            a.x + (b.x - a.x) * (end / length),
            a.y + (b.y - a.y) * (end / length),
        );
        window.paint_quad(fill(rect(from, to), colour));
        at += dash + gap;
    }
}

fn arrowhead(
    window: &mut Window,
    before: gpui::Point<Pixels>,
    end: gpui::Point<Pixels>,
    colour: Hsla,
    zoom: f32,
) {
    let (dx, dy) = (f32::from(end.x - before.x), f32::from(end.y - before.y));
    let length = dx.hypot(dy).max(0.001);
    let (ux, uy) = (dx / length, dy / length);
    let long = (9.0 * zoom).clamp(6.0, 10.0);
    let wide = long * 0.5;
    let base = point(end.x - px(ux * long), end.y - px(uy * long));
    let mut path = PathBuilder::fill();
    path.move_to(end);
    path.line_to(point(base.x - px(uy * wide), base.y + px(ux * wide)));
    path.line_to(point(base.x + px(uy * wide), base.y - px(ux * wide)));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, colour);
    }
}

/// A port's direction as a small chevron inside it: pointing into the card
/// for `in`, out of it for `out`, both ways for `inout` (a shape, not a
/// colour, §3.5).
#[allow(clippy::too_many_arguments)]
fn direction_mark(
    window: &mut Window,
    centre: gpui::Point<Pixels>,
    side: PortSide,
    direction: PortDirection,
    diameter: f32,
    ring: Hsla,
    filled: bool,
    theme: &Theme,
) {
    // The outward normal of the card's side.
    let (nx, ny) = match side {
        PortSide::Left => (-1.0, 0.0),
        PortSide::Right => (1.0, 0.0),
        PortSide::Top => (0.0, -1.0),
        PortSide::Bottom => (0.0, 1.0),
    };
    let pointing: &[f32] = match direction {
        PortDirection::In => &[-1.0],
        PortDirection::Out => &[1.0],
        PortDirection::InOut => &[-1.0, 1.0],
        PortDirection::Unspecified => &[],
    };
    let colour = if filled { theme.canvas } else { ring };
    let s = diameter * 0.22;
    for sign in pointing {
        let (dx, dy) = (nx * sign, ny * sign);
        let tip = point(centre.x + px(dx * s), centre.y + px(dy * s));
        let back = point(centre.x - px(dx * s * 0.6), centre.y - px(dy * s * 0.6));
        let mut path = PathBuilder::fill();
        path.move_to(tip);
        path.line_to(point(back.x - px(dy * s), back.y + px(dx * s)));
        path.line_to(point(back.x + px(dy * s), back.y - px(dx * s)));
        path.close();
        if let Ok(path) = path.build() {
            window.paint_path(path, colour);
        }
    }
}

/// How far down a card its kind, name and type reach on screen, as
/// `card_text` lays them out (text keeps a legible size as cards shrink).
fn text_block(frame: &Frame, node: &SceneNode) -> f32 {
    let zoom = frame.camera.zoom;
    let floor = theme::text::XS * frame.ui_scale.max(1.0) * 0.9;
    let caption = (10.0 * zoom).clamp(floor, 12.0);
    let name_size = ((if node.is_container { 15.0 } else { 14.5 }) * zoom).clamp(floor + 1.0, 24.0);
    let small = (11.0 * zoom).clamp(floor, 15.0);
    let detail = if node.semantic.detail.is_empty() {
        0.0
    } else {
        small * 1.25
    };
    (9.0 * zoom).clamp(3.0, 12.0) + caption + 5.0 * zoom.clamp(0.5, 1.4) + name_size * 1.3 + detail
}

/// Card text laid out top to bottom so nothing overlaps at any zoom: the
/// kind and marks, the name, the type, then the attribute and item lines.
/// A row that would need text below the smallest size is left out.
fn card_text(frame: &Frame, screen: &Screen, node: &SceneNode, window: &mut Window, cx: &mut App) {
    let theme = &frame.theme;
    let zoom = frame.camera.zoom;
    let rect = screen.rect(node.bounds);
    let (width, height) = (f32::from(rect.size.width), f32::from(rect.size.height));
    let floor = theme::text::XS * frame.ui_scale.max(1.0) * 0.9;
    if width < 40.0 || height < 14.0 {
        return;
    }
    let removed = node.diff == DiffMark::Removed;
    let pad = (12.0 * zoom).clamp(5.0, 16.0);
    let inner = px(width - 2.0 * pad);
    let x = rect.origin.x + px(pad);
    let mut y = rect.origin.y + px((9.0 * zoom).clamp(3.0, 12.0));
    let caption = (10.0 * zoom).clamp(floor, 12.0);
    let name_size = ((if node.is_container { 15.0 } else { 14.5 }) * zoom).clamp(floor + 1.0, 24.0);
    let text = if removed {
        theme.text_muted
    } else {
        theme.text
    };
    let lod = frame.lod;
    let marks_row = height >= caption + name_size + 2.0 * pad && lod >= LodLevel::Summary;
    if marks_row {
        // Right-aligned marks: change, problems, lock.
        let mut right = rect.origin.x + rect.size.width - px(pad);
        let middle = y + px(caption * 0.5);
        if node.diff != DiffMark::Unchanged {
            let (badge, colour) = match node.diff {
                DiffMark::Added => ("NEW", theme.success.text),
                DiffMark::Removed => ("DELETED", theme.danger.text),
                _ => ("CHANGED", theme.info.text),
            };
            let w = measure(window, badge, text_size(caption), theme::SEMIBOLD);
            label(
                window,
                cx,
                badge,
                point(right - w, y),
                text_size(caption),
                theme::SEMIBOLD,
                theme::SANS,
                colour,
                w + px(1.0),
            );
            right -= w + px(caption * 0.6);
        }
        if node.semantic.problems > 0 {
            let count = node.semantic.problems.to_string();
            let d = px(caption * 1.35);
            let pill = measure(window, &count, text_size(caption * 0.9), theme::SEMIBOLD) + d * 0.7;
            let pill_bounds = Bounds::new(point(right - pill, middle - d * 0.5), size(pill, d));
            window.paint_quad(quad(
                pill_bounds,
                d * 0.5,
                theme.warning.soft,
                px(1.0),
                theme.warning.border,
                BorderStyle::Solid,
            ));
            label(
                window,
                cx,
                &count,
                point(pill_bounds.origin.x + d * 0.35, middle - px(caption * 0.55)),
                text_size(caption * 0.9),
                theme::SEMIBOLD,
                theme::SANS,
                theme.warning.text,
                pill,
            );
            right -= pill + px(caption * 0.5);
        }
        if node.semantic.lock.locked() {
            let s = px(caption * 1.25);
            lock_icon(
                window,
                cx,
                point(right - s, middle - s * 0.5),
                s,
                node.semantic.lock,
                theme,
            );
            right -= s + px(caption * 0.5);
        }
        let kind = node.semantic.keyword.to_uppercase();
        label(
            window,
            cx,
            &kind,
            point(x, y),
            text_size(caption),
            theme::SEMIBOLD,
            theme::SANS,
            category_colour(node.category, theme),
            (right - x - px(4.0)).max(px(0.0)),
        );
        y += px(caption + 5.0 * zoom.clamp(0.5, 1.4));
    }
    let name = if node.is_container {
        format!(
            "{}  {}",
            if node.collapsed { "▸" } else { "▾" },
            node.semantic.name
        )
    } else {
        node.semantic.name.clone()
    };
    let bottom = rect.origin.y + rect.size.height;
    if bottom - y < px(name_size) {
        return;
    }
    label(
        window,
        cx,
        &name,
        point(x, y),
        text_size(name_size),
        theme::MEDIUM,
        theme::SANS,
        text,
        inner,
    );
    y += px(name_size * 1.3);
    let features = node.semantic.features.len();
    let features_top = rect.origin.y
        + px((node.bounds.height() - agq_studio_scene::feature_block(features)) * zoom);
    let small = (11.0 * zoom).clamp(floor, 15.0);
    if lod >= LodLevel::Summary
        && !node.semantic.detail.is_empty()
        && y + px(small) <= features_top.min(bottom - px(2.0))
    {
        label(
            window,
            cx,
            &node.semantic.detail,
            point(x, y),
            text_size(small),
            theme::REGULAR,
            theme::MONO,
            theme.text_muted,
            inner,
        );
    }
    let line = agq_studio_scene::FEATURE_LINE * zoom;
    if !node.is_container && (lod < LodLevel::Features || line < floor + 2.0) {
        // Lines too small to read are bars their length.
        let h = (line * 0.34).max(1.5);
        for (index, feature) in node
            .semantic
            .features
            .iter()
            .take(agq_studio_scene::MAX_FEATURE_LINES)
            .enumerate()
        {
            let top = features_top + px(index as f32 * line + (line - h) * 0.4);
            if top < y || top + px(h) > bottom {
                continue;
            }
            let w = (feature.text.chars().count() as f32 * 6.0 * zoom).min(f32::from(inner));
            greek(window, point(x, top), w, h, theme.text_faint.opacity(0.4));
        }
    } else if lod >= LodLevel::Features && !node.is_container {
        let size_pt = (10.5 * zoom).min(line * 0.78).clamp(floor, 14.0);
        let shown = features.min(agq_studio_scene::MAX_FEATURE_LINES);
        let more = features - shown;
        for (index, feature) in node.semantic.features.iter().take(shown).enumerate() {
            let top = features_top + px(index as f32 * line);
            if top < y {
                continue;
            }
            let last = index + 1 == shown && more > 0;
            let content = if last {
                format!("+{} more", more + 1)
            } else {
                feature.text.clone()
            };
            let colour = if frame.inspected == Some(feature.id) && !last {
                theme.accent.text
            } else if feature.problems > 0 && !last {
                theme.warning.text
            } else {
                theme.text_secondary
            };
            if frame.inspected == Some(feature.id) && !last {
                window.paint_quad(quad(
                    Bounds::new(
                        point(rect.origin.x + px(3.0), top - px(line * 0.12)),
                        size(rect.size.width - px(6.0), px(line * 0.95)),
                    ),
                    px(4.0),
                    theme.accent.soft,
                    px(0.0),
                    gpui::transparent_black(),
                    BorderStyle::Solid,
                ));
            }
            let lock_room = if feature.lock.locked() {
                size_pt * 1.6
            } else {
                0.0
            };
            label(
                window,
                cx,
                &content,
                point(x, top),
                text_size(size_pt),
                theme::REGULAR,
                theme::MONO,
                colour,
                inner - px(lock_room),
            );
            if feature.lock.locked() && !last {
                let s = px(size_pt * 1.1);
                lock_icon(
                    window,
                    cx,
                    point(x + inner - s, top + px(size_pt * 0.1)),
                    s,
                    feature.lock,
                    theme,
                );
            }
        }
    }
}

/// A rounded bar standing in for text too small to read.
fn greek(window: &mut Window, origin: gpui::Point<Pixels>, width: f32, height: f32, colour: Hsla) {
    if width <= 1.0 {
        return;
    }
    window.paint_quad(quad(
        Bounds::new(origin, size(px(width), px(height))),
        px(height * 0.5),
        colour,
        px(0.0),
        gpui::transparent_black(),
        BorderStyle::Solid,
    ));
}

fn measure(window: &mut Window, text: &str, size: Pixels, weight: FontWeight) -> Pixels {
    let mut run_font = font(theme::SANS);
    run_font.weight = weight;
    window
        .text_system()
        .shape_line(
            SharedString::from(text.to_string()),
            size,
            &[TextRun {
                len: text.len(),
                font: run_font,
                color: gpui::black(),
                background_color: None,
                underline: None,
                strikethrough: None,
            }],
            None,
        )
        .width
}

/// The lock mark: full for an element that carries the lock, faint for one
/// covered by an owner's lock.
fn lock_icon(
    window: &mut Window,
    cx: &mut App,
    origin: gpui::Point<Pixels>,
    s: Pixels,
    lock: LockMark,
    theme: &Theme,
) {
    let colour = if lock == LockMark::Own {
        theme.warning.text
    } else {
        theme.warning.text.opacity(0.45)
    };
    let _ = window.paint_svg(
        Bounds::new(origin, size(s, s)),
        crate::ui::IconName::Lock.path().into(),
        None,
        gpui::TransformationMatrix::unit(),
        colour,
        cx,
    );
}

fn port_labels(
    frame: &Frame,
    screen: &Screen,
    objects: &VisibleScene<'_>,
    window: &mut Window,
    cx: &mut App,
) {
    let theme = &frame.theme;
    let zoom = frame.camera.zoom;
    for port in &objects.ports {
        if !port_visible(frame, port.owner, port.id) {
            continue;
        }
        let selected = frame
            .selection
            .targets
            .contains(&SceneTarget::Port(port.owner, port.id));
        let hovered = frame.hovered == Some(SceneTarget::Port(port.owner, port.id));
        if !(frame.lod >= LodLevel::Relationships
            || selected
            || hovered
            || frame.selection.contains(port.owner)
            || frame.lod >= LodLevel::Features && objects.ports.len() <= 60)
        {
            continue;
        }
        let position = screen.point(port.position);
        let owner = frame.lookup.node(&frame.scene, port.owner);
        // A label that would run into the card's own text is left out; the
        // port still shows, and its name is in its tooltip.
        if let Some(node) = owner
            && !port.label_in_header
        {
            let top = screen.point(node.bounds.min).y;
            let half = text_size(theme::text::XS * frame.ui_scale.max(1.0)) * 0.625;
            if position.y - half < top + px(text_block(frame, node)) {
                continue;
            }
        }
        let expanded = owner.is_some_and(|n| n.is_container && !n.collapsed);
        let owner_width = owner.map_or(200.0, |n| n.bounds.width() * zoom);
        let colour = if selected || hovered {
            theme.accent.text
        } else {
            theme.text_muted
        };
        let size_px = text_size(theme::text::XS * frame.ui_scale.max(1.0));
        let max = px((owner_width * 0.44).max(40.0));
        let natural = measure(window, &port.name, size_px, theme::MEDIUM).min(max);
        let height = size_px * 1.25;
        let header = port.label_in_header && zoom * 24.0 >= f32::from(height);
        let offset = |dx: f32, dy: Pixels| point(position.x + px(dx), position.y + dy);
        let origin = match (expanded, header, port.side) {
            (true, true, PortSide::Left) | (false, _, PortSide::Left) => {
                offset(10.0, -height * 0.5)
            }
            (true, true, PortSide::Right) | (false, _, PortSide::Right) => {
                point(position.x - px(10.0) - natural, position.y - height * 0.5)
            }
            (true, false, PortSide::Left) => {
                point(position.x - px(10.0) - natural, position.y - height * 0.5)
            }
            (true, false, PortSide::Right) => offset(10.0, -height * 0.5),
            (_, _, PortSide::Top) => {
                point(position.x - natural * 0.5, position.y - px(10.0) - height)
            }
            (_, _, PortSide::Bottom) => point(position.x - natural * 0.5, position.y + px(10.0)),
        };
        if expanded && !header {
            window.paint_quad(quad(
                Bounds::new(
                    point(origin.x - px(3.0), origin.y - px(1.0)),
                    size(natural + px(6.0), height + px(2.0)),
                ),
                px(3.0),
                theme.canvas,
                px(0.0),
                gpui::transparent_black(),
                BorderStyle::Solid,
            ));
        }
        label(
            window,
            cx,
            &port.name,
            origin,
            size_px,
            theme::MEDIUM,
            theme::SANS,
            colour,
            max,
        );
        if port.lock.locked() {
            let s = px(11.0);
            let x = if origin.x >= position.x {
                origin.x + natural + px(4.0)
            } else {
                origin.x - s - px(4.0)
            };
            lock_icon(
                window,
                cx,
                point(x, origin.y + (height - s) * 0.5),
                s,
                port.lock,
                theme,
            );
        }
    }
}

/// Relationship labels on raised pills with a short leader to their route,
/// placed clear of cards, earlier labels and dense routes.
fn edge_labels(
    frame: &Frame,
    screen: &Screen,
    objects: &VisibleScene<'_>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    use crate::relationship_labels::{PADDING, RouteObstacles, place};
    let theme = &frame.theme;
    if frame.lod < LodLevel::Features && frame.selection.targets.is_empty() {
        return;
    }
    let local = |p: gpui::Point<Pixels>| {
        Point::new(
            f32::from(p.x - bounds.origin.x),
            f32::from(p.y - bounds.origin.y),
        )
    };
    let viewport = Rect::new(
        0.0,
        0.0,
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    );
    let obstacles: Vec<Rect> = objects
        .nodes
        .iter()
        .map(|node| {
            let r = screen.rect(node.bounds);
            let mut rect = Rect::from_points(local(r.origin), local(r.bottom_right()));
            if node.is_container && !node.collapsed {
                rect.max.y = rect.max.y.min(rect.min.y + 58.0 * frame.camera.zoom);
            }
            rect
        })
        .collect();
    let mut routes = RouteObstacles::default();
    for edge in &objects.edges {
        for pair in edge.points.windows(2) {
            routes.insert(
                local(screen.point(pair[0])),
                local(screen.point(pair[1])),
                viewport,
            );
        }
    }
    let size_px = text_size(theme::text::XS * frame.ui_scale.max(1.0));
    let mut placed = Vec::new();
    let mut automatic = 0;
    window.paint_layer(bounds, |window| {
        for edge in &objects.edges {
            let semantic = &edge.semantic;
            let target = SceneTarget::Edge(semantic.id.clone());
            let explicit = frame.selection.targets.contains(&target)
                || frame.hovered.as_ref() == Some(&target);
            let incident = frame.selection.contains(semantic.source.node)
                || frame.selection.contains(semantic.target.node)
                || semantic
                    .element
                    .is_some_and(|e| frame.highlights.contains_key(&e));
            let shown = explicit
                || semantic.lock.locked()
                || (incident && objects.edges.len() <= 40)
                || (frame.lod >= LodLevel::Relationships && objects.edges.len() <= 16);
            if !shown || (!explicit && automatic >= 10) || semantic.label.is_empty() {
                continue;
            }
            let max_text = (f32::from(bounds.size.width) * 0.3).clamp(80.0, 280.0);
            let natural =
                f32::from(measure(window, &semantic.label, size_px, theme::MEDIUM)).min(max_text);
            let lock_room = if semantic.lock.locked() { 16.0 } else { 0.0 };
            let pill = agq_studio_scene::Size::new(
                natural + 2.0 * PADDING.width + lock_room,
                f32::from(size_px) * 1.25 + 2.0 * PADDING.height,
            );
            let route: Vec<Point> = edge
                .points
                .iter()
                .map(|p| local(screen.point(*p)))
                .collect();
            let Some(placement) = place(
                &route, pill, viewport, &obstacles, &placed, explicit, &routes,
            ) else {
                continue;
            };
            let to_window = |p: Point| point(bounds.origin.x + px(p.x), bounds.origin.y + px(p.y));
            let rect = Bounds::from_corners(
                to_window(placement.bounds.min),
                to_window(placement.bounds.max),
            );
            let anchor = to_window(placement.anchor);
            let (line, text) = if explicit {
                (theme.accent.solid, theme.text)
            } else {
                (theme.border_strong, theme.text_secondary)
            };
            let end = point(
                anchor
                    .x
                    .clamp(rect.origin.x, rect.origin.x + rect.size.width),
                anchor
                    .y
                    .clamp(rect.origin.y, rect.origin.y + rect.size.height),
            );
            segment(window, anchor, end, px(1.0), line, false);
            window.paint_quad(quad(
                Bounds::new(
                    point(anchor.x - px(2.5), anchor.y - px(2.5)),
                    size(px(5.0), px(5.0)),
                ),
                px(2.5),
                line,
                px(0.0),
                gpui::transparent_black(),
                BorderStyle::Solid,
            ));
            window.paint_drop_shadows(
                rect,
                Corners::all(rect.size.height * 0.5),
                &theme.shadow_small(),
            );
            window.paint_quad(quad(
                rect,
                rect.size.height * 0.5,
                theme.raised,
                px(1.0),
                if explicit {
                    theme.accent.solid
                } else {
                    theme.border
                },
                BorderStyle::Solid,
            ));
            label(
                window,
                cx,
                &semantic.label,
                point(
                    rect.origin.x + px(PADDING.width),
                    rect.origin.y + px(PADDING.height),
                ),
                size_px,
                theme::MEDIUM,
                theme::SANS,
                text,
                px(natural + 1.0),
            );
            if semantic.lock.locked() {
                let s = px(11.0);
                lock_icon(
                    window,
                    cx,
                    point(
                        rect.origin.x + rect.size.width - px(PADDING.width) - s,
                        rect.origin.y + (rect.size.height - s) * 0.5,
                    ),
                    s,
                    semantic.lock,
                    theme,
                );
            }
            placed.push(placement.bounds);
            automatic += usize::from(!explicit);
        }
    });
}

fn gesture(frame: &Frame, screen: &Screen, window: &mut Window) {
    let theme = &frame.theme;
    match &frame.gesture {
        Some(Gesture::Marquee { start, end }) => {
            let area = Bounds::from_corners(screen.point(*start), screen.point(*end));
            let area = Bounds::from_corners(
                point(
                    area.origin.x.min(area.bottom_right().x),
                    area.origin.y.min(area.bottom_right().y),
                ),
                point(
                    area.origin.x.max(area.bottom_right().x),
                    area.origin.y.max(area.bottom_right().y),
                ),
            );
            window.paint_quad(quad(
                area,
                px(2.0),
                theme.accent.solid.opacity(0.08),
                px(1.0),
                theme.accent.solid,
                BorderStyle::Solid,
            ));
        }
        Some(Gesture::Move { card, start, now }) => {
            if let Some(node) = frame.lookup.node(&frame.scene, *card) {
                let moved = node
                    .bounds
                    .translate(Point::new(now.x - start.x, now.y - start.y));
                let rect = screen.rect(moved);
                window.paint_drop_shadows(rect, Corners::all(px(8.0)), &theme.shadow_overlay());
                window.paint_quad(quad(
                    rect,
                    px(8.0),
                    theme.raised.opacity(0.85),
                    px(1.5),
                    theme.accent.solid,
                    BorderStyle::Solid,
                ));
            }
        }
        Some(Gesture::Connect { card, port, now }) => {
            if let Some(from) = frame.lookup.port(&frame.scene, *card, *port) {
                let a = screen.point(from.position);
                let b = screen.point(*now);
                let mut path = PathBuilder::stroke(px(2.0));
                path.move_to(a);
                path.line_to(b);
                if let Ok(path) = path.build() {
                    window.paint_path(path, theme.accent.solid);
                }
                window.paint_quad(quad(
                    Bounds::new(point(b.x - px(4.0), b.y - px(4.0)), size(px(8.0), px(8.0))),
                    px(4.0),
                    theme.accent.solid,
                    px(0.0),
                    gpui::transparent_black(),
                    BorderStyle::Solid,
                ));
            }
        }
        _ => {}
    }
}
