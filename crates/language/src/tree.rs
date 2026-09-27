//! The authored element tree: one generic element shape with a kind and the
//! properties the SysML subset needs. Text is parsed into it and printed from it.

use std::collections::HashMap;
use std::fmt;

/// Stable identity of an element. Opaque; never derived from names.
///
/// Parsing assigns ids in document order. Edits keep the ids of the elements
/// they touch, and new elements get fresh ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementId(pub(crate) u64);

impl ElementId {
    /// The raw number, for mapping to a persistent identity file.
    pub fn get(self) -> u64 {
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
    /// `satisfy requirement by feature;`
    Satisfy,
    /// `import A::*;` or `import A::B;`
    Import,
    /// `doc /* text */`
    Doc,
    /// A construct outside the subset, kept as verbatim text (`text`) and
    /// reported as unsupported. `note` names the construct.
    Unsupported,
    /// Text that could not be parsed, kept verbatim (`text`) and reported.
    /// `note` is the syntax error message.
    SyntaxError,
}

impl ElementKind {
    /// The keyword(s) this kind is written with.
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
            Satisfy => "satisfy",
            Import => "import",
            Doc => "doc",
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
            Part | Port | Item | Attribute | Connection | Interface | Requirement | Subject
        )
    }

    /// Kinds whose members can be named from outside (`A::b`).
    pub fn is_namespace(self) -> bool {
        self == ElementKind::Package || self.is_definition() || self.is_usage()
    }

    /// The usage kind that belongs to a definition kind (part def -> part).
    pub fn usage_kind(self) -> Option<ElementKind> {
        use ElementKind::*;
        Some(match self {
            PartDef => Part,
            PortDef => Port,
            ItemDef => Item,
            AttributeDef => Attribute,
            ConnectionDef => Connection,
            InterfaceDef => Interface,
            RequirementDef => Requirement,
            _ => return None,
        })
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

/// `a.b.c`: each step names a feature of the previous step's types.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeatureChain {
    pub steps: Vec<QualifiedName>,
}

impl FeatureChain {
    /// Parses `a.b.c` from plain names; for building trees in code and tests.
    pub fn from_dotted(text: &str) -> Self {
        FeatureChain {
            steps: text.split('.').map(|s| QualifiedName::new([s])).collect(),
        }
    }
}

impl fmt::Display for FeatureChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, step) in self.steps.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "{step}")?;
        }
        Ok(())
    }
}

/// `: T` or, for ports, the conjugated `: ~T`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRef {
    pub name: QualifiedName,
    pub conjugated: bool,
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.conjugated {
            f.write_str("~")?;
        }
        write!(f, "{}", self.name)
    }
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

/// One authored element. Fields a kind does not use stay empty.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub kind: ElementKind,
    pub name: Option<String>,
    pub owner: Option<ElementId>,
    pub children: Vec<ElementId>,
    pub location: Option<Location>,
    pub visibility: Visibility,
    pub is_abstract: bool,
    /// `end` feature of a connection or interface definition.
    pub is_end: bool,
    pub direction: Option<Direction>,
    /// `: T` (usages, subjects).
    pub typed_by: Vec<TypeRef>,
    /// `:> A`: specialisation for definitions, subsetting for usages.
    pub specializes: Vec<QualifiedName>,
    /// `:>> x`
    pub redefines: Vec<QualifiedName>,
    pub multiplicity: Option<Multiplicity>,
    /// `= literal`
    pub value: Option<Literal>,
    /// Connection and interface usages: `connect a.b to c.d`.
    pub ends: Vec<FeatureChain>,
    /// Import: the imported name. Satisfy: the satisfied requirement.
    pub target: Option<QualifiedName>,
    /// Import: `::*`, all members of the target.
    pub wildcard: bool,
    /// Satisfy: `by feature`.
    pub by: Option<FeatureChain>,
    /// Doc: the comment text. Unsupported and SyntaxError: the verbatim source.
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

    /// The name used for lookup: the declared name or, for `:>> x` without a
    /// name, the name of the redefined feature.
    pub fn effective_name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .or_else(|| self.redefines.first().map(QualifiedName::last))
    }
}

/// A source document: a path and its top-level elements in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub path: String,
    pub members: Vec<ElementId>,
}

/// Where a new element goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    Document(usize),
    Element(ElementId),
}

/// The authored model: documents with their element trees.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub documents: Vec<Document>,
    elements: HashMap<ElementId, Element>,
    next_id: u64,
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

    pub fn add_document(&mut self, path: &str) -> usize {
        self.documents.push(Document {
            path: path.to_string(),
            members: Vec::new(),
        });
        self.documents.len() - 1
    }

    /// Adds `element` as the last member of `parent` and returns its new id.
    pub fn add(&mut self, parent: Parent, mut element: Element) -> ElementId {
        let id = ElementId(self.next_id);
        self.next_id += 1;
        element.children.clear();
        match parent {
            Parent::Document(index) => {
                element.owner = None;
                self.documents[index].members.push(id);
            }
            Parent::Element(owner) => {
                element.owner = Some(owner);
                self.elements
                    .get_mut(&owner)
                    .expect("parent element exists")
                    .children
                    .push(id);
            }
        }
        self.elements.insert(id, element);
        id
    }

    /// Removes an element and everything it owns. Returns the removed ids.
    pub fn remove(&mut self, id: ElementId) -> Vec<ElementId> {
        let Some(element) = self.elements.get(&id) else {
            return Vec::new();
        };
        match element.owner {
            Some(owner) => self.get_mut(owner).children.retain(|c| *c != id),
            None => self
                .documents
                .iter_mut()
                .for_each(|d| d.members.retain(|m| *m != id)),
        }
        let removed = self.descendants(id);
        for gone in &removed {
            self.elements.remove(gone);
        }
        removed
    }

    pub fn get(&self, id: ElementId) -> Option<&Element> {
        self.elements.get(&id)
    }

    pub fn get_mut(&mut self, id: ElementId) -> &mut Element {
        self.elements.get_mut(&id).expect("element exists")
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

    /// Finds an element by qualified name through owned members only (no
    /// imports or inheritance). Convenient for tests and tools.
    pub fn find(&self, qualified: &str) -> Option<ElementId> {
        let mut candidates: Vec<ElementId> = self.roots().collect();
        let mut found = None;
        for segment in qualified.split("::") {
            found = candidates
                .iter()
                .copied()
                .find(|c| self[*c].effective_name() == Some(segment));
            candidates = self[found?].children.clone();
        }
        found
    }

    /// `Package::Definition::feature`, for messages.
    pub fn qualified_name(&self, id: ElementId) -> String {
        let mut parts = Vec::new();
        let mut next = Some(id);
        while let Some(current) = next {
            let e = &self[current];
            parts.push(match e.effective_name() {
                Some(name) => QualifiedName::new([name]).to_string(),
                None => format!("({})", self.describe_unnamed(e)),
            });
            next = e.owner;
        }
        parts.reverse();
        parts.join("::")
    }

    fn describe_unnamed(&self, e: &Element) -> String {
        match e.kind {
            ElementKind::Satisfy => match &e.target {
                Some(target) => format!("satisfy {target}"),
                None => "satisfy".into(),
            },
            ElementKind::Import => match &e.target {
                Some(target) if e.wildcard => format!("import {target}::*"),
                Some(target) => format!("import {target}"),
                None => "import".into(),
            },
            ElementKind::Connection | ElementKind::Interface if !e.ends.is_empty() => {
                let ends: Vec<String> = e.ends.iter().map(ToString::to_string).collect();
                format!("connect {}", ends.join(" to "))
            }
            kind => kind.keyword().into(),
        }
    }

    /// `path:line:column`, or `path` alone for elements created by edits.
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
