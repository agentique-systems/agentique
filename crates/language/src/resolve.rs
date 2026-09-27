//! Name resolution and linking over the authored tree plus the built-in library.
//!
//! A name written in an element is looked up in its owner, then outward
//! through the enclosing namespaces, then in its document's top-level members
//! and imports, then among all top-level members and the library. In one
//! namespace, in order:
//! 1. owned members (these hide everything else),
//! 2. inherited members: members of the generals (types, specialised,
//!    subsetted and redefined elements), minus those redefined here,
//! 3. imported members.
//!
//! Inheritance is lookup; nothing is copied. Several different candidates are
//! an ambiguity error. Linked references are followed by identity; only
//! unlinked ones are resolved by name. Results are memoised for one pass.

use crate::library::library;
use crate::tree::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LookupError {
    NotFound(String),
    Ambiguous(String, Vec<ElementId>),
    /// A linked target that no longer exists.
    Removed(String),
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

/// Links every reference that has no target yet to the element its name
/// resolves to. Linked references keep their targets, so renaming or moving
/// the target never re-binds them by name. References that do not resolve
/// stay unlinked (and are reported by [`crate::validate`]); the next call
/// tries them again.
pub fn link(tree: &mut Tree) {
    let updates: Vec<(ElementId, usize, Vec<ElementId>)> = {
        let model = Model::new(tree, library());
        let mut updates = Vec::new();
        for id in tree.walk() {
            for (index, (role, reference)) in tree[id].references().into_iter().enumerate() {
                if reference.is_linked() {
                    continue;
                }
                if let Ok(targets) = model.resolve_by_name(id, role, reference) {
                    updates.push((id, index, targets));
                }
            }
        }
        updates
    };
    for (id, index, targets) in updates {
        let element = tree.get_mut(id).expect("walked element exists");
        let reference = element
            .references_mut()
            .into_iter()
            .nth(index)
            .expect("same reference order");
        for (step, target) in reference.steps.iter_mut().zip(targets) {
            step.target = Some(target);
        }
    }
}

pub(crate) struct Model<'a> {
    pub tree: &'a Tree,
    library: &'a Tree,
    /// Owned members by effective name; `None` is the top level.
    index: HashMap<Option<ElementId>, HashMap<&'a str, Vec<ElementId>>>,
    /// The document of each authored top-level element.
    root_document: HashMap<ElementId, usize>,
    /// Owners (`None` = top level) of at least one import. Most namespaces
    /// import nothing, and lookups skip them without reading their members.
    importers: HashSet<Option<ElementId>>,
    generals: RefCell<HashMap<ElementId, Rc<[ElementId]>>>,
    redefined: RefCell<HashMap<ElementId, Rc<[ElementId]>>>,
    redefined_here: RefCell<HashMap<ElementId, Rc<HashSet<ElementId>>>>,
    imports: RefCell<HashMap<ElementId, Option<ElementId>>>,
    /// Cycle guards: work in progress per element.
    computing: RefCell<Vec<ElementId>>,
    redefining: RefCell<Vec<ElementId>>,
    inheriting: RefCell<Vec<ElementId>>,
    importing: RefCell<Vec<ElementId>>,
}

impl<'a> Model<'a> {
    pub fn new(tree: &'a Tree, library: &'a Tree) -> Self {
        let mut index: HashMap<Option<ElementId>, HashMap<&'a str, Vec<ElementId>>> =
            HashMap::new();
        let mut importers = HashSet::new();
        for source in [tree, library] {
            for id in source.walk() {
                if source[id].kind == ElementKind::Import {
                    importers.insert(source[id].owner());
                }
                if let Some(name) = source.effective_name(id) {
                    index
                        .entry(source[id].owner())
                        .or_default()
                        .entry(name)
                        .or_default()
                        .push(id);
                }
            }
        }
        let root_document = tree
            .documents()
            .iter()
            .enumerate()
            .flat_map(|(i, d)| d.members().iter().map(move |m| (*m, i)))
            .collect();
        Model {
            tree,
            library,
            index,
            root_document,
            importers,
            generals: RefCell::default(),
            redefined: RefCell::default(),
            redefined_here: RefCell::default(),
            imports: RefCell::default(),
            computing: RefCell::default(),
            redefining: RefCell::default(),
            inheriting: RefCell::default(),
            importing: RefCell::default(),
        }
    }

    /// The element, authored or built in. Panics if it does not exist.
    pub fn get(&self, id: ElementId) -> &'a Element {
        match self.tree.get(id) {
            Some(element) => element,
            None => &self.library[id],
        }
    }

    pub fn exists(&self, id: ElementId) -> bool {
        self.tree.contains(id) || self.library.contains(id)
    }

    pub fn is_library(&self, id: ElementId) -> bool {
        !self.tree.contains(id) && self.library.contains(id)
    }

    /// The effective name of an element.
    pub fn name(&self, id: ElementId) -> Option<&'a str> {
        if self.is_library(id) {
            self.library.effective_name(id)
        } else {
            self.tree.effective_name(id)
        }
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

    // ---- references ----

    /// The elements a reference's steps point at: its linked targets, or for
    /// an unlinked reference, what its names resolve to now.
    pub fn resolve_reference(
        &self,
        holder: ElementId,
        role: Role,
        reference: &Reference,
    ) -> Result<Vec<ElementId>, LookupError> {
        if reference.steps.is_empty() {
            return Err(LookupError::NotFound(String::new()));
        }
        if !reference.is_linked() {
            return self.resolve_by_name(holder, role, reference);
        }
        reference
            .steps
            .iter()
            .map(|step| match step.target {
                Some(target) if self.exists(target) => Ok(target),
                _ => Err(LookupError::Removed(step.name.to_string())),
            })
            .collect()
    }

    /// The element a whole reference points at.
    pub fn target(
        &self,
        holder: ElementId,
        role: Role,
        reference: &Reference,
    ) -> Result<ElementId, LookupError> {
        let steps = self.resolve_reference(holder, role, reference)?;
        steps
            .last()
            .copied()
            .ok_or_else(|| LookupError::NotFound(String::new()))
    }

    /// What a reference's written names resolve to now, ignoring its links.
    pub fn resolve_by_name(
        &self,
        holder: ElementId,
        role: Role,
        reference: &Reference,
    ) -> Result<Vec<ElementId>, LookupError> {
        if reference.steps.is_empty() {
            return Err(LookupError::NotFound(String::new()));
        }
        let mut steps: Vec<ElementId> = Vec::with_capacity(reference.steps.len());
        for (i, step) in reference.steps.iter().enumerate() {
            let found = if i == 0 {
                match (role, self.get(holder).kind) {
                    (Role::Redefines, _) => self.resolve_redefined(holder, &step.name)?,
                    (Role::Target, ElementKind::Import) => {
                        self.resolve_from(holder, &step.name, false)?
                    }
                    _ => self.resolve(holder, &step.name)?,
                }
            } else if step.name.segments.len() > 1 {
                return Err(LookupError::Unsupported(format!(
                    "qualified name `{}` inside a feature chain",
                    step.name
                )));
            } else {
                let candidates =
                    self.features_named(steps[i - 1], step.name.last(), Access::Outside);
                self.one(step.name.last(), candidates)?
            };
            steps.push(found);
        }
        Ok(steps)
    }

    /// The text to print for a reference, and whether it leads back to the
    /// reference's targets from where it is written. Tried in order: as
    /// written, the same path with the targets' current names, the first
    /// target's qualified name. It does not lead back when the target is
    /// private, hidden by another element of the same name, or inside an
    /// unnamed element. Unlinked references and removed targets print as
    /// written.
    pub fn name_for(&self, holder: ElementId, role: Role, reference: &Reference) -> (String, bool) {
        let (printed, ok) = self.printed(holder, role, reference);
        (printed.to_string(), ok)
    }

    /// [`Model::name_for`] as a reference: the steps renamed as printed.
    pub fn printed(
        &self,
        holder: ElementId,
        role: Role,
        reference: &Reference,
    ) -> (Reference, bool) {
        let targets = match self.resolve_reference(holder, role, reference) {
            Ok(targets) if reference.is_linked() => targets,
            _ => return (reference.clone(), true),
        };
        let leads_back = |candidate: &Reference| {
            self.resolve_by_name(holder, role, candidate).as_ref() == Ok(&targets)
        };
        if leads_back(reference) {
            return (reference.clone(), true);
        }
        let mut renamed = reference.clone();
        for (step, target) in renamed.steps.iter_mut().zip(&targets) {
            if let (Some(last), Some(name)) = (step.name.segments.last_mut(), self.name(*target)) {
                *last = name.to_string();
            }
        }
        if leads_back(&renamed) {
            return (renamed, true);
        }
        if let Some(path) = self.path(targets[0]) {
            renamed.steps[0].name = QualifiedName::new(path);
        }
        let ok = leads_back(&renamed);
        (renamed, ok)
    }

    /// Effective names from the top level down to `id`.
    fn path(&self, id: ElementId) -> Option<Vec<String>> {
        let mut path = Vec::new();
        let mut current = Some(id);
        while let Some(next) = current {
            path.push(self.name(next)?.to_string());
            current = self.get(next).owner();
        }
        path.reverse();
        Some(path)
    }

    // ---- lookup by name ----

    /// Resolves `A::B::c` as written in element `holder`.
    pub fn resolve(
        &self,
        holder: ElementId,
        name: &QualifiedName,
    ) -> Result<ElementId, LookupError> {
        self.resolve_from(holder, name, true)
    }

    /// Like [`Model::resolve`]; without `own_imports` the imports of the
    /// namespace holding `holder` are not searched. An import's own name is
    /// resolved that way, so imports never depend on their siblings.
    fn resolve_from(
        &self,
        holder: ElementId,
        name: &QualifiedName,
        own_imports: bool,
    ) -> Result<ElementId, LookupError> {
        let (first, rest) = name
            .segments
            .split_first()
            .ok_or_else(|| LookupError::NotFound(String::new()))?;
        let mut found = self.lexical(holder, first, own_imports)?;
        for segment in rest {
            found = self.member(found, segment, holder)?;
        }
        Ok(found)
    }

    /// Resolves a name from the top level, e.g. `ScalarValues::String`.
    pub fn resolve_global(&self, name: &QualifiedName) -> Result<ElementId, LookupError> {
        let (first, rest) = name
            .segments
            .split_first()
            .ok_or_else(|| LookupError::NotFound(String::new()))?;
        let mut found = self.one(first, self.owned(None, first).to_vec())?;
        for segment in rest {
            let candidates = self.members_named(found, segment, Access::Outside);
            found = self.one(segment, candidates)?;
        }
        Ok(found)
    }

    fn member(
        &self,
        namespace: ElementId,
        segment: &str,
        holder: ElementId,
    ) -> Result<ElementId, LookupError> {
        let access = if self.encloses(namespace, holder) {
            Access::Inside
        } else {
            Access::Outside
        };
        self.one(segment, self.members_named(namespace, segment, access))
    }

    fn lexical(
        &self,
        holder: ElementId,
        name: &str,
        own_imports: bool,
    ) -> Result<ElementId, LookupError> {
        let first = self.get(holder).owner();
        let mut current = first;
        while let Some(namespace) = current {
            let found = if current == first && !own_imports {
                self.features_named(namespace, name, Access::Inside)
            } else {
                self.members_named(namespace, name, Access::Inside)
            };
            if !found.is_empty() {
                return self.one(name, found);
            }
            current = self.get(namespace).owner();
        }
        if let Some(document) = self.document(holder) {
            let owned: Vec<ElementId> = self
                .owned(None, name)
                .iter()
                .copied()
                .filter(|id| self.root_document.get(id) == Some(&document))
                .collect();
            if !owned.is_empty() {
                return self.one(name, owned);
            }
            if own_imports || first.is_some() {
                let members = self.tree.documents()[document].members();
                let imported = self.imported_by(members, None, name, Access::Inside);
                if !imported.is_empty() {
                    return self.one(name, imported);
                }
            }
        }
        self.one(name, self.owned(None, name).to_vec())
    }

    /// The document an authored element belongs to.
    fn document(&self, id: ElementId) -> Option<usize> {
        let mut root = id;
        while let Some(owner) = self.tree.get(root)?.owner() {
            root = owner;
        }
        self.root_document.get(&root).copied()
    }

    fn one(&self, name: &str, mut candidates: Vec<ElementId>) -> Result<ElementId, LookupError> {
        candidates.dedup();
        match candidates.len() {
            0 => Err(LookupError::NotFound(name.to_string())),
            1 => Ok(candidates[0]),
            _ => Err(LookupError::Ambiguous(name.to_string(), candidates)),
        }
    }

    /// Is `holder` inside `namespace`?
    fn encloses(&self, namespace: ElementId, holder: ElementId) -> bool {
        let mut current = self.get(holder).owner();
        while let Some(id) = current {
            if id == namespace {
                return true;
            }
            current = self.get(id).owner();
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
            .filter(|id| visible(self.get(*id).visibility, access))
            .collect();
        if !owned.is_empty() {
            return owned;
        }
        self.inherited_named(namespace, name, access)
    }

    /// Members of the generals with this name, minus features redefined in
    /// `namespace` and minus candidates redefined by another candidate.
    /// Outsiders see only public members; specialisations also protected ones.
    fn inherited_named(&self, namespace: ElementId, name: &str, access: Access) -> Vec<ElementId> {
        let kind = self.get(namespace).kind;
        if !(kind.is_definition() || kind.is_usage())
            || self.inheriting.borrow().contains(&namespace)
        {
            return Vec::new();
        }
        self.inheriting.borrow_mut().push(namespace);
        let through = if access == Access::Outside {
            Access::Outside
        } else {
            Access::Inherit
        };
        let redefined_here = self.redefined_here(namespace);
        let mut found: Vec<ElementId> = Vec::new();
        for general in self.generals(namespace).iter() {
            for candidate in self.features_named(*general, name, through) {
                if !redefined_here.contains(&candidate) && !found.contains(&candidate) {
                    found.push(candidate);
                }
            }
        }
        self.inheriting.borrow_mut().pop();
        self.without_redefined(found)
    }

    /// Drops candidates that another candidate redefines (inheritance diamonds).
    fn without_redefined(&self, mut candidates: Vec<ElementId>) -> Vec<ElementId> {
        if candidates.len() > 1 {
            let all = candidates.clone();
            candidates.retain(|c| {
                !all.iter()
                    .any(|other| other != c && self.redefines(*other, *c))
            });
        }
        candidates
    }

    /// Only for inherited lookup by the owner itself: features `owner` inherits, by name.
    pub fn inherited(&self, owner: ElementId, name: &str) -> Vec<ElementId> {
        self.inherited_named(owner, name, Access::Inherit)
    }

    fn imported_named(&self, namespace: ElementId, name: &str, access: Access) -> Vec<ElementId> {
        self.imported_by(
            self.get(namespace).children(),
            Some(namespace),
            name,
            access,
        )
    }

    /// What the imports among `members` (of namespace `origin`, if any)
    /// bring in under `name`. Wildcard imports lead on through the public
    /// imports of namespaces that have no such member themselves; each
    /// namespace is searched once, so import cycles end and the search stays
    /// linear.
    fn imported_by(
        &self,
        members: &'a [ElementId],
        origin: Option<ElementId>,
        name: &str,
        access: Access,
    ) -> Vec<ElementId> {
        // The members of one namespace or document share an owner.
        let imports = |members: &[ElementId]| {
            members
                .first()
                .is_some_and(|m| self.importers.contains(&self.get(*m).owner()))
        };
        if !imports(members) {
            return Vec::new();
        }
        let mut found = Vec::new();
        let mut searched: HashSet<ElementId> = origin.into_iter().collect();
        let mut queue = VecDeque::from([(members, access)]);
        while let Some((members, access)) = queue.pop_front() {
            if !imports(members) {
                continue;
            }
            for &import in members {
                let element = self.get(import);
                if element.kind != ElementKind::Import
                    || (access != Access::Inside && element.visibility != Visibility::Public)
                {
                    continue;
                }
                let Some(target) = self.import_target(import) else {
                    continue;
                };
                let brought = if !element.wildcard {
                    if self.name(target) == Some(name) {
                        vec![target]
                    } else {
                        Vec::new()
                    }
                } else if searched.insert(target) {
                    let features = self.features_named(target, name, Access::Outside);
                    if features.is_empty() {
                        queue.push_back((self.get(target).children(), Access::Outside));
                    }
                    features
                } else {
                    Vec::new()
                };
                for id in brought {
                    if !found.contains(&id) {
                        found.push(id);
                    }
                }
            }
        }
        found
    }

    /// The namespace or element an import names, if it resolves. Memoised;
    /// an import met again while its own name is being resolved counts as
    /// unresolved there.
    pub fn import_target(&self, import: ElementId) -> Option<ElementId> {
        if let Some(cached) = self.imports.borrow().get(&import) {
            return *cached;
        }
        if self.importing.borrow().contains(&import) {
            return None;
        }
        let reference = self.get(import).target.as_ref()?;
        let outermost = self.importing.borrow().is_empty();
        self.importing.borrow_mut().push(import);
        let target = self.target(import, Role::Target, reference).ok();
        self.importing.borrow_mut().pop();
        if outermost {
            self.imports.borrow_mut().insert(import, target);
        }
        target
    }

    // ---- generals and redefinition ----

    /// Direct generals: specialised definitions, or a usage's types,
    /// subsetted and redefined features. Unresolved references are skipped.
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
        for reference in &element.typed_by {
            out.extend(self.target(id, Role::TypedBy, reference).ok());
        }
        for reference in &element.specializes {
            out.extend(self.target(id, Role::Specializes, reference).ok());
        }
        out.extend(self.redefined(id).iter().copied());
        out.retain(|g| *g != id);
        self.computing.borrow_mut().pop();
        let out: Rc<[ElementId]> = out.into();
        self.generals.borrow_mut().insert(id, out.clone());
        out
    }

    /// The features `id` redefines: its `:>>` targets or, without any, the
    /// implied ones (derived, never written): an end redefines the end at
    /// the same position in each general of its owner, and a subject
    /// redefines the subject of each general requirement.
    pub fn redefined(&self, id: ElementId) -> Rc<[ElementId]> {
        if let Some(cached) = self.redefined.borrow().get(&id) {
            return cached.clone();
        }
        if self.redefining.borrow().contains(&id) {
            return Rc::from([]);
        }
        self.redefining.borrow_mut().push(id);
        let element = self.get(id);
        let mut targets: Vec<ElementId> = element
            .redefines
            .iter()
            .filter_map(|r| self.target(id, Role::Redefines, r).ok())
            .collect();
        if element.redefines.is_empty() {
            targets = self.implied_redefinitions(id);
        }
        self.redefining.borrow_mut().pop();
        let targets: Rc<[ElementId]> = targets.into();
        self.redefined.borrow_mut().insert(id, targets.clone());
        targets
    }

    fn implied_redefinitions(&self, id: ElementId) -> Vec<ElementId> {
        let element = self.get(id);
        let Some(owner) = element.owner() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if element.is_end {
            let position = self
                .get(owner)
                .children()
                .iter()
                .filter(|c| self.get(**c).is_end)
                .position(|c| *c == id);
            for general in self.generals(owner).iter() {
                out.extend(position.and_then(|p| self.ends(*general).get(p).copied()));
            }
        }
        if element.kind == ElementKind::Subject {
            for general in self.generals(owner).iter() {
                out.extend(self.subject(*general));
            }
        }
        out.retain(|t| *t != id);
        out
    }

    /// The end features of a connection or interface, in order.
    pub fn ends(&self, namespace: ElementId) -> Vec<ElementId> {
        self.features(namespace)
            .into_iter()
            .filter(|f| self.get(*f).is_end)
            .collect()
    }

    /// The subject of a requirement.
    pub fn subject(&self, requirement: ElementId) -> Option<ElementId> {
        self.features(requirement)
            .into_iter()
            .find(|f| self.get(*f).kind == ElementKind::Subject)
    }

    /// A redefinition by name must target a feature the owner inherits.
    fn resolve_redefined(
        &self,
        id: ElementId,
        name: &QualifiedName,
    ) -> Result<ElementId, LookupError> {
        let Some(owner) = self.get(id).owner() else {
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
            return self.one(name.last(), self.without_redefined(candidates));
        }
        let target = self.resolve(id, name)?;
        if self.is_inherited(owner, target) {
            Ok(target)
        } else {
            Err(LookupError::NotFound(name.to_string()))
        }
    }

    /// Is `feature` owned by a general of `owner` (so `owner` inherits it)?
    pub fn is_inherited(&self, owner: ElementId, feature: ElementId) -> bool {
        self.get(feature)
            .owner()
            .is_some_and(|o| o != owner && self.specializes(owner, o))
    }

    /// Features that the owned members of `namespace` redefine.
    fn redefined_here(&self, namespace: ElementId) -> Rc<HashSet<ElementId>> {
        if let Some(cached) = self.redefined_here.borrow().get(&namespace) {
            return cached.clone();
        }
        let set: HashSet<ElementId> = self
            .get(namespace)
            .children()
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
            let id = seen[i];
            let element = self.get(id);
            let types: Vec<(ElementId, bool)> = element
                .typed_by
                .iter()
                .filter_map(|r| self.target(id, Role::TypedBy, r).ok())
                .map(|def| (def, element.conjugated))
                .collect();
            if !types.is_empty() {
                return types;
            }
            for g in self.generals(id).iter() {
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
            for child in self.get(id).children() {
                if let Some(name) = self.name(*child)
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

fn visible(visibility: Visibility, access: Access) -> bool {
    match visibility {
        Visibility::Public => true,
        Visibility::Protected => access != Access::Outside,
        Visibility::Private => access == Access::Inside,
    }
}
