//! Geometry from ordinary widgets for opt-in real-runtime input qualification.
//! Recording a rectangle never changes application or semantic state.
use agq_kernel::ElementId;
use agq_modeling_repository::{ProjectId, ProjectRevisionId};
use eframe::egui::{self, Rect};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Project(ProjectId),
    ExplorerSearch,
    ExplorerElement(ElementId),
    ComparisonCurrent,
    ComparisonCandidate,
    ComparisonDiff,
    Standards,
    InspectorElement(ElementId),
    InspectorRelationship(String),
    HistoryRevision(ProjectRevisionId),
    HistoryReturnHead,
}

#[derive(Clone)]
struct Recorded {
    rect: Rect,
    frame: u64,
}

pub fn record(ctx: &egui::Context, target: Target, rect: Rect) {
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("native-real-input-target", target)),
            Recorded { rect, frame },
        );
    });
}

pub fn target(ctx: &egui::Context, key: Target) -> Result<Rect, String> {
    ctx.data(|data| {
        data.get_temp::<Recorded>(egui::Id::new(("native-real-input-target", key.clone())))
    })
    .filter(|item| {
        item.rect.is_finite()
            && item.rect.is_positive()
            && ctx.cumulative_frame_nr().saturating_sub(item.frame) <= 2
            && ctx.viewport_rect().contains_rect(item.rect)
    })
    .map(|item| item.rect)
    .ok_or_else(|| format!("Visible current widget geometry unavailable for {key:?}"))
}
