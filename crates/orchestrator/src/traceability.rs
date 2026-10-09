//! Traceability (C-55, ROADMAP §1.1, §4.16): Agentique's purpose constrains
//! what its autonomous cycles change, through the records, requirements,
//! locks and gates that exist already. A proposal names the requirements of
//! the project's model it serves and the elements and contracts it affects;
//! both are resolved by identity in the base commit's model when it is
//! submitted ([`resolve`]). At review, what the commit changed (model
//! elements by identity, and the parts whose linked code changed) is
//! compared with what the proposal named ([`trace`]), and the cumulative
//! change since the Operator's approved baseline (the tag
//! [`APPROVED_BASELINE`] on the remote `origin`, which only the Operator
//! creates or moves) is counted at the root ([`cumulative`]). The reviewer judges both: the lists
//! and the numbers inform, they do not decide. The model's purpose
//! requirement is found by identity ([`purpose_changes`]) for the gate no
//! cycle passes when it changes it (`gates::purpose`).

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
/// Orchestrator reads it ([`approved_baseline`]); agents' worktree sessions
/// are refused `git tag`.
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
/// model, or why nothing was resolved (the project has no model).
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
        match &self.skipped {
            Some(why) => format!("not resolved: {why}"),
            None => {
                let ids = |list: &[Resolved]| {
                    list.iter()
                        .map(|r| format!("{} (#{})", r.name, r.element))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                format!(
                    "serves {}; parts {}",
                    ids(&self.serves),
                    if self.parts.is_empty() {
                        "none".to_string()
                    } else {
                        ids(&self.parts)
                    }
                )
            }
        }
    }
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
/// requirements there are, for the lead to correct.
pub fn resolve(
    serves: &[String],
    parts: &[String],
    model: &Elements,
) -> Result<Resolution, String> {
    let mut resolution = Resolution::default();
    let mut problems = Vec::new();
    for name in serves {
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
}

/// The part that owns `element` in `model`: itself when it is a part, else
/// its nearest owner that is.
fn owning_part<'a>(model: &'a Elements, element: &'a Described) -> Option<&'a Described> {
    std::iter::once(element)
        .chain(element.owners.iter().filter_map(|id| model.get(id)))
        .find(|e| PART_KINDS.contains(&e.kind.as_str()))
}

/// Whether a change `changed` and a named element `named` are the same, or
/// one owns the other: what a named element owns is named with it, and a
/// named element is changed when its part (never a package, which only
/// holds) changed.
fn related(changed: &Described, named: &Described) -> bool {
    changed.id == named.id
        || changed.owners.contains(&named.id)
        || (named.owners.contains(&changed.id) && changed.kind != "package")
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
    let mut items: Vec<Item> = Vec::new();
    for element in outermost(&compared.created, &compared.after) {
        items.push(Item {
            element,
            how: "created",
            part: owning_part(&compared.after, element),
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
        });
    }
    for element in outermost(&compared.deleted, &compared.before) {
        items.push(Item {
            element,
            how: "deleted",
            part: owning_part(&compared.before, element),
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
            });
        }
    }
    // The named elements, by identity where they resolved.
    let mut named: Vec<&Described> = Vec::new();
    let mut not_changed: Vec<String> = Vec::new();
    let ids: Vec<u64> = resolved
        .filter(|r| r.skipped.is_none() && !r.parts.is_empty())
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
        .filter(|item| !named.iter().any(|n| related(item.element, n)))
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
        if !items.iter().any(|item| related(item.element, n)) {
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
fn purpose_ids(model: &Elements) -> impl Iterator<Item = u64> + '_ {
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
}

/// What a change does to the model's purpose requirement (C-55), found by
/// identity in the base's model (renaming it hides nothing) and in the
/// commit's (creating one is changing it): each of its elements, and
/// everything they own, created, updated or deleted. Empty when there is
/// none on either side, or it is unchanged.
pub fn purpose_changes(compared: &Compared) -> Vec<String> {
    let protected: BTreeSet<u64> = purpose_ids(&compared.before)
        .chain(purpose_ids(&compared.after))
        .collect();
    let hit =
        |e: &Described| protected.contains(&e.id) || e.owners.iter().any(|o| protected.contains(o));
    let mut found = Vec::new();
    for (ids, model, how) in [
        (&compared.created, &compared.after, "created"),
        (&compared.updated, &compared.after, "updated"),
        (&compared.deleted, &compared.before, "deleted"),
    ] {
        for element in ids.iter().filter_map(|id| model.get(id)).filter(|e| hit(e)) {
            found.push(format!("{} ({}, {how})", element.name, element.kind));
        }
    }
    found
}

/// One group of the cumulative change, counted at the root.
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
    /// Anything else the reviewer should know of the range (the baseline is
    /// not an ancestor of the commit; the test changes could not be read).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Cumulative {
    /// Since what, in words.
    fn since_text(&self) -> String {
        if self.approved {
            format!(
                "Cumulative change since the approved baseline at {} (the tag {APPROVED_BASELINE} on origin) to {}",
                short(&self.since),
                short(&self.commit)
            )
        } else {
            format!(
                "No approved baseline recorded; compared with the objective's start: cumulative change since {} to {}",
                short(&self.since),
                short(&self.commit)
            )
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

/// The commit the tag [`APPROVED_BASELINE`] points at on `repository`'s
/// remote `origin`, where the Operator pushes it, if they did. The local
/// tag is never read: a cycle's worktree shares the repository's local
/// refs, so only the remote's tag is the Operator's for certain. An error
/// says why the remote could not be read.
pub fn approved_baseline(repository: &Path) -> Result<Option<String>, String> {
    let tag = format!("refs/tags/{APPROVED_BASELINE}");
    let listed = crate::forge::run(
        repository,
        &["git", "ls-remote", "--tags", "origin", &tag],
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

/// The base commit's model, read from `folder` (a clean checkout of
/// `base`), as a proposal's `serves` and `parts` are resolved in it; or why
/// there is none: a project without a model resolves nothing, and its
/// proposals say so.
pub fn base_model(folder: &Path, base: &str) -> Result<Elements, String> {
    if !folder.join("model").is_dir() {
        return Err("the project has no model, so `serves` and `parts` are not resolved".into());
    }
    agq_assistant::model_tools::model_at(folder, base).map_err(|e| {
        format!(
            "the base commit's model could not be read ({e}), so `serves` and `parts` are not resolved"
        )
    })
}

/// The implementation links' texts to read for a change in `folder` (a
/// checkout of `repository`): the checkout's and the base's, so dropping a
/// link hides nothing.
pub fn links_of(repository: &Path, folder: &Path, base: &str) -> Vec<String> {
    let mut links = Vec::new();
    if let Ok(text) = std::fs::read_to_string(folder.join("model").join("links.json")) {
        links.push(text);
    }
    if let Ok(shown) = crate::forge::run(
        repository,
        &["git", "show", &format!("{base}:model/links.json")],
        Duration::from_secs(60),
    ) {
        links.push(shown.stdout);
    }
    links
}

/// What the change from `base` to `commit` (checked out clean in `folder`)
/// does to the model's purpose requirement, by identity, for its gate; a
/// change that removes the model removes any purpose requirement with it.
pub fn purpose_in(
    folder: &Path,
    base: &str,
    commit: &str,
    patch: &Patch,
) -> Result<Vec<String>, String> {
    if folder.join("model").is_dir() {
        agq_assistant::model_tools::compare_commits(folder, base, commit)
            .map(|compared| purpose_changes(&compared))
    } else if patch
        .files
        .iter()
        .any(|f| f.path.starts_with("model/") && f.status == "deleted")
    {
        Ok(vec!["the model, which the change removes".into()])
    } else {
        Ok(Vec::new())
    }
}

/// What `commit` of `repository` (checked out clean in `folder`) changed
/// against its `base` (`patch`), compared with what `proposal` named in
/// `parts`.
pub fn traced_in(
    repository: &Path,
    folder: &Path,
    base: &str,
    commit: &str,
    patch: &Patch,
    proposal: &Proposal,
) -> Traced {
    let mut traced = if folder.join("model").is_dir() {
        match agq_assistant::model_tools::compare_commits(folder, base, commit) {
            Ok(compared) => {
                let files: Vec<String> = patch
                    .files
                    .iter()
                    .map(|f| f.path.replace('\\', "/"))
                    .collect();
                trace(
                    &compared,
                    &files,
                    &links_of(repository, folder, base),
                    &proposal.parts,
                    proposal.resolved.as_ref(),
                )
            }
            Err(error) => Traced {
                skipped: Some(format!("the model could not be compared ({error})")),
                ..Traced::default()
            },
        }
    } else {
        Traced {
            skipped: Some("the project has no model".into()),
            ..Traced::default()
        }
    };
    traced.commit = commit.to_string();
    traced.base = base.to_string();
    traced
}

/// The cumulative change to `commit` of `repository` (checked out clean in
/// `folder`) since the Operator's approved baseline (the tag on `origin`),
/// or, without one, since `start` (the objective's start): its model, and
/// the tests and checks the baseline guard lists over the range.
pub fn cumulative_in(repository: &Path, folder: &Path, start: &str, commit: &str) -> Cumulative {
    let mut notes = Vec::new();
    let (since, approved) = match approved_baseline(repository) {
        Ok(Some(baseline)) => (baseline, true),
        Ok(None) => (start.to_string(), false),
        Err(error) => {
            notes.push(format!(
                "the approved baseline is read from the remote `origin`, which could not be read ({error})"
            ));
            (start.to_string(), false)
        }
    };
    let compared = if folder.join("model").is_dir() {
        agq_assistant::model_tools::compare_commits(folder, &since, commit)
            .map_err(|e| format!("the model could not be compared ({e})"))
    } else {
        Err("the project has no model".to_string())
    };
    let mut cumulative = cumulative(
        compared.as_ref().map_err(String::as_str),
        &since,
        commit,
        approved,
    );
    cumulative.notes = notes;
    match agq_execution::git::patch_of(repository, &since, commit) {
        Ok(range) => cumulative.test_changes = crate::gates::listed_test_changes(&range),
        Err(error) => cumulative.notes.push(format!(
            "the tests and checks over the range could not be read ({error})"
        )),
    }
    if approved && !crate::forge::is_ancestor(repository, &since, commit).unwrap_or(false) {
        cumulative
            .notes
            .push("the approved baseline is not an ancestor of the reviewed commit".into());
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

    #[test]
    fn the_purpose_requirement_is_found_by_identity_in_the_root_package() {
        let mut before = base();
        before.insert(10, element(10, "Shop::Purpose", "requirement def", &[1]));
        before.insert(11, element(11, "Shop::Purpose::(doc)", "doc", &[10, 1]));
        before.insert(12, element(12, "Shop::purpose", "requirement", &[1]));
        // Not in the root package: not the purpose requirement.
        before.insert(
            13,
            element(13, "Shop::Store::Purpose", "requirement def", &[2, 1]),
        );
        let mut after = before.clone();
        // Renamed: still itself, by identity.
        after.insert(10, element(10, "Shop::Aim", "requirement def", &[1]));
        let changed = |updated: Vec<u64>| Compared {
            before: before.clone(),
            after: after.clone(),
            created: vec![],
            updated,
            deleted: vec![],
        };
        assert_eq!(
            purpose_changes(&changed(vec![11])),
            vec!["Shop::Purpose::(doc) (doc, updated)"]
        );
        assert_eq!(
            purpose_changes(&changed(vec![10])),
            vec!["Shop::Aim (requirement def, updated)"]
        );
        assert!(purpose_changes(&changed(vec![13, 2])).is_empty());
        // Everything it owns: a subrequirement and a satisfy relationship
        // created inside it, a doc deleted from it.
        let mut owning = after.clone();
        owning.insert(14, element(14, "Shop::Aim::sells", "requirement", &[10, 1]));
        owning.insert(
            15,
            element(15, "Shop::purpose::(satisfy Store)", "satisfy", &[12, 1]),
        );
        let compared = Compared {
            before: before.clone(),
            after: owning,
            created: vec![14, 15],
            updated: vec![],
            deleted: vec![11],
        };
        assert_eq!(
            purpose_changes(&compared),
            vec![
                "Shop::Aim::sells (requirement, created)",
                "Shop::purpose::(satisfy Store) (satisfy, created)",
                "Shop::Purpose::(doc) (doc, deleted)"
            ]
        );
        // Created where there was none: changing the purpose too.
        let mut created = base();
        created.insert(20, element(20, "Shop::purpose", "requirement", &[1]));
        let compared = Compared {
            before: base(),
            after: created,
            created: vec![20],
            updated: vec![],
            deleted: vec![],
        };
        assert_eq!(
            purpose_changes(&compared),
            vec!["Shop::purpose (requirement, created)"]
        );
    }
}
