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
                        if ui
                            .add_enabled(
                                self.authenticated_runtime_ready(),
                                egui::Button::new("New project…"),
                            )
                            .clicked()
                        {
                            self.new_project_dialog();
                            ui.close();
                        }
                        if ui
                            .add_enabled(
                                self.fixture.is_none() && self.binding.is_some(),
                                egui::Button::new("Add source document…"),
                            )
                            .clicked()
                            && self.allow_context_change()
                        {
                            self.project_dialog.import_open = true;
                            ui.close();
                        }
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
                            ("typography", "Long engineering names and Unicode"),
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
                        if self.fixture.is_some() {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("VISUAL FIXTURE")
                                    .size(CAPTION)
                                    .color(theme.amber),
                            );
                        }
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
                    if !self.pending.is_empty() || self.scene_builder.busy {
                        ui.spinner();
                    }
                    if self.scene_builder.busy {
                        ui.label(
                            muted("Updating view · previous view remains available", theme).small(),
                        );
                    }
                    ui.label(muted(&self.status, theme).small());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            muted(format!("{}%", (self.camera.zoom * 100.0) as u32), theme).small(),
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
        if let Some(preparation) = &mut self.preparation {
            egui::TopBottomPanel::bottom("candidate_preparation")
                .frame(egui::Frame::new().fill(theme.elevated).inner_margin(16))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.vertical(|ui| {
                            ui.strong(if preparation.cancelled { "CANCELLATION REQUESTED" } else { "PREPARING WORKING CANDIDATE" });
                            ui.label(&preparation.intent);
                            ui.label(if preparation.cancelled {
                                "Current reconstruction will finish safely; its result will be discarded."
                            } else {
                                "Source edit and semantic reconstruction running · validation has not started"
                            });
                            ui.label(muted(format!("Elapsed {:.1} s · current revision remains available", preparation.started.elapsed().as_secs_f32()), theme).small());
                        });
                        let cancel = ui.add_enabled(!preparation.cancelled, egui::Button::new("Cancel preparation"));
                        crate::automation::record(ui.ctx(), crate::automation::Target::CancelPreparation, cancel.rect);
                        if cancel.clicked() {
                            preparation.cancelled = true;
                        }
                    });
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(target) = self
            .revision_retry
            .filter(|target| self.project_id() == Some(target.project))
        {
            egui::TopBottomPanel::bottom("revision_view_retry")
                .frame(egui::Frame::new().fill(theme.elevated).inner_margin(12))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let committed =
                            self.committed_receipt
                                .as_ref()
                                .is_some_and(|(project, receipt)| {
                                    *project == target.project
                                        && receipt.revision_id == target.revision
                                });
                        ui.label(if committed {
                            "Revision committed durably; its view is unavailable."
                        } else {
                            "Requested revision view is unavailable; current revision retained."
                        });
                        if ui
                            .add_enabled(
                                self.pending_revision.is_none() && !self.bridge.mutation_pending(),
                                egui::Button::new("Retry revision view"),
                            )
                            .clicked()
                        {
                            self.retry_revision_view();
                        }
                    });
                });
        }
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
                            if self.lifecycle_unknown {
                                ui.label(
                                    RichText::new("CANDIDATE STATE NEEDS REFRESH")
                                        .strong()
                                        .size(14.0),
                                );
                                ui.label(
                                    muted("Model actions wait for the authoritative state", theme)
                                        .small(),
                                );
                                if ui
                                    .add_enabled(
                                        self.lifecycle_request == 0,
                                        egui::Button::new("Refresh candidate state"),
                                    )
                                    .clicked()
                                {
                                    self.reconcile_candidate_lifecycle();
                                }
                            } else {
                                ui.label(RichText::new(review.title()).strong().size(14.0));
                                ui.label(muted(review.description(), theme).small());
                            }
                        });
                        ui.add_space(24.0);
                        for mode in [
                            ComparisonMode::Current,
                            ComparisonMode::Candidate,
                            ComparisonMode::Diff,
                        ] {
                            let response = ui.add_enabled(
                                !self.bridge.mutation_pending(),
                                egui::Button::selectable(
                                    self.comparison == mode,
                                    format!("{mode:?}"),
                                ),
                            );
                            crate::real_targets::record(
                                ctx,
                                match mode {
                                    ComparisonMode::Current => {
                                        crate::real_targets::Target::ComparisonCurrent
                                    }
                                    ComparisonMode::Candidate => {
                                        crate::real_targets::Target::ComparisonCandidate
                                    }
                                    ComparisonMode::Diff => {
                                        crate::real_targets::Target::ComparisonDiff
                                    }
                                },
                                response.rect,
                            );
                            if response.clicked() {
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
                    if let Some(candidate) = &self.candidate {
                        let actor = if candidate.actor == "human-operator" {
                            "Human operator"
                        } else {
                            candidate.actor.as_str()
                        };
                        let summary = format!("{} · Proposed by {actor}", candidate.intent);
                        ui.add(egui::Label::new(&summary).truncate())
                            .on_hover_text(summary);
                    }
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
                            self.local_views_menu(ui);
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
                            if self.focus.is_some() {
                                self.breadcrumb_navigation(ui);
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
                                let before = self
                                    .candidate
                                    .as_ref()
                                    .map(|c| c.before.revision_id)
                                    .or_else(|| {
                                        self.compare_before.as_ref().map(|p| p.revision_id)
                                    });
                                if let Some(before) = before {
                                    ui.label(
                                        muted(
                                            format!(
                                                "{} → {}",
                                                short_revision(before),
                                                short_revision(self.scene.revision_id)
                                            ),
                                            theme,
                                        )
                                        .small(),
                                    );
                                }
                                let count = |mark| {
                                    self.scene.nodes.iter().filter(|n| n.diff == mark).count()
                                };
                                ui.label(
                                    RichText::new(format!(
                                        "+ {} added   − {} removed   ~ {} changed",
                                        count(agq_studio_scene::DiffMark::Added),
                                        count(agq_studio_scene::DiffMark::Removed),
                                        count(agq_studio_scene::DiffMark::Changed)
                                    ))
                                    .size(11.0)
                                    .color(theme.green),
                                );
                                if ui.small_button("Focus changes").clicked() {
                                    self.execute(CommandId::FocusChanges, ctx);
                                }
                            }
                        });
                        if self.comparison == ComparisonMode::Diff {
                            self.diff_review_panel(ui);
                        }
                        if self.world == World::System && self.focus.is_some() {
                            let visible: std::collections::BTreeSet<_> = self
                                .scene
                                .nodes
                                .iter()
                                .flat_map(|node| {
                                    std::iter::once(node.id()).chain(
                                        node.semantic.features.iter().map(|feature| feature.id),
                                    )
                                })
                                .collect();
                            let external = self
                                .active_projection()
                                .edges
                                .iter()
                                .filter(|edge| {
                                    edge.family == agq_modeling_view::RelationshipFamily::Connection
                                        && (visible.contains(&edge.source)
                                            != visible.contains(&edge.target))
                                })
                                .count();
                            if external > 0 {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        muted(
                                            format!(
                                                "{external} connections continue outside this focus"
                                            ),
                                            theme,
                                        )
                                        .small(),
                                    );
                                    if ui.small_button("Explore connection context").clicked() {
                                        if let Some(focus) = self.focus {
                                            self.select(SceneTarget::Node(focus), false);
                                        }
                                        self.switch_world(World::Graph);
                                    }
                                });
                            }
                        }
                        if self.world == World::Requirements {
                            self.requirements_summary(ui);
                        }
                        if self.world == World::Graph {
                            ui.add_enabled_ui(!self.bridge.mutation_pending(), |ui| {
                                ui.add_space(6.0);
                                if self.comparison == ComparisonMode::Diff {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.menu_button("Graph filters", |ui| {
                                            ui.set_max_width(560.0);
                                            ui.label(
                                                muted("Filters apply to both revisions.", theme)
                                                    .small(),
                                            );
                                            self.graph_family_controls(ui);
                                            self.graph_standard_control(ui);
                                        });
                                        ui.label(muted(format!(
                                            "{} relationship families · standard expansion {}",
                                            self.families.len(),
                                            if self.include_standard { "on" } else { "off" },
                                        ), theme).small());
                                    });
                                } else {
                                    self.graph_family_controls(ui);
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
                                                .add_enabled(
                                                    reason.is_none(),
                                                    egui::Button::new(label),
                                                )
                                                .clicked()
                                            {
                                                self.execute(command, ctx);
                                            }
                                        }
                                        if self.comparison != ComparisonMode::Diff
                                            && self.expanded.is_some()
                                            && ui.button("Show loaded view").clicked()
                                        {
                                            self.expanded = None;
                                            self.rebuild();
                                        }
                                    });
                                    self.graph_standard_control(ui);
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
    fn graph_family_controls(&mut self, ui: &mut egui::Ui) {
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
    }

    fn graph_standard_control(&mut self, ui: &mut egui::Ui) {
        let mut include = self.include_standard;
        let standards = ui.checkbox(&mut include, "Expand adjacent standard dependencies");
        crate::real_targets::record(
            ui.ctx(),
            crate::real_targets::Target::Standards,
            standards.rect,
        );
        if standards.changed() {
            self.include_standard = include;
            self.request_projection();
        }
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
        let search_label = ui.label(muted("Find an element", theme).small());
        let search_input = ui
            .add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Find an element…")
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(search_label.id);
        crate::real_targets::record(
            ui.ctx(),
            crate::real_targets::Target::ExplorerSearch,
            search_input.rect,
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(if self.world == World::Requirements {
                    "Requirement neighborhood"
                } else {
                    "Architecture"
                })
                .strong(),
            );
            ui.label(muted(self.projection.nodes.len().to_string(), theme).small());
        });
        ui.add_space(8.0);
        let nodes = self.filtered_outliner();
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
                            let disclosure = ui.add(
                                egui::Button::new(if *collapsed { "▸" } else { "▾" })
                                    .frame(false)
                                    .small(),
                            );
                            disclosure.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    format!(
                                        "{} {name}",
                                        if *collapsed { "Expand" } else { "Collapse" }
                                    ),
                                )
                            });
                            if disclosure.clicked() {
                                if !self.collapsed.remove(id) {
                                    self.collapsed.insert(*id);
                                }
                                self.rebuild();
                            }
                        } else {
                            ui.label(RichText::new(category_icon(*category)).color(theme.muted));
                        }
                        let response = ui
                            .add_sized(
                                [ui.available_width().max(1.0), 28.0],
                                egui::Button::selectable(self.selection.contains(*id), name)
                                    .truncate(),
                            )
                            .on_hover_text(name);
                        crate::real_targets::record(
                            ui.ctx(),
                            crate::real_targets::Target::ExplorerElement(*id),
                            response.rect,
                        );
                        let target = if *container {
                            SceneTarget::Container(*id)
                        } else {
                            SceneTarget::Node(*id)
                        };
                        let mut same_object_click = false;
                        if response.clicked() {
                            // Filtering can place a different canonical object
                            // under the same toolkit row between two clicks.
                            same_object_click = self.explorer_clicks.click(
                                self.generation,
                                Some(&target),
                                response
                                    .interact_pointer_pos()
                                    .map_or([0.0, 0.0], |p| [p.x, p.y]),
                                ui.input(|i| i.time),
                            );
                            self.select(target, ui.input(|i| i.modifiers.shift));
                        }
                        if response.double_clicked() && same_object_click {
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
                            let opening = self.opening.as_ref().filter(|opening| opening.running());
                            let failed = self.opening.as_ref().is_some_and(|opening| opening.failed);
                            ui.heading(if opening.is_some() {
                                "Opening your workspace"
                            } else if failed {
                                "Workspace could not open"
                            } else if !self.authenticated_runtime_ready() {
                                "Set up your engineering workspace"
                            } else if self.projects.is_empty() {
                                "Create your first project"
                            } else {
                                "Choose a project"
                            });
                            ui.add_space(14.0);
                            if let Some(opening) = opening {
                                ui.horizontal(|ui| {
                                    if !self.reduced_motion { ui.spinner(); }
                                    ui.label(RichText::new(opening.title).strong());
                                });
                                ui.label(opening.detail);
                                ui.add_space(8.0);
                                ui.label(muted(opening.elapsed_text(), theme));
                                ctx.request_repaint_after(std::time::Duration::from_secs(1));
                            } else {
                                ui.label(if failed {
                                    "Opening stopped before the workspace was ready. See the details below."
                                } else if !self.pending.is_empty() {
                                    "Opening the selected project revision…"
                                } else if !self.authenticated_runtime_ready() {
                                    "Choose a local Agentique runtime bundle to authenticate and install."
                                } else if self.projects.is_empty() {
                                    "Your semantic runtime is ready. Create an empty Working project to begin."
                                } else {
                                    "Your semantic runtime is ready. Select a project to continue."
                                });
                                if failed && let Some(opening) = &self.opening {
                                    ui.label(muted(format!("Stopped while: {}", opening.title), theme));
                                    ui.label(muted(opening.elapsed_text(), theme));
                                }
                            }
                            egui::CollapsingHeader::new("Opening details").open(failed.then_some(true)).show(ui, |ui| {
                                ui.label(&self.setup_reason);
                            });
                            ui.add_space(18.0);
                            for project in self.projects.clone() {
                                let project_button = ui.add_sized([500.0, 42.0], egui::Button::new(&project.name));
                                crate::real_targets::record(ctx, crate::real_targets::Target::Project(project.id), project_button.rect);
                                if project_button.clicked() {
                                    self.open_project(project.id);
                                }
                            }
                            if self.authenticated_runtime_ready() && ui.button("New project…").clicked() {
                                self.new_project_dialog();
                            }
                            if !self.authenticated_runtime_ready() && !self.opening.as_ref().is_some_and(|opening| opening.running()) {
                                ui.label(muted("Required runtime: KerML v9 + SysML v3", theme));
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
                                            self.requested_definition = None;
                                            self.deferred_definition = None;
                                            self.focus_changes_pending = false;
                                            self.pending.insert(id);
                                            self.opening = Some(crate::loading::OpeningProgress::new(id));
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
                                "Explore a sample architecture",
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
                            if !self.pending.is_empty() && !self.opening.as_ref().is_some_and(|opening| opening.running()) {
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
        self.project_dialogs(ctx);
        if self.palette {
            self.command_palette(ctx);
        }
        self.part_edit_dialog(ctx);
        if self.show_explain {
            let mut open = true;
            if let Some(window) = egui::Window::new("Explain · semantic provenance")
                .open(&mut open)
                .default_pos(egui::pos2(420.0, 240.0))
                .default_size([750.0, 440.0])
                .show(ctx, |ui| self.explain_content(ui))
            {
                crate::automation::record(
                    ctx,
                    crate::automation::Target::ExplainWindow,
                    window.response.rect,
                );
            }
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
                ui.heading("Design history");
                ui.label(muted(
                    "Explore a revision, compare its design changes, then return to the branch head.",
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
                        revision_title(r, &history.revisions),
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
                        r.metadata.created.clone(),
                        r.metadata.description.clone().unwrap_or_default(),
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
                    String::new(),
                    "Authored architecture before candidate coordination".into(),
                ),
                (
                    after.revision_id,
                    "Candidate coordination".into(),
                    "Visual fixture".into(),
                    Some(before.revision_id),
                    "architecture-experiment".into(),
                    String::new(),
                    "Add CandidateCoordinator within ModelingPlatform".into(),
                ),
            ]
        };
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.add_space(28.0);
            let base_key = ui
                .id()
                .with(("history-comparison-base", self.binding.map(|b| b.project)));
            let mut base = ui
                .ctx()
                .data(|data| data.get_temp::<agq_modeling_workspace::ProjectRevisionId>(base_key))
                .or_else(|| {
                    revisions
                        .iter()
                        .find(|r| r.0 == self.projection.revision_id)
                        .and_then(|r| r.3)
                })
                .or_else(|| revisions.first().map(|r| r.0));
            egui::ComboBox::from_id_salt(base_key)
                .width(230.0)
                .selected_text(
                    base.and_then(|id| revisions.iter().find(|r| r.0 == id))
                        .map_or("Choose comparison base", |r| r.1.as_str()),
                )
                .show_ui(ui, |ui| {
                    for revision in &revisions {
                        ui.selectable_value(
                            &mut base,
                            Some(revision.0),
                            format!("{} · {}", revision.1, short_revision(revision.0)),
                        );
                    }
                });
            if let Some(base) = base {
                ui.ctx().data_mut(|data| data.insert_temp(base_key, base));
                if ui
                    .add_enabled(
                        base != self.projection.revision_id
                            && self.binding.is_some()
                            && self.candidate.is_none(),
                        egui::Button::new("Compare with selected revision"),
                    )
                    .clicked()
                {
                    self.compare_selected_revision(base);
                }
            }
            if ui.button("Compare parent").clicked() {
                self.switch_world(World::System);
                self.execute(CommandId::Compare, ui.ctx());
            }
            let head = self
                .history
                .as_ref()
                .and_then(|history| {
                    history
                        .branches
                        .iter()
                        .find(|branch| branch.id == history.project.default_branch)
                })
                .map(|branch| (branch.head, branch.name.clone()));
            if let Some((head, branch)) = head
                && ui
                    .add_enabled(
                        head != self.projection.revision_id,
                        egui::Button::new(format!("Return to {branch} head")),
                    )
                    .clicked()
            {
                self.return_to_revision(head);
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(20.0);
            for (index, (revision, name, status, parent, branches, created, description)) in
                revisions.iter().enumerate()
            {
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 132.0),
                    egui::Sense::click(),
                );
                crate::automation::record(
                    ui.ctx(),
                    crate::automation::Target::HistoryRevision(*revision),
                    rect,
                );
                let selected = *revision == self.projection.revision_id;
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        true,
                        selected,
                        format!("{name}, {status}, revision {}", short_revision(*revision)),
                    )
                });
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
                                    * (132.0 + ui.spacing().item_spacing.y),
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
                        if selected || response.has_focus() {
                            1.5
                        } else {
                            1.0
                        },
                        if selected || response.has_focus() {
                            theme.accent
                        } else {
                            theme.border
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                let mut title = egui::text::LayoutJob::simple_singleline(
                    name.clone(),
                    FontId::proportional(18.0),
                    theme.text,
                );
                title.wrap.max_width = (card.width() - 40.0).max(1.0);
                title.wrap.max_rows = 1;
                title.wrap.break_anywhere = true;
                ui.painter().galley(
                    card.min + Vec2::new(20.0, 17.0),
                    ui.painter().layout_job(title),
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
                let detail = if description.is_empty() {
                    created.clone()
                } else if created.is_empty() {
                    description.clone()
                } else {
                    format!("{created} · {description}")
                };
                let mut detail_job = egui::text::LayoutJob::simple_singleline(
                    detail,
                    FontId::proportional(11.0),
                    theme.muted,
                );
                detail_job.wrap.max_width = (card.width() - 40.0).max(1.0);
                detail_job.wrap.max_rows = 1;
                detail_job.wrap.break_anywhere = true;
                ui.painter().galley(
                    card.min + Vec2::new(20.0, 76.0),
                    ui.painter().layout_job(detail_job),
                    theme.muted,
                );
                if response.clicked() {
                    self.select_revision(*revision);
                }
                response.on_hover_text(format!(
                    "{name}\n{description}\nCreated {created}\nRevision {revision}"
                ));
            }
        });
    }
}

/// Use recorded edit intent where available. Generic imports fall back to exact
/// source differences, explicitly named as source changes rather than invented
/// semantic summaries. No semantic revision reconstruction is needed for cards.
fn revision_title(
    revision: &agq_modeling_repository::RevisionManifest,
    revisions: &[agq_modeling_repository::RevisionManifest],
) -> String {
    if let Some(name) = revision
        .metadata
        .name
        .as_ref()
        .filter(|name| !name.trim().is_empty())
    {
        return name.clone();
    }
    let parent = revision
        .parent_revision_id
        .and_then(|id| revisions.iter().find(|r| r.revision_id == id));
    let changed: Vec<_> = revision
        .documents
        .iter()
        .filter(|document| {
            !parent.is_some_and(|parent| {
                parent.documents.iter().any(|old| {
                    old.path == document.path && old.content_digest == document.content_digest
                })
            })
        })
        .map(|document| document.path.as_str())
        .collect();
    let removed: Vec<_> = parent
        .into_iter()
        .flat_map(|parent| &parent.documents)
        .filter(|document| {
            !revision
                .documents
                .iter()
                .any(|current| current.path == document.path)
        })
        .map(|document| document.path.as_str())
        .collect();
    if !removed.is_empty() {
        return if removed.len() == 1 && changed.is_empty() {
            format!("Removed source {}", removed[0])
        } else {
            format!(
                "Updated {} · removed {} source documents",
                changed.len(),
                removed.len()
            )
        };
    }
    if changed.is_empty() {
        if revision.documents.is_empty() {
            "Empty working project".into()
        } else {
            "Design checkpoint".into()
        }
    } else if changed.len() == 1 {
        format!(
            "{} {}",
            if parent.is_some() {
                "Updated source"
            } else {
                "Imported"
            },
            changed[0]
        )
    } else {
        format!(
            "{} {} source documents",
            if parent.is_some() {
                "Updated"
            } else {
                "Imported"
            },
            changed.len()
        )
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
