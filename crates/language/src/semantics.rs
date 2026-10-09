//! Read-only questions about what a tree means, answered by the same lookup
//! and rules that [`validate`](crate::validate) uses, so that tools (the
//! Library, the Studio) never keep a second copy of a rule: the features a
//! definition or usage has (owned and inherited, found by lookup and never
//! copied), the types and generals of an element, and whether two ports fit
//! (the `incompatible-ends` rule, deviation 8).
//!
//! Build one [`Semantics`] for a tree and ask it many questions; it
//! remembers what it works out, so build a new one after the tree changes.

use crate::resolve::LookupError;
use crate::tree::{
    Direction, Element, ElementId, ElementKind, QualifiedName, Reference, Role, Tree,
};
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

    /// The element a qualified name from the top level names, in the tree or
    /// in the built-in library, such as `Agents::Agent`.
    pub fn resolve(&self, qualified: &str) -> Option<ElementId> {
        let name = QualifiedName::new(qualified.split("::"));
        self.checker.model.resolve_global(&name).ok()
    }

    /// The elements each step of a reference held by `holder` points at: its
    /// linked targets, or what its names resolve to now. `Err` says why not,
    /// in plain words.
    pub fn steps(
        &self,
        holder: ElementId,
        role: Role,
        reference: &Reference,
    ) -> Result<Vec<ElementId>, String> {
        if !self.checker.model.exists(holder) {
            return Err(format!("{holder} does not exist"));
        }
        self.checker
            .model
            .resolve_reference(holder, role, reference)
            .map_err(|error| match error {
                LookupError::NotFound(name) => format!("cannot find `{name}`"),
                LookupError::Ambiguous(name, _) => format!("`{name}` is ambiguous"),
                LookupError::Removed(name) => format!("`{name}` no longer exists"),
                LookupError::Unsupported(what) => format!("{what} is not supported"),
            })
    }

    /// The features `feature` redefines: its `:>>` targets or, without any,
    /// the implied ones.
    pub fn redefined(&self, feature: ElementId) -> Vec<ElementId> {
        if !self.checker.model.exists(feature) {
            return Vec::new();
        }
        self.checker.model.redefined(feature).to_vec()
    }

    /// Whether a part or item usage refers to its value instead of
    /// containing it (SysML 7.6.3): `Some(true)` when it is referential
    /// itself (`ref`, no kind keyword, directed, an `end`, or owned by a
    /// package) and so is every part or item usage it redefines, directly
    /// or indirectly (a redefinition has the values of what it redefines);
    /// `Some(false)` for a composite part or item; `None` for anything
    /// else ([`Semantics::part_kind`]). The rule `validate` checks by.
    pub fn referential(&self, feature: ElementId) -> Option<bool> {
        if !self.checker.model.exists(feature) {
            return None;
        }
        self.checker
            .part_binding(feature)
            .map(|(referential, _)| referential)
    }

    /// `Part` or `Item` for a part or item usage, or for a usage without a
    /// kind keyword that redefines one; `None` for anything else.
    pub fn part_kind(&self, feature: ElementId) -> Option<ElementKind> {
        if !self.checker.model.exists(feature) {
            return None;
        }
        self.checker.part_kind(feature)
    }

    /// The usage whose value `feature` has: itself when it has a value,
    /// else the nearest feature it redefines that has one. For a reference,
    /// the value is the part it is bound to; `None` means it is not bound.
    pub fn value_holder(&self, feature: ElementId) -> Option<ElementId> {
        if !self.checker.model.exists(feature) {
            return None;
        }
        self.checker.value_holder(feature)
    }

    /// The directed features of a port as seen from outside it, with its
    /// conjugation applied: (name, direction, type).
    pub fn directed_features(
        &self,
        port: ElementId,
    ) -> Vec<(String, Direction, Option<ElementId>)> {
        if !self.checker.model.exists(port) {
            return Vec::new();
        }
        self.checker.directed_features(port)
    }

    /// The effective name of an element, in the tree or the library.
    pub fn name(&self, id: ElementId) -> Option<&'a str> {
        self.checker
            .model
            .exists(id)
            .then(|| self.checker.model.name(id))
            .flatten()
    }

    /// `Package::Definition::feature`, for messages, in the tree or the library.
    pub fn qualified_name(&self, id: ElementId) -> String {
        if !self.checker.model.exists(id) {
            return id.to_string();
        }
        self.checker.model.describe(id)
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
