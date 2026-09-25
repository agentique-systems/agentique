use crate::{NodeCategory, Point, Rect, SceneError, SceneOptions, Size};
use agq_kernel::ElementId;
use agq_modeling_view::{RelationshipFamily, ViewNode, ViewOrigin, ViewProjection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Session geometry can survive revisions without entering model checkpoints.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayoutMemory {
    pub bounds: BTreeMap<ElementId, Rect>,
    /// Graph-only presentation anchors. Kept separately from cached geometry so
    /// a hierarchy visit or a changing card size cannot overwrite a fixed origin.
    /// Missing/filtered identities never reserve space in a graph layout.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pinned: BTreeMap<ElementId, Point>,
}
impl LayoutMemory {
    /// Pin an observed graph node's top-left position. The card may still resize
    /// to show newly projected features; its semantic record is never changed.
    /// Active pin conflicts are checked against the next graph projection.
    pub fn pin(&mut self, id: ElementId, bounds: Rect) -> Result<(), PinError> {
        if !bounds.finite() || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Err(PinError::InvalidBounds(id));
        }
        self.bounds.insert(id, bounds);
        self.pinned.insert(id, bounds.min);
        Ok(())
    }
    /// Release the constraint while keeping the last geometry as a layout hint.
    pub fn unpin(&mut self, id: ElementId) -> bool {
        self.pinned.remove(&id).is_some()
    }
    pub fn is_pinned(&self, id: ElementId) -> bool {
        self.pinned.contains_key(&id)
    }
}
/// Presentation errors, never language validation or model mutations.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PinError {
    #[error("graph pin for {0} has invalid geometry")]
    InvalidBounds(ElementId),
    #[error("graph pins for {first} and {second} overlap; unpin one to rearrange the view")]
    Conflict { first: ElementId, second: ElementId },
}
#[derive(Clone, Debug)]
pub struct LayoutInput {
    pub nodes: Vec<ViewNode>,
    pub children: BTreeMap<ElementId, Vec<ElementId>>,
    pub parents: BTreeMap<ElementId, ElementId>,
    pub depths: BTreeMap<ElementId, usize>,
    pub collapsed: BTreeSet<ElementId>,
    /// Port endpoints resolve to their visible owner's topology node.
    pub edges: Vec<(ElementId, ElementId)>,
    pub(crate) boundary_ports: Vec<BoundaryPort>,
}
#[derive(Clone, Debug)]
pub(crate) struct BoundaryPort {
    pub id: ElementId,
    pub owner: ElementId,
    pub semantic_owner: ElementId,
    pub name: String,
    pub origin: ViewOrigin,
}
impl LayoutInput {
    pub fn from_projection(
        projection: &ViewProjection,
        options: &SceneOptions,
    ) -> Result<Self, SceneError> {
        let all: BTreeMap<_, _> = projection.nodes.iter().map(|n| (n.id, n)).collect();
        let mut parents = BTreeMap::new();
        for n in &projection.nodes {
            if let Some(owner) = n.owner.filter(|id| all.contains_key(id)) {
                parents.insert(n.id, owner);
            }
        }
        // Iterative color traversal detects cycles without recursive stack growth.
        let mut depths = BTreeMap::new();
        for node in &projection.nodes {
            let mut path = Vec::new();
            let mut visiting = BTreeSet::new();
            let mut id = node.id;
            while !depths.contains_key(&id) {
                if !visiting.insert(id) {
                    return Err(SceneError::OwnershipCycle(id));
                }
                path.push(id);
                match parents.get(&id) {
                    Some(parent) => id = *parent,
                    None => break,
                }
            }
            let mut depth = depths.get(&id).copied().map_or(0, |d| d + 1);
            for id in path.into_iter().rev() {
                depths.insert(id, depth);
                depth += 1;
            }
        }
        let mut displayable = BTreeMap::new();
        let mut ordered: Vec<_> = projection.nodes.iter().collect();
        ordered.sort_by_key(|n| (depths[&n.id], n.id));
        for n in ordered {
            let parent_shown = parents.get(&n.id).is_none_or(|p| {
                displayable.get(p).copied().unwrap_or(false) && !options.collapsed.contains(p)
            });
            let focused = options.focus.is_none()
                || options.focus == Some(n.id)
                || parents
                    .get(&n.id)
                    .is_some_and(|p| displayable.get(p).copied().unwrap_or(false));
            let is_port = NodeCategory::from_semantic_kind(&n.semantic_kind) == NodeCategory::Port
                && n.owner.is_some_and(|id| all.contains_key(&id));
            displayable.insert(
                n.id,
                !is_port && focused && (parent_shown || options.focus == Some(n.id)),
            );
        }
        let nodes: Vec<_> = projection
            .nodes
            .iter()
            .filter(|n| displayable[&n.id])
            .cloned()
            .collect();
        let mut children: BTreeMap<ElementId, Vec<ElementId>> = BTreeMap::new();
        for n in &nodes {
            // Preserve collapsed containment metadata even though children hide.
            if let Some(owner) = n.owner.filter(|p| displayable.get(p) == Some(&true)) {
                children.entry(owner).or_default().push(n.id);
            }
        }
        for id in &options.collapsed {
            let children_all: Vec<_> = projection
                .nodes
                .iter()
                .filter(|n| n.owner == Some(*id))
                .map(|n| n.id)
                .collect();
            if !children_all.is_empty() {
                children.insert(*id, children_all);
            }
        }
        for children in children.values_mut() {
            children.sort();
        }
        parents.retain(|child, parent| {
            displayable.get(child) == Some(&true) && displayable.get(parent) == Some(&true)
        });
        let visible: BTreeSet<_> = nodes.iter().map(|n| n.id).collect();
        let boundary_ports = if options.hierarchy {
            boundary_ports(projection, &visible, &options.collapsed)
        } else {
            Vec::new()
        };
        let mut port_owners: BTreeMap<_, _> = projection
            .nodes
            .iter()
            .filter_map(|n| n.owner.map(|owner| (n.id, owner)))
            .collect();
        for n in &nodes {
            for f in &n.features {
                port_owners.insert(f.id, n.id);
            }
        }
        let endpoint = |id: ElementId| {
            if visible.contains(&id) {
                Some(id)
            } else {
                port_owners
                    .get(&id)
                    .copied()
                    .filter(|p| visible.contains(p))
            }
        };
        let edges = projection
            .edges
            .iter()
            .filter_map(|e| Some((endpoint(e.source)?, endpoint(e.target)?)))
            .collect();
        if !options.hierarchy {
            parents.clear();
            children.clear();
            for depth in depths.values_mut() {
                *depth = 0;
            }
        }
        Ok(Self {
            nodes,
            children,
            parents,
            depths,
            collapsed: options.collapsed.clone(),
            edges,
            boundary_ports,
        })
    }
    pub(crate) fn node_size(&self, node: &ViewNode, base: Size) -> Size {
        let ports = node.counts.ports.max(
            node.features
                .iter()
                .filter(|feature| {
                    NodeCategory::from_semantic_kind(&feature.semantic_kind) == NodeCategory::Port
                })
                .count(),
        ) + self
            .boundary_ports
            .iter()
            .filter(|p| p.owner == node.id)
            .count();
        Size::new(
            base.width,
            base.height + ports.div_ceil(2).saturating_sub(2) as f32 * 24.0,
        )
    }
}

/// Resolve external connections to the boundary of a collapsed subsystem while
/// preserving every actual endpoint identity. Internal links stay hidden.
fn boundary_ports(
    projection: &ViewProjection,
    visible: &BTreeSet<ElementId>,
    collapsed: &BTreeSet<ElementId>,
) -> Vec<BoundaryPort> {
    if collapsed.is_empty() {
        return Vec::new();
    }
    let nodes: BTreeMap<_, _> = projection.nodes.iter().map(|n| (n.id, n)).collect();
    let mut ports = BTreeMap::new();
    for node in &projection.nodes {
        if NodeCategory::from_semantic_kind(&node.semantic_kind) == NodeCategory::Port
            && let Some(owner) = node.owner
        {
            ports.insert(node.id, (owner, node.name.clone(), node.origin));
        }
        for feature in &node.features {
            if NodeCategory::from_semantic_kind(&feature.semantic_kind) == NodeCategory::Port {
                ports.insert(feature.id, (node.id, feature.name.clone(), node.origin));
            }
        }
    }
    let boundary = |mut owner| {
        while !visible.contains(&owner) {
            owner = nodes.get(&owner)?.owner?;
        }
        Some(owner)
    };
    let endpoint_owner = |id| boundary(ports.get(&id).map_or(id, |(owner, _, _)| *owner));
    let mut external = BTreeSet::new();
    for edge in &projection.edges {
        if edge.family == RelationshipFamily::Connection
            && let (Some(a), Some(b)) = (endpoint_owner(edge.source), endpoint_owner(edge.target))
            && a != b
        {
            external.insert(edge.source);
            external.insert(edge.target);
        }
    }
    ports
        .into_iter()
        .filter_map(|(id, (owner, name, origin))| {
            let displayed = boundary(owner)?;
            (displayed != owner && collapsed.contains(&displayed) && external.contains(&id)).then(
                || BoundaryPort {
                    id,
                    owner: displayed,
                    semantic_owner: owner,
                    name: format!(
                        "{} · {name}",
                        nodes.get(&owner).map_or("Part", |n| n.name.as_str())
                    ),
                    origin,
                },
            )
        })
        .collect()
}
#[derive(Clone, Debug, Default)]
pub struct LayoutResult {
    pub bounds: BTreeMap<ElementId, Rect>,
    /// Conflicting constraints are explicit; callers must not display an
    /// incomplete/overlapping result as a successful graph layout.
    pub pin_error: Option<PinError>,
}
/// Layout implementations receive only disposable public view records.
pub trait LayoutEngine: Send + Sync {
    fn layout(&self, input: &LayoutInput, previous: Option<&LayoutMemory>) -> LayoutResult;
}
#[derive(Clone, Copy, Debug)]
pub struct HierarchyLayout {
    pub node_size: Size,
    pub gap: f32,
    pub inset: f32,
    pub header: f32,
    pub max_columns: usize,
}
impl Default for HierarchyLayout {
    fn default() -> Self {
        Self {
            node_size: Size::new(232.0, 118.0),
            gap: 44.0,
            inset: 30.0,
            header: 72.0,
            max_columns: 3,
        }
    }
}
impl LayoutEngine for HierarchyLayout {
    fn layout(&self, input: &LayoutInput, previous: Option<&LayoutMemory>) -> LayoutResult {
        let ids: BTreeSet<_> = input.nodes.iter().map(|n| n.id).collect();
        let mut ordered: Vec<_> = input.nodes.iter().collect();
        ordered.sort_by_key(|n| std::cmp::Reverse((input.depths[&n.id], n.id)));
        let mut sizes = BTreeMap::new();
        let mut offsets = BTreeMap::new();
        for n in &ordered {
            let children = input
                .children
                .get(&n.id)
                .filter(|_| !input.collapsed.contains(&n.id))
                .map_or(&[][..], Vec::as_slice);
            let children: Vec<_> = children
                .iter()
                .copied()
                .filter(|id| ids.contains(id))
                .collect();
            if children.is_empty() {
                sizes.insert(n.id, input.node_size(n, self.node_size));
                continue;
            }
            let origin = Point::new(self.inset, self.header);
            let placed = self.pack(&children, &sizes, previous, n.id, origin);
            let width = placed
                .values()
                .map(|r| r.max.x)
                .fold(self.node_size.width, f32::max)
                + self.inset;
            let height = placed
                .values()
                .map(|r| r.max.y)
                .fold(self.node_size.height, f32::max)
                + self.inset;
            // Keep an established containment envelope when a child disappears.
            // Besides preserving the mental map, this reserves room for diff
            // ghosts before neighboring containers are packed.
            let old = previous.and_then(|memory| memory.bounds.get(&n.id));
            sizes.insert(
                n.id,
                Size::new(
                    old.map_or(width, |bounds| width.max(bounds.width())),
                    old.map_or(height, |bounds| height.max(bounds.height())),
                ),
            );
            offsets.extend(placed.into_iter().map(|(id, r)| (id, r.min)));
        }
        let roots: Vec<_> = ids
            .iter()
            .filter(|id| !input.parents.contains_key(id))
            .copied()
            .collect();
        let roots = self.pack(
            &roots,
            &sizes,
            previous,
            ElementId::from_u128(0),
            Point::default(),
        );
        let mut result = LayoutResult::default();
        ordered.reverse();
        for n in ordered {
            let position = if let Some(parent) = input.parents.get(&n.id) {
                let parent: &Rect = &result.bounds[parent];
                let offset = offsets[&n.id];
                Point::new(parent.min.x + offset.x, parent.min.y + offset.y)
            } else {
                roots[&n.id].min
            };
            let size = sizes[&n.id];
            result.bounds.insert(
                n.id,
                Rect::new(position.x, position.y, size.width, size.height),
            );
        }
        result
    }
}
impl HierarchyLayout {
    fn pack(
        &self,
        children: &[ElementId],
        sizes: &BTreeMap<ElementId, Size>,
        previous: Option<&LayoutMemory>,
        parent: ElementId,
        origin: Point,
    ) -> BTreeMap<ElementId, Rect> {
        // Root definitions have very different sizes: one may contain a whole
        // subsystem while its type context is a handful of small cards. A grid
        // whose every cell inherits the largest width and height creates vast
        // empty bands and forces fit-to-view below readable zoom.
        if parent == ElementId::from_u128(0)
            && children.len() > 1
            && previous
                .is_none_or(|memory| children.iter().all(|id| !memory.bounds.contains_key(id)))
        {
            return self.pack_roots(children, sizes, origin);
        }
        let mut result = BTreeMap::new();
        let mut occupied = crate::spatial::RectIndex::new(320.0);
        let parent_old = previous
            .and_then(|p| p.bounds.get(&parent))
            .map_or(Point::default(), |r| r.min);
        // Restore old siblings before allocating added objects; insertions cannot
        // steal a retained position merely because their identity sorts earlier.
        if let Some(previous) = previous {
            for id in children {
                if let Some(old) = previous.bounds.get(id).filter(|r| r.finite()) {
                    let size = sizes[id];
                    let rect = Rect::new(
                        (old.min.x - parent_old.x).max(origin.x),
                        (old.min.y - parent_old.y).max(origin.y),
                        size.width,
                        size.height,
                    );
                    if occupied.query(rect.inflate(self.gap * 0.45)).is_empty() {
                        occupied.insert(rect, *id);
                        result.insert(*id, rect);
                    }
                }
            }
        }
        let mut columns = if children.len() > self.max_columns * 4 {
            (children.len() as f32).sqrt().ceil() as usize
        } else if parent != ElementId::from_u128(0) && children.len() <= 4 {
            1
        } else {
            self.max_columns.min(children.len()).max(1)
        };
        let cell_width = children
            .iter()
            .map(|id| sizes[id].width)
            .fold(0.0, f32::max)
            + self.gap;
        // Preserve the established sibling column count for local insertions.
        // Switching a four-part column into a three-column grid for the fifth
        // part needlessly pushes every neighboring subsystem out of place.
        if parent != ElementId::from_u128(0)
            && let Some(old) = previous.and_then(|memory| memory.bounds.get(&parent))
        {
            columns = ((old.width() - 2.0 * self.inset + self.gap) / cell_width)
                .floor()
                .max(1.0) as usize;
        }
        let cell_height = children
            .iter()
            .map(|id| sizes[id].height)
            .fold(0.0, f32::max)
            + self.gap;
        let mut slot = 0;
        for id in children {
            if result.contains_key(id) {
                continue;
            }
            let size = sizes[id];
            loop {
                let rect = Rect::new(
                    origin.x + (slot % columns) as f32 * cell_width,
                    origin.y + (slot / columns) as f32 * cell_height,
                    size.width,
                    size.height,
                );
                slot += 1;
                if occupied.query(rect.inflate(self.gap * 0.45)).is_empty() {
                    occupied.insert(rect, *id);
                    result.insert(*id, rect);
                    break;
                }
            }
        }
        result
    }

    fn pack_roots(
        &self,
        children: &[ElementId],
        sizes: &BTreeMap<ElementId, Size>,
        origin: Point,
    ) -> BTreeMap<ElementId, Rect> {
        let columns = if children.len() > self.max_columns * 4 {
            (children.len() as f32).sqrt().ceil() as usize
        } else {
            self.max_columns.min(children.len()).max(1)
        };
        let mut ordered = children.to_vec();
        ordered.sort_by(|a, b| sizes[b].height.total_cmp(&sizes[a].height).then(a.cmp(b)));
        let mut heights = vec![0.0_f32; columns];
        let mut widths = vec![0.0_f32; columns];
        let mut assignments = Vec::with_capacity(children.len());
        for id in ordered {
            let column = (0..columns)
                .min_by(|a, b| heights[*a].total_cmp(&heights[*b]).then(a.cmp(b)))
                .expect("at least one root column");
            assignments.push((id, column, heights[column]));
            widths[column] = widths[column].max(sizes[&id].width);
            heights[column] += sizes[&id].height + self.gap;
        }
        let mut left = origin.x;
        let offsets: Vec<_> = widths
            .into_iter()
            .map(|width| {
                let offset = left;
                left += width + self.gap;
                offset
            })
            .collect();
        assignments
            .into_iter()
            .map(|(id, column, y)| {
                let size = sizes[&id];
                (
                    id,
                    Rect::new(offsets[column], origin.y + y, size.width, size.height),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod root_packing_tests {
    use super::*;

    #[test]
    fn a_large_subsystem_does_not_inflate_every_context_card_cell() {
        let ids: Vec<_> = (1..=9).map(ElementId::from_u128).collect();
        let mut sizes: BTreeMap<_, _> = ids
            .iter()
            .map(|id| (*id, Size::new(232.0, 118.0)))
            .collect();
        sizes.insert(ids[0], Size::new(850.0, 500.0));
        let layout = HierarchyLayout::default();
        let first = layout.pack(
            &ids,
            &sizes,
            None,
            ElementId::from_u128(0),
            Point::default(),
        );
        assert!(first.values().map(|r| r.max.x).fold(0.0_f32, f32::max) < 1500.0);
        assert!(first.values().map(|r| r.max.y).fold(0.0_f32, f32::max) < 700.0);
        for (id, bounds) in &first {
            for (other, other_bounds) in &first {
                if id != other {
                    assert!(!bounds.intersects(*other_bounds));
                }
            }
        }
        let memory = LayoutMemory {
            bounds: first.clone(),
            ..Default::default()
        };
        let restored = layout.pack(
            &ids,
            &sizes,
            Some(&memory),
            ElementId::from_u128(0),
            Point::default(),
        );
        assert_eq!(first, restored);
    }
}
