//! The Surface's scene: layout, routing, hit testing and culling for the model.
//!
//! The Studio describes what to draw as a small [`SceneInput`] (built from the
//! element tree by [`SceneInput::from_tree`]); this crate turns it into
//! positioned nodes, ports and routed edges. Geometry, collapsed groups, camera
//! and change marks are presentation only: nothing here changes the model.
#![forbid(unsafe_code)]
mod camera;
pub mod fixtures;
mod geometry;
mod graph_layout;
mod input;
mod layout;
mod lookup;
mod requirements;
mod routing;
mod spatial;
pub use agq_language::ElementId;
pub use camera::*;
pub use geometry::*;
pub use graph_layout::*;
pub use input::*;
pub use layout::*;
pub use lookup::*;
pub use requirements::*;
pub use routing::*;
pub use spatial::*;
use std::collections::{BTreeMap, BTreeSet};

/// How a node is drawn. Follows the SysML kind of the element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeCategory {
    Package,
    /// A part, item, port, connection or interface definition.
    Definition,
    Part,
    Item,
    Attribute,
    Requirement,
}
impl NodeCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Package => "PACKAGE",
            Self::Definition => "DEFINITION",
            Self::Part => "PART",
            Self::Item => "ITEM",
            Self::Attribute => "ATTRIBUTE",
            Self::Requirement => "REQUIREMENT",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DiffMark {
    #[default]
    Unchanged,
    Added,
    Removed,
    Changed,
}
/// What is selected on the Surface, by identity, never by position.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SceneTarget {
    Node(ElementId),
    /// A port shown on a card: (card, port). The same port can be shown on
    /// several cards, such as on its definition and on usages of it.
    Port(ElementId, ElementId),
    Edge(String),
    Container(ElementId),
}
impl SceneTarget {
    /// The model element: the card, the port, or none for an edge.
    pub fn element_id(&self) -> Option<ElementId> {
        match self {
            Self::Node(id) | Self::Port(_, id) | Self::Container(id) => Some(*id),
            Self::Edge(_) => None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub semantic: InputNode,
    pub category: NodeCategory,
    /// A definition that owns no cards: shown compact, after the structure.
    pub secondary: bool,
    pub bounds: Rect,
    pub depth: usize,
    pub is_container: bool,
    pub collapsed: bool,
    pub diff: DiffMark,
}
impl SceneNode {
    pub fn id(&self) -> ElementId {
        self.semantic.id
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortSide {
    Left,
    Right,
    Top,
    Bottom,
}
#[derive(Clone, Debug)]
pub struct ScenePort {
    pub id: ElementId,
    pub owner: ElementId,
    /// The port's real owner when it is shown on the boundary of a collapsed
    /// container that owns it indirectly. The port id is always the real one.
    pub proxy_for_owner: Option<ElementId>,
    pub name: String,
    pub position: Point,
    /// The label sits in the header of an expanded container.
    pub label_in_header: bool,
    pub side: PortSide,
    pub direction: PortDirection,
    pub lock: LockMark,
    /// The definition that owns the port when the card shows it through its type.
    pub defined_in: Option<ElementId>,
    pub diff: DiffMark,
}
#[derive(Clone, Debug)]
pub struct SceneEdge {
    pub semantic: InputEdge,
    pub points: Vec<Point>,
    pub bounds: Rect,
    pub quality: RouteQuality,
    pub diff: DiffMark,
    /// What the route was made from, so an update can keep it.
    ends: RouteEnds,
}
#[derive(Clone, Debug)]
pub struct SceneContainer {
    pub element_id: ElementId,
    pub bounds: Rect,
    pub children: Vec<ElementId>,
    pub collapsed: bool,
}
/// Which layout arranges the scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutKind {
    /// Containers hold what they own.
    #[default]
    Hierarchy,
    /// Layered by the edges between elements; ownership is not shown as nesting.
    Graph,
    /// Requirements, what satisfies them and their subjects, in lanes.
    Requirements,
}
/// How many edges a scene routed, and how many it kept from the scene it was
/// updated from ([`Scene::update`]). A built scene routes every edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Routing {
    pub routed: usize,
    pub kept: usize,
}
#[derive(Clone, Debug, Default)]
pub struct SceneOptions {
    pub collapsed: BTreeSet<ElementId>,
    /// Show only this element and what it owns.
    pub focus: Option<ElementId>,
    pub layout: LayoutKind,
}
#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error(transparent)]
    GraphPin(#[from] PinError),
    #[error("the input contains element {0} twice")]
    DuplicateElement(ElementId),
    #[error("the input contains an ownership cycle at {0}")]
    OwnershipCycle(ElementId),
    #[error("the input contains edge {0} twice")]
    DuplicateEdge(String),
    #[error("layout returned missing or nonfinite geometry for {0}")]
    InvalidGeometry(ElementId),
}
#[derive(Clone, Debug)]
pub struct Scene {
    /// Copied from the input; lookups and indexes check it.
    pub generation: u64,
    pub nodes: Vec<SceneNode>,
    pub ports: Vec<ScenePort>,
    pub edges: Vec<SceneEdge>,
    pub containers: Vec<SceneContainer>,
    pub warnings: Vec<String>,
    bounds: Rect,
    memory: LayoutMemory,
    routing: Routing,
}
/// The layout `options` choose.
fn layout_for(input: &SceneInput, options: &SceneOptions) -> Box<dyn LayoutEngine> {
    match options.layout {
        LayoutKind::Requirements => Box::new(RequirementsLayout::new(input)),
        LayoutKind::Hierarchy => Box::new(HierarchyLayout::default()),
        LayoutKind::Graph => Box::new(GraphLayout::default()),
    }
}
impl Scene {
    /// Lays out every card and routes every edge. Cards keep their places
    /// from `previous` where they still fit.
    pub fn build(
        input: &SceneInput,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
    ) -> Result<Self, SceneError> {
        Self::with_layout(input, options, previous, &*layout_for(input, options))
    }
    /// The scene for `input` after a change, made from this one. Cards are
    /// placed exactly as [`Scene::build`] places them with `previous`
    /// (normally this scene's memory), which is cheap; routing is not, so
    /// only the edges the change touches are routed again. An edge keeps
    /// its route when its ends and lane are unchanged and the route crosses
    /// neither the old nor the new place of a card that was added, removed,
    /// moved or resized. [`Scene::routing`] says how many were kept.
    ///
    /// One deliberate difference from a build: a kept route may still go
    /// around a card that has since moved away, where a build would now
    /// find a shorter one. Routes stay put, like cards.
    pub fn update(
        &self,
        input: &SceneInput,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
    ) -> Result<Self, SceneError> {
        Self::lay_out(
            input,
            options,
            previous,
            &*layout_for(input, options),
            Some(self),
        )
    }
    pub fn with_layout(
        input: &SceneInput,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
        layout: &dyn LayoutEngine,
    ) -> Result<Self, SceneError> {
        Self::lay_out(input, options, previous, layout, None)
    }
    fn lay_out(
        input: &SceneInput,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
        layout: &dyn LayoutEngine,
        earlier: Option<&Scene>,
    ) -> Result<Self, SceneError> {
        let mut seen = BTreeSet::new();
        for n in &input.nodes {
            if !seen.insert(n.id) {
                return Err(SceneError::DuplicateElement(n.id));
            }
        }
        let mut edge_ids = BTreeSet::new();
        for e in &input.edges {
            if !edge_ids.insert(&e.id) {
                return Err(SceneError::DuplicateEdge(e.id.clone()));
            }
        }
        let layout_input = LayoutInput::from_input(input, options)?;
        let result = layout.layout(&layout_input, previous);
        if let Some(error) = result.pin_error {
            return Err(error.into());
        }
        let mut nodes = Vec::with_capacity(layout_input.nodes.len());
        for n in &layout_input.nodes {
            let bounds = *result
                .bounds
                .get(&n.id)
                .ok_or(SceneError::InvalidGeometry(n.id))?;
            if !bounds.finite() {
                return Err(SceneError::InvalidGeometry(n.id));
            }
            let has_children = layout_input
                .children
                .get(&n.id)
                .is_some_and(|v| !v.is_empty());
            nodes.push(SceneNode {
                semantic: n.clone(),
                category: n.kind,
                secondary: layout::is_secondary(n, has_children),
                bounds,
                depth: layout_input.depths[&n.id],
                is_container: layout_input
                    .children
                    .get(&n.id)
                    .is_some_and(|v| !v.is_empty()),
                collapsed: options.collapsed.contains(&n.id),
                diff: DiffMark::Unchanged,
            });
        }
        // Parents precede children so renderers can draw containment.
        nodes.sort_by_key(|n| (n.depth, n.id()));
        let mut child_top = BTreeMap::<ElementId, f32>::new();
        for node in &nodes {
            if let Some(owner) = node.semantic.owner {
                child_top
                    .entry(owner)
                    .and_modify(|y| *y = y.min(node.bounds.min.y))
                    .or_insert(node.bounds.min.y);
            }
        }
        let mut ports = Vec::new();
        let mut port_ids = BTreeSet::new();
        for n in &nodes {
            let mut features: Vec<_> = n.semantic.ports.iter().map(|p| (p.clone(), None)).collect();
            features.extend(
                layout_input
                    .boundary_ports
                    .iter()
                    .filter(|p| p.owner == n.id())
                    .map(|p| (p.port.clone(), Some(p.semantic_owner))),
            );
            features.sort_by_key(|(port, _)| port.id);
            features.dedup_by_key(|(port, _)| port.id);
            let count = features.len();
            let label_in_header = n.is_container
                && !n.collapsed
                && layout::port_strip_header(count).is_some_and(|header| {
                    child_top
                        .get(&n.id())
                        .is_some_and(|top| *top >= n.bounds.min.y + header)
                });
            for (i, (port, proxy_for_owner)) in features.into_iter().enumerate() {
                let id = port.id;
                if !port_ids.insert((n.id(), id)) {
                    continue;
                }
                let side = if i % 2 == 0 {
                    PortSide::Left
                } else {
                    PortSide::Right
                };
                let y = n.bounds.min.y
                    + if label_in_header {
                        layout::PORT_STRIP_FIRST + layout::PORT_STRIP_ROW * (i / 2) as f32
                    } else {
                        62.0 + (n.bounds.height()
                            - layout::feature_block(n.semantic.features.len())
                            - 82.0)
                            .max(12.0)
                            * ((i / 2 + 1) as f32 / ((count.div_ceil(2) + 1) as f32))
                    };
                let x = if side == PortSide::Left {
                    n.bounds.min.x
                } else {
                    n.bounds.max.x
                };
                ports.push(ScenePort {
                    id,
                    owner: n.id(),
                    proxy_for_owner,
                    name: port.name,
                    position: Point::new(x, y),
                    label_in_header,
                    side,
                    direction: port.direction,
                    lock: port.lock,
                    defined_in: port.defined_in,
                    diff: DiffMark::Unchanged,
                });
            }
        }
        let containers = nodes
            .iter()
            .filter(|n| n.is_container)
            .map(|n| SceneContainer {
                element_id: n.id(),
                bounds: n.bounds,
                children: layout_input.children[&n.id()].clone(),
                collapsed: n.collapsed,
            })
            .collect();
        let (edges, routed) =
            route_edges(&nodes, &ports, &input.edges, &layout_input.hidden, earlier);
        let routing = Routing {
            routed,
            kept: edges.len() - routed,
        };
        let bounds = nodes
            .iter()
            .map(|n| n.bounds)
            .chain(edges.iter().map(|e| e.bounds))
            .reduce(Rect::union)
            .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
        let mut warnings = Vec::new();
        let obstructed = edges
            .iter()
            .filter(|e| e.quality == RouteQuality::Obstructed)
            .count();
        if obstructed > 0 {
            warnings.push(format!(
                "{obstructed} routes cross other elements; the router could not find a clear path"
            ));
        }
        let mut memory = previous.cloned().unwrap_or_default();
        memory.bounds.extend(result.bounds);
        Ok(Self {
            generation: input.generation,
            nodes,
            ports,
            edges,
            containers,
            warnings,
            bounds,
            memory,
            routing,
        })
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn routing(&self) -> Routing {
        self.routing
    }
    /// The bounds of the structure: every card except packages and secondary
    /// definitions. The whole scene when there is no structure.
    pub fn structure_bounds(&self) -> Rect {
        self.nodes
            .iter()
            .filter(|n| !n.secondary && n.category != NodeCategory::Package)
            .map(|n| n.bounds)
            .reduce(Rect::union)
            .unwrap_or(self.bounds)
    }
    pub fn memory(&self) -> &LayoutMemory {
        &self.memory
    }
    pub fn node(&self, id: ElementId) -> Option<&SceneNode> {
        self.nodes.iter().find(|n| n.id() == id)
    }
    pub fn target_bounds(&self, target: &SceneTarget) -> Option<Rect> {
        match target {
            SceneTarget::Node(id) | SceneTarget::Container(id) => self.node(*id).map(|n| n.bounds),
            SceneTarget::Port(owner, id) => self
                .ports
                .iter()
                .find(|p| p.id == *id && p.owner == *owner)
                .map(|p| Rect::new(p.position.x - 7.0, p.position.y - 7.0, 14.0, 14.0)),
            SceneTarget::Edge(id) => self
                .edges
                .iter()
                .find(|e| e.semantic.id == *id)
                .map(|e| e.bounds),
        }
    }
    /// A scene that shows what differs between two versions of the model:
    /// the later version with added and changed elements marked, and the
    /// removed ones kept, marked, where they were. Removed cards are laid
    /// out together with the current ones, so they never overlap them.
    /// `changed` names elements known to differ in ways their card does not
    /// show (for example an attribute of a part); they are marked changed.
    pub fn comparison(
        before: &SceneInput,
        after: &SceneInput,
        changed: &BTreeSet<ElementId>,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
    ) -> Result<Self, SceneError> {
        let old_nodes: BTreeMap<_, _> = before.nodes.iter().map(|n| (n.id, n)).collect();
        let new_nodes: BTreeMap<_, _> = after.nodes.iter().map(|n| (n.id, n)).collect();
        let mut merged = after.clone();
        // Removed ports stay on their cards.
        for node in &mut merged.nodes {
            if let Some(old) = old_nodes.get(&node.id) {
                for port in &old.ports {
                    if !node.ports.iter().any(|p| p.id == port.id) {
                        node.ports.push(port.clone());
                    }
                }
            }
        }
        merged.nodes.extend(
            before
                .nodes
                .iter()
                .filter(|n| !new_nodes.contains_key(&n.id))
                .cloned(),
        );
        let new_edges: BTreeMap<_, _> = after.edges.iter().map(|e| (&e.id, e)).collect();
        let old_edges: BTreeMap<_, _> = before.edges.iter().map(|e| (&e.id, e)).collect();
        merged.edges.extend(
            before
                .edges
                .iter()
                .filter(|e| !new_edges.contains_key(&e.id))
                .cloned(),
        );
        let mut scene = Self::build(&merged, options, previous)?;
        for node in &mut scene.nodes {
            node.diff = match (old_nodes.get(&node.id()), new_nodes.get(&node.id())) {
                (None, _) => DiffMark::Added,
                (Some(_), None) => DiffMark::Removed,
                (Some(old), Some(new)) if old != new || changed.contains(&node.id()) => {
                    DiffMark::Changed
                }
                _ => DiffMark::Unchanged,
            };
        }
        let ports_of = |nodes: &BTreeMap<ElementId, &InputNode>, card: ElementId| {
            nodes
                .get(&card)
                .map(|n| n.ports.clone())
                .unwrap_or_default()
        };
        for port in &mut scene.ports {
            let card = port.proxy_for_owner.unwrap_or(port.owner);
            let old = ports_of(&old_nodes, card);
            let new = ports_of(&new_nodes, card);
            port.diff = match (
                old.iter().find(|p| p.id == port.id),
                new.iter().find(|p| p.id == port.id),
            ) {
                (None, _) => DiffMark::Added,
                (Some(_), None) => DiffMark::Removed,
                (Some(a), Some(b)) if a != b || changed.contains(&port.id) => DiffMark::Changed,
                _ => DiffMark::Unchanged,
            };
        }
        for edge in &mut scene.edges {
            edge.diff = match (
                old_edges.get(&edge.semantic.id),
                new_edges.get(&edge.semantic.id),
            ) {
                (None, _) => DiffMark::Added,
                (Some(_), None) => DiffMark::Removed,
                (Some(a), Some(b))
                    if a != b || b.element.is_some_and(|id| changed.contains(&id)) =>
                {
                    DiffMark::Changed
                }
                _ => DiffMark::Unchanged,
            };
        }
        Ok(scene)
    }
}
