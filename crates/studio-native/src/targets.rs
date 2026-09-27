//! Widget geometry for the scripted input driver (`--features automation`).
//! Ordinary widget construction records its rectangle; the driver clicks it
//! through egui RawInput. Without the feature, outside tests, recording is a no-op.
use eframe::egui::{self, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Viewport,
    PaletteInput,
    /// A text field, by its label.
    Field(&'static str),
    /// A button, by its label.
    Button(&'static str),
    /// An element link in the Conversation, by the element's raw id.
    Link(u64),
    /// A Conversation message's text, by its entry and slot.
    Message(usize, usize),
}

#[cfg(any(test, feature = "automation"))]
pub fn record(ctx: &egui::Context, target: Target, rect: Rect) {
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new(("native-interaction-target", target)), rect)
    });
}

#[cfg(not(any(test, feature = "automation")))]
#[inline(always)]
pub fn record(_: &egui::Context, _: Target, _: Rect) {}

#[cfg(feature = "automation")]
pub fn target(ctx: &egui::Context, key: Target) -> Result<Rect, String> {
    ctx.data(|data| data.get_temp::<Rect>(egui::Id::new(("native-interaction-target", key))))
        .filter(|rect| rect.is_finite() && rect.is_positive())
        .ok_or_else(|| format!("UI geometry for {key:?} has not been recorded"))
}
