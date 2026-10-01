//! Describing a block: a compact summary in words for the Assistant (never
//! SysML text), and a structural preview for the Studio: the block's public
//! ports, its major parts in columns from where requests enter to where
//! they leave, and the connections between them.

use crate::copy::{closure, is_standard, path_of};
use crate::index::{Block, Feature, all_generals, feature_of, name_of};
use crate::{Index, Library, Scope};
use agq_language::{ElementId, ElementKind, Semantics, Tree};

/// A port on the preview's edge or on an inner part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPort {
    pub name: String,
    pub type_name: String,
    /// Items come in first: drawn on the left.
    pub inbound: bool,
    pub inherited: bool,
}

/// An inner part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewPart {
    pub name: String,
    pub type_name: String,
    /// As written, such as `[1..*]`.
    pub multiplicity: Option<String>,
    pub ports: Vec<PreviewPort>,
    /// Its type has parts of its own.
    pub composite: bool,
    pub inherited: bool,
    /// Where it is drawn: columns run from where requests enter to where
    /// they leave; rows stack parts of one column.
    pub column: usize,
    pub row: usize,
}

/// One end of a preview link: a port on the edge, or an inner part (and
/// one of its ports).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewEnd {
    Boundary(usize),
    Part(usize, Option<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewLink {
    pub from: PreviewEnd,
    pub to: PreviewEnd,
    /// The connection's name or type, if it has one.
    pub label: String,
}

/// What a block is, what it exposes and what is inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub name: String,
    pub kind_label: String,
    pub ports: Vec<PreviewPort>,
    pub parts: Vec<PreviewPart>,
    pub links: Vec<PreviewLink>,
    /// `(name, "= 300")` or `(name, ": Positive")`.
    pub attributes: Vec<(String, String)>,
    /// `(name, text)`.
    pub requirements: Vec<(String, String)>,
    /// Directed items of a port definition: `(name, "in : Request")`.
    pub items: Vec<(String, String)>,
    pub columns: usize,
}

impl Library {
    /// The structural preview of a block.
    pub fn preview(&self, index: &Index, block: usize, project: Option<&Tree>) -> Option<Preview> {
        let block = index.get(block)?;
        let located = self.locate(&block.reference, project)?;
        let tree = located.tree;
        let semantics = Semantics::new(tree);
        let port = |f: &Feature| PreviewPort {
            name: f.name.clone(),
            type_name: f.type_name.clone(),
            inbound: f.inbound,
            inherited: f.inherited_from.is_some(),
        };
        let mut parts: Vec<PreviewPart> = block
            .parts
            .iter()
            .map(|part| {
                let features = semantics.features(part.element);
                let ports = features
                    .iter()
                    .filter(|f| semantics.element(**f).map(|e| e.kind) == Some(ElementKind::Port))
                    .map(|f| port(&feature_of(tree, &semantics, part.element, *f)))
                    .collect();
                let composite = semantics.types_of(part.element).iter().any(|(ty, _)| {
                    semantics
                        .features(*ty)
                        .iter()
                        .any(|f| semantics.element(*f).map(|e| e.kind) == Some(ElementKind::Part))
                });
                PreviewPart {
                    name: part.name.clone(),
                    type_name: part.type_name.clone(),
                    multiplicity: part.multiplicity.map(|m| m.to_string()),
                    ports,
                    composite,
                    inherited: part.inherited_from.is_some(),
                    column: 0,
                    row: 0,
                }
            })
            .collect();
        let ports: Vec<PreviewPort> = block.ports.iter().map(port).collect();
        let links = links(tree, &semantics, block, &ports, &parts);
        let columns = arrange(&mut parts, &links, &ports);
        Some(Preview {
            name: block.name.clone(),
            kind_label: block.kind_label(),
            ports,
            parts,
            links,
            attributes: block
                .attributes
                .iter()
                .map(|a| (a.name.clone(), value_text(a)))
                .collect(),
            requirements: block
                .requirements
                .iter()
                .map(|r| (r.name.clone(), r.text.clone()))
                .collect(),
            items: block
                .items
                .iter()
                .map(|i| {
                    let direction = semantics
                        .element(i.element)
                        .and_then(|e| e.direction)
                        .map_or(String::new(), |d| format!("{} ", d.keyword()));
                    (
                        i.name.clone(),
                        format!("{direction}: {}", i.type_name).trim().to_string(),
                    )
                })
                .collect(),
            columns,
        })
    }

    /// A block in words for the Assistant: purpose, identity, public ports
    /// and their items, parts, connections, settings, requirements, what it
    /// needs and whether the project has it.
    pub fn describe(&self, index: &Index, block: usize, project: Option<&Tree>) -> String {
        let Some(summary) = index.get(block) else {
            return "There is no such block.".into();
        };
        let Some(located) = self.locate(&summary.reference, project) else {
            return format!("`{}` could not be found.", summary.reference);
        };
        let tree = located.tree;
        let semantics = Semantics::new(tree);
        let mut lines = Vec::new();
        let mut head = format!(
            "`{}` ({}), {}: {}",
            summary.reference,
            summary.kind_label(),
            summary.source_label(),
            summary.qualified_name
        );
        if summary.composite() {
            head.push_str(" — composite");
        }
        lines.push(head);
        if !summary.doc.is_empty() {
            lines.push(format!("Purpose: {}", summary.doc));
        }
        if !summary.generals.is_empty() {
            lines.push(format!("Specialises: {}", summary.generals.join(", ")));
        }
        let feature_line = |f: &Feature| {
            let mut line = format!("{} : {}", f.name, f.type_name);
            if let Some(m) = f.multiplicity {
                line.push_str(&format!(" {m}"));
            }
            if let Some(v) = &f.value {
                line.push_str(&format!(" = {v}"));
            }
            if let Some(from) = &f.inherited_from {
                line.push_str(&format!(" (from {from})"));
            }
            line
        };
        if !summary.ports.is_empty() {
            let ports: Vec<String> = summary
                .ports
                .iter()
                .map(|p| {
                    format!(
                        "{} — {}",
                        feature_line(p),
                        if p.inbound {
                            "serves or receives"
                        } else {
                            "calls or sends"
                        }
                    )
                })
                .collect();
            lines.push(format!("Ports: {}", ports.join("; ")));
            // What each port type carries, once.
            let mut seen = Vec::new();
            let mut carried = Vec::new();
            for p in &summary.ports {
                for (ty, _) in semantics.types_of(p.element) {
                    if seen.contains(&ty) {
                        continue;
                    }
                    seen.push(ty);
                    let items: Vec<String> = semantics
                        .features(ty)
                        .into_iter()
                        .filter_map(|f| {
                            let e = semantics.element(f)?;
                            let direction = e.direction?;
                            let item_type = semantics
                                .types_of(f)
                                .first()
                                .and_then(|(t, _)| name_of(tree, *t))
                                .unwrap_or_else(|| "untyped".into());
                            Some(format!(
                                "{} {} : {item_type}",
                                direction.keyword(),
                                tree.effective_name(f).unwrap_or("")
                            ))
                        })
                        .collect();
                    carried.push(format!(
                        "{} carries {}",
                        name_of(tree, ty).unwrap_or_default(),
                        if items.is_empty() {
                            "nothing directed".into()
                        } else {
                            items.join(", ")
                        }
                    ));
                }
            }
            lines.push(format!(
                "Port types: {}. A conjugated port (~) reverses every direction; ports fit when every item meets one of the opposite direction.",
                carried.join("; ")
            ));
        }
        if !summary.items.is_empty() {
            let items: Vec<String> = summary.items.iter().map(feature_line).collect();
            lines.push(format!("Items: {}", items.join("; ")));
        }
        if !summary.parts.is_empty() {
            let parts: Vec<String> = summary.parts.iter().map(feature_line).collect();
            lines.push(format!("Parts: {}", parts.join("; ")));
        }
        let connections = connections_text(tree, &semantics, summary.element);
        if !connections.is_empty() {
            lines.push(format!("Connections: {}", connections.join("; ")));
        }
        if !summary.attributes.is_empty() {
            let attributes: Vec<String> = summary.attributes.iter().map(feature_line).collect();
            lines.push(format!("Attributes: {}", attributes.join("; ")));
        }
        // Settings of inner parts, which a usage can override by redefinition.
        let mut inner = Vec::new();
        for part in &summary.parts {
            for f in semantics.features(part.element) {
                let e = semantics.element(f).expect("a feature exists");
                if e.kind == ElementKind::Attribute {
                    let feature = feature_of(tree, &semantics, part.element, f);
                    inner.push(format!("{}.{}", part.name, feature_line(&feature)));
                }
            }
        }
        if !inner.is_empty() {
            lines.push(format!("Inner attributes: {}", inner.join("; ")));
        }
        if !summary.requirements.is_empty() {
            let requirements: Vec<String> = summary
                .requirements
                .iter()
                .map(|r| format!("{} : {} — {}", r.name, r.type_name, r.text))
                .collect();
            lines.push(format!("Requirements: {}", requirements.join("; ")));
        }
        // Behaviour it runs, and the scenarios that come with it (C-50).
        let machine = semantics
            .features(located.element)
            .into_iter()
            .chain(tree[located.element].children().iter().copied())
            .find(|f| {
                semantics
                    .element(*f)
                    .is_some_and(|e| e.kind == ElementKind::State && e.exhibit)
            });
        if let Some(machine) = machine {
            let states = tree
                .get(machine)
                .map(|m| {
                    m.children()
                        .iter()
                        .filter(|c| tree[**c].kind == ElementKind::State)
                        .filter_map(|c| tree.effective_name(*c))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            lines.push(format!(
                "Behaviour: a state machine that runs in scenarios (states {states})."
            ));
        }
        let scenarios: Vec<&str> = tree
            .walk()
            .into_iter()
            .filter(|id| tree[*id].kind == ElementKind::VerificationDef)
            .filter(|id| {
                tree[*id].children().iter().any(|c| {
                    tree[*c].kind == ElementKind::Subject
                        && tree[*c].typed_by.first().and_then(|r| r.target())
                            == Some(located.element)
                })
            })
            .filter_map(|id| tree.effective_name(id))
            .collect();
        if !scenarios.is_empty() {
            lines.push(format!(
                "Scenarios that come with it: {}.",
                scenarios.join(", ")
            ));
        }
        if summary.reference.scope != Scope::Project && !located.standard {
            match closure(tree, located.element) {
                Ok(units) => {
                    let needed: Vec<String> = units
                        .iter()
                        .filter(|u| **u != located.element)
                        .map(|u| path_of(tree, *u).join("::"))
                        .collect();
                    let present: Vec<&String> = needed
                        .iter()
                        .filter(|q| project.is_some_and(|p| p.find(q).is_some()))
                        .collect();
                    if !needed.is_empty() {
                        lines.push(format!(
                            "Needs (copied with it unless the project has them): {}{}",
                            needed.join(", "),
                            if present.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    " — already in the project: {}",
                                    present
                                        .iter()
                                        .map(|s| s.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )
                            }
                        ));
                    }
                }
                Err(missing) => {
                    let listed: Vec<String> = missing.iter().map(ToString::to_string).collect();
                    lines.push(format!("Cannot be used: {}", listed.join("; ")));
                }
            }
            match index.project_copy(summary) {
                Some(copy) => lines.push(format!(
                    "In the project: copied as `{}`, unchanged; used by {} usage(s).",
                    index.blocks()[copy].qualified_name,
                    index.blocks()[copy].usages
                )),
                None if project.is_some_and(|p| p.find(&summary.qualified_name).is_some()) => {
                    lines.push(format!(
                        "In the project: a different `{}` exists; using this block reports the conflict.",
                        summary.qualified_name
                    ))
                }
                None => lines.push("In the project: not yet.".into()),
            }
        } else if summary.reference.scope == Scope::Project {
            lines.push(format!(
                "In the project: used by {} usage(s), specialised by {} definition(s){}.",
                summary.usages,
                summary.specializations,
                summary
                    .origin
                    .map(|o| format!(
                        "; a {} copy from {}",
                        if o.changed { "changed" } else { "unchanged" },
                        o.scope.label()
                    ))
                    .unwrap_or_default()
            ));
        }
        if summary.problems > 0 {
            lines.push(format!("Problems in My Library: {}", summary.problems));
        }
        lines.join("\n")
    }
}

/// `The value as written`, or `: Type [m]` when there is none.
fn value_text(feature: &Feature) -> String {
    match &feature.value {
        Some(value) => format!("= {value}"),
        None => {
            let mut text = format!(": {}", feature.type_name);
            if let Some(m) = feature.multiplicity {
                text.push_str(&format!(" {m}"));
            }
            text
        }
    }
}

/// The connections of a definition (owned and inherited), as written.
fn connections_text(tree: &Tree, semantics: &Semantics, definition: ElementId) -> Vec<String> {
    all_generals(semantics, definition)
        .into_iter()
        .filter(|g| !is_standard(*g))
        .filter_map(|g| tree.get(g))
        .flat_map(|g| g.children().to_vec())
        .filter(|c| {
            matches!(
                tree[*c].kind,
                ElementKind::Connection | ElementKind::Interface
            ) && tree[*c].ends.len() == 2
        })
        .map(|c| {
            let e = &tree[c];
            let mut label = String::new();
            if let Some(name) = &e.name {
                label.push_str(name);
            }
            if let Some(ty) = e.typed_by.first() {
                if !label.is_empty() {
                    label.push(' ');
                }
                label.push_str(&format!(": {}", ty.last_name()));
            }
            let ends = format!("{} to {}", e.ends[0], e.ends[1]);
            if label.is_empty() {
                ends
            } else {
                format!("{label}, {ends}")
            }
        })
        .collect()
}

/// The connections of a block as preview links between its edge ports and
/// its inner parts' ports.
fn links(
    tree: &Tree,
    semantics: &Semantics,
    block: &Block,
    ports: &[PreviewPort],
    parts: &[PreviewPart],
) -> Vec<PreviewLink> {
    let end = |reference: &agq_language::Reference| -> Option<PreviewEnd> {
        let names: Vec<String> = reference
            .steps
            .iter()
            .map(|s| {
                s.target
                    .and_then(|t| tree.effective_name(t).map(str::to_string))
                    .unwrap_or_else(|| s.name.last().to_string())
            })
            .collect();
        match names.as_slice() {
            [one] => ports
                .iter()
                .position(|p| p.name == *one)
                .map(PreviewEnd::Boundary)
                .or_else(|| {
                    parts
                        .iter()
                        .position(|p| p.name == *one)
                        .map(|i| PreviewEnd::Part(i, None))
                }),
            [part, port, ..] => parts
                .iter()
                .position(|p| p.name == *part)
                .map(|i| PreviewEnd::Part(i, parts[i].ports.iter().position(|q| q.name == *port))),
            [] => None,
        }
    };
    all_generals(semantics, block.element)
        .into_iter()
        .filter(|g| !is_standard(*g))
        .filter_map(|g| tree.get(g))
        .flat_map(|g| g.children().to_vec())
        .filter_map(|c| {
            let e = &tree[c];
            if !matches!(e.kind, ElementKind::Connection | ElementKind::Interface)
                || e.ends.len() != 2
            {
                return None;
            }
            let label = e
                .name
                .clone()
                .or_else(|| e.typed_by.first().map(|t| t.last_name().to_string()))
                .unwrap_or_default();
            Some(PreviewLink {
                from: end(&e.ends[0])?,
                to: end(&e.ends[1])?,
                label,
            })
        })
        .collect()
}

/// Places parts in columns: a part another part calls or sends to goes to
/// its right; returns the number of columns.
fn arrange(parts: &mut [PreviewPart], links: &[PreviewLink], ports: &[PreviewPort]) -> usize {
    let n = parts.len();
    if n == 0 {
        return 0;
    }
    // Edges between parts, from the calling or sending side.
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let outbound = |end: PreviewEnd| match end {
        PreviewEnd::Part(i, Some(p)) => Some(!parts[i].ports[p].inbound),
        PreviewEnd::Boundary(b) => Some(ports[b].inbound),
        _ => None,
    };
    for link in links {
        if let (PreviewEnd::Part(a, _), PreviewEnd::Part(b, _)) = (link.from, link.to) {
            match (outbound(link.from), outbound(link.to)) {
                (_, Some(true)) | (Some(false), _) => edges.push((b, a)),
                _ => edges.push((a, b)),
            }
        }
    }
    let mut column = vec![0usize; n];
    for _ in 0..n {
        let mut changed = false;
        for (from, to) in &edges {
            if column[*to] < column[*from] + 1 && column[*from] + 1 < n {
                column[*to] = column[*from] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut rows = vec![0usize; n.max(1)];
    for (i, part) in parts.iter_mut().enumerate() {
        part.column = column[i];
        part.row = rows[column[i]];
        rows[column[i]] += 1;
    }
    column.iter().max().map_or(0, |m| m + 1)
}

/// How an element is named to the Operator: its name, or what an unnamed
/// connection joins.
pub(crate) fn display_name(tree: &Tree, id: ElementId) -> String {
    let Some(element) = tree.get(id) else {
        return "(deleted)".into();
    };
    if let Some(name) = tree.effective_name(id) {
        return name.to_string();
    }
    match element.kind {
        ElementKind::Connection | ElementKind::Interface if element.ends.len() == 2 => {
            format!("{} → {}", element.ends[0], element.ends[1])
        }
        kind => kind.keyword().to_string(),
    }
}
