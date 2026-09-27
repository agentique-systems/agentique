//! The component gallery (`--fixture components`; ROADMAP §3.1, R-26, §8.5
//! rule 2): every token and component in one place, in the generated dark,
//! light and high-contrast palettes, so drift in the design system is
//! visible at a glance. W5.2 moves the Studio's drawing onto the tokens; a
//! component enters this gallery before it is used.

use crate::theme::{self, Theme};
use crate::tokens::{self, INPUTS, Mode, Palette, Rgba, Scale, ThemeInputs};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

fn color(rgba: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba.0, rgba.1, rgba.2, rgba.3)
}

/// Draws the gallery, filling the space the Surface would take.
pub fn show(ui: &mut egui::Ui, theme: Theme) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(tokens::space::GRID[6]);
        heading(ui, theme, "Palettes (generated from base hue, accent hue and contrast)");
        let high_contrast = ThemeInputs {
            contrast: 1.25,
            ..INPUTS
        };
        for (name, palette) in [
            ("Dark", Palette::generate(INPUTS, Mode::Dark)),
            ("Light", Palette::generate(INPUTS, Mode::Light)),
            ("High contrast, dark", Palette::generate(high_contrast, Mode::Dark)),
        ] {
            ui.label(RichText::new(name).color(theme.text_secondary));
            for (role, scale) in [
                ("neutral", palette.neutral),
                ("accent", palette.accent),
                ("success", palette.success),
                ("warning", palette.warning),
                ("danger", palette.danger),
                ("info", palette.info),
            ] {
                swatches(ui, theme, role, scale);
            }
            text_sample(ui, palette);
            ui.add_space(tokens::space::GRID[5]);
        }

        heading(ui, theme, "Type scale (Inter; weights 400, 500, 600)");
        for size in [
            tokens::text::XS,
            tokens::text::SM,
            tokens::text::BASE,
            tokens::text::PROSE,
            tokens::text::LG,
            tokens::text::XL,
            tokens::text::XXL,
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{size:>4} px")).font(theme::regular(tokens::text::XS)).color(theme.muted));
                ui.label(RichText::new("Link store").font(theme::regular(size)).color(theme.text));
                ui.label(RichText::new("Link store").font(theme::medium(size)).color(theme.text));
                ui.label(RichText::new("Link store").font(theme::semibold(size)).color(theme.text));
            });
        }

        heading(ui, theme, "Spacing (4 px grid) and radii");
        ui.horizontal_wrapped(|ui| {
            for step in tokens::space::GRID {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(step.max(1.0), 12.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, theme.accent);
                ui.label(RichText::new(format!("{step}")).font(theme::regular(tokens::text::XS)).color(theme.muted));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (name, radius) in [
                ("tag", tokens::radius::TAG),
                ("control", tokens::radius::CONTROL),
                ("card", tokens::radius::CARD),
                ("menu", tokens::radius::MENU),
                ("dialog", tokens::radius::DIALOG),
            ] {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 40.0), egui::Sense::hover());
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(radius as u8),
                    Stroke::new(tokens::stroke::HAIRLINE, theme.border_strong),
                    egui::StrokeKind::Inside,
                );
                ui.label(RichText::new(name).font(theme::regular(tokens::text::XS)).color(theme.muted));
            }
        });

        heading(ui, theme, "Motion");
        ui.label(
            RichText::new(format!(
                "press and hover-in {} ms · hover-out {} ms · panels {} ms · camera {} ms · reduced motion: jumps, fades ≤ {} ms · change highlight holds {} ms, fades {} ms",
                tokens::motion::PRESS_MS,
                tokens::motion::HOVER_OUT_MS,
                tokens::motion::PANEL_MS,
                tokens::motion::CAMERA_MS,
                tokens::motion::REDUCED_FADE_MS,
                tokens::motion::CHANGE_HOLD_MS,
                tokens::motion::CHANGE_FADE_MS
            ))
            .color(theme.text_secondary),
        );

        heading(ui, theme, "Primitives, at rest, disabled and selected");
        ui.horizontal_wrapped(|ui| {
            ui.add(crate::edit::primary_button(theme, "Primary"));
            let _ = ui.button("Secondary");
            ui.add_enabled(false, egui::Button::new("Disabled"));
            let _ = ui.selectable_label(true, "Selected");
            let _ = ui.small_button("Small");
        });
        ui.horizontal_wrapped(|ui| {
            let mut text = String::from("Text field");
            ui.add(egui::TextEdit::singleline(&mut text).desired_width(160.0));
            let mut hidden = String::from("hidden text");
            ui.add(egui::TextEdit::singleline(&mut hidden).password(true).desired_width(160.0));
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
            ui.add(egui::Spinner::new().size(14.0));
            ui.add(egui::ProgressBar::new(0.6).desired_width(160.0));
            key_cap(ui, theme, "Ctrl+,");
        });

        heading(ui, theme, "Components to build (W5.2), each in these states");
        ui.label(RichText::new(tokens::STATES.join(" · ")).color(theme.text_secondary));
        ui.label(RichText::new(format!("Work states: {}", tokens::WORK_STATES.join(" · "))).color(theme.text_secondary));
        for (component, variants) in tokens::COMPONENTS {
            let text = if variants.is_empty() {
                component.to_string()
            } else {
                format!("{component}: {}", variants.join(", "))
            };
            ui.label(RichText::new(text).font(theme::regular(tokens::text::SM)).color(theme.muted));
        }
    });
}

fn heading(ui: &mut egui::Ui, theme: Theme, text: &str) {
    ui.add_space(tokens::space::GRID[5]);
    ui.label(
        RichText::new(text)
            .font(theme::semibold(tokens::text::LG))
            .color(theme.text),
    );
    ui.add_space(tokens::space::GRID[3]);
}

fn swatches(ui: &mut egui::Ui, theme: Theme, role: &str, scale: Scale) {
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(64.0, 18.0),
            egui::Label::new(
                RichText::new(role)
                    .font(theme::regular(tokens::text::XS))
                    .color(theme.muted),
            ),
        );
        for step in 1..=12 {
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(28.0, 18.0), egui::Sense::hover());
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(tokens::radius::TAG as u8),
                color(scale.step(step)),
            );
            response.on_hover_text(format!("{role} {step}"));
        }
    });
}

/// Body text at steps 11 and 12 on backgrounds 1–3, as the contrast rule
/// requires (§3.5).
fn text_sample(ui: &mut egui::Ui, palette: Palette) {
    ui.horizontal(|ui| {
        for background in 1..=3 {
            egui::Frame::new()
                .fill(color(palette.neutral.step(background)))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.label(RichText::new("Step 12 text").color(color(palette.neutral.step(12))));
                    ui.label(RichText::new("Step 11 text").color(color(palette.neutral.step(11))));
                    ui.label(RichText::new("Accent 11").color(color(palette.accent.step(11))));
                });
        }
    });
}

fn key_cap(ui: &mut egui::Ui, theme: Theme, keys: &str) {
    egui::Frame::new()
        .stroke(Stroke::new(tokens::stroke::HAIRLINE, theme.border_strong))
        .corner_radius(CornerRadius::same(tokens::radius::CONTROL as u8))
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new(keys)
                    .font(theme::medium(tokens::text::XS))
                    .color(theme.text_secondary),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gallery_draws_in_both_themes_and_lists_every_component() {
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
            for (component, _) in tokens::COMPONENTS {
                assert!(drawn.contains(component), "{component} is not listed");
            }
        }
    }
}
