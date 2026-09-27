//! The Requirements Panel: each requirement with its subject, what satisfies
//! it, and its problems.
use crate::app::StudioApp;
use agq_language::{Element, ElementId, ElementKind, Parent, Reference};
use agq_studio_scene::SceneTarget;
use agq_system_state::Operation;
use eframe::egui::{self, RichText};

#[derive(Clone)]
pub struct Row {
    id: ElementId,
    keyword: &'static str,
    name: String,
    doc: Option<String>,
    subjects: Vec<String>,
    satisfied_by: Vec<String>,
    problems: Vec<String>,
}

impl StudioApp {
    pub fn requirements_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let Some(project) = &self.project else {
            ui.label(crate::app::muted(
                "Open a project to see its requirements.",
                theme,
            ));
            return;
        };
        let state = project.state();
        let tree = state.tree();
        if self
            .requirement_rows
            .as_ref()
            .is_none_or(|(generation, _)| *generation != self.generation)
        {
            self.requirement_rows = Some((self.generation, rows(state)));
        }
        let rows = self
            .requirement_rows
            .as_ref()
            .map(|(_, rows)| rows.clone())
            .unwrap_or_default();
        if rows.is_empty() {
            ui.label(crate::app::muted(
                "No requirements yet. R creates one.",
                theme,
            ));
            return;
        }
        let selected_part = self.selected_card().filter(|id| {
            tree.get(*id)
                .is_some_and(|e| matches!(e.kind, ElementKind::Part | ElementKind::Item))
        });
        let mut select = None;
        let mut satisfy = None;
        for row in &rows {
            ui.add_space(6.0);
            let title = ui.add(
                egui::Button::new(RichText::new(format!("{}  {}", row.keyword, row.name)).strong())
                    .frame(false),
            );
            if title.clicked() {
                select = Some(row.id);
            }
            if let Some(doc) = &row.doc {
                ui.label(crate::app::muted(doc, theme));
            }
            for subject in &row.subjects {
                ui.label(format!("Subject: {subject}"));
            }
            if row.satisfied_by.is_empty() {
                ui.label(RichText::new("Not satisfied by anything yet").color(theme.muted));
            }
            for by in &row.satisfied_by {
                ui.label(RichText::new(format!("✓ Satisfied by {by}")).color(theme.green));
            }
            for problem in &row.problems {
                ui.label(RichText::new(format!("• {problem}")).color(theme.amber));
            }
            if self.editable()
                && row.keyword == "requirement"
                && let Some(part) = selected_part
                && ui
                    .small_button(format!(
                        "Satisfied by {}",
                        tree.effective_name(part).unwrap_or("the selected part")
                    ))
                    .clicked()
            {
                satisfy = Some((row.id, part));
            }
            ui.separator();
        }
        if let Some(id) = select {
            let target = SceneTarget::Node(id);
            if self.scene.target_bounds(&target).is_some() {
                self.select(target.clone(), false);
                self.frame_target(&target);
            }
        }
        if let Some((requirement, part)) = satisfy {
            self.satisfy(requirement, part);
        }
    }

    /// Adds `satisfy requirement by part` next to the requirement.
    fn satisfy(&mut self, requirement: ElementId, part: ElementId) {
        let Some(tree) = self.project.as_ref().map(|p| p.state().tree()) else {
            return;
        };
        let Some(owner) = tree.get(requirement).and_then(Element::owner) else {
            self.status = "A top-level requirement cannot hold a satisfy relationship".into();
            return;
        };
        // Both ends are linked by identity; the printed text names them so
        // that they resolve back to the same elements from the satisfy's owner.
        let mut element = Element::new(ElementKind::Satisfy);
        element.target = Some(Reference::to(
            requirement,
            tree.effective_name(requirement).unwrap_or(""),
        ));
        element.by = Some(Reference::to(part, tree.effective_name(part).unwrap_or("")));
        let description = format!(
            "{} satisfies {}",
            tree.effective_name(part).unwrap_or("part"),
            tree.effective_name(requirement).unwrap_or("requirement")
        );
        self.operation(
            &description,
            Operation::Create {
                parent: Parent::Element(owner),
                element: Box::new(element),
            },
        );
    }
}

/// The rows of the Requirements Panel, built once per model version.
fn rows(state: &agq_system_state::SystemState) -> Vec<Row> {
    let tree = state.tree();
    let satisfies: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::Satisfy)
        .collect();
    let mut rows = Vec::new();
    for id in tree.walk() {
        let e = &tree[id];
        if !matches!(
            e.kind,
            ElementKind::Requirement | ElementKind::RequirementDef
        ) {
            continue;
        }
        let children = || e.children().iter().map(|c| (*c, &tree[*c]));
        let definition = e.typed_by.first().and_then(Reference::target);
        rows.push(Row {
            id,
            keyword: e.kind.keyword(),
            name: crate::edit::display_path(tree, id),
            doc: children()
                .find(|(_, c)| c.kind == ElementKind::Doc)
                .and_then(|(_, c)| c.text.clone()),
            subjects: children()
                .filter(|(_, c)| c.kind == ElementKind::Subject)
                .map(|(_, c)| {
                    c.typed_by
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .collect(),
            satisfied_by: satisfies
                .iter()
                .copied()
                .filter(|s| {
                    let s = &tree[*s];
                    s.kind == ElementKind::Satisfy
                        && s.target
                            .as_ref()
                            .and_then(Reference::target)
                            .is_some_and(|t| t == id || Some(t) == definition)
                })
                .filter_map(|s| tree[s].by.as_ref().map(ToString::to_string))
                .collect(),
            problems: tree
                .descendants(id)
                .into_iter()
                .flat_map(|d| state.diagnostics_for(d).map(|p| p.message.clone()))
                .collect(),
        });
    }
    rows
}
