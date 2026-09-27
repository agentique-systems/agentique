//! Names for screen readers (AccessKit, ROADMAP §3.5) where egui has none:
//! Conversation messages and tool cards, and the Surface's cards, which the
//! GPU draws without widgets. Labels are built only while a screen reader
//! (or a test) has turned AccessKit on.
use eframe::egui::{self, accesskit::Role};

/// Whether AccessKit output is being built this frame.
pub fn enabled(ui: &egui::Ui) -> bool {
    // `ui`'s own node always exists while AccessKit is on.
    ui.ctx()
        .accesskit_node_builder(ui.unique_id(), |_| ())
        .is_some()
}

/// Gives `ui`'s own node a role and a label.
pub fn name_ui(ui: &egui::Ui, role: Role, label: impl FnOnce() -> String) {
    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(role);
        node.set_label(label());
    });
}

/// A named node, child of `ui`'s node, for something drawn without a widget
/// at `rect`. Its hit area is empty, so it never takes the pointer.
pub fn name_rect(
    ui: &egui::Ui,
    id: egui::Id,
    rect: egui::Rect,
    role: Role,
    selected: Option<bool>,
    label: impl FnOnce() -> String,
) {
    if !enabled(ui) {
        return;
    }
    ui.interact(
        egui::Rect::from_min_size(rect.min, egui::Vec2::ZERO),
        id,
        egui::Sense::hover(),
    );
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(role);
        node.set_label(label());
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
        if let Some(selected) = selected {
            node.set_selected(selected);
        }
    });
}

/// The labels of every node in an AccessKit update, for tests and journeys.
#[cfg(test)]
pub fn labels(output: &egui::PlatformOutput) -> Vec<(Role, String)> {
    output
        .accesskit_update
        .iter()
        .flat_map(|update| &update.nodes)
        .filter_map(|(_, node)| Some((node.role(), node.label()?.to_string())))
        .collect()
}
