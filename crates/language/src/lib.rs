//! LanguageCore: a deliberate subset of SysML v2 as an in-memory element tree.
//!
//! - [`parse`] reads SysML text documents into a [`Tree`].
//! - [`print`] writes a tree back as canonical SysML text (`doc` comments
//!   survive; `//` notes do not).
//! - [`validate`] reports everything wrong with a tree as [`Diagnostic`]s:
//!   syntax errors, unsupported constructs, unresolved or ambiguous names,
//!   wrong kinds of type, bad redefinitions, connections whose ends do not
//!   fit, and requirements satisfied by the wrong kind of subject.
//!
//! Elements keep their [`ElementId`] across edits; names are never identity.
//! Inheritance is lookup: inherited features are found through the generals,
//! never copied. The supported subset is listed in `docs/subset.md`, and
//! departures from the standard in `docs/deviations.md`.
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

mod lexer;
mod library;
mod parser;
mod printer;
mod resolve;
mod tree;
mod validate;

pub use library::{LIBRARY_TEXT, library};
pub use parser::{Source, parse};
pub use printer::{print, print_element};
pub use tree::{
    Direction, Document, Element, ElementId, ElementKind, FeatureChain, Literal, Location,
    Multiplicity, Parent, QualifiedName, Tree, TypeRef, Visibility,
};
pub use validate::{Diagnostic, validate};
