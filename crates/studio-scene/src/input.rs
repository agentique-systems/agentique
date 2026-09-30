//! What the Surface draws, built from the element tree.
//!
//! Nodes are the elements shown as cards (packages, definitions, parts, items,
//! attributes, requirements). Ports are shown on the cards of their owners
//! and, found through the type (never copied), on the cards of usages typed
//! by that owner. Connections, interfaces, satisfy relationships, typing and
//! specialisation are edges.
use crate::NodeCategory;
use agq_language::{Direction, Element, ElementId, ElementKind, Reference, Tree};
use std::collections::{BTreeMap, BTreeSet};

/// A port's direction as written; `Unspecified` when the model gives none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortDirection {
    Unspecified,
    In,
    Out,
    InOut,
}

/// How a lock applies to an element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LockMark {
    #[default]
    None,
    /// The element carries the lock.
    Own,
    /// The element is covered by the lock of an element that owns it.
    Covered,
}
impl LockMark {
    pub fn locked(self) -> bool {
        self != Self::None
    }
}

/// One port shown on a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPort {
    pub id: ElementId,
    pub name: String,
    pub direction: PortDirection,
    pub lock: LockMark,
    /// The definition that owns the port, when the card shows it through its
    /// type. Changing the port changes that definition and every usage of it.
    pub defined_in: Option<ElementId>,
}

/// One attribute or item line on a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFeature {
    pub id: ElementId,
    /// Such as `attribute capacity : Natural`.
    pub text: String,
    pub lock: LockMark,
    pub problems: usize,
}

/// Where a card's element stands in its definition: its own, found
/// through a general (inherited), or a redefinition of an inherited one
/// (an override). Shown by the card's edge, never by colour (§8.5 rule 4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NodeOrigin {
    #[default]
    Own,
    /// Owned by a general of the card that shows it: drawn inside a
    /// definition opened on the Surface, where the general is not shown.
    Inherited,
    /// Overrides an inherited feature in this definition or usage only.
    Override,
}

/// One card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputNode {
    pub id: ElementId,
    pub kind: NodeCategory,
    /// The SysML keyword, such as `part def`.
    pub keyword: &'static str,
    pub name: String,
    /// The type, specialisation, multiplicity or value as written, such as
    /// `: HttpApi [1]`.
    pub detail: String,
    /// The owning card, if the owner is shown.
    pub owner: Option<ElementId>,
    pub ports: Vec<InputPort>,
    /// Attributes and items owned by the element, one line each.
    pub features: Vec<InputFeature>,
    pub lock: LockMark,
    /// Problems reported at this element or at elements it owns that have no
    /// card of their own.
    pub problems: usize,
    pub origin: NodeOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    Connection,
    Interface,
    Satisfy,
    Typing,
    Specialization,
}
impl EdgeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Connection => "connection",
            Self::Interface => "interface",
            Self::Satisfy => "satisfy",
            Self::Typing => "typed by",
            Self::Specialization => "specializes",
        }
    }
}

/// An edge end: a card, or a port shown on a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputEnd {
    pub node: ElementId,
    pub port: Option<ElementId>,
}
impl InputEnd {
    pub fn node(node: ElementId) -> Self {
        Self { node, port: None }
    }
    /// The port if there is one, else the card.
    pub fn element(self) -> ElementId {
        self.port.unwrap_or(self.node)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEdge {
    /// Unique within the input.
    pub id: String,
    /// The element that is the edge (a connection, interface or satisfy), if any.
    pub element: Option<ElementId>,
    pub kind: EdgeKind,
    pub source: InputEnd,
    pub target: InputEnd,
    pub directed: bool,
    pub label: String,
    pub problems: usize,
    /// The lock that covers the edge's element, if any.
    pub lock: LockMark,
}

/// Everything the Surface shows, for one version of the model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneInput {
    /// Increases whenever the input changes; scenes and indexes carry it.
    pub generation: u64,
    pub nodes: Vec<InputNode>,
    pub edges: Vec<InputEdge>,
}

fn category(kind: ElementKind) -> Option<NodeCategory> {
    use ElementKind::*;
    Some(match kind {
        Package => NodeCategory::Package,
        PartDef | ItemDef | PortDef | ConnectionDef | InterfaceDef | AttributeDef => {
            NodeCategory::Definition
        }
        RequirementDef | Requirement => NodeCategory::Requirement,
        Part => NodeCategory::Part,
        Item => NodeCategory::Item,
        Attribute => NodeCategory::Attribute,
        _ => return None,
    })
}

fn direction(direction: Option<Direction>) -> PortDirection {
    match direction {
        None => PortDirection::Unspecified,
        Some(Direction::In) => PortDirection::In,
        Some(Direction::Out) => PortDirection::Out,
        Some(Direction::InOut) => PortDirection::InOut,
    }
}

/// `: T [1] = 5` or `:> General`, as written.
pub fn detail(element: &Element) -> String {
    let mut out = String::new();
    let list = |references: &[Reference]| {
        references
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !element.typed_by.is_empty() {
        out.push_str(": ");
        if element.conjugated {
            out.push('~');
        }
        out.push_str(&list(&element.typed_by));
    }
    if !element.specializes.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(":> ");
        out.push_str(&list(&element.specializes));
    }
    if let Some(multiplicity) = element.multiplicity {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&multiplicity.to_string());
    }
    if let Some(value) = &element.value {
        out.push_str(&format!(" = {value}"));
    }
    out.trim().to_string()
}

impl SceneInput {
    /// Builds the input from a tree. `locks` are the elements that carry a
    /// lock; `problems` counts the problems reported at each element.
    pub fn from_tree(
        tree: &Tree,
        locks: &BTreeSet<ElementId>,
        problems: &BTreeMap<ElementId, usize>,
        generation: u64,
    ) -> Self {
        let order = tree.walk();
        let lock = |id: ElementId| {
            if locks.contains(&id) {
                return LockMark::Own;
            }
            let mut current = tree.get(id).and_then(Element::owner);
            while let Some(owner) = current {
                if locks.contains(&owner) {
                    return LockMark::Covered;
                }
                current = tree.get(owner).and_then(Element::owner);
            }
            LockMark::None
        };
        // Attributes and items inside another element are lines on its card.
        let line = |id: ElementId| {
            let element = &tree[id];
            matches!(element.kind, ElementKind::Attribute | ElementKind::Item)
                && element
                    .owner()
                    .is_some_and(|owner| tree[owner].kind != ElementKind::Package)
        };
        // What a scenario sets up (its stand-ins) is shown with the
        // scenario, not as architecture (C-50).
        let in_scenario = |id: ElementId| {
            let mut current = tree.get(id).and_then(Element::owner);
            while let Some(owner) = current {
                if tree[owner].kind == ElementKind::VerificationDef {
                    return true;
                }
                current = tree.get(owner).and_then(Element::owner);
            }
            false
        };
        let shown: BTreeSet<ElementId> = order
            .iter()
            .copied()
            .filter(|id| category(tree[*id].kind).is_some() && !line(*id) && !in_scenario(*id))
            .collect();
        // The nearest shown element at or above each element.
        let card_of = |mut id: ElementId| -> Option<ElementId> {
            loop {
                if shown.contains(&id) {
                    return Some(id);
                }
                id = tree.get(id)?.owner()?;
            }
        };
        let mut node_problems = BTreeMap::<ElementId, usize>::new();
        let mut edge_problems = BTreeMap::<ElementId, usize>::new();
        for (id, count) in problems {
            let Some(element) = tree.get(*id) else {
                continue;
            };
            if matches!(
                element.kind,
                ElementKind::Connection | ElementKind::Interface | ElementKind::Satisfy
            ) {
                *edge_problems.entry(*id).or_default() += count;
            }
            if let Some(card) = card_of(*id) {
                *node_problems.entry(card).or_default() += count;
            }
        }
        let name = |id: ElementId| {
            tree.effective_name(id)
                .map(str::to_string)
                .unwrap_or_else(|| format!("({})", tree[id].kind.keyword()))
        };
        let own_ports = |id: ElementId| -> Vec<ElementId> {
            tree[id]
                .children()
                .iter()
                .copied()
                .filter(|c| tree[*c].kind == ElementKind::Port)
                .collect()
        };
        let mut nodes = Vec::new();
        for id in order.iter().copied().filter(|id| shown.contains(id)) {
            let element = &tree[id];
            // Own ports first, then ports found through the types and their
            // generals, unless a port of the same name is already shown.
            let mut ports = own_ports(id);
            let mut seen_names: BTreeSet<String> = ports.iter().map(|p| name(*p)).collect();
            let mut visited = BTreeSet::new();
            let mut pending: Vec<ElementId> = element
                .typed_by
                .iter()
                .chain(&element.specializes)
                .filter_map(Reference::target)
                .collect();
            // An override has the ports of the feature it redefines.
            pending.extend(element.redefines.iter().filter_map(Reference::target));
            while let Some(general) = pending.pop() {
                if !visited.insert(general) || visited.len() > 32 {
                    continue;
                }
                let Some(definition) = tree.get(general) else {
                    continue;
                };
                if definition.kind.is_usage() {
                    // A redefined feature: through its own types and redefinitions.
                    pending.extend(
                        definition
                            .typed_by
                            .iter()
                            .chain(&definition.redefines)
                            .filter_map(Reference::target),
                    );
                    continue;
                }
                for port in own_ports(general) {
                    if seen_names.insert(name(port)) {
                        ports.push(port);
                    }
                }
                pending.extend(definition.specializes.iter().filter_map(Reference::target));
            }
            let origin = if element.kind.is_usage() && !element.redefines.is_empty() {
                NodeOrigin::Override
            } else {
                NodeOrigin::Own
            };
            let mut shown_detail = detail(element);
            if origin == NodeOrigin::Override {
                // Its type is the redefined feature's unless it gives one.
                if element.typed_by.is_empty()
                    && let Some(redefined) = element
                        .redefines
                        .first()
                        .and_then(Reference::target)
                        .and_then(|t| tree.get(t))
                {
                    let inherited = detail(redefined);
                    shown_detail = format!("{inherited} {shown_detail}").trim().to_string();
                }
                shown_detail = format!("{shown_detail} · override")
                    .trim_start_matches(" · ")
                    .to_string();
            }
            nodes.push(InputNode {
                id,
                kind: category(element.kind).expect("shown elements have a category"),
                keyword: element.kind.keyword(),
                name: name(id),
                detail: shown_detail,
                origin,
                owner: element.owner().and_then(card_of),
                ports: ports
                    .into_iter()
                    .map(|port| InputPort {
                        id: port,
                        name: name(port),
                        direction: direction(tree[port].direction),
                        lock: lock(port),
                        defined_in: tree[port].owner().filter(|owner| *owner != id),
                    })
                    .collect(),
                features: element
                    .children()
                    .iter()
                    .copied()
                    .filter(|c| line(*c))
                    .map(|c| {
                        let feature = &tree[c];
                        let direction = feature
                            .direction
                            .map(|d| format!("{} ", d.keyword()))
                            .unwrap_or_default();
                        InputFeature {
                            id: c,
                            text: format!(
                                "{direction}{} {} {}",
                                feature.kind.keyword(),
                                name(c),
                                detail(feature)
                            )
                            .trim_end()
                            .to_string(),
                            lock: lock(c),
                            problems: tree
                                .descendants(c)
                                .iter()
                                .filter_map(|d| problems.get(d))
                                .sum(),
                        }
                    })
                    .collect(),
                lock: lock(id),
                problems: node_problems.get(&id).copied().unwrap_or(0),
            });
        }
        let mut edges = Vec::new();
        for id in order.iter().copied() {
            let element = &tree[id];
            match element.kind {
                ElementKind::Connection | ElementKind::Interface if element.ends.len() == 2 => {
                    let (Some(source), Some(target)) = (
                        end(tree, &shown, &element.ends[0]),
                        end(tree, &shown, &element.ends[1]),
                    ) else {
                        continue;
                    };
                    let interface = element.kind == ElementKind::Interface;
                    let mut label = element.name.clone().unwrap_or_default();
                    if let Some(definition) = element.typed_by.first() {
                        if !label.is_empty() {
                            label.push_str(" : ");
                        }
                        label.push_str(&definition.to_string());
                    }
                    if label.is_empty() {
                        label = format!("{} → {}", element.ends[0], element.ends[1]);
                    }
                    edges.push(InputEdge {
                        id: format!("element:{}", id.raw()),
                        element: Some(id),
                        kind: if interface {
                            EdgeKind::Interface
                        } else {
                            EdgeKind::Connection
                        },
                        source,
                        target,
                        directed: false,
                        label,
                        problems: edge_problems.get(&id).copied().unwrap_or(0),
                        lock: lock(id),
                    });
                }
                ElementKind::Satisfy => {
                    let (Some(by), Some(requirement)) = (
                        element.by.as_ref().and_then(|r| end(tree, &shown, r)),
                        element.target.as_ref().and_then(|r| end(tree, &shown, r)),
                    ) else {
                        continue;
                    };
                    edges.push(InputEdge {
                        id: format!("element:{}", id.raw()),
                        element: Some(id),
                        kind: EdgeKind::Satisfy,
                        source: by,
                        target: requirement,
                        directed: true,
                        label: "satisfies".into(),
                        problems: edge_problems.get(&id).copied().unwrap_or(0),
                        lock: lock(id),
                    });
                }
                ElementKind::Subject => {
                    // A requirement's subject: from the requirement to the subject's type.
                    let Some(requirement) = element.owner().filter(|o| shown.contains(o)) else {
                        continue;
                    };
                    for (index, reference) in element.typed_by.iter().enumerate() {
                        if let Some(target) = reference.target().filter(|t| shown.contains(t)) {
                            edges.push(InputEdge {
                                id: format!("subject:{}:{index}", id.raw()),
                                element: Some(id),
                                kind: EdgeKind::Typing,
                                source: InputEnd::node(requirement),
                                target: InputEnd::node(target),
                                directed: true,
                                label: format!("subject {}", name(id)),
                                problems: 0,
                                lock: lock(id),
                            });
                        }
                    }
                }
                _ if shown.contains(&id) => {
                    for (kind, references) in [
                        (EdgeKind::Typing, &element.typed_by),
                        (EdgeKind::Specialization, &element.specializes),
                    ] {
                        for (index, reference) in references.iter().enumerate() {
                            if let Some(target) = reference.target().filter(|t| shown.contains(t)) {
                                edges.push(InputEdge {
                                    id: format!("{}:{}:{index}", kind.label(), id.raw()),
                                    element: None,
                                    kind,
                                    source: InputEnd::node(id),
                                    target: InputEnd::node(target),
                                    directed: true,
                                    label: format!("{} {}", kind.label(), reference),
                                    problems: 0,
                                    lock: LockMark::None,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        SceneInput {
            generation,
            nodes,
            edges,
        }
    }

    /// Keeps only the edges of the given kinds.
    pub fn retain_edges(&mut self, kinds: &[EdgeKind]) {
        self.edges.retain(|edge| kinds.contains(&edge.kind));
    }

    /// The input for the requirements view: requirements, what satisfies
    /// them and their subjects, without nesting.
    pub fn requirements_view(&self) -> SceneInput {
        let mut keep: BTreeSet<ElementId> = self
            .nodes
            .iter()
            .filter(|n| n.kind == NodeCategory::Requirement)
            .map(|n| n.id)
            .collect();
        let edges: Vec<_> = self
            .edges
            .iter()
            .filter(|e| {
                e.kind == EdgeKind::Satisfy
                    || (e.kind == EdgeKind::Typing && keep.contains(&e.source.node))
            })
            .cloned()
            .collect();
        for edge in &edges {
            keep.insert(edge.source.node);
            keep.insert(edge.target.node);
        }
        SceneInput {
            generation: self.generation,
            nodes: self
                .nodes
                .iter()
                .filter(|n| keep.contains(&n.id))
                .map(|n| InputNode {
                    owner: None,
                    ports: Vec::new(),
                    features: Vec::new(),
                    ..n.clone()
                })
                .collect(),
            edges: edges
                .into_iter()
                .map(|mut e| {
                    e.source.port = None;
                    e.target.port = None;
                    e
                })
                .collect(),
        }
    }
}

/// Where a reference ends on the Surface: `a.b.port` ends at the port on the
/// card of `b` (the step before it); a reference to a card ends at the card.
fn end(tree: &Tree, shown: &BTreeSet<ElementId>, reference: &Reference) -> Option<InputEnd> {
    let targets: Vec<ElementId> = reference
        .steps
        .iter()
        .map(|s| s.target)
        .collect::<Option<_>>()?;
    let last = *targets.last()?;
    let element = tree.get(last)?;
    if element.kind == ElementKind::Port {
        let node = if targets.len() >= 2 {
            targets[targets.len() - 2]
        } else {
            element.owner()?
        };
        let node = nearest_shown(tree, shown, node)?;
        return Some(InputEnd {
            node,
            port: Some(last),
        });
    }
    nearest_shown(tree, shown, last).map(InputEnd::node)
}

fn nearest_shown(tree: &Tree, shown: &BTreeSet<ElementId>, mut id: ElementId) -> Option<ElementId> {
    loop {
        if shown.contains(&id) {
            return Some(id);
        }
        id = tree.get(id)?.owner()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use agq_language::validate;

    const MODEL: &str = "package P {
    port def Signal;
    part def D {
        port p : Signal;
    }
    part def S {
        part a : D {
            attribute speed : Missing;
        }
        part b {
            port q : Signal;
        }
        interface connect a.p to b.q;
    }
}";

    #[test]
    fn locks_problems_and_feature_lines_come_from_the_tree() {
        let tree = fixtures::tree(MODEL);
        let s = tree.find("P::S").unwrap();
        let mut problems = BTreeMap::new();
        for diagnostic in validate(&tree) {
            *problems.entry(diagnostic.element).or_insert(0) += 1;
        }
        let input = SceneInput::from_tree(&tree, &BTreeSet::from([s]), &problems, 7);
        assert_eq!(input.generation, 7);
        let card = |name: &str| input.nodes.iter().find(|n| n.name == name).unwrap();
        // S carries the lock; what it owns is covered by it.
        assert_eq!(card("S").lock, LockMark::Own);
        assert_eq!(card("a").lock, LockMark::Covered);
        assert_eq!(card("D").lock, LockMark::None);
        // The attribute is a line on a's card, covered by the lock, with its problem.
        let a = card("a");
        assert_eq!(a.features.len(), 1);
        assert_eq!(a.features[0].text, "attribute speed : Missing");
        assert_eq!(a.features[0].lock, LockMark::Covered);
        assert_eq!(a.features[0].problems, 1);
        // The problem rolls up to the card that shows it.
        assert!(a.problems >= 1);
        assert!(input.nodes.iter().all(|n| n.name != "speed"));
        // a shows D's port, defined in D and not covered by S's lock.
        let d = card("D").id;
        assert_eq!(a.ports[0].defined_in, Some(d));
        assert_eq!(a.ports[0].lock, LockMark::None);
        assert_eq!(card("b").ports[0].lock, LockMark::Covered);
        // The interface is an edge between the two ports, covered by the lock.
        let edge = input
            .edges
            .iter()
            .find(|e| e.kind == EdgeKind::Interface)
            .unwrap();
        assert_eq!(edge.label, "a.p → b.q");
        assert_eq!(edge.lock, LockMark::Covered);
        assert_eq!(edge.source.port, Some(a.ports[0].id));
    }

    #[test]
    fn what_a_scenario_sets_up_is_not_architecture() {
        let tree = fixtures::tree(
            "package P {
    part def S { part worker; }
    verification def Works {
        subject s : S;
        part slow : S;
    }
}",
        );
        let input = SceneInput::from_tree(&tree, &BTreeSet::new(), &BTreeMap::new(), 1);
        assert!(input.nodes.iter().any(|n| n.name == "worker"));
        assert!(
            input.nodes.iter().all(|n| n.name != "slow"),
            "the scenario's part is not a card"
        );
    }
}
