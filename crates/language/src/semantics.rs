//! Read-only questions about what a tree means, answered by the same lookup
//! and rules that [`validate`](crate::validate) uses, so that tools (the
//! Library, the Studio) never keep a second copy of a rule: the features a
//! definition or usage has (owned and inherited, found by lookup and never
//! copied), the types and generals of an element, and whether two ports fit
//! (the `incompatible-ends` rule, deviation 8).
//!
//! Build one [`Semantics`] for a tree and ask it many questions; it
//! remembers what it works out, so build a new one after the tree changes.

use crate::tree::{Element, ElementId, ElementKind, Tree};
use crate::validate::Checker;

/// Questions about one tree (and the built-in library).
pub struct Semantics<'a> {
    checker: Checker<'a>,
}

impl<'a> Semantics<'a> {
    pub fn new(tree: &'a Tree) -> Self {
        Semantics {
            checker: Checker::new(tree),
        }
    }

    /// The element, in the tree or in the built-in library.
    pub fn element(&self, id: ElementId) -> Option<&'a Element> {
        self.checker
            .model
            .exists(id)
            .then(|| self.checker.model.get(id))
    }

    /// Whether `id` belongs to the built-in library.
    pub fn is_library(&self, id: ElementId) -> bool {
        self.checker.model.is_library(id)
    }

    /// Every feature of a definition or usage that its users see: owned and
    /// inherited (through types, specialisations, subsetted and redefined
    /// features), in declaration order, with redefined ones replaced by
    /// their redefinitions.
    pub fn features(&self, namespace: ElementId) -> Vec<ElementId> {
        if !self.checker.model.exists(namespace) {
            return Vec::new();
        }
        self.checker.model.features(namespace)
    }

    /// The definitions typing a feature, each with whether the type is
    /// conjugated (`~P`): its own types, or else those of the features it
    /// subsets or redefines.
    pub fn types_of(&self, feature: ElementId) -> Vec<(ElementId, bool)> {
        if !self.checker.model.exists(feature) {
            return Vec::new();
        }
        self.checker.model.types_of(feature)
    }

    /// Direct generals: the definitions a definition specialises, or a
    /// usage's types, subsetted and redefined features.
    pub fn generals(&self, id: ElementId) -> Vec<ElementId> {
        if !self.checker.model.exists(id) {
            return Vec::new();
        }
        self.checker.model.generals(id).to_vec()
    }

    /// Whether `specific` is `general` or reaches it through its generals.
    pub fn specializes(&self, specific: ElementId, general: ElementId) -> bool {
        self.checker.model.exists(specific) && self.checker.model.specializes(specific, general)
    }

    /// Whether `feature` is owned by a general of `owner`, so that `owner`
    /// has it by inheritance rather than owning it.
    pub fn is_inherited(&self, owner: ElementId, feature: ElementId) -> bool {
        self.checker.model.exists(feature) && self.checker.model.is_inherited(owner, feature)
    }

    /// Whether ports `a` and `b` can be connected, by the rule `validate`
    /// applies to a connection between them: `Err` says why not, in plain
    /// words. The ports face each other, unless `passes_on` says that `a` is
    /// a port of the part that contains `b`'s part (`p` to `inner.p`), where
    /// directions stay the same.
    pub fn ports_fit(&self, a: ElementId, b: ElementId, passes_on: bool) -> Result<(), String> {
        for port in [a, b] {
            let kind = self.element(port).map(|e| e.kind);
            if kind != Some(ElementKind::Port) {
                return Err(format!("{port} is not a port"));
            }
        }
        match self.checker.port_mismatch(a, b, passes_on) {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }
}
