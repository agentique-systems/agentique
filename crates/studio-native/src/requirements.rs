//! The Requirements Panel's content: each requirement with its subject, what
//! satisfies it, and its problems.
use crate::studio::Studio;
use agq_language::{Element, ElementId, ElementKind, Parent, Reference};
use agq_studio_scene::SceneTarget;
use agq_system_state::Operation;

#[derive(Clone)]
pub struct Row {
    pub id: ElementId,
    pub keyword: &'static str,
    pub name: String,
    pub doc: Option<String>,
    pub subjects: Vec<String>,
    pub satisfied_by: Vec<String>,
    pub problems: Vec<String>,
}

impl Studio {
    /// The Requirements Panel's rows for the current model, built once per
    /// model version.
    pub fn requirements(&mut self) -> Vec<Row> {
        let Some(project) = &self.project else {
            return Vec::new();
        };
        if self
            .requirement_rows
            .as_ref()
            .is_none_or(|(generation, _)| *generation != self.generation)
        {
            self.requirement_rows = Some((self.generation, rows(project.state())));
        }
        self.requirement_rows
            .as_ref()
            .map(|(_, rows)| rows.clone())
            .unwrap_or_default()
    }

    /// The selected part or item a requirement can be satisfied by.
    pub fn satisfying_part(&self) -> Option<ElementId> {
        let tree = self.project.as_ref()?.state().tree();
        self.selected_card().filter(|id| {
            tree.get(*id)
                .is_some_and(|e| matches!(e.kind, ElementKind::Part | ElementKind::Item))
        })
    }

    /// Selects an element that has a card and moves the camera to it.
    pub fn show_element(&mut self, id: ElementId) {
        let target = SceneTarget::Node(id);
        if self.scene.target_bounds(&target).is_some() {
            self.select(target.clone(), false);
            self.frame_target(&target);
        }
    }

    /// Adds `satisfy requirement by part` next to the requirement.
    pub fn satisfy(&mut self, requirement: ElementId, part: ElementId) {
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
