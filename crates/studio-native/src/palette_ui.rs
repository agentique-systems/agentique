//! One keyboard cursor across contextual commands and visible engineering objects.
use crate::{
    app::StudioApp,
    commands::{self, CommandId},
};
use agq_kernel::ElementId;
use agq_studio_scene::SceneTarget;
use eframe::egui::{self, Align2, Key, Modifiers, Vec2};

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
    label: String,
    reason: Option<&'static str>,
}

impl StudioApp {
    pub fn command_palette(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut chosen = None;
        let cursor_id = egui::Id::new("command-palette-cursor");
        let mut cursor = ctx
            .data(|data| data.get_temp::<Cursor>(cursor_id))
            .unwrap_or_default();
        egui::Window::new("Commands")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, [0.0, 130.0])
            .fixed_size([590.0, 420.0])
            .show(ctx, |ui| {
                let down = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowDown));
                let up = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::ArrowUp));
                let enter = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
                let label = ui.label("Command or element");
                let input = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text("Find a command or focus an element…")
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(label.id);
                if self.palette_focus {
                    input.request_focus();
                    self.palette_focus = false;
                }
                crate::automation::record(ctx, crate::automation::Target::PaletteInput, input.rect);
                let context = self.context();
                let mut rows: Vec<_> = commands::search(&self.palette_query)
                    .map(|command| {
                        let reason = commands::unavailable(command.id, &context);
                        Row {
                            action: Action::Command(command.id),
                            label: format!(
                                "{}    {}\n{}",
                                command.label,
                                command.shortcut,
                                reason.unwrap_or(command.description)
                            ),
                            reason,
                        }
                    })
                    .collect();
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
                        label: format!(
                            "Focus: {}\n{} in the loaded view",
                            node.semantic.name,
                            node.category.label()
                        ),
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
                ui.label(
                    crate::app::muted("↑ ↓ choose · Enter run · Esc close", self.theme).small(),
                );
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if rows.is_empty() {
                        ui.label("No matching commands or visible objects");
                    }
                    for (index, row) in rows.iter().enumerate() {
                        let response = ui.add_enabled(
                            row.reason.is_none(),
                            egui::Button::new(&row.label)
                                .selected(index == cursor.row)
                                .min_size(Vec2::new(ui.available_width(), 52.0)),
                        );
                        if index == cursor.row && (up || down || input.changed()) {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                        if response.clicked()
                            || (enter && index == cursor.row && row.reason.is_none())
                        {
                            chosen = Some(row.action);
                        }
                    }
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
