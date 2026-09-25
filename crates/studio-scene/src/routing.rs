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
    let attachments = node_attachments(&nodes, &ports, &edges);
    for edge in edges {
        let Some(source_center) = center(edge.source, &nodes, &ports) else {
            continue;
        };
        let Some(target_center) = center(edge.target, &nodes, &ports) else {
            continue;
        };
        let Some(source) = endpoint(
            edge.source,
            target_center,
            attachments.get(&(edge.id.as_str(), true)),
            &nodes,
            &ports,
        ) else {
            continue;
        };
        let Some(target) = endpoint(
            edge.target,
            source_center,
            attachments.get(&(edge.id.as_str(), false)),
            &nodes,
            &ports,
        ) else {
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
/// Separate relationship attachment lanes on ordinary node boundaries. These
/// positions are drawing geometry only: they introduce no ScenePort or semantic
/// identity. Actual modeled ports always retain their exact position.
fn node_attachments<'a>(
    nodes: &BTreeMap<ElementId, &SceneNode>,
    ports: &BTreeMap<ElementId, &ScenePort>,
    edges: &[&'a ViewEdge],
) -> BTreeMap<(&'a str, bool), (f32, f32)> {
    let mut sides = BTreeMap::<(ElementId, bool), Vec<(&str, bool, f32)>>::new();
    for edge in edges {
        for (id, other, source) in [
            (edge.source, edge.target, true),
            (edge.target, edge.source, false),
        ] {
            if ports.contains_key(&id) {
                continue;
            }
            let (Some(node), Some(toward)) = (nodes.get(&id), center(other, nodes, ports)) else {
                continue;
            };
            sides
                .entry((id, toward.x >= node.bounds.center().x))
                .or_default()
                .push((edge.id.as_str(), source, toward.y));
        }
    }
    let mut attachments = BTreeMap::new();
    for ((id, _), mut entries) in sides {
        // Neighbor order prevents avoidable local crossings. Identity is the
        // tie-breaker, so transport order cannot change the drawing.
        entries.sort_by(|a, b| {
            a.2.total_cmp(&b.2)
                .then_with(|| a.0.cmp(b.0))
                .then_with(|| a.1.cmp(&b.1))
        });
        let bounds = nodes[&id].bounds;
        let spacing = ((bounds.height() - 36.0).max(0.0)
            / entries.len().saturating_sub(1).max(1) as f32)
            .min(18.0);
        let middle = (entries.len() - 1) as f32 * 0.5;
        for (index, (edge, source, _)) in entries.into_iter().enumerate() {
            let y = bounds.center().y + (index as f32 - middle) * spacing;
            // Stagger the nearby turn as well as the endpoint, keeping the
            // final corridor and arrowhead independently traceable.
            // Stay inside the 42-world-unit minimum node gutter, including
            // obstacle clearance; long stubs can pierce unrelated neighbors.
            let stub_length = 22.0 + (index % 3) as f32 * 3.0;
            attachments.insert((edge, source), (y, stub_length));
        }
    }
    attachments
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
    attachment: Option<&(f32, f32)>,
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
    let (y, stub_length) = attachment.copied().unwrap_or((center.y, 22.0));
    let point = Point::new(x, y);
    Some(Endpoint {
        point,
        stub: Point::new(x + if right { stub_length } else { -stub_length }, y),
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
    if lane > 0.0 && (a.y - b.y).abs() < 1.0 {
        candidates.insert(0, via_y(a.y + 24.0 + lane));
    }
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
