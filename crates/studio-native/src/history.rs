//! The History Panel: checkpoints, newest first, and a visual "what changed"
//! between two of them, or between one and now, shown on the Surface.
use crate::{
    app::StudioApp,
    targets::{Target, record},
};
use agq_language::{ElementId, ElementKind, Tree, validate};
use agq_studio_scene::{SceneInput, SceneTarget};
use agq_system_state::{Checkpoint, Project, compare};
use eframe::egui::{self, RichText};
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
    loaded: bool,
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
}

fn ago(time: i64) -> String {
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

impl StudioApp {
    pub fn history_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let Some(project) = &self.project else {
            ui.label(crate::app::muted(
                "Open a project to see its history.",
                theme,
            ));
            return;
        };
        if !self.history.loaded {
            self.history.reload(project);
        }
        let uncommitted = self.history.uncommitted.unwrap_or(true);
        ui.horizontal(|ui| {
            let button = ui.button("Checkpoint…");
            record(ui.ctx(), Target::Button("Checkpoint…"), button.rect);
            if button.clicked() {
                self.dialog = Some(crate::edit::Dialog::Checkpoint {
                    message: String::new(),
                });
            }
            ui.label(crate::app::muted("Ctrl+S", theme).small());
        });
        theme.section(ui, "CHECKPOINTS · NEWEST FIRST");
        ui.label(if uncommitted {
            RichText::new("●  Now · changes since the last checkpoint").color(theme.accent)
        } else {
            RichText::new("●  Now · same as the last checkpoint").color(theme.muted)
        });
        if let Some(error) = &self.history.error {
            ui.label(
                RichText::new(format!("The history could not be read: {error}")).color(theme.error),
            );
        } else if self.history.checkpoints.is_empty() {
            ui.label(crate::app::muted(
                "No checkpoints yet. Ctrl+S records one.",
                theme,
            ));
        }
        let mut toggled = None;
        for (index, checkpoint) in self.history.checkpoints.iter().enumerate() {
            let selected = self.history.selected.contains(&checkpoint.id);
            let label = format!("{}\n{}", checkpoint.message, ago(checkpoint.time));
            let response = ui.add(
                egui::Button::selectable(selected, label)
                    .min_size(egui::vec2(ui.available_width(), 40.0)),
            );
            if index < 8 {
                record(ui.ctx(), Target::Button(HISTORY_ROWS[index]), response.rect);
            }
            if response.clicked() {
                toggled = Some(checkpoint.id.clone());
            }
        }
        if let Some(id) = toggled {
            if let Some(position) = self.history.selected.iter().position(|s| *s == id) {
                self.history.selected.remove(position);
            } else {
                self.history.selected.push(id);
                if self.history.selected.len() > 2 {
                    self.history.selected.remove(0);
                }
            }
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let label = match self.history.selected.len() {
                1 => "Show changes since then",
                _ => "Show changes between them",
            };
            let show = ui.add_enabled(!self.history.selected.is_empty(), egui::Button::new(label));
            record(ui.ctx(), Target::Button("Show changes"), show.rect);
            if show.clicked() {
                self.compare_selected();
            }
            if self.comparison.is_some() && ui.button("Close comparison").clicked() {
                self.comparison = None;
                self.rebuild();
            }
        });
        self.comparison_list(ui);
    }

    fn compare_selected(&mut self) {
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

    fn comparison_list(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme;
        let Some(comparison) = &self.comparison else {
            return;
        };
        theme.section(ui, "WHAT CHANGED");
        if !comparison.after_is_now {
            ui.label(crate::app::muted(
                "The Surface shows the later checkpoint. Close the comparison to edit.",
                theme,
            ));
        }
        if comparison.created.is_empty()
            && comparison.updated.is_empty()
            && comparison.deleted.is_empty()
        {
            ui.label(crate::app::muted("No differences.", theme));
        }
        let mut focus = None;
        for (id, name) in &comparison.created {
            if ui
                .add(
                    egui::Button::new(RichText::new(format!("+  {name}")).color(theme.green))
                        .frame(false),
                )
                .clicked()
            {
                focus = Some(*id);
            }
        }
        for (id, name) in &comparison.updated {
            if ui
                .add(
                    egui::Button::new(RichText::new(format!("~  {name}")).color(theme.violet))
                        .frame(false),
                )
                .clicked()
            {
                focus = Some(*id);
            }
        }
        for name in &comparison.deleted {
            ui.label(
                RichText::new(format!("−  {name}"))
                    .color(theme.error)
                    .strikethrough(),
            );
        }
        if let Some(id) = focus {
            let target = SceneTarget::Node(id);
            if self.scene.target_bounds(&target).is_some() {
                self.select(target.clone(), false);
                self.frame_target(&target);
            }
        }
    }
}

/// Automation names for the first history rows.
pub const HISTORY_ROWS: [&str; 8] = [
    "History 1",
    "History 2",
    "History 3",
    "History 4",
    "History 5",
    "History 6",
    "History 7",
    "History 8",
];
