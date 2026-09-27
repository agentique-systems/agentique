//! CPU-side budgets checked in CI (ROADMAP §3.3, R-27): building the Surface's
//! scene (layout, routing, indexing) at 1,000 and 10,000 elements.
//!
//! Every change rebuilds the whole scene today, so a scene build is the
//! Surface part of "edit to Surface". The targets (50 ms at 1k, 100 ms at 10k,
//! C-33) are not met yet: incremental layout is W5.5. Until then the ceilings
//! below guard against regressions at about three times today's reference
//! measurements (§5.2: 114 ms and 2,662 ms), and are tightened as W5.5 lands.
//! Budgets are measured in release builds:
//!
//! ```text
//! cargo test --release -p agq-studio-scene --test budgets
//! ```

use agq_studio_scene::{LayoutKind, Scene, SceneOptions, fixtures};
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

#[test]
#[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
fn scene_build_at_one_thousand_elements() {
    let elapsed = scene_build(1_000, 3);
    println!("scene build, 1k elements: {elapsed:?} (target 50 ms, W5.5)");
    assert!(elapsed < Duration::from_millis(350), "{elapsed:?}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
fn scene_build_at_ten_thousand_elements() {
    let elapsed = scene_build(10_000, 1);
    println!("scene build, 10k elements: {elapsed:?} (target 100 ms, W5.5)");
    assert!(elapsed < Duration::from_millis(8_000), "{elapsed:?}");
}
