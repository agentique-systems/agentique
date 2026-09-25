use agq_modeling_workspace::ProjectRevisionId;
use agq_studio_scene::*;
use std::{
    collections::BTreeSet,
    hash::{DefaultHasher, Hash, Hasher},
};

fn architecture() -> SemanticScene {
    SemanticScene::from_projection(&fixtures::architecture(), &SceneOptions::default(), None)
        .unwrap()
}

fn identities(visible: &VisibleScene<'_>) -> Vec<SceneTarget> {
    visible
        .nodes
        .iter()
        .map(|node| {
            if node.is_container {
                SceneTarget::Container(node.id())
            } else {
                SceneTarget::Node(node.id())
            }
        })
        .chain(visible.ports.iter().map(|port| SceneTarget::Port(port.id)))
        .chain(
            visible
                .edges
                .iter()
                .map(|edge| SceneTarget::Edge(edge.semantic.id.clone())),
        )
        .collect()
}

// A full scan is deliberately independent of the grid and identity lookup.
fn scan(scene: &SemanticScene, bounds: Rect) -> Vec<SceneTarget> {
    let nodes = scene
        .nodes
        .iter()
        .filter(|node| node.bounds.intersects(bounds));
    let ports = scene.ports.iter().filter(|port| {
        Rect::new(port.position.x - 7.0, port.position.y - 7.0, 14.0, 14.0).intersects(bounds)
    });
    let edges = scene.edges.iter().filter(|edge| {
        edge.points
            .windows(2)
            .any(|points| Rect::from_points(points[0], points[1]).intersects(bounds))
    });
    identities(&VisibleScene {
        nodes: nodes.collect(),
        ports: ports.collect(),
        edges: edges.collect(),
    })
}

fn assert_views(scene: &SemanticScene) {
    let spatial = SpatialIndex::build(scene);
    let lookup = SceneLookup::build(scene);
    let bounds = scene.bounds();
    let mut viewports = vec![
        bounds,
        Rect::new(-1.0e20, -1.0e20, 2.0e20, 2.0e20),
        Rect::new(-1.0e6, -1.0e6, 10.0, 10.0),
    ];
    for i in 0..32 {
        let t = i as f32 / 31.0;
        viewports.push(Rect::new(
            bounds.min.x + bounds.width() * t,
            bounds.min.y + bounds.height() * (1.0 - t),
            bounds.width() * 0.2,
            bounds.height() * 0.3,
        ));
    }
    for viewport in viewports {
        let borrowed = spatial.visible_scene(scene, viewport);
        let targets = spatial.visible(viewport);
        let legacy = lookup.visible(scene, &targets);
        let actual = identities(&borrowed);
        assert_eq!(
            actual,
            scan(scene, viewport),
            "exact scan identities and scene order"
        );
        assert_eq!(actual, identities(&legacy), "old public API equivalence");
        assert_eq!(
            actual.len(),
            actual.iter().collect::<BTreeSet<_>>().len(),
            "one edge per identity regardless of intersecting segments"
        );
    }
}

#[test]
fn borrowed_culling_matches_scan_and_legacy_on_adversarial_scenes() {
    assert_views(&architecture());
    for (_, projection) in fixtures::adversarial() {
        for hierarchy in [false, true] {
            let scene = SemanticScene::from_projection(
                &projection,
                &SceneOptions {
                    hierarchy,
                    ..Default::default()
                },
                None,
            )
            .unwrap();
            assert_views(&scene);
        }
    }
}

#[test]
fn borrowed_culling_preserves_collapsed_proxy_ports_and_diff_ghosts() {
    let mut options = SceneOptions::default();
    options.collapsed.insert(fixtures::id(2));
    let scene = SemanticScene::from_projection(&fixtures::architecture(), &options, None).unwrap();
    assert!(scene.ports.iter().any(|p| p.proxy_for_owner.is_some()));
    assert_views(&scene);
    let (before, after) = fixtures::revision_diff();
    let before = SemanticScene::from_projection(&before, &SceneOptions::default(), None).unwrap();
    let mut after =
        SemanticScene::from_projection(&after, &SceneOptions::default(), Some(before.memory()))
            .unwrap();
    after.apply_diff(&before);
    assert!(after.nodes.iter().any(|n| n.diff == DiffMark::Removed));
    assert_views(&after);
}

#[test]
fn cross_viewport_relationships_are_retained_without_visible_endpoints() {
    let mut scene = architecture();
    scene.edges[0].points = vec![
        Point::new(-10_000.0, -10_000.0),
        Point::new(10_000.0, -10_000.0),
    ];
    let spatial = SpatialIndex::build(&scene);
    let visible = spatial.visible_scene(&scene, Rect::new(-10.0, -10_001.0, 20.0, 2.0));
    assert!(visible.nodes.is_empty());
    assert!(visible.ports.is_empty());
    assert_eq!(visible.edges.len(), 1);
    assert_eq!(visible.edges[0].semantic.id, scene.edges[0].semantic.id);
}

#[test]
fn stale_slots_cannot_borrow_different_identities_or_revisions() {
    let scene = architecture();
    let index = SpatialIndex::build(&scene);
    let all = Rect::new(-1.0e20, -1.0e20, 2.0e20, 2.0e20);
    let mut changed = scene.clone();
    changed.revision_id = ProjectRevisionId::from_u128(9);
    assert!(identities(&index.visible_scene(&changed, all)).is_empty());
    changed = scene.clone();
    changed.nodes.rotate_left(1);
    changed.ports.rotate_left(1);
    changed.edges.rotate_left(1);
    assert!(identities(&index.visible_scene(&changed, all)).is_empty());
    changed.nodes.clear();
    changed.ports.clear();
    changed.edges.clear();
    assert!(identities(&index.visible_scene(&changed, all)).is_empty());
}

#[test]
fn borrowed_identity_hash_tracks_visible_identity_without_cloning_records() {
    fn digest(visible: &VisibleScene<'_>) -> u64 {
        let mut state = DefaultHasher::new();
        visible.hash(&mut state);
        state.finish()
    }
    let scene = architecture();
    let index = SpatialIndex::build(&scene);
    let visible = index.visible_scene(&scene, scene.bounds());
    assert_eq!(
        digest(&visible),
        digest(&index.visible_scene(&scene, scene.bounds()))
    );
    let empty = index.visible_scene(&scene, Rect::new(-1.0e6, -1.0e6, 1.0, 1.0));
    assert_ne!(digest(&visible), digest(&empty));
    for node in visible.nodes {
        assert!(
            scene
                .nodes
                .iter()
                .any(|original| std::ptr::eq(original, node))
        );
    }
}
