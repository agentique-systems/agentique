//! Name resolution over the authored tree plus the built-in library.
//!
//! Lookup rules, in order, for a name in a namespace:
//! 1. owned members (these hide everything else),
//! 2. inherited members: members of the generals (types, specialised,
//!    subsetted and redefined elements), minus those redefined here,
//! 3. imported members.
//!
//! Inheritance is lookup; nothing is copied. Results are memoised for one
//! validation run only. Several different candidates are an ambiguity error.

use crate::tree::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LookupError {
    NotFound(String),
    Ambiguous(String, Vec<ElementId>),
    /// A reference form outside the subset, e.g. a qualified chain step.
    Unsupported(String),
}

/// Who is looking: code inside the namespace, a specialisation, or outsiders.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    Inside,
    Inherit,
    Outside,
}

pub(crate) struct Model<'a> {
    pub tree: &'a Tree,
    library: &'a Tree,
    /// Owned members by effective name; `None` is the global namespace.
    index: HashMap<Option<ElementId>, HashMap<&'a str, Vec<ElementId>>>,
    generals: RefCell<HashMap<ElementId, Rc<[ElementId]>>>,
    redefined: RefCell<HashMap<ElementId, Rc<[ElementId]>>>,
    redefined_here: RefCell<HashMap<ElementId, Rc<HashSet<ElementId>>>>,
    imports: RefCell<HashMap<ElementId, Option<ElementId>>>,
    /// Elements whose generals are being computed (cycle guard).
    computing: RefCell<Vec<ElementId>>,
    /// Namespaces whose inherited members are being looked up (cycle guard).
    inheriting: RefCell<Vec<ElementId>>,
    /// Imports whose targets are being resolved (cycle guard).
    importing: RefCell<Vec<ElementId>>,
}

impl<'a> Model<'a> {
    pub fn new(tree: &'a Tree, library: &'a Tree) -> Self {
        let mut index: HashMap<Option<ElementId>, HashMap<&'a str, Vec<ElementId>>> =
            HashMap::new();
        for source in [tree, library] {
            for id in source.walk() {
                let element = &source[id];
                if let Some(name) = element.effective_name() {
                    index
                        .entry(element.owner)
                        .or_default()
                        .entry(name)
                        .or_default()
                        .push(id);
                }
            }
        }
        Model {
            tree,
            library,
            index,
            generals: RefCell::default(),
            redefined: RefCell::default(),
            redefined_here: RefCell::default(),
            imports: RefCell::default(),
            computing: RefCell::default(),
            inheriting: RefCell::default(),
            importing: RefCell::default(),
        }
    }

    pub fn get(&self, id: ElementId) -> &'a Element {
        match self.tree.get(id) {
            Some(element) => element,
            None => &self.library[id],
        }
    }

    pub fn is_library(&self, id: ElementId) -> bool {
        !self.tree.contains(id)
    }

    /// The qualified name, for messages.
    pub fn describe(&self, id: ElementId) -> String {
        if self.is_library(id) {
            self.library.qualified_name(id)
        } else {
            self.tree.qualified_name(id)
        }
    }

    /// Owned members with the same effective name, per namespace (`None` = top level).
    pub fn duplicates(&self) -> impl Iterator<Item = &[ElementId]> {
        self.index
            .values()
            .flat_map(|names| names.values())
            .filter(|ids| ids.len() > 1)
            .map(Vec::as_slice)
    }

    // ---- lookup ----

    /// Resolves `A::B::c` as written in namespace `scope` (`None` = top level).
    pub fn resolve(
        &self,
        scope: Option<ElementId>,
        name: &QualifiedName,
    ) -> Result<ElementId, LookupError> {
        let (first, rest) = name
            .segments
            .split_first()
            .ok_or_else(|| LookupError::NotFound(String::new()))?;
        let mut found = self.lexical(scope, first)?;
        for segment in rest {
            let access = if self.encloses(found, scope) {
                Access::Inside
            } else {
                Access::Outside
            };
            let candidates = self.members_named(found, segment, access);
            found = self.one(segment, candidates)?;
        }
        Ok(found)
    }

    /// Resolves `a.b.c`: the first step as a name, each further step as a
    /// feature of the previous one. Returns the element of every step.
    pub fn resolve_chain(
        &self,
        scope: Option<ElementId>,
        chain: &FeatureChain,
    ) -> Result<Vec<ElementId>, LookupError> {
        let mut steps = Vec::with_capacity(chain.steps.len());
        for (i, step) in chain.steps.iter().enumerate() {
            let found = if i == 0 {
                self.resolve(scope, step)?
            } else if step.segments.len() > 1 {
                return Err(LookupError::Unsupported(format!(
                    "qualified name `{step}` inside a feature chain"
                )));
            } else {
                let previous = steps[i - 1];
                let candidates = self.features_named(previous, step.last(), Access::Outside);
                self.one(step.last(), candidates)?
            };
            steps.push(found);
        }
        Ok(steps)
    }

    fn lexical(&self, scope: Option<ElementId>, name: &str) -> Result<ElementId, LookupError> {
        let mut current = scope;
        while let Some(namespace) = current {
            let found = self.members_named(namespace, name, Access::Inside);
            if !found.is_empty() {
                return self.one(name, found);
            }
            current = self.get(namespace).owner;
        }
        let global = self.owned(None, name).to_vec();
        self.one(name, global)
    }

    fn one(&self, name: &str, mut candidates: Vec<ElementId>) -> Result<ElementId, LookupError> {
        candidates.dedup();
        match candidates.len() {
            0 => Err(LookupError::NotFound(name.to_string())),
            1 => Ok(candidates[0]),
            _ => Err(LookupError::Ambiguous(name.to_string(), candidates)),
        }
    }

    /// Is `scope` the namespace `namespace` or inside it?
    fn encloses(&self, namespace: ElementId, scope: Option<ElementId>) -> bool {
        let mut current = scope;
        while let Some(id) = current {
            if id == namespace {
                return true;
            }
            current = self.get(id).owner;
        }
        false
    }

    fn owned(&self, namespace: Option<ElementId>, name: &str) -> &[ElementId] {
        self.index
            .get(&namespace)
            .and_then(|names| names.get(name))
            .map_or(&[], Vec::as_slice)
    }

    fn members_named(&self, namespace: ElementId, name: &str, access: Access) -> Vec<ElementId> {
        let found = self.features_named(namespace, name, access);
        if !found.is_empty() {
            return found;
        }
        self.imported_named(namespace, name, access)
    }

    /// Owned members, or else inherited ones (never imported).
    fn features_named(&self, namespace: ElementId, name: &str, access: Access) -> Vec<ElementId> {
        let owned: Vec<ElementId> = self
            .owned(Some(namespace), name)
            .iter()
            .copied()
            .filter(|id| match self.get(*id).visibility {
                Visibility::Public => true,
                Visibility::Protected => access != Access::Outside,
                Visibility::Private => access == Access::Inside,
            })
            .collect();
        if !owned.is_empty() {
            return owned;
        }
        self.inherited_named(namespace, name)
    }

    /// Members of the generals with this name, minus features redefined in
    /// `namespace` and minus candidates redefined by another candidate.
    pub(crate) fn inherited_named(&self, namespace: ElementId, name: &str) -> Vec<ElementId> {
        let kind = self.get(namespace).kind;
        if !(kind.is_definition() || kind.is_usage()) {
            return Vec::new();
        }
        if self.inheriting.borrow().contains(&namespace) {
            return Vec::new(); // specialisation cycle; reported by validation
        }
        self.inheriting.borrow_mut().push(namespace);
        let redefined_here = self.redefined_here(namespace);
        let mut found: Vec<ElementId> = Vec::new();
        for general in self.generals(namespace).iter() {
            for candidate in self.features_named(*general, name, Access::Inherit) {
                if !redefined_here.contains(&candidate) && !found.contains(&candidate) {
                    found.push(candidate);
                }
            }
        }
        if found.len() > 1 {
            let all = found.clone();
            found.retain(|c| {
                !all.iter()
                    .any(|other| other != c && self.redefines(*other, *c))
            });
        }
        self.inheriting.borrow_mut().pop();
        found
    }

    fn imported_named(&self, namespace: ElementId, name: &str, access: Access) -> Vec<ElementId> {
        let mut found = Vec::new();
        for &import in &self.get(namespace).children {
            let element = self.get(import);
            if element.kind != ElementKind::Import
                || (access != Access::Inside && element.visibility != Visibility::Public)
            {
                continue;
            }
            let Some(target) = self.import_target(import) else {
                continue;
            };
            if element.wildcard {
                for id in self.members_named(target, name, Access::Outside) {
                    if !found.contains(&id) {
                        found.push(id);
                    }
                }
            } else if self.get(target).effective_name() == Some(name) && !found.contains(&target) {
                found.push(target);
            }
        }
        found
    }

    /// The namespace or element an import names, if it resolves.
    pub fn import_target(&self, import: ElementId) -> Option<ElementId> {
        self.resolve_import(import).ok()
    }

    pub fn resolve_import(&self, import: ElementId) -> Result<ElementId, LookupError> {
        if let Some(cached) = self.imports.borrow().get(&import) {
            return cached.ok_or_else(|| LookupError::NotFound(String::new()));
        }
        if self.importing.borrow().contains(&import) {
            return Err(LookupError::NotFound(String::new()));
        }
        let element = self.get(import);
        let Some(target) = &element.target else {
            return Err(LookupError::NotFound(String::new()));
        };
        let outermost = self.importing.borrow().is_empty();
        self.importing.borrow_mut().push(import);
        let result = self.resolve(element.owner, target);
        self.importing.borrow_mut().pop();
        if outermost {
            self.imports
                .borrow_mut()
                .insert(import, result.clone().ok());
        }
        result
    }

    // ---- generals ----

    /// Direct generals: specialised definitions, or a usage's types,
    /// subsetted and redefined features. Unresolved names are skipped.
    pub fn generals(&self, id: ElementId) -> Rc<[ElementId]> {
        if let Some(cached) = self.generals.borrow().get(&id) {
            return cached.clone();
        }
        if self.computing.borrow().contains(&id) {
            return Rc::from([]); // a name in the declaration needs the declaration itself
        }
        self.computing.borrow_mut().push(id);
        let element = self.get(id);
        let mut out: Vec<ElementId> = Vec::new();
        for ty in &element.typed_by {
            out.extend(self.resolve(element.owner, &ty.name).ok());
        }
        for general in &element.specializes {
            out.extend(self.resolve(element.owner, general).ok());
        }
        out.extend(self.redefined(id).iter().copied());
        out.retain(|g| *g != id);
        self.computing.borrow_mut().pop();
        let out: Rc<[ElementId]> = out.into();
        self.generals.borrow_mut().insert(id, out.clone());
        out
    }

    /// Resolved `:>>` targets of `id`.
    pub fn redefined(&self, id: ElementId) -> Rc<[ElementId]> {
        if let Some(cached) = self.redefined.borrow().get(&id) {
            return cached.clone();
        }
        let targets: Vec<ElementId> = self
            .get(id)
            .redefines
            .iter()
            .filter_map(|name| self.resolve_redefined(id, name).ok())
            .collect();
        let targets: Rc<[ElementId]> = targets.into();
        self.redefined.borrow_mut().insert(id, targets.clone());
        targets
    }

    /// A redefined feature must be inherited by the redefining feature's owner.
    pub fn resolve_redefined(
        &self,
        id: ElementId,
        name: &QualifiedName,
    ) -> Result<ElementId, LookupError> {
        let Some(owner) = self.get(id).owner else {
            return Err(LookupError::NotFound(name.to_string()));
        };
        if name.segments.len() == 1 {
            let mut candidates = Vec::new();
            for general in self.generals(owner).iter() {
                for c in self.features_named(*general, name.last(), Access::Inherit) {
                    if !candidates.contains(&c) {
                        candidates.push(c);
                    }
                }
            }
            if candidates.len() > 1 {
                let all = candidates.clone();
                candidates.retain(|c| !all.iter().any(|o| o != c && self.redefines(*o, *c)));
            }
            return self.one(name.last(), candidates);
        }
        let target = self.resolve(Some(owner), name)?;
        let target_owner = self.get(target).owner;
        let inherited = target_owner.is_some_and(|o| o != owner && self.specializes(owner, o));
        if inherited {
            Ok(target)
        } else {
            Err(LookupError::NotFound(name.to_string()))
        }
    }

    /// Features that the owned members of `namespace` redefine.
    fn redefined_here(&self, namespace: ElementId) -> Rc<HashSet<ElementId>> {
        if let Some(cached) = self.redefined_here.borrow().get(&namespace) {
            return cached.clone();
        }
        let set: HashSet<ElementId> = self
            .get(namespace)
            .children
            .iter()
            .flat_map(|child| self.redefined(*child).to_vec())
            .collect();
        let set = Rc::new(set);
        self.redefined_here
            .borrow_mut()
            .insert(namespace, set.clone());
        set
    }

    /// Does `feature` redefine `target`, directly or through other redefinitions?
    fn redefines(&self, feature: ElementId, target: ElementId) -> bool {
        let mut seen = HashSet::new();
        let mut stack = vec![feature];
        while let Some(next) = stack.pop() {
            if !seen.insert(next) {
                continue;
            }
            for redefined in self.redefined(next).iter() {
                if *redefined == target {
                    return true;
                }
                stack.push(*redefined);
            }
        }
        false
    }

    /// Is `general` reachable from `specific` through generals (or equal)?
    pub fn specializes(&self, specific: ElementId, general: ElementId) -> bool {
        self.all_generals(specific).contains(&general)
    }

    /// `id` and all its generals, transitively. Safe on cycles.
    pub fn all_generals(&self, id: ElementId) -> Vec<ElementId> {
        let mut seen = vec![id];
        let mut i = 0;
        while i < seen.len() {
            for g in self.generals(seen[i]).iter() {
                if !seen.contains(g) {
                    seen.push(*g);
                }
            }
            i += 1;
        }
        seen
    }

    /// Is `id` among its own generals (a specialisation cycle)?
    pub fn in_cycle(&self, id: ElementId) -> bool {
        self.generals(id)
            .iter()
            .any(|g| self.all_generals(*g).contains(&id))
    }

    /// The definitions typing a feature, with the conjugation flag: its own
    /// typings, or else those of the features it subsets or redefines.
    pub fn types_of(&self, feature: ElementId) -> Vec<(ElementId, bool)> {
        let mut seen = vec![feature];
        let mut i = 0;
        while i < seen.len() {
            let element = self.get(seen[i]);
            let types: Vec<(ElementId, bool)> = element
                .typed_by
                .iter()
                .filter_map(|t| {
                    let def = self.resolve(element.owner, &t.name).ok()?;
                    Some((def, t.conjugated))
                })
                .collect();
            if !types.is_empty() {
                return types;
            }
            for g in self.generals(seen[i]).iter() {
                if self.get(*g).kind.is_usage() && !seen.contains(g) {
                    seen.push(*g);
                }
            }
            i += 1;
        }
        Vec::new()
    }

    /// All features of a namespace visible to users of it: owned and inherited,
    /// in declaration order, with redefined ones replaced.
    pub fn features(&self, namespace: ElementId) -> Vec<ElementId> {
        let mut names: Vec<&str> = Vec::new();
        for id in self.all_generals(namespace) {
            for child in &self.get(id).children {
                if let Some(name) = self.get(*child).effective_name()
                    && !names.contains(&name)
                {
                    names.push(name);
                }
            }
        }
        names
            .into_iter()
            .filter_map(|name| {
                let found = self.features_named(namespace, name, Access::Outside);
                (found.len() == 1).then(|| found[0])
            })
            .filter(|id| self.get(*id).kind.is_usage())
            .collect()
    }
}
