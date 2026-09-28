//! The History Panel's content: checkpoints, newest first, and a visual
//! "what changed" between two of them, or between one and now, shown on the
//! Surface.
use crate::studio::Studio;
use agq_language::{ElementId, ElementKind, Tree, validate};
use agq_studio_scene::SceneInput;
use agq_system_state::{Checkpoint, Project, compare};
use std::collections::{BTreeMap, BTreeSet};
use std::time::SystemTime;

/// Two versions of the model and what differs between them.
pub struct Comparison {
    pub before_label: String,
    pub after_label: String,
    pub before: SceneInput,
    /// The later version; the current model when `after_is_now`.
    pub after: SceneInput,
    pub after_is_now: bool,
    /// Elements that changed in ways their card does not show.
    pub changed: BTreeSet<ElementId>,
    pub created: Vec<(ElementId, String)>,
    pub updated: Vec<(ElementId, String)>,
    pub deleted: Vec<String>,
    before_tree: Tree,
}

fn input_of(tree: &Tree) -> SceneInput {
    let mut problems = BTreeMap::new();
    for diagnostic in validate(tree) {
        *problems.entry(diagnostic.element).or_insert(0) += 1;
    }
    SceneInput::from_tree(tree, &BTreeSet::new(), &problems, 0)
}

impl Comparison {
    pub fn of_trees(
        before_label: &str,
        before: Tree,
        after_label: &str,
        after: &Tree,
        after_input: SceneInput,
        after_is_now: bool,
    ) -> Self {
        let mut comparison = Comparison {
            before_label: before_label.into(),
            after_label: after_label.into(),
            before: input_of(&before),
            after: after_input,
            after_is_now,
            changed: BTreeSet::new(),
            created: Vec::new(),
            updated: Vec::new(),
            deleted: Vec::new(),
            before_tree: before,
        };
        comparison.recompute(after);
        comparison
    }

    /// Follows the current model when the comparison ends at "now".
    pub fn update_now(&mut self, tree: &Tree, input: &SceneInput) {
        self.after = input.clone();
        self.recompute(tree);
    }

    fn recompute(&mut self, after: &Tree) {
        let before = &self.before_tree;
        let difference = compare(before, after);
        let cards: BTreeSet<_> = self.after.nodes.iter().map(|n| n.id).collect();
        let nearest_card = |mut id: ElementId| loop {
            if cards.contains(&id) {
                return Some(id);
            }
            id = after.get(id)?.owner()?;
        };
        self.changed = difference
            .updated
            .iter()
            .chain(&difference.created)
            .filter_map(|id| nearest_card(*id))
            .filter(|id| !difference.created.contains(id))
            .collect();
        // A card whose owned non-card element was deleted changed as well.
        for id in &difference.deleted {
            let mut owner = before.get(*id).and_then(|e| e.owner());
            while let Some(current) = owner {
                if cards.contains(&current) && after.contains(current) {
                    self.changed.insert(current);
                    break;
                }
                owner = before.get(current).and_then(|e| e.owner());
            }
        }
        // Named elements, and the connections and relationships between them.
        let named = |tree: &Tree, id: ElementId| {
            tree.effective_name(id).is_some()
                || tree.get(id).is_some_and(|e| {
                    matches!(
                        e.kind,
                        ElementKind::Connection | ElementKind::Interface | ElementKind::Satisfy
                    )
                })
        };
        self.created = difference
            .created
            .iter()
            .filter(|id| named(after, **id))
            .map(|id| (*id, crate::edit::display_path(after, *id)))
            .collect();
        self.updated = difference
            .updated
            .iter()
            .filter(|id| named(after, **id) && cards.contains(id))
            .map(|id| (*id, crate::edit::display_path(after, *id)))
            .collect();
        self.deleted = difference
            .deleted
            .iter()
            .filter(|id| named(before, **id))
            .map(|id| crate::edit::display_path(before, *id))
            .collect();
    }
}

#[derive(Default)]
pub struct HistoryPanel {
    pub checkpoints: Vec<Checkpoint>,
    pub loaded: bool,
    /// Selected checkpoint ids, at most two.
    pub selected: Vec<String>,
    /// Why the checkpoints could not be read, if they could not.
    pub error: Option<String>,
    /// Whether the model differs from the last checkpoint; updated on
    /// change events and checkpoints, not every frame.
    pub uncommitted: Option<bool>,
}
impl HistoryPanel {
    pub fn reload(&mut self, project: &Project) {
        match project.checkpoints() {
            Ok(checkpoints) => {
                self.checkpoints = checkpoints;
                self.error = None;
            }
            Err(error) => {
                self.checkpoints.clear();
                self.error = Some(error.to_string());
            }
        }
        self.uncommitted = project.has_uncommitted_changes().ok();
        self.loaded = true;
        let ids: BTreeSet<_> = self.checkpoints.iter().map(|c| c.id.clone()).collect();
        self.selected.retain(|id| ids.contains(id));
    }

    /// Selects or deselects a checkpoint; at most two stay selected.
    pub fn toggle(&mut self, id: &str) {
        if let Some(position) = self.selected.iter().position(|s| s == id) {
            self.selected.remove(position);
        } else {
            self.selected.push(id.to_string());
            if self.selected.len() > 2 {
                self.selected.remove(0);
            }
        }
    }
}

/// How long ago a checkpoint was recorded, in words.
pub fn ago(time: i64) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let seconds = (now - time).max(0);
    match seconds {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", seconds / 60),
        3600..=86399 => format!("{} h ago", seconds / 3600),
        _ => format!("{} days ago", seconds / 86400),
    }
}

impl Studio {
    /// Reads the checkpoints once a project is shown.
    pub fn load_history(&mut self) {
        if let Some(project) = &self.project
            && !self.history.loaded
        {
            self.history.reload(project);
        }
    }

    /// Shows what changed since the selected checkpoint, or between the two.
    pub fn compare_selected(&mut self) {
        let Some(project) = &self.project else { return };
        // Checkpoints are listed newest first: the later index is older.
        let mut chosen: Vec<&Checkpoint> = self
            .history
            .checkpoints
            .iter()
            .filter(|c| self.history.selected.contains(&c.id))
            .collect();
        chosen.reverse();
        let comparison = match chosen[..] {
            [only] => project.tree_at(&only.id).map(|before| {
                Comparison::of_trees(
                    &only.message,
                    before,
                    "now",
                    project.state().tree(),
                    self.input.clone(),
                    true,
                )
            }),
            [older, newer] => project.tree_at(&older.id).and_then(|before| {
                let after = project.tree_at(&newer.id)?;
                let after_input = input_of(&after);
                Ok(Comparison::of_trees(
                    &older.message,
                    before,
                    &newer.message,
                    &after,
                    after_input,
                    false,
                ))
            }),
            _ => return,
        };
        match comparison {
            Ok(comparison) => {
                self.status = format!(
                    "What changed from “{}” to {}",
                    comparison.before_label,
                    if comparison.after_is_now {
                        "now".to_string()
                    } else {
                        format!("“{}”", comparison.after_label)
                    }
                );
                self.comparison = Some(comparison);
                self.rebuild();
            }
            Err(error) => self.status = format!("The checkpoint could not be read: {error}"),
        }
    }

    /// Ends the "what changed" comparison.
    pub fn close_comparison(&mut self) {
        self.comparison = None;
        self.rebuild();
    }
}
