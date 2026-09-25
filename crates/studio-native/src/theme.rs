//! Small design system shared by chrome, scene and accessibility treatments.
use eframe::egui::{self, Color32, FontId, RichText, Stroke};

pub const SPACE: f32 = 8.0;
pub const BODY: f32 = 14.0;
pub const CAPTION: f32 = 11.0;
pub const TITLE: f32 = 23.0;
pub const PANEL_WIDTH: f32 = 274.0;

#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub contrast: bool,
    pub canvas: Color32,
    pub surface: Color32,
    pub elevated: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub green: Color32,
    pub amber: Color32,
    pub violet: Color32,
}
impl Theme {
    pub fn new(dark: bool, contrast: bool) -> Self {
        let rgb = Color32::from_rgb;
        if dark {
            Self { dark, contrast, canvas: rgb(17,22,29), surface: rgb(24,30,39), elevated: rgb(33,41,53), border: if contrast {rgb(136,158,183)} else {rgb(51,63,80)}, text: rgb(229,235,243), muted: if contrast {rgb(200,211,224)} else {rgb(143,160,181)}, accent: rgb(121,184,225), green: rgb(126,199,169), amber: rgb(221,184,122), violet: rgb(174,161,221) }
        } else {
            Self { dark, contrast, canvas: rgb(236,240,245), surface: rgb(250,251,253), elevated: rgb(255,255,255), border: if contrast {rgb(72,89,112)} else {rgb(194,205,218)}, text: rgb(33,46,63), muted: rgb(84,103,124), accent: rgb(27,103,153), green: rgb(38,116,89), amber: rgb(140,93,35), violet: rgb(112,84,167) }
        }
    }
    pub fn install(self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.visuals = if self.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        style.visuals.panel_fill = self.surface;
        style.visuals.window_fill = self.surface;
        style.visuals.extreme_bg_color = self.canvas;
        style.visuals.faint_bg_color = self.elevated;
        style.visuals.override_text_color = Some(self.text);
        style.visuals.selection.bg_fill = self.accent.gamma_multiply(0.27);
        style.visuals.selection.stroke = Stroke::new(1.5, self.accent);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border);
        style.visuals.widgets.inactive.weak_bg_fill = self.elevated;
        style.visuals.widgets.active.weak_bg_fill = self.accent.gamma_multiply(0.25);
        style.visuals.widgets.hovered.weak_bg_fill = self.border;
        style.spacing.item_spacing = egui::vec2(SPACE, SPACE);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size.y = 30.0;
        style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(BODY));
        style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(BODY));
        style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(CAPTION));
        ctx.set_style(style);
    }
    pub fn section(self, ui: &mut egui::Ui, text: &str) {
        ui.add_space(14.0);
        ui.label(RichText::new(text).size(CAPTION).color(self.muted).strong());
        ui.add_space(3.0);
    }
}
impl Default for Theme {
    fn default() -> Self { Self::new(true, false) }
}
