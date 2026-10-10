//! Traceability (C-55, ROADMAP §1.1, §4.16): Agentique's purpose constrains
//! what its autonomous cycles change, through the records, requirements,
//! locks and gates that exist already. A proposal names the requirements of
//! the project's model it serves and the elements and contracts it affects;
//! both are resolved by identity in the base commit's model when it is
//! submitted ([`resolve`]). At review, what the commit changed (model
//! elements by identity, and the parts whose linked code changed) is
//! compared with what the proposal named ([`trace`]), and the cumulative
//! change since the Operator's approved baseline (the tag
//! [`APPROVED_BASELINE`], read on the remote whose URL the objective
//! recorded when it was created, which only the Operator moves) is counted
//! at the root ([`cumulative`]). The reviewer judges both: the lists and the
//! numbers inform, they do not decide. The model's purpose requirement is
//! found by identity ([`purpose_changes`]), as the base and the commits the
//! objective protects declare it, for the gate no cycle passes when it
//! changes it (`gates::purpose`); a project that declares none is not
//! governed by it. Models, links and their presence are read by the
//! commits' trees, through a clean checkout in which nothing ran, never
//! through a checkout where the change's code ran.

use crate::builds::short;
use crate::record::Proposal;
use agq_assistant::model_tools::{Compared, Described};
use agq_execution::git::Patch;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

/// A project's model at a commit, by identity.
pub type Elements = BTreeMap<u64, Described>;

/// The git tag of the Operator's approved baseline: the commit whose
/// product the Operator last approved as a whole. Only the Operator creates
/// or moves it, and pushes it to the remote `origin`, where the
/// Orchestrator reads it at the URL the objective recorded
/// ([`approved_baseline`]); agents' worktree sessions are refused `git tag`
/// and changes to the remotes.
pub const APPROVED_BASELINE: &str = "approved-baseline";

/// The kinds a requirement of the model has: a definition or a usage.
pub const REQUIREMENT_KINDS: [&str; 2] = ["requirement def", "requirement"];

/// The kinds a part has: a definition or a usage.
const PART_KINDS: [&str; 2] = ["part def", "part"];

/// The most names a list in a brief or the thread shows; the rest are
/// counted.
const SHOWN: usize = 40;

/// The most lines a record keeps of a list.
const KEPT: usize = 200;

fn is_requirement(element: &Described) -> bool {
    REQUIREMENT_KINDS.contains(&element.kind.as_str())
}

/// The element named `name` (a qualified name, as the model prints it).
fn find<'a>(model: &'a Elements, name: &str) -> Option<&'a Described> {
    let name = name.trim();
    model.values().find(|e| e.name == name)
}

/// A name of a proposal, resolved by identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolved {
    pub name: String,
    pub element: u64,
    pub kind: String,
}

impl Resolved {
    fn of(element: &Described) -> Resolved {
        Resolved {
            name: element.name.clone(),
            element: element.id,
            kind: element.kind.clone(),
        }
    }
}

/// What a proposal's `serves` and `parts` resolved to in the base commit's
/// model, or why `serves` was not resolved: the project has no model (and
/// nothing was resolved), or its model declares no requirement yet.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serves: Vec<Resolved>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Resolved>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

impl Resolution {
    /// In a line, for the proposal's text.
    pub fn text(&self) -> String {
        let ids = |list: &[Resolved]| {
            list.iter()
                .map(|r| format!("{} (#{})", r.name, r.element))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let serves = match &self.skipped {
            Some(why) => format!("not resolved ({why})"),
            None => ids(&self.serves),
        };
        let parts = match (self.parts.is_empty(), &self.skipped) {
            (false, _) => ids(&self.parts),
            (true, Some(_)) => "not resolved".to_string(),
            (true, None) => "none".to_string(),
        };
        format!("serves {serves}; parts {parts}")
    }
}

/// Why `serves` is not resolved in a model that declares no requirement
/// yet (a project's own, before its requirements are written): it is
/// recorded as the lead states it (C-55).
pub const NO_REQUIREMENTS: &str =
    "the base commit's model declares no requirement yet, so `serves` is recorded as stated";

/// Whether `model` declares a requirement.
pub fn has_requirements(model: &Elements) -> bool {
    model.values().any(is_requirement)
}

/// The requirement named `name` in `model`, or why it is none.
pub fn requirement<'a>(model: &'a Elements, name: &str) -> Result<&'a Described, String> {
    match find(model, name) {
        Some(element) if is_requirement(element) => Ok(element),
        Some(element) => Err(format!(
            "`{}` is a {}, not a requirement",
            name.trim(),
            element.kind
        )),
        None => Err(format!(
            "`{}` is not an element of the base commit's model",
            name.trim()
        )),
    }
}

/// `serves` and `parts` resolved in `model`, the base commit's (C-55): each
/// name of `serves` a requirement (a requirement def or usage), each of
/// `parts` an existing element; or what does not resolve, with the
/// requirements there are, for the lead to correct. A model that declares
/// no requirement yet resolves `parts` only, and says why
/// ([`NO_REQUIREMENTS`]).
pub fn resolve(
    serves: &[String],
    parts: &[String],
    model: &Elements,
) -> Result<Resolution, String> {
    let mut resolution = Resolution::default();
    let mut problems = Vec::new();
    if !has_requirements(model) {
        resolution.skipped = Some(NO_REQUIREMENTS.into());
    }
    for name in serves.iter().filter(|_| resolution.skipped.is_none()) {
        match requirement(model, name) {
            Ok(element) => resolution.serves.push(Resolved::of(element)),
            Err(problem) => problems.push(problem),
        }
    }
    if !problems.is_empty() {
        let requirements: Vec<&str> = model
            .values()
            .filter(|e| is_requirement(e))
            .map(|e| e.name.as_str())
            .collect();
        problems.push(format!(
            "`serves` names requirements of the model; its requirements are: {}",
            if requirements.is_empty() {
                "none".to_string()
            } else {
                shown(&requirements)
            }
        ));
    }
    let mut unknown = Vec::new();
    for name in parts {
        match find(model, name) {
            Some(element) => resolution.parts.push(Resolved::of(element)),
            None => unknown.push(format!("`{}`", name.trim())),
        }
    }
    if !unknown.is_empty() {
        problems.push(format!(
            "`parts`: {} {} not an element of the base commit's model (name existing elements and contracts; for new ones, the element that will own them)",
            unknown.join(", "),
            if unknown.len() == 1 { "is" } else { "are" }
        ));
    }
    if problems.is_empty() {
        Ok(resolution)
    } else {
        Err(problems.join("; "))
    }
}

/// `names` joined, at most [`SHOWN`] of them and how many more.
fn shown<S: AsRef<str>>(names: &[S]) -> String {
    let mut text = names
        .iter()
        .take(SHOWN)
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ");
    if names.len() > SHOWN {
        text.push_str(&format!(" … and {} more", names.len() - SHOWN));
    }
    text
}

/// `lines`, at most [`KEPT`] of them, the last saying how many more there
/// were.
fn kept(mut lines: Vec<String>) -> Vec<String> {
    if lines.len() > KEPT {
        let more = lines.len() - KEPT;
        lines.truncate(KEPT);
        lines.push(format!("… and {more} more"));
    }
    lines
}

/// What the cycle's commit changed compared with what its proposal named
/// (C-55), as the reviewer judges it and the cycle records it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Traced {
    /// The commit traced, and the base it is compared with.
    pub commit: String,
    pub base: String,
    /// How many model elements and parts (by their code) it changed.
    #[serde(default)]
    pub changed: usize,
    /// Changed, but not named in the proposal's `parts`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_named: Vec<String>,
    /// Named in `parts`, but not changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_changed: Vec<String>,
    /// Why nothing was traced (the project has no model), when nothing was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

impl Traced {
    /// In a line, for the thread.
    pub fn line(&self) -> String {
        match &self.skipped {
            Some(why) => format!(
                "Traceability of {}: not traced ({why})",
                short(&self.commit)
            ),
            None => format!(
                "Traceability of {}: {} element(s) and part(s) changed; {} changed but not named, {} named but not changed",
                short(&self.commit),
                self.changed,
                self.not_named.len(),
                self.not_changed.len()
            ),
        }
    }

    /// The two lists, for the reviewer's brief and the thread.
    pub fn text(&self) -> String {
        if let Some(why) = &self.skipped {
            return format!("not traced: {why}");
        }
        let list = |lines: &[String]| {
            if lines.is_empty() {
                " none".to_string()
            } else {
                lines
                    .iter()
                    .take(SHOWN)
                    .map(|l| format!("\n  - {l}"))
                    .collect::<String>()
                    + &if lines.len() > SHOWN {
                        format!("\n  - … and {} more", lines.len() - SHOWN)
                    } else {
                        String::new()
                    }
            }
        };
        format!(
            "changed but not named in `parts`:{}\nnamed in `parts` but not changed:{}",
            list(&self.not_named),
            list(&self.not_changed)
        )
    }
}

/// One thing a commit changed, as traceability compares it.
struct Item<'a> {
    element: &'a Described,
    how: &'static str,
    /// The part that owns it (itself, for a part).
    part: Option<&'a Described>,
    /// What changed may be what it holds: its linked code, or an update of
    /// an element that holds a created or deleted one. A named element it
    /// holds may then be what changed; an update of its own properties is
    /// its own.
    holds: bool,
}

/// The part that owns `element` in `model`: itself when it is a part, else
/// its nearest owner that is.
fn owning_part<'a>(model: &'a Elements, element: &'a Described) -> Option<&'a Described> {
    std::iter::once(element)
        .chain(element.owners.iter().filter_map(|id| model.get(id)))
        .find(|e| PART_KINDS.contains(&e.kind.as_str()))
}

/// Whether a change and a named element are related: the same element, or
/// the change inside it (what a named element owns is named with it), or a
/// change that may be what a non-package owner of it holds (its code, or a
/// member added or removed).
fn related(item: &Item, named: &Described) -> bool {
    let changed = item.element;
    changed.id == named.id
        || changed.owners.contains(&named.id)
        || (item.holds && named.owners.contains(&changed.id) && changed.kind != "package")
}

/// The changed elements of `ids` in `model` that no other of them owns.
fn outermost<'a>(ids: &[u64], model: &'a Elements) -> Vec<&'a Described> {
    let set: BTreeSet<u64> = ids.iter().copied().collect();
    ids.iter()
        .filter_map(|id| model.get(id))
        .filter(|e| e.owners.first().is_none_or(|owner| !set.contains(owner)))
        .collect()
}

/// What `compared` (the base's and the commit's models) and the changed
/// `files` (through the implementation `links`, the commit's and the
/// base's) show the commit changed, against the elements the proposal named
/// in `parts` (by the identities they resolved to, `resolved`, or by name):
/// the changes no named element relates to, and the named elements no
/// change relates to. An updated package that kept its name changed only
/// what it holds, which is listed itself.
pub fn trace(
    compared: &Compared,
    files: &[String],
    links: &[String],
    parts: &[String],
    resolved: Option<&Resolution>,
) -> Traced {
    let created = outermost(&compared.created, &compared.after);
    let deleted = outermost(&compared.deleted, &compared.before);
    // The elements that hold a created or deleted one directly.
    let holding: BTreeSet<u64> = created
        .iter()
        .chain(&deleted)
        .filter_map(|e| e.owners.first().copied())
        .collect();
    let mut items: Vec<Item> = Vec::new();
    for element in created {
        items.push(Item {
            element,
            how: "created",
            part: owning_part(&compared.after, element),
            holds: false,
        });
    }
    for element in compared
        .updated
        .iter()
        .filter_map(|id| compared.after.get(id))
    {
        let kept_name = compared
            .before
            .get(&element.id)
            .is_some_and(|b| b.name == element.name);
        if element.kind == "package" && kept_name {
            continue;
        }
        items.push(Item {
            element,
            how: "updated",
            part: owning_part(&compared.after, element),
            holds: holding.contains(&element.id),
        });
    }
    for element in deleted {
        items.push(Item {
            element,
            how: "deleted",
            part: owning_part(&compared.before, element),
            holds: false,
        });
    }
    // The parts whose linked code changed (a link's path covering a file).
    let mut unreadable = None;
    let mut linked: BTreeSet<u64> = BTreeSet::new();
    for text in links {
        match agq_implementation::links::Links::parse(text) {
            Ok(parsed) => {
                for link in &parsed.links {
                    if files.iter().any(|f| agq_execution::within(f, &link.path)) {
                        linked.insert(link.element().raw());
                    }
                }
            }
            Err(error) => unreadable = Some(error),
        }
    }
    let mut parts_changed: BTreeSet<u64> = BTreeSet::new();
    for id in linked {
        let (model, element) = match (compared.after.get(&id), compared.before.get(&id)) {
            (Some(e), _) => (&compared.after, e),
            (None, Some(e)) => (&compared.before, e),
            (None, None) => continue,
        };
        let part = owning_part(model, element).unwrap_or(element);
        if parts_changed.insert(part.id) {
            items.push(Item {
                element: part,
                how: "code",
                part: Some(part),
                holds: true,
            });
        }
    }
    // The named elements, by identity where they resolved.
    let mut named: Vec<&Described> = Vec::new();
    let mut not_changed: Vec<String> = Vec::new();
    let ids: Vec<u64> = resolved
        .map(|r| r.parts.iter().map(|p| p.element).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        for name in parts {
            match find(&compared.before, name).or_else(|| find(&compared.after, name)) {
                Some(element) => named.push(element),
                None => not_changed.push(format!("{} (not an element of the model)", name.trim())),
            }
        }
    } else {
        for id in ids {
            if let Some(element) = compared.before.get(&id).or_else(|| compared.after.get(&id)) {
                named.push(element);
            }
        }
    }
    let mut not_named: Vec<String> = items
        .iter()
        .filter(|item| !named.iter().any(|n| related(item, n)))
        .map(|item| {
            let what = if item.how == "code" {
                "its linked code changed".to_string()
            } else {
                item.how.to_string()
            };
            match item.part {
                Some(part) if part.id != item.element.id => format!(
                    "{} ({}, {what}; in part {})",
                    item.element.name, item.element.kind, part.name
                ),
                _ => format!("{} ({}, {what})", item.element.name, item.element.kind),
            }
        })
        .collect();
    if let Some(error) = unreadable {
        not_named.push(format!(
            "code not traced: the implementation links cannot be read ({error})"
        ));
    }
    for n in &named {
        if !items.iter().any(|item| related(item, n)) {
            not_changed.push(format!("{} ({})", n.name, n.kind));
        }
    }
    Traced {
        commit: String::new(),
        base: String::new(),
        changed: items.len(),
        not_named: kept(not_named),
        not_changed: kept(not_changed),
        skipped: None,
    }
}

/// The ids of the model's purpose requirement in `model`: the requirement
/// def `Purpose` and the requirement usage `purpose` directly in a root
/// package.
pub fn purpose_ids(model: &Elements) -> BTreeSet<u64> {
    model
        .values()
        .filter(|e| {
            let last = e.name.rsplit("::").next().unwrap_or_default();
            let in_root = matches!(e.owners.as_slice(), [root] if model
                .get(root)
                .is_some_and(|r| r.kind == "package" && r.owners.is_empty()));
            in_root
                && ((e.kind == "requirement def" && last == "Purpose")
                    || (e.kind == "requirement" && last == "purpose"))
        })
        .map(|e| e.id)
        .collect()
}

/// What a change does to the model's purpose (C-55), for its gate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PurposeCheck {
    /// The project declares a root purpose requirement (in the base's model,
    /// or in an earlier one the objective protects: its start's, the
    /// approved baseline's). Only then are the purpose and `ROADMAP.md`
    /// protected: a project without one (a user's, before its purpose is
    /// modelled) is not governed by them.
    pub governed: bool,
    /// What the change does to it, each in a line.
    pub changes: Vec<String>,
}

/// What a change (`compared`: the base's model and the commit's) does to
/// the model's purpose requirement (C-55), found by identity: the elements
/// `protected` (taken from the base's model and from earlier ones, so that
/// moving it out of a root package in one cycle hides nothing in the next)
/// and those the commit's model declares (creating one is changing it).
/// Each of them, and everything they own, created, updated or deleted, and
/// each whose owners or qualified name changed (the root package wrapped in
/// another, say), is a change. A project whose base and earlier models
/// declare none is not governed: nothing to protect.
pub fn purpose_changes(compared: &Compared, earlier: &BTreeSet<u64>) -> PurposeCheck {
    let declared: BTreeSet<u64> = purpose_ids(&compared.before)
        .into_iter()
        .chain(earlier.iter().copied())
        .collect();
    if declared.is_empty() {
        return PurposeCheck::default();
    }
    let protected: BTreeSet<u64> = declared
        .into_iter()
        .chain(purpose_ids(&compared.after))
        .collect();
    let hit =
        |e: &Described| protected.contains(&e.id) || e.owners.iter().any(|o| protected.contains(o));
    let mut changes = Vec::new();
    for (ids, model, how) in [
        (&compared.created, &compared.after, "created"),
        (&compared.updated, &compared.after, "updated"),
        (&compared.deleted, &compared.before, "deleted"),
    ] {
        for element in ids.iter().filter_map(|id| model.get(id)).filter(|e| hit(e)) {
            changes.push(format!("{} ({}, {how})", element.name, element.kind));
        }
    }
    // Moved or renamed without being updated itself: an owner of it moved.
    for id in &protected {
        if let (Some(before), Some(after)) = (compared.before.get(id), compared.after.get(id))
            && (before.owners != after.owners || before.name != after.name)
            && !compared.updated.contains(id)
        {
            changes.push(format!(
                "{} ({}, moved from {})",
                after.name, after.kind, before.name
            ));
        }
    }
    PurposeCheck {
        governed: true,
        changes,
    }
}

// One group of the cumulative change, counted at the root.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    /// `top-level part defs`, `dependencies`, `requirements` or `locked
    /// elements`.
    pub what: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// Kept, but changed or holding what changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<String>,
}

/// The cumulative change from the approved baseline (or, without one, the
/// objective's start) to the reviewed commit (C-55): the reviewer judges
/// whether it still serves the purpose; the numbers inform, they do not
/// decide.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cumulative {
    /// The reviewed commit.
    pub commit: String,
    /// The commit it is compared with.
    pub since: String,
    /// `since` is the approved baseline; otherwise the objective's start.
    #[serde(default)]
    pub approved: bool,
    /// Model elements created, updated and deleted (those inside a created
    /// or deleted element counted with it).
    #[serde(default)]
    pub created: usize,
    #[serde(default)]
    pub updated: usize,
    #[serde(default)]
    pub deleted: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    /// What the baseline guard lists over the range.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_changes: Vec<String>,
    /// Why the model was not compared, when it was not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
    /// Why the approved baseline could not be read, when it could not (the
    /// objective's start stands in for it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread: Option<String>,
    /// Anything else the reviewer should know of the range (the baseline is
    /// not an ancestor of the commit, or not in the local repository; the
    /// test changes could not be read).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Cumulative {
    /// Since what, in words.
    fn since_text(&self) -> String {
        let range = format!(
            "cumulative change since {} to {}",
            short(&self.since),
            short(&self.commit)
        );
        match (self.approved, &self.unread) {
            (true, _) => format!(
                "Cumulative change since the approved baseline at {} (the tag {APPROVED_BASELINE} on origin) to {}",
                short(&self.since),
                short(&self.commit)
            ),
            (false, Some(why)) => format!(
                "The approved baseline could not be read ({why}); compared with the objective's start: {range}"
            ),
            (false, None) => format!(
                "No approved baseline recorded; compared with the objective's start: {range}"
            ),
        }
    }

    /// In a line, for the thread.
    pub fn line(&self) -> String {
        let model = match &self.skipped {
            Some(why) => format!("the model not compared ({why})"),
            None => {
                let groups: Vec<String> = self
                    .groups
                    .iter()
                    .map(|g| {
                        format!(
                            "{} +{} −{} ~{}",
                            g.what,
                            g.added.len(),
                            g.removed.len(),
                            g.changed.len()
                        )
                    })
                    .collect();
                format!(
                    "model elements +{} ~{} −{}; {}",
                    self.created,
                    self.updated,
                    self.deleted,
                    groups.join("; ")
                )
            }
        };
        format!(
            "{}: {model}; {} change(s) to tests and checks",
            self.since_text(),
            self.test_changes.len()
        )
    }

    /// In full, for the reviewer's brief and the thread.
    pub fn text(&self) -> String {
        let mut text = format!("{}.", self.since_text());
        match &self.skipped {
            Some(why) => text.push_str(&format!("\nThe model was not compared: {why}.")),
            None => {
                text.push_str(&format!(
                    "\nModel elements: {} created, {} updated, {} deleted. At the root:",
                    self.created, self.updated, self.deleted
                ));
                for group in &self.groups {
                    let names = |verb: &str, list: &[String]| {
                        if list.is_empty() {
                            format!("{verb} 0")
                        } else {
                            format!("{verb} {} ({})", list.len(), shown(list))
                        }
                    };
                    text.push_str(&format!(
                        "\n- {}: {}; {}; {}",
                        group.what,
                        names("added", &group.added),
                        names("removed", &group.removed),
                        names("changed", &group.changed)
                    ));
                }
            }
        }
        text.push_str(&format!(
            "\nTests and checks the baseline guard lists over this range: {}",
            if self.test_changes.is_empty() {
                "none".to_string()
            } else {
                shown(&self.test_changes)
            }
        ));
        for note in &self.notes {
            text.push_str(&format!("\nNote: {note}"));
        }
        text
    }
}

/// Whether `element` is a part def at the root: directly in a package that
/// is not in another, or in no package.
fn top_part_def(model: &Elements, element: &Described) -> bool {
    element.kind == "part def"
        && match element.owners.as_slice() {
            [] => true,
            [root] => model
                .get(root)
                .is_some_and(|r| r.kind == "package" && r.owners.is_empty()),
            _ => false,
        }
}

/// The groups of the change `compared` shows, counted at the root (C-55):
/// top-level part defs, dependencies, requirements, and locked elements.
pub fn groups(compared: &Compared) -> Vec<Group> {
    // What holds a change: every owner of a created, updated or deleted
    // element.
    let holding: BTreeSet<u64> = compared
        .created
        .iter()
        .chain(&compared.updated)
        .filter_map(|id| compared.after.get(id))
        .chain(
            compared
                .deleted
                .iter()
                .filter_map(|id| compared.before.get(id)),
        )
        .flat_map(|e| e.owners.iter().copied())
        .collect();
    let updated: BTreeSet<u64> = compared.updated.iter().copied().collect();
    let group = |what: &str, is: &dyn Fn(&Elements, &Described) -> bool| {
        let added = compared
            .created
            .iter()
            .filter_map(|id| compared.after.get(id))
            .filter(|e| is(&compared.after, e))
            .map(|e| e.name.clone())
            .collect();
        let removed = compared
            .deleted
            .iter()
            .filter_map(|id| compared.before.get(id))
            .filter(|e| is(&compared.before, e))
            .map(|e| e.name.clone())
            .collect();
        let changed = compared
            .after
            .values()
            .filter(|e| compared.before.contains_key(&e.id))
            .filter(|e| is(&compared.after, e))
            .filter(|e| updated.contains(&e.id) || holding.contains(&e.id))
            .map(|e| e.name.clone())
            .collect();
        Group {
            what: what.to_string(),
            added,
            removed,
            changed,
        }
    };
    let mut groups = vec![
        group("top-level part defs", &|model, e| top_part_def(model, e)),
        group("dependencies", &|_, e| e.kind == "dependency"),
        group("requirements", &|_, e| is_requirement(e)),
    ];
    // Locked at the start of the range or at its end.
    let locked_before = |id: &u64| compared.before.get(id).is_some_and(|e| e.locked);
    let locked_after = |id: &u64| compared.after.get(id).is_some_and(|e| e.locked);
    let names = |ids: &[u64], model: &Elements, locked: &dyn Fn(&u64) -> bool| -> Vec<String> {
        ids.iter()
            .filter(|id| locked(id))
            .filter_map(|id| model.get(id))
            .map(|e| e.name.clone())
            .collect()
    };
    groups.push(Group {
        what: "locked elements".into(),
        added: names(&compared.created, &compared.after, &locked_after),
        removed: names(&compared.deleted, &compared.before, &locked_before),
        changed: names(&compared.updated, &compared.after, &|id| {
            locked_before(id) || locked_after(id)
        }),
    });
    groups
}

/// The cumulative change `compared` shows from `since` to `commit` (C-55):
/// elements created, updated and deleted, and the groups at the root.
pub fn cumulative(
    compared: Result<&Compared, &str>,
    since: &str,
    commit: &str,
    approved: bool,
) -> Cumulative {
    let mut cumulative = Cumulative {
        commit: commit.to_string(),
        since: since.to_string(),
        approved,
        ..Cumulative::default()
    };
    match compared {
        Ok(compared) => {
            cumulative.created = outermost(&compared.created, &compared.after).len();
            cumulative.deleted = outermost(&compared.deleted, &compared.before).len();
            cumulative.updated = compared.updated.len();
            cumulative.groups = groups(compared);
            for group in &mut cumulative.groups {
                for list in [&mut group.added, &mut group.removed, &mut group.changed] {
                    *list = kept(std::mem::take(list));
                }
            }
        }
        Err(why) => cumulative.skipped = Some(why.to_string()),
    }
    cumulative
}

/// The commit the tag [`APPROVED_BASELINE`] points at on the remote at
/// `origin` (the URL recorded when the objective was created, so an agent
/// that redirects the remote `origin` redirects nothing), if the Operator
/// pushed it there. The local tag is never read: a cycle's worktree shares
/// the repository's local refs. An error says why the remote could not be
/// read.
pub fn approved_baseline(repository: &Path, origin: &str) -> Result<Option<String>, String> {
    if origin.trim().is_empty() || origin.starts_with('-') {
        return Err(format!("`{origin}` is not a remote's URL"));
    }
    let tag = format!("refs/tags/{APPROVED_BASELINE}");
    let listed = crate::forge::run(
        repository,
        &["git", "ls-remote", "--tags", origin, &tag],
        Duration::from_secs(120),
    )?;
    // `<id>\t<ref>` lines; an annotated tag also lists its commit, peeled,
    // as `<ref>^{}`.
    let mut plain = None;
    let mut peeled = None;
    for line in listed.stdout.lines() {
        if let Some((id, name)) = line.split_once('\t') {
            let id = id.trim().to_string();
            if name.trim() == format!("{tag}^{{}}") {
                peeled = Some(id);
            } else if name.trim() == tag {
                plain = Some(id);
            }
        }
    }
    Ok(peeled.or(plain).filter(|id| !id.is_empty()))
}

/// Whether `commit` of `repository` has a model folder, by its tree: never
/// by a checkout's files, which code run in that checkout may have changed.
pub fn has_model(repository: &Path, commit: &str) -> Result<bool, String> {
    let listed = crate::forge::run(
        repository,
        &["git", "ls-tree", "-d", "--name-only", commit, "--", "model"],
        Duration::from_secs(60),
    )?;
    Ok(listed.stdout.lines().any(|line| line.trim() == "model"))
}

/// The projects of `repository` at `commit`, by its tree (C-54, the W13.7
/// repair): the folders that hold a model's `.sysml` files themselves, as
/// the repository names them (`model`, `models/url-shortener`), in order;
/// not the pinned standards (`standards/`), not test fixtures (in a
/// `tests` or `fixtures` folder), and not a folder inside another listed.
pub fn projects(repository: &Path, commit: &str) -> Result<Vec<String>, String> {
    let listed = crate::forge::run(
        repository,
        &["git", "ls-tree", "-r", "-z", "--name-only", commit],
        Duration::from_secs(60),
    )?;
    let folders: BTreeSet<String> = listed
        .stdout
        .split('\0')
        .filter_map(|path| path.trim().rsplit_once('/'))
        .filter(|(_, file)| file.ends_with(".sysml"))
        .map(|(folder, _)| folder.to_string())
        .filter(|folder| {
            !folder.starts_with("standards/")
                && !folder
                    .split('/')
                    .any(|part| part == "tests" || part == "fixtures")
        })
        .collect();
    Ok(folders
        .iter()
        .filter(|folder| {
            !folders
                .iter()
                .any(|other| folder.starts_with(&format!("{other}/")))
        })
        .cloned()
        .collect())
}

/// The model of project `project` of `checkout` (a checkout of a commit in
/// which nothing ran), by identity, as the names of an exploration's plan
/// are resolved in it: read through a fresh repository in `scratch` that
/// holds a copy of its model files (a project other than the repository's
/// own `model` has no history of its own to read it from), removed after.
pub fn project_model(checkout: &Path, project: &str, scratch: &Path) -> Result<Elements, String> {
    let _ = std::fs::remove_dir_all(scratch);
    let read = (|| {
        crate::explore::copy_model(&checkout.join(project), &scratch.join("model"))?;
        let commit = agq_execution::git::init_and_commit(scratch, "the project's model")
            .map_err(|e| e.to_string())?;
        agq_assistant::model_tools::model_at(scratch, &commit)
    })();
    let _ = std::fs::remove_dir_all(scratch);
    read.map_err(|e| format!("the model of {project} could not be read ({e})"))
}

/// The tree of project `project` of `repository` at `revision`
/// (`git rev-parse <revision>:<project>`): its identity in the repository,
/// when git knows it.
pub fn project_tree(repository: &Path, revision: &str, project: &str) -> Option<String> {
    crate::forge::run(
        repository,
        &["git", "rev-parse", &format!("{revision}:{project}")],
        Duration::from_secs(60),
    )
    .ok()
    .map(|found| found.stdout.trim().to_string())
    .filter(|tree| !tree.is_empty())
}

/// The names of `names` that are no element of `model`.
pub fn unknown<'a>(model: &Elements, names: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| find(model, name).is_none())
        .map(|name| format!("`{}`", name.trim()))
        .collect()
}

/// Whether `commit` is in `repository` (an approved baseline may not have
/// been fetched).
fn present(repository: &Path, commit: &str) -> bool {
    crate::forge::run(
        repository,
        &["git", "cat-file", "-e", &format!("{commit}^{{commit}}")],
        Duration::from_secs(60),
    )
    .is_ok()
}

/// The base commit's model, by identity, read through `reader` (a clean
/// checkout in which nothing ran), as a proposal's `serves` and `parts` are
/// resolved in it: none when the base has no model folder (by its tree);
/// an error, which stops the cycle, when it has one that cannot be read.
pub fn base_model(
    repository: &Path,
    reader: &Path,
    base: &str,
) -> Result<Option<Elements>, String> {
    if !has_model(repository, base)? {
        return Ok(None);
    }
    agq_assistant::model_tools::model_at(reader, base)
        .map(Some)
        .map_err(|e| format!("the base commit's model could not be read ({e})"))
}

/// Why a project's proposals resolve nothing: it has no model.
pub const NO_MODEL: &str = "the project has no model, so `serves` and `parts` are not resolved";

/// The implementation links' texts of `base` and `commit` of `repository`,
/// by their trees (dropping a link hides nothing, and no checkout's files
/// count).
pub fn links_of(repository: &Path, base: &str, commit: &str) -> Vec<String> {
    [base, commit]
        .iter()
        .filter_map(|at| {
            crate::forge::run(
                repository,
                &["git", "show", &format!("{at}:model/links.json")],
                Duration::from_secs(60),
            )
            .ok()
        })
        .map(|shown| shown.stdout)
        .collect()
}

/// What the change from `base` to `commit` does to the model's purpose
/// requirement (C-55), for its gate, read through `reader` (a clean
/// checkout of a commit that has a model, in which nothing ran; none when
/// no commit concerned has one): protected by identity as the base's model
/// and the `earlier` commits' (the objective's start, the approved
/// baseline) declare it. An earlier commit that cannot be read protects
/// nothing more; the base or the commit that cannot be read is an error.
pub fn purpose_in(
    reader: Option<&Path>,
    base: &str,
    commit: &str,
    earlier: &[String],
) -> Result<PurposeCheck, String> {
    let Some(reader) = reader else {
        return Ok(PurposeCheck::default());
    };
    let compared = agq_assistant::model_tools::compare_commits(reader, base, commit)
        .map_err(|e| format!("the model could not be compared ({e})"))?;
    let mut protected = BTreeSet::new();
    for at in earlier.iter().filter(|at| at.as_str() != base) {
        if let Ok(model) = agq_assistant::model_tools::model_at(reader, at) {
            protected.extend(purpose_ids(&model));
        }
    }
    Ok(purpose_changes(&compared, &protected))
}

/// What `commit` of `repository` changed against its `base` (`patch`),
/// compared with what `proposal` named in `parts`, read through `reader` (a
/// clean checkout of a commit that has a model; none when neither has
/// one).
pub fn traced_in(
    repository: &Path,
    reader: Option<&Path>,
    base: &str,
    commit: &str,
    patch: &Patch,
    proposal: &Proposal,
) -> Traced {
    let mut traced = match reader {
        Some(reader) => match agq_assistant::model_tools::compare_commits(reader, base, commit) {
            Ok(compared) => {
                let files: Vec<String> = patch
                    .files
                    .iter()
                    .map(|f| f.path.replace('\\', "/"))
                    .collect();
                trace(
                    &compared,
                    &files,
                    &links_of(repository, base, commit),
                    &proposal.parts,
                    proposal.resolved.as_ref(),
                )
            }
            Err(error) => Traced {
                skipped: Some(format!("the model could not be compared ({error})")),
                ..Traced::default()
            },
        },
        None => Traced {
            skipped: Some("the project has no model".into()),
            ..Traced::default()
        },
    };
    traced.commit = commit.to_string();
    traced.base = base.to_string();
    traced
}

/// Where the cumulative change of a review starts (C-55): the approved
/// baseline on the remote at `origin`, when there is one, else `start`
/// (the objective's), with why no baseline was read when one could not be.
pub fn since(
    repository: &Path,
    origin: Option<&str>,
    start: &str,
) -> (String, bool, Option<String>) {
    let Some(origin) = origin else {
        return (
            start.to_string(),
            false,
            Some("no remote `origin` was recorded when the objective was created".into()),
        );
    };
    match approved_baseline(repository, origin) {
        Ok(Some(baseline)) => (baseline, true, None),
        Ok(None) => (start.to_string(), false, None),
        Err(error) => (
            start.to_string(),
            false,
            Some(format!(
                "the remote at {origin} could not be read ({error})"
            )),
        ),
    }
}

/// The cumulative change to `commit` of `repository` since the Operator's
/// approved baseline on the remote at `origin`, or, without one, since
/// `start` (the objective's start): its model, read through `reader` (a
/// clean checkout of a commit that has a model; none when none has), and
/// the tests and checks the baseline guard lists over the range.
pub fn cumulative_in(
    repository: &Path,
    reader: Option<&Path>,
    origin: Option<&str>,
    start: &str,
    commit: &str,
) -> Cumulative {
    let (since, approved, unread) = since(repository, origin, start);
    let mut notes = Vec::new();
    let here = present(repository, &since);
    if !here {
        notes.push(format!(
            "the commit {} is not in the local repository (fetch it to compare with it)",
            short(&since)
        ));
    } else if approved && !crate::forge::is_ancestor(repository, &since, commit).unwrap_or(false) {
        notes.push("the approved baseline is not an ancestor of the reviewed commit".into());
    }
    let compared = match reader {
        Some(reader) => agq_assistant::model_tools::compare_commits(reader, &since, commit)
            .map_err(|e| format!("the model could not be compared ({e})")),
        None => Err("the project has no model".to_string()),
    };
    let mut cumulative = cumulative(
        compared.as_ref().map_err(String::as_str),
        &since,
        commit,
        approved,
    );
    cumulative.unread = unread;
    cumulative.notes = notes;
    match agq_execution::git::patch_of(repository, &since, commit) {
        Ok(range) => cumulative.test_changes = crate::gates::listed_test_changes(&range),
        Err(error) if here => cumulative.notes.push(format!(
            "the tests and checks over the range could not be read ({error})"
        )),
        Err(_) => {}
    }
    cumulative
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(id: u64, name: &str, kind: &str, owners: &[u64]) -> Described {
        Described {
            id,
            name: name.into(),
            kind: kind.into(),
            owners: owners.to_vec(),
            locked: false,
        }
    }

    fn model(elements: &[Described]) -> Elements {
        elements.iter().map(|e| (e.id, e.clone())).collect()
    }

    fn base() -> Elements {
        model(&[
            element(1, "Shop", "package", &[]),
            element(2, "Shop::Store", "part def", &[1]),
            element(3, "Shop::Cart", "part def", &[1]),
            element(4, "Shop::Fast", "requirement def", &[1]),
            element(5, "Shop::fast", "requirement", &[1]),
            element(6, "Shop::Store::open", "attribute", &[2, 1]),
        ])
    }

    /// C-55: `serves` resolves to requirements only, `parts` to existing
    /// elements, by identity; what does not resolve says why.
    #[test]
    fn names_resolve_to_requirements_and_existing_elements() {
        let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let resolved = resolve(
            &names(&["Shop::Fast", "Shop::fast"]),
            &names(&["Shop::Store", "Shop::Store::open"]),
            &base(),
        )
        .unwrap();
        assert_eq!(
            resolved
                .serves
                .iter()
                .map(|r| r.element)
                .collect::<Vec<_>>(),
            vec![4, 5]
        );
        assert_eq!(
            resolved.parts.iter().map(|r| r.element).collect::<Vec<_>>(),
            vec![2, 6]
        );
        let part = resolve(&names(&["Shop::Store"]), &[], &base()).unwrap_err();
        assert!(part.contains("is a part def, not a requirement"), "{part}");
        assert!(part.contains("Shop::Fast, Shop::fast"), "{part}");
        let missing = resolve(&names(&["Shop::Quick"]), &[], &base()).unwrap_err();
        assert!(
            missing.contains("`Shop::Quick` is not an element"),
            "{missing}"
        );
        let unknown =
            resolve(&names(&["Shop::Fast"]), &names(&["Shop::Basket"]), &base()).unwrap_err();
        assert!(
            unknown.contains("`parts`: `Shop::Basket` is not"),
            "{unknown}"
        );
        assert!(!unknown.contains("`serves`"), "{unknown}");
    }

    /// C-55: what a commit changed against what was named: a change inside
    /// a named element is named; a sibling is not; a named element nothing
    /// touched is listed; a package that only holds a new element is not.
    #[test]
    fn changes_are_traced_against_what_the_proposal_named() {
        let before = base();
        let mut after = base();
        after.insert(7, element(7, "Shop::Store::close", "attribute", &[2, 1]));
        after.insert(8, element(8, "Shop::Slow", "requirement def", &[1]));
        let compared = Compared {
            before,
            after,
            created: vec![7, 8],
            updated: vec![2, 1, 3],
            deleted: vec![],
        };
        let traced = trace(
            &compared,
            &[],
            &[],
            &["Shop::Store".to_string(), "Shop::Fast".to_string()],
            None,
        );
        assert_eq!(
            traced.not_named,
            vec![
                "Shop::Slow (requirement def, created)".to_string(),
                "Shop::Cart (part def, updated)".to_string()
            ]
        );
        assert_eq!(traced.not_changed, vec!["Shop::Fast (requirement def)"]);
        assert_eq!(traced.changed, 4);
    }

    /// The base model with a root purpose requirement: `Purpose` (10) with
    /// its doc (11), and `purpose` (12); and a `Purpose` that is not at the
    /// root (13).
    fn governed() -> Elements {
        let mut model = base();
        model.insert(10, element(10, "Shop::Purpose", "requirement def", &[1]));
        model.insert(11, element(11, "Shop::Purpose::(doc)", "doc", &[10, 1]));
        model.insert(12, element(12, "Shop::purpose", "requirement", &[1]));
        model.insert(
            13,
            element(13, "Shop::Store::Purpose", "requirement def", &[2, 1]),
        );
        model
    }

    fn compared(before: Elements, after: Elements, changes: [Vec<u64>; 3]) -> Compared {
        let [created, updated, deleted] = changes;
        Compared {
            before,
            after,
            created,
            updated,
            deleted,
        }
    }

    /// C-55: the purpose requirement and everything it owns, by identity,
    /// in a root package: renamed it is still itself; a `Purpose` elsewhere
    /// is not it.
    #[test]
    fn the_purpose_requirement_is_found_by_identity_in_the_root_package() {
        let none = BTreeSet::new();
        let check = |c: &Compared| purpose_changes(c, &none);
        let doc = check(&compared(
            governed(),
            governed(),
            [vec![], vec![11], vec![]],
        ));
        assert!(doc.governed);
        assert_eq!(doc.changes, vec!["Shop::Purpose::(doc) (doc, updated)"]);
        let mut renamed = governed();
        renamed.insert(10, element(10, "Shop::Aim", "requirement def", &[1]));
        renamed.insert(11, element(11, "Shop::Aim::(doc)", "doc", &[10, 1]));
        assert_eq!(
            check(&compared(
                governed(),
                renamed.clone(),
                [vec![], vec![10], vec![]]
            ))
            .changes,
            vec!["Shop::Aim (requirement def, updated)"]
        );
        assert!(
            check(&compared(
                governed(),
                governed(),
                [vec![], vec![13, 2], vec![]]
            ))
            .changes
            .is_empty()
        );
        // Everything it owns: a subrequirement and a satisfy relationship
        // created inside it, a doc deleted from it.
        let mut owning = governed();
        owning.remove(&11);
        owning.insert(
            14,
            element(14, "Shop::Purpose::sells", "requirement", &[10, 1]),
        );
        owning.insert(
            15,
            element(15, "Shop::purpose::(satisfy Store)", "satisfy", &[12, 1]),
        );
        assert_eq!(
            check(&compared(
                governed(),
                owning,
                [vec![14, 15], vec![], vec![11]]
            ))
            .changes,
            vec![
                "Shop::Purpose::sells (requirement, created)",
                "Shop::purpose::(satisfy Store) (satisfy, created)",
                "Shop::Purpose::(doc) (doc, deleted)"
            ]
        );
    }

    /// C-55: only a project whose base (or an earlier commit the objective
    /// protects) declares a root purpose requirement is governed: a user's
    /// project without one is not, and creating one there changes nothing
    /// protected; in a governed one, a second one created is a change.
    #[test]
    fn only_a_project_that_declares_its_purpose_is_governed() {
        let none = BTreeSet::new();
        let mut created = base();
        created.insert(20, element(20, "Shop::purpose", "requirement", &[1]));
        let ungoverned = purpose_changes(
            &compared(base(), created, [vec![20], vec![], vec![]]),
            &none,
        );
        assert!(!ungoverned.governed && ungoverned.changes.is_empty());
        let mut second = governed();
        second.insert(21, element(21, "Shop::Purpose#2", "requirement def", &[1]));
        let mut second_named = second.clone();
        second_named.insert(21, element(21, "Shop::Purpose", "requirement def", &[1]));
        let check = purpose_changes(
            &compared(governed(), second_named, [vec![21], vec![], vec![]]),
            &none,
        );
        assert!(check.governed);
        assert_eq!(
            check.changes,
            vec!["Shop::Purpose (requirement def, created)"]
        );
    }

    /// C-55, the two-step bypass: wrapping the root package in another moves
    /// the purpose without updating it, which is a change; and once it is
    /// out of a root package, the earlier commits' models still protect it
    /// by identity.
    #[test]
    fn moving_the_purpose_out_of_the_root_is_a_change_and_hides_it_from_nothing() {
        let none = BTreeSet::new();
        // Shop wrapped in Outer (30): Purpose's owners and name change.
        let mut wrapped = Elements::new();
        wrapped.insert(30, element(30, "Outer", "package", &[]));
        for (id, e) in governed() {
            let mut owners = e.owners.clone();
            owners.push(30);
            wrapped.insert(
                id,
                element(id, &format!("Outer::{}", e.name), &e.kind, &owners),
            );
        }
        let moved = purpose_changes(
            &compared(governed(), wrapped.clone(), [vec![30], vec![1], vec![]]),
            &none,
        );
        assert!(moved.governed);
        assert!(
            moved.changes.contains(
                &"Outer::Shop::Purpose (requirement def, moved from Shop::Purpose)".to_string()
            ),
            "{:?}",
            moved.changes
        );
        // The next cycle: its base has no root purpose any more, but the
        // objective's start does.
        let earlier = purpose_ids(&governed());
        assert_eq!(earlier, BTreeSet::from([10, 12]));
        let mut rewritten = wrapped.clone();
        rewritten.insert(
            11,
            element(11, "Outer::Shop::Purpose::(doc)", "doc", &[10, 1, 30]),
        );
        let next = compared(wrapped, rewritten, [vec![], vec![11], vec![]]);
        assert!(
            !purpose_changes(&next, &none).governed,
            "the base alone no longer shows it"
        );
        let protected = purpose_changes(&next, &earlier);
        assert!(protected.governed);
        assert_eq!(
            protected.changes,
            vec!["Outer::Shop::Purpose::(doc) (doc, updated)"]
        );
    }

    /// C-55: a named element's owner whose own properties changed is not
    /// named by it; one whose change is a member added, or its code, is.
    #[test]
    fn a_named_member_does_not_name_its_owners_own_change() {
        let mut after = base();
        after.insert(7, element(7, "Shop::Store::close", "attribute", &[2, 1]));
        let names = vec!["Shop::Store::open".to_string()];
        // Store's own properties changed: not named by its member.
        let own = trace(
            &compared(base(), base(), [vec![], vec![2], vec![]]),
            &[],
            &[],
            &names,
            None,
        );
        assert_eq!(own.not_named, vec!["Shop::Store (part def, updated)"]);
        // Store holds a new member: its update may be that.
        let held = trace(
            &compared(base(), after, [vec![7], vec![2], vec![]]),
            &[],
            &[],
            &names,
            None,
        );
        assert_eq!(
            held.not_named,
            vec!["Shop::Store::close (attribute, created; in part Shop::Store)"]
        );
    }

    /// C-55: a model that declares no requirement yet resolves `parts` and
    /// records `serves` as stated, saying why.
    #[test]
    fn a_model_without_requirements_records_serves_as_stated() {
        let mut model = base();
        model.remove(&4);
        model.remove(&5);
        let resolved = resolve(
            &["Shop::Fast".to_string()],
            &["Shop::Store".to_string()],
            &model,
        )
        .unwrap();
        assert!(resolved.serves.is_empty());
        assert_eq!(resolved.parts[0].element, 2);
        assert_eq!(resolved.skipped.as_deref(), Some(NO_REQUIREMENTS));
        assert!(resolve(&[], &["Shop::Basket".to_string()], &model).is_err());
    }
}
