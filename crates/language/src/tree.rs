//! The authored element tree: one generic element shape with a kind and the
//! properties the SysML subset needs. Text is parsed into it and printed from it.
//!
//! References hold the name as written and, once linked, the identity of the
//! element they point at. A linked reference keeps pointing at that element
//! when it is renamed or moved; names are never identity.

use std::collections::HashMap;
use std::fmt;

/// Ids from here up belong to the built-in library.
pub(crate) const FIRST_LIBRARY_ID: u64 = 1 << 48;

/// Stable identity of an element. Opaque; never derived from names.
///
/// Parsing assigns ids in document order; [`Tree::rekey`] replaces them with
/// stored ones. Edits keep ids, new elements get fresh ids, and removed ids
/// are never handed out again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementId(u64);

impl ElementId {
    /// An id read back from storage. Ids from 2^48 up are reserved for the
    /// built-in library.
    pub fn from_raw(raw: u64) -> Self {
        ElementId(raw)
    }

    /// The number to store.
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// What an element is. Definitions and usages use the SysML keywords' names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElementKind {
    Package,
    PartDef,
    Part,
    PortDef,
    Port,
    ItemDef,
    Item,
    AttributeDef,
    Attribute,
    ConnectionDef,
    Connection,
    InterfaceDef,
    Interface,
    RequirementDef,
    Requirement,
    /// `subject s : T;` inside a requirement.
    Subject,
    /// A usage written without a kind keyword, such as `:>> x = 5;` or
    /// `x : T;` (a SysML reference usage). Printed without a keyword.
    Reference,
    /// `satisfy requirement by feature;`
    Satisfy,
    /// `import A::*;` or `import A::B;`
    Import,
    /// `doc /* text */`
    Doc,
    /// A bare `/* text */` comment.
    Comment,
    /// A construct outside the subset, kept as verbatim text (`text`) and
    /// reported as unsupported. `note` names the construct.
    Unsupported,
    /// Text that could not be parsed, kept verbatim (`text`) and reported.
    /// `note` is the syntax error message.
    SyntaxError,
}

impl ElementKind {
    /// The keyword(s) this kind is written with (used in messages too).
    pub fn keyword(self) -> &'static str {
        use ElementKind::*;
        match self {
            Package => "package",
            PartDef => "part def",
            Part => "part",
            PortDef => "port def",
            Port => "port",
            ItemDef => "item def",
            Item => "item",
            AttributeDef => "attribute def",
            Attribute => "attribute",
            ConnectionDef => "connection def",
            Connection => "connection",
            InterfaceDef => "interface def",
            Interface => "interface",
            RequirementDef => "requirement def",
            Requirement => "requirement",
            Subject => "subject",
            Reference => "ref",
            Satisfy => "satisfy",
            Import => "import",
            Doc => "doc",
            Comment => "comment",
            Unsupported => "unsupported",
            SyntaxError => "syntax error",
        }
    }

    pub fn is_definition(self) -> bool {
        use ElementKind::*;
        matches!(
            self,
            PartDef
                | PortDef
                | ItemDef
                | AttributeDef
                | ConnectionDef
                | InterfaceDef
                | RequirementDef
        )
    }

    /// Usages are the features: they have types and can be looked up by name.
    pub fn is_usage(self) -> bool {
        use ElementKind::*;
        matches!(
            self,
            Part | Port
                | Item
                | Attribute
                | Connection
                | Interface
                | Requirement
                | Subject
                | Reference
        )
    }

    /// Kinds whose members can be named from outside (`A::b`).
    pub fn is_namespace(self) -> bool {
        self == ElementKind::Package || self.is_definition() || self.is_usage()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visibility {
    #[default]
    Public,
    Protected,
    Private,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
    InOut,
}

impl Direction {
    pub fn keyword(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
            Direction::InOut => "inout",
        }
    }

    /// The direction seen from the other side of a conjugated port.
    pub fn flipped(self) -> Direction {
        match self {
            Direction::In => Direction::Out,
            Direction::Out => Direction::In,
            Direction::InOut => Direction::InOut,
        }
    }
}

/// `A::B::c`
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct QualifiedName {
    pub segments: Vec<String>,
}

impl QualifiedName {
    pub fn new<S: Into<String>>(segments: impl IntoIterator<Item = S>) -> Self {
        QualifiedName {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    pub fn last(&self) -> &str {
        self.segments.last().map_or("", String::as_str)
    }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                f.write_str("::")?;
            }
            write_name(f, segment)?;
        }
        Ok(())
    }
}

/// One step of a reference: the name as written and, once linked, its target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub name: QualifiedName,
    pub target: Option<ElementId>,
}

/// A reference to another element. `A::B` has one step; a feature chain
/// `a.b.c` has one step per feature. Linking fills in each step's target;
/// an unresolved reference keeps only its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub steps: Vec<Step>,
}

impl Reference {
    /// An unlinked reference from plain text: `.` separates chain steps and
    /// `::` name segments. For building trees in code and tests.
    pub fn new(text: &str) -> Self {
        Reference {
            steps: text
                .split('.')
                .map(|step| Step {
                    name: QualifiedName::new(step.split("::")),
                    target: None,
                })
                .collect(),
        }
    }

    /// A one-step reference already linked to `target`.
    pub fn to(target: ElementId, name: &str) -> Self {
        let mut reference = Reference::new(name);
        reference.steps.truncate(1);
        reference.steps[0].target = Some(target);
        reference
    }

    /// The element the whole reference points at, once linked.
    pub fn target(&self) -> Option<ElementId> {
        self.steps.last().and_then(|step| step.target)
    }

    pub fn is_linked(&self) -> bool {
        self.steps.iter().all(|step| step.target.is_some())
    }

    /// The last name segment as written.
    pub fn last_name(&self) -> &str {
        self.steps.last().map_or("", |step| step.name.last())
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, step) in self.steps.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "{}", step.name)?;
        }
        Ok(())
    }
}

/// Which property of an element a reference fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// `: T`
    TypedBy,
    /// `:>` specialisation or subsetting
    Specializes,
    /// `:>>`
    Redefines,
    /// A connection or interface end, `connect a.b to c.d`
    End,
    /// An import's imported name, or the requirement a `satisfy` names
    Target,
    /// `satisfy R by x`
    By,
}

/// `[lower..upper]`; `upper: None` is `*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Multiplicity {
    pub lower: u64,
    pub upper: Option<u64>,
}

impl fmt::Display for Multiplicity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.lower, self.upper) {
            (0, None) => f.write_str("[*]"),
            (lower, None) => write!(f, "[{lower}..*]"),
            (lower, Some(upper)) if lower == upper => write!(f, "[{lower}]"),
            (lower, Some(upper)) => write!(f, "[{lower}..{upper}]"),
        }
    }
}

/// A feature value `= literal`. Numbers keep their exact decimal text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    Boolean(bool),
    Integer(String),
    Real(String),
    /// The text between the quotes, escapes kept as written.
    String(String),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Boolean(b) => write!(f, "{b}"),
            Literal::Integer(text) | Literal::Real(text) => f.write_str(text),
            Literal::String(text) => write!(f, "\"{text}\""),
        }
    }
}

/// Where an element was read from. `document` indexes [`Tree::documents`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub document: usize,
    pub line: u32,
    pub column: u32,
}

/// One authored element. Fields a kind does not use stay empty. Its place in
/// the tree (owner and children) changes only through [`Tree`] methods.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: ElementKind,
    pub name: Option<String>,
    owner: Option<ElementId>,
    children: Vec<ElementId>,
    pub location: Option<Location>,
    pub visibility: Visibility,
    pub is_abstract: bool,
    /// `end` feature of a connection or interface definition.
    pub is_end: bool,
    pub direction: Option<Direction>,
    /// `: ~P`: the port's type is conjugated.
    pub conjugated: bool,
    /// `: T` (usages, subjects).
    pub typed_by: Vec<Reference>,
    /// `:> A`: specialisation for definitions, subsetting for usages.
    pub specializes: Vec<Reference>,
    /// `:>> x`
    pub redefines: Vec<Reference>,
    pub multiplicity: Option<Multiplicity>,
    /// `= literal`
    pub value: Option<Literal>,
    /// Connection and interface usages: `connect a.b to c.d`.
    pub ends: Vec<Reference>,
    /// Import: the imported name. Satisfy: the satisfied requirement.
    pub target: Option<Reference>,
    /// Import: `::*`, all members of the target.
    pub wildcard: bool,
    /// Satisfy: `by feature`.
    pub by: Option<Reference>,
    /// Doc and comment: the text. Unsupported and SyntaxError: the verbatim source.
    pub text: Option<String>,
    /// Unsupported: the construct. SyntaxError: the message.
    pub note: Option<String>,
}

impl Element {
    pub fn new(kind: ElementKind) -> Self {
        Element {
            kind,
            name: None,
            owner: None,
            children: Vec::new(),
            location: None,
            visibility: Visibility::Public,
            is_abstract: false,
            is_end: false,
            direction: None,
            conjugated: false,
            typed_by: Vec::new(),
            specializes: Vec::new(),
            redefines: Vec::new(),
            multiplicity: None,
            value: None,
            ends: Vec::new(),
            target: None,
            wildcard: false,
            by: None,
            text: None,
            note: None,
        }
    }

    pub fn named(kind: ElementKind, name: &str) -> Self {
        let mut element = Element::new(kind);
        element.name = Some(name.to_string());
        element
    }

    pub fn owner(&self) -> Option<ElementId> {
        self.owner
    }

    pub fn children(&self) -> &[ElementId] {
        &self.children
    }

    /// Every reference this element holds, with the property it fills.
    pub fn references(&self) -> Vec<(Role, &Reference)> {
        let mut out = Vec::new();
        out.extend(self.typed_by.iter().map(|r| (Role::TypedBy, r)));
        out.extend(self.specializes.iter().map(|r| (Role::Specializes, r)));
        out.extend(self.redefines.iter().map(|r| (Role::Redefines, r)));
        out.extend(self.ends.iter().map(|r| (Role::End, r)));
        out.extend(self.target.iter().map(|r| (Role::Target, r)));
        out.extend(self.by.iter().map(|r| (Role::By, r)));
        out
    }

    pub(crate) fn references_mut(&mut self) -> Vec<&mut Reference> {
        let mut out: Vec<&mut Reference> = Vec::new();
        out.extend(self.typed_by.iter_mut());
        out.extend(self.specializes.iter_mut());
        out.extend(self.redefines.iter_mut());
        out.extend(self.ends.iter_mut());
        out.extend(self.target.iter_mut());
        out.extend(self.by.iter_mut());
        out
    }
}

/// A source document: a path and its top-level elements in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub path: String,
    members: Vec<ElementId>,
}

impl Document {
    pub fn members(&self) -> &[ElementId] {
        &self.members
    }
}

/// Where an element goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    Document(usize),
    Element(ElementId),
}

/// Why a structural change was refused. The tree is unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeError {
    NoSuchElement(ElementId),
    NoSuchDocument(usize),
    /// Moving an element into itself or into one of its descendants.
    IntoItself,
    /// A stored id that is reserved for the library or used twice.
    BadId(ElementId),
}

/// The authored model: documents with their element trees.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    documents: Vec<Document>,
    elements: HashMap<ElementId, Element>,
    next_id: u64,
}

impl Default for Tree {
    fn default() -> Self {
        Tree::new()
    }
}

impl Tree {
    pub fn new() -> Self {
        Self::with_first_id(1)
    }

    pub(crate) fn with_first_id(first: u64) -> Self {
        Tree {
            documents: Vec::new(),
            elements: HashMap::new(),
            next_id: first,
        }
    }

    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    pub fn add_document(&mut self, path: &str) -> usize {
        self.documents.push(Document {
            path: path.to_string(),
            members: Vec::new(),
        });
        self.documents.len() - 1
    }

    /// Adds `element` as the last member of `parent`; returns its new id.
    pub fn add(&mut self, parent: Parent, element: Element) -> Result<ElementId, TreeError> {
        self.insert(parent, usize::MAX, element)
    }

    /// Adds `element` at `position` among the members of `parent` (clamped
    /// to the end); returns its new id. Children of `element` are dropped.
    pub fn insert(
        &mut self,
        parent: Parent,
        position: usize,
        mut element: Element,
    ) -> Result<ElementId, TreeError> {
        let id = ElementId(self.next_id);
        let members = self.members_mut(parent)?;
        members.insert(position.min(members.len()), id);
        self.next_id += 1;
        element.owner = match parent {
            Parent::Document(_) => None,
            Parent::Element(owner) => Some(owner),
        };
        element.children.clear();
        self.elements.insert(id, element);
        Ok(id)
    }

    /// Moves an element (with everything it owns) to `position` among the
    /// members of `parent`. Its id and the references to it are unchanged.
    pub fn move_to(
        &mut self,
        id: ElementId,
        parent: Parent,
        position: usize,
    ) -> Result<(), TreeError> {
        if !self.contains(id) {
            return Err(TreeError::NoSuchElement(id));
        }
        if let Parent::Element(target) = parent {
            if !self.contains(target) {
                return Err(TreeError::NoSuchElement(target));
            }
            if self.descendants(id).contains(&target) {
                return Err(TreeError::IntoItself);
            }
        }
        self.members_mut(parent)?;
        self.detach(id);
        let members = self.members_mut(parent)?;
        members.insert(position.min(members.len()), id);
        self.elements.get_mut(&id).expect("checked above").owner = match parent {
            Parent::Document(_) => None,
            Parent::Element(owner) => Some(owner),
        };
        Ok(())
    }

    /// Removes an element and everything it owns. Returns the removed ids.
    /// References to them stay linked to the removed ids and are reported.
    pub fn remove(&mut self, id: ElementId) -> Vec<ElementId> {
        if !self.contains(id) {
            return Vec::new();
        }
        self.detach(id);
        let removed = self.descendants(id);
        for gone in &removed {
            self.elements.remove(gone);
        }
        removed
    }

    fn detach(&mut self, id: ElementId) {
        match self.elements[&id].owner {
            Some(owner) => self
                .elements
                .get_mut(&owner)
                .expect("owner exists")
                .children
                .retain(|c| *c != id),
            None => self
                .documents
                .iter_mut()
                .for_each(|d| d.members.retain(|m| *m != id)),
        }
    }

    fn members_mut(&mut self, parent: Parent) -> Result<&mut Vec<ElementId>, TreeError> {
        match parent {
            Parent::Document(index) => self
                .documents
                .get_mut(index)
                .map(|d| &mut d.members)
                .ok_or(TreeError::NoSuchDocument(index)),
            Parent::Element(owner) => self
                .elements
                .get_mut(&owner)
                .map(|e| &mut e.children)
                .ok_or(TreeError::NoSuchElement(owner)),
        }
    }

    /// Replaces parsed ids with stored ones (`ids` maps parsed id to stored
    /// id). Elements not in the map get fresh ids, returned in document
    /// order. Future ids start above every id in use, so none is reused.
    pub fn rekey(
        &mut self,
        ids: &HashMap<ElementId, ElementId>,
    ) -> Result<Vec<ElementId>, TreeError> {
        // Targets of removed elements are retired ids. They are re-keyed too
        // (through `ids`, or to a fresh id), so such references stay removed
        // and never meet a new element.
        let retired: std::collections::BTreeSet<ElementId> = self
            .references()
            .into_iter()
            .flat_map(|(_, _, r)| r.steps.iter().filter_map(|s| s.target))
            .filter(|t| t.0 < FIRST_LIBRARY_ID && !self.contains(*t))
            .collect();
        let mut used = std::collections::HashSet::new();
        for (old, new) in ids {
            if (self.contains(*old) || retired.contains(old))
                && (new.0 == 0 || new.0 >= FIRST_LIBRARY_ID || !used.insert(*new))
            {
                return Err(TreeError::BadId(*new));
            }
        }
        // Stored ids of elements no longer in the text are retired: stay above them too.
        let highest = ids.values().map(|id| id.0).max().unwrap_or(0);
        self.next_id = self.next_id.max(highest + 1);
        let mut map = HashMap::new();
        let mut fresh = Vec::new();
        for id in self.walk().into_iter().chain(retired.iter().copied()) {
            let new = match ids.get(&id) {
                Some(new) => *new,
                None => {
                    let new = ElementId(self.next_id);
                    self.next_id += 1;
                    if self.contains(id) {
                        fresh.push(new);
                    }
                    new
                }
            };
            map.insert(id, new);
        }
        let remap = |id: ElementId| map.get(&id).copied();
        let elements = std::mem::take(&mut self.elements);
        for (id, mut element) in elements {
            element.owner = element.owner.and_then(remap);
            element.children = element.children.iter().filter_map(|c| remap(*c)).collect();
            for reference in element.references_mut() {
                for step in &mut reference.steps {
                    // Library targets keep their ids.
                    step.target = step
                        .target
                        .map(|t| if t.0 >= FIRST_LIBRARY_ID { t } else { map[&t] });
                }
            }
            self.elements.insert(map[&id], element);
        }
        for document in &mut self.documents {
            document.members = document.members.iter().filter_map(|m| remap(*m)).collect();
        }
        Ok(fresh)
    }

    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.elements.get(&id)
    }

    /// Edits an element's properties; its place in the tree changes only
    /// through [`Tree::move_to`], [`Tree::insert`] and [`Tree::remove`].
    pub fn get_mut(&mut self, id: ElementId) -> Option<&mut Element> {
        self.elements.get_mut(&id)
    }

    pub fn contains(&self, id: ElementId) -> bool {
        self.elements.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Top-level elements of all documents, in order.
    pub fn roots(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.documents
            .iter()
            .flat_map(|d| d.members.iter().copied())
    }

    /// `id` and everything it owns, depth first.
    pub fn descendants(&self, id: ElementId) -> Vec<ElementId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(next) = stack.pop() {
            out.push(next);
            if let Some(e) = self.get(next) {
                stack.extend(e.children.iter().rev());
            }
        }
        out
    }

    /// All elements in document order.
    pub fn walk(&self) -> Vec<ElementId> {
        self.roots().flat_map(|r| self.descendants(r)).collect()
    }

    /// The document an element belongs to.
    pub fn document_of(&self, id: ElementId) -> Option<usize> {
        let mut root = id;
        while let Some(owner) = self.get(root)?.owner {
            root = owner;
        }
        self.documents
            .iter()
            .position(|d| d.members.contains(&root))
    }

    /// Every reference in the tree: the element holding it, its role, and the reference.
    pub fn references(&self) -> Vec<(ElementId, Role, &Reference)> {
        self.walk()
            .into_iter()
            .flat_map(|id| {
                self.elements[&id]
                    .references()
                    .into_iter()
                    .map(move |(role, reference)| (id, role, reference))
            })
            .collect()
    }

    /// The linked references that point at `target` (at any chain step).
    pub fn references_to(&self, target: ElementId) -> Vec<(ElementId, Role)> {
        self.references()
            .into_iter()
            .filter(|(_, _, r)| r.steps.iter().any(|s| s.target == Some(target)))
            .map(|(id, role, _)| (id, role))
            .collect()
    }

    /// The name used for lookup: the declared name or, for `:>> x` without
    /// one, the current name of the redefined feature.
    pub fn effective_name(&self, id: ElementId) -> Option<&str> {
        let mut current = id;
        for _ in 0..32 {
            let element = self.get(current)?;
            if let Some(name) = &element.name {
                return Some(name);
            }
            let redefined = element.redefines.first()?;
            match redefined.target() {
                Some(target) if self.contains(target) => current = target,
                _ => return Some(redefined.last_name()),
            }
        }
        None
    }

    /// Finds an element by qualified name through owned members only (no
    /// imports or inheritance). Convenient for tests and tools.
    pub fn find(&self, qualified: &str) -> Option<ElementId> {
        let mut candidates: Vec<ElementId> = self.roots().collect();
        let mut found = None;
        for segment in qualified.split("::") {
            found = candidates
                .iter()
                .copied()
                .find(|c| self.effective_name(*c) == Some(segment));
            candidates = self[found?].children.clone();
        }
        found
    }

    /// `Package::Definition::feature`, for messages.
    pub fn qualified_name(&self, id: ElementId) -> String {
        let mut parts = Vec::new();
        let mut next = Some(id);
        while let Some(current) = next {
            let Some(element) = self.get(current) else {
                parts.push(current.to_string()); // removed: `#id`
                break;
            };
            parts.push(match self.effective_name(current) {
                Some(name) => QualifiedName::new([name]).to_string(),
                None => format!("({})", self.describe_unnamed(element)),
            });
            next = element.owner;
        }
        parts.reverse();
        parts.join("::")
    }

    fn describe_unnamed(&self, e: &Element) -> String {
        match (e.kind, &e.target) {
            (ElementKind::Satisfy, Some(target)) => format!("satisfy {target}"),
            (ElementKind::Import, Some(target)) if e.wildcard => format!("import {target}::*"),
            (ElementKind::Import, Some(target)) => format!("import {target}"),
            (ElementKind::Connection | ElementKind::Interface, _) if e.ends.len() == 2 => {
                format!("connect {} to {}", e.ends[0], e.ends[1])
            }
            (kind, _) => kind.keyword().into(),
        }
    }

    /// `path:line:column` of a location.
    pub fn describe_location(&self, location: Location) -> String {
        let path = self
            .documents
            .get(location.document)
            .map_or("?", |d| d.path.as_str());
        format!("{path}:{}:{}", location.line, location.column)
    }

    /// Clears source locations; two trees parsed from differently formatted
    /// but equivalent text are then equal.
    pub fn without_locations(mut self) -> Tree {
        for element in self.elements.values_mut() {
            element.location = None;
        }
        self
    }
}

impl std::ops::Index<ElementId> for Tree {
    type Output = Element;
    /// Panics if the element does not exist; use [`Tree::get`] when unsure.
    fn index(&self, id: ElementId) -> &Element {
        &self.elements[&id]
    }
}

/// Writes a name, quoting it when it is not a plain identifier or is a keyword.
pub(crate) fn write_name(f: &mut impl fmt::Write, name: &str) -> fmt::Result {
    let plain = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !crate::lexer::is_keyword(name);
    if plain {
        f.write_str(name)
    } else {
        f.write_char('\'')?;
        for c in name.chars() {
            if c == '\'' || c == '\\' {
                f.write_char('\\')?;
            }
            f.write_char(c)?;
        }
        f.write_char('\'')
    }
}
