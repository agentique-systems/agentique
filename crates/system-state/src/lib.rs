//! SystemState: the live, authoritative model of one project (REALIGNMENT §3.1).
//!
//! The Surface and the Assistant change the model through the same typed
//! [`Operation`]s, grouped into one atomic [`Change`] (§3.2). A change either
//! applies completely or is rejected with a [`Rejection`] and leaves the state
//! (model and locks) as it was:
//!
//! - it was prepared against an older revision ([`Rejection::Stale`]);
//! - it touches a locked element the Operator has not confirmed
//!   ([`Rejection::Locked`]);
//! - an operation cannot be carried out ([`Rejection::Invalid`]): the element
//!   does not exist, the element cannot have the property (a part def has no
//!   type), or an element it writes could not be saved as SysML text and read
//!   back the same (an empty name, a name with a line break, a string value
//!   with an unescaped quote, a connection with three ends), as
//!   [`agq_language::writable`] decides.
//!
//! A change that is well formed but makes the model invalid (a type that does
//! not exist, ports that do not fit, a part inside an attribute, a duplicate
//! name) is applied; the problems are reported as
//! [`diagnostics`](SystemState::diagnostics) at the elements concerned, and the
//! change can be fixed or undone (R-18).
//!
//! A lock on an element covers it and everything it owns (R-11). Every applied
//! change, undo and redo returns a [`ChangeEvent`] naming the elements that
//! were created, updated or deleted, so views can update only what changed.
//!
//! A [`Project`] keeps a System State saved in a project folder, with
//! checkpoints and branches in git (R-6).
#![forbid(unsafe_code)]

mod project;

pub use project::{ApplyError, Checkpoint, HistoryError, Project, ProjectError};

use agq_language::{
    Diagnostic, Direction, Element, ElementId, ElementKind, Field, Literal, Multiplicity, Parent,
    QualifiedName, Reference, Step, Tree, TreeError, Visibility, link, printed_reference, validate,
    writable,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
    /// [`ChangeEvent::created`] in operation order. Packages and definitions
    /// need a name; unsupported text and syntax errors cannot be created.
    Create {
        parent: Parent,
        element: Box<Element>,
    },
    /// Removes an element and everything it owns. References to it keep the
    /// name they are saved with (its current name) and are linked again by
    /// that name, as when the saved text is read back; if nothing has the
    /// name they are reported as problems until they are changed. Deleting an
    /// element that an earlier operation of the same change removed does
    /// nothing.
    Delete { element: ElementId },
    /// Gives an element a new name. References to it stay bound to it. A name
    /// that is already taken is applied and reported as a duplicate.
    Rename { element: ElementId, name: String },
    /// Makes `parent` the owner of an element (as its last member), within a
    /// document or into another one. The element keeps its identity and
    /// references to it stay bound.
    Move { element: ElementId, parent: Parent },
    /// Adds a connection or interface usage (`kind` is
    /// [`ElementKind::Connection`] or [`ElementKind::Interface`]) owned by
    /// `parent`, from one feature to another, optionally typed by a connection
    /// or interface definition. The ends are feature chains (`store.orders`)
    /// resolved from `parent`; ends that do not exist or do not fit are
    /// reported as problems at the new connection.
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
/// clears it. Setting a property the element's kind does not have is
/// rejected; see [`Property::applies_to`].
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    /// `: T`, the types of a usage.
    TypedBy(Vec<Reference>),
    /// `: ~P`, a port typed by the conjugate of its port definition. A
    /// conjugated usage has exactly one type.
    Conjugated(bool),
    /// `:>`, specialisation of a definition or subsetting of a usage.
    Specializes(Vec<Reference>),
    /// `:>>`, redefinition.
    Redefines(Vec<Reference>),
    Multiplicity(Option<Multiplicity>),
    Direction(Option<Direction>),
    /// `= value`. Numbers are written as in SysML text (`-3`, `1.5e3`), and
    /// a string's quotes and backslashes inside it are escaped (`\"`, `\\`).
    Value(Option<Literal>),
    Abstract(bool),
    Visibility(Visibility),
    /// The two ends of a connection or interface usage (or none).
    Ends(Vec<Reference>),
    /// The requirement a `satisfy` names, or an import's imported name.
    Target(Option<Reference>),
    /// `satisfy R by feature`.
    By(Option<Reference>),
    /// The element's documentation comment.
    Doc(Option<String>),
}

impl Property {
    /// The part of the text form this property is; a doc comment is a
    /// member of the element.
    pub fn field(&self) -> Field {
        match self {
            Property::TypedBy(_) => Field::TypedBy,
            Property::Conjugated(_) => Field::Conjugated,
            Property::Specializes(_) => Field::Specializes,
            Property::Redefines(_) => Field::Redefines,
            Property::Multiplicity(_) => Field::Multiplicity,
            Property::Direction(_) => Field::Direction,
            Property::Value(_) => Field::Value,
            Property::Abstract(_) => Field::Abstract,
            Property::Visibility(_) => Field::Visibility,
            Property::Ends(_) => Field::Ends,
            Property::Target(_) => Field::Target,
            Property::By(_) => Field::By,
            Property::Doc(_) => Field::Members,
        }
    }

    /// Whether elements of `kind` have this property in SysML text.
    pub fn applies_to(&self, kind: ElementKind) -> bool {
        self.field().fits(kind)
    }

    /// The property's name in messages.
    fn label(&self) -> &'static str {
        match self {
            Property::Doc(_) => "doc comment",
            other => other.field().label(),
        }
    }
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

    /// The same change, prepared against `revision`: it is rejected as
    /// [`Rejection::Stale`] if the model has changed since.
    pub fn with_base(mut self, revision: u64) -> Self {
        self.base = Some(revision);
        self
    }
}

/// Why a change was not applied. The state is unchanged.
/// [`SystemState::explain`] describes it with the elements' names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The change was prepared against an older revision.
    Stale { base: u64, current: u64 },
    /// The change touches these locked elements (the elements that carry the
    /// lock) without the Operator's confirmation.
    Locked { elements: Vec<ElementId> },
    /// Operation number `operation` (from 0) cannot be carried out; `reason`
    /// says why in plain language, naming elements by qualified name.
    Invalid { operation: usize, reason: String },
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::Stale { base, current } => write!(
                f,
                "the model changed while this change was prepared (revision {base}, now {current}); prepare it again"
            ),
            Rejection::Locked { elements } => write!(
                f,
                "the change touches {} locked element(s) and needs the Operator's confirmation",
                elements.len()
            ),
            Rejection::Invalid { operation, reason } => {
                write!(f, "{reason} (operation {})", operation + 1)
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
    /// New elements: for an applied change, those its operations created in
    /// operation order, then any others (such as a doc comment set as a
    /// property); otherwise in document order.
    pub created: Vec<ElementId>,
    /// Elements whose properties, owner, members, document or lock changed.
    pub updated: Vec<ElementId>,
    pub deleted: Vec<ElementId>,
}

#[derive(Clone)]
struct Snapshot {
    tree: Tree,
    locks: BTreeSet<ElementId>,
    actor: Actor,
    description: String,
    /// On the undo stack: the revision the change was applied or redone at.
    revision: u64,
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
        lock_of(&self.tree, &self.locks, element)
    }

    /// The description of the change [`undo`](Self::undo) would revert.
    pub fn undo_description(&self) -> Option<&str> {
        self.undo.last().map(|step| step.description.as_str())
    }

    /// The description of the change [`redo`](Self::redo) would reapply.
    pub fn redo_description(&self) -> Option<&str> {
        self.redo.last().map(|step| step.description.as_str())
    }

    /// A rejection in plain language for the Operator, naming locked
    /// elements by qualified name.
    pub fn explain(&self, rejection: &Rejection) -> String {
        let Rejection::Locked { elements } = rejection else {
            return rejection.to_string();
        };
        let names: Vec<String> = elements
            .iter()
            .map(|id| format!("`{}`", self.tree.qualified_name(*id)))
            .collect();
        match names.as_slice() {
            [one] => format!("{one} is locked; changing it needs the Operator's confirmation"),
            _ => format!(
                "{} are locked; changing them needs the Operator's confirmation",
                names.join(", ")
            ),
        }
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
        let mut edit = Edit {
            before: &self.tree,
            tree: self.tree.clone(),
            locks: self.locks.clone(),
            removed: BTreeSet::new(),
            written: BTreeMap::new(),
        };
        let mut created = Vec::new();
        for (index, operation) in change.operations.iter().enumerate() {
            match edit.apply(index, operation) {
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
        edit.check_written()
            .map_err(|(operation, reason)| Rejection::Invalid { operation, reason })?;
        let Edit {
            mut tree,
            mut locks,
            ..
        } = edit;
        locks.retain(|id| tree.contains(*id));
        link(&mut tree);
        let mut before = Snapshot {
            tree: std::mem::replace(&mut self.tree, tree),
            locks: std::mem::replace(&mut self.locks, locks),
            actor: change.actor,
            description: change.description,
            revision: 0,
        };
        let mut event = self.finish(&before, EventKind::Applied);
        // Operation order first, for callers that need the new ids.
        let mut ordered: Vec<ElementId> = created
            .into_iter()
            .filter(|id| self.tree.contains(*id))
            .collect();
        for id in event.created {
            if !ordered.contains(&id) {
                ordered.push(id);
            }
        }
        event.created = ordered;
        before.revision = self.revision;
        self.undo.push(before);
        self.redo.clear();
        Ok(event)
    }

    /// Reverts the most recent applied change.
    pub fn undo(&mut self) -> Option<ChangeEvent> {
        let previous = self.undo.pop()?;
        let current = self.swap(previous);
        let event = self.finish(&current, EventKind::Undone);
        self.redo.push(current);
        Some(event)
    }

    /// Reapplies the most recently undone change.
    pub fn redo(&mut self) -> Option<ChangeEvent> {
        let next = self.redo.pop()?;
        let mut current = self.swap(next);
        let event = self.finish(&current, EventKind::Redone);
        current.revision = self.revision;
        self.undo.push(current);
        Some(event)
    }

    /// Undoes, most recent first, every change applied or redone after
    /// `revision`, whoever made it: the Assistant's work since it started
    /// (R-12), but also any change or redo the Operator made in between.
    /// Each is one undo step and can be redone. Returns the events in order.
    pub fn undo_since(&mut self, revision: u64) -> Vec<ChangeEvent> {
        let mut events = Vec::new();
        while self
            .undo
            .last()
            .is_some_and(|step| step.revision > revision)
        {
            events.extend(self.undo());
        }
        events
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
            revision: 0,
        };
        self.undo.clear();
        self.redo.clear();
        self.finish(&previous, EventKind::Loaded)
    }

    /// Installs `snapshot` as the current state and returns the state it
    /// replaced, labelled with the snapshot's actor and description.
    fn swap(&mut self, snapshot: Snapshot) -> Snapshot {
        let mut tree = snapshot.tree;
        // Ids handed out before the undo are not handed out again.
        tree.reserve_ids(self.tree.next_id());
        Snapshot {
            tree: std::mem::replace(&mut self.tree, tree),
            locks: std::mem::replace(&mut self.locks, snapshot.locks),
            actor: snapshot.actor,
            description: snapshot.description,
            revision: snapshot.revision,
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
        Operation::Rename { element, .. } => vec![(*element, false)],
        Operation::Set { element, property } => {
            let mut ids = vec![(*element, false)];
            // The doc comment is a member that can carry a lock of its own.
            if let Property::Doc(_) = property {
                ids.extend(doc_of(tree, *element).map(|doc| (doc, false)));
            }
            ids
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

/// A change being carried out on copies of the model and locks.
struct Edit<'a> {
    /// The model before the change, to name elements removed by it.
    before: &'a Tree,
    tree: Tree,
    locks: BTreeSet<ElementId>,
    /// Elements removed by earlier operations of the change.
    removed: BTreeSet<ElementId>,
    /// Elements created, renamed or given properties, with the last
    /// operation that did so: checked with [`writable`] once all operations
    /// are done, so that the order of operations does not matter.
    written: BTreeMap<ElementId, usize>,
}

impl Edit<'_> {
    /// Carries out one operation; returns the id of a new element.
    fn apply(&mut self, index: usize, operation: &Operation) -> Result<Option<ElementId>, String> {
        match operation {
            Operation::Create { parent, element } => {
                self.check_parent(*parent)?;
                if matches!(
                    element.kind,
                    ElementKind::Unsupported | ElementKind::SyntaxError
                ) {
                    return Err(format!(
                        "{} cannot be created; only elements of the supported SysML subset can",
                        a(element.kind)
                    ));
                }
                let mut element = (**element).clone();
                element.location = None;
                let id = self.tree.add(*parent, element).map_err(describe)?;
                self.written.insert(id, index);
                Ok(Some(id))
            }
            Operation::Delete { element } => {
                if self.removed.contains(element) {
                    return Ok(None);
                }
                self.existing(*element)?;
                self.remove(*element);
                Ok(None)
            }
            Operation::Rename { element, name } => {
                self.existing(*element)?;
                self.tree.get_mut(*element).expect("checked above").name = Some(name.clone());
                self.written.insert(*element, index);
                Ok(None)
            }
            Operation::Move { element, parent } => {
                self.existing(*element)?;
                self.check_parent(*parent)?;
                self.tree
                    .move_to(*element, *parent, usize::MAX)
                    .map_err(|error| match (error, parent) {
                        (TreeError::IntoItself, Parent::Element(target)) => format!(
                            "{} cannot be moved into itself or into {}, which it owns",
                            self.name(*element),
                            self.name(*target)
                        ),
                        (error, _) => describe(error),
                    })?;
                Ok(None)
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
                        "a connection is a connection or an interface, not {}",
                        a(*kind)
                    ));
                }
                self.check_parent(Parent::Element(*parent))?;
                let mut element = Element::new(*kind);
                element.name = name.clone();
                element.typed_by = definition.iter().cloned().collect();
                element.ends = vec![from.clone(), to.clone()];
                let id = self
                    .tree
                    .add(Parent::Element(*parent), element)
                    .map_err(describe)?;
                self.written.insert(id, index);
                Ok(Some(id))
            }
            Operation::Set { element, property } => {
                let kind = self.existing(*element)?.kind;
                if !property.applies_to(kind) {
                    return Err(format!(
                        "{}: {} has no {}",
                        self.name(*element),
                        a(kind),
                        property.label()
                    ));
                }
                self.set(*element, property, index);
                Ok(None)
            }
            Operation::Lock { element } => {
                self.existing(*element)?;
                self.locks.insert(*element);
                Ok(None)
            }
            Operation::Unlock { element } => {
                self.existing(*element)?;
                match lock_of(&self.tree, &self.locks, *element) {
                    Some(lock) if lock == *element => {
                        self.locks.remove(element);
                        Ok(None)
                    }
                    Some(lock) => Err(format!(
                        "{} has no lock of its own; it is covered by the lock on {}",
                        self.name(*element),
                        self.name(lock)
                    )),
                    None => Err(format!("{} is not locked", self.name(*element))),
                }
            }
        }
    }

    /// Changes one property; its applicability is checked by the caller.
    fn set(&mut self, element: ElementId, property: &Property, index: usize) {
        self.written.insert(element, index);
        if let Property::Doc(text) = property {
            return self.set_doc(element, text.as_deref(), index);
        }
        let target = self.tree.get_mut(element).expect("checked by the caller");
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
    }

    /// Sets, replaces or removes the element's first `doc` comment.
    fn set_doc(&mut self, element: ElementId, text: Option<&str>, index: usize) {
        match (doc_of(&self.tree, element), text) {
            (Some(doc), Some(text)) => {
                self.tree.get_mut(doc).expect("a child exists").text = Some(text.to_string());
                self.written.insert(doc, index);
            }
            (Some(doc), None) => self.remove(doc),
            (None, Some(text)) => {
                let mut new = Element::new(ElementKind::Doc);
                new.text = Some(text.to_string());
                let doc = self
                    .tree
                    .insert(Parent::Element(element), 0, new)
                    .expect("the element exists");
                self.written.insert(doc, index);
            }
            (None, None) => {}
        }
    }

    /// Removes an element and everything it owns, with their locks.
    fn remove(&mut self, element: ElementId) {
        let removed = delete(&mut self.tree, &mut self.locks, element);
        self.removed.extend(removed);
    }

    /// Every written element must print and read back as it is.
    fn check_written(&self) -> Result<(), (usize, String)> {
        let mut written: Vec<(usize, ElementId)> = self
            .written
            .iter()
            .map(|(id, index)| (*index, *id))
            .collect();
        written.sort();
        for (index, id) in written {
            if self.tree.contains(id) {
                writable(&self.tree, id)
                    .map_err(|reason| (index, format!("{}: {reason}", self.name(id))))?;
            }
        }
        Ok(())
    }

    /// The element, or why it cannot be changed.
    fn existing(&self, id: ElementId) -> Result<&Element, String> {
        self.tree.get(id).ok_or_else(|| {
            if self.before.contains(id) {
                format!(
                    "`{}` was removed by an earlier operation of this change",
                    self.before.qualified_name(id)
                )
            } else {
                format!("element {id} does not exist")
            }
        })
    }

    /// A parent is a document or an element that can own members.
    fn check_parent(&self, parent: Parent) -> Result<(), String> {
        match parent {
            Parent::Document(index) if index < self.tree.documents().len() => Ok(()),
            Parent::Document(index) => Err(format!("document {index} does not exist")),
            Parent::Element(id) => {
                let kind = self.existing(id)?.kind;
                if Field::Members.fits(kind) {
                    Ok(())
                } else {
                    Err(format!(
                        "{} is {} and cannot own elements",
                        self.name(id),
                        a(kind)
                    ))
                }
            }
        }
    }

    /// An element's qualified name in backticks, for messages.
    fn name(&self, id: ElementId) -> String {
        format!("`{}`", self.tree.qualified_name(id))
    }
}

/// The element carrying the lock that covers `element`: its own or that of
/// an owner.
fn lock_of(tree: &Tree, locks: &BTreeSet<ElementId>, element: ElementId) -> Option<ElementId> {
    let mut current = Some(element);
    while let Some(id) = current {
        if locks.contains(&id) {
            return Some(id);
        }
        current = tree.get(id).and_then(Element::owner);
    }
    None
}

/// The element's first `doc` comment, which `Property::Doc` changes.
fn doc_of(tree: &Tree, element: ElementId) -> Option<ElementId> {
    tree.get(element)?
        .children()
        .iter()
        .copied()
        .find(|id| tree[*id].kind == ElementKind::Doc)
}

/// `a part`, `an item def`: a kind with its article, for messages.
fn a(kind: ElementKind) -> String {
    let keyword = kind.keyword();
    let article = if keyword.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {keyword}")
}

fn describe(error: TreeError) -> String {
    match error {
        TreeError::NoSuchElement(id) | TreeError::BadId(id) => {
            format!("element {id} does not exist")
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
    /// In both, with different properties, owner or members, or (for
    /// top-level elements) a different document or place in it: renamed and
    /// moved elements are updated.
    pub updated: Vec<ElementId>,
    /// In `before` only, in document order.
    pub deleted: Vec<ElementId>,
}

/// Compares two versions of a model by element identity: the basis of change
/// events and of the "what changed" view between checkpoints.
///
/// It compares what elements mean, so a model and the same model saved and
/// read back compare equal: where an element was read from does not count,
/// and a linked reference is its target, whatever name it was written with.
pub fn compare(before: &Tree, after: &Tree) -> Comparison {
    let moved = moved_roots(before, after);
    let mut comparison = Comparison::default();
    for id in after.walk() {
        match before.get(id) {
            None => comparison.created.push(id),
            Some(old) if !same(old, &after[id]) || moved.contains(&id) => {
                comparison.updated.push(id)
            }
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

/// Top-level elements that moved: to another document, or to another place
/// in their document. A move among an element's members changes the owner's
/// members instead, and the owner is updated.
fn moved_roots(before: &Tree, after: &Tree) -> HashSet<ElementId> {
    let mut moved = HashSet::new();
    for (index, document) in after.documents().iter().enumerate() {
        let old = before
            .documents()
            .get(index)
            .map_or(&[][..], |d| d.members());
        let kept: HashSet<ElementId> = old.iter().copied().collect();
        let new: Vec<ElementId> = document.members().to_vec();
        // New here: from another document (or from inside an element, whose
        // owner then changed anyway).
        moved.extend(
            new.iter()
                .filter(|id| !kept.contains(*id) && before.contains(**id)),
        );
        let stayed: HashSet<ElementId> = new.iter().copied().collect();
        let old: Vec<ElementId> = old
            .iter()
            .copied()
            .filter(|id| stayed.contains(id))
            .collect();
        let new: Vec<ElementId> = new.into_iter().filter(|id| kept.contains(id)).collect();
        moved.extend(reordered(&old, &new));
    }
    moved
}

/// The elements of `new` (a reordering of `old`) that changed place: those
/// outside a longest run of elements that kept their order.
fn reordered(old: &[ElementId], new: &[ElementId]) -> Vec<ElementId> {
    let position: HashMap<ElementId, usize> =
        old.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let order: Vec<usize> = new.iter().map(|id| position[id]).collect();
    // A longest increasing run of old positions, by patience sorting:
    // `tails[n]` ends the best run of length n + 1 found so far.
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![None; order.len()];
    for (i, &value) in order.iter().enumerate() {
        let length = tails.partition_point(|&t| order[t] < value);
        previous[i] = length.checked_sub(1).map(|l| tails[l]);
        if length == tails.len() {
            tails.push(i);
        } else {
            tails[length] = i;
        }
    }
    let mut kept = vec![false; order.len()];
    let mut next = tails.last().copied();
    while let Some(i) = next {
        kept[i] = true;
        next = previous[i];
    }
    new.iter()
        .zip(kept)
        .filter(|(_, kept)| !kept)
        .map(|(id, _)| *id)
        .collect()
}

fn same(a: &Element, b: &Element) -> bool {
    a == b || meaning(a) == meaning(b)
}

/// An element without its source location and without the written names of
/// linked reference steps.
fn meaning(element: &Element) -> Element {
    let mut element = element.clone();
    element.location = None;
    for reference in references_mut(&mut element) {
        for step in &mut reference.steps {
            if step.target.is_some() {
                step.name = QualifiedName::default();
            }
        }
    }
    element
}

/// Removes an element and everything it owns; returns the removed ids.
/// References to them are unlinked and keep the names they are saved with
/// at this moment (the targets' current names), so linking binds them again
/// by those names, exactly as reading the saved text back does.
fn delete(tree: &mut Tree, locks: &mut BTreeSet<ElementId>, element: ElementId) -> Vec<ElementId> {
    let removed: BTreeSet<ElementId> = tree.descendants(element).into_iter().collect();
    let mut written = Vec::new();
    for holder in tree.walk() {
        if removed.contains(&holder) {
            continue;
        }
        for (index, (role, reference)) in tree[holder].references().into_iter().enumerate() {
            let points_at_removed = |step: &Step| step.target.is_some_and(|t| removed.contains(&t));
            if reference.steps.iter().any(points_at_removed) {
                let printed = printed_reference(tree, holder, role, reference);
                written.push((holder, index, printed));
            }
        }
    }
    let removed = tree.remove(element);
    for id in &removed {
        locks.remove(id);
    }
    for (holder, index, mut reference) in written {
        reference
            .steps
            .iter_mut()
            .for_each(|step| step.target = None);
        let holder = tree.get_mut(holder).expect("a holder is not removed");
        *references_mut(holder)
            .nth(index)
            .expect("the order of Element::references") = reference;
    }
    removed
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
