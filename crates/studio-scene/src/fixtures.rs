//! Deterministic VISUAL FIXTURES using the actual public projection DTO.
//!
//! These records are not reconstructed semantics, accepted publications or
//! repository revisions. They test scene/interaction quality only. IDs live in
//! a reserved fixture namespace and all metadata states their limited authority.
use agq_kernel::{ElementId, RuleId};
use agq_modeling_view::*;
use agq_modeling_workspace::ProjectRevisionId;
use std::collections::BTreeMap;

pub const FIXTURE_REVISION: u128 = 0xfa170000000000000000000000000001;
pub fn id(value: u128) -> ElementId {
    ElementId::from_u128(0xfa170000000000000000000000000000 | value)
}
pub fn revision() -> ProjectRevisionId {
    ProjectRevisionId::from_u128(FIXTURE_REVISION)
}
fn node(value: u128, name: &str, kind: &str, owner: Option<u128>) -> ViewNode {
    ViewNode {
        id: id(value),
        revision_id: revision(),
        semantic_kind: kind.into(),
        name: name.into(),
        qualified_name: Some(format!("VisualFixture::{name}")),
        owner: owner.map(id),
        origin: ViewOrigin::Authored,
        source_available: false,
        features: vec![],
        counts: FeatureCounts::default(),
        badges: vec![],
    }
}
fn edge(
    value: u128,
    source: u128,
    target: u128,
    family: RelationshipFamily,
    label: &str,
) -> ViewEdge {
    ViewEdge {
        id: format!("fixture-edge-{value:08}"),
        relationship_id: Some(id(1_000_000 + value)),
        revision_id: revision(),
        family,
        semantic_kind: match family {
            RelationshipFamily::Connection => "ConnectionUsage",
            RelationshipFamily::Typing => "FeatureTyping",
            RelationshipFamily::Ownership => "OwningMembership",
            RelationshipFamily::Requirement => "SatisfyRequirementUsage",
            RelationshipFamily::Verification => "RequirementVerificationMembership",
            _ => "Specialization",
        }
        .into(),
        source: id(source),
        target: id(target),
        origin: ViewOrigin::Authored,
        rule_id: None,
        label: label.into(),
        directed: family != RelationshipFamily::Connection,
        order: 0,
    }
}
fn projection(
    name: &str,
    kind: ViewKind,
    nodes: Vec<ViewNode>,
    edges: Vec<ViewEdge>,
) -> ViewProjection {
    let mut grouped: BTreeMap<ElementId, Vec<ElementId>> = BTreeMap::new();
    for n in &nodes {
        if let Some(owner) = n.owner {
            grouped.entry(owner).or_default().push(n.id);
        }
    }
    let groups = grouped
        .into_iter()
        .map(|(element_id, children)| ViewGroup {
            element_id,
            children,
        })
        .collect();
    let count = nodes.len();
    ViewProjection {
        revision_id: revision(),
        view: ViewDefinition {
            name: name.into(),
            kind,
            relationship_families: RelationshipFamily::all(),
            ..ViewDefinition::default()
        },
        nodes,
        edges,
        groups,
        metadata: ViewMetadata {
            suggested_focus: None,
            scope: "VISUAL FIXTURE — no language or real-model semantic acceptance".into(),
            producer_completeness: "Fixture: not evaluated".into(),
            local_element_count: count,
            omitted_standard_endpoints: 0,
            warnings: vec![],
        },
    }
}
/// Screenshot reference: three coherent subsystem containers, nine components,
/// stable semantic ports and an explicitly derived relationship.
pub fn architecture() -> ViewProjection {
    let mut nodes = vec![
        node(1, "NativeStudio", "PartDefinition", None),
        node(2, "ModelingPlatform", "PartDefinition", None),
        node(3, "AgentFabric", "PartDefinition", None),
        node(11, "SystemWorld", "PartUsage", Some(1)),
        node(12, "GraphWorld", "PartUsage", Some(1)),
        node(13, "Inspector", "PartUsage", Some(1)),
        node(21, "ModelRepository", "PartUsage", Some(2)),
        node(22, "ModelingService", "PartUsage", Some(2)),
        node(23, "ProjectWorkspace", "PartUsage", Some(2)),
        node(31, "DecisionAgent", "PartUsage", Some(3)),
        node(32, "ProposalEngine", "PartUsage", Some(3)),
        node(33, "ModelingAPI", "InterfaceUsage", Some(3)),
    ];
    for n in &mut nodes {
        if n.owner.is_some() {
            let value = n.id.as_u128() & 0xffff;
            n.features = vec![
                FeatureSummary {
                    id: id(value * 100 + 1),
                    name: "request".into(),
                    semantic_kind: "PortUsage".into(),
                },
                FeatureSummary {
                    id: id(value * 100 + 2),
                    name: "result".into(),
                    semantic_kind: "PortUsage".into(),
                },
            ];
            n.counts.ports = 2;
        } else {
            n.counts.parts = 3;
        }
    }
    let pairs = [
        (1102, 2101),
        (1202, 2201),
        (1302, 2301),
        (2102, 3101),
        (2202, 3201),
        (2302, 3301),
    ];
    let mut edges: Vec<_> = pairs
        .into_iter()
        .enumerate()
        .map(|(i, (a, b))| edge(i as u128, a, b, RelationshipFamily::Connection, "contract"))
        .collect();
    edges.push(edge(
        10,
        21,
        23,
        RelationshipFamily::Reference,
        "durable checkpoint",
    ));
    edges.push(edge(
        11,
        22,
        21,
        RelationshipFamily::Reference,
        "revision-bound service",
    ));
    edges.push(edge(
        12,
        31,
        32,
        RelationshipFamily::Reference,
        "candidate proposal",
    ));
    edges.push(edge(13, 12, 11, RelationshipFamily::Typing, "shared scene"));
    if let Some(derived) = edges.last_mut() {
        derived.origin = ViewOrigin::Derived;
        derived.rule_id = Some(RuleId::from_u128(73));
    }
    projection(
        "Agentique-like architecture · visual fixture",
        ViewKind::Architecture,
        nodes,
        edges,
    )
}
pub fn dense_ports() -> ViewProjection {
    let mut view = architecture();
    view.view.name = "Dense semantic ports · visual fixture".into();
    let mut next = 100;
    for a in [11, 12, 13, 21, 22, 23, 31, 32, 33] {
        for b in [11, 12, 13, 21, 22, 23, 31, 32, 33] {
            if a != b && ((a + b) % 3 == 0) {
                view.edges.push(edge(
                    next,
                    a * 100 + 2,
                    b * 100 + 1,
                    RelationshipFamily::Connection,
                    "signal",
                ));
                next += 1;
            }
        }
    }
    view
}
pub fn requirements() -> ViewProjection {
    let nodes = vec![
        node(
            101,
            "Durable engineering history",
            "RequirementDefinition",
            None,
        ),
        node(
            102,
            "Revision-bound agent edits",
            "RequirementDefinition",
            None,
        ),
        node(
            103,
            "Responsive spatial navigation",
            "RequirementDefinition",
            None,
        ),
        node(21, "ModelRepository", "PartUsage", None),
        node(22, "ModelingService", "PartUsage", None),
        node(11, "SystemWorld", "PartUsage", None),
        node(111, "Crash recovery acceptance", "ActionUsage", None),
        node(112, "Candidate CAS acceptance", "ActionUsage", None),
        node(113, "Viewport frame budget", "ActionUsage", None),
    ];
    let edges = vec![
        edge(1, 21, 101, RelationshipFamily::Requirement, "satisfies"),
        edge(2, 22, 102, RelationshipFamily::Requirement, "satisfies"),
        edge(3, 11, 103, RelationshipFamily::Requirement, "satisfies"),
        edge(4, 111, 101, RelationshipFamily::Verification, "verifies"),
        edge(5, 112, 102, RelationshipFamily::Verification, "verifies"),
        edge(6, 113, 103, RelationshipFamily::Verification, "verifies"),
    ];
    projection(
        "Requirement knowledge graph · visual fixture",
        ViewKind::Requirements,
        nodes,
        edges,
    )
}
/// Local/ring plus column-neighbor topology with exact requested counts.
pub fn stress(node_count: usize, edge_count: usize) -> ViewProjection {
    let nodes = (0..node_count)
        .map(|i| {
            node(
                i as u128 + 1,
                &format!("Subsystem_{i:05}"),
                "PartUsage",
                None,
            )
        })
        .collect();
    let width = (node_count as f32).sqrt().ceil() as usize;
    let edges = if node_count == 0 {
        vec![]
    } else {
        (0..edge_count)
            .map(|i| {
                let source = i % node_count;
                let offset = if i < node_count { 1 } else { width.max(1) };
                edge(
                    i as u128,
                    source as u128 + 1,
                    ((source + offset) % node_count) as u128 + 1,
                    RelationshipFamily::Connection,
                    "channel",
                )
            })
            .collect()
    };
    projection(
        &format!("{node_count} nodes / {edge_count} edges · VISUAL FIXTURE"),
        ViewKind::SemanticGraph,
        nodes,
        edges,
    )
}
/// A comparison fixture, never a candidate or a durable repository commit.
pub fn revision_diff() -> (ViewProjection, ViewProjection) {
    let before = architecture();
    let mut after = before.clone();
    after.revision_id = ProjectRevisionId::from_u128(FIXTURE_REVISION + 1);
    for n in &mut after.nodes {
        n.revision_id = after.revision_id;
        if n.id == id(22) {
            n.name = "RevisionModelingService".into();
        }
    }
    for e in &mut after.edges {
        e.revision_id = after.revision_id;
    }
    let mut added = node(24, "CandidateCoordinator", "PartUsage", Some(2));
    added.revision_id = after.revision_id;
    after.nodes.push(added);
    after.nodes.retain(|n| n.id != id(13));
    after
        .edges
        .retain(|e| e.source != id(1302) && e.target != id(1301));
    let mut children: BTreeMap<ElementId, Vec<ElementId>> = BTreeMap::new();
    for node in &after.nodes {
        if let Some(owner) = node.owner {
            children.entry(owner).or_default().push(node.id);
        }
    }
    // The fixture's group summaries must describe its edited projection. These
    // are fixture records only, not a semantic reconstruction or validation.
    for node in &mut after.nodes {
        if let Some(children) = children.get(&node.id) {
            node.counts.parts = children.len();
        }
    }
    after.groups = children
        .into_iter()
        .map(|(element_id, children)| ViewGroup {
            element_id,
            children,
        })
        .collect();
    after.metadata.local_element_count = after.nodes.len();
    after.view.name = "Revision comparison · visual fixture".into();
    (before, after)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_fixture_updates_owned_group_summaries() {
        let (before, after) = revision_diff();
        assert_eq!(
            before
                .nodes
                .iter()
                .find(|n| n.id == id(1))
                .unwrap()
                .counts
                .parts,
            3
        );
        assert_eq!(
            after
                .nodes
                .iter()
                .find(|n| n.id == id(1))
                .unwrap()
                .counts
                .parts,
            2
        );
        assert_eq!(
            after
                .nodes
                .iter()
                .find(|n| n.id == id(2))
                .unwrap()
                .counts
                .parts,
            4
        );
        assert!(
            !after
                .groups
                .iter()
                .flat_map(|g| &g.children)
                .any(|child| *child == id(13))
        );
        assert!(
            after
                .groups
                .iter()
                .find(|g| g.element_id == id(2))
                .unwrap()
                .children
                .contains(&id(24))
        );
    }
}
