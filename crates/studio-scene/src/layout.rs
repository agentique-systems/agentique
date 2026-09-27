use crate::{
    EdgeKind, InputNode, InputPort, LayoutKind, Point, Rect, SceneError, SceneInput, SceneOptions,
    Size,
};
use agq_language::ElementId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Where cards were placed. Presentation only: saved with the session, never
/// with the model.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayoutMemory {
    #[serde(with = "by_raw_id")]
    pub bounds: BTreeMap<ElementId, Rect>,
    /// Graph-only presentation anchors. Kept separately from cached geometry so
    /// a hierarchy visit or a changing card size cannot overwrite a fixed origin.
    /// Missing/filtered identities never reserve space in a graph layout.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "by_raw_id"
    )]
    pub pinned: BTreeMap<ElementId, Point>,
}
/// Element ids are stored as their raw numbers.
mod by_raw_id {
    use agq_language::ElementId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;
    pub fn serialize<T: Serialize, S: Serializer>(
        map: &BTreeMap<ElementId, T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let raw: BTreeMap<u64, &T> = map.iter().map(|(id, v)| (id.raw(), v)).collect();
        raw.serialize(serializer)
    }
    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<ElementId, T>, D::Error> {
        let raw = BTreeMap::<u64, T>::deserialize(deserializer)?;
        Ok(raw
            .into_iter()
            .map(|(id, v)| (ElementId::from_raw(id), v))
            .collect())
    }
}
impl LayoutMemory {
    /// Pin an observed graph node's top-left position. The card may still resize
    /// to show newly projected features; its semantic record is never changed.
    /// Pin conflicts are checked at the next graph layout.
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
    pub nodes: Vec<InputNode>,
    pub children: BTreeMap<ElementId, Vec<ElementId>>,
    pub parents: BTreeMap<ElementId, ElementId>,
    pub depths: BTreeMap<ElementId, usize>,
    pub collapsed: BTreeSet<ElementId>,
    /// Edge ends resolved to their shown cards.
    pub edges: Vec<(ElementId, ElementId)>,
    pub(crate) boundary_ports: Vec<BoundaryPort>,
    /// Hidden cards mapped to the nearest shown card that contains them.
    pub(crate) hidden: BTreeMap<ElementId, ElementId>,
}
#[derive(Clone, Debug)]
pub(crate) struct BoundaryPort {
    /// The port, named with its card: `api · storage`.
    pub port: InputPort,
    pub owner: ElementId,
    pub semantic_owner: ElementId,
}
impl LayoutInput {
    pub fn from_input(input: &SceneInput, options: &SceneOptions) -> Result<Self, SceneError> {
        let all: BTreeMap<_, _> = input.nodes.iter().map(|n| (n.id, n)).collect();
        let mut parents = BTreeMap::new();
        for n in &input.nodes {
            if let Some(owner) = n.owner.filter(|id| all.contains_key(id)) {
                parents.insert(n.id, owner);
            }
        }
        // Iterative traversal detects cycles without recursive stack growth.
        let mut depths = BTreeMap::new();
        for node in &input.nodes {
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
        let hierarchy = options.layout == LayoutKind::Hierarchy;
        let mut displayable = BTreeMap::new();
        let mut ordered: Vec<_> = input.nodes.iter().collect();
        ordered.sort_by_key(|n| (depths[&n.id], n.id));
        for n in ordered {
            let parent_shown = parents.get(&n.id).is_none_or(|p| {
                displayable.get(p).copied().unwrap_or(false)
                    && !(hierarchy && options.collapsed.contains(p))
            });
            let focused = options.focus.is_none()
                || options.focus == Some(n.id)
                || parents
                    .get(&n.id)
                    .is_some_and(|p| displayable.get(p).copied().unwrap_or(false));
            displayable.insert(
                n.id,
                focused && (parent_shown || options.focus == Some(n.id)),
            );
        }
        let nodes: Vec<_> = input
            .nodes
            .iter()
            .filter(|n| displayable[&n.id])
            .cloned()
            .collect();
        let visible: BTreeSet<_> = nodes.iter().map(|n| n.id).collect();
        let mut hidden = BTreeMap::new();
        for n in &input.nodes {
            if visible.contains(&n.id) {
                continue;
            }
            let mut owner = parents.get(&n.id).copied();
            while let Some(id) = owner {
                if visible.contains(&id) {
                    hidden.insert(n.id, id);
                    break;
                }
                owner = parents.get(&id).copied();
            }
        }
        let mut children: BTreeMap<ElementId, Vec<ElementId>> = BTreeMap::new();
        for n in &nodes {
            if let Some(owner) = n.owner.filter(|p| visible.contains(p)) {
                children.entry(owner).or_default().push(n.id);
            }
        }
        // Collapsed containers keep their containment even though children hide.
        for id in &options.collapsed {
            let owned: Vec<_> = input
                .nodes
                .iter()
                .filter(|n| n.owner == Some(*id))
                .map(|n| n.id)
                .collect();
            if !owned.is_empty() && visible.contains(id) {
                children.insert(*id, owned);
            }
        }
        for children in children.values_mut() {
            children.sort();
        }
        parents.retain(|child, parent| visible.contains(child) && visible.contains(parent));
        let boundary_ports = if hierarchy {
            boundary_ports(input, &visible, &hidden, &options.collapsed)
        } else {
            Vec::new()
        };
        let shown = |id: ElementId| {
            if visible.contains(&id) {
                Some(id)
            } else {
                hidden.get(&id).copied()
            }
        };
        let edges = input
            .edges
            .iter()
            .filter_map(|e| Some((shown(e.source.node)?, shown(e.target.node)?)))
            .collect();
        if !hierarchy {
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
            hidden,
        })
    }
    fn port_count(&self, id: ElementId) -> usize {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .map_or(0, |n| n.ports.len())
            + self.boundary_ports.iter().filter(|p| p.owner == id).count()
    }
    pub(crate) fn node_size(&self, node: &InputNode, base: Size) -> Size {
        let ports = node.ports.len()
            + self
                .boundary_ports
                .iter()
                .filter(|p| p.owner == node.id)
                .count();
        let port_rows = ports.div_ceil(2).saturating_sub(2) as f32 * 24.0;
        Size::new(
            base.width,
            base.height + port_rows + feature_block(node.features.len()),
        )
    }
}

/// Feature lines (attributes, items) shown on a card, and their height.
pub const MAX_FEATURE_LINES: usize = 6;
pub const FEATURE_LINE: f32 = 16.0;
/// The height of the feature lines at the bottom of a card, below its ports.
pub fn feature_block(features: usize) -> f32 {
    match features.min(MAX_FEATURE_LINES) {
        0 => 0.0,
        lines => lines as f32 * FEATURE_LINE + 10.0,
    }
}

// At most two fixed rows. Large interfaces keep their distributed boundary;
// adding more ports must not turn a container header into another outliner.
pub(crate) const PORT_STRIP_FIRST: f32 = 72.0;
pub(crate) const PORT_STRIP_ROW: f32 = 24.0;
pub(crate) fn port_strip_header(count: usize) -> Option<f32> {
    (1..=4).contains(&count).then(|| {
        PORT_STRIP_FIRST + PORT_STRIP_ROW * count.div_ceil(2).saturating_sub(1) as f32 + 20.0
    })
}

/// Ports of hidden cards inside a collapsed container that connect to
/// something outside it are shown on the container's boundary, with their
/// real identity. Links inside the container stay hidden.
fn boundary_ports(
    input: &SceneInput,
    visible: &BTreeSet<ElementId>,
    hidden: &BTreeMap<ElementId, ElementId>,
    collapsed: &BTreeSet<ElementId>,
) -> Vec<BoundaryPort> {
    if collapsed.is_empty() {
        return Vec::new();
    }
    let shown = |id: ElementId| {
        if visible.contains(&id) {
            Some(id)
        } else {
            hidden.get(&id).copied()
        }
    };
    let mut external = BTreeSet::new();
    for edge in &input.edges {
        if matches!(edge.kind, EdgeKind::Connection | EdgeKind::Interface)
            && let (Some(a), Some(b)) = (shown(edge.source.node), shown(edge.target.node))
            && a != b
        {
            for end in [edge.source, edge.target] {
                if let Some(port) = end.port {
                    external.insert((end.node, port));
                }
            }
        }
    }
    let nodes: BTreeMap<_, _> = input.nodes.iter().map(|n| (n.id, n)).collect();
    let mut result = Vec::new();
    for (node, port) in external {
        let (Some(owner), Some(card)) = (shown(node), nodes.get(&node)) else {
            continue;
        };
        if owner == node || !collapsed.contains(&owner) {
            continue;
        }
        let Some(info) = card.ports.iter().find(|p| p.id == port) else {
            continue;
        };
        result.push(BoundaryPort {
            port: InputPort {
                name: format!("{} · {}", card.name, info.name),
                ..info.clone()
            },
            owner,
            semantic_owner: node,
        });
    }
    result
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
        let secondary: BTreeSet<_> = input
            .nodes
            .iter()
            .filter(|n| is_secondary(n, input.children.get(&n.id).is_some_and(|c| !c.is_empty())))
            .map(|n| n.id)
            .collect();
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
                let base = if secondary.contains(&n.id) {
                    // A definition with nothing to show inside is a short card.
                    let empty = n.ports.is_empty() && n.features.is_empty();
                    Size::new(
                        SECONDARY_WIDTH,
                        if empty { 64.0 } else { self.node_size.height },
                    )
                } else {
                    self.node_size
                };
                sizes.insert(n.id, input.node_size(n, base));
                continue;
            }
            let header = port_strip_header(input.port_count(n.id))
                .map_or(self.header, |strip| self.header.max(strip));
            let origin = Point::new(self.inset, header);
            let placed = self.pack_structure(&children, &secondary, &sizes, previous, n.id, origin);
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
            // A container fits what it shows now. Removed cards in a "what
            // changed" view are laid out with the others (`Scene::comparison`).
            sizes.insert(n.id, Size::new(width, height));
            offsets.extend(placed.into_iter().map(|(id, r)| (id, r.min)));
        }
        let roots: Vec<_> = ids
            .iter()
            .filter(|id| !input.parents.contains_key(id))
            .copied()
            .collect();
        let roots = self.pack_structure(
            &roots,
            &secondary,
            &sizes,
            previous,
            ElementId::from_raw(0),
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
/// Width of secondary cards (definitions that own no cards).
pub const SECONDARY_WIDTH: f32 = 196.0;

/// A definition that owns no cards is secondary on the Surface: the
/// structure (usages, their connections and the definitions that contain
/// them) comes first, type definitions after it, in a compact band.
pub fn is_secondary(node: &InputNode, has_children: bool) -> bool {
    node.keyword.ends_with(" def") && !has_children
}

impl HierarchyLayout {
    /// Packs the structure first and the secondary cards in a band below it.
    fn pack_structure(
        &self,
        children: &[ElementId],
        secondary: &BTreeSet<ElementId>,
        sizes: &BTreeMap<ElementId, Size>,
        previous: Option<&LayoutMemory>,
        parent: ElementId,
        origin: Point,
    ) -> BTreeMap<ElementId, Rect> {
        let (rest, primary): (Vec<ElementId>, Vec<ElementId>) =
            children.iter().partition(|id| secondary.contains(id));
        if primary.is_empty() || rest.is_empty() {
            return self.pack(children, sizes, previous, parent, origin);
        }
        let mut placed = self.pack(&primary, sizes, previous, parent, origin);
        let right = placed.values().map(|r| r.max.x).fold(origin.x, f32::max);
        let bottom = placed.values().map(|r| r.max.y).fold(origin.y, f32::max);
        let band_origin = Point::new(origin.x, bottom + self.gap * 1.5);
        let columns = (((right - origin.x + self.gap) / (SECONDARY_WIDTH + self.gap)).floor()
            as usize)
            .max(self.max_columns);
        let mut band = self.pack(&rest, sizes, previous, parent, band_origin);
        let collides = band.values().any(|r| {
            r.min.y < band_origin.y || placed.values().any(|p| p.inflate(4.0).intersects(*r))
        });
        if collides || previous.is_none() {
            band = self.pack_roots(&rest, sizes, band_origin, columns);
        }
        placed.extend(band);
        placed
    }

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
        if children.len() > 1
            && previous
                .is_none_or(|memory| children.iter().all(|id| !memory.bounds.contains_key(id)))
        {
            // A few cards inside a container stack in one column; larger or
            // mixed groups use columns.
            let stacked = parent != ElementId::from_raw(0)
                && children.len() <= 4
                && children
                    .iter()
                    .all(|id| sizes[id].height <= self.node_size.height * 1.6);
            let columns = if stacked { 1 } else { self.max_columns };
            return self.pack_roots(children, sizes, origin, columns);
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
        } else if parent != ElementId::from_raw(0) && children.len() <= 4 {
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
        if parent != ElementId::from_raw(0)
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
        max_columns: usize,
    ) -> BTreeMap<ElementId, Rect> {
        let columns = if children.len() > max_columns * 4 {
            (children.len() as f32).sqrt().ceil() as usize
        } else {
            max_columns.min(children.len()).max(1)
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
        let ids: Vec<_> = (1..=9).map(ElementId::from_raw).collect();
        let mut sizes: BTreeMap<_, _> = ids
            .iter()
            .map(|id| (*id, Size::new(232.0, 118.0)))
            .collect();
        sizes.insert(ids[0], Size::new(850.0, 500.0));
        let layout = HierarchyLayout::default();
        let first = layout.pack(&ids, &sizes, None, ElementId::from_raw(0), Point::default());
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
            ElementId::from_raw(0),
            Point::default(),
        );
        assert_eq!(first, restored);
    }
}
