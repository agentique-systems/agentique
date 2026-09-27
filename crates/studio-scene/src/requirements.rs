//! Lanes for the requirements view: requirements, what satisfies them, and
//! their subjects. Roles come from element kinds and edges, never from names.
use crate::{
    EdgeKind, LayoutEngine, LayoutInput, LayoutMemory, LayoutResult, NodeCategory, Rect,
    SceneInput, Size,
};
use agq_language::ElementId;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequirementRole {
    Requirement,
    Satisfies,
    Subject,
    Context,
}

impl RequirementRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Requirement => "REQUIREMENTS",
            Self::Satisfies => "SATISFIED BY",
            Self::Subject => "SUBJECTS",
            Self::Context => "OTHER",
        }
    }
}

/// The lane of each card in the requirements view.
pub fn requirement_roles(input: &SceneInput) -> BTreeMap<ElementId, RequirementRole> {
    let mut roles: BTreeMap<_, _> = input
        .nodes
        .iter()
        .map(|node| {
            let role = if node.kind == NodeCategory::Requirement {
                RequirementRole::Requirement
            } else {
                RequirementRole::Context
            };
            (node.id, role)
        })
        .collect();
    for edge in &input.edges {
        let target_role = roles.get(&edge.target.node).copied();
        let source_role = roles.get(&edge.source.node).copied();
        if edge.kind == EdgeKind::Satisfy && source_role == Some(RequirementRole::Context) {
            roles.insert(edge.source.node, RequirementRole::Satisfies);
        }
        if edge.kind == EdgeKind::Typing
            && source_role == Some(RequirementRole::Requirement)
            && target_role == Some(RequirementRole::Context)
        {
            roles.insert(edge.target.node, RequirementRole::Subject);
        }
    }
    roles
}

/// Lanes in role order; empty lanes are omitted. Positions are presentation only.
pub struct RequirementsLayout {
    roles: BTreeMap<ElementId, RequirementRole>,
}
impl RequirementsLayout {
    pub fn new(input: &SceneInput) -> Self {
        Self {
            roles: requirement_roles(input),
        }
    }
}
impl LayoutEngine for RequirementsLayout {
    fn layout(&self, input: &LayoutInput, _previous: Option<&LayoutMemory>) -> LayoutResult {
        let mut lanes = BTreeMap::<RequirementRole, Vec<_>>::new();
        for node in &input.nodes {
            lanes
                .entry(
                    self.roles
                        .get(&node.id)
                        .copied()
                        .unwrap_or(RequirementRole::Context),
                )
                .or_default()
                .push(node);
        }
        let mut bounds = BTreeMap::new();
        for (column, (_, mut nodes)) in lanes.into_iter().enumerate() {
            nodes.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
            let mut y = 54.0;
            for node in nodes {
                let size = input.node_size(node, Size::new(248.0, 126.0));
                bounds.insert(
                    node.id,
                    Rect::new(column as f32 * 360.0, y, size.width, size.height),
                );
                y += size.height + 56.0;
            }
        }
        LayoutResult {
            bounds,
            pin_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutKind, Scene, SceneOptions, fixtures};

    #[test]
    fn requirements_satisfiers_and_subjects_are_ordered_lanes() {
        let input = fixtures::architecture().requirements_view();
        let roles = requirement_roles(&input);
        let find = |name: &str| {
            input
                .nodes
                .iter()
                .find(|n| n.name == name)
                .unwrap_or_else(|| panic!("{name} is in the requirements view"))
                .id
        };
        let requirement = find("uniqueCodes");
        let satisfier = find("store");
        let subject = find("LinkStore");
        assert_eq!(roles[&requirement], RequirementRole::Requirement);
        assert_eq!(roles[&satisfier], RequirementRole::Satisfies);
        assert_eq!(roles[&subject], RequirementRole::Subject);
        let scene = Scene::build(
            &input,
            &SceneOptions {
                layout: LayoutKind::Requirements,
                ..Default::default()
            },
            None,
        )
        .unwrap();
        let x = |id| scene.node(id).unwrap().bounds.min.x;
        assert!(x(requirement) < x(satisfier));
        assert!(x(satisfier) < x(subject));
        assert!(
            scene
                .edges
                .iter()
                .any(|e| e.semantic.kind == EdgeKind::Satisfy)
        );
    }
}
