//! Intentional engineering sections from public projections; metamodel storage stays private.
use crate::{
    app::{StudioApp, muted, short_revision},
    commands::CommandId,
    theme::{CAPTION, TITLE},
};
use agq_kernel::ElementId;
use agq_modeling_view::{
    ExplanationNodeKind, ExplanationProjection, FeatureSummary, RelationshipFamily, ViewEdge,
    ViewOrigin,
};
use agq_studio_scene::{NodeCategory, SceneTarget};
use eframe::egui::{self, RichText};

impl StudioApp {
    fn feature_link(&mut self, ui: &mut egui::Ui, feature: &FeatureSummary) {
        let target = if self.lookup.port(&self.scene, feature.id).is_some() {
            Some(SceneTarget::Port(feature.id))
        } else {
            self.scene
                .node(feature.id)
                .map(|_| SceneTarget::Node(feature.id))
        };
        let response = ui.add_enabled(
            target.is_some(),
            egui::Button::new(&feature.name).frame(false),
        );
        crate::real_targets::record(
            ui.ctx(),
            crate::real_targets::Target::InspectorElement(feature.id),
            response.rect,
        );
        let clicked = response
            .on_hover_text(format!(
                "{}\n{}",
                kind_label(&feature.semantic_kind),
                feature.id
            ))
            .clicked();
        if clicked && let Some(target) = target {
            self.select(target, false);
        }
    }

    fn endpoint_name(&self, id: ElementId) -> String {
        self.lookup
            .port(&self.scene, id)
            .map(|port| {
                let owner = self
                    .scene
                    .node(port.owner)
                    .map_or("", |node| node.semantic.name.as_str());
                format!("{owner}.{}", port.name)
            })
            .or_else(|| {
                self.active_projection()
                    .nodes
                    .iter()
                    .find(|node| node.id == id)
                    .map(|node| node.name.clone())
            })
            .unwrap_or_else(|| "Outside this view".into())
    }

    fn relationship_sections(
        &mut self,
        ui: &mut egui::Ui,
        selected: ElementId,
        edges: &[ViewEdge],
    ) {
        let theme = self.theme;
        for (title, families) in [
            ("CONNECTIONS", &[RelationshipFamily::Connection][..]),
            ("REQUIREMENT LINKS", &[RelationshipFamily::Requirement][..]),
            (
                "VERIFICATION LINKS",
                &[RelationshipFamily::Verification][..],
            ),
        ] {
            let matching: Vec<_> = edges
                .iter()
                .filter(|edge| families.contains(&edge.family))
                .collect();
            if matching.is_empty() {
                continue;
            }
            theme.section(ui, title);
            for edge in matching {
                let counterpart = if edge.source == selected
                    || self.lookup.endpoint_owner(edge.source) == selected
                {
                    edge.target
                } else {
                    edge.source
                };
                let label = format!("{} · {}", edge.label, self.endpoint_name(counterpart));
                let visible = self
                    .scene
                    .edges
                    .iter()
                    .any(|item| item.semantic.id == edge.id);
                let response = ui
                    .add_enabled(visible, egui::Button::new(label).frame(false))
                    .on_hover_text(format!("{:?} · {:?}", edge.family, edge.origin));
                crate::real_targets::record(
                    ui.ctx(),
                    crate::real_targets::Target::InspectorRelationship(edge.id.clone()),
                    response.rect,
                );
                if response.clicked() {
                    self.select(SceneTarget::Edge(edge.id.clone()), false);
                }
            }
        }
    }

    pub fn explain_content(&self, ui: &mut egui::Ui) {
        let theme = self.theme;
        if let Some(explanation) = &self.explanation {
            ui.heading("Why this exists");
            ui.label(explanation_summary(explanation));
            ui.add_space(12.0);
            explanation_diagram(ui, explanation, theme);
            ui.label(
                muted(
                    "Arrows show evidence supporting a semantic conclusion.",
                    theme,
                )
                .small(),
            );
            if explanation.truncated {
                ui.label(
                    RichText::new(format!(
                        "Showing a bounded explanation from {} evidence dependencies.",
                        explanation.evidence_count
                    ))
                    .color(theme.amber),
                );
            }
            ui.collapsing("Evidence · exact facts and rule", |ui| {
                for node in &explanation.nodes {
                    ui.label(&node.label);
                    ui.label(muted(format!("{:?} · {}", node.kind, node.id), theme).small());
                }
                for edge in &explanation.edges {
                    ui.label(
                        muted(
                            format!("{} — {} → {}", edge.source, edge.label, edge.target),
                            theme,
                        )
                        .small(),
                    );
                }
            });
            ui.collapsing("Advanced · publication context", |ui| {
                value(ui, "Revision", &explanation.revision_id.to_string(), theme);
                value(ui, "Subject", &explanation.subject_id.to_string(), theme);
                value(ui, "Profile", &explanation.profile, theme);
                if let Some(rule) = explanation.rule_id {
                    value(ui, "Rule", &rule.to_string(), theme);
                }
                value(ui, "Origin", origin_label(explanation.origin), theme);
            });
        } else if self.fixture.is_some() {
            ui.heading("Why this relationship exists");
            ui.label("Authored intent can cause a semantic rule to add a relationship. Real evidence is available after opening an authenticated project.");
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.label("Authored intent");
                    ui.label(muted("Part and type reference", theme).small());
                });
                ui.label("→");
                ui.group(|ui| {
                    ui.label("Semantic rule");
                    ui.label(muted("Applies to this declaration", theme).small());
                });
                ui.label("→");
                ui.group(|ui| {
                    ui.label("Derived relationship");
                    ui.label(muted("Effective model", theme).small());
                });
            });
            ui.add_space(16.0);
            ui.label(
                RichText::new("VISUAL FIXTURE · Illustrative explanation; no canonical proof.")
                    .color(theme.amber),
            );
        } else {
            ui.spinner();
            ui.label("Loading evidence for the selected revision…");
        }
    }

    pub fn inspector_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("INSPECTOR")
                    .size(CAPTION)
                    .strong()
                    .color(theme.muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    muted(format!("{} selected", self.selection.targets.len()), theme).small(),
                );
            });
        });
        ui.add_space(18.0);
        if self.show_agent {
            egui::Frame::new()
                .fill(theme.elevated)
                .inner_margin(10)
                .corner_radius(6)
                .show(ui, |ui| {
                    ui.strong("Dependency view");
                    if let Some(activity) = &self.agent_activity {
                        ui.label(
                            muted(
                                if activity.fixture {
                                    "Built-in query · visual fixture"
                                } else {
                                    "Built-in semantic query agent"
                                },
                                theme,
                            )
                            .small(),
                        );
                        ui.label(format!("Dependencies of {}", activity.root_name));
                        ui.label(
                            muted(
                                format!("Target {}", short_revision(activity.revision)),
                                theme,
                            )
                            .small(),
                        );
                        ui.label(
                            muted(
                                if let Some(error) = &activity.error {
                                    format!("Failed: {error}")
                                } else if activity.complete {
                                    format!("Complete · {} result elements", activity.result_count)
                                } else {
                                    "Running · waiting for semantic projection".into()
                                },
                                theme,
                            )
                            .small(),
                        );
                        ui.collapsing("Query identity", |ui| {
                            ui.label(activity.root.to_string());
                            ui.label(activity.revision.to_string());
                        });
                    } else {
                        ui.label(
                            muted("Restored temporary view · no running agent request", theme)
                                .small(),
                        );
                    }
                    ui.label(muted("Temporary view · read-only authority", theme).small());
                    ui.separator();
                    ui.label("Decision mock suggests Graph");
                    ui.label(
                        muted(
                            "Illustrative weight 0.72 · not calibrated confidence",
                            theme,
                        )
                        .small(),
                    );
                    if ui.button("Dismiss agent view").clicked() {
                        self.show_agent = false;
                        self.dependencies = None;
                        self.expanded = None;
                        self.focus = None;
                        self.request_projection();
                        self.fit_pending = true;
                        self.status = "Agent view dismissed · model unchanged".into();
                    }
                });
            ui.add_space(16.0);
        }
        let primary = self.selection.primary.clone();
        let element = self.selected_element();
        let node = element.and_then(|id| self.scene.node(id)).cloned();
        if let Some(inspector) = self.inspector.clone() {
            ui.label(RichText::new(&inspector.element.name).size(TITLE).strong());
            ui.label(muted(kind_label(&inspector.element.semantic_kind), theme));
            ui.label(muted(origin_label(inspector.element.origin), theme).small());
            if let Some(owner) = &inspector.owner {
                theme.section(ui, "WITHIN");
                self.feature_link(ui, owner);
            }
            if !inspector.effective_types.is_empty() {
                theme.section(
                    ui,
                    if NodeCategory::from_semantic_kind(&inspector.element.semantic_kind)
                        == NodeCategory::Port
                    {
                        "INTERFACE / TYPE"
                    } else {
                        "DEFINED BY"
                    },
                );
                for feature in &inspector.effective_types {
                    self.feature_link(ui, feature);
                }
            }
            for (title, categories) in [
                ("PARTS", &[NodeCategory::Part, NodeCategory::System][..]),
                (
                    "PORTS & INTERFACES",
                    &[NodeCategory::Port, NodeCategory::Interface][..],
                ),
                ("REQUIREMENTS", &[NodeCategory::Requirement][..]),
                ("BEHAVIOR", &[NodeCategory::Action, NodeCategory::State][..]),
            ] {
                let features: Vec<_> = inspector
                    .owned_features
                    .iter()
                    .filter(|feature| {
                        categories
                            .contains(&NodeCategory::from_semantic_kind(&feature.semantic_kind))
                    })
                    .collect();
                if !features.is_empty() {
                    theme.section(ui, title);
                    for feature in features {
                        self.feature_link(ui, feature);
                    }
                }
            }
            self.relationship_sections(ui, inspector.element.id, &inspector.relationships);
            if NodeCategory::from_semantic_kind(&inspector.element.semantic_kind)
                == NodeCategory::Requirement
            {
                ui.label(muted("Links express modeled relationships. They do not establish verification success.", theme).small());
            }
            if NodeCategory::from_semantic_kind(&inspector.element.semantic_kind)
                == NodeCategory::Port
            {
                let direction = self
                    .scene
                    .ports
                    .iter()
                    .find(|port| port.id == inspector.element.id)
                    .map(|port| format!("{:?}", port.direction))
                    .unwrap_or_else(|| "Not provided by this projection".into());
                value(ui, "Direction", &direction, theme);
            }
            ui.collapsing("Effective semantics", |ui| {
                for (label, features) in [
                    ("Effective features", &inspector.effective_features),
                    ("Specializes", &inspector.specializations),
                    ("Subsets", &inspector.subsettings),
                    ("Redefines", &inspector.redefinitions),
                ] {
                    if !features.is_empty() {
                        ui.label(muted(label, theme).small());
                        for feature in features {
                            self.feature_link(ui, feature);
                        }
                    }
                }
                for query in &inspector.queries {
                    ui.label(
                        muted(format!("{}: {}", query.name, query.completeness), theme).small(),
                    );
                }
            });
            if let Some(source) = &inspector.source {
                theme.section(ui, "SOURCE");
                if ui
                    .link(source.path.as_deref().unwrap_or("Authored document"))
                    .clicked()
                {
                    self.execute(CommandId::Source, ui.ctx());
                }
            }
            ui.collapsing("Advanced · evidence and identity", |ui| {
                ui.label(inspector.element.id.to_string());
                ui.label(&inspector.element.semantic_kind);
                value(
                    ui,
                    "Revision",
                    &short_revision(inspector.revision_id),
                    theme,
                );
                value(ui, "Profile", &inspector.profile, theme);
                for query in inspector.queries {
                    ui.label(format!(
                        "{}: {} dependencies · {} searches",
                        query.name, query.positive_dependency_count, query.search_dependency_count
                    ));
                    for diagnostic in query.diagnostics {
                        ui.label(diagnostic);
                    }
                }
            });
        } else if let Some(node) = node {
            ui.label(RichText::new(&node.semantic.name).size(TITLE).strong());
            ui.label(muted(kind_label(&node.semantic.semantic_kind), theme));
            ui.add_space(9.0);
            ui.label(
                RichText::new(if self.fixture.is_some() {
                    "VISUAL FIXTURE"
                } else {
                    "Loading semantic inspection…"
                })
                .size(CAPTION)
                .color(theme.amber),
            );
            theme.section(ui, "STRUCTURE");
            if let Some(owner) = node
                .semantic
                .owner
                .and_then(|id| self.projection.nodes.iter().find(|n| n.id == id))
            {
                value(ui, "Owner", &owner.name, theme);
            }
            value(ui, "Parts", &node.semantic.counts.parts.to_string(), theme);
            value(ui, "Ports", &node.semantic.counts.ports.to_string(), theme);
            if !node.semantic.features.is_empty() {
                theme.section(ui, "PORTS AND FEATURES");
            }
            for feature in &node.semantic.features {
                if ui
                    .selectable_label(
                        self.selection.contains(feature.id),
                        format!("◇  {}", feature.name),
                    )
                    .on_hover_text(&feature.semantic_kind)
                    .clicked()
                {
                    self.select(SceneTarget::Port(feature.id), false);
                }
            }
            theme.section(ui, "RELATIONSHIPS");
            let relationships: Vec<_> = self
                .active_projection()
                .edges
                .iter()
                .filter(|e| {
                    e.source == node.id()
                        || e.target == node.id()
                        || node
                            .semantic
                            .features
                            .iter()
                            .any(|f| f.id == e.source || f.id == e.target)
                })
                .cloned()
                .collect();
            for edge in relationships.iter().take(6) {
                let source_owner = self.lookup.endpoint_owner(edge.source);
                let counterpart = if source_owner == node.id() {
                    edge.target
                } else {
                    edge.source
                };
                let target_name = self
                    .lookup
                    .port(&self.scene, counterpart)
                    .map(|p| {
                        format!(
                            "{}.{}",
                            self.lookup
                                .node(&self.scene, p.owner)
                                .map_or("?", |n| n.semantic.name.as_str()),
                            p.name
                        )
                    })
                    .or_else(|| {
                        self.lookup
                            .node(&self.scene, counterpart)
                            .map(|n| n.semantic.name.clone())
                    })
                    .unwrap_or_else(|| "Outside current view".into());
                let direction = if !edge.directed {
                    "—"
                } else if self.lookup.endpoint_owner(edge.source) == node.id() {
                    "→"
                } else {
                    "←"
                };
                if ui
                    .selectable_label(false, format!("{} {direction} {}", edge.label, target_name))
                    .on_hover_text(format!("{:?} · {:?}", edge.family, edge.origin))
                    .clicked()
                {
                    self.select(SceneTarget::Edge(edge.id.clone()), false);
                }
            }
            theme.section(ui, "PROVENANCE");
            value(ui, "Origin", &format!("{:?}", node.semantic.origin), theme);
            ui.label(
                muted(
                    "Projection identity is preserved across every world.",
                    theme,
                )
                .small(),
            );
            ui.collapsing("Advanced · identity", |ui| {
                ui.label(node.id().to_string());
                ui.label(node.semantic.revision_id.to_string());
            });
        } else if let Some(SceneTarget::Port(id)) = primary {
            if let Some(port) = self.scene.ports.iter().find(|p| p.id == id).cloned() {
                ui.label(RichText::new(&port.name).size(TITLE));
                ui.label(muted("Semantic port", theme));
                theme.section(ui, "INTERFACE");
                value(ui, "Direction", &format!("{:?}", port.direction), theme);
                if let Some(owner) = self
                    .active_projection()
                    .nodes
                    .iter()
                    .find(|node| node.id == port.proxy_for_owner.unwrap_or(port.owner))
                {
                    value(ui, "Owner", &owner.name, theme);
                }
                if port.proxy_for_owner.is_some() {
                    ui.label(muted("Shown on the collapsed subsystem boundary; original port identity retained.", theme).small());
                }
                ui.label(muted("Endpoint order does not establish flow direction.", theme).small());
                ui.collapsing("Identity", |ui| {
                    ui.label(id.to_string());
                    ui.label(port.revision_id.to_string());
                });
            }
        } else if let Some(SceneTarget::Edge(id)) = primary {
            if let Some(edge) = self
                .scene
                .edges
                .iter()
                .find(|e| e.semantic.id == id)
                .cloned()
            {
                ui.label(RichText::new(&edge.semantic.label).size(TITLE));
                ui.label(muted(format!("{:?}", edge.semantic.family), theme));
                theme.section(ui, "RELATIONSHIP");
                value(ui, "Origin", &format!("{:?}", edge.semantic.origin), theme);
                value(
                    ui,
                    "Directed",
                    if edge.semantic.directed {
                        "Yes"
                    } else {
                        "Not asserted"
                    },
                    theme,
                );
                value(ui, "Routing", &format!("{:?}", edge.quality), theme);
                ui.label(muted(if self.fixture.is_some(){"Illustrative relationship. Real evidence requires the authenticated project."}else{"Loading canonical relationship…"},theme));
                ui.collapsing("Identity", |ui| {
                    ui.label(edge.semantic.id);
                    if let Some(id) = edge.semantic.relationship_id {
                        ui.label(id.to_string());
                    }
                });
            }
        } else {
            ui.label(RichText::new("Everything has context.").size(20.0));
            ui.add_space(12.0);
            ui.label(muted(
                "Select a system, part, port, or relationship to inspect its engineering meaning.",
                theme,
            ));
            theme.section(ui, "NAVIGATION");
            ui.label(muted("Click to select\nShift-click to add\nDrag canvas to pan\nWheel to zoom\nF to focus\nCtrl+K for commands",theme));
        }
        if self.selection.primary.is_some() {
            theme.section(ui, "ACTIONS");
            for (label, id) in [
                ("Focus selection", CommandId::Focus),
                ("Explain", CommandId::Explain),
                ("Show dependencies", CommandId::Dependencies),
            ] {
                if ui.button(label).clicked() {
                    self.execute(id, ui.ctx());
                }
            }
        }
        if self.show_agent {
            let presentation = crate::agents::dependency_view(
                self.scene.revision_id,
                &self.dependencies.clone().unwrap_or_default(),
            );
            theme.section(ui, "DECISION AGENT");
            ui.label("Suggested lens: Graph World");
            ui.label(
                muted(
                    format!("Provider: {}", presentation.decision.provider),
                    theme,
                )
                .small(),
            );
            ui.label(muted("Illustrative weights · not calibrated probabilities", theme).small());
            for (name, score) in &presentation.decision.answers[0].probabilities {
                ui.add(
                    egui::ProgressBar::new(*score as f32)
                        .text(format!("{name}   {score:.2}"))
                        .fill(theme.accent),
                );
            }
            ui.label(
                muted(
                    format!(
                        "{} · {} elements",
                        presentation.overlay.label,
                        presentation.observation.selected_elements.len()
                    ),
                    theme,
                )
                .small(),
            );
            ui.label(muted("Presentation suggestion · no model mutation", theme).small());
        }
    }
}
fn value(ui: &mut egui::Ui, key: &str, value: &str, theme: crate::theme::Theme) {
    ui.horizontal_wrapped(|ui| {
        ui.label(muted(key, theme));
        ui.label(value);
    });
}

fn origin_label(origin: ViewOrigin) -> &'static str {
    match origin {
        ViewOrigin::Authored => "Authored in this project",
        ViewOrigin::Derived => "Derived from model semantics",
        ViewOrigin::Standard => "From the accepted standard library",
        ViewOrigin::Generated => "Generated model element",
    }
}

fn explanation_summary(explanation: &ExplanationProjection) -> String {
    match explanation.origin {
        ViewOrigin::Derived => format!("The model includes this fact because a semantic rule is supported by {} recorded evidence dependencies. The diagram traces that immediate support.", explanation.evidence_count),
        ViewOrigin::Authored => "This fact comes directly from the project's authored model. No semantic producer is asserted for this fact.".into(),
        ViewOrigin::Standard => "This fact belongs to the accepted standard library used by this revision.".into(),
        ViewOrigin::Generated => "This is a generated model fact. Inspect its exact identity and publication context below.".into(),
    }
}

/// The topology is taken only from the proof projection. A display arrow is
/// never inferred from node ordering or used as a modeled system connection.
fn explanation_diagram(
    ui: &mut egui::Ui,
    explanation: &ExplanationProjection,
    theme: crate::theme::Theme,
) {
    use egui::{Align2, FontId, Pos2, Rect, Sense, Stroke, Vec2};
    use std::collections::BTreeMap;
    let mut columns = [Vec::new(), Vec::new(), Vec::new()];
    for node in &explanation.nodes {
        let column = if node.kind == ExplanationNodeKind::Rule {
            1
        } else if explanation.edges.iter().any(|edge| {
            edge.target == node.id
                && explanation.nodes.iter().any(|source| {
                    source.id == edge.source && source.kind == ExplanationNodeKind::Rule
                })
        }) {
            2
        } else {
            0
        };
        columns[column].push(node);
    }
    let rows = columns.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let height = rows as f32 * 90.0 + 30.0;
    egui::ScrollArea::both()
        .id_salt("explanation-diagram")
        .max_height(350.0)
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(700.0, height), Sense::hover());
            let mut positions = BTreeMap::new();
            for (column, nodes) in columns.iter().enumerate() {
                for (row, node) in nodes.iter().enumerate() {
                    let y = if column == 0 {
                        row as f32 * 90.0
                    } else {
                        (height - 100.0) * 0.5
                    };
                    positions.insert(
                        node.id.as_str(),
                        Rect::from_min_size(
                            rect.min + Vec2::new(column as f32 * 246.0, y + 20.0),
                            Vec2::new(208.0, 72.0),
                        ),
                    );
                }
            }
            for edge in &explanation.edges {
                if let (Some(source), Some(target)) = (
                    positions.get(edge.source.as_str()),
                    positions.get(edge.target.as_str()),
                ) {
                    let start = source.right_center();
                    let end = target.left_center();
                    let middle = (start.x + end.x) * 0.5;
                    ui.painter().add(egui::Shape::line(
                        vec![
                            start,
                            Pos2::new(middle, start.y),
                            Pos2::new(middle, end.y),
                            end,
                        ],
                        Stroke::new(1.4, theme.muted),
                    ));
                    ui.painter().arrow(
                        end - Vec2::new(8.0, 0.0),
                        Vec2::new(8.0, 0.0),
                        Stroke::new(1.4, theme.muted),
                    );
                }
            }
            for node in &explanation.nodes {
                let Some(card) = positions.get(node.id.as_str()).copied() else {
                    continue;
                };
                let rule = node.kind == ExplanationNodeKind::Rule;
                ui.painter().rect(
                    card,
                    7.0,
                    if rule { theme.elevated } else { theme.surface },
                    Stroke::new(1.0, if rule { theme.accent } else { theme.border }),
                    egui::StrokeKind::Inside,
                );
                let heading = if rule {
                    "SEMANTIC RULE"
                } else if columns[2].iter().any(|outcome| outcome.id == node.id) {
                    "CONSEQUENCE"
                } else {
                    "EVIDENCE"
                };
                ui.painter().text(
                    card.min + Vec2::new(10.0, 10.0),
                    Align2::LEFT_TOP,
                    heading,
                    FontId::proportional(10.0),
                    theme.muted,
                );
                let mut job = egui::text::LayoutJob::simple(
                    node.label.clone(),
                    FontId::proportional(12.0),
                    theme.text,
                    card.width() - 20.0,
                );
                job.wrap.max_rows = 2;
                job.wrap.break_anywhere = true;
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                ui.painter()
                    .galley(card.min + Vec2::new(10.0, 28.0), galley, theme.text);
                let response = ui.interact(card, ui.id().with(&node.id), Sense::hover());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Label,
                        true,
                        format!("{heading}: {}", node.label),
                    )
                });
                response.on_hover_text(format!("{}\n{}", node.label, node.id));
            }
        });
}

/// Presentation copy for known public projection kinds. Unknown kinds retain
/// their exact name; this mapping never determines semantic behavior.
fn kind_label(kind: &str) -> &str {
    match kind {
        "PartDefinition" => "Part definition",
        "PartUsage" => "Part",
        "PortDefinition" => "Port definition",
        "PortUsage" => "Port",
        "InterfaceDefinition" => "Interface definition",
        "InterfaceUsage" => "Interface",
        "ConnectionDefinition" => "Connection definition",
        "ConnectionUsage" => "Connection",
        "RequirementDefinition" => "Requirement definition",
        "RequirementUsage" => "Requirement",
        "ActionDefinition" => "Action definition",
        "ActionUsage" => "Action",
        "StateDefinition" => "State definition",
        "StateUsage" => "State",
        "AttributeDefinition" => "Attribute definition",
        "AttributeUsage" => "Attribute usage",
        _ => kind,
    }
}
