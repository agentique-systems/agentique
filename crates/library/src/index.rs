//! The index: one summary per building block, read from the model (name,
//! package, `doc`, features found through the language's own lookup), so
//! that browsing and searching never walk or parse the model while drawing.

use crate::{BlockRef, Library, ROOT, Scope, built_in, copy};
use agq_language::{Direction, ElementId, ElementKind, Multiplicity, Role, Semantics, Tree};
use std::collections::HashMap;

/// One feature of a block as the Library shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub element: ElementId,
    pub name: String,
    pub kind: ElementKind,
    /// `RequestPort`, `~RequestPort`, `Positive`; empty when untyped.
    pub type_name: String,
    pub multiplicity: Option<Multiplicity>,
    /// The value as written, such as `300`.
    pub value: Option<String>,
    /// For ports: items come in first (it serves or receives), so the port
    /// is drawn on the left; otherwise it calls or sends, on the right.
    pub inbound: bool,
    /// The definition it is inherited from, when it is not the block's own.
    pub inherited_from: Option<String>,
    /// For requirements: the requirement's text, and what satisfies it.
    pub text: String,
}

/// Where a project definition came from, when it is a copy of a block from
/// the built-in library or My Library: the same qualified name there.
/// `changed` when the project's copy no longer has the same content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin {
    pub scope: Scope,
    pub changed: bool,
}

/// A building block: one reusable definition, summarised.
#[derive(Clone, Debug)]
pub struct Block {
    pub reference: BlockRef,
    /// The definition, in its scope's tree (the project's for project blocks).
    pub element: ElementId,
    pub kind: ElementKind,
    pub name: String,
    pub qualified_name: String,
    /// The packages that hold it, without the `Library` root: `["Storage"]`.
    pub category: Vec<String>,
    /// The first sentence of its documentation.
    pub summary: String,
    pub doc: String,
    pub is_abstract: bool,
    /// From the language's standard library: referred to, never copied.
    pub standard: bool,
    /// The definitions it specialises, by name.
    pub generals: Vec<String>,
    pub ports: Vec<Feature>,
    pub attributes: Vec<Feature>,
    pub parts: Vec<Feature>,
    /// Items: a port's directed items, or items a definition holds.
    pub items: Vec<Feature>,
    pub requirements: Vec<Feature>,
    /// Connections and interfaces it owns or inherits.
    pub connections: usize,
    /// Project blocks: usages typed by it, and definitions specialising it.
    pub usages: usize,
    pub specializations: usize,
    pub origin: Option<Origin>,
    /// Problems reported in its source (My Library).
    pub problems: usize,
    pub(crate) keys: Keys,
}

impl Block {
    /// It has parts of its own or inherited: a composite block.
    pub fn composite(&self) -> bool {
        !self.parts.is_empty()
    }

    /// `part def`, or `abstract part def`.
    pub fn kind_label(&self) -> String {
        if self.is_abstract {
            format!("abstract {}", self.kind.keyword())
        } else {
            self.kind.keyword().to_string()
        }
    }

    /// Its source as the Operator reads it: `Built-in`, `Project`, `My
    /// Library`, or for a project copy `Project · from Built-in`.
    pub fn source_label(&self) -> String {
        match (self.reference.scope, self.origin) {
            (Scope::Project, Some(origin)) if origin.changed => {
                format!("Project · changed from {}", origin.scope.label())
            }
            (Scope::Project, Some(origin)) => format!("Project · from {}", origin.scope.label()),
            (scope, _) => scope.label().to_string(),
        }
    }
}

/// Lower-case texts searched for a block.
#[derive(Clone, Debug, Default)]
pub(crate) struct Keys {
    pub name: String,
    pub qualified: String,
    /// Documentation, category and feature names and types.
    pub words: String,
}

/// Every building block of the three scopes. The built-in and My Library
/// part is rebuilt when My Library changes, the project part when the
/// project's revision changes.
#[derive(Default)]
pub struct Index {
    blocks: Vec<Block>,
    /// Blocks from here on are the project's.
    project_start: usize,
    library_version: Option<u64>,
    project_revision: Option<u64>,
}

impl Index {
    pub fn build(library: &Library, project: Option<(&Tree, u64)>) -> Index {
        let mut index = Index::default();
        index.refresh(library, project);
        index
    }

    /// Rebuilds what changed since the index was built.
    pub fn refresh(&mut self, library: &Library, project: Option<(&Tree, u64)>) {
        let revision = project.map(|(_, revision)| revision);
        let library_changed = self.library_version != Some(library.version());
        if library_changed {
            let mut external = Vec::new();
            summarise_tree(built_in(), Scope::BuiltIn, false, &mut external);
            summarise_tree(agq_language::library(), Scope::BuiltIn, true, &mut external);
            summarise_tree(library.mine(), Scope::Mine, false, &mut external);
            let problems = problem_counts(library);
            for block in &mut external {
                if block.reference.scope == Scope::Mine {
                    block.problems = problems.get(&block.element).copied().unwrap_or(0);
                }
            }
            let project_blocks = self.blocks.split_off(self.project_start);
            self.project_start = external.len();
            self.blocks = external;
            self.blocks.extend(project_blocks);
            self.library_version = Some(library.version());
        }
        if library_changed || self.project_revision != revision {
            self.blocks.truncate(self.project_start);
            if let Some((tree, _)) = project {
                let mut blocks = Vec::new();
                summarise_tree(tree, Scope::Project, false, &mut blocks);
                let (usages, specializations) = uses(tree);
                for block in &mut blocks {
                    block.usages = usages.get(&block.element).copied().unwrap_or(0);
                    block.specializations =
                        specializations.get(&block.element).copied().unwrap_or(0);
                    block.origin = origin(library, tree, block);
                }
                self.blocks.extend(blocks);
            }
            self.project_revision = revision;
        }
    }

    /// Whether the index shows this library version and project revision.
    pub fn is_current(&self, library: &Library, project_revision: Option<u64>) -> bool {
        self.library_version == Some(library.version()) && self.project_revision == project_revision
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn get(&self, index: usize) -> Option<&Block> {
        self.blocks.get(index)
    }

    pub fn position(&self, reference: &BlockRef) -> Option<usize> {
        self.blocks.iter().position(|b| &b.reference == reference)
    }

    pub fn find(&self, reference: &BlockRef) -> Option<&Block> {
        self.position(reference).map(|i| &self.blocks[i])
    }

    /// The project's block for a definition.
    pub fn project_block(&self, element: ElementId) -> Option<usize> {
        (self.project_start..self.blocks.len()).find(|i| self.blocks[*i].element == element)
    }

    /// A built-in or My Library block that the project already holds, as an
    /// unchanged copy: the project's block, which stands for it.
    pub fn project_copy(&self, block: &Block) -> Option<usize> {
        if block.reference.scope == Scope::Project || block.standard {
            return None;
        }
        (self.project_start..self.blocks.len()).find(|i| {
            let candidate = &self.blocks[*i];
            candidate.qualified_name == block.qualified_name
                && candidate
                    .origin
                    .is_some_and(|o| o.scope == block.reference.scope && !o.changed)
        })
    }

    /// The categories of a scope (all scopes when `None`), sorted.
    pub fn categories(&self, scope: Option<Scope>) -> Vec<Vec<String>> {
        let mut out: Vec<Vec<String>> = self
            .blocks
            .iter()
            .filter(|b| scope.is_none_or(|s| b.reference.scope == s))
            .map(|b| b.category.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Summaries of every definition in `tree` (standard: only the concrete
/// ones, which are the ones a model uses as types).
fn summarise_tree(tree: &Tree, scope: Scope, standard: bool, out: &mut Vec<Block>) {
    let semantics = Semantics::new(tree);
    for id in tree.walk() {
        let element = &tree[id];
        if !element.kind.is_definition() || (standard && element.is_abstract) {
            continue;
        }
        if tree.effective_name(id).is_none() {
            continue;
        }
        out.push(summarise(tree, &semantics, id, scope, standard));
    }
}

/// The summary of one definition.
pub(crate) fn summarise(
    tree: &Tree,
    semantics: &Semantics,
    id: ElementId,
    scope: Scope,
    standard: bool,
) -> Block {
    let element = &tree[id];
    let name = tree.effective_name(id).unwrap_or_default().to_string();
    let qualified_name = tree.qualified_name(id);
    let mut category: Vec<String> = Vec::new();
    let mut owner = element.owner();
    while let Some(o) = owner {
        category.push(tree.effective_name(o).unwrap_or("").to_string());
        owner = tree[o].owner();
    }
    category.reverse();
    if category.first().map(String::as_str) == Some(ROOT) && scope != Scope::Project {
        category.remove(0);
    }
    let doc = doc_of(tree, id);
    let generals: Vec<String> = element
        .specializes
        .iter()
        .map(|r| {
            r.target()
                .and_then(|t| name_of(tree, t))
                .unwrap_or_else(|| r.last_name().to_string())
        })
        .collect();
    let mut block = Block {
        reference: BlockRef::new(scope, &qualified_name),
        element: id,
        kind: element.kind,
        name,
        qualified_name,
        category,
        summary: summary(&doc),
        doc,
        is_abstract: element.is_abstract,
        standard,
        generals,
        ports: Vec::new(),
        attributes: Vec::new(),
        parts: Vec::new(),
        items: Vec::new(),
        requirements: Vec::new(),
        connections: 0,
        usages: 0,
        specializations: 0,
        origin: None,
        problems: 0,
        keys: Keys::default(),
    };
    if !standard {
        for feature in semantics.features(id) {
            let summary = feature_of(tree, semantics, id, feature);
            match summary.kind {
                ElementKind::Port => block.ports.push(summary),
                ElementKind::Attribute => block.attributes.push(summary),
                ElementKind::Part => block.parts.push(summary),
                ElementKind::Item => block.items.push(summary),
                ElementKind::Requirement => block.requirements.push(summary),
                _ => {}
            }
        }
        block.connections = all_generals(semantics, id)
            .into_iter()
            .filter(|g| !semantics.is_library(*g))
            .flat_map(|g| {
                tree.get(g)
                    .map(|e| e.children().to_vec())
                    .unwrap_or_default()
            })
            .filter(|c| {
                matches!(
                    tree[*c].kind,
                    ElementKind::Connection | ElementKind::Interface
                )
            })
            .count();
    }
    block.keys = keys(&block);
    block
}

/// One feature, with what the Library shows about it.
pub(crate) fn feature_of(
    tree: &Tree,
    semantics: &Semantics,
    owner: ElementId,
    feature: ElementId,
) -> Feature {
    let element = semantics
        .element(feature)
        .expect("features come from the tree");
    let types = semantics.types_of(feature);
    let type_name = types
        .iter()
        .map(|(ty, conjugated)| {
            let name = name_of(tree, *ty).unwrap_or_default();
            if *conjugated {
                format!("~{name}")
            } else {
                name
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let inbound = match types.first() {
        Some((port_def, conjugated)) if element.kind == ElementKind::Port => {
            let first = semantics
                .features(*port_def)
                .into_iter()
                .find_map(|f| semantics.element(f).and_then(|e| e.direction));
            !matches!(
                first.map(|d| if *conjugated { d.flipped() } else { d }),
                Some(Direction::Out)
            )
        }
        _ => true,
    };
    let inherited_from = semantics
        .is_inherited(owner, feature)
        .then(|| element.owner().and_then(|o| name_of(tree, o)))
        .flatten();
    let mut text = String::new();
    if element.kind == ElementKind::Requirement {
        text = doc_of(tree, feature);
        if text.is_empty()
            && let Some((definition, _)) = types.first()
        {
            text = doc_of(tree, *definition);
        }
        if let Some(by) = satisfied_by(tree, semantics, owner, feature) {
            text = format!("{text} (satisfied by {by})").trim().to_string();
        }
    }
    Feature {
        element: feature,
        name: tree.effective_name(feature).unwrap_or_default().to_string(),
        kind: element.kind,
        type_name,
        multiplicity: element.multiplicity,
        value: element.value.as_ref().map(ToString::to_string),
        inbound,
        inherited_from,
        text,
    }
}

/// What satisfies a requirement usage of `owner`, as written.
fn satisfied_by(
    tree: &Tree,
    semantics: &Semantics,
    owner: ElementId,
    requirement: ElementId,
) -> Option<String> {
    all_generals(semantics, owner)
        .into_iter()
        .filter_map(|g| tree.get(g))
        .flat_map(|g| g.children().iter().copied())
        .filter(|c| tree[*c].kind == ElementKind::Satisfy)
        .find(|c| {
            tree[*c]
                .target
                .as_ref()
                .and_then(|t| t.target())
                .is_some_and(|t| t == requirement)
        })
        .and_then(|c| tree[c].by.as_ref().map(ToString::to_string))
}

/// `id` and all its generals, nearest first.
pub(crate) fn all_generals(semantics: &Semantics, id: ElementId) -> Vec<ElementId> {
    let mut seen = vec![id];
    let mut i = 0;
    while i < seen.len() {
        for general in semantics.generals(seen[i]) {
            if !seen.contains(&general) {
                seen.push(general);
            }
        }
        i += 1;
    }
    seen
}

/// The effective name of an element of `tree` or the standard library.
pub(crate) fn name_of(tree: &Tree, id: ElementId) -> Option<String> {
    tree.effective_name(id)
        .or_else(|| agq_language::library().effective_name(id))
        .map(str::to_string)
}

/// The element's documentation, as plain text on one line.
pub(crate) fn doc_of(tree: &Tree, id: ElementId) -> String {
    let Some(element) = tree.get(id) else {
        return String::new();
    };
    element
        .children()
        .iter()
        .find(|c| tree[**c].kind == ElementKind::Doc)
        .and_then(|doc| tree[*doc].text.as_deref())
        .map(clean_doc)
        .unwrap_or_default()
}

/// A doc comment's text without the comment markers and line breaks.
pub fn clean_doc(text: &str) -> String {
    text.trim()
        .trim_start_matches("/*")
        .trim_end_matches("*/")
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first sentence, at most about 160 characters.
pub(crate) fn summary(doc: &str) -> String {
    let end = doc
        .char_indices()
        .find(|(i, c)| *c == '.' && doc[i + 1..].chars().next().is_none_or(char::is_whitespace))
        .map_or(doc.len(), |(i, _)| i + 1);
    let first = &doc[..end];
    if first.chars().count() <= 160 {
        return first.to_string();
    }
    let cut: String = first.chars().take(157).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{cut}…")
}

fn keys(block: &Block) -> Keys {
    let mut words = vec![block.doc.clone(), block.category.join(" ")];
    words.extend(block.generals.iter().cloned());
    for feature in block
        .ports
        .iter()
        .chain(&block.parts)
        .chain(&block.attributes)
    {
        words.push(feature.name.clone());
        words.push(feature.type_name.trim_start_matches('~').to_string());
    }
    Keys {
        name: block.name.to_lowercase(),
        qualified: block.qualified_name.to_lowercase(),
        words: words.join(" ").to_lowercase(),
    }
}

/// How many usages are typed by each definition, and how many definitions
/// specialise it.
fn uses(tree: &Tree) -> (HashMap<ElementId, usize>, HashMap<ElementId, usize>) {
    let mut usages: HashMap<ElementId, usize> = HashMap::new();
    let mut specializations: HashMap<ElementId, usize> = HashMap::new();
    for (holder, role, reference) in tree.references() {
        let Some(target) = reference.target() else {
            continue;
        };
        match role {
            Role::TypedBy if tree[holder].kind.is_usage() => {
                *usages.entry(target).or_default() += 1
            }
            Role::Specializes if tree[holder].kind.is_definition() => {
                *specializations.entry(target).or_default() += 1
            }
            _ => {}
        }
    }
    (usages, specializations)
}

/// A project definition that has a built-in or My Library block's
/// qualified name is a copy of it, changed or not.
fn origin(library: &Library, project: &Tree, block: &Block) -> Option<Origin> {
    if block.category.first().map(String::as_str) != Some(ROOT) {
        return None;
    }
    for (scope, tree) in [(Scope::BuiltIn, built_in()), (Scope::Mine, library.mine())] {
        if let Some(source) = tree.find(&block.qualified_name)
            && tree[source].kind == block.kind
        {
            let changed = copy::difference(tree, source, project, block.element).is_some();
            return Some(Origin { scope, changed });
        }
    }
    None
}

/// Problems per My Library definition (at it or anything it owns).
fn problem_counts(library: &Library) -> HashMap<ElementId, usize> {
    let tree = library.mine();
    let mut counts = HashMap::new();
    for diagnostic in agq_language::validate(tree) {
        let mut current = Some(diagnostic.element);
        while let Some(id) = current {
            if tree.get(id).is_some_and(|e| e.kind.is_definition()) {
                *counts.entry(id).or_default() += 1;
            }
            current = tree.get(id).and_then(|e| e.owner());
        }
    }
    counts
}
