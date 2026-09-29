//! Planning Library actions as ordinary System State changes: using a block
//! (copying what it needs, adding a usage, setting values, connecting it),
//! specialising a definition, overriding an inherited feature, extracting a
//! block from a selection, and saving a definition to My Library.
//!
//! Every plan is tried on a copy of the model before it is handed over, so
//! a plan that is returned applies; the Studio then applies it like any
//! other change (locks ask, undo reverts it in one step).

use crate::copy::{
    CopyError, Creation, Resolution, apply_to_tree, closure, find_path, path_of, plan_copy, unit_of,
};
use crate::{BlockRef, Conflict, Library, Missing, ROOT, Scope, usage_kind};
use agq_language::{
    Element, ElementId, ElementKind, Field, Literal, Parent, QualifiedName, Reference, Role,
    Semantics, Step, Tree, link, printed_reference,
};
use agq_system_state::{Actor, Change, Operation, Property, SystemState};
use std::collections::{BTreeSet, HashMap};
use std::fmt;

/// Where a new usage's port is connected: `port` as shown on `card` (a
/// usage showing a port of its type, or a definition with its own port).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectTo {
    pub card: ElementId,
    pub port: ElementId,
    /// The block's port to connect by name; the first that fits when `None`.
    pub with: Option<String>,
}

/// Using a building block: a usage of it in `parent`.
#[derive(Clone, Debug, PartialEq)]
pub struct Use {
    pub block: BlockRef,
    pub parent: Parent,
    /// The usage's name; a free name after the block's when `None`.
    pub name: Option<String>,
    /// Inherited attributes given values in this usage (`:>> name = value`).
    pub values: Vec<(String, Literal)>,
    pub connect: Option<ConnectTo>,
    pub resolution: Resolution,
}

impl Use {
    pub fn new(block: BlockRef, parent: Parent) -> Self {
        Use {
            block,
            parent,
            name: None,
            values: Vec::new(),
            connect: None,
            resolution: Resolution::Ask,
        }
    }
}

/// A planned change and what it will do.
#[derive(Clone, Debug)]
pub struct Plan {
    pub change: Change,
    /// The main new element (the usage, the new definition), by the id it
    /// gets when the change is applied.
    pub created: Option<ElementId>,
    /// Definitions copied into the project, and identical copies reused,
    /// by qualified name; copies made under another name (from, to).
    pub imported: Vec<String>,
    pub reused: Vec<String>,
    pub renamed: Vec<(String, String)>,
    /// The connection made, as written: `api.backend to publicApi.api`.
    pub connected: Option<String>,
    /// Problems the change leaves at the elements it creates or changes.
    pub problems: Vec<String>,
}

/// Why a plan was not made. The model is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The project already has different definitions with these names.
    Conflicts(Vec<Conflict>),
    /// The block refers to definitions that cannot be found.
    Missing(Vec<Missing>),
    /// It cannot be done, and why, in plain words.
    Invalid(String),
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::Conflicts(conflicts) => {
                let listed: Vec<String> = conflicts.iter().map(ToString::to_string).collect();
                write!(f, "{}", listed.join("; "))
            }
            PlanError::Missing(missing) => {
                let listed: Vec<String> = missing.iter().map(ToString::to_string).collect();
                write!(f, "the block cannot be used: {}", listed.join("; "))
            }
            PlanError::Invalid(reason) => f.write_str(reason),
        }
    }
}

impl std::error::Error for PlanError {}

fn invalid(reason: impl Into<String>) -> PlanError {
    PlanError::Invalid(reason.into())
}

/// `rateLimitedApi` for `RateLimitedApi`, made unique among `parent`'s
/// members with a number.
pub fn default_usage_name(tree: &Tree, parent: Parent, block: &str) -> String {
    let mut chars = block.chars();
    let base: String = match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => "usage".into(),
    };
    let taken = |name: &str| {
        members(tree, parent)
            .iter()
            .any(|m| tree.effective_name(*m) == Some(name))
    };
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}{n}"))
        .find(|name| !taken(name))
        .expect("an unused name exists")
}

fn members(tree: &Tree, parent: Parent) -> Vec<ElementId> {
    match parent {
        Parent::Element(id) => tree
            .get(id)
            .map(|e| e.children().to_vec())
            .unwrap_or_default(),
        Parent::Document(index) => tree
            .documents()
            .get(index)
            .map(|d| d.members().to_vec())
            .unwrap_or_default(),
    }
}

fn check_parent(tree: &Tree, parent: Parent) -> Result<(), PlanError> {
    match parent {
        Parent::Document(index) if index < tree.documents().len() => Ok(()),
        Parent::Document(_) => Err(invalid("the model has no such document")),
        Parent::Element(id) => match tree.get(id) {
            None => Err(invalid(format!("element {id} does not exist"))),
            Some(e) if !Field::Members.fits(e.kind) => Err(invalid(format!(
                "`{}` is a {} and cannot own elements",
                tree.qualified_name(id),
                e.kind.keyword()
            ))),
            Some(_) => Ok(()),
        },
    }
}

/// A one-step reference already pointing at `target`.
fn to(tree: &Tree, target: ElementId, name: &str) -> Reference {
    let name = tree
        .effective_name(target)
        .map(str::to_string)
        .unwrap_or_else(|| name.to_string());
    Reference::to(target, &name)
}

/// A feature chain through `path`, every step pointing at its element.
fn chain(names: &[(ElementId, String)]) -> Reference {
    Reference {
        steps: names
            .iter()
            .map(|(id, name)| Step {
                name: QualifiedName::new([name.as_str()]),
                target: Some(*id),
            })
            .collect(),
    }
}

/// Tries a change on a copy of the state (locks confirmed, since the
/// Operator confirms them when it is applied for real): the ids created in
/// operation order and the problems left at the elements it touches.
fn trial(state: &SystemState, change: &Change) -> Result<(Vec<ElementId>, Vec<String>), PlanError> {
    let mut copy = SystemState::new(state.tree().clone(), state.locks().clone());
    let mut unlocked = change.clone();
    unlocked.base = None;
    unlocked.confirmed = state.locks().iter().copied().collect();
    let event = copy
        .apply(unlocked)
        .map_err(|rejection| invalid(rejection.to_string()))?;
    let touched: BTreeSet<ElementId> = event
        .created
        .iter()
        .chain(&event.updated)
        .copied()
        .collect();
    let problems = copy
        .diagnostics()
        .iter()
        .filter(|d| touched.contains(&d.element))
        .map(|d| format!("`{}`: {}", copy.tree().qualified_name(d.element), d.message))
        .collect();
    Ok((event.created, problems))
}

/// Checks that the creations of a change got the ids the plan gave them.
fn predicted(tree: &Tree, created: &[ElementId], count: usize) -> Result<(), PlanError> {
    let base = tree.next_id().raw();
    let expected: Vec<ElementId> = (0..count as u64)
        .map(|n| ElementId::from_raw(base + n))
        .collect();
    if created.len() < count || created[..count] != expected[..] {
        return Err(invalid(
            "the change could not be prepared consistently; try again",
        ));
    }
    Ok(())
}

impl Library {
    /// Plans using a block: copies of the definitions it needs (reusing
    /// identical ones the project has), a usage typed by it in
    /// `request.parent`, any values, and a connection. One change.
    pub fn plan_use(
        &self,
        state: &SystemState,
        request: &Use,
        actor: Actor,
    ) -> Result<Plan, PlanError> {
        let tree = state.tree();
        let located = self
            .locate(&request.block, Some(tree))
            .ok_or_else(|| invalid(format!("there is no building block `{}`", request.block)))?;
        let block = &located.tree[located.element];
        let block_name = located
            .tree
            .effective_name(located.element)
            .unwrap_or_default()
            .to_string();
        let Some(kind) = usage_kind(block.kind) else {
            return Err(invalid(format!(
                "`{block_name}` is a {}: it types connections between two ports, so connect the ports and choose it as the connection's type",
                block.kind.keyword()
            )));
        };
        if block.is_abstract && kind == ElementKind::Part {
            return Err(invalid(format!(
                "`{block_name}` is abstract: specialise it with a definition of your own, then use that"
            )));
        }
        check_parent(tree, request.parent)?;
        let copied = located.scope != Scope::Project && !located.standard;
        let mut plan = Plan {
            change: Change::new(actor, "", Vec::new()),
            created: None,
            imported: Vec::new(),
            reused: Vec::new(),
            renamed: Vec::new(),
            connected: None,
            problems: Vec::new(),
        };
        let (mut creations, map) = if copied {
            let units = closure(located.tree, located.element).map_err(PlanError::Missing)?;
            let copy = plan_copy(
                located.tree,
                &units,
                tree,
                &|unit| path_of(located.tree, unit),
                request.resolution,
            )
            .map_err(|error| match error {
                CopyError::Conflicts(conflicts) => PlanError::Conflicts(conflicts),
                CopyError::Invalid(reason) => PlanError::Invalid(reason),
            })?;
            plan.imported = copy.created;
            plan.reused = copy.reused;
            plan.renamed = copy.renamed;
            (copy.creations, copy.map)
        } else {
            (Vec::new(), HashMap::new())
        };
        let definition = if copied {
            map[&located.element]
        } else {
            located.element
        };
        let base = tree.next_id().raw();
        let usage = ElementId::from_raw(base + creations.len() as u64);
        let name = match &request.name {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => default_usage_name(tree, request.parent, &block_name),
        };
        let mut element = Element::named(kind, &name);
        let type_name = match plan
            .renamed
            .iter()
            .find(|(from, _)| *from == located.tree.qualified_name(located.element))
        {
            Some((_, renamed)) => renamed
                .rsplit("::")
                .next()
                .unwrap_or(&block_name)
                .to_string(),
            None => block_name.clone(),
        };
        element.typed_by = vec![Reference::to(definition, &type_name)];
        creations.push(Creation {
            parent: request.parent,
            element,
        });
        // Values redefine inherited attributes inside the usage; a dotted
        // name (`cache.ttlSeconds`) reaches an attribute of an inner part
        // through a redefinition of that part, shared by its values.
        if !request.values.is_empty() {
            let semantics = Semantics::new(located.tree);
            let base_id = tree.next_id().raw();
            // Redefinitions created so far: (source feature path) -> new id.
            let mut made: Vec<(Vec<ElementId>, ElementId)> = Vec::new();
            for (path_name, value) in &request.values {
                let mut owner = located.element;
                let mut container = usage;
                let mut path: Vec<ElementId> = Vec::new();
                let steps: Vec<&str> = path_name.split('.').map(str::trim).collect();
                for (depth, step) in steps.iter().enumerate() {
                    let last = depth + 1 == steps.len();
                    let feature = semantics
                        .features(owner)
                        .into_iter()
                        .find(|f| located.tree.effective_name(*f) == Some(*step))
                        .ok_or_else(|| {
                            invalid(format!(
                                "`{block_name}` has no {} `{path_name}` to give a value",
                                if steps.len() > 1 {
                                    "feature"
                                } else {
                                    "attribute"
                                }
                            ))
                        })?;
                    let kind = semantics
                        .element(feature)
                        .map(|e| e.kind)
                        .unwrap_or(ElementKind::Attribute);
                    if last && kind != ElementKind::Attribute {
                        return Err(invalid(format!(
                            "`{path_name}` is a {}, not an attribute; only attributes take values",
                            kind.keyword()
                        )));
                    }
                    path.push(feature);
                    let target = if copied { map[&feature] } else { feature };
                    if !last {
                        if let Some((_, id)) = made.iter().find(|(p, _)| *p == path) {
                            container = *id;
                        } else {
                            let mut redefinition = Element::new(kind);
                            redefinition.redefines = vec![Reference::to(target, step)];
                            let id = ElementId::from_raw(base_id + creations.len() as u64);
                            creations.push(Creation {
                                parent: Parent::Element(container),
                                element: redefinition,
                            });
                            made.push((path.clone(), id));
                            container = id;
                        }
                        owner = feature;
                        continue;
                    }
                    let mut redefinition = Element::new(ElementKind::Attribute);
                    redefinition.redefines = vec![Reference::to(target, step)];
                    redefinition.value = Some(value.clone());
                    creations.push(Creation {
                        parent: Parent::Element(container),
                        element: redefinition,
                    });
                }
            }
        }
        let count = creations.len();
        let mut operations: Vec<Operation> = creations
            .into_iter()
            .map(|c| Operation::Create {
                parent: c.parent,
                element: Box::new(c.element),
            })
            .collect();
        let source = match located.scope {
            Scope::Project => String::new(),
            _ if located.standard => String::new(),
            _ => " from the Library".to_string(),
        };
        let mut description = format!("Add {name} : {type_name}{source}");
        if let Some(connect) = &request.connect {
            let (operation, written) = self.connection(
                state,
                &operations,
                request.parent,
                (usage, &name),
                connect,
                actor,
            )?;
            operations.push(operation);
            description = format!("{description}, connected to {}", written.1);
            plan.connected = Some(format!("{} to {}", written.0, written.1));
        }
        let change = Change::new(actor, &description, operations).with_base(state.revision());
        let (created, problems) = trial(state, &change)?;
        predicted(tree, &created, count)?;
        plan.change = change;
        plan.created = Some(usage);
        plan.problems = problems;
        Ok(plan)
    }

    /// The connection from the new usage's port to `connect.port` on
    /// `connect.card`: found by trying the usage's creation on a copy and
    /// asking the language which of its ports fit. Returns the operation
    /// and the two ends as written (the usage's first).
    fn connection(
        &self,
        state: &SystemState,
        operations: &[Operation],
        parent: Parent,
        (usage, usage_name): (ElementId, &str),
        connect: &ConnectTo,
        actor: Actor,
    ) -> Result<(Operation, (String, String)), PlanError> {
        let Parent::Element(owner) = parent else {
            return Err(invalid(
                "a connection needs an owner: add the block inside a part or part definition",
            ));
        };
        let tree = state.tree();
        // The path from the owner down to the card, then the port.
        let mut path = Vec::new();
        let mut current = connect.card;
        while current != owner {
            path.push(current);
            current = tree.get(current).and_then(Element::owner).ok_or_else(|| {
                invalid(format!(
                    "`{}` is not inside `{}`, where the new block goes",
                    tree.qualified_name(connect.card),
                    tree.qualified_name(owner)
                ))
            })?;
        }
        path.reverse();
        let passes_on = path.is_empty();
        let mut other: Vec<(ElementId, String)> = path
            .iter()
            .map(|id| (*id, tree.effective_name(*id).unwrap_or("").to_string()))
            .collect();
        other.push((
            connect.port,
            tree.effective_name(connect.port).unwrap_or("").to_string(),
        ));
        let mut copy = SystemState::new(tree.clone(), state.locks().clone());
        let mut change = Change::new(actor, "trial", operations.to_vec());
        change.confirmed = state.locks().iter().copied().collect();
        copy.apply(change)
            .map_err(|rejection| invalid(rejection.to_string()))?;
        let semantics = Semantics::new(copy.tree());
        let ports: Vec<ElementId> = semantics
            .features(usage)
            .into_iter()
            .filter(|f| semantics.element(*f).map(|e| e.kind) == Some(ElementKind::Port))
            .collect();
        let candidates: Vec<ElementId> = match &connect.with {
            Some(name) => {
                let found: Vec<ElementId> = ports
                    .iter()
                    .copied()
                    .filter(|p| copy.tree().effective_name(*p) == Some(name.as_str()))
                    .collect();
                if found.is_empty() {
                    return Err(invalid(format!("`{usage_name}` has no port `{name}`")));
                }
                found
            }
            None => ports.clone(),
        };
        let mut reasons = Vec::new();
        let chosen = candidates.iter().copied().find(|port| {
            match semantics.ports_fit(connect.port, *port, passes_on) {
                Ok(()) => true,
                Err(reason) => {
                    reasons.push(format!(
                        "`{}`: {reason}",
                        copy.tree().effective_name(*port).unwrap_or("")
                    ));
                    false
                }
            }
        });
        let Some(port) = chosen else {
            let target = other
                .iter()
                .map(|(_, n)| n.as_str())
                .collect::<Vec<_>>()
                .join(".");
            return Err(invalid(if ports.is_empty() {
                format!("`{usage_name}` has no ports to connect to `{target}`")
            } else {
                format!(
                    "no port of `{usage_name}` fits `{target}`: {}",
                    reasons.join("; ")
                )
            }));
        };
        let mine = chain(&[
            (usage, usage_name.to_string()),
            (
                port,
                copy.tree().effective_name(port).unwrap_or("").to_string(),
            ),
        ]);
        let theirs = chain(&other);
        let written = (mine.to_string(), theirs.to_string());
        let kind = if passes_on {
            ElementKind::Connection
        } else {
            ElementKind::Interface
        };
        Ok((
            Operation::Connect {
                parent: owner,
                kind,
                name: None,
                definition: None,
                from: theirs,
                to: mine,
            },
            written,
        ))
    }

    /// Plans a specialisation of a project definition: `kind def name :>
    /// definition`, beside the Operator's own definitions (never inside the
    /// copied `Library` package), and, with `retype`, that usage typed by
    /// it instead.
    pub fn plan_specialize(
        &self,
        state: &SystemState,
        definition: ElementId,
        name: &str,
        retype: Option<ElementId>,
        actor: Actor,
    ) -> Result<Plan, PlanError> {
        let tree = state.tree();
        let general = tree
            .get(definition)
            .filter(|e| e.kind.is_definition())
            .ok_or_else(|| invalid("choose a definition to specialise"))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(invalid("the specialisation needs a name"));
        }
        let general_name = tree
            .effective_name(definition)
            .unwrap_or_default()
            .to_string();
        let package = own_package(tree, retype.unwrap_or(definition));
        if members(tree, package)
            .iter()
            .any(|m| tree.effective_name(*m) == Some(name))
        {
            return Err(invalid(format!(
                "`{name}` is already used there; choose another name"
            )));
        }
        let new = tree.next_id();
        let mut element = Element::named(general.kind, name);
        element.specializes = vec![to(tree, definition, &general_name)];
        let mut operations = vec![Operation::Create {
            parent: package,
            element: Box::new(element),
        }];
        let mut description = format!("Specialise {general_name} as {name}");
        if let Some(usage) = retype {
            let usage_element = tree
                .get(usage)
                .filter(|e| e.kind.is_usage())
                .ok_or_else(|| invalid("only a usage can be typed by the specialisation"))?;
            operations.push(Operation::Set {
                element: usage,
                property: Property::TypedBy(vec![Reference::to(new, name)]),
            });
            if usage_element.conjugated {
                operations.push(Operation::Set {
                    element: usage,
                    property: Property::Conjugated(false),
                });
            }
            description = format!(
                "{description} for {}",
                tree.effective_name(usage).unwrap_or("the usage")
            );
        }
        let change = Change::new(actor, &description, operations).with_base(state.revision());
        let (created, problems) = trial(state, &change)?;
        predicted(tree, &created, 1)?;
        Ok(Plan {
            change,
            created: Some(new),
            imported: Vec::new(),
            reused: Vec::new(),
            renamed: Vec::new(),
            connected: None,
            problems,
        })
    }

    /// Plans overriding an inherited feature of `owner` (a usage or a
    /// definition) by redefinition, only there: `path` runs from a feature
    /// of `owner` to the feature to override (`[cache, ttlSeconds]` for the
    /// cache's time to live inside a cached store). Redefinitions already
    /// there are changed rather than repeated.
    pub fn plan_override(
        &self,
        state: &SystemState,
        owner: ElementId,
        path: &[ElementId],
        what: Override,
        actor: Actor,
    ) -> Result<Plan, PlanError> {
        let tree = state.tree();
        if path.is_empty() || !tree.contains(owner) {
            return Err(invalid("choose an inherited feature to override"));
        }
        let semantics = Semantics::new(tree);
        let mut operations = Vec::new();
        let mut next = tree.next_id().raw();
        // The element the next redefinition goes into, and whether it is
        // already in the model (not created by this change).
        let mut container = owner;
        let mut exists = true;
        let mut names = Vec::new();
        for (depth, feature) in path.iter().copied().enumerate() {
            let last = depth + 1 == path.len();
            let feature_element = tree
                .get(feature)
                .ok_or_else(|| invalid("the feature to override does not exist"))?;
            let name = tree.effective_name(feature).unwrap_or_default().to_string();
            names.push(name.clone());
            if exists {
                if let Some(existing) = redefinition_in(tree, &semantics, container, feature) {
                    if last {
                        operations.extend(what.operations(tree, existing));
                    } else {
                        container = existing;
                    }
                    continue;
                }
                if tree[container].children().contains(&feature) {
                    if last {
                        return Err(invalid(format!(
                            "`{name}` belongs to `{}` itself: change it there instead of overriding it",
                            tree.qualified_name(container)
                        )));
                    }
                    container = feature;
                    continue;
                }
                if !semantics.features(container).contains(&feature) {
                    return Err(invalid(format!(
                        "`{name}` is not a feature of `{}`",
                        tree.qualified_name(container)
                    )));
                }
            }
            let mut redefinition = Element::new(feature_element.kind);
            redefinition.redefines = vec![Reference::to(feature, &name)];
            if last {
                match &what {
                    Override::Value(value) => {
                        if feature_element.kind != ElementKind::Attribute {
                            return Err(invalid(format!(
                                "only an attribute has a value; `{name}` is a {}",
                                feature_element.kind.keyword()
                            )));
                        }
                        redefinition.value = Some(value.clone());
                    }
                    Override::Type(definition) => {
                        redefinition.typed_by = vec![to(tree, *definition, "")];
                    }
                }
            }
            operations.push(Operation::Create {
                parent: Parent::Element(container),
                element: Box::new(redefinition),
            });
            container = ElementId::from_raw(next);
            next += 1;
            exists = false;
        }
        let owner_name = tree.effective_name(owner).unwrap_or("it");
        let description = match &what {
            Override::Value(value) => {
                format!("Override {} in {owner_name} with {value}", names.join("."))
            }
            Override::Type(definition) => format!(
                "Override the type of {} in {owner_name} with {}",
                names.join("."),
                tree.effective_name(*definition).unwrap_or("another type")
            ),
        };
        let count = operations
            .iter()
            .filter(|o| matches!(o, Operation::Create { .. }))
            .count();
        let change = Change::new(actor, &description, operations).with_base(state.revision());
        let (created, problems) = trial(state, &change)?;
        predicted(tree, &created, count)?;
        Ok(Plan {
            change,
            created: created.last().copied(),
            imported: Vec::new(),
            reused: Vec::new(),
            renamed: Vec::new(),
            connected: None,
            problems,
        })
    }

    /// Plans saving a project definition to My Library, with what it needs:
    /// definitions the project copied from the Library keep their place
    /// under `Library`; the Operator's own go under `Library::<category>`.
    /// My Library's identical copies are reused; a different one with the
    /// same name is a conflict, unless `replace` says to replace it.
    pub fn plan_save(
        &self,
        project: &Tree,
        definition: ElementId,
        category: &str,
        replace: bool,
    ) -> Result<SavePlan, PlanError> {
        project
            .get(definition)
            .filter(|e| e.kind.is_definition())
            .ok_or_else(|| invalid("choose a definition to save"))?;
        let category = category.trim();
        if category.is_empty() || category.contains("::") || category.contains('\n') {
            return Err(invalid("the category is a plain name, such as `Storage`"));
        }
        let units = closure(project, definition).map_err(PlanError::Missing)?;
        let place = |unit: ElementId| -> Vec<String> {
            let path = path_of(project, unit);
            if path.first().map(String::as_str) == Some(ROOT) {
                path
            } else {
                vec![
                    ROOT.to_string(),
                    category.to_string(),
                    project.effective_name(unit).unwrap_or_default().to_string(),
                ]
            }
        };
        let mut mine = self.mine().clone();
        let mut replaced = Vec::new();
        if replace {
            // Replace what differs: remove it first, and bind references to
            // it again by name once the new copy is in.
            let copy = plan_copy(project, &units, &mine, &place, Resolution::Ask);
            if let Err(CopyError::Conflicts(conflicts)) = copy {
                for conflict in &conflicts {
                    replaced.push(conflict.qualified_name.clone());
                    remove_keeping_names(&mut mine, conflict.existing);
                }
            }
        }
        let copy =
            plan_copy(project, &units, &mine, &place, Resolution::Ask).map_err(
                |error| match error {
                    CopyError::Conflicts(conflicts) => PlanError::Conflicts(conflicts),
                    CopyError::Invalid(reason) => PlanError::Invalid(reason),
                },
            )?;
        let saved = place(unit_of(project, definition)).join("::");
        let added = copy.created.clone();
        let reused = copy.reused.clone();
        apply_to_tree(&mut mine, copy.creations).map_err(invalid)?;
        link(&mut mine);
        let problems = agq_language::validate(&mine)
            .into_iter()
            .map(|d| format!("`{}`: {}", mine.qualified_name(d.element), d.message))
            .collect();
        Ok(SavePlan {
            tree: mine,
            block: BlockRef::new(Scope::Mine, &saved),
            added,
            reused,
            replaced,
            problems,
        })
    }

    /// Saves a planned My Library change to its file.
    pub fn save(&mut self, plan: SavePlan) -> std::io::Result<()> {
        self.save_mine(plan.tree)
    }

    /// Removes a block from My Library (projects that copied it keep their
    /// copies). Refused while another block of My Library uses it.
    pub fn remove_mine(&mut self, qualified_name: &str) -> Result<(), String> {
        let mut mine = self.mine().clone();
        let element = mine
            .find(qualified_name)
            .ok_or_else(|| format!("My Library has no `{qualified_name}`"))?;
        let unit = unit_of(&mine, element);
        let inside: BTreeSet<ElementId> = mine.descendants(unit).into_iter().collect();
        let users: BTreeSet<String> = mine
            .references()
            .into_iter()
            .filter(|(holder, _, reference)| {
                !inside.contains(holder)
                    && reference
                        .steps
                        .iter()
                        .any(|s| s.target.is_some_and(|t| inside.contains(&t)))
            })
            .map(|(holder, _, _)| mine.qualified_name(unit_of(&mine, holder)))
            .collect();
        if !users.is_empty() {
            let listed: Vec<String> = users.into_iter().map(|u| format!("`{u}`")).collect();
            return Err(format!(
                "`{qualified_name}` is used by {}; remove {} first",
                listed.join(", "),
                if listed.len() == 1 { "it" } else { "them" }
            ));
        }
        let mut owner = mine[unit].owner();
        mine.remove(unit);
        // Packages left empty go too.
        while let Some(package) = owner {
            if mine[package].kind != ElementKind::Package
                || mine[package]
                    .children()
                    .iter()
                    .any(|c| !matches!(mine[*c].kind, ElementKind::Doc | ElementKind::Comment))
            {
                break;
            }
            owner = mine[package].owner();
            mine.remove(package);
        }
        self.save_mine(mine).map_err(|error| error.to_string())
    }
}

/// A planned change to My Library.
#[derive(Clone, Debug, PartialEq)]
pub struct SavePlan {
    /// My Library as it will be.
    pub tree: Tree,
    /// The saved block.
    pub block: BlockRef,
    /// Qualified names added, reused (already there, identical) and
    /// replaced (there with other content before).
    pub added: Vec<String>,
    pub reused: Vec<String>,
    pub replaced: Vec<String>,
    /// Problems My Library will have, in plain words.
    pub problems: Vec<String>,
}

/// Removes an element; references to it elsewhere keep the names they are
/// written with and are bound again by name (as the System State does).
fn remove_keeping_names(tree: &mut Tree, element: ElementId) {
    let removed: BTreeSet<ElementId> = tree.descendants(element).into_iter().collect();
    let mut rewrite = Vec::new();
    for holder in tree.walk() {
        if removed.contains(&holder) {
            continue;
        }
        for (index, (role, reference)) in tree[holder].references().into_iter().enumerate() {
            if reference
                .steps
                .iter()
                .any(|s| s.target.is_some_and(|t| removed.contains(&t)))
            {
                rewrite.push((
                    holder,
                    index,
                    role,
                    printed_reference(tree, holder, role, reference),
                ));
            }
        }
    }
    tree.remove(element);
    for (holder, index, _role, mut reference) in rewrite {
        for step in &mut reference.steps {
            step.target = None;
        }
        let element = tree.get_mut(holder).expect("a holder is not removed");
        let slot = references_mut(element)
            .nth(index)
            .expect("the order of Element::references");
        *slot = reference;
    }
}

fn references_mut(element: &mut Element) -> impl Iterator<Item = &mut Reference> {
    element
        .typed_by
        .iter_mut()
        .chain(&mut element.specializes)
        .chain(&mut element.redefines)
        .chain(&mut element.ends)
        .chain(&mut element.target)
        .chain(&mut element.by)
}

/// What an override sets.
#[derive(Clone, Debug, PartialEq)]
pub enum Override {
    /// A value for an attribute.
    Value(Literal),
    /// Another type for a part, port or item: a specialisation of its type.
    Type(ElementId),
}

impl Override {
    fn operations(&self, tree: &Tree, existing: ElementId) -> Vec<Operation> {
        match self {
            Override::Value(value) => vec![Operation::Set {
                element: existing,
                property: Property::Value(Some(value.clone())),
            }],
            Override::Type(definition) => vec![Operation::Set {
                element: existing,
                property: Property::TypedBy(vec![to(tree, *definition, "")]),
            }],
        }
    }
}

/// The member of `container` that redefines `feature` (or a redefinition
/// of it), if any.
fn redefinition_in(
    tree: &Tree,
    semantics: &Semantics,
    container: ElementId,
    feature: ElementId,
) -> Option<ElementId> {
    tree.get(container)?
        .children()
        .iter()
        .copied()
        .find(|child| {
            tree[*child].redefines.iter().any(|r| {
                r.target()
                    .is_some_and(|t| t == feature || semantics.specializes(t, feature))
            })
        })
}

/// Where the Operator's own definitions go near `element`: its nearest
/// package outside the copied `Library` package, else the first top-level
/// package that is not `Library`, else the top of the first document.
fn own_package(tree: &Tree, element: ElementId) -> Parent {
    let mut current = tree.get(element).and_then(Element::owner);
    let mut nearest = None;
    while let Some(id) = current {
        if tree[id].kind == ElementKind::Package && nearest.is_none() {
            nearest = Some(id);
        }
        current = tree[id].owner();
    }
    if let Some(package) = nearest
        && path_of(tree, package).first().map(String::as_str) != Some(ROOT)
    {
        return Parent::Element(package);
    }
    tree.roots()
        .find(|r| tree[*r].kind == ElementKind::Package && tree.effective_name(*r) != Some(ROOT))
        .map_or(Parent::Document(0), Parent::Element)
}

/// A boundary port of a block extracted from a selection: the port of an
/// inner part that a connection from outside reaches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Boundary {
    /// The new definition's port, named after the inner port.
    pub name: String,
    /// The inner part and its port.
    pub part: ElementId,
    pub port: ElementId,
    /// The connections from outside that will reach it through the port.
    pub connections: Vec<ElementId>,
}

/// What "Create building block from selection" would do, before it is done.
#[derive(Clone, Debug)]
pub struct Extraction {
    /// The element that owns the selected parts, and the parts.
    pub owner: ElementId,
    pub parts: Vec<ElementId>,
    /// Connections between the selected parts: moved inside.
    pub internal: Vec<ElementId>,
    /// Ports the new definition exposes.
    pub boundary: Vec<Boundary>,
    /// References elsewhere that reach a selected part through the owner
    /// (such as `satisfy R by system.store`): they go through the new usage.
    through: Vec<(ElementId, usize)>,
    /// Why it cannot be done as it stands, in plain words.
    pub blockers: Vec<String>,
    /// Where the new definition goes.
    pub package: Parent,
}

impl Extraction {
    /// Analyses a selection of parts that share an owner.
    pub fn analyse(state: &SystemState, selected: &[ElementId]) -> Result<Extraction, String> {
        let tree = state.tree();
        let mut parts: Vec<ElementId> = Vec::new();
        for id in selected {
            if tree.get(*id).map(|e| e.kind) == Some(ElementKind::Part) && !parts.contains(id) {
                parts.push(*id);
            }
        }
        if parts.is_empty() {
            return Err("select one or more parts to make a building block of".into());
        }
        let owner = tree[parts[0]]
            .owner()
            .ok_or("a part at the top level has no owner to hold the new block")?;
        if parts.iter().any(|p| tree[*p].owner() != Some(owner)) {
            return Err("select parts that share one owner".into());
        }
        let selected: BTreeSet<ElementId> = parts.iter().copied().collect();
        let moved: BTreeSet<ElementId> = parts.iter().flat_map(|p| tree.descendants(*p)).collect();
        let mut internal = Vec::new();
        let mut boundary: Vec<Boundary> = Vec::new();
        let mut blockers = Vec::new();
        let first_step = |r: &Reference| r.steps.first().and_then(|s| s.target);
        for child in tree[owner].children().iter().copied() {
            let element = &tree[child];
            if !matches!(
                element.kind,
                ElementKind::Connection | ElementKind::Interface
            ) || element.ends.len() != 2
            {
                continue;
            }
            let inside: Vec<bool> = element
                .ends
                .iter()
                .map(|end| first_step(end).is_some_and(|t| selected.contains(&t)))
                .collect();
            match (inside[0], inside[1]) {
                (true, true) => internal.push(child),
                (false, false) => {}
                _ => {
                    let end = &element.ends[if inside[0] { 0 } else { 1 }];
                    let targets: Vec<ElementId> =
                        end.steps.iter().filter_map(|s| s.target).collect();
                    let port = targets.get(1).copied().filter(|p| {
                        targets.len() == 2
                            && tree.get(*p).map(|e| e.kind) == Some(ElementKind::Port)
                    });
                    match port {
                        Some(port) => {
                            let part = targets[0];
                            match boundary.iter_mut().find(|b| b.part == part && b.port == port) {
                                Some(existing) => existing.connections.push(child),
                                None => boundary.push(Boundary {
                                    name: tree.effective_name(port).unwrap_or("port").to_string(),
                                    part,
                                    port,
                                    connections: vec![child],
                                }),
                            }
                        }
                        None => blockers.push(format!(
                            "the connection `{}` reaches `{end}`, which is not a port of a selected part; connect it through a port first",
                            crate::describe::display_name(tree, child)
                        )),
                    }
                }
            }
        }
        // Boundary ports are named after the inner ports; the same name
        // twice is told apart by the part's name.
        let names: Vec<String> = boundary.iter().map(|b| b.name.clone()).collect();
        let part_names: BTreeSet<&str> = parts
            .iter()
            .filter_map(|p| tree.effective_name(*p))
            .collect();
        for b in &mut boundary {
            if names.iter().filter(|n| **n == b.name).count() > 1
                || part_names.contains(b.name.as_str())
            {
                let part = tree.effective_name(b.part).unwrap_or("part");
                let mut port = b.name.chars();
                b.name = match port.next() {
                    Some(first) => format!("{part}{}{}", first.to_uppercase(), port.as_str()),
                    None => part.to_string(),
                };
            }
        }
        // Other references that reach a selected part.
        let mut through = Vec::new();
        let handled: BTreeSet<ElementId> = internal
            .iter()
            .chain(boundary.iter().flat_map(|b| &b.connections))
            .copied()
            .collect();
        for holder in tree.walk() {
            if moved.contains(&holder) || handled.contains(&holder) {
                continue;
            }
            for (index, (role, reference)) in tree[holder].references().into_iter().enumerate() {
                let Some(target) = reference
                    .steps
                    .iter()
                    .filter_map(|s| s.target)
                    .find(|t| selected.contains(t))
                else {
                    continue;
                };
                if matches!(role, Role::Redefines | Role::TypedBy | Role::Specializes) {
                    blockers.push(format!(
                        "`{}` refers to `{}` in a way that cannot go through the new block",
                        tree.qualified_name(holder),
                        tree.effective_name(target).unwrap_or("")
                    ));
                    continue;
                }
                through.push((holder, index));
            }
        }
        Ok(Extraction {
            owner,
            parts,
            internal,
            boundary,
            through,
            blockers,
            package: own_package(tree, owner),
        })
    }

    /// Plans the extraction as one change: a definition holding the parts,
    /// their connections and the boundary ports, passing items on to the
    /// inner ports; a usage of it where the parts were; connections from
    /// outside re-routed through its ports. Parts keep their identity.
    pub fn plan(
        &self,
        state: &SystemState,
        definition: &str,
        usage: &str,
        actor: Actor,
    ) -> Result<Plan, PlanError> {
        if let Some(blocker) = self.blockers.first() {
            return Err(invalid(blocker.clone()));
        }
        let tree = state.tree();
        let (definition, usage) = (definition.trim(), usage.trim());
        if definition.is_empty() || usage.is_empty() {
            return Err(invalid("the new definition and its usage need names"));
        }
        if find_path(tree, &{
            let mut path = match self.package {
                Parent::Element(p) => path_of(tree, p),
                Parent::Document(_) => Vec::new(),
            };
            path.push(definition.to_string());
            path
        })
        .is_some()
        {
            return Err(invalid(format!(
                "`{definition}` is already used there; choose another name"
            )));
        }
        let mut next = tree.next_id().raw();
        let mut take = || {
            let id = ElementId::from_raw(next);
            next += 1;
            id
        };
        let mut operations = Vec::new();
        let mut creates = 0;
        let def_id = take();
        creates += 1;
        operations.push(Operation::Create {
            parent: self.package,
            element: Box::new(Element::named(ElementKind::PartDef, definition)),
        });
        let semantics = Semantics::new(tree);
        let mut ports = Vec::new();
        for b in &self.boundary {
            let id = take();
            creates += 1;
            let mut port = Element::named(ElementKind::Port, &b.name);
            if let Some((ty, conjugated)) = semantics.types_of(b.port).first().copied() {
                port.typed_by = vec![to(tree, ty, "")];
                port.conjugated = conjugated;
            }
            operations.push(Operation::Create {
                parent: Parent::Element(def_id),
                element: Box::new(port),
            });
            ports.push(id);
        }
        for part in &self.parts {
            operations.push(Operation::Move {
                element: *part,
                parent: Parent::Element(def_id),
            });
        }
        for connection in &self.internal {
            operations.push(Operation::Move {
                element: *connection,
                parent: Parent::Element(def_id),
            });
        }
        let name = |id: ElementId| tree.effective_name(id).unwrap_or("").to_string();
        for (b, port) in self.boundary.iter().zip(&ports) {
            take();
            creates += 1;
            operations.push(Operation::Connect {
                parent: def_id,
                kind: ElementKind::Connection,
                name: None,
                definition: None,
                from: chain(&[(*port, b.name.clone())]),
                to: chain(&[(b.part, name(b.part)), (b.port, name(b.port))]),
            });
        }
        let usage_id = take();
        creates += 1;
        let mut usage_element = Element::named(ElementKind::Part, usage);
        usage_element.typed_by = vec![Reference::to(def_id, definition)];
        operations.push(Operation::Create {
            parent: Parent::Element(self.owner),
            element: Box::new(usage_element),
        });
        // Connections from outside now end at the new usage's ports.
        for (b, port) in self.boundary.iter().zip(&ports) {
            for connection in &b.connections {
                let ends: Vec<Reference> = tree[*connection]
                    .ends
                    .iter()
                    .map(|end| {
                        if end.steps.first().and_then(|s| s.target) == Some(b.part) {
                            chain(&[(usage_id, usage.to_string()), (*port, b.name.clone())])
                        } else {
                            end.clone()
                        }
                    })
                    .collect();
                operations.push(Operation::Set {
                    element: *connection,
                    property: Property::Ends(ends),
                });
            }
        }
        // Other references reach the parts through the new usage: one
        // change of each property of each holder.
        let selected: BTreeSet<ElementId> = self.parts.iter().copied().collect();
        let mut rewritten: Vec<(ElementId, Element)> = Vec::new();
        for (holder, index) in &self.through {
            let position = match rewritten.iter().position(|(h, _)| h == holder) {
                Some(position) => position,
                None => {
                    rewritten.push((*holder, tree[*holder].clone()));
                    rewritten.len() - 1
                }
            };
            let element = &mut rewritten[position].1;
            let reference = references_mut(element)
                .nth(*index)
                .expect("the order of Element::references");
            if let Some(at) = reference
                .steps
                .iter()
                .position(|s| s.target.is_some_and(|t| selected.contains(&t)))
            {
                reference.steps.insert(
                    at,
                    Step {
                        name: QualifiedName::new([usage]),
                        target: Some(usage_id),
                    },
                );
            }
        }
        for (holder, element) in rewritten {
            let old = &tree[holder];
            if element.ends != old.ends {
                operations.push(Operation::Set {
                    element: holder,
                    property: Property::Ends(element.ends.clone()),
                });
            }
            if element.by != old.by {
                operations.push(Operation::Set {
                    element: holder,
                    property: Property::By(element.by.clone()),
                });
            }
            if element.target != old.target {
                operations.push(Operation::Set {
                    element: holder,
                    property: Property::Target(element.target.clone()),
                });
            }
        }
        let names: Vec<String> = self.parts.iter().map(|p| name(*p)).collect();
        let description = format!(
            "Create building block {definition} from {}",
            names.join(", ")
        );
        let change = Change::new(actor, &description, operations).with_base(state.revision());
        let (created, problems) = trial(state, &change)?;
        predicted(tree, &created, creates)?;
        Ok(Plan {
            change,
            created: Some(usage_id),
            imported: Vec::new(),
            reused: Vec::new(),
            renamed: Vec::new(),
            connected: None,
            problems,
        })
    }
}
