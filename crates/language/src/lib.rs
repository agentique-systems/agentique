//! LanguageCore: a deliberate subset of SysML v2 as an in-memory element tree.
//!
//! - [`parse`] reads SysML text documents into a [`Tree`] and links its
//!   references: each [`Reference`] keeps the name as written and the
//!   [`ElementId`] it resolved to.
//! - [`link`] links references that are still unresolved (after edits).
//!   Linked references keep their targets across renames and moves.
//! - [`print`] writes a tree back as canonical SysML text (`doc` and `/* */`
//!   comments survive; `//` notes do not), naming each linked target so that
//!   the name resolves back to it.
//! - [`validate`] reports everything wrong with a tree as [`Diagnostic`]s:
//!   syntax errors, unsupported constructs, unresolved, ambiguous or removed
//!   targets, wrong kinds of type, bad redefinitions, connections whose ends
//!   do not fit, parts that contain themselves, and requirements satisfied by
//!   the wrong kind of subject.
//! - [`writable`] checks that an element built or changed in code can be
//!   printed and read back as the same element.
//! - [`Semantics`] answers questions with the same lookup and rules as
//!   `validate`: a definition's features (owned and inherited), types,
//!   generals, and whether two ports fit.
//!
//! Names are never identity. Inheritance is lookup: inherited features are
//! found through the generals, never copied. The supported subset is listed
//! in `docs/subset.md`, and departures from the standard in `docs/deviations.md`.
//!
//! ```
//! use agq_language::{parse, print, validate, Source};
//!
//! let text = "package P { part def A; part a : A; }";
//! let tree = parse(&[Source::new("p.sysml", text)]);
//! assert!(validate(&tree).is_empty());
//! assert!(print(&tree)[0].text.contains("part a : A;"));
//! ```
#![forbid(unsafe_code)]

mod expression;
mod lexer;
mod library;
mod parser;
mod printer;
mod resolve;
mod semantics;
mod tree;
mod validate;
mod writable;

pub use expression::{Argument, BinaryOp, Expression, UnaryOp};
pub use library::{LIBRARY_TEXT, library};
pub use parser::{Source, parse};
pub use printer::{print, print_element, print_expression, printed_reference};
pub use resolve::link;
pub use semantics::Semantics;
pub use tree::{
    Direction, Document, Element, ElementId, ElementKind, Literal, Location, Multiplicity, Parent,
    QualifiedName, Reference, Role, StateAction, Step, Tree, TreeError, Visibility,
};
pub use validate::{Diagnostic, validate};
pub use writable::{Field, writable};
