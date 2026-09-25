use crate::{DiffMark, Point, PortSide, Rect, SceneEdge, SceneNode, ScenePort, spatial::RectIndex};
use agq_kernel::ElementId;
use agq_modeling_view::ViewEdge;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteQuality {
    Clear,
    Obstructed,
}
#[derive(Clone, Copy)]
struct Endpoint {
    point: Point,
    stub: Point,
    owner: ElementId,
    bounds: Rect,
}
pub(crate) fn route_edges(
    nodes: &[SceneNode],
    ports: &[ScenePort],
    edges: &[ViewEdge],
) -> Vec<SceneEdge> {
    let nodes: BTreeMap<_, _> = nodes.iter().map(|n| (n.id(), n)).collect();
    let ports: BTreeMap<_, _> = ports.iter().map(|p| (p.id, p)).collect();
    let mut obstacles = RectIndex::new(320.0);
    for n in nodes.values().filter(|n| !n.is_container || n.collapsed) {
        obstacles.insert(n.bounds.inflate(10.0), (n.id(), n.bounds.inflate(10.0)));
    }
    let mut parallel = BTreeMap::<(ElementId, ElementId), usize>::new();
    let mut result = Vec::new();
    let mut edges: Vec<_> = edges.iter().collect();
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    for edge in edges {
        let Some(source_center) = center(edge.source, &nodes, &ports) else {
            continue;
        };
        let Some(target_center) = center(edge.target, &nodes, &ports) else {
            continue;
        };
        let Some(source) = endpoint(edge.source, target_center, &nodes, &ports) else {
            continue;
        };
        let Some(target) = endpoint(edge.target, source_center, &nodes, &ports) else {
            continue;
        };
        let pair = if edge.source <= edge.target {
            (edge.source, edge.target)
        } else {
            (edge.target, edge.source)
        };
        let index = parallel.entry(pair).or_default();
        let lane = *index as f32 * 11.0;
        *index += 1;
        let (points, quality) = route(source, target, lane, &obstacles);
        let bounds = points
            .iter()
            .copied()
            .map(|p| Rect::new(p.x, p.y, 0.0, 0.0))
            .reduce(Rect::union)
            .unwrap_or_default();
        result.push(SceneEdge {
            semantic: edge.clone(),
            points,
            bounds,
            quality,
            diff: DiffMark::Unchanged,
        });
    }
    result
}
fn center(
    id: ElementId,
    nodes: &BTreeMap<ElementId, &SceneNode>,
    ports: &BTreeMap<ElementId, &ScenePort>,
) -> Option<Point> {
    ports
        .get(&id)
        .map(|p| p.position)
        .or_else(|| nodes.get(&id).map(|n| n.bounds.center()))
}
fn endpoint(
    id: ElementId,
    toward: Point,
    nodes: &BTreeMap<ElementId, &SceneNode>,
    ports: &BTreeMap<ElementId, &ScenePort>,
) -> Option<Endpoint> {
    if let Some(p) = ports.get(&id) {
        let bounds = nodes.get(&p.owner)?.bounds;
        let stub = match p.side {
            PortSide::Left => Point::new(p.position.x - 22.0, p.position.y),
            PortSide::Right => Point::new(p.position.x + 22.0, p.position.y),
            PortSide::Top => Point::new(p.position.x, p.position.y - 22.0),
            PortSide::Bottom => Point::new(p.position.x, p.position.y + 22.0),
        };
        return Some(Endpoint {
            point: p.position,
            stub,
            owner: p.owner,
            bounds,
        });
    }
    let bounds = nodes.get(&id)?.bounds;
    let center = bounds.center();
    let right = toward.x >= center.x;
    let x = if right { bounds.max.x } else { bounds.min.x };
    let point = Point::new(x, center.y);
    Some(Endpoint {
        point,
        stub: Point::new(x + if right { 22.0 } else { -22.0 }, center.y),
        owner: id,
        bounds,
    })
}
fn route(
    source: Endpoint,
    target: Endpoint,
    lane: f32,
    obstacles: &RectIndex<(ElementId, Rect)>,
) -> (Vec<Point>, RouteQuality) {
    let bounds = source.bounds.union(target.bounds);
    if source.owner == target.owner {
        let x = bounds.max.x + 34.0 + lane;
        let y = bounds.min.y - 30.0 - lane;
        let points = simplify(vec![
            source.point,
            source.stub,
            Point::new(x, source.stub.y),
            Point::new(x, y),
            Point::new(target.stub.x, y),
            target.stub,
            target.point,
        ]);
        let clear = blockers(&points, source.owner, target.owner, obstacles).is_empty();
        return (
            points,
            if clear {
                RouteQuality::Clear
            } else {
                RouteQuality::Obstructed
            },
        );
    }
    let a = source.stub;
    let b = target.stub;
    let via_x = |x: f32| {
        simplify(vec![
            source.point,
            a,
            Point::new(x, a.y),
            Point::new(x, b.y),
            b,
            target.point,
        ])
    };
    let via_y = |y: f32| {
        simplify(vec![
            source.point,
            a,
            Point::new(a.x, y),
            Point::new(b.x, y),
            b,
            target.point,
        ])
    };
    let mut candidates = vec![
        via_x((a.x + b.x) * 0.5 + lane),
        via_y((a.y + b.y) * 0.5 + lane),
        via_y(bounds.min.y - 28.0 - lane),
        via_y(bounds.max.y + 28.0 + lane),
        via_x(bounds.min.x - 28.0 - lane),
        via_x(bounds.max.x + 28.0 + lane),
    ];
    let mut best = None::<(usize, f32, Vec<Point>)>;
    let mut index = 0;
    while index < candidates.len() && index < 24 {
        let path = &candidates[index];
        let collisions = blockers(path, source.owner, target.owner, obstacles);
        let length = path.windows(2).map(|p| p[0].distance(p[1])).sum::<f32>();
        if best.as_ref().is_none_or(|(count, old_length, _)| {
            collisions.len() < *count || (collisions.len() == *count && length < *old_length)
        }) {
            best = Some((collisions.len(), length, path.clone()));
        }
        // Initial direct candidate wins if clear; otherwise examine alternate
        // corridors and the near sides of encountered obstacles, bounded work.
        if index == 0 && collisions.is_empty() {
            break;
        }
        if index < 2 {
            for r in collisions.into_iter().take(4) {
                candidates.push(via_x(r.min.x - 16.0 - lane));
                candidates.push(via_x(r.max.x + 16.0 + lane));
                candidates.push(via_y(r.min.y - 16.0 - lane));
                candidates.push(via_y(r.max.y + 16.0 + lane));
            }
        }
        index += 1;
    }
    let (count, _, points) = best.expect("router always evaluates a candidate");
    (
        points,
        if count == 0 {
            RouteQuality::Clear
        } else {
            RouteQuality::Obstructed
        },
    )
}
fn blockers(
    points: &[Point],
    source: ElementId,
    target: ElementId,
    index: &RectIndex<(ElementId, Rect)>,
) -> Vec<Rect> {
    let mut blockers = BTreeMap::new();
    for pair in points.windows(2) {
        let line = Rect::from_points(pair[0], pair[1]);
        for (id, bounds) in index.query(line) {
            if *id == source || *id == target {
                continue;
            }
            // Touching a corridor boundary is allowed; crossing its interior is not.
            let cross = if pair[0].x == pair[1].x {
                pair[0].x > bounds.min.x
                    && pair[0].x < bounds.max.x
                    && line.max.y > bounds.min.y
                    && line.min.y < bounds.max.y
            } else {
                pair[0].y > bounds.min.y
                    && pair[0].y < bounds.max.y
                    && line.max.x > bounds.min.x
                    && line.min.x < bounds.max.x
            };
            if cross {
                blockers.insert(*id, *bounds);
            }
        }
    }
    blockers.into_values().collect()
}
fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut result: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if result.last() == Some(&p) {
            continue;
        }
        while result.len() >= 2 {
            let a = result[result.len() - 2];
            let b = result[result.len() - 1];
            // Preserve reversal stubs: removing those could send an edge through
            // the node from whose semantic port it exits.
            let collinear = (a.x == b.x && b.x == p.x && (b.y - a.y) * (p.y - b.y) >= 0.0)
                || (a.y == b.y && b.y == p.y && (b.x - a.x) * (p.x - b.x) >= 0.0);
            if collinear {
                result.pop();
            } else {
                break;
            }
        }
        result.push(p);
    }
    result
}
