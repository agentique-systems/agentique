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
    element: ElementId,
    point: Point,
    stub: Point,
    owner: ElementId,
    bounds: Rect,
    side: PortSide,
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
            element: id,
            point: p.position,
            stub,
            owner: p.owner,
            bounds,
            side: p.side,
        });
    }
    let bounds = nodes.get(&id)?.bounds;
    let center = bounds.center();
    let right = toward.x >= center.x;
    let x = if right { bounds.max.x } else { bounds.min.x };
    let (y, stub_length) = attachment.copied().unwrap_or((center.y, 22.0));
    let point = Point::new(x, y);
    Some(Endpoint {
        element: id,
        point,
        stub: Point::new(x + if right { stub_length } else { -stub_length }, y),
        owner: id,
        bounds,
        side: if right {
            PortSide::Right
        } else {
            PortSide::Left
        },
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
        return route_around_owner(source, target, lane, obstacles);
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

/// The owner is not an obstacle to its own endpoints, but its interior is not a
/// shortcut between boundary ports. Walk an exterior perimeter in either
/// direction; this also avoids child cards when they are not yet expanded.
fn route_around_owner(
    source: Endpoint,
    target: Endpoint,
    lane: f32,
    obstacles: &RectIndex<(ElementId, Rect)>,
) -> (Vec<Point>, RouteQuality) {
    let mut best = None::<(usize, f32, Vec<Point>)>;
    for extra in [0.0, 24.0, 64.0, 128.0] {
        let clearance = 34.0 + lane + extra;
        for clockwise in [true, false] {
            let points = if source.side == target.side
                && (source.element == target.element || source.point == target.point)
            {
                owner_loop(source, target, clearance, clockwise)
            } else {
                owner_perimeter(source, target, clearance, clockwise)
            };
            let collisions = blockers(&points, source.owner, target.owner, obstacles).len();
            let length = points.windows(2).map(|p| p[0].distance(p[1])).sum();
            if best.as_ref().is_none_or(|(count, old_length, _)| {
                collisions < *count || (collisions == *count && length < *old_length)
            }) {
                best = Some((collisions, length, points));
            }
        }
        // Wider rectangles cannot shorten an already clear perimeter. Both
        // directions above were considered before accepting this envelope.
        if best.as_ref().is_some_and(|(count, _, _)| *count == 0) {
            break;
        }
    }
    let (collisions, _, points) = best.expect("owner router evaluates bounded alternatives");
    (
        points,
        if collisions == 0 {
            RouteQuality::Clear
        } else {
            RouteQuality::Obstructed
        },
    )
}

fn owner_loop(source: Endpoint, target: Endpoint, clearance: f32, clockwise: bool) -> Vec<Point> {
    // Node self-relationships have separate attachment lanes. Draw the loop
    // beyond one attachment, or reverse the walk to try beyond the other;
    // the shorter perimeter between them would collapse into a small U.
    let (start, finish) = if clockwise {
        (source, target)
    } else {
        (target, source)
    };
    let outward = match source.side {
        PortSide::Left => Point::new(-1.0, 0.0),
        PortSide::Right => Point::new(1.0, 0.0),
        PortSide::Top => Point::new(0.0, -1.0),
        PortSide::Bottom => Point::new(0.0, 1.0),
    };
    let tangent = Point::new(-outward.y, outward.x);
    let along =
        (finish.point.x - start.point.x) * tangent.x + (finish.point.y - start.point.y) * tangent.y;
    let sign = if along > 0.0 || (along == 0.0 && clockwise) {
        1.0
    } else {
        -1.0
    };
    let beyond = along + sign * clearance;
    // Ordinary node attachments stagger stub lengths (22/25/28). Both
    // endpoints can occupy the same boundary point with different stubs.
    let source_length = start.point.distance(start.stub);
    let target_length = finish.point.distance(finish.stub);
    let far = clearance.max(source_length.max(target_length) + 12.0);
    let point = |out: f32, along: f32| {
        Point::new(
            start.point.x + outward.x * out + tangent.x * along,
            start.point.y + outward.y * out + tangent.y * along,
        )
    };
    let mut points = vec![
        start.point,
        start.stub,
        point(far, 0.0),
        point(far, beyond),
        point(target_length, beyond),
        finish.stub,
        finish.point,
    ];
    if !clockwise {
        points.reverse();
    }
    simplify(points)
}

fn owner_perimeter(
    source: Endpoint,
    target: Endpoint,
    clearance: f32,
    clockwise: bool,
) -> Vec<Point> {
    let bounds = source.bounds.union(target.bounds).inflate(clearance);
    let width = bounds.width();
    let height = bounds.height();
    let perimeter = 2.0 * (width + height);
    let exit = |endpoint: Endpoint| match endpoint.side {
        PortSide::Top => (
            Point::new(endpoint.point.x, bounds.min.y),
            endpoint.point.x - bounds.min.x,
        ),
        PortSide::Right => (
            Point::new(bounds.max.x, endpoint.point.y),
            width + endpoint.point.y - bounds.min.y,
        ),
        PortSide::Bottom => (
            Point::new(endpoint.point.x, bounds.max.y),
            width + height + bounds.max.x - endpoint.point.x,
        ),
        PortSide::Left => (
            Point::new(bounds.min.x, endpoint.point.y),
            2.0 * width + height + bounds.max.y - endpoint.point.y,
        ),
    };
    let (start, finish) = if clockwise {
        (source, target)
    } else {
        (target, source)
    };
    let (a, start_distance) = exit(start);
    let (b, mut end_distance) = exit(finish);
    if end_distance <= start_distance {
        end_distance += perimeter;
    }
    let corners = [
        (0.0, bounds.min),
        (width, Point::new(bounds.max.x, bounds.min.y)),
        (width + height, bounds.max),
        (2.0 * width + height, Point::new(bounds.min.x, bounds.max.y)),
    ];
    let mut points = vec![start.point, start.stub, a];
    for lap in [0.0, perimeter] {
        for (distance, point) in corners {
            if distance + lap > start_distance && distance + lap < end_distance {
                points.push(point);
            }
        }
    }
    points.extend([b, finish.stub, finish.point]);
    if !clockwise {
        points.reverse();
    }
    simplify(points)
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

#[cfg(test)]
mod owner_route_tests {
    use super::*;
    use crate::{NodeCategory, PortDirection, fixtures};
    use agq_modeling_view::ViewOrigin;

    fn node(id: u128, bounds: Rect, container: bool) -> SceneNode {
        let mut semantic = fixtures::architecture().nodes[0].clone();
        semantic.id = fixtures::id(id);
        SceneNode {
            semantic,
            category: NodeCategory::Part,
            bounds,
            depth: usize::from(!container),
            is_container: container,
            collapsed: false,
            diff: DiffMark::Unchanged,
        }
    }
    fn port(id: u128, side: PortSide, position: Point) -> ScenePort {
        ScenePort {
            id: fixtures::id(id),
            revision_id: fixtures::revision(),
            owner: fixtures::id(1),
            proxy_for_owner: None,
            name: format!("port{id}"),
            position,
            label_in_header: false,
            side,
            direction: PortDirection::Unspecified,
            origin: ViewOrigin::Authored,
            diff: DiffMark::Unchanged,
        }
    }
    fn edge(source: &ScenePort, target: &ScenePort) -> ViewEdge {
        let mut edge = fixtures::architecture().edges[0].clone();
        edge.source = source.id;
        edge.target = target.id;
        edge
    }
    fn assert_boundary_route(
        route: &SceneEdge,
        source: &ScenePort,
        target: &ScenePort,
        owner: Rect,
    ) {
        assert_eq!(route.points.first(), Some(&source.position));
        assert_eq!(route.points.last(), Some(&target.position));
        assert!(
            route
                .points
                .windows(2)
                .all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y)
        );
        let mut interior = RectIndex::new(320.0);
        interior.insert(owner, (fixtures::id(1), owner));
        assert!(
            blockers(
                &route.points,
                fixtures::id(999),
                fixtures::id(999),
                &interior
            )
            .is_empty(),
            "a boundary connection must not cross the owner's body: {:?}",
            route.points
        );
    }

    #[test]
    fn platform_boundary_connection_avoids_actual_style_child_grid_and_tries_other_perimeter() {
        // Same geometry failure as real-run04: opposite boundary ports at the
        // second child row, with nine cards occupying the expanded owner body.
        let owner = Rect::new(0.0, 0.0, 844.0, 542.0);
        let mut nodes = vec![node(1, owner, true)];
        for row in 0..3 {
            for column in 0..3 {
                let mut child = node(
                    10 + row * 3 + column,
                    Rect::new(
                        30.0 + column as f32 * 276.0,
                        72.0 + row as f32 * 162.0,
                        232.0,
                        118.0,
                    ),
                    false,
                );
                child.semantic.owner = Some(fixtures::id(1));
                nodes.push(child);
            }
        }
        // The shorter lower perimeter is blocked; the upper one stays clear.
        nodes.push(node(50, Rect::new(-100.0, 550.0, 1044.0, 80.0), false));
        let ports = [
            port(1001, PortSide::Left, Point::new(0.0, 292.0)),
            port(1002, PortSide::Right, Point::new(844.0, 292.0)),
        ];
        let semantic = edge(&ports[0], &ports[1]);
        let routes = route_edges(&nodes, &ports, std::slice::from_ref(&semantic));
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].semantic, semantic);
        assert_eq!(routes[0].quality, RouteQuality::Clear);
        assert_boundary_route(&routes[0], &ports[0], &ports[1], owner);
        assert!(routes[0].points.iter().any(|point| point.y < owner.min.y));
    }

    #[test]
    fn every_pair_of_boundary_sides_and_same_port_loops_preserves_exact_endpoints() {
        let owner = Rect::new(100.0, 80.0, 300.0, 220.0);
        let nodes = [node(1, owner, true)];
        let position = |side, second| {
            let fraction = if second { 0.7 } else { 0.3 };
            match side {
                PortSide::Left => Point::new(owner.min.x, owner.min.y + owner.height() * fraction),
                PortSide::Right => Point::new(owner.max.x, owner.min.y + owner.height() * fraction),
                PortSide::Top => Point::new(owner.min.x + owner.width() * fraction, owner.min.y),
                PortSide::Bottom => Point::new(owner.min.x + owner.width() * fraction, owner.max.y),
            }
        };
        for a in [
            PortSide::Left,
            PortSide::Right,
            PortSide::Top,
            PortSide::Bottom,
        ] {
            for b in [
                PortSide::Left,
                PortSide::Right,
                PortSide::Top,
                PortSide::Bottom,
            ] {
                let ports = [
                    port(1001, a, position(a, false)),
                    port(1002, b, position(b, true)),
                ];
                let routes = route_edges(&nodes, &ports, &[edge(&ports[0], &ports[1])]);
                assert_eq!(routes[0].quality, RouteQuality::Clear);
                assert_boundary_route(&routes[0], &ports[0], &ports[1], owner);
            }
            let port = port(1001, a, position(a, false));
            let routes = route_edges(&nodes, std::slice::from_ref(&port), &[edge(&port, &port)]);
            assert_eq!(routes[0].quality, RouteQuality::Clear);
            assert_boundary_route(&routes[0], &port, &port, owner);
            assert!(routes[0].points.len() >= 5);
            assert!(routes[0].bounds.width() > 0.0 && routes[0].bounds.height() > 0.0);
        }
    }

    #[test]
    fn coincident_generic_anchors_with_different_stubs_stay_orthogonal() {
        let bounds = Rect::new(100.0, 80.0, 300.0, 220.0);
        for (side, point, outward) in [
            (
                PortSide::Left,
                Point::new(100.0, 190.0),
                Point::new(-1.0, 0.0),
            ),
            (
                PortSide::Right,
                Point::new(400.0, 190.0),
                Point::new(1.0, 0.0),
            ),
            (
                PortSide::Top,
                Point::new(250.0, 80.0),
                Point::new(0.0, -1.0),
            ),
            (
                PortSide::Bottom,
                Point::new(250.0, 300.0),
                Point::new(0.0, 1.0),
            ),
        ] {
            for (source_length, target_length) in [(25.0, 28.0), (40.0, 25.0)] {
                let endpoint = |length| Endpoint {
                    element: fixtures::id(1),
                    point,
                    stub: Point::new(point.x + outward.x * length, point.y + outward.y * length),
                    owner: fixtures::id(1),
                    bounds,
                    side,
                };
                let source = endpoint(source_length);
                let target = endpoint(target_length);
                let (points, quality) = route(source, target, 0.0, &RectIndex::new(320.0));
                assert_eq!(quality, RouteQuality::Clear);
                assert_eq!(points.first(), Some(&point));
                assert_eq!(points.last(), Some(&point));
                assert_eq!(points[points.len() - 2], target.stub);
                assert!(points.len() >= 5);
                assert!(
                    points
                        .windows(2)
                        .all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y),
                    "non-default stubs must not introduce a diagonal: {points:?}"
                );
                let mut interior = RectIndex::new(320.0);
                interior.insert(bounds, (fixtures::id(1), bounds));
                assert!(
                    blockers(&points, fixtures::id(999), fixtures::id(999), &interior).is_empty()
                );
            }
        }
    }

    #[test]
    fn owner_perimeter_parallel_lanes_stay_distinct_and_unavoidable_obstructions_stay_honest() {
        let owner = Rect::new(0.0, 0.0, 400.0, 240.0);
        let mut nodes = vec![node(1, owner, true)];
        let ports = [
            port(1001, PortSide::Left, Point::new(0.0, 120.0)),
            port(1002, PortSide::Right, Point::new(400.0, 120.0)),
        ];
        let first = edge(&ports[0], &ports[1]);
        let mut second = first.clone();
        second.id.push_str("-parallel");
        second.relationship_id = Some(fixtures::id(7000));
        let edges = [first, second];
        let routes = route_edges(&nodes, &ports, &edges);
        assert_ne!(routes[0].points, routes[1].points);
        for route in &routes {
            assert_boundary_route(route, &ports[0], &ports[1], owner);
            assert_eq!(route.quality, RouteQuality::Clear);
        }
        assert_eq!(
            routes
                .iter()
                .map(|route| &route.semantic)
                .collect::<Vec<_>>(),
            edges.iter().collect::<Vec<_>>()
        );
        nodes.push(node(50, Rect::new(-12.0, 110.0, 24.0, 20.0), false));
        let blocked = route_edges(&nodes, &ports, &edges);
        assert!(
            blocked
                .iter()
                .all(|route| route.quality == RouteQuality::Obstructed)
        );
        for route in &blocked {
            assert_boundary_route(route, &ports[0], &ports[1], owner);
        }
    }
}
