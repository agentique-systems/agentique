//! S5.1: what one edit costs at 1k and 10k elements, and whether unrelated
//! cards stay where they were. Builds the stress fixture, makes one edit,
//! and compares a full build with `Scene::update` (both with the previous
//! layout memory, and both followed by the spatial index and the lookup the
//! Studio rebuilds); for both layouts and five kinds of edit.
//!
//! `cargo run --release -p agq-studio-scene --example edit_benchmark`

use agq_studio_scene::{
    EdgeKind, ElementId, LayoutKind, Scene, SceneInput, SceneLookup, SceneOptions, SpatialIndex,
    fixtures,
};
use std::{collections::BTreeSet, time::Instant};

/// One edit to a stress fixture, and the cards it is about.
fn edit(input: &SceneInput, kind: &str) -> (SceneInput, BTreeSet<ElementId>) {
    let mut after = input.clone();
    after.generation += 1;
    let middle = input.nodes.len() / 2;
    let card = input.nodes[middle].clone();
    let mut about = BTreeSet::from([card.id]);
    match kind {
        // A card gets a longer name.
        "rename" => after.nodes[middle].name.push_str(" renamed"),
        // A new card in the same container, connected to the card.
        "add card" => {
            let mut new = card.clone();
            new.id = fixtures::id(9_000_000);
            new.name = "Added".into();
            new.ports = vec![fixtures::port(9_000_001, "in")];
            about.insert(new.id);
            after.nodes.push(new);
            after.edges.push(fixtures::edge(
                "added",
                EdgeKind::Connection,
                fixtures::at(card.id.raw(), card.ports[1].id.raw()),
                fixtures::at(9_000_000, 9_000_001),
            ));
        }
        // The card and its connections are removed.
        "remove card" => {
            after.nodes.retain(|n| n.id != card.id);
            after
                .edges
                .retain(|e| e.source.node != card.id && e.target.node != card.id);
        }
        // A new connection between two existing cards.
        "connect" => {
            let other = &input.nodes[middle + 7];
            about.insert(other.id);
            after.edges.push(fixtures::edge(
                "connected",
                EdgeKind::Connection,
                fixtures::at(card.id.raw(), card.ports[1].id.raw()),
                fixtures::at(other.id.raw(), other.ports[0].id.raw()),
            ));
        }
        // The card gets two more ports, and grows.
        "add ports" => after.nodes[middle].ports.extend([
            fixtures::port(9_000_002, "extra 1"),
            fixtures::port(9_000_003, "extra 2"),
        ]),
        _ => unreachable!("known edit"),
    }
    (after, about)
}

fn ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    for layout in [LayoutKind::Hierarchy, LayoutKind::Graph] {
        for count in [1_000, 10_000] {
            let input = fixtures::stress(count, count * 2);
            let options = SceneOptions {
                layout,
                ..Default::default()
            };
            let started = Instant::now();
            let first = Scene::build(&input, &options, None).expect("lays out");
            let first_ms = ms(started);
            println!("{layout:?} {count}: first build {first_ms:.1} ms");
            for kind in ["rename", "add card", "remove card", "connect", "add ports"] {
                let (after, about) = edit(&input, kind);
                let started = Instant::now();
                let full = Scene::build(&after, &options, Some(first.memory())).expect("lays out");
                let full_ms = ms(started);
                let started = Instant::now();
                let updated = first
                    .update(&after, &options, Some(first.memory()))
                    .expect("lays out");
                let update_ms = ms(started);
                let started = Instant::now();
                let index = SpatialIndex::build(&updated);
                let lookup = SceneLookup::build(&updated);
                let index_ms = ms(started);
                std::hint::black_box((index, lookup));
                let moved = updated
                    .nodes
                    .iter()
                    .filter(|n| !about.contains(&n.id()))
                    .filter(|n| first.node(n.id()).is_some_and(|f| f.bounds != n.bounds))
                    .count();
                let same_cards = full.nodes.len() == updated.nodes.len()
                    && full
                        .nodes
                        .iter()
                        .zip(&updated.nodes)
                        .all(|(a, b)| a.id() == b.id() && a.bounds == b.bounds);
                let routing = updated.routing();
                println!(
                    "  {kind:<11} full build {full_ms:>7.1} ms | update {update_ms:>5.1} ms + index and lookup {index_ms:>4.1} ms = {:>5.1} ms | routed {}, kept {} | other cards moved {moved} | cards as a full build: {same_cards}",
                    update_ms + index_ms,
                    routing.routed,
                    routing.kept
                );
            }
        }
    }
}
