//! Requirement navigation stays bound to the displayed immutable revision.
use crate::{
    app::{ComparisonMode, StudioApp, muted},
    navigation::World,
};
use agq_modeling_view::ViewDefinition;
use agq_studio_scene::{Point, RequirementRole, requirement_roles};
use eframe::egui::{self, Align2, FontId, Stroke, Vec2};
use std::collections::BTreeMap;

impl StudioApp {
    pub fn show_requirements_selection(&mut self) {
        let Some(focus) = self.selected_element() else {
            return;
        };
        if self.comparison == ComparisonMode::Diff {
            self.status =
                "Review Current or Candidate before opening a requirement neighborhood".into();
            return;
        }
        self.remember_location();
        self.restore_world_filters(World::Requirements);
        self.world = World::Requirements;
        self.focus = Some(focus);
        self.expanded = None;
        self.search.clear();
        self.fit_pending = true;
        self.request_projection_definition(ViewDefinition {
            focus: Some(focus),
            ..ViewDefinition::requirements()
        });
        self.status = "Requirements concerning the selected element · exact modeled links".into();
    }

    pub fn requirements_summary(&mut self, ui: &mut egui::Ui) {
        let roles = requirement_roles(self.active_projection());
        let count = |role| roles.values().filter(|value| **value == role).count();
        ui.horizontal_wrapped(|ui| {
            ui.label(muted(format!("{} obligations · {} subjects · {} satisfaction declarations · {} verification elements",
                count(RequirementRole::Requirement), count(RequirementRole::Subject),
                count(RequirementRole::Satisfaction), count(RequirementRole::Verification)), self.theme).small());
            if self.focus.is_some() && ui.small_button("All requirements").clicked() {
                self.focus = None;
                self.request_projection_definition(ViewDefinition::requirements());
            }
        });
        ui.label(muted("Follow the subject to the system it concerns. Modeled links do not establish satisfaction or verification success.", self.theme).small());
    }

    pub fn requirement_lane_labels(&self, painter: &egui::Painter, viewport: egui::Rect) {
        if self.world != World::Requirements {
            return;
        }
        let roles = requirement_roles(self.active_projection());
        let mut lanes = BTreeMap::<RequirementRole, (f32, f32, f32)>::new();
        for node in &self.scene.nodes {
            let role = roles
                .get(&node.id())
                .copied()
                .unwrap_or(RequirementRole::Context);
            lanes
                .entry(role)
                .and_modify(|bounds| {
                    bounds.0 = bounds.0.min(node.bounds.min.x);
                    bounds.1 = bounds.1.max(node.bounds.max.x);
                    bounds.2 = bounds.2.min(node.bounds.min.y);
                })
                .or_insert((node.bounds.min.x, node.bounds.max.x, node.bounds.min.y));
        }
        for (role, (left, right, top)) in lanes {
            let a = self.camera.world_to_screen(Point::new(left, top - 30.0));
            let b = self.camera.world_to_screen(Point::new(right, top - 30.0));
            let start = viewport.min + Vec2::new(a.x, a.y);
            painter.text(
                start,
                Align2::LEFT_BOTTOM,
                role.label(),
                FontId::proportional(11.0),
                self.theme.muted,
            );
            painter.line_segment(
                [
                    start + Vec2::new(0.0, 8.0),
                    viewport.min + Vec2::new(b.x, b.y + 8.0),
                ],
                Stroke::new(1.0, self.theme.border),
            );
        }
    }
}
