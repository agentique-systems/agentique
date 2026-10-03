//! The Assistant's model tools on a working copy's own model (a task's or an
//! objective's cycle's worktree, C-50, C-53): the same tools and System
//! State operations as in the Conversation, on the worktree's project, so a
//! proposed model change stays proposed until it is integrated. A locked
//! element is refused unless the objective names it.
//!
//! [`locked_changes`] is §4.15's integration check on a worktree's commit:
//! the elements it changes that were locked then or are locked now, and
//! whether it changed the locks.

use crate::conversation::ToolResult;
use crate::tools::{Prepared, cap, prepare};
use crate::turn::ToolCall;
use agq_language::{ElementId, Tree};
use agq_system_state::{Actor, ApplyError, Project};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A working copy's model, opened when a tool first needs it.
pub struct WorkingModel {
    folder: PathBuf,
    project: Option<Project>,
    /// Locked elements (qualified names) changes may touch: those an
    /// objective names. Empty for a task (a worker cannot confirm).
    confirmed: Vec<String>,
}

impl WorkingModel {
    pub fn new(folder: impl Into<PathBuf>) -> WorkingModel {
        WorkingModel {
            folder: folder.into(),
            project: None,
            confirmed: Vec::new(),
        }
    }

    /// Lets changes touch the locked elements an objective names.
    pub fn confirming(mut self, names: Vec<String>) -> WorkingModel {
        self.confirmed = names;
        self
    }

    fn project(&mut self) -> Result<&mut Project, String> {
        if self.project.is_none() {
            let project = Project::open(&self.folder)
                .map_err(|e| format!("The working copy's model cannot be opened: {e}"))?;
            self.project = Some(project);
        }
        Ok(self.project.as_mut().expect("opened above"))
    }

    /// Carries out a model tool (read_model, find_elements, get_problems,
    /// apply_changes, inspect_behaviour, list_scenarios) on this model.
    pub fn execute(&mut self, call: &ToolCall) -> ToolResult {
        let confirmed = self.confirmed.clone();
        let project = match self.project() {
            Ok(project) => project,
            Err(error) => return ToolResult::error(error),
        };
        let library = agq_library::Library::default();
        match prepare(project.state(), &library, &call.name, &call.input) {
            Prepared::Answer(text) => ToolResult::answer(cap(text)),
            Prepared::Change(mut change) => {
                change.actor = Actor::Assistant;
                change.confirmed = confirmed
                    .iter()
                    .filter_map(|name| project.state().tree().find(name))
                    .collect();
                match project.apply(change) {
                    Ok(event) => ToolResult::applied(project.state(), &event),
                    Err(ApplyError::Rejection(rejection)) => {
                        ToolResult::rejected(project.state(), &rejection)
                    }
                    Err(error) => ToolResult::error(format!("Not applied: {error}")),
                }
            }
            Prepared::Invalid(message) => ToolResult::error(message),
            _ => ToolResult::error(format!("`{}` is not available here", call.name)),
        }
    }

    /// Closes the model (its files are saved with each change; this
    /// releases its lock), returning it if a tool read or changed it.
    pub fn close(&mut self) -> Option<Tree> {
        let project = self.project.take()?;
        Some(project.state().tree().clone())
    }
}

/// Whether `id` or one of its owners is in `locks` (a lock covers the part
/// and what it owns, R-11).
fn covered(tree: &Tree, id: ElementId, locks: &BTreeSet<ElementId>) -> bool {
    let mut next = Some(id);
    while let Some(id) = next {
        if locks.contains(&id) {
            return true;
        }
        next = tree.get(id).and_then(|element| element.owner());
    }
    false
}

/// What a worktree's commit changes in locked parts of the model, compared
/// with `base` (§4.15): each changed element (by qualified name) covered by
/// a lock then or now, and "the locks" if the commit changed them. Names in
/// `allowed` (an objective's locked elements) and what they own are not
/// listed. The model is read from `folder` as committed.
pub fn locked_changes(
    folder: &Path,
    base: &str,
    allowed: &[String],
) -> Result<Vec<String>, String> {
    let project = Project::open(folder).map_err(|e| e.to_string())?;
    let before = project.tree_at(base).map_err(|e| e.to_string())?;
    let locks_before = project.locks_at(base).map_err(|e| e.to_string())?;
    let after = project.state().tree();
    let locks_now = project.state().locks().clone();
    let mut found = Vec::new();
    if locks_before != locks_now {
        found.push("the locks".to_string());
    }
    let allowed_ids: Vec<ElementId> = allowed
        .iter()
        .filter_map(|name| after.find(name).or_else(|| before.find(name)))
        .collect();
    let named = |tree: &Tree, id: ElementId| {
        let set: BTreeSet<ElementId> = allowed_ids.iter().copied().collect();
        covered(tree, id, &set)
    };
    let comparison = agq_system_state::compare(&before, after);
    let mut check = |tree: &Tree, id: ElementId| {
        let locked = covered(tree, id, &locks_before) || covered(tree, id, &locks_now);
        if locked && !named(tree, id) {
            let name = tree.qualified_name(id);
            if !found.contains(&name) {
                found.push(name);
            }
        }
    };
    for id in comparison.updated.iter().chain(&comparison.created) {
        check(after, *id);
    }
    for id in &comparison.deleted {
        check(&before, *id);
    }
    Ok(found)
}
