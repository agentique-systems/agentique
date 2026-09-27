//! The Studio's frame: top bar, outline, Panels, status bar and the start
//! screen. The Surface fills the middle.
use crate::{
    app::{Panel, StudioApp},
    commands::{self, CommandId},
    navigation::SurfaceView,
    targets::{Target, record},
    theme,
};
use agq_studio_scene::{LockMark, NodeCategory, SceneTarget};
use eframe::egui::{self, RichText, Vec2};

impl StudioApp {
    pub fn shell(&mut self, root: &mut egui::Ui, ctx: &egui::Context) {
        let theme = self.theme;
        egui::Panel::top("top-bar")
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Agentique").font(theme::semibold(theme::HEADING)));
                    ui.add_space(6.0);
                    let title = match (&self.project, &self.fixture) {
                        (Some(project), _) => project.folder().file_name().map_or_else(
                            || project.folder().display().to_string(),
                            |n| n.to_string_lossy().into_owned(),
                        ),
                        (None, Some(name)) => format!("Example: {name} (read-only)"),
                        (None, None) => "No project".into(),
                    };
                    ui.label(crate::app::muted(title, theme));
                    ui.add_space(18.0);
                    for view in SurfaceView::ALL {
                        let button =
                            ui.add(egui::Button::selectable(self.view == view, view.title()));
                        if button.clicked() {
                            self.set_view(view);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⌘ Commands").on_hover_text("Ctrl+K").clicked() {
                            self.execute(CommandId::Palette, ctx);
                        }
                        if ui
                            .add(egui::Button::selectable(self.settings.open, "Settings"))
                            .on_hover_text("Ctrl+,")
                            .clicked()
                        {
                            self.execute(CommandId::Settings, ctx);
                        }
                        let context = self.context();
                        for (id, label) in [
                            (CommandId::Checkpoint, "Checkpoint"),
                            (CommandId::Redo, "Redo"),
                            (CommandId::Undo, "Undo"),
                        ] {
                            let command = commands::command(id);
                            let enabled = commands::unavailable(id, &context).is_none();
                            let tip = match id {
                                CommandId::Undo => self
                                    .project
                                    .as_ref()
                                    .and_then(|p| p.state().undo_description())
                                    .map(|d| format!("Undo {d} ({})", command.shortcut)),
                                CommandId::Redo => self
                                    .project
                                    .as_ref()
                                    .and_then(|p| p.state().redo_description())
                                    .map(|d| format!("Redo {d} ({})", command.shortcut)),
                                _ => None,
                            }
                            .unwrap_or_else(|| format!("{} ({})", command.label, command.shortcut));
                            if ui
                                .add_enabled(enabled, egui::Button::new(label))
                                .on_hover_text(tip)
                                .clicked()
                            {
                                self.execute(id, ctx);
                            }
                        }
                        if ui.button("Open…").clicked() {
                            self.execute(CommandId::OpenProject, ctx);
                        }
                        if ui.button("New…").clicked() {
                            self.execute(CommandId::NewProject, ctx);
                        }
                    });
                });
            });
        egui::Panel::bottom("status-bar")
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(14, 5)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    if self.fixture.is_some() {
                        ui.label(
                            RichText::new("EXAMPLE · READ-ONLY")
                                .font(theme::semibold(theme::CAPTION))
                                .color(theme.muted),
                        );
                    }
                    ui.label(crate::app::muted(&self.status, theme).small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(project) = &self.project {
                            let state = project.state();
                            let problems = state.diagnostics().len();
                            ui.label(
                                RichText::new(format!(
                                    "{problems} problem{}",
                                    if problems == 1 { "" } else { "s" }
                                ))
                                .small()
                                .color(if problems > 0 {
                                    theme.amber
                                } else {
                                    theme.muted
                                }),
                            );
                            ui.label(
                                crate::app::muted(format!("{} locked", state.locks().len()), theme)
                                    .small(),
                            );
                            match &self.saved {
                                Ok(()) => {
                                    ui.label(crate::app::muted("Saved", theme).small());
                                }
                                Err(reason) => {
                                    ui.label(RichText::new("Not saved").small().color(theme.amber))
                                        .on_hover_text(reason);
                                }
                            }
                        }
                        ui.label(
                            crate::app::muted(
                                format!("{} elements on the Surface", self.scene.nodes.len()),
                                theme,
                            )
                            .small(),
                        );
                        ui.label(
                            crate::app::muted(
                                "Drag to pan · Wheel to zoom · Ctrl+K for commands",
                                theme,
                            )
                            .small(),
                        );
                    });
                });
            });
        // Settings take the place of the Surface and the Panels (§3.7).
        if self.settings.open {
            let changed = self.settings.ui(root, theme);
            self.apply_settings(ctx, changed);
            return;
        }
        if self.project.is_none() && self.fixture.is_none() {
            egui::CentralPanel::default().show(root, |ui| self.start_screen(ui, ctx));
            return;
        }
        // The component gallery takes the place of the Outline, the Surface
        // and the Panels.
        if self.fixture.as_deref() == Some("components") {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(theme.canvas)
                        .inner_margin(egui::Margin::same(
                            crate::tokens::space::PANEL_PADDING as i8,
                        )),
                )
                .show(root, |ui| crate::gallery::show(ui, theme));
            return;
        }
        egui::Panel::left("outline")
            .default_size(250.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(root, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    theme.section(ui, "OUTLINE");
                    self.outline(ui);
                    if self.project.is_some() {
                        self.problems_list(ui);
                    }
                });
            });
        if self.project.is_some() && self.conversation.shown {
            self.conversation_column(root);
        }
        egui::Panel::right("panels")
            .default_size(theme::PANEL_WIDTH + 40.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    for (panel, label) in [
                        (Panel::Inspector, "Inspector"),
                        (Panel::Requirements, "Requirements"),
                        (Panel::History, "History"),
                    ] {
                        let tab = ui.add(egui::Button::selectable(self.panel == panel, label));
                        record(ui.ctx(), Target::Button(label), tab.rect);
                        if tab.clicked() {
                            self.panel = panel;
                        }
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| match self.panel {
                    Panel::Inspector => self.inspector_panel(ui),
                    Panel::Requirements => self.requirements_panel(ui),
                    Panel::History => self.history_panel(ui),
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.canvas))
            .show(root, |ui| self.viewport(ui));
    }

    /// The cards on the Surface as an indented list; click to select.
    fn outline(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let mut chosen = None;
        let mut order: Vec<usize> = (0..self.scene.nodes.len()).collect();
        // Parents before children, siblings in model order.
        let position: std::collections::BTreeMap<_, _> = self
            .input
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id, i))
            .collect();
        order.sort_by_key(|i| {
            position
                .get(&self.scene.nodes[*i].id())
                .copied()
                .unwrap_or(usize::MAX)
        });
        for index in order.into_iter().take(400) {
            let node = &self.scene.nodes[index];
            let selected = self.selection.contains(node.id());
            let mut text = format!(
                "{}{}  {}",
                "    ".repeat(node.depth.min(6)),
                node.semantic.keyword,
                node.semantic.name
            );
            match node.semantic.lock {
                LockMark::Own => text.push_str("  · locked"),
                LockMark::Covered => text.push_str("  · locked with owner"),
                LockMark::None => {}
            }
            if node.semantic.problems > 0 {
                text.push_str(&format!("  · {} problem(s)", node.semantic.problems));
            }
            let color = match node.category {
                NodeCategory::Requirement => theme.amber,
                NodeCategory::Package => theme.muted,
                _ => theme.text,
            };
            let response = ui.add(
                egui::Button::selectable(selected, RichText::new(text).color(color))
                    .min_size(Vec2::new(ui.available_width(), 0.0)),
            );
            if response.clicked() {
                chosen = Some(node.id());
            }
        }
        if let Some(id) = chosen {
            let target = SceneTarget::Node(id);
            self.select(target.clone(), false);
            self.frame_target(&target);
        }
    }

    fn start_screen(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // No project opened yet: the first run's welcome (R-46, E1).
        if self.session.recent.is_empty() && self.fixture.is_none() {
            self.welcome(ui, ctx);
            return;
        }
        let theme = self.theme;
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.22);
            ui.label(RichText::new("Agentique").font(theme::semibold(theme::DISPLAY)));
            ui.label(crate::app::muted(
                "Build and change a system's architecture on the Surface.",
                theme,
            ));
            ui.add_space(24.0);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 360.0).max(0.0) / 2.0);
                let new = ui.add_sized([170.0, 38.0], egui::Button::new("New project…"));
                record(ui.ctx(), Target::Button("New project…"), new.rect);
                if new.clicked() {
                    self.execute(CommandId::NewProject, ctx);
                }
                let open = ui.add_sized([170.0, 38.0], egui::Button::new("Open project…"));
                record(ui.ctx(), Target::Button("Open project…"), open.rect);
                if open.clicked() {
                    self.execute(CommandId::OpenProject, ctx);
                }
            });
            if !self.session.recent.is_empty() {
                ui.add_space(24.0);
                theme.section(ui, "RECENT PROJECTS");
                let recent = self.session.recent.clone();
                for folder in recent {
                    if ui
                        .add(egui::Button::new(folder.display().to_string()).frame(false))
                        .clicked()
                    {
                        self.open_project(&folder);
                    }
                }
            }
            ui.add_space(24.0);
            if ui
                .add(
                    egui::Button::new(crate::app::muted("Look at an example (read-only)", theme))
                        .frame(false),
                )
                .clicked()
            {
                self.show_fixture("architecture");
            }
            if !self.status.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new(&self.status).color(theme.amber));
            }
        });
    }

    /// Three steps: what Agentique is, connect a provider (or skip), create
    /// or open a project, with the URL shortener as a sample (R-46, E1).
    fn welcome(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let theme = self.theme;
        let width = (ui.available_width() - 2.0 * theme::SPACE_L).min(560.0);
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.14);
            ui.label(RichText::new("Welcome to Agentique").font(theme::semibold(theme::DISPLAY)));
            ui.add_space(theme::SPACE_L);
            ui.allocate_ui(egui::vec2(width, 0.0), |ui| {
                ui.set_width(width);
                step(
                    ui,
                    theme,
                    "1",
                    "What it is",
                    "Design a system's architecture with an AI Assistant: parts, ports, interfaces and requirements on the Surface, kept as SysML v2 text in git. Every change shows where it happens and can be undone.",
                );
                let key = if self.conversation.key_missing.is_none() {
                    format!("A key is set: the Assistant uses {}.", self.conversation.model_name)
                } else {
                    "No key yet. You can skip this: everything but the Assistant works without one."
                        .to_string()
                };
                step(ui, theme, "2", "Connect a model provider", &key);
                ui.horizontal(|ui| {
                    ui.add_space(theme::SPACE_L * 2.0);
                    let settings = ui.button("Open Settings (Ctrl+,)");
                    record(ui.ctx(), Target::Button("Open Settings"), settings.rect);
                    if settings.clicked() {
                        self.execute(CommandId::Settings, ctx);
                    }
                });
                ui.add_space(theme::SPACE);
                step(
                    ui,
                    theme,
                    "3",
                    "Create or open a project",
                    "A project is a folder with its model in git. Start empty, open one, or start from the URL shortener of the walkthrough.",
                );
                ui.horizontal(|ui| {
                    ui.add_space(theme::SPACE_L * 2.0);
                    let new = ui.add(crate::edit::primary_button(theme, "New project…"));
                    record(ui.ctx(), Target::Button("New project…"), new.rect);
                    if new.clicked() {
                        self.execute(CommandId::NewProject, ctx);
                    }
                    let open = ui.button("Open project…");
                    record(ui.ctx(), Target::Button("Open project…"), open.rect);
                    if open.clicked() {
                        self.execute(CommandId::OpenProject, ctx);
                    }
                    let sample = ui.button("Start from the URL shortener");
                    record(ui.ctx(), Target::Button("Start from the URL shortener"), sample.rect);
                    if sample.clicked() {
                        self.dialog = Some(crate::edit::Dialog::sample_project());
                    }
                });
            });
            if !self.status.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new(&self.status).color(theme.amber));
            }
        });
    }
}

/// One numbered step of the welcome.
fn step(ui: &mut egui::Ui, theme: theme::Theme, number: &str, title: &str, text: &str) {
    ui.add_space(theme::SPACE);
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [theme::SPACE_L * 2.0, 0.0],
            egui::Label::new(
                RichText::new(number)
                    .font(theme::semibold(theme::HEADING))
                    .color(theme.accent),
            ),
        );
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .font(theme::semibold(theme::HEADING))
                    .color(theme.text),
            );
            ui.label(RichText::new(text).color(theme.text_secondary));
        });
    });
    ui.add_space(theme::SPACE_S);
}
