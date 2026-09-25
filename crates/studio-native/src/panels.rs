use crate::{
    app::{ComparisonMode, StudioApp, muted, short_revision},
    commands::{self, CommandId},
    navigation::World,
    theme::{CAPTION, PANEL_WIDTH},
};
use agq_studio_scene::{NodeCategory, SceneTarget};
use eframe::egui::{self, Align, Align2, FontId, Layout, RichText, Stroke, Vec2};

impl StudioApp {
    pub fn shell(&mut self, ctx: &egui::Context) {
        let theme = self.theme;
        egui::TopBottomPanel::top("application_bar")
            .exact_height(62.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(20, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                    let center = rect.center();
                    let painter = ui.painter();
                    painter.line_segment(
                        [
                            center + Vec2::new(-10.0, 9.0),
                            center + Vec2::new(0.0, -10.0),
                        ],
                        Stroke::new(2.4, theme.accent),
                    );
                    painter.line_segment(
                        [
                            center + Vec2::new(0.0, -10.0),
                            center + Vec2::new(10.0, 9.0),
                        ],
                        Stroke::new(2.4, theme.accent),
                    );
                    painter.line_segment(
                        [center + Vec2::new(-5.0, 2.0), center + Vec2::new(6.0, 2.0)],
                        Stroke::new(2.4, theme.accent),
                    );
                    ui.label(RichText::new("AGENTIQUE").size(19.0).strong());
                    ui.add_space(18.0);
                    ui.separator();
                    ui.add_space(12.0);
                    let project_name = self
                        .history
                        .as_ref()
                        .map_or("Agentique", |h| h.project.name.as_str())
                        .to_owned();
                    ui.menu_button(project_name, |ui| {
                        ui.label(muted("PROJECTS", theme).small());
                        for project in self.projects.clone() {
                            if ui.button(&project.name).clicked() {
                                self.open_project(project.id);
                                ui.close();
                            }
                        }
                        ui.separator();
                        ui.label(muted("VISUAL FIXTURES", theme).small());
                        for (name, label) in [
                            ("architecture", "Agentique architecture"),
                            ("ports", "Dense ports and connections"),
                            ("requirements", "Requirement knowledge graph"),
                            ("diff", "Revision comparison"),
                            ("stress1000", "1,000 nodes / 2,000 edges"),
                            ("stress10000", "10,000 nodes / 20,000 edges"),
                        ] {
                            if ui.button(label).clicked() {
                                self.load_fixture(name);
                                ui.close();
                            }
                        }
                        ui.separator();
                        if ui.button("Runtime setup / projects").clicked()
                            && self.allow_context_change()
                        {
                            self.ready = false;
                            ui.close();
                        }
                    });
                    ui.label(muted("/", theme));
                    ui.label(muted("Engineering workspace", theme));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Commands  Ctrl K").clicked() {
                            self.palette = true;
                            self.palette_focus = true;
                        }
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(if self.fixture.is_some() {
                                "VISUAL FIXTURE"
                            } else {
                                "IN-PROCESS"
                            })
                            .size(CAPTION)
                            .color(if self.fixture.is_some() {
                                theme.amber
                            } else {
                                theme.green
                            }),
                        );
                    });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .exact_height(32.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(16, 5)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if !self.pending.is_empty() {
                        ui.spinner();
                    }
                    ui.label(muted(&self.status, theme).small());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            muted(
                                format!(
                                    "{}%  ·  {:?}",
                                    (self.camera.zoom * 100.0) as u32,
                                    self.lod.level()
                                ),
                                theme,
                            )
                            .small(),
                        );
                        ui.separator();
                        ui.label(
                            muted(
                                format!(
                                    "{} / {} elements",
                                    self.timing.visible_nodes,
                                    self.scene.nodes.len()
                                ),
                                theme,
                            )
                            .small(),
                        );
                        ui.separator();
                        ui.label(muted(short_revision(self.scene.revision_id), theme).small());
                    });
                });
            });
        if self.candidate.is_some() {
            egui::TopBottomPanel::bottom("candidate_review")
                .exact_height(96.0)
                .frame(
                    egui::Frame::new()
                        .fill(theme.elevated)
                        .inner_margin(egui::Margin::symmetric(22, 14)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let review = self.context().candidate;
                            ui.label(RichText::new(review.title()).strong().size(14.0));
                            ui.label(muted(review.description(), theme).small());
                        });
                        ui.add_space(24.0);
                        for mode in [
                            ComparisonMode::Current,
                            ComparisonMode::Candidate,
                            ComparisonMode::Diff,
                        ] {
                            if ui
                                .add_enabled(
                                    !self.bridge.mutation_pending(),
                                    egui::Button::selectable(
                                        self.comparison == mode,
                                        format!("{mode:?}"),
                                    ),
                                )
                                .clicked()
                            {
                                self.change_comparison(mode);
                            }
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            for (label, id) in [
                                ("Commit", CommandId::Commit),
                                ("Validate", CommandId::Validate),
                                ("Cancel", CommandId::Cancel),
                            ] {
                                let reason = commands::unavailable(id, &self.context());
                                let response =
                                    ui.add_enabled(reason.is_none(), egui::Button::new(label));
                                if response.clicked() {
                                    self.execute(id, ctx);
                                }
                                if let Some(reason) = reason {
                                    response.on_disabled_hover_text(reason);
                                }
                            }
                        });
                    });
                });
        }
        egui::SidePanel::left("outliner")
            .resizable(true)
            .default_width(238.0)
            .width_range(205.0..=360.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(16, 18)),
            )
            .show(ctx, |ui| self.outliner(ui));
        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(PANEL_WIDTH)
            .width_range(254.0..=420.0)
            .frame(
                egui::Frame::new()
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::symmetric(20, 18)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.inspector_panel(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.canvas))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme.canvas)
                    .inner_margin(egui::Margin::symmetric(22, 15))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (world, name) in [
                                (World::System, "System"),
                                (World::Graph, "Graph"),
                                (World::Requirements, "Requirements"),
                                (World::History, "History"),
                            ] {
                                if ui
                                    .selectable_label(
                                        self.world == world,
                                        RichText::new(name).strong(),
                                    )
                                    .clicked()
                                {
                                    self.switch_world(world);
                                }
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("Fit").on_hover_text("Fit view · Home").clicked() {
                                    self.fit_pending = true;
                                }
                                if ui.button("↑").on_hover_text("Up to owner").clicked() {
                                    self.execute(CommandId::Up, ctx);
                                }
                                if ui.button("→").on_hover_text("Forward").clicked() {
                                    self.execute(CommandId::Forward, ctx);
                                }
                                if ui.button("←").on_hover_text("Back").clicked() {
                                    self.execute(CommandId::Back, ctx);
                                }
                            });
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new(muted("Agentique", theme)).frame(false))
                                .clicked()
                            {
                                self.execute(CommandId::Home, ctx);
                            }
                            if let Some(id) = self.focus
                                && let Some(node) =
                                    self.projection.nodes.iter().find(|n| n.id == id)
                            {
                                ui.label(muted("/", theme));
                                ui.label(&node.name);
                            } else {
                                ui.label(muted("/", theme));
                                ui.label(muted(self.world.title(), theme));
                            }
                            if self.dependencies.is_some() {
                                ui.label(
                                    RichText::new("TEMPORARY VIEW")
                                        .size(10.0)
                                        .color(theme.accent),
                                );
                            }
                            if self.comparison == ComparisonMode::Diff {
                                ui.label(
                                    RichText::new("+ added   − removed   ~ changed")
                                        .size(11.0)
                                        .color(theme.green),
                                );
                            }
                        });
                        if self.world == World::Graph {
                            ui.add_enabled_ui(!self.bridge.mutation_pending(), |ui| {
                                ui.add_space(6.0);
                                ui.horizontal_wrapped(|ui| {
                                    for family in agq_modeling_view::RelationshipFamily::all() {
                                        let mut enabled = self.families.contains(&family);
                                        if ui
                                            .toggle_value(&mut enabled, format!("{family:?}"))
                                            .changed()
                                        {
                                            if enabled {
                                                self.families.insert(family);
                                            } else {
                                                self.families.remove(&family);
                                            }
                                            self.request_projection();
                                        }
                                    }
                                });
                                ui.horizontal_wrapped(|ui| {
                                    for (label, command) in [
                                        ("Incoming", CommandId::ExpandIncoming),
                                        ("Outgoing", CommandId::ExpandOutgoing),
                                        ("Next hop", CommandId::ExpandBoth),
                                        ("One hop", CommandId::CollapseNeighborhood),
                                    ] {
                                        let reason =
                                            commands::unavailable(command, &self.context());
                                        if ui
                                            .add_enabled(reason.is_none(), egui::Button::new(label))
                                            .clicked()
                                        {
                                            self.execute(command, ctx);
                                        }
                                    }
                                    if self.expanded.is_some()
                                        && ui.button("Show loaded view").clicked()
                                    {
                                        self.expanded = None;
                                        self.rebuild();
                                    }
                                });
                                let mut include = self.include_standard;
                                if ui
                                    .checkbox(&mut include, "Expand adjacent standard dependencies")
                                    .changed()
                                {
                                    self.include_standard = include;
                                    self.request_projection();
                                }
                            });
                        }
                    });
                if self.world == World::History {
                    self.history_world(ui);
                } else {
                    self.viewport(ui);
                }
            });
    }
    fn outliner(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        ui.label(
            RichText::new("PROJECT EXPLORER")
                .size(CAPTION)
                .strong()
                .color(theme.muted),
        );
        ui.add_space(14.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Find an element…")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Architecture").strong());
            ui.label(muted(self.projection.nodes.len().to_string(), theme).small());
        });
        ui.add_space(8.0);
        let search = self.search.to_lowercase();
        let nodes: Vec<_> = self
            .outliner_order
            .iter()
            .copied()
            .filter(|index| {
                search.is_empty()
                    || self.scene.nodes[*index]
                        .semantic
                        .name
                        .to_lowercase()
                        .contains(&search)
            })
            .collect();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, 32.0, nodes.len(), |ui, range| {
                for index in range {
                    let node = &self.scene.nodes[nodes[index]];
                    let row = (
                        node.id(),
                        node.semantic.name.clone(),
                        node.depth,
                        node.is_container,
                        node.category,
                        node.collapsed,
                    );
                    let (id, name, depth, container, category, collapsed) = &row;
                    ui.horizontal(|ui| {
                        ui.add_space((*depth as f32 * 12.0).min(48.0));
                        if *container {
                            if ui
                                .add(
                                    egui::Button::new(if *collapsed { "▸" } else { "▾" })
                                        .frame(false)
                                        .small(),
                                )
                                .clicked()
                            {
                                if !self.collapsed.remove(id) {
                                    self.collapsed.insert(*id);
                                }
                                self.rebuild();
                            }
                        } else {
                            ui.label(RichText::new(category_icon(*category)).color(theme.muted));
                        }
                        let response = ui
                            .selectable_label(self.selection.contains(*id), name)
                            .on_hover_text(name);
                        if response.clicked() {
                            self.select(
                                if *container {
                                    SceneTarget::Container(*id)
                                } else {
                                    SceneTarget::Node(*id)
                                },
                                ui.input(|i| i.modifiers.shift),
                            );
                        }
                        if response.double_clicked() {
                            self.execute(CommandId::Focus, ui.ctx());
                        }
                    });
                }
            });
    }
    pub fn setup(&mut self, ctx: &egui::Context) {
        let theme = self.theme;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme.canvas))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(RichText::new("AGENTIQUE").size(38.0).strong());
                    ui.label(RichText::new("Native Studio").size(24.0).color(theme.muted));
                    ui.add_space(34.0);
                    egui::Frame::new()
                        .fill(theme.surface)
                        .corner_radius(12)
                        .inner_margin(32)
                        .show(ui, |ui| {
                            ui.set_max_width(610.0);
                            ui.heading(if self.projects.is_empty() {
                                "Set up your engineering workspace"
                            } else {
                                "Choose a project"
                            });
                            ui.add_space(14.0);
                            ui.label(if !self.pending.is_empty() {
                                "Preparing your engineering workspace…"
                            } else if self.projects.is_empty() {
                                "Install an authenticated Agentique runtime bundle to open real projects."
                            } else {
                                "Your semantic runtime is ready. Select a project to continue."
                            });
                            ui.collapsing("Runtime details", |ui| {
                                ui.label(&self.setup_reason);
                            });
                            ui.add_space(18.0);
                            for project in self.projects.clone() {
                                if ui
                                    .add_sized([500.0, 42.0], egui::Button::new(&project.name))
                                    .clicked()
                                {
                                    self.open_project(project.id);
                                }
                            }
                            if self.projects.is_empty() {
                                ui.label(muted("Accepted KerML v9 + SysML v3 runtime", theme));
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.bundle_path)
                                        .hint_text("Absolute path to accepted-runtime.agq-runtime")
                                        .desired_width(520.0),
                                );
                                if ui
                                    .add_enabled(
                                        self.pending.is_empty() && !self.bundle_path.is_empty(),
                                        egui::Button::new("Authenticate and install runtime"),
                                    )
                                    .clicked()
                                {
                                    match self.bridge.open(
                                        self.config.clone(),
                                        Some(self.bundle_path.clone().into()),
                                    ) {
                                        Ok(id) => {
                                            self.pending.insert(id);
                                            self.setup_reason =
                                                "Authenticating immutable runtime bundle…".into();
                                        }
                                        Err(error) => self.setup_reason = error,
                                    }
                                }
                            }
                            ui.add_space(18.0);
                            ui.separator();
                            ui.add_space(14.0);
                            ui.label(muted(
                                "Explore the native interaction and rendering foundation",
                                theme,
                            ));
                            if ui.button("Open architecture visual fixture").clicked() {
                                self.load_fixture("architecture");
                            }
                            ui.label(
                                RichText::new(
                                    "Fixtures cannot validate or commit semantic model changes.",
                                )
                                .small()
                                .color(theme.amber),
                            );
                            if !self.pending.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Modeling work continues in the background");
                                });
                            }
                        });
                });
            });
    }
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let theme = self.theme;
        if self.palette {
            let mut open = true;
            egui::Window::new("Commands")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_TOP, [0.0, 130.0])
                .fixed_size([590.0, 420.0])
                .show(ctx, |ui| {
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text("Find a command or focus an element…")
                            .desired_width(f32::INFINITY),
                    );
                    if self.palette_focus {
                        input.request_focus();
                        self.palette_focus = false;
                    }
                    crate::automation::record(
                        ctx,
                        crate::automation::Target::PaletteInput,
                        input.rect,
                    );
                    ui.add_space(8.0);
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let results: Vec<_> = commands::search(&self.palette_query).collect();
                        let first_available = results
                            .iter()
                            .find(|command| {
                                commands::unavailable(command.id, &self.context()).is_none()
                            })
                            .map(|command| command.id);
                        for command in results {
                            let reason = commands::unavailable(command.id, &self.context());
                            let label = format!(
                                "{}    {}\n{}",
                                command.label,
                                command.shortcut,
                                reason.unwrap_or(command.description)
                            );
                            let response = ui.add_enabled(
                                reason.is_none(),
                                egui::Button::new(label)
                                    .min_size(Vec2::new(ui.available_width(), 52.0)),
                            );
                            if response.clicked()
                                || (first_available == Some(command.id)
                                    && enter
                                    && reason.is_none())
                            {
                                self.execute(command.id, ctx);
                                self.palette = false;
                            }
                        }
                        if !self.palette_query.trim().is_empty() {
                            let query = self
                                .palette_query
                                .trim_start_matches("Focus ")
                                .to_lowercase();
                            let nodes: Vec<_> = self
                                .projection
                                .nodes
                                .iter()
                                .filter(|n| commands::fuzzy_score(&query, &n.name).is_some())
                                .take(8)
                                .map(|n| (n.id, n.name.clone()))
                                .collect();
                            for (id, name) in nodes {
                                if ui.button(format!("Focus {name}")).clicked() {
                                    self.select(SceneTarget::Node(id), false);
                                    self.execute(CommandId::Focus, ctx);
                                    self.palette = false;
                                }
                            }
                        }
                    });
                });
            self.palette &= open;
        }
        if self.create_dialog {
            let mut open = true;
            egui::Window::new("Create nested part").open(&mut open).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER,Vec2::ZERO).default_width(440.0).show(ctx,|ui|{
                let owner=self.selected_element().and_then(|id|self.projection.nodes.iter().find(|n|n.id==id)).map_or("Selected owner",|n|n.name.as_str());
                ui.label(format!("Inside {owner}"));ui.add_space(10.0);
                ui.label("Part name");let name=ui.add(egui::TextEdit::singleline(&mut self.new_part_name).desired_width(f32::INFINITY));
                crate::automation::record(ctx,crate::automation::Target::CandidateName,name.rect);
                ui.add_space(16.0);ui.label(muted(if self.fixture.is_some(){"This creates a visual preview of typed intent. Semantic reconstruction requires the accepted runtime."}else{"A source-backed Working candidate will be reconstructed. Review and validate it before committing."},theme));
                ui.add_space(16.0);let prepare=ui.button("Prepare candidate");
                crate::automation::record(ctx,crate::automation::Target::CandidatePrepare,prepare.rect);
                if prepare.clicked(){self.prepare_part();}
            });
            self.create_dialog &= open;
        }
        if self.show_explain {
            let mut open = true;
            egui::Window::new("Explain · semantic provenance")
                .open(&mut open)
                .default_size([750.0, 440.0])
                .show(ctx, |ui| self.explain_content(ui));
            self.show_explain = open;
        }
        if self.show_source {
            let mut open = true;
            egui::Window::new("Source · selected revision")
                .open(&mut open)
                .default_size([750.0, 520.0])
                .show(ctx, |ui| {
                    let mut text = if let Some(source) = &self.source {
                        format!(
                            "{} · bytes {}..{}\n\n{}",
                            source.path, source.start, source.end, source.source
                        )
                    } else if self.fixture.is_none() {
                        format!(
                            "Loading source for selected revision {}…",
                            self.selection
                                .primary
                                .as_ref()
                                .and_then(|target| self.scene.target_revision(target))
                                .map(short_revision)
                                .unwrap_or_default()
                        )
                    } else if let Some(candidate) = &self.candidate {
                        candidate.source.clone()
                    } else {
                        "Visual fixtures have no canonical authored source.".into()
                    };
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
                });
            self.show_source = open;
        }
    }
    fn history_world(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(28.0);
            ui.vertical(|ui| {
                ui.heading("Immutable design history");
                ui.label(muted(
                    "Select a revision to change the entire workspace context.",
                    theme,
                ));
            });
        });
        let revisions: Vec<_> = if let Some(history) = &self.history {
            history
                .revisions
                .iter()
                .map(|r| {
                    (
                        r.revision_id,
                        r.metadata
                            .name
                            .clone()
                            .unwrap_or_else(|| "Design revision".into()),
                        format!("{:?}", r.validation)
                            .split('(')
                            .next()
                            .unwrap_or("Working")
                            .to_owned(),
                        r.parent_revision_id,
                        history
                            .branches
                            .iter()
                            .filter(|b| b.head == r.revision_id)
                            .map(|b| b.name.as_str())
                            .collect::<Vec<_>>()
                            .join(" · "),
                    )
                })
                .collect()
        } else {
            let (before, after) = agq_studio_scene::fixtures::revision_diff();
            vec![
                (
                    before.revision_id,
                    "Architecture baseline".into(),
                    "Visual fixture".into(),
                    None,
                    "main".into(),
                ),
                (
                    after.revision_id,
                    "Candidate coordination".into(),
                    "Visual fixture".into(),
                    Some(before.revision_id),
                    "architecture-experiment".into(),
                ),
            ]
        };
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(20.0);
            for (index, (revision, name, status, parent, branches)) in revisions.iter().enumerate()
            {
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 110.0),
                    egui::Sense::click(),
                );
                crate::automation::record(
                    ui.ctx(),
                    crate::automation::Target::HistoryRevision(*revision),
                    rect,
                );
                let selected = *revision == self.projection.revision_id;
                let dot = egui::pos2(rect.left() + 50.0, rect.center().y);
                if let Some(parent_index) =
                    parent.and_then(|parent| revisions.iter().position(|r| r.0 == parent))
                {
                    ui.painter().line_segment(
                        [
                            dot,
                            dot + Vec2::new(
                                0.0,
                                (parent_index as f32 - index as f32)
                                    * (110.0 + ui.spacing().item_spacing.y),
                            ),
                        ],
                        Stroke::new(2.0, theme.border),
                    );
                }
                ui.painter().circle_filled(
                    dot,
                    6.0,
                    if selected { theme.accent } else { theme.muted },
                );
                let card = egui::Rect::from_min_max(
                    rect.min + Vec2::new(78.0, 10.0),
                    rect.max - Vec2::new(28.0, 10.0),
                );
                ui.painter().rect(
                    card,
                    8.0,
                    theme.surface,
                    Stroke::new(
                        if selected { 1.5 } else { 1.0 },
                        if selected { theme.accent } else { theme.border },
                    ),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    card.min + Vec2::new(20.0, 17.0),
                    Align2::LEFT_TOP,
                    name,
                    FontId::proportional(18.0),
                    theme.text,
                );
                ui.painter().text(
                    card.min + Vec2::new(20.0, 49.0),
                    Align2::LEFT_TOP,
                    format!(
                        "{}   ·   {}   ·   {}",
                        short_revision(*revision),
                        status,
                        branches
                    ),
                    FontId::proportional(12.0),
                    theme.muted,
                );
                if response.clicked() {
                    self.select_revision(*revision);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(78.0);
            if ui.button("Compare with parent").clicked() {
                self.switch_world(World::System);
                self.execute(CommandId::Compare, ui.ctx());
            }
        });
    }
}
pub fn category_icon(category: NodeCategory) -> &'static str {
    match category {
        NodeCategory::System => "▣",
        NodeCategory::Part => "□",
        NodeCategory::Port => "◇",
        NodeCategory::Interface => "↔",
        NodeCategory::Requirement => "≡",
        NodeCategory::Action => "▶",
        NodeCategory::State => "○",
        NodeCategory::Agent => "✦",
        NodeCategory::Package => "▤",
        NodeCategory::Feature => "·",
        NodeCategory::Unknown => "?",
    }
}
