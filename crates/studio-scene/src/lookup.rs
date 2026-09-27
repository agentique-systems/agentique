use crate::{Scene, SceneEdge, SceneNode, ScenePort, SceneTarget};
use agq_language::ElementId;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

/// Identity-to-position accelerator for one disposable scene generation.
///
/// Rebuild after every layout. It stores
/// indices, never a second copy of presentation or semantic records. Accessors
/// validate revision and identity before borrowing, so a stale index cannot
/// return an unrelated object or panic after a scene shrinks.
#[derive(Clone, Debug)]
pub struct SceneLookup {
    generation: u64,
    nodes: HashMap<ElementId, usize>,
    ports: HashMap<(ElementId, ElementId), usize>,
    edges: HashMap<String, usize>,
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
    pub fn build(scene: &Scene) -> Self {
        Self {
            generation: scene.generation,
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
                .map(|(i, p)| ((p.owner, p.id), i))
                .collect(),
            edges: scene
                .edges
                .iter()
                .enumerate()
                .map(|(i, e)| (e.semantic.id.clone(), i))
                .collect(),
        }
    }
    pub fn node<'scene>(&self, scene: &'scene Scene, id: ElementId) -> Option<&'scene SceneNode> {
        if scene.generation != self.generation {
            return None;
        }
        scene
            .nodes
            .get(*self.nodes.get(&id)?)
            .filter(|n| n.id() == id)
    }
    /// The port `id` shown on the card `owner`.
    pub fn port<'scene>(
        &self,
        scene: &'scene Scene,
        owner: ElementId,
        id: ElementId,
    ) -> Option<&'scene ScenePort> {
        if scene.generation != self.generation {
            return None;
        }
        scene
            .ports
            .get(*self.ports.get(&(owner, id))?)
            .filter(|p| p.id == id && p.owner == owner)
    }
    pub fn edge<'scene>(&self, scene: &'scene Scene, id: &str) -> Option<&'scene SceneEdge> {
        if scene.generation != self.generation {
            return None;
        }
        scene
            .edges
            .get(*self.edges.get(id)?)
            .filter(|e| e.semantic.id == id)
    }
    /// O(V log V) ordering of visible targets, independent of total scene size.
    /// Hash lookups resolve identity; index sorting preserves containment order.
    pub fn visible<'scene, 'target>(
        &self,
        scene: &'scene Scene,
        targets: impl IntoIterator<Item = &'target SceneTarget>,
    ) -> VisibleScene<'scene> {
        if scene.generation != self.generation {
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
                SceneTarget::Port(owner, id) => {
                    if let Some(index) = self.ports.get(&(*owner, *id))
                        && scene
                            .ports
                            .get(*index)
                            .is_some_and(|p| p.id == *id && p.owner == *owner)
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
