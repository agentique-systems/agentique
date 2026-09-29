//! The Library tools (ROADMAP C-49, §4.13): searching building blocks,
//! reading one in words, using one (one System State change, planned by the
//! Library exactly as for the Operator), and saving a definition to My
//! Library after the Operator confirms.

use super::{Prepared, literal, optional_str, required_str};
use agq_language::{ElementId, ElementKind, Parent, Semantics};
use agq_library::{BlockRef, ConnectTo, Index, Library, PlanError, Query, Resolution, Scope, Use};
use agq_system_state::{Actor, Change, SystemState};
use serde_json::Value;

/// The definition kinds a search can be narrowed to, by keyword.
pub(super) const KINDS: &[ElementKind] = &[
    ElementKind::PartDef,
    ElementKind::PortDef,
    ElementKind::ItemDef,
    ElementKind::AttributeDef,
    ElementKind::InterfaceDef,
    ElementKind::ConnectionDef,
    ElementKind::RequirementDef,
];

fn index(state: &SystemState, library: &Library) -> Index {
    Index::build(library, Some((state.tree(), state.revision())))
}

/// A block named as `scope:Qualified::Name`, a qualified name (the
/// project's first, then the built-in library, then My Library) or a plain
/// name that only one block has.
fn block(index: &Index, text: &str) -> Result<usize, String> {
    let text = text.trim();
    if let Some(reference) = BlockRef::parse(text) {
        return index.position(&reference).ok_or_else(|| {
            format!("there is no building block `{text}`; search_library finds blocks by name")
        });
    }
    for scope in [Scope::Project, Scope::BuiltIn, Scope::Mine] {
        if let Some(found) = index.position(&BlockRef::new(scope, text)) {
            return Ok(found);
        }
    }
    let named: Vec<usize> = index
        .blocks()
        .iter()
        .enumerate()
        .filter(|(_, b)| b.name == text && index.project_copy(b).is_none())
        .map(|(i, _)| i)
        .collect();
    match named.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!(
            "there is no building block `{text}`; search_library finds blocks by name"
        )),
        several => {
            let listed: Vec<String> = several
                .iter()
                .map(|i| format!("`{}`", index.blocks()[*i].reference))
                .collect();
            Err(format!(
                "`{text}` names several blocks: {}; name one of them",
                listed.join(", ")
            ))
        }
    }
}

/// A port as a feature chain: `Owner::part.port`, or `Owner::Definition::port`.
/// Returns the card showing it and the port.
fn port_at(state: &SystemState, text: &str) -> Result<(ElementId, ElementId), String> {
    let tree = state.tree();
    let mut steps = text.split('.');
    let first = steps.next().unwrap_or_default().trim();
    let mut current = tree.find(first).ok_or_else(|| {
        format!("there is no element `{first}`; use its qualified name, e.g. `Package::Part`")
    })?;
    let mut card = tree[current].owner().unwrap_or(current);
    let semantics = Semantics::new(tree);
    for step in steps {
        let step = step.trim();
        let next = semantics
            .features(current)
            .into_iter()
            .find(|f| tree.effective_name(*f) == Some(step))
            .ok_or_else(|| format!("`{}` has no feature `{step}`", tree.qualified_name(current)))?;
        card = current;
        current = next;
    }
    if tree[current].kind != ElementKind::Port {
        return Err(format!(
            "`{text}` is a {}, not a port",
            tree[current].kind.keyword()
        ));
    }
    Ok((card, current))
}

/// `search_library`: blocks matching words, a kind, a scope and a port.
pub(super) fn search(
    state: &SystemState,
    library: &Library,
    input: &Value,
) -> Result<String, String> {
    let index = index(state, library);
    let text = optional_str(input, "query")?.unwrap_or("");
    let kinds: Vec<ElementKind> = match optional_str(input, "kind")? {
        Some(keyword) => vec![
            KINDS
                .iter()
                .copied()
                .find(|k| k.keyword() == keyword)
                .ok_or_else(|| format!("unknown kind `{keyword}`"))?,
        ],
        None => Vec::new(),
    };
    let scope = match optional_str(input, "scope")? {
        None | Some("all") => None,
        Some(key) => Some(Scope::from_key(key).ok_or_else(|| format!("unknown scope `{key}`"))?),
    };
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(12)
        .clamp(1, 40) as usize;
    let fits = match optional_str(input, "fits_port")? {
        Some(port) => {
            let (_, port_id) = port_at(state, port)?;
            Some((
                port.to_string(),
                library.compatible(&index, state.tree(), port_id, false),
            ))
        }
        None => None,
    };
    let only: Option<Vec<usize>> = fits
        .as_ref()
        .map(|(_, fits)| fits.iter().map(|f| f.block).collect());
    let hits = index.search(&Query {
        text,
        scope,
        kinds: &kinds,
        only: only.as_deref(),
        limit,
    });
    let mut lines = Vec::new();
    let what = match (&fits, text.trim().is_empty()) {
        (Some((port, _)), true) => format!("that fit `{port}`"),
        (Some((port, _)), false) => format!("for \"{text}\" that fit `{port}`"),
        (None, true) => "in the Library".to_string(),
        (None, false) => format!("for \"{text}\""),
    };
    lines.push(match hits.len() {
        0 => format!("No building blocks {what}."),
        1 => format!("1 building block {what}:"),
        n => format!("{n} building blocks {what}:"),
    });
    for hit in &hits {
        let b = &index.blocks()[hit.block];
        let mut line = format!("- `{}` — {}", b.reference, b.kind_label());
        if b.composite() {
            line.push_str(", composite");
        }
        line.push_str(&format!(", {}", b.source_label()));
        if !b.category.is_empty() {
            line.push_str(&format!(" ({})", b.category.join(" › ")));
        }
        if b.reference.scope == Scope::Project && b.usages > 0 {
            line.push_str(&format!(", used {}×", b.usages));
        }
        if !b.summary.is_empty() {
            line.push_str(&format!(": {}", b.summary));
        }
        if !b.ports.is_empty() {
            let ports: Vec<String> = b
                .ports
                .iter()
                .map(|p| format!("{} : {}", p.name, p.type_name))
                .collect();
            line.push_str(&format!(" Ports: {}.", ports.join(", ")));
        }
        if let Some((_, fits)) = &fits
            && let Some(fit) = fits.iter().find(|f| f.block == hit.block)
        {
            line.push_str(&format!(" Fits with: {}.", fit.ports.join(", ")));
        }
        lines.push(line);
    }
    if hits.is_empty() {
        if fits.is_some() {
            lines
                .push("No block has a port that fits; model what is needed in the project.".into());
        } else {
            lines.push(
                "Try other words, or model the concept in the project if nothing fits.".into(),
            );
        }
    } else {
        lines.push("Read a block with read_library_block before using it. Built-in and My Library blocks are copied into the project's Library package when used.".into());
    }
    Ok(lines.join("\n"))
}

/// `read_library_block`: one block in words.
pub(super) fn read(
    state: &SystemState,
    library: &Library,
    input: &Value,
) -> Result<String, String> {
    let index = index(state, library);
    let found = block(&index, required_str(input, "block")?)?;
    Ok(library.describe(&index, found, Some(state.tree())))
}

/// `use_library_block`: one change that brings the block in and uses it.
pub(super) fn use_block(
    state: &SystemState,
    library: &Library,
    input: &Value,
) -> Result<Change, String> {
    let index = index(state, library);
    let found = block(&index, required_str(input, "block")?)?;
    let tree = state.tree();
    let parent_name = required_str(input, "parent")?;
    let parent = tree.find(parent_name).ok_or_else(|| {
        format!("there is no element `{parent_name}`; use its qualified name, e.g. `Package::Part`")
    })?;
    let mut request = Use::new(
        index.blocks()[found].reference.clone(),
        Parent::Element(parent),
    );
    request.name = optional_str(input, "name")?.map(str::to_string);
    if let Some(values) = input.get("values") {
        let Value::Object(values) = values else {
            return Err("`values` must map attribute names to values".into());
        };
        for (name, value) in values {
            request.values.push((name.clone(), literal(value)?));
        }
    }
    if let Some(chain) = optional_str(input, "connect_to")? {
        let full = format!("{parent_name}.{chain}");
        let (card, port) = port_at(state, &full)?;
        let card = if chain.contains('.') { card } else { parent };
        request.connect = Some(ConnectTo {
            card,
            port,
            with: optional_str(input, "connect_with")?.map(str::to_string),
        });
    }
    request.resolution = match optional_str(input, "if_exists")? {
        None | Some("ask") => Resolution::Ask,
        Some("use_existing") => Resolution::UseExisting,
        Some("copy_renamed") => Resolution::Rename,
        Some(other) => return Err(format!("unknown if_exists `{other}`")),
    };
    library
        .plan_use(state, &request, Actor::Assistant)
        .map(|plan| plan.change)
        .map_err(|error| match error {
            PlanError::Conflicts(conflicts) => {
                let listed: Vec<String> = conflicts.iter().map(ToString::to_string).collect();
                format!(
                    "{}. Nothing was changed. Ask the Operator which to keep, then call again with if_exists \"use_existing\" (use the project's definition) or \"copy_renamed\" (copy the block's under another name)",
                    listed.join("; ")
                )
            }
            other => other.to_string(),
        })
}

/// `save_to_library`: a planned My Library change, for the Operator to
/// confirm.
pub(super) fn save(
    state: &SystemState,
    library: &Library,
    input: &Value,
) -> Result<Prepared, String> {
    let tree = state.tree();
    let name = required_str(input, "definition")?;
    let definition = tree
        .find(name)
        .ok_or_else(|| format!("there is no element `{name}`; use its qualified name"))?;
    let category = match optional_str(input, "category")? {
        Some(category) => category.to_string(),
        None => tree
            .get(definition)
            .and_then(|e| e.owner())
            .and_then(|o| tree.effective_name(o))
            .unwrap_or("Blocks")
            .to_string(),
    };
    let replace = input.get("replace") == Some(&Value::Bool(true));
    let plan = library
        .plan_save(tree, definition, &category, replace)
        .map_err(|error| match error {
            PlanError::Conflicts(conflicts) => {
                let listed: Vec<String> = conflicts.iter().map(ToString::to_string).collect();
                format!(
                    "My Library: {}. Nothing was saved. Ask the Operator whether to replace it, then call again with replace: true",
                    listed.join("; ")
                )
            }
            other => other.to_string(),
        })?;
    let added = plan.added.len();
    let question = format!(
        "Save `{name}` to My Library as `{}`? It will be offered in every project, and a project that uses it gets its own copy.{}",
        plan.block.qualified_name,
        if plan.replaced.is_empty() {
            String::new()
        } else {
            format!(" This replaces My Library's {}.", plan.replaced.join(", "))
        }
    );
    let saved = format!(
        "Saved to My Library as `{}` ({added} definition(s) added, {} already there). The Operator can use it in any project.",
        plan.block,
        plan.reused.len()
    );
    Ok(Prepared::SaveToLibrary {
        plan: Box::new(plan),
        question,
        saved,
    })
}
