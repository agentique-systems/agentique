use crate::{Point, Rect, Scene, SceneTarget, VisibleScene, segment_distance};
use std::collections::BTreeSet;

/// Uniform world grid with an overflow lane for long edges/large containers.
/// Each object is inserted once per touched cell; queries deduplicate identities.
///
/// The cells are one array that grows to cover what is inserted: indexing a
/// 10k scene touches about 1.5 million cells, and a map of cells took most
/// of that time.
#[derive(Clone, Debug)]
pub(crate) struct RectIndex<T> {
    cell: f32,
    /// The cell at the grid's top left, and the grid's size in cells.
    first: (i32, i32),
    columns: i32,
    rows: i32,
    /// Item numbers, row by row.
    cells: Vec<Vec<u32>>,
    large: Vec<usize>,
    items: Vec<(Rect, T)>,
}
/// The most cells the grid grows to; what does not fit goes to the overflow lane.
const MAX_CELLS: i64 = 1 << 18;
/// The number of cells from (x0, y0) to (x1, y1), inclusive.
fn span(x0: i32, y0: i32, x1: i32, y1: i32) -> i128 {
    (i128::from(x1) - i128::from(x0) + 1) * (i128::from(y1) - i128::from(y0) + 1)
}
impl<T> RectIndex<T> {
    pub fn new(cell: f32) -> Self {
        Self {
            cell,
            first: (0, 0),
            columns: 0,
            rows: 0,
            cells: Vec::new(),
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
        if span(x0, y0, x1, y1) > 128 || !self.cover(x0, y0, x1, y1) {
            self.large.push(id);
            return;
        }
        for y in y0..=y1 {
            let row = (y - self.first.1) as usize * self.columns as usize;
            for x in x0..=x1 {
                self.cells[row + (x - self.first.0) as usize].push(id as u32);
            }
        }
    }
    /// Grows the grid to cover the cells from (x0, y0) to (x1, y1), at least
    /// doubling each side that grows, so repeated growth stays cheap. False
    /// when that would take more than [`MAX_CELLS`].
    fn cover(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let (fx, fy) = (i64::from(self.first.0), i64::from(self.first.1));
        let (columns, rows) = (i64::from(self.columns), i64::from(self.rows));
        // Ends are exclusive from here on.
        let (x0, y0, x1, y1) = (
            i64::from(x0),
            i64::from(y0),
            i64::from(x1) + 1,
            i64::from(y1) + 1,
        );
        if x0 >= fx && y0 >= fy && x1 <= fx + columns && y1 <= fy + rows {
            return true;
        }
        let exact = if self.cells.is_empty() {
            (x0, y0, x1, y1)
        } else {
            (
                x0.min(fx),
                y0.min(fy),
                x1.max(fx + columns),
                y1.max(fy + rows),
            )
        };
        let doubled = if self.cells.is_empty() {
            exact
        } else {
            (
                if x0 < fx { x0.min(fx - columns) } else { fx },
                if y0 < fy { y0.min(fy - rows) } else { fy },
                if x1 > fx + columns {
                    x1.max(fx + 2 * columns)
                } else {
                    fx + columns
                },
                if y1 > fy + rows {
                    y1.max(fy + 2 * rows)
                } else {
                    fy + rows
                },
            )
        };
        // Bounds first: far-apart items (a pin far out in the layout memory,
        // say) make spans whose product does not fit in i64.
        let fits = |(a, b, c, d): (i64, i64, i64, i64)| {
            a >= i64::from(i32::MIN)
                && b >= i64::from(i32::MIN)
                && c <= i64::from(i32::MAX)
                && d <= i64::from(i32::MAX)
                && (c - a).checked_mul(d - b).is_some_and(|n| n <= MAX_CELLS)
        };
        let Some((nx0, ny0, nx1, ny1)) = [doubled, exact].into_iter().find(|e| fits(*e)) else {
            return false;
        };
        let new_columns = (nx1 - nx0) as usize;
        let mut cells = vec![Vec::new(); new_columns * (ny1 - ny0) as usize];
        for row in 0..rows {
            for column in 0..columns {
                let old = (row * columns + column) as usize;
                let new = (fy + row - ny0) as usize * new_columns + (fx + column - nx0) as usize;
                cells[new] = std::mem::take(&mut self.cells[old]);
            }
        }
        self.first = (nx0 as i32, ny0 as i32);
        self.columns = (nx1 - nx0) as i32;
        self.rows = (ny1 - ny0) as i32;
        self.cells = cells;
        true
    }
    /// Each intersecting item once, in insertion order, including overflow items.
    pub fn query(&self, r: Rect) -> Vec<&T> {
        let (x0, y0, x1, y1) = self.range(r);
        if span(x0, y0, x1, y1) > 4096 {
            return self
                .items
                .iter()
                .filter(|(bounds, _)| bounds.intersects(r))
                .map(|(_, v)| v)
                .collect();
        }
        let mut ids = Vec::new();
        // Only the part of the query inside the grid has cells.
        let (x0, x1) = (
            x0.max(self.first.0),
            x1.min(self.first.0.saturating_add(self.columns - 1)),
        );
        let (y0, y1) = (
            y0.max(self.first.1),
            y1.min(self.first.1.saturating_add(self.rows - 1)),
        );
        if !self.cells.is_empty() {
            for y in y0..=y1 {
                let row = (y - self.first.1) as usize * self.columns as usize;
                for x in x0..=x1 {
                    ids.extend(
                        self.cells[row + (x - self.first.0) as usize]
                            .iter()
                            .map(|id| *id as usize),
                    );
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
/// Rebuild after every layout. Each routed
/// segment refers to one shared identity slot, rather than owning an edge name.
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    generation: u64,
    index: RectIndex<HitItem>,
    targets: Vec<IndexedTarget>,
}
impl SpatialIndex {
    pub fn build(scene: &Scene) -> Self {
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
                identity: SceneTarget::Port(p.owner, p.id),
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
            generation: scene.generation,
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
            .filter_map(|hit| match hit.segment {
                Some((a, b)) => {
                    let distance = segment_distance(point, a, b);
                    (distance <= tolerance).then_some((hit, distance))
                }
                None => hit
                    .bounds
                    .inflate(tolerance.min(3.0))
                    .contains(point)
                    .then_some((hit, 0.0)),
            })
            .min_by(|(a, a_distance), (b, b_distance)| {
                a.priority
                    .cmp(&b.priority)
                    // Parallel orthogonal segments all have zero-area bounds.
                    // Within the existing port/node/edge priority, choose the
                    // line nearest the pointer before using identity as a tie.
                    .then_with(|| a_distance.total_cmp(b_distance))
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
            .map(|(item, _)| self.targets[item.target].identity.clone())
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
        scene: &'scene Scene,
        bounds: Rect,
    ) -> VisibleScene<'scene> {
        if scene.generation != self.generation {
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
                SceneTarget::Port(owner, id) => {
                    if let Some(port) = scene
                        .ports
                        .get(target.scene_index)
                        .filter(|port| port.id == *id && port.owner == *owner)
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

#[cfg(test)]
mod grid_tests {
    use super::*;

    #[test]
    fn the_grid_grows_in_every_direction_and_far_items_overflow() {
        let mut index = RectIndex::new(100.0);
        let places = [
            (0.0, 0.0),
            (-950.0, 20.0),
            (30.0, -1_720.0),
            (4_000.0, 3_100.0),
            (-12.5, 7_777.0),
            (1.0e9, -1.0e9),
        ];
        for (i, (x, y)) in places.iter().enumerate() {
            index.insert(Rect::new(*x, *y, 40.0, 30.0), i);
        }
        assert!(index.cells.len() as i64 <= MAX_CELLS);
        assert_eq!(index.large, vec![5]);
        for (i, (x, y)) in places.iter().enumerate() {
            let hits = index.query(Rect::new(*x + 10.0, *y + 10.0, 1.0, 1.0));
            assert_eq!(hits, vec![&i], "item {i}");
        }
        assert!(
            index
                .query(Rect::new(2_000.0, 2_000.0, 5.0, 5.0))
                .is_empty()
        );
    }

    #[test]
    fn items_very_far_apart_overflow_instead_of_growing_the_grid() {
        let mut index = RectIndex::new(100.0);
        index.insert(Rect::new(-1.0e12, -1.0e12, 40.0, 30.0), 0);
        index.insert(Rect::new(1.5e11, 1.5e11, 40.0, 30.0), 1);
        index.insert(Rect::new(3.0e11, -3.0e11, 40.0, 30.0), 2);
        assert!(index.cells.len() as i64 <= MAX_CELLS);
        for (i, (x, y)) in [(-1.0e12, -1.0e12), (1.5e11, 1.5e11), (3.0e11, -3.0e11)]
            .into_iter()
            .enumerate()
        {
            let hits = index.query(Rect::new(x + 10.0, y + 10.0, 1.0, 1.0));
            assert_eq!(hits, vec![&i], "item {i}");
        }
    }
}
