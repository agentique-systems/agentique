//! Engineering roles are presentation categories of exact projected relationships.
//! They add neither a satisfaction result nor shortcut semantic relationships.
use crate::{LayoutEngine, LayoutInput, LayoutMemory, LayoutResult, Rect, Size};
use agq_kernel::ElementId;
use agq_modeling_view::{RelationshipFamily, ViewProjection};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequirementRole {
    Requirement,
    Subject,
    Architecture,
    Satisfaction,
    Verification,
    Context,
}

impl RequirementRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Requirement => "REQUIREMENTS",
            Self::Subject => "SUBJECTS",
            Self::Architecture => "SYSTEM / PART",
            Self::Satisfaction => "SATISFACTION LINKS",
            Self::Verification => "VERIFICATION LINKS",
            Self::Context => "OWNED DETAIL",
        }
    }
}

/// Classify existing DTOs using metaclass and relationship kind. Display names
/// and edge labels never establish an engineering role. Unclassified details
/// remain visible as context instead of being promoted to satisfaction evidence.
pub fn requirement_roles(view: &ViewProjection) -> BTreeMap<ElementId, RequirementRole> {
    let mut roles: BTreeMap<_, _> = view
        .nodes
        .iter()
        .map(|node| {
            let role = match node.semantic_kind.as_str() {
                "VerificationCaseDefinition" | "VerificationCaseUsage" => {
                    RequirementRole::Verification
                }
                "SatisfyRequirementUsage" => RequirementRole::Satisfaction,
                "RequirementDefinition" | "RequirementUsage" => RequirementRole::Requirement,
                _ => RequirementRole::Context,
            };
            (node.id, role)
        })
        .collect();
    for edge in &view.edges {
        if edge.semantic_kind == "SubjectMembership"
            && edge.family == RelationshipFamily::Requirement
            && roles.get(&edge.source) == Some(&RequirementRole::Requirement)
        {
            roles.insert(edge.target, RequirementRole::Subject);
        }
    }
    for edge in &view.edges {
        if edge.family == RelationshipFamily::Typing
            && roles.get(&edge.source) == Some(&RequirementRole::Subject)
            && roles.get(&edge.target) == Some(&RequirementRole::Context)
        {
            roles.insert(edge.target, RequirementRole::Architecture);
        }
        if edge.family == RelationshipFamily::Verification
            && roles.get(&edge.source) == Some(&RequirementRole::Verification)
            && roles.get(&edge.target) == Some(&RequirementRole::Context)
        {
            roles.insert(edge.target, RequirementRole::Verification);
        }
    }
    roles
}

/// Ordered lanes keep the literal subject and its type legible as separate
/// canonical identities. Empty lanes are omitted; coordinates are disposable.
pub struct RequirementsLayout {
    roles: BTreeMap<ElementId, RequirementRole>,
}
impl RequirementsLayout {
    pub fn new(view: &ViewProjection) -> Self {
        Self {
            roles: requirement_roles(view),
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
    use crate::{SceneOptions, SemanticScene, fixtures};

    #[test]
    fn canonical_subject_and_type_are_separate_ordered_lanes_without_claiming_satisfaction() {
        let mut view = fixtures::requirements();
        view.nodes.truncate(3);
        view.nodes[0].semantic_kind = "RequirementDefinition".into();
        view.nodes[1].semantic_kind = "ReferenceUsage".into();
        view.nodes[2].semantic_kind = "PartDefinition".into();
        let ids: Vec<_> = view.nodes.iter().map(|n| n.id).collect();
        view.edges.truncate(2);
        view.edges[0].source = ids[0];
        view.edges[0].target = ids[1];
        view.edges[0].semantic_kind = "SubjectMembership".into();
        view.edges[1].source = ids[1];
        view.edges[1].target = ids[2];
        view.edges[1].family = RelationshipFamily::Typing;
        view.edges[1].semantic_kind = "FeatureTyping".into();
        let roles = requirement_roles(&view);
        assert_eq!(roles[&ids[0]], RequirementRole::Requirement);
        assert_eq!(roles[&ids[1]], RequirementRole::Subject);
        assert_eq!(roles[&ids[2]], RequirementRole::Architecture);
        assert!(!roles.values().any(|r| *r == RequirementRole::Satisfaction));
        let scene = SemanticScene::from_projection(
            &view,
            &SceneOptions {
                hierarchy: false,
                ..Default::default()
            },
            None,
        )
        .unwrap();
        assert!(
            scene.node(ids[0]).unwrap().bounds.max.x < scene.node(ids[1]).unwrap().bounds.min.x
        );
        assert!(
            scene.node(ids[1]).unwrap().bounds.max.x < scene.node(ids[2]).unwrap().bounds.min.x
        );
        assert_eq!(scene.edges.len(), view.edges.len());
        for edge in &scene.edges {
            assert!(view.edges.contains(&edge.semantic));
        }
    }
}
