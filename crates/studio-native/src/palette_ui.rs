//! One keyboard cursor across contextual commands and visible engineering objects.
use crate::{
    app::StudioApp,
    commands::{self, CommandId},
    theme::{self, Theme},
};
use agq_kernel::ElementId;
use agq_studio_scene::SceneTarget;
use eframe::egui::{self, Align2, Key, Modifiers, Sense, Vec2};

#[derive(Clone, Default)]
struct Cursor {
    query: String,
    row: usize,
}
#[derive(Clone, Copy)]
enum Action {
    Command(CommandId),
    Focus(ElementId),
}
struct Row {
    action: Action,
    title: String,
    detail: String,
    shortcut: &'static str,
    reason: Option<&'static str>,
}

const WIDTH: f32 = 620.0;
const LIST_HEIGHT: f32 = 380.0;
const ROW_HEIGHT: f32 = 48.0;

impl StudioApp {
    pub fn command_palette(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut chosen = None;
        let cursor_id = egui::Id::new("command-palette-cursor");
        let mut cursor = ctx
            .data(|data| data.get_temp::<Cursor>(cursor_id))
            .unwrap_or_default();
        let theme = self.theme;
        egui::Window::new("Commands")
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, [0.0, 110.0])
            .min_width(WIDTH)
            .max_width(WIDTH)
            .frame(
                egui::Frame::window(&ctx.style())
                    .inner_margin(0)
                    .corner_radius(theme::RADIUS_XL),
            )
            .show(ctx, |ui| {
                let down = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
                let up = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
                let enter = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let input = egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 18,
                        right: 18,
                        top: 14,
                        bottom: 12,
                    })
                    .show(ui, |ui| {
                        let label = ui.label(theme.overline("Command or element"));
                        ui.add_space(theme::SPACE);
                        ui.horizontal(|ui| {
                            search_icon(ui, theme);
                            ui.add_space(theme::SPACE);
                            ui.add(
                                egui::TextEdit::singleline(&mut self.palette_query)
                                    .hint_text(
                                        "Search commands, or type an element name to focus it…",
                                    )
                                    .font(theme::regular(theme::HEADING + 1.0))
                                    .frame(false)
                                    .margin(Vec2::ZERO)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(label.id)
                        })
                        .inner
                    })
                    .inner;
                if self.palette_focus {
                    input.request_focus();
                    self.palette_focus = false;
                }
                crate::targets::record(ctx, crate::targets::Target::PaletteInput, input.rect);
                divider(ui, theme);
                let context = self.context();
                let mut rows: Vec<_> = commands::search(&self.palette_query)
                    .map(|command| {
                        let reason = commands::unavailable(command.id, &context);
                        Row {
                            action: Action::Command(command.id),
                            title: command.label.into(),
                            detail: reason.unwrap_or(command.description).into(),
                            shortcut: command.shortcut,
                            reason,
                        }
                    })
                    .collect();
                let commands_found = rows.len();
                let query = self.palette_query.trim().to_lowercase();
                let query = query
                    .strip_prefix("focus:")
                    .or_else(|| query.strip_prefix("focus "))
                    .unwrap_or(&query)
                    .trim();
                if !query.is_empty() {
                    let mut matches: Vec<_> = self
                        .scene
                        .nodes
                        .iter()
                        .filter_map(|node| {
                            commands::fuzzy_score(query, &node.semantic.name)
                                .map(|score| (score, node))
                        })
                        .collect();
                    matches.sort_by_key(|(score, _)| *score);
                    rows.extend(matches.into_iter().take(12).map(|(_, node)| Row {
                        action: Action::Focus(node.id()),
                        title: format!("Focus: {}", node.semantic.name),
                        detail: format!("{} in the loaded view", node.category.label()),
                        shortcut: "",
                        reason: None,
                    }));
                }
                let enabled: Vec<_> = rows
                    .iter()
                    .enumerate()
                    .filter_map(|(i, row)| row.reason.is_none().then_some(i))
                    .collect();
                if cursor.query != self.palette_query || !enabled.contains(&cursor.row) {
                    cursor.query = self.palette_query.clone();
                    cursor.row = enabled.first().copied().unwrap_or(0);
                }
                if (up || down) && !enabled.is_empty() {
                    let current = enabled
                        .iter()
                        .position(|row| *row == cursor.row)
                        .unwrap_or(0);
                    cursor.row = enabled[if down {
                        (current + 1) % enabled.len()
                    } else {
                        (current + enabled.len() - 1) % enabled.len()
                    }];
                }
                egui::ScrollArea::vertical()
                    .max_height(LIST_HEIGHT)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                if rows.is_empty() {
                                    ui.add_space(theme::SPACE_XL);
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(
                                                "No matching commands or visible elements",
                                            )
                                            .color(theme.muted),
                                        );
                                    });
                                    ui.add_space(theme::SPACE_XL);
                                }
                                for (index, row) in rows.iter().enumerate() {
                                    if index == 0 && commands_found > 0 {
                                        group_heading(ui, theme, "Commands");
                                    } else if index == commands_found {
                                        group_heading(ui, theme, "Elements");
                                    }
                                    let current = index == cursor.row;
                                    let response = palette_row(ui, theme, row, current);
                                    if current && (up || down || input.changed()) {
                                        response.scroll_to_me(Some(egui::Align::Center));
                                    }
                                    if response.clicked()
                                        || (enter && current && row.reason.is_none())
                                    {
                                        chosen = Some(row.action);
                                    }
                                }
                            });
                    });
                divider(ui, theme);
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(18, 9))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = theme::SPACE_S + 2.0;
                            for (keys, action) in
                                [("↑ ↓", "choose"), ("Enter", "run"), ("Esc", "close")]
                            {
                                keycap(ui, theme, keys);
                                ui.label(
                                    egui::RichText::new(action)
                                        .font(theme::regular(theme::CAPTION))
                                        .color(theme.muted),
                                );
                                ui.add_space(theme::SPACE);
                            }
                        });
                    });
            });
        ctx.data_mut(|data| data.insert_temp(cursor_id, cursor));
        self.palette &= open;
        if let Some(action) = chosen {
            self.palette = false;
            match action {
                Action::Command(command) => self.execute(command, ctx),
                Action::Focus(id) => {
                    self.select(SceneTarget::Node(id), false);
                    self.execute(CommandId::Focus, ctx);
                }
            }
        }
    }
}

/// One result: title and detail on the left, shortcut keycaps on the right.
/// The keyboard cursor and hover share one highlight; unavailable commands
/// stay visible with their reason but cannot run.
fn palette_row(ui: &mut egui::Ui, theme: Theme, row: &Row, current: bool) -> egui::Response {
    let enabled = row.reason.is_none();
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ROW_HEIGHT),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            enabled,
            current,
            format!("{}, {}", row.title, row.detail),
        )
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if current && enabled {
        painter.rect_filled(rect, theme::RADIUS, theme.selection);
    } else if enabled && response.hovered() {
        painter.rect_filled(rect, theme::RADIUS, theme.hover);
    }
    let (title, detail) = if enabled {
        (theme.text, theme.muted)
    } else {
        let faded = theme.muted.gamma_multiply(0.8);
        (faded, faded)
    };
    let inner = rect.shrink2(Vec2::new(theme::SPACE_L, 7.0));
    let mut right = inner.right();
    if !row.shortcut.is_empty() {
        for key in row.shortcut.rsplit('+') {
            let galley =
                painter.layout_no_wrap(key.to_owned(), theme::medium(theme::CAPTION), detail);
            let cap = egui::Rect::from_min_max(
                egui::pos2(
                    right - galley.size().x - 2.0 * theme::SPACE_S - 2.0,
                    inner.center().y - 10.0,
                ),
                egui::pos2(right, inner.center().y + 10.0),
            );
            painter.rect(
                cap,
                theme::RADIUS_S,
                theme.surface,
                egui::Stroke::new(theme::HAIRLINE, theme.border_strong),
                egui::StrokeKind::Inside,
            );
            painter.galley(cap.center() - galley.size() * 0.5, galley, detail);
            right = cap.left() - theme::SPACE_S;
        }
    }
    let width = (right - inner.left() - theme::SPACE).max(20.0);
    let text = |text: &str, font: egui::FontId, color| {
        let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font, color);
        job.wrap.max_width = width;
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.wrap.overflow_character = Some('…');
        painter.layout_job(job)
    };
    let title_galley = text(&row.title, theme::medium(theme::BODY), title);
    let detail_galley = text(&row.detail, theme::regular(theme::LABEL), detail);
    painter.galley(inner.left_top(), title_galley, title);
    painter.galley(
        egui::pos2(inner.left(), inner.bottom() - detail_galley.size().y),
        detail_galley,
        detail,
    );
    response
}

fn group_heading(ui: &mut egui::Ui, theme: Theme, text: &str) {
    ui.add_space(theme::SPACE);
    ui.horizontal(|ui| {
        ui.add_space(theme::SPACE_L);
        ui.label(theme.overline(text));
    });
    ui.add_space(theme::SPACE_S);
}

fn keycap(ui: &mut egui::Ui, theme: Theme, keys: &str) {
    egui::Frame::new()
        .fill(theme.surface)
        .stroke(egui::Stroke::new(theme::HAIRLINE, theme.border_strong))
        .corner_radius(theme::RADIUS_S)
        .inner_margin(egui::Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(keys)
                    .font(theme::medium(theme::CAPTION))
                    .color(theme.text_secondary),
            );
        });
}

fn divider(ui: &mut egui::Ui, theme: Theme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.border);
}

fn search_icon(ui: &mut egui::Ui, theme: Theme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
    let stroke = egui::Stroke::new(1.6, theme.muted);
    let center = rect.center() - Vec2::splat(1.5);
    ui.painter().circle_stroke(center, 5.5, stroke);
    ui.painter().line_segment(
        [center + Vec2::splat(4.0), center + Vec2::splat(8.0)],
        stroke,
    );
}
