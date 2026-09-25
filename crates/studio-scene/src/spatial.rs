use crate::{Point, Rect, SceneTarget, SemanticScene, segment_distance};
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
        if (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1) > 128 {
            self.large.push(id);
            return;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cells.entry((x, y)).or_default().push(id);
            }
        }
    }
    pub fn query(&self, r: Rect) -> Vec<&T> {
        let (x0, y0, x1, y1) = self.range(r);
        if (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1) > 4096 {
            return self
                .items
                .iter()
                .filter(|(bounds, _)| bounds.intersects(r))
                .map(|(_, v)| v)
                .collect();
        }
        let mut ids = BTreeSet::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if let Some(cell) = self.cells.get(&(x, y)) {
                    ids.extend(cell.iter().copied());
                }
            }
        }
        ids.extend(self.large.iter().copied());
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
    target: SceneTarget,
    bounds: Rect,
    segment: Option<(Point, Point)>,
    priority: u8,
}
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    index: RectIndex<HitItem>,
}
impl SpatialIndex {
    pub fn build(scene: &SemanticScene) -> Self {
        let mut index = RectIndex::new(256.0);
        for n in &scene.nodes {
            let target = if n.is_container {
                SceneTarget::Container(n.id())
            } else {
                SceneTarget::Node(n.id())
            };
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
        for p in &scene.ports {
            let bounds = Rect::new(p.position.x - 7.0, p.position.y - 7.0, 14.0, 14.0);
            index.insert(
                bounds,
                HitItem {
                    target: SceneTarget::Port(p.id),
                    bounds,
                    segment: None,
                    priority: 0,
                },
            );
        }
        for edge in &scene.edges {
            for pair in edge.points.windows(2) {
                let bounds = Rect::from_points(pair[0], pair[1]);
                index.insert(
                    bounds,
                    HitItem {
                        target: SceneTarget::Edge(edge.semantic.id.clone()),
                        bounds,
                        segment: Some((pair[0], pair[1])),
                        priority: 2,
                    },
                );
            }
        }
        Self { index }
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
                    .then_with(|| a.target.cmp(&b.target))
            })
            .map(|item| item.target.clone())
    }
    /// Culling includes edges crossing the viewport with both endpoints outside.
    pub fn query(&self, bounds: Rect) -> Vec<SceneTarget> {
        self.index
            .query(bounds)
            .into_iter()
            .map(|i| i.target.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    /// Fully enclosed nodes and ports; containers only if their entire box fits.
    pub fn marquee(&self, bounds: Rect) -> Vec<SceneTarget> {
        self.index
            .query(bounds)
            .into_iter()
            .filter(|i| i.segment.is_none() && bounds.contains_rect(i.bounds))
            .map(|i| i.target.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    pub fn visible(&self, bounds: Rect) -> Vec<SceneTarget> {
        self.query(bounds)
    }
}
