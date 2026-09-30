//! Copying a block's definitions from one tree into another: the block's
//! dependency closure, the comparison that finds identical copies already
//! there, and the plan of new elements with every reference pointing at its
//! target in the destination by identity (never re-bound by name).
//!
//! A plan predicts the ids the destination hands out: a tree gives each new
//! element the next id, so the n-th new element of a change gets
//! `next_id + n`. The plan's creations must be applied in order, all of
//! them, to a destination at the revision the plan was made for.

use agq_language::{Element, ElementId, ElementKind, Parent, Reference, Tree};
use std::collections::HashMap;
use std::fmt;

/// A reference inside a block that leads nowhere: the block cannot be used
/// until it is fixed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    /// The element holding the reference, by qualified name.
    pub element: String,
    /// The reference as written.
    pub reference: String,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` refers to `{}`, which cannot be found",
            self.element, self.reference
        )
    }
}

/// A definition the destination already holds under the same qualified
/// name, with different content. Nothing is overwritten: the Operator
/// chooses to use the destination's definition or to copy under another
/// name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub qualified_name: String,
    /// The destination's element.
    pub existing: ElementId,
    /// The first difference, in plain words.
    pub difference: String,
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` already exists with different content: {}",
            self.qualified_name, self.difference
        )
    }
}

/// What to do when the destination already has a different definition with
/// the same qualified name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Resolution {
    /// Stop and report the conflicts.
    #[default]
    Ask,
    /// Use the destination's definition as it is.
    UseExisting,
    /// Copy the block's definition under an unused name beside it.
    Rename,
}

/// The element directly under a package (or at the top level) that holds
/// `id`: the unit that is copied whole.
pub(crate) fn unit_of(tree: &Tree, mut id: ElementId) -> ElementId {
    while let Some(owner) = tree.get(id).and_then(Element::owner) {
        if tree[owner].kind == ElementKind::Package {
            break;
        }
        id = owner;
    }
    id
}

/// The units a block needs besides the standard library: the block's own
/// unit and, transitively, every unit its elements refer to, in the order
/// found. References that lead nowhere are reported.
pub fn closure(tree: &Tree, block: ElementId) -> Result<Vec<ElementId>, Vec<Missing>> {
    let mut units = vec![unit_of(tree, block)];
    let mut missing = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        for id in tree.descendants(unit) {
            for (_, reference) in tree[id].references() {
                for step in &reference.steps {
                    let Some(target) = step.target else {
                        missing.push(Missing {
                            element: tree.qualified_name(id),
                            reference: reference.to_string(),
                        });
                        break;
                    };
                    if is_standard(target) {
                        continue;
                    }
                    if !tree.contains(target) {
                        missing.push(Missing {
                            element: tree.qualified_name(id),
                            reference: reference.to_string(),
                        });
                        break;
                    }
                    let needed = unit_of(tree, target);
                    if !units.contains(&needed) {
                        units.push(needed);
                    }
                }
            }
        }
        i += 1;
    }
    if missing.is_empty() {
        Ok(units)
    } else {
        missing.dedup();
        Err(missing)
    }
}

/// Ids of the language's standard library (`ScalarValues`): referred to,
/// never copied.
pub(crate) fn is_standard(id: ElementId) -> bool {
    agq_language::library().contains(id)
}

/// Names from the top level down to `id`; an unnamed element is `#n`, its
/// position among its owner's members.
pub(crate) fn path_of(tree: &Tree, id: ElementId) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = Some(id);
    while let Some(here) = current {
        let owner = tree.get(here).and_then(Element::owner);
        path.push(match tree.effective_name(here) {
            Some(name) => name.to_string(),
            None => {
                let siblings = match owner {
                    Some(o) => tree[o].children(),
                    None => &[],
                };
                format!("#{}", siblings.iter().position(|s| *s == here).unwrap_or(0))
            }
        });
        current = owner;
    }
    path.reverse();
    path
}

/// The element at `path`, following owned members only.
pub(crate) fn find_path(tree: &Tree, path: &[String]) -> Option<ElementId> {
    let mut candidates: Vec<ElementId> = tree.roots().collect();
    let mut found = None;
    for segment in path {
        let here = match segment
            .strip_prefix('#')
            .and_then(|n| n.parse::<usize>().ok())
        {
            Some(position) => candidates.get(position).copied(),
            None => candidates
                .iter()
                .copied()
                .find(|c| tree.effective_name(*c) == Some(segment.as_str())),
        }?;
        found = Some(here);
        candidates = tree[here].children().to_vec();
    }
    found
}

/// The first difference in content between `a` (in `a_tree`) and `b` (in
/// `b_tree`), or `None` when they are the same. References are compared by
/// where their targets are (`target_path` gives the path an `a_tree` target
/// has in `b_tree`), standard-library targets by identity, unlinked ones by
/// the text written. Members are compared in order. Source locations and
/// the names references were written with do not count.
pub(crate) fn difference_placed(
    a_tree: &Tree,
    a: ElementId,
    b_tree: &Tree,
    b: ElementId,
    target_path: &dyn Fn(ElementId) -> Vec<String>,
) -> Option<String> {
    let (x, y) = (&a_tree[a], &b_tree[b]);
    let label = || {
        a_tree
            .effective_name(a)
            .map(|n| format!("`{n}`"))
            .unwrap_or_else(|| format!("a {}", x.kind.keyword()))
    };
    if x.kind != y.kind {
        return Some(format!(
            "{} is a {} here and a {} there",
            label(),
            x.kind.keyword(),
            y.kind.keyword()
        ));
    }
    if a_tree.effective_name(a) != b_tree.effective_name(b) {
        return Some(format!(
            "`{}` is named `{}` there",
            a_tree.effective_name(a).unwrap_or("(unnamed)"),
            b_tree.effective_name(b).unwrap_or("(unnamed)")
        ));
    }
    let flags = |e: &Element| {
        (
            e.visibility,
            e.is_abstract,
            e.is_end,
            e.direction,
            e.conjugated,
            e.multiplicity,
            e.wildcard,
        )
    };
    if flags(x) != flags(y) {
        return Some(format!("{} is declared differently", label()));
    }
    if x.value != y.value {
        let show = |e: &Element| {
            e.value
                .as_ref()
                .map_or("no value".into(), |v| format!("{v}"))
        };
        return Some(format!("{} is {} instead of {}", label(), show(y), show(x)));
    }
    if x.text != y.text || x.note != y.note {
        return Some(match x.kind {
            ElementKind::Doc | ElementKind::Comment => "its documentation differs".to_string(),
            _ => format!("{} is written differently", label()),
        });
    }
    let refs_a = x.references();
    let refs_b = y.references();
    if refs_a.len() != refs_b.len() {
        return Some(format!("{} refers to other elements", label()));
    }
    for ((role_a, ra), (role_b, rb)) in refs_a.iter().zip(&refs_b) {
        if role_a != role_b || ra.steps.len() != rb.steps.len() {
            return Some(format!("{} refers to other elements", label()));
        }
        for (sa, sb) in ra.steps.iter().zip(&rb.steps) {
            let same = match (sa.target, sb.target) {
                (Some(ta), Some(tb)) if is_standard(ta) || is_standard(tb) => ta == tb,
                (Some(ta), Some(tb)) => {
                    b_tree.contains(tb) && target_path(ta) == path_of(b_tree, tb)
                }
                (None, None) => sa.name == sb.name,
                _ => false,
            };
            if !same {
                return Some(format!("{} refers to `{rb}` there, not `{ra}`", label()));
            }
        }
    }
    let (ca, cb) = (x.children(), y.children());
    for (child_a, child_b) in ca.iter().zip(cb) {
        if let Some(difference) = difference_placed(a_tree, *child_a, b_tree, *child_b, target_path)
        {
            return Some(difference);
        }
    }
    match ca.len().cmp(&cb.len()) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(format!(
            "{} has {} here that it lacks there",
            label(),
            describe_member(a_tree, ca[cb.len()])
        )),
        std::cmp::Ordering::Less => Some(format!(
            "{} has {} there",
            label(),
            describe_member(b_tree, cb[ca.len()])
        )),
    }
}

/// The first difference between two elements at the same place in two
/// trees (targets compared by their qualified paths).
pub(crate) fn difference(
    a_tree: &Tree,
    a: ElementId,
    b_tree: &Tree,
    b: ElementId,
) -> Option<String> {
    difference_placed(a_tree, a, b_tree, b, &|t| path_of(a_tree, t))
}

fn describe_member(tree: &Tree, id: ElementId) -> String {
    let element = &tree[id];
    match tree.effective_name(id) {
        Some(name) => format!("{} `{name}`", element.kind.keyword()),
        None if element.kind == ElementKind::Doc => "documentation".into(),
        None => format!("a {}", element.kind.keyword()),
    }
}

/// Where each new element goes: an existing element or document of the
/// destination, or an element created earlier in the plan (by its predicted
/// id, which is an ordinary id of the destination once applied).
pub(crate) struct Creation {
    pub parent: Parent,
    pub element: Element,
}

/// A copy of a block's units into a destination tree.
pub(crate) struct CopyPlan {
    /// New elements in the order they must be created.
    pub creations: Vec<Creation>,
    /// Every source element (of the copied units) to its destination element.
    pub map: HashMap<ElementId, ElementId>,
    /// Destination qualified names of the units created, reused and renamed.
    pub created: Vec<String>,
    pub reused: Vec<String>,
    pub renamed: Vec<(String, String)>,
}

/// How a unit is placed: where it goes in the destination.
pub(crate) type Placement<'a> = &'a dyn Fn(ElementId) -> Vec<String>;

/// Plans copying `units` of `source` into `dest`. Each unit goes to the
/// path `place` gives it: an identical element already there is reused, a
/// different one is a conflict (settled by `resolution`), a missing one is
/// created, with any missing packages above it. References to copied or
/// reused elements point at them by identity; references to the standard
/// library stay as they are.
pub(crate) fn plan_copy(
    source: &Tree,
    units: &[ElementId],
    dest: &Tree,
    place: Placement,
    resolution: Resolution,
) -> Result<CopyPlan, CopyError> {
    let placed: HashMap<ElementId, Vec<String>> = units.iter().map(|u| (*u, place(*u))).collect();
    // The path a source element will have in the destination.
    let target_path = |target: ElementId| -> Vec<String> {
        let unit = unit_of(source, target);
        match placed.get(&unit) {
            Some(path) => {
                let mut full = path.clone();
                let own = path_of(source, unit).len();
                full.extend(path_of(source, target).into_iter().skip(own));
                full
            }
            None => path_of(source, target),
        }
    };
    let mut map = HashMap::new();
    let mut plan = CopyPlan {
        creations: Vec::new(),
        map: HashMap::new(),
        created: Vec::new(),
        reused: Vec::new(),
        renamed: Vec::new(),
    };
    let mut conflicts = Vec::new();
    // Units to create, with the name to create them under.
    let mut to_create: Vec<(ElementId, Vec<String>)> = Vec::new();
    for &unit in units {
        let path = placed[&unit].clone();
        let existing = find_path(dest, &path);
        let difference =
            existing.map(|e| (e, difference_placed(source, unit, dest, e, &target_path)));
        match difference {
            None => to_create.push((unit, path)),
            Some((existing, None)) => {
                map_same(source, unit, dest, existing, &mut map);
                plan.reused.push(path.join("::"));
            }
            Some((existing, Some(difference))) => match resolution {
                Resolution::Ask => conflicts.push(Conflict {
                    qualified_name: path.join("::"),
                    existing,
                    difference,
                }),
                Resolution::UseExisting if dest[existing].kind == source[unit].kind => {
                    map_by_name(source, unit, dest, existing, &mut map);
                    plan.reused.push(path.join("::"));
                }
                Resolution::UseExisting | Resolution::Rename => {
                    let (parent, name) = path.split_at(path.len() - 1);
                    let taken = |candidate: &str| {
                        let mut full = parent.to_vec();
                        full.push(candidate.to_string());
                        find_path(dest, &full).is_some()
                            || to_create.iter().any(|(_, p)| *p == full)
                    };
                    let new_name = (2..)
                        .map(|n| format!("{}{n}", name[0]))
                        .find(|candidate| !taken(candidate))
                        .expect("an unused name exists");
                    let mut renamed = parent.to_vec();
                    renamed.push(new_name);
                    plan.renamed.push((path.join("::"), renamed.join("::")));
                    to_create.push((unit, renamed));
                }
            },
        }
    }
    if !conflicts.is_empty() {
        return Err(CopyError::Conflicts(conflicts));
    }
    // Allocate ids in creation order: missing packages as first needed,
    // then each unit's elements, owners before members.
    enum Slot {
        Package(String),
        Copy(ElementId),
    }
    let mut next = dest.next_id().raw();
    let mut slots: Vec<(Parent, Slot, ElementId)> = Vec::new();
    let mut packages: HashMap<Vec<String>, ElementId> = HashMap::new();
    let mut renames: HashMap<ElementId, String> = HashMap::new();
    for (unit, path) in &to_create {
        let mut parent = Parent::Document(0);
        for depth in 1..path.len() {
            let prefix = path[..depth].to_vec();
            let found = packages
                .get(&prefix)
                .copied()
                .or_else(|| find_path(dest, &prefix));
            let package = match found {
                Some(existing) => {
                    if dest
                        .get(existing)
                        .is_some_and(|e| !agq_language::Field::Members.fits(e.kind))
                    {
                        return Err(CopyError::Invalid(format!(
                            "`{}` is a {}, which cannot hold `{}`",
                            prefix.join("::"),
                            dest[existing].kind.keyword(),
                            path.join("::")
                        )));
                    }
                    existing
                }
                None => {
                    let id = ElementId::from_raw(next);
                    next += 1;
                    slots.push((
                        parent,
                        Slot::Package(prefix.last().cloned().unwrap_or_default()),
                        id,
                    ));
                    packages.insert(prefix.clone(), id);
                    id
                }
            };
            parent = Parent::Element(package);
        }
        for (i, element) in source.descendants(*unit).into_iter().enumerate() {
            let id = ElementId::from_raw(next);
            next += 1;
            map.insert(element, id);
            let owner = if i == 0 {
                parent
            } else {
                Parent::Element(map[&source[element].owner().expect("a member has an owner")])
            };
            slots.push((owner, Slot::Copy(element), id));
        }
        let original = source.effective_name(*unit).unwrap_or_default();
        let name = path.last().cloned().unwrap_or_default();
        if name != original {
            renames.insert(*unit, name);
        }
        plan.created.push(path.join("::"));
    }
    // References to a unit copied under another name are written with it.
    let renamed_to: HashMap<ElementId, String> = renames
        .iter()
        .map(|(unit, name)| (map[unit], name.clone()))
        .collect();
    for (parent, slot, _) in slots {
        let element = match slot {
            Slot::Package(name) => Element::named(ElementKind::Package, &name),
            Slot::Copy(original) => {
                let mut element = source[original].clone();
                element.location = None;
                if let Some(name) = renames.get(&original) {
                    element.name = Some(name.clone());
                }
                for reference in references_mut(&mut element) {
                    retarget(reference, &map, &renamed_to);
                }
                element
            }
        };
        plan.creations.push(Creation { parent, element });
    }
    plan.map = map;
    Ok(plan)
}

/// Why a copy could not be planned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CopyError {
    Conflicts(Vec<Conflict>),
    Invalid(String),
}

/// Points every step at its target's copy; standard targets stay.
fn retarget(
    reference: &mut Reference,
    map: &HashMap<ElementId, ElementId>,
    renamed_to: &HashMap<ElementId, String>,
) {
    for step in &mut reference.steps {
        if let Some(target) = step.target
            && !is_standard(target)
        {
            step.target = map.get(&target).copied();
            if let Some(name) = step.target.and_then(|t| renamed_to.get(&t))
                && let Some(last) = step.name.segments.last_mut()
            {
                *last = name.clone();
            }
        }
    }
}

fn references_mut(element: &mut Element) -> impl Iterator<Item = &mut Reference> {
    element
        .typed_by
        .iter_mut()
        .chain(&mut element.specializes)
        .chain(&mut element.redefines)
        .chain(&mut element.ends)
        .chain(&mut element.target)
        .chain(&mut element.by)
}

/// Maps a unit onto an identical one: members pair up in order.
fn map_same(
    source: &Tree,
    a: ElementId,
    dest: &Tree,
    b: ElementId,
    map: &mut HashMap<ElementId, ElementId>,
) {
    map.insert(a, b);
    for (x, y) in source[a].children().iter().zip(dest[b].children()) {
        map_same(source, *x, dest, *y, map);
    }
}

/// Maps a unit onto a different one of the same name: members pair up by
/// name, and members without a counterpart stay unmapped (references to
/// them are then reported where they are used).
fn map_by_name(
    source: &Tree,
    a: ElementId,
    dest: &Tree,
    b: ElementId,
    map: &mut HashMap<ElementId, ElementId>,
) {
    map.insert(a, b);
    for child in source[a].children() {
        let Some(name) = source.effective_name(*child) else {
            continue;
        };
        if let Some(counterpart) = dest[b]
            .children()
            .iter()
            .find(|c| dest.effective_name(**c) == Some(name))
        {
            map_by_name(source, *child, dest, *counterpart, map);
        }
    }
}

/// Applies a plan's creations to a tree directly (My Library), checking
/// that each element gets the id the plan predicted.
pub(crate) fn apply_to_tree(tree: &mut Tree, creations: Vec<Creation>) -> Result<(), String> {
    for creation in creations {
        let expected = tree.next_id();
        let id = tree
            .add(creation.parent, creation.element)
            .map_err(|error| format!("{error:?}"))?;
        if id != expected {
            return Err("the library changed while the copy was planned".into());
        }
    }
    Ok(())
}
