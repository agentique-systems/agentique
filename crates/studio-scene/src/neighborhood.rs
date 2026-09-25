use crate::NodeCategory;
use agq_kernel::ElementId;
use agq_modeling_view::{RelationshipFamily, ViewProjection};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeighborhoodDirection {
    Incoming,
    Outgoing,
    Both,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Neighborhood {
    /// Includes real semantic ports and their visible owners, when supplied by
    /// the projection. No omitted standard library population is invented.
    pub elements: BTreeSet<ElementId>,
    pub relationships: BTreeSet<String>,
}
/// One deliberate hop in the supplied revision-bound projection. This never
/// queries hidden model data, mutates semantic ownership or follows an evolving
/// seed set accidentally. Callers may request another hop explicitly.
pub fn expand_neighborhood(
    projection: &ViewProjection,
    seeds: &BTreeSet<ElementId>,
    families: &BTreeSet<RelationshipFamily>,
    direction: NeighborhoodDirection,
) -> Neighborhood {
    let mut owners = BTreeMap::new();
    for n in &projection.nodes {
        if NodeCategory::from_semantic_kind(&n.semantic_kind) == NodeCategory::Port
            && let Some(owner) = n.owner
        {
            owners.insert(n.id, owner);
        }
        for feature in &n.features {
            if NodeCategory::from_semantic_kind(&feature.semantic_kind) == NodeCategory::Port {
                owners.insert(feature.id, n.id);
            }
        }
    }
    let mut expanded_seeds = seeds.clone();
    for (port, owner) in &owners {
        if seeds.contains(owner) || seeds.contains(port) {
            expanded_seeds.extend([*port, *owner]);
        }
    }
    let mut result = Neighborhood {
        elements: expanded_seeds.clone(),
        relationships: BTreeSet::new(),
    };
    for edge in &projection.edges {
        if !families.contains(&edge.family) {
            continue;
        }
        let outgoing = (direction != NeighborhoodDirection::Incoming || !edge.directed)
            && expanded_seeds.contains(&edge.source);
        let incoming = (direction != NeighborhoodDirection::Outgoing || !edge.directed)
            && expanded_seeds.contains(&edge.target);
        if outgoing || incoming {
            result.relationships.insert(edge.id.clone());
            result.elements.extend([edge.source, edge.target]);
            for endpoint in [edge.source, edge.target] {
                if let Some(owner) = owners.get(&endpoint) {
                    result.elements.insert(*owner);
                }
            }
        }
    }
    result
}
