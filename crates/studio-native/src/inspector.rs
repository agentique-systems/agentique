//! Intentional engineering sections from public projections; metamodel storage stays private.
use crate::{
    app::{StudioApp, muted, short_revision},
    commands::CommandId,
    theme::{CAPTION, TITLE},
};
use agq_studio_scene::SceneTarget;
use eframe::egui::{self, RichText};

impl StudioApp {
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
        let primary = self.selection.primary.clone();
        let element = self.selected_element();
        let node = element.and_then(|id| self.scene.node(id)).cloned();
        if let Some(inspector) = self.inspector.clone() {
            ui.label(RichText::new(&inspector.element.name).size(TITLE).strong());
            ui.label(muted(&inspector.element.semantic_kind, theme));
            theme.section(ui, "IDENTITY");
            value(
                ui,
                "Revision",
                &short_revision(inspector.revision_id),
                theme,
            );
            value(
                ui,
                "Origin",
                &format!("{:?}", inspector.element.origin),
                theme,
            );
            theme.section(ui, "STRUCTURE");
            if let Some(owner) = inspector.owner {
                value(ui, "Owner", &owner.name, theme);
            }
            value(
                ui,
                "Owned features",
                &inspector.owned_features.len().to_string(),
                theme,
            );
            value(
                ui,
                "Effective features",
                &inspector.effective_features.len().to_string(),
                theme,
            );
            theme.section(ui, "SEMANTICS");
            for feature in inspector
                .effective_types
                .iter()
                .chain(inspector.specializations.iter())
            {
                ui.label(&feature.name);
                ui.label(muted(&feature.semantic_kind, theme).small());
            }
            for query in &inspector.queries {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&query.name);
                    ui.label(muted(&query.completeness, theme).small());
                });
            }
            theme.section(ui, "ENGINEERING");
            value(
                ui,
                "Connections / relationships",
                &inspector.relationships.len().to_string(),
                theme,
            );
            theme.section(ui, "PROVENANCE");
            if let Some(source) = inspector.source {
                ui.label(source.path.unwrap_or_else(|| "Authored document".into()));
            }
            ui.label(muted(inspector.profile, theme).small());
            ui.collapsing("Advanced · evidence and identity", |ui| {
                ui.label(element.map(|id| id.to_string()).unwrap_or_default());
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
            ui.label(muted(&node.semantic.semantic_kind, theme));
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
                theme.section(ui, "FEATURES");
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
                if ui
                    .selectable_label(false, format!("{} → {}", edge.label, target_name))
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
                if let Some(owner) = self.scene.node(port.owner) {
                    value(ui, "Owner", &owner.semantic.name, theme);
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
            ui.label(muted("Illustrative choice distribution", theme).small());
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
