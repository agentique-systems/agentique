//! Hand-made scene inputs shared by the integration tests.
#![allow(dead_code)]
use agq_studio_scene::fixtures::{at, card, edge, id, node, port};
use agq_studio_scene::*;

/// Three containers (1, 2, 3) of three components each (11..13, 21..23,
/// 31..33). Component `c` has ports `c*100+1` ("request") and `c*100+2`
/// ("result"). Six connections link a result port to a request port in the
/// next container; the first one ends at port 2101 of component 21. Four
/// directed edges link cards.
pub fn grid() -> SceneInput {
    let names = [
        (1, "Studio", None),
        (2, "Platform", None),
        (3, "Services", None),
        (11, "Surface", Some(1)),
        (12, "Graph", Some(1)),
        (13, "Inspector", Some(1)),
        (21, "Repository", Some(2)),
        (22, "Service", Some(2)),
        (23, "Workspace", Some(2)),
        (31, "Planner", Some(3)),
        (32, "Engine", Some(3)),
        (33, "Api", Some(3)),
    ];
    let nodes = names
        .into_iter()
        .map(|(value, name, owner)| {
            let mut card = node(
                value,
                name,
                if owner.is_some() {
                    NodeCategory::Part
                } else {
                    NodeCategory::Definition
                },
                owner,
            );
            if owner.is_some() {
                card.ports = vec![
                    port(value * 100 + 1, "request"),
                    port(value * 100 + 2, "result"),
                ];
            }
            card
        })
        .collect();
    let mut edges: Vec<_> = [
        (11, 1102, 21, 2101),
        (12, 1202, 22, 2201),
        (13, 1302, 23, 2301),
        (21, 2102, 31, 3101),
        (22, 2202, 32, 3201),
        (23, 2302, 33, 3301),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (a, pa, b, pb))| {
        let mut connection = edge(&format!("c{i}"), EdgeKind::Connection, at(a, pa), at(b, pb));
        connection.element = Some(id(5000 + i as u64));
        connection
    })
    .collect();
    edges.push(edge("t10", EdgeKind::Typing, card(21), card(23)));
    edges.push(edge("t11", EdgeKind::Typing, card(22), card(21)));
    edges.push(edge("t12", EdgeKind::Typing, card(31), card(32)));
    edges.push(edge("s13", EdgeKind::Specialization, card(12), card(11)));
    SceneInput {
        generation: 1,
        nodes,
        edges,
    }
}

/// [`grid`] before and after an edit: component 22 renamed, 24 added to
/// container 2, 13 removed with its connection.
pub fn grid_change() -> (SceneInput, SceneInput) {
    let before = grid();
    let mut after = before.clone();
    after.generation = 2;
    for card in &mut after.nodes {
        if card.id == id(22) {
            card.name = "RenamedService".into();
        }
    }
    let mut added = node(24, "Coordinator", NodeCategory::Part, Some(2));
    added.ports = vec![port(2401, "request"), port(2402, "result")];
    after.nodes.push(added);
    after.nodes.retain(|card| card.id != id(13));
    after
        .edges
        .retain(|e| e.source.node != id(13) && e.target.node != id(13));
    (before, after)
}

/// `count` cards 1..=count without owners or ports; `edges` directed-free
/// card-to-card connections forming a ring, then links to column neighbours.
pub fn flat(count: usize, edges: usize) -> SceneInput {
    let nodes = (0..count)
        .map(|i| {
            node(
                i as u64 + 1,
                &format!("Part{i:05}"),
                NodeCategory::Part,
                None,
            )
        })
        .collect();
    let width = (count as f32).sqrt().ceil() as usize;
    let edges = if count == 0 {
        Vec::new()
    } else {
        (0..edges)
            .map(|i| {
                let source = i % count;
                let offset = if i < count { 1 } else { width.max(1) };
                let target = (source + offset) % count;
                let mut link = edge(
                    &format!("e{i:06}"),
                    EdgeKind::Connection,
                    card(source as u64 + 1),
                    card(target as u64 + 1),
                );
                link.element = Some(id(1_000_000 + i as u64));
                link
            })
            .collect()
    };
    SceneInput {
        generation: 1,
        nodes,
        edges,
    }
}

/// Inputs that stress presentation: fan-out, depth, many ports, dense links,
/// parallel edges, long names, requirements, an edit and cycles.
pub fn adversarial() -> Vec<(&'static str, SceneInput)> {
    let mut wide = grid();
    for index in 0..32 {
        wide.nodes.push(node(
            10_000 + index,
            &format!("Branch_{index:02}"),
            NodeCategory::Part,
            Some(2),
        ));
    }
    let deep = SceneInput {
        generation: 1,
        nodes: (1..=64)
            .map(|index| {
                node(
                    index,
                    &format!("Level_{index:02}"),
                    NodeCategory::Part,
                    (index > 1).then_some(index - 1),
                )
            })
            .collect(),
        edges: Vec::new(),
    };
    let mut ports = grid();
    for card in ports.nodes.iter_mut().filter(|card| card.owner.is_some()) {
        let base = card.id.raw() * 1000;
        card.ports.extend(
            (0..22).map(|index| port(base + index, &format!("thermalPressure_{index:02}_μPa"))),
        );
    }
    let mut parallel = grid();
    for index in 0..12 {
        let mut link = parallel.edges[0].clone();
        link.id = format!("parallel-{index}");
        parallel.edges.push(link);
    }
    let mut names = grid();
    for card in &mut names.nodes {
        card.name = format!(
            "{}::ThermalProtection_ΔT_μPa_長い工学名_InterfaceRequirement",
            card.name
        );
    }
    vec![
        ("wide-fan-out", wide),
        ("deep-hierarchy", deep),
        ("many-ports", ports),
        ("dense-cross-links", fixtures::dense_ports()),
        ("parallel-edges", parallel),
        ("long-unicode-names", names),
        ("typography", fixtures::typography()),
        ("url-shortener", fixtures::architecture()),
        ("requirements", fixtures::architecture().requirements_view()),
        ("edited", grid_change().1),
        ("cycles", flat(80, 160)),
    ]
}

pub fn options(layout: LayoutKind) -> SceneOptions {
    SceneOptions {
        layout,
        ..Default::default()
    }
}
