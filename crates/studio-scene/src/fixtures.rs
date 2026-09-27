//! Scene inputs for tests, benchmarks and the Studio's `--fixture` option.
//!
//! The architecture fixture is the Scenario A model (the URL shortener) parsed
//! with the language core. The others stress the Surface: many ports, long
//! names, deep nesting and large generated graphs.
use crate::{
    EdgeKind, InputEdge, InputEnd, InputNode, InputPort, LockMark, NodeCategory, PortDirection,
    SceneInput,
};
use agq_language::{
    Element, ElementId, ElementKind, Parent, Reference, Source, Tree, link, parse, validate,
};
use std::collections::{BTreeMap, BTreeSet};

/// The Scenario A model, `models/url-shortener/UrlShortener.sysml`.
pub const URL_SHORTENER: &str = include_str!("../../../models/url-shortener/UrlShortener.sysml");

/// An element id for hand-made inputs.
pub fn id(value: u64) -> ElementId {
    ElementId::from_raw(value)
}

/// Parses SysML text as one document.
pub fn tree(text: &str) -> Tree {
    parse(&[Source::new("fixture.sysml", text)])
}

/// The scene input for a tree, with its problems and no locks.
pub fn input(tree: &Tree) -> SceneInput {
    let mut problems = BTreeMap::new();
    for diagnostic in validate(tree) {
        *problems.entry(diagnostic.element).or_insert(0) += 1;
    }
    SceneInput::from_tree(tree, &BTreeSet::new(), &problems, 1)
}

/// A hand-made card.
pub fn node(value: u64, name: &str, kind: NodeCategory, owner: Option<u64>) -> InputNode {
    InputNode {
        id: id(value),
        kind,
        keyword: match kind {
            NodeCategory::Package => "package",
            NodeCategory::Definition => "part def",
            NodeCategory::Requirement => "requirement",
            NodeCategory::Item => "item",
            NodeCategory::Attribute => "attribute",
            _ => "part",
        },
        name: name.into(),
        detail: String::new(),
        owner: owner.map(id),
        ports: Vec::new(),
        features: Vec::new(),
        lock: LockMark::None,
        problems: 0,
    }
}

/// A hand-made port for [`node`].
pub fn port(value: u64, name: &str) -> InputPort {
    InputPort {
        id: id(value),
        name: name.into(),
        direction: PortDirection::Unspecified,
        lock: LockMark::None,
        defined_in: None,
    }
}

/// A hand-made edge between two ends (see [`at`] and [`card`]).
pub fn edge(name: &str, kind: EdgeKind, source: InputEnd, target: InputEnd) -> InputEdge {
    InputEdge {
        id: name.into(),
        element: None,
        kind,
        source,
        target,
        directed: !matches!(kind, EdgeKind::Connection | EdgeKind::Interface),
        label: name.into(),
        problems: 0,
        lock: LockMark::None,
    }
}

/// The end at port `port` on card `node`.
pub fn at(node: u64, port: u64) -> InputEnd {
    InputEnd {
        node: id(node),
        port: Some(id(port)),
    }
}

/// The end at card `node`.
pub fn card(node: u64) -> InputEnd {
    InputEnd::node(id(node))
}

/// The URL shortener.
pub fn architecture() -> SceneInput {
    input(&tree(URL_SHORTENER))
}

/// A system whose parts have many ports, densely connected.
pub fn dense_ports() -> SceneInput {
    let mut text = String::from("package DensePorts {\n    port def Signal;\n");
    for part in 0..6 {
        text.push_str(&format!("    part def Unit{part} {{\n"));
        for port in 0..8 {
            text.push_str(&format!("        port p{port} : Signal;\n"));
        }
        text.push_str("    }\n");
    }
    text.push_str("    part def System {\n");
    for part in 0..6 {
        text.push_str(&format!("        part unit{part} : Unit{part};\n"));
    }
    for a in 0..6 {
        for b in 0..6 {
            if a != b && (a + b) % 3 == 0 {
                text.push_str(&format!("        connect unit{a}.p{b} to unit{b}.p{a};\n"));
            }
        }
    }
    text.push_str("    }\n}\n");
    input(&tree(&text))
}

/// Long names in several scripts, to check that labels never clip illegibly.
pub fn typography() -> SceneInput {
    let text = "package 'Thermal protection ΔT (μPa) 長い工学名' {
    part def 'Überdruck-Schutzeinrichtung mit sehr langem Namen' {
        port 'eingehende Messwerte für Temperatur und Druck';
        port '出力ポート_長い名前_ThermalPressure';
    }
    part def 'Répartiteur de charge thermique' {
        port 'entrée';
    }
    part def 'System with a deliberately long and descriptive name' {
        part 'protection unit' : 'Überdruck-Schutzeinrichtung mit sehr langem Namen';
        part 'load distributor' : 'Répartiteur de charge thermique';
        connect 'protection unit'.'出力ポート_長い名前_ThermalPressure' to 'load distributor'.'entrée';
    }
    requirement def 'The protection unit shall limit the pressure to the configured maximum at all times';
}
";
    input(&tree(text))
}

/// A generated graph of `nodes` parts in containers and `edges` connections.
pub fn stress(nodes: usize, edges: usize) -> SceneInput {
    let containers = (nodes / 50).max(1);
    let mut input = SceneInput {
        generation: 1,
        ..Default::default()
    };
    for c in 0..containers {
        input.nodes.push(node(
            c as u64 + 1,
            &format!("Subsystem{c:03}"),
            NodeCategory::Definition,
            None,
        ));
    }
    let base = containers as u64 + 1;
    for n in 0..nodes {
        let mut part = node(
            base + n as u64,
            &format!("Component{n:05}"),
            NodeCategory::Part,
            Some((n % containers) as u64 + 1),
        );
        part.ports = vec![
            port(1_000_000 + 2 * n as u64, "in"),
            port(1_000_001 + 2 * n as u64, "out"),
        ];
        input.nodes.push(part);
    }
    let mut seed = 0x5eed_u64;
    for e in 0..edges {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = (seed >> 33) as usize % nodes;
        let b = (e * 7 + 3) % nodes;
        input.edges.push(edge(
            &format!("e{e:06}"),
            EdgeKind::Connection,
            at(base + a as u64, 1_000_001 + 2 * a as u64),
            at(base + b as u64, 1_000_000 + 2 * b as u64),
        ));
    }
    input
}

/// The URL shortener before and after an edit made on the same tree, so
/// identities are kept: a part and its two connections removed, a part
/// added and a part renamed.
pub fn change_trees() -> (Tree, Tree) {
    let before = tree(URL_SHORTENER);
    let mut tree = before.clone();
    let service = tree
        .find("UrlShortener::UrlShortenerService")
        .expect("the service exists");
    for path in ["clickStats", "clickReporting", "statsQuery"] {
        if let Some(id) = tree.find(&format!("UrlShortener::UrlShortenerService::{path}")) {
            tree.remove(id);
        }
    }
    if let Some(store) = tree.find("UrlShortener::UrlShortenerService::store")
        && let Some(element) = tree.get_mut(store)
    {
        element.name = Some("links".into());
    }
    let link_store = tree
        .find("UrlShortener::LinkStore")
        .expect("LinkStore exists");
    let mut cache = Element::named(ElementKind::Part, "cache");
    cache.typed_by = vec![Reference::to(link_store, "LinkStore")];
    tree.add(Parent::Element(service), cache)
        .expect("the service can own parts");
    link(&mut tree);
    (before, tree)
}

/// [`change_trees`] as scene inputs; the second input's generation is higher.
pub fn change() -> (SceneInput, SceneInput) {
    let (before, after) = change_trees();
    let before = input(&before);
    let mut after = input(&after);
    after.generation = before.generation + 1;
    (before, after)
}
