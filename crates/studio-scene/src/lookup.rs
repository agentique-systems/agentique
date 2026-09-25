use crate::{SceneEdge, SceneNode, ScenePort, SceneTarget, SemanticScene};
use agq_kernel::ElementId;
use agq_modeling_workspace::ProjectRevisionId;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

/// Identity-to-position accelerator for one disposable scene generation.
///
/// Rebuild after layout, projection replacement or applying a diff. It stores
/// indices, never a second copy of presentation or semantic records. Accessors
/// validate revision and identity before borrowing, so a stale index cannot
/// return an unrelated object or panic after a projection shrinks.
#[derive(Clone, Debug)]
pub struct SceneLookup {
    revision_id: ProjectRevisionId,
    nodes: HashMap<ElementId, usize>,
    ports: HashMap<ElementId, usize>,
    edges: HashMap<String, usize>,
    port_owners: HashMap<ElementId, ElementId>,
}
/// Culled borrowed records in the scene's original draw order. Parents precede
/// their children. Only visible targets are resolved; no full-scene scan occurs.
#[derive(Debug, Default)]
pub struct VisibleScene<'scene> {
    pub nodes: Vec<&'scene SceneNode>,
    pub ports: Vec<&'scene ScenePort>,
    pub edges: Vec<&'scene SceneEdge>,
}
/// Hash only visible identities in original draw order. A renderer must combine
/// this with its scene generation and rendering options: identity alone is not
/// a geometry fingerprint or an authorization to reuse another revision.
impl Hash for VisibleScene<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.nodes.len().hash(state);
        for node in &self.nodes {
            node.id().hash(state);
            node.is_container.hash(state);
        }
        self.ports.len().hash(state);
        for port in &self.ports {
            port.id.hash(state);
        }
        self.edges.len().hash(state);
        for edge in &self.edges {
            edge.semantic.id.hash(state);
        }
    }
}
impl SceneLookup {
    pub fn build(scene: &SemanticScene) -> Self {
        Self {
            revision_id: scene.revision_id,
            nodes: scene
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.id(), i))
                .collect(),
            ports: scene
                .ports
                .iter()
                .enumerate()
                .map(|(i, p)| (p.id, i))
                .collect(),
            edges: scene
                .edges
                .iter()
                .enumerate()
                .map(|(i, e)| (e.semantic.id.clone(), i))
                .collect(),
            port_owners: scene.ports.iter().map(|p| (p.id, p.owner)).collect(),
        }
    }
    pub fn node<'scene>(
        &self,
        scene: &'scene SemanticScene,
        id: ElementId,
    ) -> Option<&'scene SceneNode> {
        if scene.revision_id != self.revision_id {
            return None;
        }
        scene
            .nodes
            .get(*self.nodes.get(&id)?)
            .filter(|n| n.id() == id)
    }
    pub fn port<'scene>(
        &self,
        scene: &'scene SemanticScene,
        id: ElementId,
    ) -> Option<&'scene ScenePort> {
        if scene.revision_id != self.revision_id {
            return None;
        }
        scene
            .ports
            .get(*self.ports.get(&id)?)
            .filter(|p| p.id == id)
    }
    pub fn edge<'scene>(
        &self,
        scene: &'scene SemanticScene,
        id: &str,
    ) -> Option<&'scene SceneEdge> {
        if scene.revision_id != self.revision_id {
            return None;
        }
        scene
            .edges
            .get(*self.edges.get(id)?)
            .filter(|e| e.semantic.id == id)
    }
    /// Resolve a semantic port to its displayed owner in expected O(1).
    /// Other endpoints keep their original identity, including omitted records.
    pub fn endpoint_owner(&self, id: ElementId) -> ElementId {
        self.port_owners.get(&id).copied().unwrap_or(id)
    }
    /// O(V log V) ordering of visible targets, independent of total scene size.
    /// Hash lookups resolve identity; index sorting preserves containment order.
    pub fn visible<'scene, 'target>(
        &self,
        scene: &'scene SemanticScene,
        targets: impl IntoIterator<Item = &'target SceneTarget>,
    ) -> VisibleScene<'scene> {
        if scene.revision_id != self.revision_id {
            return VisibleScene::default();
        }
        let mut nodes = Vec::new();
        let mut ports = Vec::new();
        let mut edges = Vec::new();
        for target in targets {
            match target {
                SceneTarget::Node(id) | SceneTarget::Container(id) => {
                    if let Some(index) = self.nodes.get(id)
                        && scene.nodes.get(*index).is_some_and(|n| n.id() == *id)
                    {
                        nodes.push(*index);
                    }
                }
                SceneTarget::Port(id) => {
                    if let Some(index) = self.ports.get(id)
                        && scene.ports.get(*index).is_some_and(|p| p.id == *id)
                    {
                        ports.push(*index);
                    }
                }
                SceneTarget::Edge(id) => {
                    if let Some(index) = self.edges.get(id)
                        && scene
                            .edges
                            .get(*index)
                            .is_some_and(|e| e.semantic.id == *id)
                    {
                        edges.push(*index);
                    }
                }
            }
        }
        for indices in [&mut nodes, &mut ports, &mut edges] {
            indices.sort_unstable();
            indices.dedup();
        }
        VisibleScene {
            nodes: nodes.into_iter().map(|i| &scene.nodes[i]).collect(),
            ports: ports.into_iter().map(|i| &scene.ports[i]).collect(),
            edges: edges.into_iter().map(|i| &scene.edges[i]).collect(),
        }
    }
}
