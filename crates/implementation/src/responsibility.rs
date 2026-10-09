//! A part as the Operator and the Assistant read it (ROADMAP §2.8 C1, C-51):
//! what it is and why it exists, what it owns, its contract, what it depends
//! on and what uses it, where it is implemented and checked, and what
//! changing it affects. Everything is read from the model and its links,
//! never kept anywhere else: the Inspector shows it and the Assistant's
//! `explain_element` tool says it, from the same answer.

use crate::links::{LinkKind, Links};
use agq_language::{Direction, ElementId, ElementKind, Role, Semantics, Tree, print_expression};
use agq_simulation::digest::closure;

/// One port of the contract: its name and type, and what it carries in and
/// out (as seen by the part that has it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractPort {
    pub port: ElementId,
    pub name: String,
    pub type_name: String,
    pub receives: Vec<String>,
    pub sends: Vec<String>,
}

/// A referential part or item (`ref part`, C-55): it refers to a part
/// that exists elsewhere and does not contain it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Referential {
    pub feature: ElementId,
    pub name: String,
    /// The part it refers to, as written where it is bound (`bus`: its own
    /// value, or that of the feature it redefines), or `None` when it is not
    /// bound: the part it refers to is not identified in this model.
    pub refers_to: Option<String>,
}

impl Referential {
    fn of(tree: &Tree, semantics: &Semantics, feature: ElementId) -> Referential {
        Referential {
            feature,
            name: tree.effective_name(feature).unwrap_or("?").to_string(),
            refers_to: semantics.value_holder(feature).and_then(|holder| {
                let value = tree.get(holder)?.expression.as_ref()?;
                Some(print_expression(tree, holder, value))
            }),
        }
    }

    /// `refers to `bus``, or that it is not bound.
    pub fn target_text(&self) -> String {
        match &self.refers_to {
            Some(target) => format!("refers to `{target}`"),
            None => "is not bound: the part it refers to is not identified in this model".into(),
        }
    }
}

/// Everything C1 asks about one part definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Responsibility {
    /// The part definition described (a usage is described by its type).
    pub definition: ElementId,
    pub name: String,
    /// Its documentation: what it is for, owns and must not do.
    pub purpose: String,
    /// What it owns as features: attributes and items, with their docs.
    pub owns: Vec<(String, String)>,
    /// Its referential parts and items: what each refers to (not owned).
    pub refers: Vec<Referential>,
    /// When the element explained is itself a referential part: what it
    /// refers to. The rest describes its type.
    pub usage: Option<Referential>,
    pub contract: Vec<ContractPort>,
    pub depends_on: Vec<ElementId>,
    pub used_by: Vec<ElementId>,
    /// Parts it exchanges items with through connections.
    pub connected: Vec<ElementId>,
    /// Implementation links on it and on what it owns: (kind, location).
    pub implemented_in: Vec<(LinkKind, String)>,
    /// Requirements a usage of it satisfies.
    pub requirements: Vec<ElementId>,
    /// Scenarios that exercise it.
    pub scenarios: Vec<ElementId>,
    /// The lock that covers it, if any.
    pub locked_by: Option<ElementId>,
}

/// The part definition `element` is, or is typed by.
fn definition_of(tree: &Tree, semantics: &Semantics, element: ElementId) -> Option<ElementId> {
    let e = tree.get(element)?;
    match e.kind {
        ElementKind::PartDef => Some(element),
        ElementKind::Part => semantics
            .types_of(element)
            .into_iter()
            .map(|(t, _)| t)
            .find(|t| tree.get(*t).is_some_and(|d| d.kind == ElementKind::PartDef)),
        _ => None,
    }
}

/// An element's documentation as one paragraph.
pub fn doc_text(tree: &Tree, id: ElementId) -> String {
    tree.get(id)
        .and_then(|e| {
            e.children()
                .iter()
                .find(|c| tree[**c].kind == ElementKind::Doc)
        })
        .and_then(|doc| tree[*doc].text.as_deref())
        .map(|text| {
            text.trim()
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .lines()
                .map(|line| line.trim().trim_start_matches('*').trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// What C1 asks about `element` (a part def or a part), or `None` for other
/// kinds of element. `locks` are the model's locks (each covers what its
/// element owns).
pub fn responsibility(
    tree: &Tree,
    links: &Links,
    locks: &[ElementId],
    element: ElementId,
) -> Option<Responsibility> {
    let semantics = Semantics::new(tree);
    let definition = definition_of(tree, &semantics, element)?;
    let name = |id: ElementId| tree.effective_name(id).unwrap_or("?").to_string();
    // Owned information and the contract.
    let mut owns = Vec::new();
    let mut refers = Vec::new();
    let mut contract = Vec::new();
    for feature in tree[definition].children() {
        let feature = *feature;
        if semantics.referential(feature) == Some(true) {
            refers.push(Referential::of(tree, &semantics, feature));
            continue;
        }
        match tree[feature].kind {
            ElementKind::Attribute | ElementKind::Item => {
                owns.push((name(feature), doc_text(tree, feature)));
            }
            ElementKind::Port => {
                let type_name = semantics
                    .types_of(feature)
                    .first()
                    .map(|(t, conjugated)| {
                        format!("{}{}", if *conjugated { "~" } else { "" }, name(*t))
                    })
                    .unwrap_or_default();
                let (mut receives, mut sends) = (Vec::new(), Vec::new());
                for (_, direction, item) in semantics.directed_features(feature) {
                    let item = item.map(&name).unwrap_or_else(|| "?".into());
                    match direction {
                        Direction::In => receives.push(item),
                        Direction::Out => sends.push(item),
                        Direction::InOut => {
                            receives.push(item.clone());
                            sends.push(item);
                        }
                    }
                }
                contract.push(ContractPort {
                    port: feature,
                    name: name(feature),
                    type_name,
                    receives,
                    sends,
                });
            }
            _ => {}
        }
    }
    // Dependencies both ways, between part defs.
    let (mut depends_on, mut used_by) = (Vec::new(), Vec::new());
    for id in tree.walk() {
        let dependency = &tree[id];
        if dependency.kind != ElementKind::Dependency || dependency.ends.len() != 2 {
            continue;
        }
        let end = |i: usize| {
            semantics
                .steps(id, Role::End, &dependency.ends[i])
                .ok()
                .and_then(|steps| steps.last().copied())
                .and_then(|e| definition_of(tree, &semantics, e).or(Some(e)))
        };
        if let (Some(client), Some(supplier)) = (end(0), end(1)) {
            if client == definition && !depends_on.contains(&supplier) {
                depends_on.push(supplier);
            }
            if supplier == definition && !used_by.contains(&client) {
                used_by.push(client);
            }
        }
    }
    // Usages of it, the parts connected to them, and what they satisfy.
    let usages: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::Part)
        .filter(|id| definition_of(tree, &semantics, *id) == Some(definition))
        .collect();
    let mut connected = Vec::new();
    for id in tree.walk() {
        let connection = &tree[id];
        if !matches!(
            connection.kind,
            ElementKind::Connection | ElementKind::Interface
        ) || connection.ends.len() != 2
        {
            continue;
        }
        let ends: Vec<Vec<ElementId>> = connection
            .ends
            .iter()
            .map(|end| semantics.steps(id, Role::End, end).unwrap_or_default())
            .collect();
        for (mine, theirs) in [(0, 1), (1, 0)] {
            if ends[mine].iter().any(|e| usages.contains(e))
                && let Some(other) = ends[theirs]
                    .iter()
                    .copied()
                    .find(|e| tree.get(*e).is_some_and(|x| x.kind == ElementKind::Part))
                && let Some(other) = definition_of(tree, &semantics, other)
                && other != definition
                && !connected.contains(&other)
            {
                connected.push(other);
            }
        }
    }
    let mut requirements = Vec::new();
    for id in tree.walk() {
        let satisfy = &tree[id];
        if satisfy.kind != ElementKind::Satisfy {
            continue;
        }
        let by = satisfy
            .by
            .as_ref()
            .and_then(|by| semantics.steps(id, Role::By, by).ok())
            .and_then(|steps| steps.last().copied());
        let target = satisfy
            .target
            .as_ref()
            .and_then(|t| semantics.steps(id, Role::Target, t).ok())
            .and_then(|steps| steps.last().copied());
        if let (Some(by), Some(target)) = (by, target)
            && (by == definition || usages.contains(&by))
            && !requirements.contains(&target)
        {
            requirements.push(target);
        }
    }
    let scenarios: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::VerificationDef)
        .filter(|id| {
            let reached = closure(tree, *id);
            reached.contains(&definition) || usages.iter().any(|u| reached.contains(u))
        })
        .collect();
    // Links on it and on what it owns (its transitions, its ports).
    let mut implemented_in = Vec::new();
    for link in &links.links {
        let id = link.element();
        if id == definition || tree.descendants(definition).contains(&id) {
            let entry = (link.kind, link.location());
            if !implemented_in.contains(&entry) {
                implemented_in.push(entry);
            }
        }
    }
    let locked_by = locks
        .iter()
        .copied()
        .find(|lock| *lock == definition || tree.descendants(*lock).contains(&definition));
    Some(Responsibility {
        definition,
        name: name(definition),
        purpose: doc_text(tree, definition),
        owns,
        refers,
        usage: (semantics.referential(element) == Some(true))
            .then(|| Referential::of(tree, &semantics, element)),
        contract,
        depends_on,
        used_by,
        connected,
        implemented_in,
        requirements,
        scenarios,
        locked_by,
    })
}

impl Responsibility {
    /// The answer in words, for the Assistant (and anyone reading text):
    /// each of C1's questions with what the model and links say, and where
    /// they say nothing.
    pub fn describe(&self, tree: &Tree) -> String {
        let names = |ids: &[ElementId]| -> String {
            if ids.is_empty() {
                "nothing".into()
            } else {
                ids.iter()
                    .map(|id| tree.qualified_name(*id))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        };
        let mut lines = vec![format!(
            "{} ({}).",
            self.name,
            tree.qualified_name(self.definition)
        )];
        if let Some(usage) = &self.usage {
            lines.push(format!(
                "`{}` is a referential part and does not contain what it refers to; it {}. What follows describes its type, {}.",
                usage.name,
                usage.target_text(),
                self.name
            ));
        }
        lines.push(format!(
            "What it is and why: {}",
            if self.purpose.is_empty() {
                "the model does not say (it has no documentation)."
            } else {
                &self.purpose
            }
        ));
        if !self.owns.is_empty() {
            lines.push(format!(
                "What it owns as features: {}.",
                self.owns
                    .iter()
                    .map(|(n, d)| if d.is_empty() {
                        n.clone()
                    } else {
                        format!("{n} ({d})")
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        if !self.refers.is_empty() {
            lines.push(format!(
                "What it refers to (not owned): {}.",
                self.refers
                    .iter()
                    .map(|r| format!("{} {}", r.name, r.target_text()))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        if self.contract.is_empty() {
            lines.push("Its contract: no ports are modelled.".into());
        } else {
            for port in &self.contract {
                lines.push(format!(
                    "Port {} : {} receives {} and sends {}.",
                    port.name,
                    port.type_name,
                    if port.receives.is_empty() {
                        "nothing".into()
                    } else {
                        port.receives.join(", ")
                    },
                    if port.sends.is_empty() {
                        "nothing".into()
                    } else {
                        port.sends.join(", ")
                    }
                ));
            }
        }
        lines.push(format!(
            "It may use (dependencies): {}.",
            names(&self.depends_on)
        ));
        lines.push(format!("What depends on it: {}.", names(&self.used_by)));
        if !self.connected.is_empty() {
            lines.push(format!(
                "It exchanges items with: {}.",
                names(&self.connected)
            ));
        }
        if self.implemented_in.is_empty() {
            lines.push(
                "Where it is implemented: no implementation links; the model does not say.".into(),
            );
        } else {
            lines.push(format!(
                "Where it is implemented and tested: {}.",
                self.implemented_in
                    .iter()
                    .map(|(kind, at)| format!("{} {at}", kind.label()))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        lines.push(format!(
            "Requirements it satisfies: {}.",
            names(&self.requirements)
        ));
        lines.push(format!(
            "Scenarios that exercise it: {}.",
            names(&self.scenarios)
        ));
        let tests = self
            .implemented_in
            .iter()
            .filter(|(kind, _)| *kind == LinkKind::Test)
            .count();
        lines.push(format!(
            "If you change it: {}{} {} scenario(s) and {tests} linked test(s) cover it{}.",
            match self.used_by.len() {
                0 => "nothing depends on it; ".to_string(),
                n => format!("{n} part(s) depend on it ({}); ", names(&self.used_by)),
            },
            match self.locked_by {
                Some(lock) => format!(
                    "it is locked (by {}), so a change to it asks first, and a patch to its code asks at integration;",
                    tree.qualified_name(lock)
                ),
                None => "it is not locked;".to_string(),
            },
            self.scenarios.len(),
            if self.requirements.is_empty() {
                String::new()
            } else {
                format!(", and it keeps {}", names(&self.requirements))
            }
        ));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_language::{Source, parse};

    #[test]
    fn a_part_is_explained_from_the_model_and_its_links() {
        let tree = parse(&[Source::new(
            "M.sysml",
            "package M {
                private import ScalarValues::*;
                item def Job { attribute id : String; }
                port def Jobs { in item job : Job; out item done : Job; }
                part def Store { doc /* Keeps jobs. */ }
                part def Queue {
                    doc /* Takes jobs in order. It owns the waiting jobs. */
                    port jobs : Jobs;
                    attribute length : Natural = 0 { doc /* How many wait. */ }
                }
                part def Worker { port jobs : ~Jobs; }
                part def System { part queue : Queue; part worker : Worker; connect worker.jobs to queue.jobs; }
                dependency from Queue to Store;
                dependency from Worker to Queue;
                requirement def InOrder { subject q : Queue; }
                requirement inOrder : InOrder;
                part system : System;
                satisfy inOrder by system.queue;
                verification def Takes {
                    subject s : System;
                    objective { verify inOrder; }
                }
            }",
        )]);
        let queue = tree.find("M::Queue").unwrap();
        let mut links = Links::default();
        links.add(&tree, queue, LinkKind::Crate, "crates/queue", None);
        let usage = tree.find("M::System::queue").unwrap();
        let r = responsibility(&tree, &links, &[queue], usage).unwrap();
        assert_eq!(r.definition, queue, "a usage is explained by its type");
        assert!(r.purpose.starts_with("Takes jobs in order."));
        assert_eq!(r.owns, vec![("length".into(), "How many wait.".into())]);
        assert_eq!(r.contract.len(), 1);
        assert_eq!(r.contract[0].receives, ["Job"]);
        assert_eq!(r.contract[0].sends, ["Job"]);
        assert_eq!(r.depends_on, [tree.find("M::Store").unwrap()]);
        assert_eq!(r.used_by, [tree.find("M::Worker").unwrap()]);
        assert_eq!(r.connected, [tree.find("M::Worker").unwrap()]);
        assert_eq!(r.requirements, [tree.find("M::inOrder").unwrap()]);
        assert_eq!(r.scenarios, [tree.find("M::Takes").unwrap()]);
        assert_eq!(r.locked_by, Some(queue));
        let text = r.describe(&tree);
        assert!(text.contains("What depends on it: M::Worker."), "{text}");
        assert!(text.contains("it is locked"), "{text}");
        assert!(text.contains("crate crates/queue"), "{text}");
        // Not a part: nothing to explain this way.
        assert!(responsibility(&tree, &links, &[], tree.find("M::Job").unwrap()).is_none());
    }

    #[test]
    fn a_referential_part_is_explained_as_referring_not_containing() {
        let tree = parse(&[Source::new(
            "M.sysml",
            "package M {
                item def Fuel;
                part def PowerBus { doc /* Carries power. */ }
                part def FlightComputer {
                    ref part supply : PowerBus;
                    ref item reserve : Fuel[0..1];
                    item log : Fuel;
                }
                part def Drone {
                    part bus : PowerBus;
                    part flightComputer : FlightComputer { ref part :>> supply = bus; }
                    ref part spare : PowerBus;
                }
            }",
        )]);
        let computer = tree.find("M::FlightComputer").unwrap();
        let r = responsibility(&tree, &Links::default(), &[], computer).unwrap();
        assert_eq!(r.owns, vec![("log".into(), String::new())]);
        let refers: Vec<(&str, Option<&str>)> = r
            .refers
            .iter()
            .map(|r| (r.name.as_str(), r.refers_to.as_deref()))
            .collect();
        assert_eq!(refers, [("supply", None), ("reserve", None)]);
        let text = r.describe(&tree);
        assert!(
            text.contains(
                "What it refers to (not owned): supply is not bound: the part it refers to is not identified in this model; reserve is not bound: the part it refers to is not identified in this model."
            ),
            "{text}"
        );
        // The bound usage, explained by its type, says what it refers to.
        let bound = tree.find("M::Drone::flightComputer::supply").unwrap();
        let r = responsibility(&tree, &Links::default(), &[], bound).unwrap();
        assert_eq!(r.definition, tree.find("M::PowerBus").unwrap());
        let text = r.describe(&tree);
        assert!(
            text.contains(
                "`supply` is a referential part and does not contain what it refers to; it refers to `bus`. What follows describes its type, PowerBus."
            ),
            "{text}"
        );
        let spare = tree.find("M::Drone::spare").unwrap();
        let text = responsibility(&tree, &Links::default(), &[], spare)
            .unwrap()
            .describe(&tree);
        assert!(
            text.contains(
                "`spare` is a referential part and does not contain what it refers to; it is not bound: the part it refers to is not identified in this model."
            ),
            "{text}"
        );
        // A redefinition without a value keeps the binding it redefines.
        let tree = parse(&[Source::new(
            "M.sysml",
            "package M {
                part def PowerBus;
                part def Drone { part bus : PowerBus; ref part main : PowerBus = bus; }
                part def Big :> Drone { ref part :>> main[1]; }
            }",
        )]);
        let main = tree.find("M::Big::main").unwrap();
        let r = responsibility(&tree, &Links::default(), &[], main).unwrap();
        assert_eq!(r.usage.map(|u| u.refers_to), Some(Some("bus".to_string())));
        // A composite part is explained as before.
        let bus = tree.find("M::Drone::bus").unwrap();
        let r = responsibility(&tree, &Links::default(), &[], bus).unwrap();
        assert_eq!(r.usage, None);
    }

    #[test]
    fn agentiques_own_parts_are_explained_with_their_code_and_checks() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../model");
        let text = std::fs::read_to_string(folder.join("Agentique.sysml")).unwrap();
        let tree = parse(&[Source::new("Agentique.sysml", text)]);
        // The links name elements by id: read them with the identity file.
        let links =
            Links::parse(&std::fs::read_to_string(folder.join("links.json")).unwrap()).unwrap();
        let identity: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join("agentique.json")).unwrap())
                .unwrap();
        // Ids differ between a fresh parse and the identity file; map by name.
        let by_name: std::collections::HashMap<String, u64> = links
            .links
            .iter()
            .map(|l| (l.name.clone(), l.element))
            .collect();
        let state = tree.find("AgentiqueArchitecture::SystemState").unwrap();
        let mut renamed = Links::default();
        for link in &links.links {
            if let Some(id) = tree.find(&link.name) {
                renamed.add(&tree, id, link.kind, &link.path, link.symbol.as_deref());
            }
        }
        assert!(identity["locks"].as_array().is_some_and(|l| !l.is_empty()));
        assert!(by_name.contains_key("AgentiqueArchitecture::SystemState"));
        let r = responsibility(&tree, &renamed, &[state], state).unwrap();
        assert!(r.purpose.contains("one change path"), "{}", r.purpose);
        assert!(
            r.used_by
                .contains(&tree.find("AgentiqueArchitecture::Studio").unwrap())
        );
        assert!(
            r.depends_on
                .contains(&tree.find("AgentiqueArchitecture::History").unwrap())
        );
        assert!(
            r.implemented_in
                .iter()
                .any(|(k, at)| *k == LinkKind::Crate && at == "crates/system-state")
        );
        assert!(r.implemented_in.iter().any(|(k, _)| *k == LinkKind::Test));
        assert!(r.scenarios.len() >= 3, "{:?}", r.scenarios);
        assert!(!r.requirements.is_empty());
    }
}
