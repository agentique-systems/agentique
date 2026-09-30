//! Library: reusable definitions, shown to the Operator and the Assistant as
//! building blocks (ROADMAP C-49, §4.13).
//!
//! A building block is not a new kind of element. It is any reusable
//! definition (`part def`, `port def`, `item def`, `attribute def`,
//! `interface def`, `connection def`, `requirement def`) from one of three
//! scopes:
//!
//! - **Built-in**: a small library of neutral software concepts shipped as
//!   SysML text (`blocks/Library.sysml`), plus the language's standard library
//!   (`ScalarValues`). Read-only.
//! - **Project**: every definition in the open project, found where it is.
//! - **My Library**: the Operator's own blocks, saved from any project into a
//!   SysML file in the app's local data.
//!
//! Using a block from the built-in library or My Library copies the
//! definitions it needs (its dependency closure) into the project's own
//! `Library` package, reusing identical copies already there, and adds a
//! usage typed by the project's definition. A project therefore never
//! depends on anything outside it, and changing My Library never changes a
//! project. Standard-library definitions are referred to, never copied.
//!
//! Everything that changes a project is planned here as one ordinary System
//! State [`Change`](agq_system_state::Change): one undo step, checked for
//! locks when the Studio applies it, the same for the Surface, the palette
//! and the Assistant. Meaning and validity stay in the language core:
//! features, types and port compatibility come from
//! [`agq_language::Semantics`]. The library keeps no metadata of its own:
//! a block's name, category, purpose, ports and parts are read from the
//! model (name, package, `doc`, features).
#![forbid(unsafe_code)]

mod compatible;
mod copy;
mod describe;
mod index;
mod plan;
pub mod search;

pub use compatible::Fit;
pub use copy::{Conflict, Missing, Resolution, closure};
pub use describe::{Preview, PreviewEnd, PreviewLink, PreviewPart, PreviewPort};
pub use index::{Block, Feature, Index, Origin};
pub use plan::{
    Boundary, ConnectTo, Extraction, Override, Plan, PlanError, SavePlan, Use, default_usage_name,
};
pub use search::{Hit, Match, Query, fuzzy};

use agq_language::{ElementId, Source, Tree, parse, print, validate};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The built-in building blocks, as SysML text.
pub const BUILT_IN_TEXT: &str = include_str!("../blocks/Library.sysml");

/// The package every copied block lives under, in a project and in My
/// Library: `Library::Storage::Cache`.
pub const ROOT: &str = "Library";

/// Where a building block comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    /// Shipped with Agentique: the built-in blocks and the standard library.
    BuiltIn,
    /// The open project's own definitions.
    Project,
    /// The Operator's own blocks, kept for use in any project.
    Mine,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Scope::BuiltIn, Scope::Project, Scope::Mine];

    /// As the Operator reads it.
    pub fn label(self) -> &'static str {
        match self {
            Scope::BuiltIn => "Built-in",
            Scope::Project => "Project",
            Scope::Mine => "My Library",
        }
    }

    /// As links and tool calls write it.
    pub fn key(self) -> &'static str {
        match self {
            Scope::BuiltIn => "built-in",
            Scope::Project => "project",
            Scope::Mine => "mine",
        }
    }

    pub fn from_key(key: &str) -> Option<Scope> {
        Scope::ALL.into_iter().find(|scope| scope.key() == key)
    }
}

/// A building block's identity across sessions: its scope and qualified
/// name, written `built-in:Library::Storage::Cache`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockRef {
    pub scope: Scope,
    pub qualified_name: String,
}

impl BlockRef {
    pub fn new(scope: Scope, qualified_name: &str) -> Self {
        BlockRef {
            scope,
            qualified_name: qualified_name.to_string(),
        }
    }

    /// Reads `built-in:Library::Storage::Cache` (the form [`Display`] writes).
    pub fn parse(text: &str) -> Option<BlockRef> {
        let (scope, name) = text.split_once(':')?;
        let scope = Scope::from_key(scope)?;
        let name = name.trim();
        (!name.is_empty() && !name.starts_with(':')).then(|| BlockRef::new(scope, name))
    }
}

impl fmt::Display for BlockRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.scope.key(), self.qualified_name)
    }
}

/// The built-in blocks, parsed once and shared.
pub fn built_in() -> &'static Tree {
    static TREE: OnceLock<Tree> = OnceLock::new();
    TREE.get_or_init(|| parse(&[Source::new("<built-in blocks>", BUILT_IN_TEXT)]))
}

/// Where a block's definition is: its scope, the tree that holds it and its
/// element there. `standard` blocks live in the language's standard library
/// and are referred to, never copied.
#[derive(Clone, Copy)]
pub struct Located<'a> {
    pub scope: Scope,
    pub tree: &'a Tree,
    pub element: ElementId,
    pub standard: bool,
}

/// The sources of building blocks outside the project: the built-in blocks
/// and My Library.
pub struct Library {
    mine: Tree,
    mine_path: Option<PathBuf>,
    mine_problems: Vec<String>,
    version: u64,
}

impl Default for Library {
    fn default() -> Self {
        Library::built_in_only()
    }
}

impl Library {
    /// The built-in blocks only, with an empty My Library that is not saved.
    pub fn built_in_only() -> Self {
        Library {
            mine: empty_mine(),
            mine_path: None,
            mine_problems: Vec::new(),
            version: 0,
        }
    }

    /// With My Library read from `path`; a missing file is an empty library.
    pub fn with_mine(path: PathBuf) -> Self {
        let mut library = Library::built_in_only();
        library.mine_path = Some(path);
        library.reload();
        library
    }

    /// Reads My Library again from its file.
    pub fn reload(&mut self) {
        self.version += 1;
        self.mine_problems.clear();
        let Some(path) = &self.mine_path else {
            return;
        };
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let name = path.file_name().map_or("My Library.sysml".into(), |n| {
                    n.to_string_lossy().to_string()
                });
                self.mine = parse(&[Source::new(&name, &text)]);
                self.mine_problems = validate(&self.mine)
                    .into_iter()
                    .map(|d| {
                        format!(
                            "`{}`: {} [{}]",
                            self.mine.qualified_name(d.element),
                            d.message,
                            d.code
                        )
                    })
                    .collect();
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.mine = empty_mine();
            }
            Err(error) => {
                self.mine = empty_mine();
                self.mine_problems
                    .push(format!("My Library could not be read: {error}"));
            }
        }
    }

    /// Where My Library is kept, if it is kept.
    pub fn mine_path(&self) -> Option<&Path> {
        self.mine_path.as_deref()
    }

    /// Problems found in My Library's file, in plain words. Blocks with
    /// problems are still listed; using one reports why it cannot be used.
    pub fn problems(&self) -> &[String] {
        &self.mine_problems
    }

    /// Increases whenever My Library changes, so indexes know to rebuild.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// My Library's tree.
    pub fn mine(&self) -> &Tree {
        &self.mine
    }

    /// Finds a block's definition. A built-in name that is not a built-in
    /// block may name a standard-library definition (`ScalarValues::String`).
    pub fn locate<'a>(
        &'a self,
        reference: &BlockRef,
        project: Option<&'a Tree>,
    ) -> Option<Located<'a>> {
        let found = |scope, tree: &'a Tree, standard| {
            let element = tree.find(&reference.qualified_name)?;
            tree[element].kind.is_definition().then_some(Located {
                scope,
                tree,
                element,
                standard,
            })
        };
        match reference.scope {
            Scope::BuiltIn => found(Scope::BuiltIn, built_in(), false)
                .or_else(|| found(Scope::BuiltIn, agq_language::library(), true)),
            Scope::Mine => found(Scope::Mine, &self.mine, false),
            Scope::Project => found(Scope::Project, project?, false),
        }
    }

    /// Replaces My Library with `tree` and writes it, atomically: the old
    /// file stays until the new one is complete.
    pub fn save_mine(&mut self, tree: Tree) -> std::io::Result<()> {
        let Some(path) = self.mine_path.clone() else {
            self.mine = tree;
            self.version += 1;
            return Ok(());
        };
        let text: String = print(&tree)
            .into_iter()
            .map(|source| source.text)
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let temporary = path.with_extension("sysml.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(&temporary, &path)?;
        self.reload();
        Ok(())
    }
}

/// An empty My Library: one document holding nothing yet.
fn empty_mine() -> Tree {
    let mut tree = Tree::new();
    tree.add_document("My Library.sysml");
    tree
}

/// The kind of usage a block of `kind` is used as: a part for a part def,
/// a port for a port def, and so on. Connection and interface definitions
/// type connections, which need ends, so they have none.
pub fn usage_kind(kind: agq_language::ElementKind) -> Option<agq_language::ElementKind> {
    use agq_language::ElementKind::*;
    Some(match kind {
        PartDef => Part,
        PortDef => Port,
        ItemDef => Item,
        AttributeDef => Attribute,
        RequirementDef => Requirement,
        _ => return None,
    })
}
