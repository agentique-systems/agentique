//! The component gallery (`--fixture components`; ROADMAP §3.1, R-26, §8.5
//! rule 2): every token and component in one place, in the generated dark,
//! light and high-contrast colours, so drift in the design system is visible
//! at a glance. W5.2 moves the Studio's drawing onto the tokens; a component
//! enters this gallery before it is used.

use crate::theme::{self, Theme};
use crate::tokens::{
    self, Colours, HIGH_CONTRAST, INPUTS, Mode, Rgba, Scale, motion, radius, space, stroke, text,
};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke, Vec2};

/// One colour swatch.
const SWATCH: Vec2 = Vec2::new(space::ROW, space::GRID[6]);
/// The column of names beside swatches and radii.
const NAME_WIDTH: f32 = space::GRID[12];
/// Text fields and progress bars in the primitives row.
const FIELD_WIDTH: f32 = space::GRID[12] * 2.5;

fn color(rgba: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3)
}

fn radius(value: f32) -> CornerRadius {
    CornerRadius::same(value as u8)
}

fn margin(x: f32, y: f32) -> Margin {
    Margin::symmetric(x as i8, y as i8)
}

/// Draws the gallery, filling the space the Surface would take.
pub fn show(ui: &mut egui::Ui, theme: Theme) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(space::GRID[6]);
        heading(
            ui,
            theme,
            "Colours (generated from base hue, accent hue and contrast)",
        );
        for (name, colours) in [
            ("Dark", Colours::generate(INPUTS, Mode::Dark)),
            ("Light", Colours::generate(INPUTS, Mode::Light)),
            (
                "High contrast, dark",
                Colours::generate(HIGH_CONTRAST, Mode::Dark),
            ),
            (
                "High contrast, light",
                Colours::generate(HIGH_CONTRAST, Mode::Light),
            ),
        ] {
            ui.label(RichText::new(name).color(theme.text_secondary));
            for (role, scale) in [
                ("neutral", colours.neutral),
                ("accent", colours.accent),
                ("success", colours.success),
                ("warning", colours.warning),
                ("danger", colours.danger),
                ("info", colours.info),
            ] {
                swatches(ui, theme, role, scale);
            }
            text_sample(ui, colours);
            ui.add_space(space::GRID[5]);
        }

        heading(ui, theme, "Type scale (Inter; weights 400, 500, 600)");
        for size in [
            text::XS,
            text::SM,
            text::BASE,
            text::PROSE,
            text::LG,
            text::XL,
            text::XXL,
        ] {
            ui.horizontal(|ui| {
                ui.add_sized(
                    Vec2::new(NAME_WIDTH, SWATCH.y),
                    egui::Label::new(caption(theme, format!("{size} px"))),
                );
                for font in [
                    theme::regular(size),
                    theme::medium(size),
                    theme::semibold(size),
                ] {
                    ui.label(RichText::new("Link store").font(font).color(theme.text));
                }
            });
        }

        heading(ui, theme, "Spacing (4 px grid), rows and controls");
        ui.horizontal_wrapped(|ui| {
            for step in space::GRID {
                bar(ui, theme, step.max(stroke::HAIRLINE), format!("{step}"));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (name, height) in [
                ("row, compact", space::ROW_COMPACT),
                ("row", space::ROW),
                ("row, comfortable", space::ROW_COMFORTABLE),
                ("control, small", space::CONTROL_SM),
                ("control", space::CONTROL),
                ("control, large", space::CONTROL_LG),
                ("panel padding", space::PANEL_PADDING),
            ] {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(NAME_WIDTH, height), egui::Sense::hover());
                ui.painter().rect_stroke(
                    rect,
                    radius(radius::CONTROL),
                    Stroke::new(stroke::HAIRLINE, theme.border_strong),
                    egui::StrokeKind::Inside,
                );
                ui.label(caption(theme, format!("{name} {height}")));
            }
        });

        heading(ui, theme, "Radii and strokes");
        ui.horizontal_wrapped(|ui| {
            for (name, value) in [
                ("tag", radius::TAG),
                ("control", radius::CONTROL),
                ("card", radius::CARD),
                ("menu", radius::MENU),
                ("dialog", radius::DIALOG),
            ] {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(NAME_WIDTH, space::GRID[10]),
                    egui::Sense::hover(),
                );
                ui.painter().rect_stroke(
                    rect,
                    radius(value),
                    Stroke::new(stroke::HAIRLINE, theme.border_strong),
                    egui::StrokeKind::Inside,
                );
                ui.label(caption(theme, name));
            }
            // A pill: half the height (`radius::FULL`).
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(NAME_WIDTH, SWATCH.y), egui::Sense::hover());
            ui.painter().rect_stroke(
                rect,
                radius((rect.height() / 2.0).min(radius::FULL)),
                Stroke::new(stroke::HAIRLINE, theme.border_strong),
                egui::StrokeKind::Inside,
            );
            ui.label(caption(theme, "full"));
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(NAME_WIDTH, SWATCH.y), egui::Sense::hover());
            ui.painter().rect_stroke(
                rect,
                radius(radius::CONTROL),
                Stroke::new(stroke::FOCUS, theme.accent),
                egui::StrokeKind::Outside,
            );
            ui.label(caption(theme, "focus ring"));
        });

        heading(ui, theme, "Motion (Fluent 2 durations and curves)");
        for line in [
            format!(
                "press and hover-in {} ms · hover-out {} ms · panels {} ms · camera {} ms",
                motion::PRESS_MS,
                motion::HOVER_OUT_MS,
                motion::PANEL_MS,
                motion::CAMERA_MS
            ),
            format!(
                "reduced motion: jumps, fades ≤ {} ms · change highlight holds {} ms, fades {} ms",
                motion::REDUCED_FADE_MS,
                motion::CHANGE_HOLD_MS,
                motion::CHANGE_FADE_MS
            ),
            format!(
                "curves: decelerate {:?} · accelerate {:?} · easy ease {:?}",
                motion::DECELERATE,
                motion::ACCELERATE,
                motion::EASY_EASE
            ),
        ] {
            ui.label(RichText::new(line).color(theme.text_secondary));
        }

        heading(
            ui,
            theme,
            "Primitives today, at rest, disabled and selected",
        );
        ui.horizontal_wrapped(|ui| {
            ui.add(crate::edit::primary_button(theme, "Primary"));
            let _ = ui.button("Secondary");
            ui.add_enabled(false, egui::Button::new("Disabled"));
            let _ = ui.selectable_label(true, "Selected");
            let _ = ui.small_button("Small");
        });
        ui.horizontal_wrapped(|ui| {
            let mut field = String::from("Text field");
            ui.add(egui::TextEdit::singleline(&mut field).desired_width(FIELD_WIDTH));
            let mut hidden = String::from("hidden text");
            ui.add(
                egui::TextEdit::singleline(&mut hidden)
                    .password(true)
                    .desired_width(FIELD_WIDTH),
            );
            let mut on = true;
            ui.checkbox(&mut on, "Checkbox");
            let mut choice = 1;
            ui.radio_value(&mut choice, 1, "Radio");
            egui::ComboBox::from_id_salt("gallery-select")
                .selected_text("Select")
                .show_ui(ui, |ui| {
                    let _ = ui.selectable_label(true, "One");
                    let _ = ui.selectable_label(false, "Two");
                });
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::Spinner::new().size(text::LG));
            ui.add(egui::ProgressBar::new(0.6).desired_width(FIELD_WIDTH));
            key_cap(ui, theme, "Ctrl+,");
        });

        heading(
            ui,
            theme,
            "Components (W5.2 builds them, each in these states)",
        );
        ui.label(
            RichText::new(format!("Every component: {}", tokens::STATES.join(" · ")))
                .color(theme.text_secondary),
        );
        let mut group = "";
        for component in tokens::COMPONENTS {
            if component.group != group {
                group = component.group;
                ui.add_space(space::GRID[3]);
                ui.label(
                    RichText::new(group)
                        .font(theme::semibold(text::SM))
                        .color(theme.text),
                );
            }
            let mut line = component.name.to_string();
            if !component.variants.is_empty() {
                line.push_str(&format!(": {}", component.variants.join(", ")));
            }
            if !component.states.is_empty() {
                line.push_str(&format!(" (states: {})", component.states.join(", ")));
            }
            ui.label(
                RichText::new(line)
                    .font(theme::regular(text::SM))
                    .color(theme.muted),
            );
        }
    });
}

fn caption(theme: Theme, text: impl Into<String>) -> RichText {
    RichText::new(text)
        .font(theme::regular(text::XS))
        .color(theme.muted)
}

fn heading(ui: &mut egui::Ui, theme: Theme, title: &str) {
    ui.add_space(space::GRID[5]);
    ui.label(
        RichText::new(title)
            .font(theme::semibold(text::LG))
            .color(theme.text),
    );
    ui.add_space(space::GRID[3]);
}

fn bar(ui: &mut egui::Ui, theme: Theme, width: f32, label: String) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, space::GRID[5]), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.accent);
    ui.label(caption(theme, label));
}

fn swatches(ui: &mut egui::Ui, theme: Theme, role: &str, scale: Scale) {
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(NAME_WIDTH, SWATCH.y),
            egui::Label::new(caption(theme, role)),
        );
        for step in 1..=12 {
            let (rect, response) = ui.allocate_exact_size(SWATCH, egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, radius(radius::TAG), color(scale.step(step)));
            response.on_hover_text(format!("{role} {step}"));
        }
    });
}

/// Body text at steps 11 and 12 on backgrounds 1–3, as the contrast rule
/// requires (§3.5), and text on the accent's solid colour.
fn text_sample(ui: &mut egui::Ui, colours: Colours) {
    ui.horizontal(|ui| {
        for background in 1..=3 {
            egui::Frame::new()
                .fill(color(colours.neutral.step(background)))
                .inner_margin(margin(space::GRID[4], space::GRID[2]))
                .show(ui, |ui| {
                    ui.label(RichText::new("Step 12 text").color(color(colours.neutral.step(12))));
                    ui.label(RichText::new("Step 11 text").color(color(colours.neutral.step(11))));
                    ui.label(RichText::new("Accent 11").color(color(colours.accent.step(11))));
                });
        }
        egui::Frame::new()
            .fill(color(colours.accent.step(9)))
            .corner_radius(radius(radius::CONTROL))
            .inner_margin(margin(space::GRID[4], space::GRID[2]))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("On accent 9").color(color(tokens::on_solid(colours.accent))),
                );
            });
    });
}

fn key_cap(ui: &mut egui::Ui, theme: Theme, keys: &str) {
    egui::Frame::new()
        .stroke(Stroke::new(stroke::HAIRLINE, theme.border_strong))
        .corner_radius(radius(radius::CONTROL))
        .inner_margin(margin(space::GRID[3], space::GRID[1]))
        .show(ui, |ui| {
            ui.label(
                RichText::new(keys)
                    .font(theme::medium(text::XS))
                    .color(theme.text_secondary),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A smoke test: the gallery draws in both themes without a panic and
    /// shows the component list and every generated theme.
    #[test]
    fn the_gallery_draws_in_both_themes() {
        for dark in [true, false] {
            let context = egui::Context::default();
            let theme = Theme::new(dark, false);
            theme.install(&context);
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1200.0, 20_000.0),
                    )),
                    ..Default::default()
                },
                |ui| show(ui, theme),
            );
            output.textures_delta.clear();
            let drawn = format!("{:?}", output.shapes);
            assert!(
                drawn.contains("command palette: groups"),
                "the list is drawn"
            );
            assert!(drawn.contains("High contrast, light"));
        }
    }
}
