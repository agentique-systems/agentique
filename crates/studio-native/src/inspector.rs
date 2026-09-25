//! Intentional engineering sections from public projections; metamodel storage stays private.
use crate::{
    app::{StudioApp, muted, short_revision},
    commands::CommandId,
    theme::{CAPTION, TITLE},
};
use agq_kernel::ElementId;
use agq_modeling_view::{
    ExplanationNodeKind, ExplanationProjection, FeatureProvenance, FeatureSummary,
    RelationshipFamily, ViewEdge, ViewOrigin,
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

    fn inherited_feature_link(&mut self, ui: &mut egui::Ui, feature: &FeatureSummary) {
        let theme = self.theme;
        ui.horizontal_wrapped(|ui| {
            self.feature_link(ui, feature);
            ui.label(muted("Inherited", theme).small()).on_hover_text(
                "The effective feature retains its original identity and declaring owner.",
            );
        });
        // Ownership comes from the canonical projection, never the part whose
        // effective features happen to include this port or interface.
        let owner = self
            .active_projection()
            .nodes
            .iter()
            .find(|node| node.id == feature.id)
            .and_then(|node| node.owner)
            .and_then(|id| {
                self.active_projection()
                    .nodes
                    .iter()
                    .find(|node| node.id == id)
            })
            .map(|node| FeatureSummary {
                id: node.id,
                name: node.name.clone(),
                semantic_kind: node.semantic_kind.clone(),
            });
        if let Some(owner) = owner {
            ui.horizontal_wrapped(|ui| {
                ui.label(muted("From", theme).small());
                self.feature_link(ui, &owner);
            });
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
            let consequence_label =
                explanation_relationship_label(explanation, self.active_projection());
            ui.heading("Why this exists");
            ui.label(explanation_summary(
                explanation,
                consequence_label.as_deref(),
            ));
            ui.add_space(12.0);
            explanation_diagram(ui, explanation, consequence_label.as_deref(), theme);
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
                        "Partial proof · {} recorded dependencies. Evidence retains all facts and links available in this view.",
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
                    ui.label(muted("VIEW RECOMMENDATION", theme).small());
                    match self.agent_view_decision(ui.ctx()) {
                        Ok(decision) => {
                            ui.strong(decision.choice.label());
                            ui.label(muted("Deterministic view mock", theme).small());
                            if decision.weights.is_empty() {
                                ui.label(muted("Choice only · no weights returned", theme).small());
                            } else {
                                ui.label(muted("Returned weights · uncalibrated", theme).small());
                                for (option, weight) in &decision.weights {
                                    ui.add(egui::ProgressBar::new(*weight as f32)
                                        .text(format!("{}  {weight:.3}", option.label()))
                                        .fill(theme.accent));
                                }
                            }
                            ui.collapsing("Decision observation", |ui| {
                                ui.label(format!("Provider: {}", decision.result.provider));
                                ui.label(format!("Intent: {}", decision.observation.intent));
                                ui.label(format!("Revision: {}", decision.observation.revision));
                                for element in &decision.observation.selected_elements {
                                    ui.label(format!("Inquiry subject: {element}"));
                                }
                                ui.label("Offered: System World, Graph World, Requirements World, History");
                                if let Some(confidence) = decision.result.answers[0].confidence {
                                    ui.label(format!("Returned confidence: {confidence:.3} · uncalibrated"));
                                } else {
                                    ui.label("No confidence value returned");
                                }
                                ui.label("Presentation suggestion · no model mutation");
                            });
                            let already_open = self.world == decision.choice.world();
                            if ui.add_enabled(!already_open, egui::Button::new(if already_open {
                                "Recommended view is open"
                            } else {
                                "Open recommended view"
                            })).clicked() {
                                self.execute(decision.choice.command(), ui.ctx());
                            }
                        }
                        Err(error) => {
                            ui.label(muted("Recommendation unavailable", theme).small());
                            ui.label(muted(error, theme).small());
                        }
                    }
                    if ui.button("Dismiss agent view").clicked() {
                        self.dismiss_agent_view();
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
                let features = section_features(
                    &inspector.owned_features,
                    &inspector.effective_features,
                    &inspector.feature_provenance,
                    categories,
                );
                if !features.is_empty() {
                    theme.section(ui, title);
                    for (feature, inherited) in features {
                        if inherited {
                            self.inherited_feature_link(ui, feature);
                        } else if inspector.feature_provenance.contains_key(&feature.id) {
                            self.feature_link(ui, feature);
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                self.feature_link(ui, feature);
                                ui.label(muted("Origin unknown", theme).small()).on_hover_text(
                                    "This owned feature has no provenance metadata in the current inspector response.",
                                );
                            });
                        }
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
                    .map(|port| crate::viewport::port_direction_label(port.direction).to_owned())
                    .unwrap_or_else(|| "Not provided by this projection".into());
                value(ui, "Direction", &direction, theme);
            }
            ui.collapsing("Effective semantics", |ui| {
                for (label, features) in [
                    ("Owned features", &inspector.owned_features),
                    ("Effective features", &inspector.effective_features),
                    ("Specializes", &inspector.specializations),
                    ("Subsets", &inspector.subsettings),
                    ("Redefines", &inspector.redefinitions),
                ] {
                    if !features.is_empty() {
                        ui.label(muted(label, theme).small());
                        for feature in features {
                            ui.horizontal_wrapped(|ui| {
                                self.feature_link(ui, feature);
                                if let Some(provenance) =
                                    inspector.feature_provenance.get(&feature.id)
                                {
                                    ui.label(muted(origin_label(provenance.origin), theme).small())
                                        .on_hover_text(if provenance.source_available {
                                            "Source is available in this revision."
                                        } else {
                                            "No source location is available in this revision."
                                        });
                                }
                            });
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
                ui.label(muted("Port", theme));
                theme.section(ui, "INTERFACE");
                value(
                    ui,
                    "Direction",
                    crate::viewport::port_direction_label(port.direction),
                    theme,
                );
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
    }
}

/// Primary sections show named authored features, including inherited ports and
/// interfaces outside the scene's current depth. Unknown owned provenance stays
/// visible; unknown inherited and known standard/derived details remain in
/// Effective semantics. Owned IDs take precedence; names never establish origin.
fn section_features<'a>(
    owned: &'a [FeatureSummary],
    effective: &'a [FeatureSummary],
    provenance: &std::collections::BTreeMap<ElementId, FeatureProvenance>,
    categories: &[NodeCategory],
) -> Vec<(&'a FeatureSummary, bool)> {
    let mut seen = std::collections::BTreeSet::new();
    owned
        .iter()
        .map(|feature| (feature, false))
        .chain(
            effective
                .iter()
                .filter(|feature| {
                    matches!(
                        NodeCategory::from_semantic_kind(&feature.semantic_kind),
                        NodeCategory::Port | NodeCategory::Interface
                    )
                })
                .map(|feature| (feature, true)),
        )
        .filter(|(feature, inherited)| {
            categories.contains(&NodeCategory::from_semantic_kind(&feature.semantic_kind))
                && provenance
                    .get(&feature.id)
                    .map_or(!*inherited, |provenance| {
                        provenance.origin == ViewOrigin::Authored && provenance.has_declared_name
                    })
                && seen.insert(feature.id)
        })
        .collect()
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

/// The summary follows the selected fact's actual incoming rule and that rule's
/// immediate support. Unrelated rules/facts never become causes by list order.
struct ExplanationSummaryGraph<'a> {
    consequence: Option<&'a agq_modeling_view::ExplanationNode>,
    rule: Option<&'a agq_modeling_view::ExplanationNode>,
    support: Vec<&'a agq_modeling_view::ExplanationNode>,
}

fn explanation_summary_graph(explanation: &ExplanationProjection) -> ExplanationSummaryGraph<'_> {
    let consequence = explanation
        .nodes
        .iter()
        .filter(|node| node.element_id == Some(explanation.subject_id))
        .find(|node| {
            explanation.edges.iter().any(|edge| {
                edge.target == node.id
                    && explanation.nodes.iter().any(|source| {
                        source.id == edge.source && source.kind == ExplanationNodeKind::Rule
                    })
            })
        })
        .or_else(|| {
            explanation
                .nodes
                .iter()
                .find(|node| node.element_id == Some(explanation.subject_id))
        });
    let rule = consequence.and_then(|consequence| {
        explanation
            .edges
            .iter()
            .filter(|edge| edge.target == consequence.id)
            .find_map(|edge| {
                explanation
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.source && node.kind == ExplanationNodeKind::Rule)
            })
    });
    let mut seen = std::collections::BTreeSet::new();
    let support = rule.map_or_else(Vec::new, |rule| {
        explanation
            .edges
            .iter()
            .filter(|edge| edge.target == rule.id && seen.insert(&edge.source))
            .filter_map(|edge| {
                explanation.nodes.iter().find(|node| {
                    node.id == edge.source
                        && node.kind != ExplanationNodeKind::Rule
                        && consequence.is_none_or(|consequence| consequence.id != node.id)
                })
            })
            .collect()
    });
    ExplanationSummaryGraph {
        consequence,
        rule,
        support,
    }
}

/// A relationship's readable endpoints must come from the same exact revision
/// and producer. Only known binary Subsetting/Specialization projections are summarized;
/// property facts and connectors retain their exact proof label.
fn explanation_relationship_label(
    explanation: &ExplanationProjection,
    projection: &agq_modeling_view::ViewProjection,
) -> Option<String> {
    if projection.revision_id != explanation.revision_id
        || explanation_summary_graph(explanation).consequence?.kind != ExplanationNodeKind::Element
    {
        return None;
    }
    let mut edges = projection
        .edges
        .iter()
        .filter(|edge| edge.relationship_id == Some(explanation.subject_id));
    let edge = edges.next()?;
    if edges.next().is_some()
        || edge.revision_id != explanation.revision_id
        || edge.rule_id != explanation.rule_id
        || edge.origin != explanation.origin
        || !matches!(
            (edge.family, edge.semantic_kind.as_str()),
            (RelationshipFamily::Subsetting, "Subsetting")
                | (
                    RelationshipFamily::Specialization,
                    "Specialization" | "Subclassification"
                )
        )
        || !edge.directed
    {
        return None;
    }
    let endpoint = |id| {
        projection
            .nodes
            .iter()
            .find(|node| node.id == id && node.revision_id == explanation.revision_id)
    };
    Some(format!(
        "{} {} {}",
        endpoint(edge.source)?.name,
        edge.label,
        endpoint(edge.target)?.name
    ))
}

fn explanation_rule_label(name: &str) -> &str {
    match name {
        // Presentation name only. The accepted rule's applicability and target
        // remain in sysml-semantics/producers.rs, not in native UI logic.
        "checkOccurrenceUsageSuboccurrenceSpecialization" => "Suboccurrence specialization",
        "checkPartDefinitionSpecialization" => "Part definition specialization",
        _ => name,
    }
}

fn explanation_summary(
    explanation: &ExplanationProjection,
    consequence_label: Option<&str>,
) -> String {
    let graph = explanation_summary_graph(explanation);
    let fact = consequence_label
        .or_else(|| graph.consequence.map(|node| node.label.as_str()))
        .map_or_else(|| "this fact".into(), |label| format!("“{label}”"));
    match explanation.origin {
        ViewOrigin::Derived => {
            let mut summary = format!("The model derives {fact}");
            if let Some(name) = &explanation.rule_name {
                let name = explanation_rule_label(name);
                summary.push_str(&format!(" by applying “{name}”"));
            }
            summary.push('.');
            if graph.rule.is_some() {
                let support: Vec<_> = graph
                    .support
                    .iter()
                    .take(2)
                    .map(|node| format!("“{}”", node.label))
                    .collect();
                if !support.is_empty() {
                    summary.push_str(&format!(
                        " Its recorded support includes {}.",
                        support.join(" and ")
                    ));
                }
            }
            if explanation.truncated {
                summary.push_str(" This is a partial view of the recorded support.");
            }
            summary
        }
        ViewOrigin::Authored => {
            format!("The fact {fact} comes directly from the project's authored model.")
        }
        ViewOrigin::Standard => format!(
            "The fact {fact} belongs to the accepted standard library used by this revision."
        ),
        ViewOrigin::Generated => format!(
            "The model records {fact} as generated. Inspect its exact identity and publication context below."
        ),
    }
}

fn explanation_display_label<'a>(
    node: &'a agq_modeling_view::ExplanationNode,
    rule_name: Option<&str>,
) -> &'a str {
    if node.kind == ExplanationNodeKind::Rule && rule_name.is_none() {
        "Semantic derivation rule"
    } else if node.kind == ExplanationNodeKind::Rule
        && rule_name == Some("checkOccurrenceUsageSuboccurrenceSpecialization")
    {
        "Suboccurrence specialization"
    } else if node.kind == ExplanationNodeKind::Rule
        && rule_name == Some("checkPartDefinitionSpecialization")
    {
        "Part definition specialization"
    } else {
        &node.label
    }
}

/// Fixed, legible card heights keep the entire causal summary in one viewport.
/// Narrow windows stack the three stages; they never create horizontal scroll.
fn explanation_card_layout(
    width: f32,
    support_count: usize,
    has_rule: bool,
    has_consequence: bool,
) -> (egui::Vec2, Vec<egui::Rect>) {
    use egui::{Pos2, Rect, Vec2};
    let card_height = 76.0;
    let gap = (width * 0.08).min(20.0);
    let horizontal = width >= 600.0;
    let mut cards = Vec::new();
    let height = if !has_rule {
        if has_consequence {
            cards.push(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(width, card_height),
            ));
        }
        card_height
    } else if horizontal {
        let card_width = (width - gap * 2.0) / 3.0;
        let height = (support_count.max(1) as f32 * (card_height + 12.0)) - 12.0;
        for row in 0..support_count {
            cards.push(Rect::from_min_size(
                egui::pos2(0.0, row as f32 * (card_height + 12.0)),
                Vec2::new(card_width, card_height),
            ));
        }
        for column in 1..=(1 + usize::from(has_consequence)) {
            cards.push(Rect::from_min_size(
                egui::pos2(
                    column as f32 * (card_width + gap),
                    (height - card_height) * 0.5,
                ),
                Vec2::new(card_width, card_height),
            ));
        }
        height
    } else {
        let support_width =
            (width - gap * support_count.saturating_sub(1) as f32) / support_count.max(1) as f32;
        for column in 0..support_count {
            cards.push(Rect::from_min_size(
                egui::pos2(column as f32 * (support_width + gap), 0.0),
                Vec2::new(support_width, card_height),
            ));
        }
        let first_row = usize::from(support_count > 0);
        for row in first_row..=(first_row + usize::from(has_consequence)) {
            cards.push(Rect::from_min_size(
                egui::pos2(0.0, row as f32 * (card_height + gap)),
                Vec2::new(width, card_height),
            ));
        }
        (first_row + 1 + usize::from(has_consequence)) as f32 * (card_height + gap) - gap
    };
    (Vec2::new(width, height), cards)
}

/// The topology is taken only from the proof projection. A display arrow is
/// never inferred from node ordering or used as a modeled system connection.
fn explanation_diagram(
    ui: &mut egui::Ui,
    explanation: &ExplanationProjection,
    consequence_label: Option<&str>,
    theme: crate::theme::Theme,
) {
    use egui::{Align2, FontId, Pos2, Rect, Sense, Stroke, Vec2};
    use std::collections::BTreeMap;
    let graph = explanation_summary_graph(explanation);
    let width = ui.available_width().max(1.0);
    let horizontal = width >= 600.0;
    let support_limit = if horizontal && ui.available_height() >= 350.0 {
        3
    } else {
        2
    };
    let mut nodes: Vec<_> = graph.support.iter().take(support_limit).copied().collect();
    let shown_support = nodes.len();
    nodes.extend(graph.rule);
    nodes.extend(graph.consequence);
    let (size, cards) = explanation_card_layout(
        width,
        shown_support,
        graph.rule.is_some(),
        graph.consequence.is_some(),
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let positions: BTreeMap<_, Rect> = nodes
        .iter()
        .zip(cards)
        .map(|(node, card)| (node.id.as_str(), card.translate(rect.min.to_vec2())))
        .collect();
    for edge in &explanation.edges {
        if let (Some(source), Some(target)) = (
            positions.get(edge.source.as_str()),
            positions.get(edge.target.as_str()),
        ) {
            let (points, end, arrow) = if horizontal {
                let start = source.right_center();
                let end = target.left_center();
                let middle = (start.x + end.x) * 0.5;
                (
                    vec![
                        start,
                        Pos2::new(middle, start.y),
                        Pos2::new(middle, end.y),
                        end,
                    ],
                    end,
                    Vec2::new(8.0, 0.0),
                )
            } else {
                let start = source.center_bottom();
                let end = target.center_top();
                let middle = (start.y + end.y) * 0.5;
                (
                    vec![
                        start,
                        Pos2::new(start.x, middle),
                        Pos2::new(end.x, middle),
                        end,
                    ],
                    end,
                    Vec2::new(0.0, 8.0),
                )
            };
            ui.painter()
                .add(egui::Shape::line(points, Stroke::new(1.4, theme.muted)));
            ui.painter()
                .arrow(end - arrow, arrow, Stroke::new(1.4, theme.muted));
        }
    }
    for node in nodes {
        let card = positions[node.id.as_str()];
        let rule = node.kind == ExplanationNodeKind::Rule;
        let consequence = graph
            .consequence
            .is_some_and(|outcome| outcome.id == node.id);
        ui.painter().rect(
            card,
            7.0,
            if rule { theme.elevated } else { theme.surface },
            Stroke::new(
                if consequence { 1.5 } else { 1.0 },
                if rule || consequence {
                    theme.accent
                } else {
                    theme.border
                },
            ),
            egui::StrokeKind::Inside,
        );
        let heading = if rule {
            "SEMANTIC RULE"
        } else if consequence {
            if graph.rule.is_some() {
                "CONSEQUENCE"
            } else {
                "SELECTED FACT"
            }
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
        let display_label = if consequence {
            consequence_label.unwrap_or(&node.label)
        } else {
            explanation_display_label(node, explanation.rule_name.as_deref())
        };
        let mut job = egui::text::LayoutJob::simple(
            display_label.to_owned(),
            FontId::proportional(12.0),
            theme.text,
            (card.width() - 20.0).max(1.0),
        );
        job.wrap.max_rows = 3;
        job.wrap.break_anywhere = false;
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        ui.painter()
            .galley(card.min + Vec2::new(10.0, 28.0), galley, theme.text);
        let response = ui.interact(card, ui.id().with(&node.id), Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Label,
                true,
                format!("{heading}: {display_label}"),
            )
        });
        response.on_hover_text(format!("{}\n{}", node.label, node.id));
    }
    if graph.rule.is_some() {
        ui.add_space(8.0);
        ui.label(muted(format!(
            "Summary · {shown_support} of {} available supporting facts shown. Expand Evidence for exact details.",
            graph.support.len(),
        ), theme).small());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(id: u128, name: &str, kind: &str) -> FeatureSummary {
        FeatureSummary {
            id: ElementId::from_u128(id),
            name: name.into(),
            semantic_kind: kind.into(),
        }
    }

    #[test]
    fn main_port_section_preserves_inherited_identity_and_same_name_distinctions() {
        let owned = vec![feature(1, "revisions", "PortUsage")];
        let effective = vec![
            owned[0].clone(),
            feature(2, "revisions", "PortUsage"),
            feature(2, "revisions", "PortUsage"),
            feature(3, "access", "InterfaceUsage"),
            feature(4, "implementation", "PartUsage"),
        ];
        let provenance = authored_provenance(owned.iter().chain(&effective));
        let rows = section_features(
            &owned,
            &effective,
            &provenance,
            &[NodeCategory::Port, NodeCategory::Interface],
        );
        let actual: Vec<_> = rows
            .iter()
            .map(|(f, inherited)| (f.id, *inherited))
            .collect();
        assert_eq!(
            actual,
            vec![
                (ElementId::from_u128(1), false),
                (ElementId::from_u128(2), true),
                (ElementId::from_u128(3), true),
            ]
        );
        assert!(std::ptr::eq(rows[0].0, &owned[0]));
        assert!(std::ptr::eq(rows[1].0, &effective[1]));
        assert!(
            section_features(&owned, &effective, &provenance, &[NodeCategory::Part]).is_empty()
        );
    }

    fn authored_provenance<'a>(
        features: impl Iterator<Item = &'a FeatureSummary>,
    ) -> std::collections::BTreeMap<ElementId, FeatureProvenance> {
        features
            .map(|feature| {
                (
                    feature.id,
                    FeatureProvenance {
                        origin: ViewOrigin::Authored,
                        source_available: true,
                        has_declared_name: true,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn primary_features_use_canonical_provenance_without_scene_or_name_guesses() {
        let owned = vec![
            feature(1, "localPart", "PartUsage"),
            feature(2, "localPort", "PortUsage"),
            feature(3, "unnamed owned fallback", "PortUsage"),
            feature(4, "owned standard", "PortUsage"),
        ];
        let effective = vec![
            owned[1].clone(),
            feature(5, "repositoryRevisions", "PortUsage"),
            feature(6, "Connector", "InterfaceUsage"),
            feature(7, "repositoryRevisions", "PortUsage"),
            feature(8, "derivedNamedPort", "PortUsage"),
            feature(9, "generatedNamedPort", "PortUsage"),
            feature(10, "unknown inherited", "PortUsage"),
            feature(11, "PortUsage", "PortUsage"),
        ];
        let unchanged_owned = owned.clone();
        let unchanged_effective = effective.clone();
        let mut provenance = authored_provenance(owned.iter().chain(&effective));
        let id = ElementId::from_u128;
        // Absence from provenance does not erase a directly owned component/port.
        provenance.remove(&id(1));
        provenance.remove(&id(2));
        provenance.remove(&id(10));
        provenance.get_mut(&id(3)).unwrap().has_declared_name = false;
        provenance.get_mut(&id(4)).unwrap().origin = ViewOrigin::Standard;
        provenance.get_mut(&id(7)).unwrap().origin = ViewOrigin::Standard;
        provenance.get_mut(&id(8)).unwrap().origin = ViewOrigin::Derived;
        provenance.get_mut(&id(9)).unwrap().origin = ViewOrigin::Generated;
        provenance.get_mut(&id(11)).unwrap().has_declared_name = false;
        // A source link is optional for canonical authored records. No visible
        // scene is consulted, including for the inherited repository port.
        provenance.get_mut(&id(6)).unwrap().source_available = false;
        let rows = section_features(
            &owned,
            &effective,
            &provenance,
            &[NodeCategory::Port, NodeCategory::Interface],
        );
        assert_eq!(
            rows.iter()
                .map(|(f, inherited)| (f.id, *inherited))
                .collect::<Vec<_>>(),
            vec![(id(2), false), (id(5), true), (id(6), true)]
        );
        assert!(std::ptr::eq(rows[1].0, &effective[1]));
        assert_eq!(
            section_features(&owned, &effective, &provenance, &[NodeCategory::Part])
                .iter()
                .map(|(f, _)| f.id)
                .collect::<Vec<_>>(),
            vec![id(1)]
        );
        // Classification never removes the exact facts from advanced inspection.
        assert_eq!(owned, unchanged_owned);
        assert_eq!(effective, unchanged_effective);
    }

    #[test]
    fn unnamed_rule_summary_preserves_exact_evidence_and_other_labels() {
        let mut node = agq_modeling_view::ExplanationNode {
            id: "rule:00000000-0000-0000-0000-000000000123".into(),
            element_id: None,
            label: "Semantic producer 00000000-0000-0000-0000-000000000123".into(),
            kind: ExplanationNodeKind::Rule,
        };
        let evidence = node.clone();
        assert_eq!(
            explanation_display_label(&node, None),
            "Semantic derivation rule"
        );
        assert_eq!(node, evidence);
        assert_eq!(
            explanation_display_label(&node, Some("known rule")),
            node.label
        );
        node.kind = ExplanationNodeKind::Fact;
        assert_eq!(explanation_display_label(&node, None), node.label);
    }

    fn explanation_test_projection() -> ExplanationProjection {
        use agq_modeling_view::{ExplanationEdge, ExplanationNode};
        let mut projection = ExplanationProjection {
            revision_id: agq_studio_scene::fixtures::revision(),
            subject_id: ElementId::from_u128(12),
            origin: ViewOrigin::Derived,
            rule_id: Some(agq_kernel::RuleId::from_u128(13)),
            rule_name: Some("checkOccurrenceUsageSuboccurrenceSpecialization".into()),
            profile: "test-only-proof".into(),
            nodes: vec![
                ExplanationNode {
                    id: "unrelated-rule".into(),
                    element_id: None,
                    label: "Unrelated rule".into(),
                    kind: ExplanationNodeKind::Rule,
                },
                ExplanationNode {
                    id: "consequence".into(),
                    element_id: Some(ElementId::from_u128(12)),
                    label: "Subsetting : Subsetting".into(),
                    kind: ExplanationNodeKind::Element,
                },
                ExplanationNode {
                    id: "rule".into(),
                    element_id: None,
                    label: "checkOccurrenceUsageSuboccurrenceSpecialization".into(),
                    kind: ExplanationNodeKind::Rule,
                },
            ],
            edges: vec![ExplanationEdge {
                source: "rule".into(),
                target: "consequence".into(),
                label: "implies".into(),
                presentation_only: true,
            }],
            evidence_count: 621,
            truncated: true,
        };
        for n in 0..8 {
            projection.nodes.push(ExplanationNode {
                id: format!("support-{n}"),
                element_id: None,
                label: format!("Supporting fact {n}"),
                kind: ExplanationNodeKind::Fact,
            });
            projection.edges.push(ExplanationEdge {
                source: format!("support-{n}"),
                target: "rule".into(),
                label: "declared evidence".into(),
                presentation_only: true,
            });
        }
        projection
    }

    #[test]
    fn explain_summary_keeps_rule_and_consequence_visible_with_many_dependencies() {
        let projection = explanation_test_projection();
        let original = projection.clone();
        let graph = explanation_summary_graph(&projection);
        assert_eq!(graph.consequence.unwrap().id, "consequence");
        assert_eq!(graph.rule.unwrap().id, "rule");
        assert_eq!(graph.support.len(), 8);
        for (width, support) in [(720.0, 3), (600.0, 3), (599.0, 2), (320.0, 2)] {
            let (size, cards) = explanation_card_layout(width, support, true, true);
            assert_eq!(cards.len(), support + 2);
            assert!(
                size.y <= 268.0,
                "the rule and conclusion must fit before the evidence disclosure"
            );
            let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, size).expand(0.001);
            for (i, card) in cards.iter().enumerate() {
                assert!(
                    bounds.contains_rect(*card),
                    "clipped card {i} at width {width}"
                );
                for other in cards.iter().skip(i + 1) {
                    assert!(
                        !card.intersects(*other),
                        "overlapping cards at width {width}"
                    );
                }
            }
        }
        assert_eq!(
            projection, original,
            "summary layout cannot truncate the exact evidence DTO"
        );
        assert_eq!(
            explanation_display_label(graph.rule.unwrap(), projection.rule_name.as_deref()),
            "Suboccurrence specialization"
        );
        assert_eq!(
            graph.rule.unwrap().label,
            "checkOccurrenceUsageSuboccurrenceSpecialization"
        );
    }

    #[test]
    fn explain_summary_never_infers_a_rule_or_support_from_unconnected_nodes() {
        let mut projection = explanation_test_projection();
        projection.edges.retain(|edge| edge.source != "rule");
        let graph = explanation_summary_graph(&projection);
        assert_eq!(graph.consequence.unwrap().id, "consequence");
        assert!(graph.rule.is_none());
        assert!(graph.support.is_empty());
        let (_, cards) =
            explanation_card_layout(320.0, graph.support.len(), graph.rule.is_some(), true);
        assert_eq!(cards.len(), 1);
    }

    #[test]
    fn explain_readable_consequence_requires_an_exact_revision_and_complete_edge() {
        let mut explanation = explanation_test_projection();
        let mut projection = agq_studio_scene::fixtures::architecture();
        // The fixture's first connections use feature-summary-only port IDs;
        // this positive case requires both canonical endpoint node DTOs.
        let endpoints: std::collections::BTreeSet<_> =
            projection.nodes.iter().map(|node| node.id).collect();
        projection
            .edges
            .retain(|edge| endpoints.contains(&edge.source) && endpoints.contains(&edge.target));
        projection.edges.truncate(1);
        let edge = &mut projection.edges[0];
        edge.relationship_id = Some(explanation.subject_id);
        edge.rule_id = explanation.rule_id;
        edge.origin = explanation.origin;
        edge.family = RelationshipFamily::Subsetting;
        edge.semantic_kind = "Subsetting".into();
        edge.directed = true;
        let expected = format!(
            "{} {} {}",
            projection
                .nodes
                .iter()
                .find(|node| node.id == edge.source)
                .unwrap()
                .name,
            edge.label,
            projection
                .nodes
                .iter()
                .find(|node| node.id == edge.target)
                .unwrap()
                .name
        );
        assert_eq!(
            explanation_relationship_label(&explanation, &projection),
            Some(expected)
        );
        let valid = projection.clone();
        let other_revision = agq_modeling_workspace::ProjectRevisionId::from_u128(88);
        for defect in 0..10 {
            let mut projection = valid.clone();
            match defect {
                0 => projection.revision_id = other_revision,
                1 => projection.edges[0].revision_id = other_revision,
                2 => projection.edges[0].rule_id = None,
                3 => projection.edges.push(projection.edges[0].clone()),
                4 => projection.nodes.clear(),
                5 => projection
                    .nodes
                    .iter_mut()
                    .for_each(|node| node.revision_id = other_revision),
                6 => projection.edges[0].semantic_kind = "ConnectionUsage".into(),
                7 => projection.edges[0].family = RelationshipFamily::Connection,
                8 => projection.edges[0].directed = false,
                9 => projection.edges[0].origin = ViewOrigin::Authored,
                _ => unreachable!(),
            }
            assert_eq!(
                explanation_relationship_label(&explanation, &projection),
                None,
                "defect {defect}"
            );
        }
        explanation
            .nodes
            .iter_mut()
            .find(|node| node.id == "consequence")
            .unwrap()
            .kind = ExplanationNodeKind::Fact;
        assert_eq!(
            explanation_relationship_label(&explanation, &valid),
            None,
            "a property proof is not the whole relationship"
        );
    }

    #[test]
    fn summary_names_only_actual_consequence_and_immediate_support() {
        use agq_modeling_view::{ExplanationEdge, ExplanationNode};
        let subject = ElementId::from_u128(1);
        let rule_id = agq_kernel::RuleId::from_u128(123);
        let mut projection = ExplanationProjection {
            revision_id: agq_modeling_workspace::ProjectRevisionId::from_u128(2),
            subject_id: subject,
            origin: ViewOrigin::Derived,
            rule_id: Some(rule_id),
            rule_name: None,
            profile: "test".into(),
            nodes: [
                ("unrelated", "Unrelated fact", ExplanationNodeKind::Fact),
                ("consequence", "engine · type", ExplanationNodeKind::Fact),
                ("rule", "Unnamed producer", ExplanationNodeKind::Rule),
                (
                    "support1",
                    "engine : PartUsage",
                    ExplanationNodeKind::Element,
                ),
                (
                    "support2",
                    "Engine : PartDefinition",
                    ExplanationNodeKind::Element,
                ),
            ]
            .into_iter()
            .map(|(id, label, kind)| ExplanationNode {
                id: if id == "rule" {
                    rule_id.to_string()
                } else {
                    id.into()
                },
                element_id: (id == "consequence").then_some(subject),
                label: if id == "rule" {
                    format!("Semantic producer {rule_id}")
                } else {
                    label.into()
                },
                kind,
            })
            .collect(),
            edges: [
                ("support1".into(), rule_id.to_string()),
                ("support2".into(), rule_id.to_string()),
                (rule_id.to_string(), "consequence".into()),
            ]
            .into_iter()
            .map(|(source, target)| ExplanationEdge {
                source,
                target,
                label: "test support".into(),
                presentation_only: true,
            })
            .collect(),
            evidence_count: 10,
            truncated: true,
        };
        let original = projection.clone();
        let summary = explanation_summary(&projection, None);
        assert_eq!(
            summary,
            "The model derives “engine · type”. Its recorded support includes “engine : PartUsage” and “Engine : PartDefinition”. This is a partial view of the recorded support."
        );
        assert!(!summary.contains(&rule_id.to_string()));
        assert!(!summary.contains("Unrelated fact"));
        assert_eq!(projection, original);
        projection.rule_name = Some("actualRecordedRule".into());
        assert!(
            explanation_summary(&projection, None).contains("by applying “actualRecordedRule”")
        );
        projection.origin = ViewOrigin::Authored;
        projection.edges.clear();
        assert_eq!(
            explanation_summary(&projection, None),
            "The fact “engine · type” comes directly from the project's authored model."
        );
    }
}
