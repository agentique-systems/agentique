use crate::{Point, Rect, SceneTarget, SemanticScene, VisibleScene, segment_distance};
use agq_modeling_workspace::ProjectRevisionId;
use std::collections::{BTreeMap, BTreeSet};

/// Uniform world grid with an overflow lane for long edges/large containers.
/// Each object is inserted once per touched cell; queries deduplicate identities.
#[derive(Clone, Debug)]
pub(crate) struct RectIndex<T> {
    cell: f32,
    cells: BTreeMap<(i32, i32), Vec<usize>>,
    large: Vec<usize>,
    items: Vec<(Rect, T)>,
}
impl<T> RectIndex<T> {
    pub fn new(cell: f32) -> Self {
        Self {
            cell,
            cells: BTreeMap::new(),
            large: Vec::new(),
            items: Vec::new(),
        }
    }
    fn range(&self, r: Rect) -> (i32, i32, i32, i32) {
        (
            (r.min.x / self.cell).floor() as i32,
            (r.min.y / self.cell).floor() as i32,
            (r.max.x / self.cell).floor() as i32,
            (r.max.y / self.cell).floor() as i32,
        )
    }
    pub fn insert(&mut self, r: Rect, item: T) {
        let id = self.items.len();
        self.items.push((r, item));
        let (x0, y0, x1, y1) = self.range(r);
        if (i128::from(x1) - i128::from(x0) + 1) * (i128::from(y1) - i128::from(y0) + 1) > 128 {
            self.large.push(id);
            return;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cells.entry((x, y)).or_default().push(id);
            }
        }
    }
    /// Each intersecting item once, in insertion order, including overflow items.
    pub fn query(&self, r: Rect) -> Vec<&T> {
        let (x0, y0, x1, y1) = self.range(r);
        if (i128::from(x1) - i128::from(x0) + 1) * (i128::from(y1) - i128::from(y0) + 1) > 4096 {
            return self
                .items
                .iter()
                .filter(|(bounds, _)| bounds.intersects(r))
                .map(|(_, v)| v)
                .collect();
        }
        let mut ids = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if let Some(cell) = self.cells.get(&(x, y)) {
                    ids.extend(cell.iter().copied());
                }
            }
        }
        ids.extend(self.large.iter().copied());
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| {
                let (bounds, item) = &self.items[id];
                bounds.intersects(r).then_some(item)
            })
            .collect()
    }
}
#[derive(Clone, Debug)]
struct HitItem {
    target: usize,
    bounds: Rect,
    segment: Option<(Point, Point)>,
    priority: u8,
}
#[derive(Clone, Debug)]
struct IndexedTarget {
    identity: SceneTarget,
    scene_index: usize,
}
/// Disposable bounds index for one scene generation.
///
/// Rebuild after layout, projection replacement or applying a diff. Each routed
/// segment refers to one shared identity slot, rather than owning an edge name.
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    revision_id: ProjectRevisionId,
    index: RectIndex<HitItem>,
    targets: Vec<IndexedTarget>,
}
impl SpatialIndex {
    pub fn build(scene: &SemanticScene) -> Self {
        let mut index = RectIndex::new(256.0);
        let mut targets =
            Vec::with_capacity(scene.nodes.len() + scene.ports.len() + scene.edges.len());
        for (scene_index, n) in scene.nodes.iter().enumerate() {
            let identity = if n.is_container {
                SceneTarget::Container(n.id())
            } else {
                SceneTarget::Node(n.id())
            };
            let target = targets.len();
            targets.push(IndexedTarget {
                identity,
                scene_index,
            });
            index.insert(
                n.bounds,
                HitItem {
                    target,
                    bounds: n.bounds,
                    segment: None,
                    priority: if n.is_container { 3 } else { 1 },
                },
            );
        }
        for (scene_index, p) in scene.ports.iter().enumerate() {
            let target = targets.len();
            targets.push(IndexedTarget {
                identity: SceneTarget::Port(p.id),
                scene_index,
            });
            let bounds = Rect::new(p.position.x - 7.0, p.position.y - 7.0, 14.0, 14.0);
            index.insert(
                bounds,
                HitItem {
                    target,
                    bounds,
                    segment: None,
                    priority: 0,
                },
            );
        }
        for (scene_index, edge) in scene.edges.iter().enumerate() {
            let target = targets.len();
            targets.push(IndexedTarget {
                identity: SceneTarget::Edge(edge.semantic.id.clone()),
                scene_index,
            });
            for pair in edge.points.windows(2) {
                let bounds = Rect::from_points(pair[0], pair[1]);
                index.insert(
                    bounds,
                    HitItem {
                        target,
                        bounds,
                        segment: Some((pair[0], pair[1])),
                        priority: 2,
                    },
                );
            }
        }
        Self {
            revision_id: scene.revision_id,
            index,
            targets,
        }
    }
    /// Tolerance is in world units; shell should divide pixel radius by zoom.
    pub fn hit_test(&self, point: Point, tolerance: f32) -> Option<SceneTarget> {
        let query = Rect::new(point.x, point.y, 0.0, 0.0).inflate(tolerance.max(0.0));
        self.index
            .query(query)
            .into_iter()
            .filter(|hit| match hit.segment {
                Some((a, b)) => segment_distance(point, a, b) <= tolerance,
                None => hit.bounds.inflate(tolerance.min(3.0)).contains(point),
            })
            .min_by(|a, b| {
                a.priority
                    .cmp(&b.priority)
                    .then_with(|| {
                        (a.bounds.width() * a.bounds.height())
                            .total_cmp(&(b.bounds.width() * b.bounds.height()))
                    })
                    .then_with(|| {
                        self.targets[a.target]
                            .identity
                            .cmp(&self.targets[b.target].identity)
                    })
            })
            .map(|item| self.targets[item.target].identity.clone())
    }
    /// Culling includes edges crossing the viewport with both endpoints outside.
    pub fn query(&self, bounds: Rect) -> Vec<SceneTarget> {
        let mut targets: Vec<_> = self
            .index
            .query(bounds)
            .into_iter()
            .map(|i| &self.targets[i.target].identity)
            .collect();
        targets.sort_unstable();
        targets.dedup();
        targets.into_iter().cloned().collect()
    }
    /// Fully enclosed nodes and ports; containers only if their entire box fits.
    pub fn marquee(&self, bounds: Rect) -> Vec<SceneTarget> {
        self.index
            .query(bounds)
            .into_iter()
            .filter(|i| i.segment.is_none() && bounds.contains_rect(i.bounds))
            .map(|i| self.targets[i.target].identity.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn visible(&self, bounds: Rect) -> Vec<SceneTarget> {
        self.query(bounds)
    }
    /// Cull and borrow records in original scene order, without allocating
    /// semantic identities or resolving them through another identity map.
    ///
    /// Revision and exact identity are checked before borrowing each stored
    /// slot. A stale index cannot return unrelated records or panic after scene
    /// replacement/shrinking. This does not authorize reuse after geometry
    /// changes: rebuild this index for every new scene generation.
    pub fn visible_scene<'scene>(
        &self,
        scene: &'scene SemanticScene,
        bounds: Rect,
    ) -> VisibleScene<'scene> {
        if scene.revision_id != self.revision_id {
            return VisibleScene::default();
        }
        let mut visible = VisibleScene::default();
        let mut previous = None;
        // Grid hits are in insertion order. All segments of an edge are
        // inserted together, so target slots are ordered and duplicates are
        // adjacent. This preserves containment order without a second sort.
        for hit in self.index.query(bounds) {
            if previous == Some(hit.target) {
                continue;
            }
            previous = Some(hit.target);
            let target = &self.targets[hit.target];
            match &target.identity {
                SceneTarget::Node(id) | SceneTarget::Container(id) => {
                    if let Some(node) = scene.nodes.get(target.scene_index).filter(|node| {
                        node.id() == *id
                            && node.is_container
                                == matches!(&target.identity, SceneTarget::Container(_))
                    }) {
                        visible.nodes.push(node);
                    }
                }
                SceneTarget::Port(id) => {
                    if let Some(port) = scene
                        .ports
                        .get(target.scene_index)
                        .filter(|port| port.id == *id)
                    {
                        visible.ports.push(port);
                    }
                }
                SceneTarget::Edge(id) => {
                    if let Some(edge) = scene
                        .edges
                        .get(target.scene_index)
                        .filter(|edge| edge.semantic.id == *id)
                    {
                        visible.edges.push(edge);
                    }
                }
            }
        }
        visible
    }
}
