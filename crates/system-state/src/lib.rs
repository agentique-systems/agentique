//! SystemState: the live, authoritative model of one project (REALIGNMENT §3.1).
//!
//! The Surface and the Assistant change the model through the same typed
//! [`Operation`]s, grouped into one atomic [`Change`] (§3.2). A change either
//! applies completely or is rejected with a [`Rejection`] and leaves the state
//! as it was:
//!
//! - it was prepared against an older revision ([`Rejection::Stale`]);
//! - it touches a locked element the Operator has not confirmed
//!   ([`Rejection::Locked`]);
//! - an operation cannot be carried out, such as renaming an element that
//!   does not exist ([`Rejection::Invalid`]).
//!
//! A change that is well formed but makes the model invalid (a type that does
//! not exist, ports that do not fit) is applied; the problems are reported as
//! [`diagnostics`](SystemState::diagnostics) at the elements concerned, and the
//! change can be fixed or undone (R-18).
//!
//! A lock on an element covers it and everything it owns (R-11). Every applied
//! change, undo and redo returns a [`ChangeEvent`] naming the elements that
//! were created, updated or deleted, so views can update only what changed.
#![forbid(unsafe_code)]

use agq_language::{
    Diagnostic, Direction, Element, ElementId, ElementKind, Literal, Multiplicity, Parent,
    Reference, Tree, TreeError, Visibility, link, validate,
};
use std::collections::BTreeSet;
use std::fmt;

/// Who made a change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    Operator,
    Assistant,
}

/// One typed change to the model. Elements are named by identity, never by
/// name, so a rename elsewhere in the same change cannot redirect an operation.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// Adds `element` as the last member of `parent`. Its id, owner and
    /// children are assigned by the tree; the new id is in
    /// [`ChangeEvent::created`] in operation order.
    Create {
        parent: Parent,
        element: Box<Element>,
    },
    /// Removes an element and everything it owns. References to it remain and
    /// are reported as problems until they are changed.
    Delete { element: ElementId },
    /// Gives an element a new name. References to it stay bound to it.
    Rename { element: ElementId, name: String },
    /// Makes `parent` the owner of an element (as its last member). The
    /// element keeps its identity and references to it stay bound.
    Move { element: ElementId, parent: Parent },
    /// Adds a connection or interface usage (`kind` is
    /// [`ElementKind::Connection`] or [`ElementKind::Interface`]) owned by
    /// `parent`, from one feature to another, optionally typed by a connection
    /// or interface definition.
    Connect {
        parent: ElementId,
        kind: ElementKind,
        name: Option<String>,
        definition: Option<Reference>,
        from: Reference,
        to: Reference,
    },
    /// Changes one property of an element.
    Set {
        element: ElementId,
        property: Property,
    },
    /// Locks an element: it and everything it owns change only with the
    /// Operator's confirmation.
    Lock { element: ElementId },
    /// Removes a lock. The Assistant needs the Operator's confirmation.
    Unlock { element: ElementId },
}

/// A property that [`Operation::Set`] can change. `None` or an empty list
/// clears it.
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    /// `: T`, the types of a usage.
    TypedBy(Vec<Reference>),
    /// `: ~P`, a port typed by the conjugate of its port definition.
    Conjugated(bool),
    /// `:>`, specialisation of a definition or subsetting of a usage.
    Specializes(Vec<Reference>),
    /// `:>>`, redefinition.
    Redefines(Vec<Reference>),
    Multiplicity(Option<Multiplicity>),
    Direction(Option<Direction>),
    /// `= value`
    Value(Option<Literal>),
    Abstract(bool),
    Visibility(Visibility),
    /// The two ends of a connection or interface usage.
    Ends(Vec<Reference>),
    /// The requirement a `satisfy` names, or an import's imported name.
    Target(Option<Reference>),
    /// `satisfy R by feature`.
    By(Option<Reference>),
    /// The element's documentation comment.
    Doc(Option<String>),
}

/// A group of operations applied together: one undo step.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub actor: Actor,
    /// What the change is for, in plain words; shown in history and undo.
    pub description: String,
    pub operations: Vec<Operation>,
    /// The revision the change was prepared against. `None` applies it to the
    /// current revision.
    pub base: Option<u64>,
    /// Locked elements (as reported by [`Rejection::Locked`]) that the
    /// Operator confirmed this change may touch.
    pub confirmed: Vec<ElementId>,
}

impl Change {
    pub fn new(actor: Actor, description: &str, operations: Vec<Operation>) -> Self {
        Change {
            actor,
            description: description.to_string(),
            operations,
            base: None,
            confirmed: Vec::new(),
        }
    }
}

/// Why a change was not applied. The state is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The change was prepared against an older revision.
    Stale { base: u64, current: u64 },
    /// The change touches these locked elements (the elements that carry the
    /// lock) without the Operator's confirmation.
    Locked { elements: Vec<ElementId> },
    /// Operation number `operation` (from 0) cannot be carried out.
    Invalid { operation: usize, reason: String },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::Stale { base, current } => write!(
                f,
                "the change was prepared against revision {base}, but the model is at revision {current}"
            ),
            Rejection::Locked { elements } => write!(
                f,
                "the change touches {} locked element(s) and needs the Operator's confirmation",
                elements.len()
            ),
            Rejection::Invalid { operation, reason } => {
                write!(f, "operation {} cannot be applied: {reason}", operation + 1)
            }
        }
    }
}

impl std::error::Error for Rejection {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Applied,
    Undone,
    Redone,
    /// The whole model was replaced, for example when a project is opened.
    Loaded,
}

/// What a change, undo, redo or load did to the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeEvent {
    /// The revision after the event.
    pub revision: u64,
    pub kind: EventKind,
    pub actor: Actor,
    pub description: String,
    /// New elements, in creation order.
    pub created: Vec<ElementId>,
    /// Elements whose properties, owner, members or lock changed.
    pub updated: Vec<ElementId>,
    pub deleted: Vec<ElementId>,
}

#[derive(Clone)]
struct Snapshot {
    tree: Tree,
    locks: BTreeSet<ElementId>,
    actor: Actor,
    description: String,
}

/// The live model of one project.
pub struct SystemState {
    tree: Tree,
    locks: BTreeSet<ElementId>,
    diagnostics: Vec<Diagnostic>,
    revision: u64,
    /// States before each applied change, most recent last.
    undo: Vec<Snapshot>,
    /// States after each undone change, most recent last.
    redo: Vec<Snapshot>,
}

impl SystemState {
    /// A System State holding `tree` (usually from [`agq_language::parse`])
    /// with the given locks, at revision 0.
    pub fn new(tree: Tree, locks: BTreeSet<ElementId>) -> Self {
        let diagnostics = validate(&tree);
        let locks = locks.into_iter().filter(|id| tree.contains(*id)).collect();
        SystemState {
            tree,
            locks,
            diagnostics,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Increases by one with every applied change, undo, redo and load.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Every problem in the current model, in document order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The problems reported at one element.
    pub fn diagnostics_for(&self, element: ElementId) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.element == element)
    }

    /// The elements that carry a lock.
    pub fn locks(&self) -> &BTreeSet<ElementId> {
        &self.locks
    }

    /// Whether an element is covered by a lock: its own or an owner's.
    pub fn is_locked(&self, element: ElementId) -> bool {
        self.lock_of(element).is_some()
    }

    /// The element carrying the lock that covers `element`, if any.
    pub fn lock_of(&self, element: ElementId) -> Option<ElementId> {
        let mut current = Some(element);
        while let Some(id) = current {
            if self.locks.contains(&id) {
                return Some(id);
            }
            current = self.tree.get(id).and_then(Element::owner);
        }
        None
    }

    /// The description of the change [`undo`](Self::undo) would revert.
    pub fn undo_description(&self) -> Option<&str> {
        self.undo.last().map(|step| step.description.as_str())
    }

    /// The description of the change [`redo`](Self::redo) would reapply.
    pub fn redo_description(&self) -> Option<&str> {
        self.redo.last().map(|step| step.description.as_str())
    }

    /// Applies a change atomically. See the crate documentation for when a
    /// change is rejected.
    pub fn apply(&mut self, change: Change) -> Result<ChangeEvent, Rejection> {
        if let Some(base) = change.base
            && base != self.revision
        {
            return Err(Rejection::Stale {
                base,
                current: self.revision,
            });
        }
        let locked = self.unconfirmed_locks(&change);
        if !locked.is_empty() {
            return Err(Rejection::Locked { elements: locked });
        }
        let mut tree = self.tree.clone();
        let mut locks = self.locks.clone();
        let mut created = Vec::new();
        for (index, operation) in change.operations.iter().enumerate() {
            let result = apply_operation(&mut tree, &mut locks, operation);
            match result {
                Ok(Some(id)) => created.push(id),
                Ok(None) => {}
                Err(reason) => {
                    return Err(Rejection::Invalid {
                        operation: index,
                        reason,
                    });
                }
            }
        }
        locks.retain(|id| tree.contains(*id));
        link(&mut tree);
        let before = Snapshot {
            tree: std::mem::replace(&mut self.tree, tree),
            locks: std::mem::replace(&mut self.locks, locks),
            actor: change.actor,
            description: change.description,
        };
        let mut event = self.finish(&before, EventKind::Applied);
        // Keep creation order for callers that need the new ids.
        event.created = created;
        self.undo.push(before);
        self.redo.clear();
        Ok(event)
    }

    /// Reverts the most recent applied change.
    pub fn undo(&mut self) -> Option<ChangeEvent> {
        let previous = self.undo.pop()?;
        let current = self.swap(previous.clone());
        let event = self.finish(&current, EventKind::Undone);
        self.redo.push(current);
        Some(event)
    }

    /// Reapplies the most recently undone change.
    pub fn redo(&mut self) -> Option<ChangeEvent> {
        let next = self.redo.pop()?;
        let current = self.swap(next.clone());
        let event = self.finish(&current, EventKind::Redone);
        self.undo.push(current);
        Some(event)
    }

    /// Replaces the whole model, for example when a project is opened or a
    /// branch is switched. Clears undo and redo.
    pub fn load(
        &mut self,
        tree: Tree,
        locks: BTreeSet<ElementId>,
        description: &str,
    ) -> ChangeEvent {
        let locks = locks.into_iter().filter(|id| tree.contains(*id)).collect();
        let previous = Snapshot {
            tree: std::mem::replace(&mut self.tree, tree),
            locks: std::mem::replace(&mut self.locks, locks),
            actor: Actor::Operator,
            description: description.to_string(),
        };
        self.undo.clear();
        self.redo.clear();
        self.finish(&previous, EventKind::Loaded)
    }

    /// Installs `snapshot` as the current state and returns the state it
    /// replaced, labelled with the snapshot's actor and description.
    fn swap(&mut self, snapshot: Snapshot) -> Snapshot {
        Snapshot {
            tree: std::mem::replace(&mut self.tree, snapshot.tree),
            locks: std::mem::replace(&mut self.locks, snapshot.locks),
            actor: snapshot.actor,
            description: snapshot.description,
        }
    }

    /// Revalidates, advances the revision and describes the difference from
    /// `before` to the current state.
    fn finish(&mut self, before: &Snapshot, kind: EventKind) -> ChangeEvent {
        self.diagnostics = validate(&self.tree);
        self.revision += 1;
        let Comparison {
            created,
            mut updated,
            deleted,
        } = compare(&before.tree, &self.tree);
        // A lock change is an update of the locked element.
        for id in before.locks.symmetric_difference(&self.locks) {
            if self.tree.contains(*id) && !created.contains(id) && !updated.contains(id) {
                updated.push(*id);
            }
        }
        ChangeEvent {
            revision: self.revision,
            kind,
            actor: before.actor,
            description: before.description.clone(),
            created,
            updated,
            deleted,
        }
    }

    /// The lock-carrying elements a change touches without confirmation.
    fn unconfirmed_locks(&self, change: &Change) -> Vec<ElementId> {
        let mut locked = BTreeSet::new();
        for operation in &change.operations {
            for (element, whole) in touched(&self.tree, operation, change.actor) {
                // Deleting or moving an element changes everything it owns.
                let covered = if whole {
                    self.tree.descendants(element)
                } else {
                    vec![element]
                };
                locked.extend(covered.into_iter().filter_map(|id| self.lock_of(id)));
            }
        }
        locked
            .into_iter()
            .filter(|lock| !change.confirmed.contains(lock))
            .collect()
    }
}

/// The existing elements an operation changes, each with whether the change
/// affects everything the element owns (delete and move) or only the element.
fn touched(tree: &Tree, operation: &Operation, actor: Actor) -> Vec<(ElementId, bool)> {
    let parent = |parent: &Parent| match parent {
        Parent::Element(id) => vec![(*id, false)],
        Parent::Document(_) => Vec::new(),
    };
    match operation {
        Operation::Create { parent: p, .. } => parent(p),
        Operation::Connect { parent: p, .. } => vec![(*p, false)],
        Operation::Rename { element, .. } | Operation::Set { element, .. } => {
            vec![(*element, false)]
        }
        Operation::Delete { element } => vec![(*element, true)],
        Operation::Move { element, parent: p } => {
            let mut ids = vec![(*element, true)];
            ids.extend(
                tree.get(*element)
                    .and_then(Element::owner)
                    .map(|owner| (owner, false)),
            );
            ids.extend(parent(p));
            ids
        }
        Operation::Lock { .. } => Vec::new(),
        Operation::Unlock { element } => match actor {
            Actor::Operator => Vec::new(),
            Actor::Assistant => vec![(*element, false)],
        },
    }
}

/// Carries out one operation; returns the id of a new element.
fn apply_operation(
    tree: &mut Tree,
    locks: &mut BTreeSet<ElementId>,
    operation: &Operation,
) -> Result<Option<ElementId>, String> {
    match operation {
        Operation::Create { parent, element } => {
            check_parent(tree, *parent)?;
            if let Some(name) = &element.name {
                check_name(name)?;
            }
            tree.add(*parent, (**element).clone())
                .map(Some)
                .map_err(describe)
        }
        Operation::Delete { element } => {
            existing(tree, *element)?;
            for id in tree.remove(*element) {
                locks.remove(&id);
            }
            Ok(None)
        }
        Operation::Rename { element, name } => {
            check_name(name)?;
            let target = existing_mut(tree, *element)?;
            if !target.kind.is_namespace() {
                return Err(format!("a {} has no name", target.kind.keyword()));
            }
            target.name = Some(name.clone());
            Ok(None)
        }
        Operation::Move { element, parent } => {
            existing(tree, *element)?;
            check_parent(tree, *parent)?;
            tree.move_to(*element, *parent, usize::MAX)
                .map(|()| None)
                .map_err(describe)
        }
        Operation::Connect {
            parent,
            kind,
            name,
            definition,
            from,
            to,
        } => {
            if !matches!(kind, ElementKind::Connection | ElementKind::Interface) {
                return Err(format!(
                    "a connection must be a connection or an interface, not a {}",
                    kind.keyword()
                ));
            }
            if let Some(name) = name {
                check_name(name)?;
            }
            check_parent(tree, Parent::Element(*parent))?;
            let mut element = Element::new(*kind);
            element.name = name.clone();
            element.typed_by = definition.iter().cloned().collect();
            element.ends = vec![from.clone(), to.clone()];
            tree.add(Parent::Element(*parent), element)
                .map(Some)
                .map_err(describe)
        }
        Operation::Set { element, property } => {
            set_property(tree, *element, property)?;
            Ok(None)
        }
        Operation::Lock { element } => {
            existing(tree, *element)?;
            locks.insert(*element);
            Ok(None)
        }
        Operation::Unlock { element } => {
            existing(tree, *element)?;
            if !locks.remove(element) {
                return Err(format!("{} is not locked", tree.qualified_name(*element)));
            }
            Ok(None)
        }
    }
}

fn set_property(tree: &mut Tree, element: ElementId, property: &Property) -> Result<(), String> {
    if let Property::Doc(text) = property {
        return set_doc(tree, element, text.as_deref());
    }
    let target = existing_mut(tree, element)?;
    match property.clone() {
        Property::TypedBy(value) => target.typed_by = value,
        Property::Conjugated(value) => target.conjugated = value,
        Property::Specializes(value) => target.specializes = value,
        Property::Redefines(value) => target.redefines = value,
        Property::Multiplicity(value) => target.multiplicity = value,
        Property::Direction(value) => target.direction = value,
        Property::Value(value) => target.value = value,
        Property::Abstract(value) => target.is_abstract = value,
        Property::Visibility(value) => target.visibility = value,
        Property::Ends(value) => target.ends = value,
        Property::Target(value) => target.target = value,
        Property::By(value) => target.by = value,
        Property::Doc(_) => unreachable!("handled above"),
    }
    Ok(())
}

/// Sets, replaces or removes the element's first `doc` comment.
fn set_doc(tree: &mut Tree, element: ElementId, text: Option<&str>) -> Result<(), String> {
    let owner = existing(tree, element)?;
    let doc = owner
        .children()
        .iter()
        .copied()
        .find(|id| tree[*id].kind == ElementKind::Doc);
    match (doc, text) {
        (Some(doc), Some(text)) => {
            tree.get_mut(doc).expect("a child exists").text = Some(text.to_string())
        }
        (Some(doc), None) => {
            tree.remove(doc);
        }
        (None, Some(text)) => {
            let mut new = Element::new(ElementKind::Doc);
            new.text = Some(text.to_string());
            tree.insert(Parent::Element(element), 0, new)
                .map_err(describe)?;
        }
        (None, None) => {}
    }
    Ok(())
}

fn existing(tree: &Tree, element: ElementId) -> Result<&Element, String> {
    tree.get(element)
        .ok_or_else(|| format!("element #{} does not exist", element.raw()))
}

fn existing_mut(tree: &mut Tree, element: ElementId) -> Result<&mut Element, String> {
    tree.get_mut(element)
        .ok_or_else(|| format!("element #{} does not exist", element.raw()))
}

/// A parent must be a document or an element that can own members.
fn check_parent(tree: &Tree, parent: Parent) -> Result<(), String> {
    match parent {
        Parent::Document(index) if index < tree.documents().len() => Ok(()),
        Parent::Document(index) => Err(format!("document {index} does not exist")),
        Parent::Element(id) => {
            let owner = existing(tree, id)?;
            if owner.kind.is_namespace() {
                Ok(())
            } else {
                Err(format!(
                    "{} is a {} and cannot own elements",
                    tree.qualified_name(id),
                    owner.kind.keyword()
                ))
            }
        }
    }
}

fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        Err("a name cannot be empty".to_string())
    } else {
        Ok(())
    }
}

fn describe(error: TreeError) -> String {
    match error {
        TreeError::NoSuchElement(id) | TreeError::BadId(id) => {
            format!("element #{} does not exist", id.raw())
        }
        TreeError::NoSuchDocument(index) => format!("document {index} does not exist"),
        TreeError::IntoItself => "an element cannot be moved into itself".to_string(),
    }
}

/// Which elements differ between two versions of a model, by identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    /// In `after` only, in document order.
    pub created: Vec<ElementId>,
    /// In both, with different properties, owner or members.
    pub updated: Vec<ElementId>,
    /// In `before` only, in document order.
    pub deleted: Vec<ElementId>,
}

/// Compares two versions of a model by element identity: the basis of change
/// events and of the "what changed" view between checkpoints.
pub fn compare(before: &Tree, after: &Tree) -> Comparison {
    let mut comparison = Comparison::default();
    for id in after.walk() {
        match before.get(id) {
            None => comparison.created.push(id),
            Some(old) if Some(old) != after.get(id) => comparison.updated.push(id),
            Some(_) => {}
        }
    }
    comparison.deleted = before
        .walk()
        .into_iter()
        .filter(|id| !after.contains(*id))
        .collect();
    comparison
}
