//! CPU-side budgets checked in CI (ROADMAP §3.3, R-27): the Surface part of
//! "edit to Surface" (a scene update, the spatial index and the lookup) and
//! a full scene build (layout, routing) at 1,000 and 10,000 elements.
//!
//! An edit updates the scene (`Scene::update`, S5.1): cards are laid out
//! again and only the edges the edit touches are routed. The targets are
//! 50 ms at 1k and 100 ms at 10k (C-33); the ceilings are about twice that,
//! so noise does not fail builds. A full build happens when a project opens;
//! its ceilings still guard against regressions at about three times the
//! reference measurements before W5.5 (§5.2: 114 ms and 2,662 ms). Budgets
//! are measured in release builds:
//!
//! ```text
//! cargo test --release -p agq-studio-scene --test budgets -- --nocapture --test-threads=1
//! ```

use agq_studio_scene::{
    EdgeKind, LayoutKind, Scene, SceneLookup, SceneOptions, SpatialIndex, fixtures,
};
use std::time::{Duration, Instant};

/// The best of `runs` builds of a stress fixture, in the graph layout the
/// Studio's stress fixtures use.
fn scene_build(nodes: usize, runs: usize) -> Duration {
    let input = fixtures::stress(nodes, nodes * 2);
    let options = SceneOptions {
        layout: LayoutKind::Graph,
        ..Default::default()
    };
    (0..runs)
        .map(|_| {
            let start = Instant::now();
            let scene = Scene::build(&input, &options, None).expect("the fixture lays out");
            let elapsed = start.elapsed();
            std::hint::black_box(scene);
            elapsed
        })
        .min()
        .expect("at least one run")
}

/// The best of `runs` edits of a stress fixture, the slower of the two
/// layouts: a new card, connected to a card in the middle, reaches the
/// Surface through a scene update, the spatial index and the lookup.
fn edit_to_surface(nodes: usize, runs: usize) -> Duration {
    let before = fixtures::stress(nodes, nodes * 2);
    let mut after = before.clone();
    after.generation += 1;
    let card = before.nodes[before.nodes.len() / 2].clone();
    let mut new = card.clone();
    new.id = fixtures::id(9_000_000);
    new.name = "Added".into();
    new.ports = vec![fixtures::port(9_000_001, "in")];
    after.nodes.push(new);
    after.edges.push(fixtures::edge(
        "added",
        EdgeKind::Connection,
        fixtures::at(card.id.raw(), card.ports[1].id.raw()),
        fixtures::at(9_000_000, 9_000_001),
    ));
    [LayoutKind::Hierarchy, LayoutKind::Graph]
        .into_iter()
        .map(|layout| {
            let options = SceneOptions {
                layout,
                ..Default::default()
            };
            let earlier = Scene::build(&before, &options, None).expect("the fixture lays out");
            (0..runs)
                .map(|_| {
                    let start = Instant::now();
                    let scene = earlier
                        .update(&after, &options, Some(earlier.memory()))
                        .expect("the edit lays out");
                    let index = SpatialIndex::build(&scene);
                    let lookup = SceneLookup::build(&scene);
                    let elapsed = start.elapsed();
                    assert_eq!(scene.routing().routed, 1, "{layout:?}");
                    std::hint::black_box((scene, index, lookup));
                    elapsed
                })
                .min()
                .expect("at least one run")
        })
        .max()
        .expect("two layouts")
}

#[test]
#[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
fn edit_to_surface_at_one_thousand_elements() {
    let elapsed = edit_to_surface(1_000, 5);
    println!("edit to Surface, 1k elements: {elapsed:?} (target 50 ms)");
    assert!(elapsed < Duration::from_millis(100), "{elapsed:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
fn edit_to_surface_at_ten_thousand_elements() {
    let elapsed = edit_to_surface(10_000, 3);
    println!("edit to Surface, 10k elements: {elapsed:?} (target 100 ms, C-33)");
    assert!(elapsed < Duration::from_millis(200), "{elapsed:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
fn scene_build_at_one_thousand_elements() {
    let elapsed = scene_build(1_000, 3);
    println!("scene build, 1k elements: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(350), "{elapsed:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
fn scene_build_at_ten_thousand_elements() {
    let elapsed = scene_build(10_000, 1);
    println!("scene build, 10k elements: {elapsed:?} (project open: target 1 s)");
    assert!(elapsed < Duration::from_millis(8_000), "{elapsed:?}");
}
