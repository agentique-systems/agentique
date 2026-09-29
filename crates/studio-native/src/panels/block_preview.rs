//! A building block's structural preview (C-49, Scenario H1): what it is,
//! what it exposes and what is inside it, in the Surface's visual language
//! (cards with a kind mark, ports on their edges, calling sides on the
//! right) at a size that stays legible. A composite shows its boundary with
//! its ports, its inner parts in columns from where requests enter to where
//! they leave, and the connections between them; an atomic block shows one
//! card with its ports. It is not a small Surface: only the top level of the
//! block's inside is drawn.
use crate::{
    surface::paint::{category_colour, label},
    ui::{ActiveTheme, Theme, r, theme},
};
use agq_library::{Preview, PreviewEnd, PreviewPart, PreviewPort};
use agq_studio_scene::NodeCategory;
use gpui::{
    App, BorderStyle, Bounds, InteractiveElement, IntoElement, ParentElement, PathBuilder, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, point, px,
    quad, size,
};
use std::rc::Rc;

/// The preview of one block, drawn to fill its box.
#[derive(IntoElement)]
pub struct BlockPreview {
    id: SharedString,
    preview: Rc<Preview>,
    height: f32,
}

impl BlockPreview {
    pub fn new(id: impl Into<SharedString>, preview: Rc<Preview>) -> Self {
        BlockPreview {
            id: id.into(),
            preview,
            height: 150.0,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

/// The height (in points) a preview needs: a composite a fixed box, an
/// atomic block its card's rows.
pub fn height(preview: &Preview) -> f32 {
    if !preview.parts.is_empty() {
        return 160.0;
    }
    let (rows, lines) = atomic_rows(preview);
    (ATOMIC_HEADER + ROW_STEP * (rows + lines) as f32 + 8.0 + 24.0).clamp(84.0, 160.0)
}

const ATOMIC_HEADER: f32 = 38.0;
const ROW_STEP: f32 = 14.0;
const MAX_LINES: usize = 3;

/// An atomic card's port rows and value lines.
fn atomic_rows(preview: &Preview) -> (usize, usize) {
    let inbound = preview.ports.iter().filter(|q| q.inbound).count();
    let outbound = preview.ports.len() - inbound;
    let lines = (preview.attributes.len() + preview.items.len()).min(MAX_LINES);
    (inbound.max(outbound), lines)
}

/// What a screen reader says for a preview.
pub fn describe(preview: &Preview) -> String {
    let names = |ports: &[PreviewPort]| {
        ports
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut text = format!("{}, {}", preview.name, preview.kind_label);
    if !preview.ports.is_empty() {
        text.push_str(&format!("; ports {}", names(&preview.ports)));
    }
    if !preview.parts.is_empty() {
        let parts: Vec<String> = preview
            .parts
            .iter()
            .map(|p| format!("{} : {}", p.name, p.type_name))
            .collect();
        text.push_str(&format!("; parts {}", parts.join(", ")));
    }
    if !preview.links.is_empty() {
        text.push_str(&format!("; {} connection(s)", preview.links.len()));
    }
    text
}

impl RenderOnce for BlockPreview {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let preview = self.preview.clone();
        let scale = f32::from(window.rem_size()) / 16.0;
        div()
            .id(self.id.clone())
            .role(gpui::Role::Figure)
            .aria_label(SharedString::from(describe(&self.preview)))
            .w_full()
            .flex_none()
            .h(r(self.height))
            .rounded(r(crate::tokens::radius::CARD))
            .bg(theme.canvas)
            .border_1()
            .border_color(theme.separator)
            .overflow_hidden()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| paint(&preview, &theme, scale, bounds, window, cx),
                )
                .size_full(),
            )
    }
}

/// Points scaled by the UI scale.
fn p(points: f32, scale: f32) -> Pixels {
    px(points * scale)
}

struct Card {
    bounds: Bounds<Pixels>,
    /// Port centres: (inbound on the left, outbound on the right).
    ports: Vec<gpui::Point<Pixels>>,
}

fn paint(
    preview: &Preview,
    theme: &Theme,
    scale: f32,
    area: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let inset = p(10.0, scale);
    let label_size = px((9.5 * scale).round());
    if preview.parts.is_empty() {
        // An atomic block: one card, its ports on its edges below the
        // title with their names inside it, its values below them.
        let width = (area.size.width - inset * 2.0 - p(8.0, scale)).min(p(240.0, scale));
        let (rows, _) = atomic_rows(preview);
        let lines: Vec<(String, String)> = preview
            .attributes
            .iter()
            .chain(preview.items.iter())
            .take(MAX_LINES)
            .cloned()
            .collect();
        let header = p(ATOMIC_HEADER, scale);
        let step = p(ROW_STEP, scale);
        let height = (header + step * (rows + lines.len()) as f32 + p(8.0, scale))
            .min(area.size.height - inset * 2.0);
        let origin = point(
            area.origin.x + (area.size.width - width) * 0.5,
            area.origin.y + (area.size.height - height) * 0.5,
        );
        let card = Bounds::new(origin, size(width, height));
        let category = if preview.kind_label.contains("requirement") {
            NodeCategory::Requirement
        } else {
            NodeCategory::Definition
        };
        draw_card(window, theme, card, category, scale);
        let inner = width - p(20.0, scale);
        label(
            window,
            cx,
            &preview.kind_label.to_uppercase(),
            point(
                card.origin.x + p(10.0, scale),
                card.origin.y + p(7.0, scale),
            ),
            px((9.0 * scale).round()),
            theme::SEMIBOLD,
            theme::SANS,
            category_colour(category, theme),
            inner,
        );
        label(
            window,
            cx,
            &preview.name,
            point(
                card.origin.x + p(10.0, scale),
                card.origin.y + p(19.0, scale),
            ),
            px((12.0 * scale).round()),
            theme::SEMIBOLD,
            theme::SANS,
            theme.text,
            inner,
        );
        let half = width * 0.5 - p(12.0, scale);
        let (mut i, mut o) = (0, 0);
        for port in &preview.ports {
            let row = if port.inbound {
                i += 1;
                i
            } else {
                o += 1;
                o
            };
            let y = card.origin.y + header + step * (row as f32 - 0.5);
            let x = if port.inbound {
                card.origin.x
            } else {
                card.origin.x + width
            };
            port_label(
                window,
                cx,
                theme,
                port,
                point(x, y),
                half,
                false,
                label_size,
                scale,
            );
            dot(window, theme, point(x, y), scale, true);
        }
        for (i, (name, detail)) in lines.iter().enumerate() {
            label(
                window,
                cx,
                &format!("{name} {detail}"),
                point(
                    card.origin.x + p(10.0, scale),
                    card.origin.y + header + step * (rows + i) as f32 + p(2.0, scale),
                ),
                px((9.5 * scale).round()),
                theme::REGULAR,
                theme::MONO,
                theme.text_muted,
                inner,
            );
        }
        return;
    }
    // A composite's boundary port names stand inside its frame, above their
    // lines; the inside keeps clear of them, up to a third of the width
    // each side.
    let widest = |window: &mut Window, inbound: bool| {
        preview
            .ports
            .iter()
            .filter(|q| q.inbound == inbound)
            .map(|q| measure_width(window, &q.name, scale))
            .fold(px(0.0), |a, b| a.max(b))
            .min(area.size.width * 0.3)
    };
    let (left_labels, right_labels) = (widest(window, true), widest(window, false));
    // A composite: its boundary with its ports, its inside in columns.
    let frame = Bounds::new(
        point(area.origin.x + inset, area.origin.y + p(8.0, scale)),
        size(
            area.size.width - inset * 2.0,
            area.size.height - p(16.0, scale),
        ),
    );
    window.paint_quad(quad(
        frame,
        p(8.0, scale),
        theme.chrome,
        px(1.0),
        theme.border_strong,
        BorderStyle::Solid,
    ));
    label(
        window,
        cx,
        &preview.name,
        point(
            frame.origin.x + p(10.0, scale),
            frame.origin.y + p(5.0, scale),
        ),
        px((10.0 * scale).round()),
        theme::SEMIBOLD,
        theme::SANS,
        theme.text_secondary,
        frame.size.width - p(20.0, scale),
    );
    let top = frame.origin.y + p(22.0, scale);
    let bottom = frame.origin.y + frame.size.height - p(8.0, scale);
    // The inside leaves room for the boundary ports' names.
    let left = frame.origin.x + p(10.0, scale) + left_labels + p(12.0, scale);
    let right = frame.origin.x + frame.size.width - p(10.0, scale) - right_labels - p(12.0, scale);
    let boundary = {
        let inbound = preview.ports.iter().filter(|q| q.inbound).count();
        let outbound = preview.ports.len() - inbound;
        let (mut i, mut o) = (0, 0);
        let mut centres = Vec::new();
        for port in &preview.ports {
            let (count, index, x) = if port.inbound {
                i += 1;
                (inbound, i, frame.origin.x)
            } else {
                o += 1;
                (outbound, o, frame.origin.x + frame.size.width)
            };
            let y = top + (bottom - top) * (index as f32 / (count as f32 + 1.0));
            let centre = point(x, y);
            port_label(
                window,
                cx,
                theme,
                port,
                centre,
                if port.inbound {
                    left_labels
                } else {
                    right_labels
                },
                true,
                label_size,
                scale,
            );
            centres.push(centre);
        }
        centres
    };
    let columns = preview.columns.max(1);
    let gap = p(12.0, scale);
    let column_width = ((right - left) - gap * (columns as f32 - 1.0)) / columns as f32;
    let rows_in = |column: usize| preview.parts.iter().filter(|q| q.column == column).count();
    let mut cards: Vec<Card> = Vec::new();
    for part in &preview.parts {
        let rows = rows_in(part.column).max(1);
        let row_gap = p(8.0, scale);
        let row_height = ((bottom - top) - row_gap * (rows as f32 - 1.0)) / rows as f32;
        let height = row_height.min(p(40.0, scale));
        let x = left + (column_width + gap) * part.column as f32;
        let y = top + (row_height + row_gap) * part.row as f32 + (row_height - height) * 0.5;
        let card = Bounds::new(point(x, y), size(column_width, height));
        cards.push(inner_card(window, cx, theme, part, card, scale));
    }
    // Connections: drawn in right angles between their ends.
    let end_point = |end: PreviewEnd, from: bool| -> Option<gpui::Point<Pixels>> {
        match end {
            PreviewEnd::Boundary(i) => boundary.get(i).copied(),
            PreviewEnd::Part(j, Some(port)) => cards.get(j)?.ports.get(port).copied(),
            PreviewEnd::Part(j, None) => {
                let card = cards.get(j)?.bounds;
                Some(point(
                    if from {
                        card.origin.x + card.size.width
                    } else {
                        card.origin.x
                    },
                    card.origin.y + card.size.height * 0.5,
                ))
            }
        }
    };
    for link in &preview.links {
        let (Some(a), Some(b)) = (end_point(link.from, true), end_point(link.to, false)) else {
            continue;
        };
        let (a, b) = if a.x <= b.x { (a, b) } else { (b, a) };
        // From the boundary, the line bends near the inner card, clear of
        // the boundary port names.
        let middle = if a.x <= frame.origin.x + px(1.0) {
            b.x - p(8.0, scale)
        } else if b.x >= frame.origin.x + frame.size.width - px(1.0) {
            a.x + p(8.0, scale)
        } else {
            a.x + (b.x - a.x) * 0.5
        };
        let mut path = PathBuilder::stroke(px(1.25 * scale));
        path.move_to(a);
        path.line_to(point(middle, a.y));
        path.line_to(point(middle, b.y));
        path.line_to(b);
        if let Ok(path) = path.build() {
            window.paint_path(path, theme.edge);
        }
    }
    // Ports over the lines.
    for card in &cards {
        for port in &card.ports {
            dot(window, theme, *port, scale, false);
        }
    }
    for port in &boundary {
        dot(window, theme, *port, scale, true);
    }
}

/// A port's name inside what it belongs to, beside it: right of an inbound
/// port, left of an outbound one; `above` its line on a composite's
/// boundary, where a connection leaves the port.
#[allow(clippy::too_many_arguments)]
fn port_label(
    window: &mut Window,
    cx: &mut App,
    theme: &Theme,
    port: &PreviewPort,
    at: gpui::Point<Pixels>,
    room: Pixels,
    above: bool,
    size: Pixels,
    scale: f32,
) {
    let colour = if port.inherited {
        theme.text_muted
    } else {
        theme.text_secondary
    };
    let room = room.max(p(20.0, scale));
    let width = measure_width(window, &port.name, scale).min(room);
    let gap = p(if above { 4.0 } else { 8.0 }, scale);
    let x = if port.inbound {
        at.x + gap
    } else {
        at.x - gap - width
    };
    let y = if above {
        at.y - p(15.0, scale)
    } else {
        at.y - p(7.0, scale)
    };
    label(
        window,
        cx,
        &port.name,
        point(x, y),
        size,
        theme::REGULAR,
        theme::MONO,
        colour,
        room,
    );
}

/// A card like the Surface's: raised, with a thin kind mark along the top.
fn draw_card(
    window: &mut Window,
    theme: &Theme,
    card: Bounds<Pixels>,
    category: NodeCategory,
    scale: f32,
) {
    window.paint_quad(quad(
        card,
        p(6.0, scale),
        theme.raised,
        px(1.0),
        theme.border,
        BorderStyle::Solid,
    ));
    window.paint_quad(quad(
        Bounds::new(
            point(card.origin.x + p(6.0, scale), card.origin.y),
            size(card.size.width - p(12.0, scale), p(2.0, scale)),
        ),
        px(1.0),
        category_colour(category, theme),
        px(0.0),
        gpui::transparent_black(),
        BorderStyle::Solid,
    ));
}

/// An inner part: its name and type, and its ports on its edges.
fn inner_card(
    window: &mut Window,
    cx: &mut App,
    theme: &Theme,
    part: &PreviewPart,
    card: Bounds<Pixels>,
    scale: f32,
) -> Card {
    draw_card(window, theme, card, NodeCategory::Part, scale);
    if part.composite {
        // A composite inside a composite: a second edge, as a stack.
        window.paint_quad(quad(
            Bounds::new(
                point(
                    card.origin.x + p(3.0, scale),
                    card.origin.y + card.size.height,
                ),
                size(card.size.width - p(6.0, scale), p(2.0, scale)),
            ),
            px(1.0),
            theme.border_strong,
            px(0.0),
            gpui::transparent_black(),
            BorderStyle::Solid,
        ));
    }
    let width = card.size.width - p(14.0, scale);
    let mut name = part.name.clone();
    if let Some(m) = &part.multiplicity {
        name.push_str(&format!(" {m}"));
    }
    label(
        window,
        cx,
        &name,
        point(card.origin.x + p(7.0, scale), card.origin.y + p(5.0, scale)),
        px((10.5 * scale).round()),
        theme::MEDIUM,
        theme::SANS,
        theme.text,
        width,
    );
    if card.size.height > p(28.0, scale) {
        label(
            window,
            cx,
            &part.type_name,
            point(
                card.origin.x + p(7.0, scale),
                card.origin.y + p(19.0, scale),
            ),
            px((9.5 * scale).round()),
            theme::REGULAR,
            theme::MONO,
            theme.text_muted,
            width,
        );
    }
    let place = |ports: &[&PreviewPort], x: Pixels| -> Vec<(usize, gpui::Point<Pixels>)> {
        let count = ports.len();
        (0..count)
            .map(|i| {
                let y =
                    card.origin.y + card.size.height * ((i as f32 + 1.0) / (count as f32 + 1.0));
                (i, point(x, y))
            })
            .collect()
    };
    let inbound: Vec<&PreviewPort> = part.ports.iter().filter(|q| q.inbound).collect();
    let outbound: Vec<&PreviewPort> = part.ports.iter().filter(|q| !q.inbound).collect();
    let left = place(&inbound, card.origin.x);
    let right = place(&outbound, card.origin.x + card.size.width);
    let (mut next_in, mut next_out) = (0, 0);
    let ports = part
        .ports
        .iter()
        .map(|port| {
            if port.inbound {
                next_in += 1;
                left[next_in - 1].1
            } else {
                next_out += 1;
                right[next_out - 1].1
            }
        })
        .collect();
    Card {
        bounds: card,
        ports,
    }
}

/// The width a port name takes, to right-align it.
fn measure_width(window: &mut Window, text: &str, scale: f32) -> Pixels {
    let mut run_font = gpui::font(theme::MONO);
    run_font.weight = theme::REGULAR;
    let runs = [gpui::TextRun {
        len: text.len(),
        font: run_font,
        color: gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    window
        .text_system()
        .shape_line(
            SharedString::from(text.to_string()),
            px((9.5 * scale).round()),
            &runs,
            None,
        )
        .width
}

/// A port: a hollow circle, filled on the boundary.
fn dot(
    window: &mut Window,
    theme: &Theme,
    centre: gpui::Point<Pixels>,
    scale: f32,
    boundary: bool,
) {
    let radius = p(if boundary { 4.0 } else { 3.0 }, scale);
    window.paint_quad(quad(
        Bounds::new(
            point(centre.x - radius, centre.y - radius),
            size(radius * 2.0, radius * 2.0),
        ),
        radius,
        if boundary {
            theme.accent.solid
        } else {
            theme.raised
        },
        px(1.25),
        theme.accent.text,
        BorderStyle::Solid,
    ));
}
