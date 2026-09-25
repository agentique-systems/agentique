use crate::{NodeCategory, Point, Rect, SceneError, SceneOptions, Size};
use agq_kernel::ElementId;
use agq_modeling_view::{ViewNode, ViewProjection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Session geometry can survive revisions without entering model checkpoints.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LayoutMemory {
    pub bounds: BTreeMap<ElementId, Rect>,
}
#[derive(Clone, Debug)]
pub struct LayoutInput {
    pub nodes: Vec<ViewNode>,
    pub children: BTreeMap<ElementId, Vec<ElementId>>,
    pub parents: BTreeMap<ElementId, ElementId>,
    pub depths: BTreeMap<ElementId, usize>,
    pub collapsed: BTreeSet<ElementId>,
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
        Ok(Self {
            nodes,
            children,
            parents,
            depths,
            collapsed: options.collapsed.clone(),
        })
    }
}
#[derive(Clone, Debug, Default)]
pub struct LayoutResult {
    pub bounds: BTreeMap<ElementId, Rect>,
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
                sizes.insert(n.id, self.node_size);
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
            sizes.insert(n.id, Size::new(width, height));
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
        let columns = if children.len() > self.max_columns * 4 {
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
}
