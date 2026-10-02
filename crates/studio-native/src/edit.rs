//! Model edits by the Operator. Every edit is a typed System State change
//! applied by the project, which saves it; nothing here changes the tree or
//! writes files. A change to a locked element asks for confirmation first.
use crate::{commands::CommandId, studio::Studio};
use agq_language::{Element, ElementId, ElementKind, Parent, QualifiedName, Reference, Step, Tree};
use agq_studio_scene::SceneTarget;
use agq_system_state::{
    Actor, ApplyError, Change, ChangeEvent, Operation, ProjectError, Property, Rejection,
};

/// What the Operator can create by hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateKind {
    Part,
    Port,
    Item,
    Attribute,
    Interface,
    Requirement,
}
impl CreateKind {
    pub fn element_kind(self, definition: bool) -> ElementKind {
        use ElementKind::*;
        match (self, definition) {
            (Self::Part, false) => Part,
            (Self::Part, true) => PartDef,
            (Self::Port, false) => Port,
            (Self::Port, true) => PortDef,
            (Self::Item, false) => Item,
            (Self::Item, true) => ItemDef,
            (Self::Attribute, false) => Attribute,
            (Self::Attribute, true) => AttributeDef,
            (Self::Interface, _) => InterfaceDef,
            (Self::Requirement, false) => Requirement,
            (Self::Requirement, true) => RequirementDef,
        }
    }
    fn can_be_usage(self) -> bool {
        self != Self::Interface
    }
}

/// A dialog waiting for the Operator.
pub enum Dialog {
    NewProject {
        folder: String,
        name: String,
        /// Start from a sample: the URL shortener (R-46), or with AI
        /// screening (C-50).
        sample: Option<crate::studio::Sample>,
    },
    OpenProject {
        folder: String,
    },
    Create {
        kind: CreateKind,
        definition: bool,
        name: String,
        parent: Parent,
    },
    /// Shown in place, over the element on the Surface.
    Rename {
        element: ElementId,
        name: String,
    },
    Checkpoint {
        message: String,
    },
    MoveTo {
        element: ElementId,
        query: String,
    },
    /// A change that needs the Operator's confirmation: it touches locked
    /// elements, or it changes a definition shared by other elements.
    Confirm {
        change: Change,
        question: String,
        /// The locked elements to confirm, as reported by the System State.
        locked: Vec<ElementId>,
        /// The revision when the question was asked; the change is applied
        /// only if the model has not changed since.
        base: Option<u64>,
        /// The id the model would give its next new element when the
        /// question was asked: a change that refers to elements it creates
        /// itself cannot be applied to a model that changed since.
        next: Option<ElementId>,
        /// A shared definition and the usage it is changed from: the
        /// Operator may specialise instead.
        shared: Option<(ElementId, ElementId)>,
    },
    /// "Specialise…": the name of a new definition that specialises
    /// `definition`; `usage`, if any, is typed by it instead.
    Specialize {
        definition: ElementId,
        usage: Option<ElementId>,
        name: String,
    },
    /// "Create building block from selection…": the proposed block, shown
    /// before anything changes, and the names of the new definition and
    /// its usage.
    ExtractBlock {
        extraction: Box<agq_library::Extraction>,
        definition: String,
        usage: String,
    },
    /// "Save to My Library…": the category; conflicts found by an earlier
    /// try, which confirming again replaces in My Library.
    SaveToLibrary {
        definition: ElementId,
        category: String,
        conflicts: Vec<agq_library::Conflict>,
    },
    /// "Override here": a value for an inherited attribute, in `owner` only.
    Override {
        owner: ElementId,
        path: Vec<ElementId>,
        value: String,
    },
    /// Using a block met definitions of the same names with other content:
    /// use the project's, or copy the block's under another name.
    LibraryConflict {
        request: agq_library::Use,
        conflicts: Vec<agq_library::Conflict>,
    },
    /// "New scenario…": its name; the subject is the selected definition.
    NewScenario {
        subject: ElementId,
        name: String,
    },
    /// Trusted-local execution for this project: what it allows, on or off.
    TrustLocal,
    /// A live evaluation: the provider, the model, the samples and what it
    /// may cost, before anything is sent.
    ConfirmLive,
    /// "Implement with the Assistant…": what the worker is asked, and what
    /// it may do, before a task starts.
    Implement {
        element: ElementId,
        instructions: String,
    },
    /// A finished task's patch, checks and proposals, to integrate or
    /// discard; `problem` says why integrating failed.
    ReviewTask {
        job: String,
        problem: Option<String>,
        /// The patch as text, read when the review opened.
        diff: String,
        /// The Operator chose to integrate although the verification is not
        /// current and passed, or the patch changes the code of locked
        /// parts; the dialog then names what and asks once more.
        confirmed: bool,
    },
    /// "Link code…": a file, and optionally a symbol in it, for an element;
    /// `kind` indexes `LinkKind::ALL`.
    LinkCode {
        element: ElementId,
        kind: usize,
        path: String,
        symbol: String,
    },
}
/// What became of a change given to [`Studio::apply_change`].
pub enum Outcome {
    Applied(ChangeEvent),
    /// The Operator is asked to confirm a change to locked elements.
    Asking,
    /// Not applied: rejected, refused by the Operator, or not saved.
    NotApplied(ApplyError),
    NoProject,
}

impl Dialog {
    pub fn new_project() -> Self {
        let folder = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(|home| {
                std::path::PathBuf::from(home)
                    .join("Agentique")
                    .join("NewSystem")
                    .display()
                    .to_string()
            })
            .unwrap_or_default();
        Dialog::NewProject {
            folder,
            name: "NewSystem".into(),
            sample: None,
        }
    }

    /// The New project dialog for the URL shortener sample (R-46).
    pub fn sample_project() -> Self {
        let Dialog::NewProject { folder, .. } = Dialog::new_project() else {
            unreachable!("new_project is the New project dialog")
        };
        let folder = std::path::Path::new(&folder)
            .with_file_name(crate::studio::SAMPLE_NAME)
            .display()
            .to_string();
        Dialog::NewProject {
            folder,
            name: crate::studio::SAMPLE_NAME.into(),
            sample: Some(crate::studio::Sample::UrlShortener),
        }
    }
}

/// A reference to `path` (from the connection's owner down to the element),
/// linked at every step.
pub fn chain(tree: &Tree, path: &[ElementId]) -> Reference {
    Reference {
        steps: path
            .iter()
            .map(|id| Step {
                name: QualifiedName::new([tree.effective_name(*id).unwrap_or("")]),
                target: Some(*id),
            })
            .collect(),
    }
}

/// `id`, its owner, its owner's owner, …
fn ancestors(tree: &Tree, id: ElementId) -> Vec<ElementId> {
    let mut out = vec![id];
    let mut current = id;
    while let Some(owner) = tree.get(current).and_then(Element::owner) {
        out.push(owner);
        current = owner;
    }
    out
}

/// Where a connection between two ends goes, and the two end references.
/// Each end is a card and optionally a port shown on it.
pub fn connection(
    tree: &Tree,
    a: (ElementId, Option<ElementId>),
    b: (ElementId, Option<ElementId>),
) -> Result<(ElementId, Reference, Reference), String> {
    let up_b = ancestors(tree, b.0);
    let common = ancestors(tree, a.0)
        .into_iter()
        .find(|id| up_b.contains(id))
        .ok_or("these elements have no common owner to hold the connection")?;
    let end = |(card, port): (ElementId, Option<ElementId>)| -> Result<Reference, String> {
        let mut path: Vec<ElementId> = ancestors(tree, card)
            .into_iter()
            .take_while(|id| *id != common)
            .collect();
        path.reverse();
        path.extend(port);
        if path.is_empty() {
            return Err("an element cannot be connected to its own owner".into());
        }
        Ok(chain(tree, &path))
    };
    Ok((common, end(a)?, end(b)?))
}

/// How an element is named to the Operator: its name, or for unnamed
/// connections, interfaces and satisfy relationships what they join.
pub fn display_name(tree: &Tree, id: ElementId) -> String {
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
        ElementKind::Satisfy => match (&element.by, &element.target) {
            (Some(by), Some(target)) => format!("{by} satisfies {target}"),
            (_, Some(target)) => format!("satisfies {target}"),
            _ => "satisfy".into(),
        },
        kind => kind.keyword().to_string(),
    }
}

/// `Owner::Element`, with [`display_name`] for every step.
pub fn display_path(tree: &Tree, id: ElementId) -> String {
    let mut path: Vec<String> = ancestors(tree, id)
        .into_iter()
        .map(|step| display_name(tree, step))
        .collect();
    path.reverse();
    path.join("::")
}

/// Whether a change works on `element` itself.
fn touches(change: &Change, element: ElementId) -> bool {
    change.operations.iter().any(|operation| match operation {
        Operation::Delete { element: e }
        | Operation::Rename { element: e, .. }
        | Operation::Move { element: e, .. }
        | Operation::Set { element: e, .. }
        | Operation::Lock { element: e }
        | Operation::Unlock { element: e } => *e == element,
        Operation::Create { .. } | Operation::Connect { .. } => false,
    })
}

/// A name not yet used among the members of `parent`.
fn unused_name(tree: &Tree, parent: Parent, base: &str) -> String {
    let members: Vec<ElementId> = match parent {
        Parent::Element(id) => tree
            .get(id)
            .map(|e| e.children().to_vec())
            .unwrap_or_default(),
        Parent::Document(index) => tree
            .documents()
            .get(index)
            .map(|d| d.members().to_vec())
            .unwrap_or_default(),
    };
    let taken = |name: &str| {
        members
            .iter()
            .any(|id| tree.effective_name(*id) == Some(name))
    };
    (1..)
        .map(|n| format!("{base}{n}"))
        .find(|name| !taken(name))
        .expect("an unused name exists")
}

impl Studio {
    fn tree(&self) -> Option<&Tree> {
        self.project.as_ref().map(|p| p.state().tree())
    }

    /// Where new elements go: inside the selected card, else inside the
    /// project's top-level package, else at the top of the first document.
    pub(crate) fn create_parent(&self) -> Parent {
        if let Some(card) = self.selected_card() {
            return Parent::Element(card);
        }
        let tree = self.tree();
        tree.and_then(|t| t.roots().find(|r| t[*r].kind == ElementKind::Package))
            .map_or(Parent::Document(0), Parent::Element)
    }

    pub fn edit(&mut self, id: CommandId) {
        use CommandId::*;
        let create = |kind| (kind, !CreateKind::can_be_usage(kind));
        let (kind, definition) = match id {
            CreatePart => create(CreateKind::Part),
            CreatePort => create(CreateKind::Port),
            CreateItem => create(CreateKind::Item),
            CreateAttribute => create(CreateKind::Attribute),
            CreateInterface => create(CreateKind::Interface),
            CreateRequirement => create(CreateKind::Requirement),
            Rename => {
                if let Some(element) = self.inspected_element()
                    && let Some(tree) = self.tree()
                {
                    let name = tree.effective_name(element).unwrap_or("").to_string();
                    self.dialog = Some(Dialog::Rename { element, name });
                } else {
                    self.status = "Select an element to rename".into();
                }
                return;
            }
            Delete => return self.delete_selected(),
            Connect => return self.connect_selected(),
            MoveTo => {
                if let Some(element) = self.working_elements().first().copied() {
                    self.dialog = Some(Dialog::MoveTo {
                        element,
                        query: String::new(),
                    });
                }
                return;
            }
            Lock => return self.toggle_locks(),
            Undo => return self.undo_redo(true),
            Redo => return self.undo_redo(false),
            Checkpoint => {
                self.dialog = Some(Dialog::Checkpoint {
                    message: String::new(),
                });
                return;
            }
            _ => return,
        };
        self.dialog = Some(Dialog::Create {
            kind,
            definition,
            name: String::new(),
            parent: self.create_parent(),
        });
    }

    /// Applies an Operator change. A change to locked elements, or to a
    /// port shared through a definition, asks for confirmation first.
    pub fn submit(&mut self, change: Change) -> Option<ChangeEvent> {
        if let Some(definition) = self.shared_definition()
            && let Some(port) = self.inspected_element()
            && touches(&change, port)
        {
            let usages = self.usages_of(definition);
            let tree = self.tree()?;
            let name = |id| tree.effective_name(id).unwrap_or("element").to_string();
            let listed: Vec<String> = usages.iter().take(6).map(|u| name(*u)).collect();
            let more = usages.len().saturating_sub(listed.len());
            let question = format!(
                "{} is defined in {}. Changing it changes {} and every usage of it: {}{}. Change the definition, or specialise it for this usage only?",
                name(port),
                name(definition),
                name(definition),
                listed.join(", "),
                if more > 0 {
                    format!(" and {more} more")
                } else {
                    String::new()
                }
            );
            let usage = self.selected_card();
            self.dialog = Some(Dialog::Confirm {
                change,
                question,
                locked: Vec::new(),
                base: None,
                next: None,
                shared: usage.map(|usage| (definition, usage)),
            });
            return None;
        }
        match self.apply_change(change) {
            Outcome::Applied(event) => Some(event),
            _ => None,
        }
    }

    /// Applies a change without asking about shared definitions: the one
    /// path for the Operator's and the Assistant's changes. A change to
    /// locked elements opens the confirmation; the Operator's answer
    /// ([`answer`](Self::answer)) decides.
    pub fn apply_change(&mut self, change: Change) -> Outcome {
        let Some(project) = self.project.as_mut() else {
            return Outcome::NoProject;
        };
        self.timing.edit_started();
        let revision = project.state().revision();
        let next = project.state().tree().next_id();
        match project.apply(change.clone()) {
            Ok(event) => {
                self.saved = Ok(());
                self.status = event.description.clone();
                self.changed(&event);
                Outcome::Applied(event)
            }
            Err(ApplyError::Rejection(Rejection::Locked { elements })) => {
                let Some(tree) = self.tree() else {
                    return Outcome::NoProject;
                };
                let names: Vec<String> = elements
                    .iter()
                    .map(|id| tree.effective_name(*id).unwrap_or("element").to_string())
                    .collect();
                let one = names.len() == 1;
                let question = if change.actor == Actor::Assistant {
                    format!(
                        "The Assistant wants to change {}, which {} locked. Allow this change?",
                        names.join(", "),
                        if one { "is" } else { "are" },
                    )
                } else {
                    format!(
                        "{} {} locked. Change {} anyway?",
                        names.join(", "),
                        if one { "is" } else { "are" },
                        if one { "it" } else { "them" },
                    )
                };
                self.dialog = Some(Dialog::Confirm {
                    change,
                    question,
                    locked: elements,
                    base: Some(revision),
                    next: Some(next),
                    shared: None,
                });
                Outcome::Asking
            }
            Err(ApplyError::Rejection(rejection @ Rejection::Stale { .. }))
                if change.actor == Actor::Assistant && change.confirmed.is_empty() =>
            {
                // The model changed after the Assistant prepared this change
                // (for example in a dialog the Operator had open): it reads
                // the model again rather than overwrite that edit.
                Outcome::NotApplied(ApplyError::Rejection(rejection))
            }
            Err(ApplyError::Rejection(Rejection::Stale { .. })) => {
                // The model changed while the Operator was deciding: ask again.
                self.status =
                    "The model changed while you were deciding; please confirm again".into();
                let mut change = change;
                change.base = None;
                change.confirmed.clear();
                self.apply_change(change)
            }
            Err(ApplyError::Rejection(rejection)) => {
                self.status = format!("{} was not done: {}", change.description, plain(&rejection));
                Outcome::NotApplied(ApplyError::Rejection(rejection))
            }
            Err(ApplyError::Project(error)) => {
                // The project reverted the change because it could not be saved.
                self.saved = Err(error.to_string());
                self.status = format!(
                    "Not applied: could not save “{}”: {error}",
                    change.description
                );
                Outcome::NotApplied(ApplyError::Project(error))
            }
        }
    }

    pub fn operation(&mut self, description: &str, operation: Operation) -> Option<ChangeEvent> {
        self.submit(Change::new(Actor::Operator, description, vec![operation]))
    }

    pub fn set_property(&mut self, element: ElementId, property: Property, what: &str) {
        let name = self
            .tree()
            .and_then(|t| t.effective_name(element))
            .unwrap_or("element")
            .to_string();
        self.operation(
            &format!("Set {what} of {name}"),
            Operation::Set { element, property },
        );
    }

    pub fn create(&mut self, kind: CreateKind, definition: bool, name: &str, parent: Parent) {
        let Some(tree) = self.tree() else { return };
        let element_kind = kind.element_kind(definition);
        let name = if name.trim().is_empty() {
            let base = element_kind.keyword().replace(" def", "");
            let base = if definition {
                let mut chars = base.chars();
                chars
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            } else {
                base
            };
            unused_name(tree, parent, &base)
        } else {
            name.trim().to_string()
        };
        let description = format!("Create {} {name}", element_kind.keyword());
        let operation = Operation::Create {
            parent,
            element: Box::new(Element::named(element_kind, &name)),
        };
        if let Some(event) = self.operation(&description, operation)
            && let Some(created) = event.created.first().copied()
        {
            let target = match (element_kind, parent) {
                (ElementKind::Port, Parent::Element(card)) => SceneTarget::Port(card, created),
                _ => SceneTarget::Node(created),
            };
            if self.scene.target_bounds(&target).is_some() {
                self.select(target, false);
            } else {
                // An attribute or item line on its owner's card.
                self.inspected = Some((self.selection.primary.clone(), created));
            }
            self.panel = crate::studio::Panel::Inspector;
        }
    }

    pub fn rename(&mut self, element: ElementId, name: &str) {
        let old = self
            .tree()
            .and_then(|t| t.effective_name(element))
            .unwrap_or("element")
            .to_string();
        if old == name.trim() {
            return;
        }
        self.operation(
            &format!("Rename {old} to {}", name.trim()),
            Operation::Rename {
                element,
                name: name.trim().to_string(),
            },
        );
    }

    fn delete_selected(&mut self) {
        let mut elements = self.working_elements();
        let Some(tree) = self.tree() else { return };
        elements.sort();
        elements.dedup();
        // Deleting an owner deletes what it owns.
        let owners: Vec<ElementId> = elements.clone();
        elements.retain(|id| {
            !ancestors(tree, *id)
                .iter()
                .skip(1)
                .any(|owner| owners.contains(owner))
        });
        if elements.is_empty() {
            self.status = "A type or specialisation is removed in the Inspector".into();
            return;
        }
        let names: Vec<String> = elements
            .iter()
            .map(|id| tree.effective_name(*id).unwrap_or("element").to_string())
            .collect();
        let operations = elements
            .into_iter()
            .map(|element| Operation::Delete { element })
            .collect();
        self.submit(Change::new(
            Actor::Operator,
            &format!("Delete {}", names.join(", ")),
            operations,
        ));
    }

    fn connect_selected(&mut self) {
        let ends: Vec<(ElementId, Option<ElementId>)> = self
            .selection
            .targets
            .iter()
            .filter_map(|target| match target {
                SceneTarget::Port(card, port) => Some((*card, Some(*port))),
                SceneTarget::Node(card) | SceneTarget::Container(card) => Some((*card, None)),
                SceneTarget::Edge(_) => None,
            })
            .collect();
        if let [a, b] = ends[..] {
            self.connect(a, b);
        }
    }

    /// Connects two ends: an interface when both are ports, else a connection.
    pub fn connect(
        &mut self,
        a: (ElementId, Option<ElementId>),
        b: (ElementId, Option<ElementId>),
    ) {
        let Some(tree) = self.tree() else { return };
        let (parent, from, to) = match connection(tree, a, b) {
            Ok(found) => found,
            Err(reason) => {
                self.status = format!("Cannot connect: {reason}");
                return;
            }
        };
        let kind = if a.1.is_some() && b.1.is_some() {
            ElementKind::Interface
        } else {
            ElementKind::Connection
        };
        let description = format!("Connect {from} to {to}");
        if let Some(event) = self.operation(
            &description,
            Operation::Connect {
                parent,
                kind,
                name: None,
                definition: None,
                from,
                to,
            },
        ) && let Some(created) = event.created.first()
        {
            let edge = format!("element:{}", created.raw());
            let target = SceneTarget::Edge(edge);
            if self.scene.target_bounds(&target).is_some() {
                self.select(target, false);
            }
        }
    }

    pub fn move_to(&mut self, element: ElementId, parent: Parent) {
        let Some(tree) = self.tree() else { return };
        let name = tree
            .effective_name(element)
            .unwrap_or("element")
            .to_string();
        let owner = match parent {
            Parent::Element(id) => tree.effective_name(id).unwrap_or("element").to_string(),
            Parent::Document(_) => "the top level".into(),
        };
        self.operation(
            &format!("Move {name} into {owner}"),
            Operation::Move { element, parent },
        );
    }

    fn toggle_locks(&mut self) {
        let Some(project) = &self.project else { return };
        let state = project.state();
        let tree = state.tree();
        let elements = self.working_elements();
        let mut operations = Vec::new();
        let mut names = Vec::new();
        for element in elements {
            names.push(
                tree.effective_name(element)
                    .unwrap_or("element")
                    .to_string(),
            );
            operations.push(if state.locks().contains(&element) {
                Operation::Unlock { element }
            } else {
                Operation::Lock { element }
            });
        }
        let locking = operations
            .iter()
            .any(|op| matches!(op, Operation::Lock { .. }));
        let verb = if locking { "Lock" } else { "Unlock" };
        self.submit(Change::new(
            Actor::Operator,
            &format!("{verb} {}", names.join(", ")),
            operations,
        ));
    }

    fn undo_redo(&mut self, undo: bool) {
        let Some(project) = &mut self.project else {
            return;
        };
        let result = if undo { project.undo() } else { project.redo() };
        match result {
            Ok(Some(event)) => {
                self.saved = Ok(());
                self.status = format!(
                    "{} {}",
                    if undo { "Undid" } else { "Redid" },
                    event.description
                );
                self.changed(&event);
            }
            Ok(None) => {
                self.status = if undo {
                    "Nothing to undo"
                } else {
                    "Nothing to redo"
                }
                .into()
            }
            Err(error) => {
                self.saved = Err(error.to_string());
                let verb = if undo { "undone" } else { "redone" };
                self.status = format!("Not {verb}: could not save the project: {error}");
            }
        }
    }

    pub fn checkpoint(&mut self, message: &str) {
        let Some(project) = &mut self.project else {
            return;
        };
        let message = if message.trim().is_empty() {
            "Checkpoint"
        } else {
            message.trim()
        };
        match project.checkpoint(message) {
            Ok(checkpoint) => {
                self.saved = Ok(());
                self.status = format!("Checkpoint: {}", checkpoint.message);
                self.history.reload(project);
            }
            Err(ProjectError::NoChanges) => {
                self.status = "Nothing changed since the last checkpoint".into();
            }
            Err(error) => {
                self.status = format!("The checkpoint was not recorded: {error}");
            }
        }
    }

    /// Answers the open confirmation. Yes applies the change with the locks
    /// confirmed, at the revision it was asked at; no leaves the model as it
    /// is. For the Assistant's change, the outcome is its tool result.
    pub fn answer(&mut self, yes: bool) {
        let Some(Dialog::Confirm {
            mut change,
            locked,
            base,
            next,
            ..
        }) = self.dialog.take()
        else {
            return;
        };
        let assistant = change.actor == Actor::Assistant;
        let current = self.project.as_ref().map(|p| p.state().revision());
        // A change that refers to elements it creates itself was planned
        // for the ids the model handed out then; after another change it
        // must be planned again, never applied to the changed model.
        let replan = yes
            && base.is_some()
            && base != current
            && next.is_some_and(|next| refers_to_new(&change, next));
        let outcome = if replan {
            self.status =
                "The model changed while you were deciding; nothing was changed. Please do it again."
                    .into();
            Outcome::NotApplied(ApplyError::Rejection(Rejection::Stale {
                base: base.unwrap_or_default(),
                current: current.unwrap_or_default(),
            }))
        } else if yes {
            change.confirmed = locked;
            change.base = base;
            self.apply_change(change)
        } else {
            Outcome::NotApplied(ApplyError::Rejection(Rejection::Locked {
                elements: locked,
            }))
        };
        if assistant {
            self.assistant_change_done(outcome);
        }
    }

    /// Owners an element can move into: namespaces outside it, and the top level.
    pub fn owner_options(&self, element: ElementId) -> Vec<(Parent, String)> {
        let Some(tree) = self.tree() else {
            return Vec::new();
        };
        let inside = tree.descendants(element);
        let current = tree.get(element).and_then(Element::owner);
        let mut options = vec![(Parent::Document(0), "Top level".to_string())];
        for id in tree.walk() {
            let kind = tree[id].kind;
            if kind.is_namespace()
                && kind != ElementKind::Port
                && !inside.contains(&id)
                && Some(id) != current
            {
                options.push((
                    Parent::Element(id),
                    format!("{}  ·  {}", tree.qualified_name(id), kind.keyword()),
                ));
            }
        }
        options
    }
}

/// Whether a change refers to an element with an id from `next` up: one it
/// creates itself.
fn refers_to_new(change: &Change, next: ElementId) -> bool {
    let new = |id: &ElementId| id.raw() >= next.raw();
    let in_reference = |r: &Reference| r.steps.iter().filter_map(|s| s.target).any(|t| new(&t));
    let in_element = |e: &Element| e.references().into_iter().any(|(_, r)| in_reference(r));
    change.operations.iter().any(|operation| match operation {
        Operation::Create { parent, element } => {
            matches!(parent, Parent::Element(id) if new(id)) || in_element(element)
        }
        Operation::Connect {
            parent,
            definition,
            from,
            to,
            ..
        } => {
            new(parent)
                || definition.as_ref().is_some_and(in_reference)
                || in_reference(from)
                || in_reference(to)
        }
        Operation::Move { element, parent } => {
            new(element) || matches!(parent, Parent::Element(id) if new(id))
        }
        Operation::Set { element, property } => {
            new(element)
                || match property {
                    Property::TypedBy(refs)
                    | Property::Specializes(refs)
                    | Property::Redefines(refs)
                    | Property::Ends(refs) => refs.iter().any(in_reference),
                    Property::Target(r) | Property::By(r) => r.as_ref().is_some_and(in_reference),
                    _ => false,
                }
        }
        Operation::Delete { element }
        | Operation::Rename { element, .. }
        | Operation::Lock { element }
        | Operation::Unlock { element } => new(element),
    })
}

/// A rejection in plain words.
pub fn plain(rejection: &Rejection) -> String {
    match rejection {
        Rejection::Invalid { reason, .. } => reason.clone(),
        Rejection::Locked { .. } => "it touches a locked element".into(),
        Rejection::Stale { .. } => "the model changed in the meantime; try again".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_studio_scene::fixtures;

    #[test]
    fn a_connection_between_ports_of_sibling_parts_lives_in_their_owner() {
        let tree = fixtures::tree(fixtures::URL_SHORTENER);
        let service = tree.find("UrlShortener::UrlShortenerService").unwrap();
        let api = tree.find("UrlShortener::UrlShortenerService::api").unwrap();
        let store = tree
            .find("UrlShortener::UrlShortenerService::store")
            .unwrap();
        let storage = tree.find("UrlShortener::HttpApi::storage").unwrap();
        let links = tree.find("UrlShortener::LinkStore::links").unwrap();
        let (parent, from, to) =
            connection(&tree, (api, Some(storage)), (store, Some(links))).unwrap();
        assert_eq!(parent, service);
        assert_eq!(from.to_string(), "api.storage");
        assert_eq!(to.to_string(), "store.links");
        assert_eq!(to.target(), Some(links));
    }

    #[test]
    fn default_names_are_unused() {
        let tree = fixtures::tree("package P { part part1; }");
        let package = tree.find("P").unwrap();
        assert_eq!(
            unused_name(&tree, Parent::Element(package), "part"),
            "part2"
        );
    }
}

#[cfg(test)]
pub(crate) mod app_tests {
    use super::*;
    use crate::studio::{Studio, System};
    use clap::Parser;

    pub(crate) struct Folder(pub(crate) std::path::PathBuf);
    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A Studio with a new project open, driven without a window.
    pub(crate) fn studio(name: &str) -> (Studio, Folder) {
        let folder = std::env::temp_dir().join(format!("agq-studio-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let session = folder.join("session.json");
        let args = crate::Args::parse_from([
            "studio",
            "--no-restore",
            "--session",
            session.to_str().unwrap(),
        ]);
        let mut app = Studio::new(
            args,
            System {
                dark: true,
                reduced_motion: false,
            },
        );
        app.camera.viewport = agq_studio_scene::Size::new(1200.0, 800.0);
        app.create_project(&folder.join("P"), "P");
        assert!(app.project.is_some(), "{}", app.status);
        (app, Folder(folder))
    }

    /// About twelve elements per component (as System State's performance
    /// test): part defs with ports, an attribute and an inner part, used and
    /// connected in one system.
    fn large_model(components: usize) -> String {
        use std::fmt::Write;
        let mut text = String::from(
            "package P {
    item def Message;
    port def Link { in item payload : Message; }
",
        );
        for i in 0..components {
            let _ = writeln!(
                text,
                "    part def Component{i} {{
        doc /* Component {i}. */
        port input : Link;
        port output : ~Link;
        attribute size : ScalarValues::Integer = {i};
        part inner{i} : Inner{i};
    }}
    part def Inner{i} {{ port a : Link; port b : ~Link; attribute weight : ScalarValues::Real = 1.5; }}"
            );
        }
        text.push_str(
            "    part def System {
",
        );
        for i in 0..components {
            let _ = writeln!(text, "        part c{i} : Component{i};");
        }
        for i in 0..components - 1 {
            let _ = writeln!(
                text,
                "        connection link{i} connect c{i}.output to c{}.input;",
                i + 1
            );
        }
        text.push_str(
            "    }
}
",
        );
        text
    }

    /// Edit to Surface at 10k elements (§3.3, C-33, S5.1): a change applied
    /// through the Studio's one path, until the scene that shows it is built
    /// (CPU side; painting and presenting it add one display frame).
    #[test]
    #[cfg_attr(debug_assertions, ignore = "budgets are measured in release builds")]
    fn an_edit_on_ten_thousand_elements_reaches_the_surface() {
        let (mut app, folder) = studio("edit-10k");
        let project = folder.0.join("P");
        std::fs::write(project.join("model").join("P.sysml"), large_model(850)).unwrap();
        app.open_project(&project);
        let elements = app.project.as_ref().unwrap().state().tree().len();
        assert!(elements >= 10_000, "{elements} elements");
        let system = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("P::System")
            .unwrap();
        let mut times = Vec::new();
        for round in 0..5 {
            let started = std::time::Instant::now();
            app.create(
                CreateKind::Part,
                false,
                &format!("added{round}"),
                Parent::Element(system),
            );
            times.push(started.elapsed());
        }
        times.sort();
        let median = times[times.len() / 2];
        println!(
            "edit to Surface at {elements} elements: median {median:?}, best {:?}; last scene {:.1} ms (layout and routing {:.1} ms, index {:.1} ms); edges routed {} of {}",
            times[0],
            app.timing.scene_ms,
            app.timing.layout_ms,
            app.timing.index_ms,
            app.scene.routing().routed,
            app.scene.routing().routed + app.scene.routing().kept,
        );
        // Target 100 ms (C-33); the ceiling is about twice that.
        assert!(median < std::time::Duration::from_millis(200), "{median:?}");
    }

    #[test]
    fn the_standard_commands_fit_zoom_find_and_list_shortcuts() {
        let (mut app, _folder) = studio("shortcuts");
        let store = part(&mut app, "store");
        app.frame_number = 10;
        app.set_view(crate::navigation::SurfaceView::Graph);
        // Fit (Shift+1) moves the camera; it does not switch the view.
        let far = agq_studio_scene::Point::new(1.0e5, 1.0e5);
        app.camera.center = far;
        app.camera_target = None;
        app.execute(CommandId::Fit);
        assert_eq!(app.view, crate::navigation::SurfaceView::Graph);
        let aimed = app
            .camera_target
            .map_or(app.camera.center, |target| target.center);
        assert!(aimed != far, "the camera was not fitted");
        // Zoom to selection (Shift+2) moves the camera to the selection.
        app.selection.primary = Some(SceneTarget::Node(store));
        app.camera_target = None;
        app.execute(CommandId::ZoomToSelection);
        assert!(app.camera_target.is_some() || app.reduced_motion);
        // Ctrl+P finds elements; ? lists every shortcut in Settings.
        app.execute(CommandId::GoToElement);
        assert_eq!(app.palette, Some(crate::studio::PaletteMode::Elements));
        app.palette = None;
        app.execute(CommandId::ShortcutHelp);
        assert!(app.settings_open);
        assert_eq!(app.settings_section, crate::settings::Section::Keyboard);
    }

    #[test]
    fn plus_minus_and_shift_0_zoom_around_the_middle_at_once() {
        let (mut app, _folder) = studio("zoom-keys");
        app.camera.zoom = 1.0;
        let center = app.camera.center;
        app.execute(CommandId::ZoomIn);
        assert!((app.camera.zoom - 1.25).abs() < 1e-4, "{}", app.camera.zoom);
        assert!(
            app.camera_target.is_none(),
            "keyboard zoom does not animate"
        );
        app.execute(CommandId::ZoomOut);
        assert!((app.camera.zoom - 1.0).abs() < 1e-4, "{}", app.camera.zoom);
        app.execute(CommandId::ZoomIn);
        app.execute(CommandId::ZoomIn);
        assert!(app.camera.zoom > 1.5);
        app.execute(CommandId::ZoomReset);
        assert!((app.camera.zoom - 1.0).abs() < 1e-4, "{}", app.camera.zoom);
        // Around the middle: the center stays where it was.
        assert!((app.camera.center.x - center.x).abs() < 1e-3);
        assert!((app.camera.center.y - center.y).abs() < 1e-3);
    }

    #[test]
    fn the_arrow_keys_move_the_selection_to_the_next_card_that_way() {
        let (mut app, _folder) = studio("arrows");
        let names = ["a", "b", "c"];
        let ids: Vec<_> = names.iter().map(|name| part(&mut app, name)).collect();
        let center = |app: &Studio, id| {
            app.scene
                .nodes
                .iter()
                .find(|node| node.id() == id)
                .unwrap()
                .bounds
                .center()
        };
        // Nothing selected: an arrow selects a card near the middle.
        app.selection.clear();
        app.select_neighbour((1.0, 0.0));
        let Some(SceneTarget::Node(first)) = app.selection.primary.clone() else {
            panic!("a card is selected")
        };
        // Every further arrow moves in its direction, or stays put at the edge.
        let mut moved = 0;
        for sign in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let Some(SceneTarget::Node(before)) = app.selection.primary.clone() else {
                panic!()
            };
            app.select_neighbour(sign);
            let Some(SceneTarget::Node(after)) = app.selection.primary.clone() else {
                panic!()
            };
            if after != before {
                moved += 1;
                let (from, to) = (center(&app, before), center(&app, after));
                let along = (to.x - from.x) * sign.0 + (to.y - from.y) * sign.1;
                assert!(along > 0.0, "{sign:?} moved the other way");
            }
        }
        assert!(ids.contains(&first));
        assert!(moved >= 1, "three cards, and no arrow reached another");
    }

    #[test]
    fn focus_mode_hides_the_panels_and_is_remembered_per_project() {
        let (mut app, folder) = studio("focus-mode");
        let first = app.project.as_ref().unwrap().folder().to_path_buf();
        app.execute(CommandId::HidePanels);
        assert!(app.panels_hidden);
        // Another project has its own layout; the first keeps focus mode.
        app.create_project(&folder.0.join("Q"), "Q");
        assert!(!app.panels_hidden);
        app.open_project(&first);
        assert!(app.panels_hidden);
    }

    #[test]
    fn single_panels_collapse_and_widths_are_remembered_per_project() {
        let (mut app, folder) = studio("collapse-panels");
        let first = app.project.as_ref().unwrap().folder().to_path_buf();
        app.execute(CommandId::ShowInspector);
        assert!(app.inspector_hidden && !app.outline_hidden);
        app.execute(CommandId::ShowOutline);
        assert!(app.outline_hidden);
        app.execute(CommandId::ShowConversation);
        assert!(!app.conversation.shown);
        app.widths.conversation = 512.0;
        app.create_project(&folder.0.join("Q"), "Q");
        assert!(!app.outline_hidden && !app.inspector_hidden && app.conversation.shown);
        assert_eq!(app.widths, crate::session::Widths::default());
        app.open_project(&first);
        assert!(app.outline_hidden && app.inspector_hidden && !app.conversation.shown);
        assert_eq!(app.widths.conversation, 512.0);
    }

    #[test]
    fn the_first_run_can_start_from_the_url_shortener() {
        let (mut app, folder) = studio("sample");
        app.create_sample(
            &folder.0.join("Sample"),
            crate::studio::SAMPLE_NAME,
            crate::studio::Sample::UrlShortener,
        );
        let project = app.project.as_ref().expect("the sample is open");
        let state = project.state();
        assert!(state.tree().find("UrlShortener::LinkStore").is_some());
        assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
        assert!(app.scene.nodes.len() > 5, "the sample is on the Surface");
    }

    fn part(app: &mut Studio, name: &str) -> ElementId {
        let package = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("P")
            .unwrap();
        app.create(CreateKind::Part, false, name, Parent::Element(package));
        app.project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find(&format!("P::{name}"))
            .unwrap()
    }

    fn name_of(app: &Studio, id: ElementId) -> String {
        let tree = app.project.as_ref().unwrap().state().tree();
        tree.effective_name(id).unwrap().to_string()
    }

    #[test]
    fn a_project_open_in_another_window_says_so() {
        let (app, folder) = studio("second");
        let project = app.project.as_ref().unwrap().folder().to_path_buf();
        let args = crate::Args::parse_from([
            "studio",
            "--no-restore",
            "--session",
            folder.0.join("second.json").to_str().unwrap(),
        ]);
        let mut second = Studio::new(
            args,
            System {
                dark: true,
                reduced_motion: false,
            },
        );
        second.open_project(&project);
        assert!(second.project.is_none());
        assert!(
            second.status.contains("open in another Agentique window"),
            "{}",
            second.status
        );
    }

    #[test]
    fn a_change_that_cannot_be_saved_is_reported_and_not_applied() {
        let (mut app, _folder) = studio("save-failure");
        let model = app.project.as_ref().unwrap().folder().join("model");
        let file = std::fs::read_dir(&model)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.extension().is_some_and(|e| e == "sysml"))
            .unwrap();
        // An edit made outside Agentique: the project refuses to overwrite it.
        let text = std::fs::read_to_string(&file).unwrap()
            + "
// edited outside
";
        std::fs::write(&file, text).unwrap();
        let package = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("P")
            .unwrap();
        app.create(CreateKind::Part, false, "api", Parent::Element(package));
        assert!(
            app.status.starts_with("Not applied: could not save"),
            "{}",
            app.status
        );
        assert!(app.saved.is_err(), "the status bar says Not saved");
        let exists = |app: &Studio| {
            app.project
                .as_ref()
                .unwrap()
                .state()
                .tree()
                .find("P::api")
                .is_some()
        };
        assert!(!exists(&app));
        app.execute(CommandId::Redo);
        assert!(!exists(&app), "redo does not apply the refused change");
        // Opening the open project reads it again, with the outside edit.
        let folder = app.project.as_ref().unwrap().folder().to_path_buf();
        app.open_project(&folder);
        assert!(app.project.is_some(), "{}", app.status);
        let package = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("P")
            .unwrap();
        app.create(CreateKind::Part, false, "api", Parent::Element(package));
        assert!(exists(&app), "{}", app.status);
        assert_eq!(app.saved, Ok(()));
    }

    #[test]
    fn a_change_is_shown_and_highlighted_where_it_happens() {
        let (mut app, _folder) = studio("highlight");
        let api = part(&mut app, "api");
        assert!(app.lookup.node(&app.scene, api).is_some());
        assert!(app.highlights.contains_key(&api));
        assert_eq!(app.saved, Ok(()));
    }

    #[test]
    fn undo_and_redo_go_through_the_project_and_update_the_surface() {
        let (mut app, _folder) = studio("undo");
        let api = part(&mut app, "api");
        app.execute(CommandId::Undo);
        assert!(app.lookup.node(&app.scene, api).is_none());
        app.execute(CommandId::Redo);
        assert!(app.lookup.node(&app.scene, api).is_some());
        assert!(app.status.starts_with("Redid"));
    }

    #[test]
    fn a_locked_change_asks_and_applies_only_when_confirmed() {
        let (mut app, _folder) = studio("lock");
        let api = part(&mut app, "api");
        app.operation("Lock api", Operation::Lock { element: api });
        app.rename(api, "gateway");
        assert!(
            matches!(app.dialog, Some(Dialog::Confirm { ref locked, .. }) if locked == &vec![api])
        );
        app.answer(false);
        assert!(app.dialog.is_none());
        assert_eq!(name_of(&app, api), "api");
        app.rename(api, "gateway");
        app.answer(true);
        assert!(app.dialog.is_none());
        assert_eq!(name_of(&app, api), "gateway");
        assert!(app.project.as_ref().unwrap().state().locks().contains(&api));
    }

    #[test]
    fn a_confirmation_given_after_the_model_changed_is_asked_again() {
        let (mut app, _folder) = studio("stale");
        let api = part(&mut app, "api");
        app.operation("Lock api", Operation::Lock { element: api });
        app.rename(api, "gateway");
        // Another change lands while the question is open.
        let other = Change::new(
            Actor::Assistant,
            "Create store",
            vec![Operation::Create {
                parent: Parent::Document(0),
                element: Box::new(Element::named(ElementKind::Part, "store")),
            }],
        );
        app.project.as_mut().unwrap().apply(other).unwrap();
        app.answer(true);
        assert!(
            matches!(app.dialog, Some(Dialog::Confirm { .. })),
            "asked again"
        );
        assert_eq!(name_of(&app, api), "api");
        app.answer(true);
        assert_eq!(name_of(&app, api), "gateway");
    }

    #[test]
    fn dialogs_block_edits_until_answered() {
        let (mut app, _folder) = studio("modal");
        let api = part(&mut app, "api");
        app.operation("Lock api", Operation::Lock { element: api });
        app.rename(api, "gateway");
        assert!(app.dialog.is_some());
        app.execute(CommandId::CreatePart);
        assert!(matches!(app.dialog, Some(Dialog::Confirm { .. })));
        assert!(app.status.contains("dialog"));
    }

    #[test]
    fn edits_are_highlighted_in_the_colour_of_who_made_them() {
        let (mut app, _folder) = studio("highlight-actor");
        let api = part(&mut app, "api");
        assert_eq!(app.highlights[&api].1, Actor::Operator);
        let package = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("P")
            .unwrap();
        let change = Change::new(
            Actor::Assistant,
            "Create store",
            vec![Operation::Create {
                parent: Parent::Element(package),
                element: Box::new(Element::named(ElementKind::Part, "store")),
            }],
        );
        let Outcome::Applied(event) = app.apply_change(change) else {
            panic!("applied")
        };
        assert_eq!(app.highlights[&event.created[0]].1, Actor::Assistant);
    }
}
