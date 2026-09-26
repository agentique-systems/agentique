//! Native Studio's disposable spatial projection, never a semantic model.
//!
//! Every identity is supplied by a revision-bound [`ViewProjection`]. Geometry,
//! collapsed groups, camera and diff marks are presentation state. This crate has
//! no repository writes, language producer access or renderer dependency.
#![forbid(unsafe_code)]
mod camera;
pub mod fixtures;
mod geometry;
mod graph_layout;
mod layout;
mod lookup;
mod neighborhood;
mod routing;
mod spatial;
use agq_kernel::ElementId;
use agq_modeling_view::{ViewEdge, ViewNode, ViewOrigin, ViewProjection};
use agq_modeling_workspace::ProjectRevisionId;
pub use camera::*;
pub use geometry::*;
pub use graph_layout::*;
pub use layout::*;
pub use lookup::*;
pub use neighborhood::*;
pub use routing::*;
pub use spatial::*;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit visual grammar. Unknown metaclasses remain visibly generic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeCategory {
    System,
    Part,
    Port,
    Interface,
    Requirement,
    Action,
    State,
    Agent,
    Package,
    Feature,
    Unknown,
}
impl NodeCategory {
    /// A transport adapter over exact public metaclass names, never substring
    /// guesses. A future typed projection field can replace this one boundary.
    pub fn from_semantic_kind(kind: &str) -> Self {
        match kind {
            "PartDefinition" | "ItemDefinition" | "Class" => Self::System,
            "PartUsage" | "ItemUsage" => Self::Part,
            "PortDefinition" | "PortUsage" => Self::Port,
            "InterfaceDefinition"
            | "InterfaceUsage"
            | "ConnectionDefinition"
            | "ConnectionUsage"
            | "Connector" => Self::Interface,
            "RequirementDefinition" | "RequirementUsage" | "RequirementConstraintMembership" => {
                Self::Requirement
            }
            "ActionDefinition" | "ActionUsage" | "CalculationDefinition" | "CalculationUsage" => {
                Self::Action
            }
            "StateDefinition" | "StateUsage" | "TransitionUsage" => Self::State,
            "Package" | "Namespace" => Self::Package,
            "Feature" | "AttributeUsage" | "AttributeDefinition" => Self::Feature,
            _ => Self::Unknown,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "DEFINITION",
            Self::Part => "PART",
            Self::Port => "PORT",
            Self::Interface => "INTERFACE",
            Self::Requirement => "REQUIREMENT",
            Self::Action => "ACTION",
            Self::State => "STATE",
            Self::Agent => "AGENT",
            Self::Package => "PACKAGE",
            Self::Feature => "FEATURE",
            Self::Unknown => "ELEMENT",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffMark {
    #[default]
    Unchanged,
    Added,
    Removed,
    Changed,
}
/// Selection identity remains independent of screen coordinates and layout.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum SceneTarget {
    Node(ElementId),
    Port(ElementId),
    Edge(String),
    Container(ElementId),
}
impl SceneTarget {
    pub fn element_id(&self) -> Option<ElementId> {
        match self {
            Self::Node(id) | Self::Port(id) | Self::Container(id) => Some(*id),
            Self::Edge(_) => None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SceneNode {
    pub semantic: ViewNode,
    pub category: NodeCategory,
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
/// No direction is fabricated from layout or canonical endpoint order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDirection {
    Unspecified,
    In,
    Out,
    InOut,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortSide {
    Left,
    Right,
    Top,
    Bottom,
}
#[derive(Clone, Debug)]
pub struct ScenePort {
    pub id: ElementId,
    pub revision_id: ProjectRevisionId,
    pub owner: ElementId,
    /// Original semantic owner when this is a boundary proxy on a collapsed
    /// subsystem. The port ID and revision still identify the actual port.
    pub proxy_for_owner: Option<ElementId>,
    pub name: String,
    pub position: Point,
    /// Disposable label slot within a clear, bounded expanded-container header.
    pub label_in_header: bool,
    pub side: PortSide,
    pub direction: PortDirection,
    pub origin: ViewOrigin,
    pub diff: DiffMark,
}
#[derive(Clone, Debug)]
pub struct SceneEdge {
    pub semantic: ViewEdge,
    pub points: Vec<Point>,
    pub bounds: Rect,
    pub quality: RouteQuality,
    pub diff: DiffMark,
}
#[derive(Clone, Debug)]
pub struct SceneContainer {
    pub element_id: ElementId,
    pub bounds: Rect,
    pub children: Vec<ElementId>,
    pub collapsed: bool,
}
/// An agent overlay cannot change canonical state. The shell owns its lifetime.
#[derive(Clone, Debug)]
pub struct SceneOverlay {
    pub id: String,
    pub revision_id: ProjectRevisionId,
    pub targets: Vec<SceneTarget>,
    pub label: String,
    pub kind: OverlayKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    Focus,
    Finding,
    Warning,
    Suggestion,
    RequirementImpact,
}
#[derive(Clone, Debug)]
pub struct SceneOptions {
    pub collapsed: BTreeSet<ElementId>,
    /// Presentation-only focus restricts display to this ownership subtree.
    pub focus: Option<ElementId>,
    /// False selects the topology layout, keeping semantic owners untouched.
    pub hierarchy: bool,
}
impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            collapsed: BTreeSet::new(),
            focus: None,
            hierarchy: true,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error(transparent)]
    GraphPin(#[from] PinError),
    #[error("projection contains mixed revision identities")]
    MixedRevisions,
    #[error("projection contains duplicate element identity {0}")]
    DuplicateElement(ElementId),
    #[error("projection contains an ownership cycle at {0}")]
    OwnershipCycle(ElementId),
    #[error("projection contains duplicate edge identity {0}")]
    DuplicateEdge(String),
    #[error("layout returned missing or nonfinite geometry for {0}")]
    InvalidGeometry(ElementId),
}
#[derive(Clone, Debug)]
pub struct SemanticScene {
    pub revision_id: ProjectRevisionId,
    pub nodes: Vec<SceneNode>,
    pub ports: Vec<ScenePort>,
    pub edges: Vec<SceneEdge>,
    pub containers: Vec<SceneContainer>,
    pub warnings: Vec<String>,
    bounds: Rect,
    memory: LayoutMemory,
}
impl SemanticScene {
    pub fn from_projection(
        projection: &ViewProjection,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
    ) -> Result<Self, SceneError> {
        if options.hierarchy {
            Self::with_layout(projection, options, previous, &HierarchyLayout::default())
        } else {
            Self::with_layout(projection, options, previous, &GraphLayout::default())
        }
    }
    pub fn with_layout(
        projection: &ViewProjection,
        options: &SceneOptions,
        previous: Option<&LayoutMemory>,
        layout: &dyn LayoutEngine,
    ) -> Result<Self, SceneError> {
        if projection
            .nodes
            .iter()
            .any(|n| n.revision_id != projection.revision_id)
            || projection
                .edges
                .iter()
                .any(|e| e.revision_id != projection.revision_id)
        {
            return Err(SceneError::MixedRevisions);
        }
        let mut seen = BTreeSet::new();
        for n in &projection.nodes {
            if !seen.insert(n.id) {
                return Err(SceneError::DuplicateElement(n.id));
            }
        }
        let mut edge_ids = BTreeSet::new();
        for e in &projection.edges {
            if !edge_ids.insert(&e.id) {
                return Err(SceneError::DuplicateEdge(e.id.clone()));
            }
        }
        let input = LayoutInput::from_projection(projection, options)?;
        let result = layout.layout(&input, previous);
        if let Some(error) = result.pin_error {
            return Err(error.into());
        }
        let mut nodes = Vec::with_capacity(input.nodes.len());
        for n in &input.nodes {
            let bounds = *result
                .bounds
                .get(&n.id)
                .ok_or(SceneError::InvalidGeometry(n.id))?;
            if !bounds.finite() {
                return Err(SceneError::InvalidGeometry(n.id));
            }
            nodes.push(SceneNode {
                semantic: n.clone(),
                category: NodeCategory::from_semantic_kind(&n.semantic_kind),
                bounds,
                depth: input.depths[&n.id],
                is_container: input.children.get(&n.id).is_some_and(|v| !v.is_empty()),
                collapsed: options.collapsed.contains(&n.id),
                diff: DiffMark::Unchanged,
            });
        }
        // Parents precede children so renderers can draw retained containment.
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
        let mut owned_ports: BTreeMap<ElementId, Vec<&ViewNode>> = BTreeMap::new();
        for p in &projection.nodes {
            if NodeCategory::from_semantic_kind(&p.semantic_kind) == NodeCategory::Port
                && let Some(owner) = p.owner
            {
                owned_ports.entry(owner).or_default().push(p);
            }
        }
        for n in &nodes {
            let mut features: Vec<_> = n
                .semantic
                .features
                .iter()
                .filter(|f| {
                    NodeCategory::from_semantic_kind(&f.semantic_kind) == NodeCategory::Port
                })
                .map(|f| (f.id, f.name.clone(), n.semantic.origin, None))
                .collect();
            features.extend(
                owned_ports
                    .get(&n.id())
                    .into_iter()
                    .flatten()
                    .map(|p| (p.id, p.name.clone(), p.origin, None)),
            );
            features.extend(
                input
                    .boundary_ports
                    .iter()
                    .filter(|p| p.owner == n.id())
                    .map(|p| (p.id, p.name.clone(), p.origin, Some(p.semantic_owner))),
            );
            features.sort_by_key(|(id, _, _, _)| *id);
            features.dedup_by_key(|(id, _, _, _)| *id);
            let count = features.len();
            let label_in_header = n.is_container
                && !n.collapsed
                && layout::port_strip_header(count).is_some_and(|header| {
                    child_top
                        .get(&n.id())
                        .is_some_and(|top| *top >= n.bounds.min.y + header)
                });
            for (i, (id, name, origin, proxy_for_owner)) in features.into_iter().enumerate() {
                if !port_ids.insert(id) {
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
                        62.0 + (n.bounds.height() - 82.0).max(12.0)
                            * ((i / 2 + 1) as f32 / ((count.div_ceil(2) + 1) as f32))
                    };
                let x = if side == PortSide::Left {
                    n.bounds.min.x
                } else {
                    n.bounds.max.x
                };
                ports.push(ScenePort {
                    id,
                    revision_id: projection.revision_id,
                    owner: n.id(),
                    proxy_for_owner,
                    name,
                    position: Point::new(x, y),
                    label_in_header,
                    side,
                    direction: PortDirection::Unspecified,
                    origin,
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
                children: input.children[&n.id()].clone(),
                collapsed: n.collapsed,
            })
            .collect();
        let edges = route_edges(&nodes, &ports, &projection.edges);
        let bounds = nodes
            .iter()
            .map(|n| n.bounds)
            .chain(edges.iter().map(|e| e.bounds))
            .reduce(Rect::union)
            .unwrap_or(Rect::new(0.0, 0.0, 1.0, 1.0));
        let mut warnings = projection.metadata.warnings.clone();
        let obstructed = edges
            .iter()
            .filter(|e| e.quality == RouteQuality::Obstructed)
            .count();
        if obstructed > 0 {
            warnings.push(format!("{obstructed} routes need refinement; bounded router retained explicit obstacle warnings"));
        }
        let mut memory = previous.cloned().unwrap_or_default();
        memory.bounds.extend(result.bounds);
        Ok(Self {
            revision_id: projection.revision_id,
            nodes,
            ports,
            edges,
            containers,
            warnings,
            bounds,
            memory,
        })
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
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
            SceneTarget::Port(id) => self
                .ports
                .iter()
                .find(|p| p.id == *id)
                .map(|p| Rect::new(p.position.x - 7.0, p.position.y - 7.0, 14.0, 14.0)),
            SceneTarget::Edge(id) => self
                .edges
                .iter()
                .find(|e| e.semantic.id == *id)
                .map(|e| e.bounds),
        }
    }
    /// Removed diff ghosts bind their original revision, not the active one.
    pub fn target_revision(&self, target: &SceneTarget) -> Option<ProjectRevisionId> {
        match target {
            SceneTarget::Node(id) | SceneTarget::Container(id) => {
                self.node(*id).map(|n| n.semantic.revision_id)
            }
            SceneTarget::Port(id) => self
                .ports
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.revision_id),
            SceneTarget::Edge(id) => self
                .edges
                .iter()
                .find(|e| e.semantic.id == *id)
                .map(|e| e.semantic.revision_id),
        }
    }
    /// Diff presentation retains removed objects with their original revision.
    /// Equality excludes revision ID itself, which changes for every projection.
    pub fn apply_diff(&mut self, parent: &SemanticScene) {
        let before: BTreeMap<_, _> = parent.nodes.iter().map(|n| (n.id(), n)).collect();
        let after: BTreeSet<_> = self.nodes.iter().map(SceneNode::id).collect();
        for n in &mut self.nodes {
            n.diff = match before.get(&n.id()) {
                None => DiffMark::Added,
                Some(p) if node_content_changed(&p.semantic, &n.semantic) => DiffMark::Changed,
                _ => DiffMark::Unchanged,
            };
        }
        for n in &parent.nodes {
            if !after.contains(&n.id()) {
                let mut ghost = n.clone();
                if let Some(owner) = n.semantic.owner
                    && let (Some(old), Some(new)) = (
                        before.get(&owner),
                        self.nodes.iter().find(|node| node.id() == owner),
                    )
                {
                    let dx = new.bounds.min.x - old.bounds.min.x;
                    let dy = new.bounds.min.y - old.bounds.min.y;
                    ghost.bounds = ghost.bounds.translate(Point::new(dx, dy));
                }
                ghost.diff = DiffMark::Removed;
                self.nodes.push(ghost);
            }
        }
        // A comparison keeps the earlier containment envelope around removed
        // ghosts. Shrinking it would visually detach a removed child from its
        // original system even though that ownership has not changed.
        for node in &mut self.nodes {
            if node.is_container
                && let Some(old) = before.get(&node.id())
                && old.is_container
            {
                // Compare extents in the owner's current coordinate frame.
                // Unioning old absolute positions creates a giant overlapping
                // container whenever collision avoidance moves a subsystem.
                node.bounds = Rect::new(
                    node.bounds.min.x,
                    node.bounds.min.y,
                    node.bounds.width().max(old.bounds.width()),
                    node.bounds.height().max(old.bounds.height()),
                );
            }
        }
        let node_bounds: BTreeMap<_, _> = self.nodes.iter().map(|n| (n.id(), n.bounds)).collect();
        for container in &mut self.containers {
            if let Some(bounds) = node_bounds.get(&container.element_id) {
                container.bounds = *bounds;
            }
        }
        let before_ports: BTreeMap<_, _> = parent.ports.iter().map(|p| (p.id, p)).collect();
        let after_ports: BTreeSet<_> = self.ports.iter().map(|p| p.id).collect();
        for p in &mut self.ports {
            p.diff = match before_ports.get(&p.id) {
                None => DiffMark::Added,
                Some(old) if old.name != p.name || old.owner != p.owner => DiffMark::Changed,
                _ => DiffMark::Unchanged,
            };
        }
        for p in &parent.ports {
            if !after_ports.contains(&p.id) {
                let mut ghost = p.clone();
                if let (Some(old), Some(new)) = (before.get(&p.owner), node_bounds.get(&p.owner)) {
                    ghost.position.x += new.min.x - old.bounds.min.x;
                    ghost.position.y += new.min.y - old.bounds.min.y;
                }
                ghost.diff = DiffMark::Removed;
                self.ports.push(ghost);
            }
        }
        let old_edges: BTreeMap<_, _> = parent.edges.iter().map(|e| (&e.semantic.id, e)).collect();
        let new_edges: BTreeSet<_> = self.edges.iter().map(|e| e.semantic.id.clone()).collect();
        for e in &mut self.edges {
            e.diff = match old_edges.get(&e.semantic.id) {
                None => DiffMark::Added,
                Some(old) if edge_content_changed(&old.semantic, &e.semantic) => DiffMark::Changed,
                _ => DiffMark::Unchanged,
            };
        }
        for e in &parent.edges {
            if !new_edges.contains(&e.semantic.id) {
                let mut ghost = e.clone();
                ghost.diff = DiffMark::Removed;
                self.edges.push(ghost);
            }
        }
        // Ghost routes follow their displayed endpoints after containment moves.
        let marks: BTreeMap<_, _> = self
            .edges
            .iter()
            .map(|edge| (edge.semantic.id.clone(), edge.diff))
            .collect();
        let semantics: Vec<_> = self
            .edges
            .iter()
            .map(|edge| edge.semantic.clone())
            .collect();
        self.edges = route_edges(&self.nodes, &self.ports, &semantics);
        for edge in &mut self.edges {
            edge.diff = marks[&edge.semantic.id];
        }
        self.bounds = self
            .nodes
            .iter()
            .map(|n| n.bounds)
            .chain(self.edges.iter().map(|e| e.bounds))
            .reduce(Rect::union)
            .unwrap_or(self.bounds);
    }
}
fn node_content_changed(a: &ViewNode, b: &ViewNode) -> bool {
    let mut a = a.clone();
    a.revision_id = b.revision_id;
    a != *b
}
fn edge_content_changed(a: &ViewEdge, b: &ViewEdge) -> bool {
    let mut a = a.clone();
    a.revision_id = b.revision_id;
    a != *b
}
