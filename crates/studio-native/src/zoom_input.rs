//! Wheel gestures belong to their event-time pointer, never a later hover.
use agq_studio_scene::{Camera2D, Point};
use eframe::egui::{self, Event, MouseWheelUnit, Pos2};

#[derive(Clone, Default)]
struct WheelInput {
    pointer: Option<Pos2>,
    frame: Option<u64>,
}

/// Consume ordered wheel events once per egui frame. Toolkit scroll smoothing
/// cannot be used here: its residual deltas have no pointer ownership and can
/// retarget when the pointer moves, a panel opens, or a monitor changes DPI.
pub fn apply(ui: &egui::Ui, response: &egui::Response, camera: &mut Camera2D) -> bool {
    let key = response.id.with("wheel-input");
    let mut state = ui
        .ctx()
        .data(|data| data.get_temp::<WheelInput>(key))
        .unwrap_or_default();
    let frame = ui.ctx().cumulative_frame_nr();
    if state.frame == Some(frame) {
        return false;
    }
    state.frame = Some(frame);
    let mut changed = false;
    let events = ui.input(|input| input.events.clone());
    if state.pointer.is_none()
        && !events.iter().any(|event| {
            matches!(
                event,
                Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone
            )
        })
    {
        // A new canvas can appear under a stationary pointer. Its toolkit
        // position is safe only when this batch contains no later pointer move.
        state.pointer = ui.input(|input| input.pointer.latest_pos());
    }
    let line_speed = ui
        .ctx()
        .options(|options| options.input_options.line_scroll_speed);
    for event in &events {
        match event {
            Event::PointerMoved(pointer) => state.pointer = Some(*pointer),
            Event::PointerButton { pos, .. } => state.pointer = Some(*pos),
            Event::PointerGone => state.pointer = None,
            Event::MouseWheel { unit, delta, .. } => {
                if let Some(pointer) = state.pointer.filter(|pointer| {
                    response.rect.contains(*pointer)
                        && ui.ctx().layer_id_at(*pointer) == Some(response.layer_id)
                }) {
                    let points = match unit {
                        MouseWheelUnit::Point => delta.y,
                        MouseWheelUnit::Line => delta.y * line_speed,
                        MouseWheelUnit::Page => delta.y * response.rect.height(),
                    };
                    if points.is_finite() && points != 0.0 {
                        camera.zoom_at(
                            Point::new(
                                pointer.x - response.rect.left(),
                                pointer.y - response.rect.top(),
                            ),
                            (points * 0.0025).clamp(-20.0, 20.0).exp(),
                        );
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, state));
    changed
}

#[cfg(test)]
mod tests;
