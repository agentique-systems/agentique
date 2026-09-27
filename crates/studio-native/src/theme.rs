//! Design tokens for the Studio: palettes, type scale, spacing, radii, strokes,
//! depth and motion. The Studio's chrome, Surface and labels take their colours
//! and sizes from here. Items still marked `allow(dead_code)` are not used yet.
//!
//! The file depends on `egui` only, so prototypes can include it unchanged.
use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Painter,
    Pos2, Rect, RichText, Shadow, Stroke, Vec2,
};
use std::sync::Arc;

// Type scale, in points. Weights come from font families; see `medium`, `semibold`.
/// Welcome and empty-state headlines.
pub const DISPLAY: f32 = 28.0;
/// Panel titles and the selected element's name.
pub const TITLE: f32 = 20.0;
/// Card titles and section headings.
pub const HEADING: f32 = 15.0;
/// Running text, list rows and buttons.
pub const BODY: f32 = 13.5;
/// Dense controls, relationship labels and port names.
pub const LABEL: f32 = 12.0;
/// Metadata, overlines and hints.
pub const CAPTION: f32 = 11.0;
/// Code blocks, identifiers and paths.
pub const CODE: f32 = 12.5;

// Spacing scale, in points. `SPACE` is the base unit.
pub const SPACE_XS: f32 = 2.0;
pub const SPACE_S: f32 = 4.0;
pub const SPACE: f32 = 8.0;
pub const SPACE_L: f32 = 12.0;
pub const SPACE_XL: f32 = 16.0;
pub const PANEL_WIDTH: f32 = 274.0;
/// Text inside inputs (`TextEdit::margin`), so fields match button height.
pub const INPUT_MARGIN: Margin = Margin::symmetric(8, 5);

// Corner radii, in points.
/// Badges, keycaps and port markers.
pub const RADIUS_S: f32 = 4.0;
/// Buttons, inputs and list rows.
pub const RADIUS: f32 = 6.0;
/// Menus, popovers and inline cards in panels.
pub const RADIUS_L: f32 = 10.0;
/// Windows and dialogs.
pub const RADIUS_XL: f32 = 14.0;

// Stroke widths, in points.
pub const HAIRLINE: f32 = 1.0;
/// Selected cards and the active control.
pub const STROKE_SELECTED: f32 = 2.0;
/// Keyboard focus rings.
#[allow(dead_code, reason = "the Surface has no keyboard-focusable cards yet")]
pub const FOCUS_RING: f32 = 2.0;

// Surface shapes and lines, in world units at zoom 1. The shape carries the
// category: rounded cards for parts, near-square requirements, pill-like behaviour.
/// Part and interface cards.
pub const CARD_RADIUS: f32 = 10.0;
/// Containers (parts that show their children).
pub const CONTAINER_RADIUS: f32 = 14.0;
/// Requirement cards.
pub const REQUIREMENT_RADIUS: f32 = 3.0;
/// Actions and states.
#[allow(dead_code, reason = "the subset has no actions or states yet")]
pub const BEHAVIOUR_RADIUS: f32 = 22.0;
pub const EDGE_WIDTH: f32 = 1.25;
/// Connections touching the selection.
pub const EDGE_WIDTH_INCIDENT: f32 = 1.75;
pub const EDGE_WIDTH_SELECTED: f32 = 2.5;
pub const PORT_SIZE: f32 = 9.0;
/// Card shadow: vertical offset and blur radius.
pub const SHADOW_OFFSET: f32 = 3.0;
pub const SHADOW_BLUR: f32 = 14.0;
/// Card shadow opacity under a dark card and under a white card; the renderer
/// interpolates by the card's fill brightness so one rule suits both themes.
pub const SHADOW_OPACITY_DARK: f32 = 0.55;
pub const SHADOW_OPACITY_LIGHT: f32 = 0.11;
/// Smallest size of the lock mark on the Surface, in screen points.
pub const LOCK_MARK: f32 = 12.0;
/// Width of dialog content, in points.
pub const DIALOG_WIDTH: f32 = 420.0;
/// Width of the soft glow around selected and changed elements, in screen points.
pub const GLOW_WIDTH: f32 = 14.0;

// Motion, in seconds.
/// egui's `animation_time` for hover and press transitions. The app applies
/// it, or zero under reduced motion; `Theme::install` leaves it alone.
pub const HOVER_SECONDS: f32 = 0.10;
/// Camera moves: fit view and following the selection (Fluent 2
/// `durationSlow`). Reduced motion makes them instant (`motion`).
pub const CAMERA_SECONDS: f32 = crate::tokens::motion::CAMERA_MS as f32 / 1000.0;
/// How long a changed element stays highlighted.
pub const CHANGED_SECONDS: f32 = 1.5;
/// How long the highlight takes to reach full strength.
pub const CHANGED_RISE_SECONDS: f32 = 0.12;

/// Strength (`0..=1`) of the changed highlight `age` seconds after the change.
/// Zero once the highlight has finished, so callers can stop repainting.
/// `scene.wgsl` mirrors this curve for highlights drawn on the Surface.
#[allow(
    dead_code,
    reason = "the fade runs on the GPU; kept to document the curve"
)]
pub fn changed_intensity(age: f32) -> f32 {
    if !(0.0..CHANGED_SECONDS).contains(&age) {
        return 0.0;
    }
    let rise = (age / CHANGED_RISE_SECONDS).min(1.0);
    let fall =
        1.0 - (age - CHANGED_RISE_SECONDS).max(0.0) / (CHANGED_SECONDS - CHANGED_RISE_SECONDS);
    rise * fall * fall
}

// Fonts: Inter 4.1's variable font (InterVariable.ttf from
// https://github.com/rsms/inter/releases/tag/v4.1) and JetBrains Mono NL 2.304
// (https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304), both under
// the SIL Open Font License 1.1; the licences sit beside the files.
const INTER: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");
const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMonoNL-Regular.ttf");
const MEDIUM: &str = "Inter Medium";
const SEMIBOLD: &str = "Inter SemiBold";
/// The type scale's weights: values of the variable font's `wght` axis.
pub const WEIGHT_REGULAR: f32 = 400.0;
pub const WEIGHT_MEDIUM: f32 = 500.0;
pub const WEIGHT_SEMIBOLD: f32 = 600.0;

/// Regular UI text at `size`.
pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}
/// Medium weight: emphasised rows, card titles, labels on the Surface.
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(MEDIUM.into()))
}
/// Semibold weight: titles and overlines.
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}
/// Code, identifiers and paths.
pub fn code(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Install the Studio fonts. Glyphs Inter lacks (such as ▸ ▾ ≡) come from
/// JetBrains Mono, then from the platform's symbol and CJK fonts (read locally,
/// never redistributed), then from egui's bundled fonts. The definitions are
/// built once per process; repeating the call costs a comparison.
pub fn install_fonts(ctx: &egui::Context) {
    static FONTS: std::sync::OnceLock<FontDefinitions> = std::sync::OnceLock::new();
    ctx.set_fonts(FONTS.get_or_init(font_definitions).clone());
}

fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    // One variable font at three weights: the bytes are shared, and each
    // family reads the `wght` axis at its own value.
    for (name, weight) in [
        ("Inter", WEIGHT_REGULAR),
        (MEDIUM, WEIGHT_MEDIUM),
        (SEMIBOLD, WEIGHT_SEMIBOLD),
    ] {
        let tweak = egui::FontTweak {
            coords: egui::epaint::text::VariationCoords::new([(b"wght", weight)]),
            ..Default::default()
        };
        fonts.font_data.insert(
            name.into(),
            Arc::new(FontData::from_static(INTER).tweak(tweak)),
        );
    }
    fonts.font_data.insert(
        "JetBrains Mono".into(),
        Arc::new(FontData::from_static(MONO_REGULAR)),
    );
    let mut fallback = vec!["JetBrains Mono".to_owned()];
    for (name, paths) in [
        (
            "Platform symbols",
            [
                "C:/Windows/Fonts/seguisym.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
            ],
        ),
        (
            "Platform CJK",
            [
                "C:/Windows/Fonts/msyh.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            ],
        ),
    ] {
        if let Some(bytes) = paths.iter().find_map(|path| std::fs::read(path).ok()) {
            fonts
                .font_data
                .insert(name.into(), Arc::new(FontData::from_owned(bytes)));
            fallback.push(name.into());
        }
    }
    fallback.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    for (family, first) in [
        (FontFamily::Proportional, "Inter"),
        (FontFamily::Name(MEDIUM.into()), MEDIUM),
        (FontFamily::Name(SEMIBOLD.into()), SEMIBOLD),
    ] {
        let list = std::iter::once(first.to_owned())
            .chain(fallback.iter().cloned())
            .collect();
        fonts.families.insert(family, list);
    }
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "JetBrains Mono".into());
    fonts
}

/// One palette. `Theme::new` is the only place colours are chosen.
#[derive(Clone, Copy)]
pub struct Theme {
    pub dark: bool,
    pub contrast: bool,
    /// The Surface background: the lowest layer.
    pub canvas: Color32,
    /// Panels and bars.
    pub surface: Color32,
    /// Cards, inputs, menus and popovers: one step above `surface`.
    pub elevated: Color32,
    /// Hovered rows and controls.
    pub hover: Color32,
    /// Hairline dividers and resting borders.
    pub border: Color32,
    /// Borders that must read at a glance: hovered controls, keycaps.
    pub border_strong: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary text: descriptions, values next to keys.
    pub text_secondary: Color32,
    /// Tertiary text: captions, hints, overlines.
    pub muted: Color32,
    /// Interaction colour: links, selection, primary actions, the part category.
    pub accent: Color32,
    /// Text and icons drawn on an `accent` fill (primary buttons).
    pub on_accent: Color32,
    /// Fill behind selected rows and selected text (translucent accent).
    pub selection: Color32,
    /// Keyboard focus rings.
    pub focus: Color32,
    /// Success and added; the interface and port category.
    pub green: Color32,
    /// Warning; the requirement category.
    pub amber: Color32,
    /// Failure and destructive actions.
    pub error: Color32,
    /// The behaviour category: actions, states, agents.
    pub violet: Color32,
    /// Highlight for elements that just changed, typically by the Assistant.
    pub changed: Color32,
    /// The lock mark on protected elements.
    pub lock: Color32,
    /// Connection lines at rest: opaque, so overlapping segments do not darken.
    pub edge: Color32,
    /// Window and popover shadows.
    pub shadow: Color32,
    /// Dims everything behind a modal dialog.
    pub backdrop: Color32,
}

impl Theme {
    pub fn new(dark: bool, contrast: bool) -> Self {
        let rgb = Color32::from_rgb;
        let pick = |normal: Color32, strong: Color32| if contrast { strong } else { normal };
        if dark {
            Self {
                dark,
                contrast,
                canvas: rgb(15, 16, 19),
                surface: rgb(22, 23, 27),
                elevated: rgb(32, 34, 40),
                hover: rgb(42, 45, 52),
                border: pick(rgb(44, 47, 55), rgb(112, 120, 138)),
                border_strong: pick(rgb(62, 66, 77), rgb(156, 164, 182)),
                text: rgb(233, 235, 239),
                text_secondary: pick(rgb(178, 183, 194), rgb(216, 220, 228)),
                muted: pick(rgb(146, 152, 166), rgb(192, 197, 208)),
                accent: rgb(112, 156, 255),
                on_accent: rgb(10, 14, 26),
                selection: Color32::from_rgba_unmultiplied(112, 156, 255, 52),
                focus: rgb(146, 182, 255),
                green: rgb(88, 200, 140),
                amber: rgb(230, 178, 88),
                error: rgb(240, 108, 108),
                violet: rgb(166, 142, 250),
                changed: rgb(232, 124, 249),
                lock: rgb(214, 219, 229),
                edge: pick(rgb(92, 98, 112), rgb(150, 158, 176)),
                shadow: Color32::from_black_alpha(140),
                backdrop: Color32::from_black_alpha(150),
            }
        } else {
            Self {
                dark,
                contrast,
                canvas: rgb(242, 243, 245),
                surface: rgb(250, 250, 251),
                elevated: rgb(255, 255, 255),
                hover: rgb(235, 237, 241),
                border: pick(rgb(222, 225, 231), rgb(120, 128, 142)),
                border_strong: pick(rgb(198, 203, 212), rgb(84, 92, 106)),
                text: rgb(20, 22, 28),
                text_secondary: pick(rgb(72, 78, 92), rgb(40, 44, 54)),
                muted: pick(rgb(100, 107, 121), rgb(66, 72, 86)),
                accent: rgb(37, 99, 235),
                on_accent: rgb(255, 255, 255),
                selection: Color32::from_rgba_unmultiplied(37, 99, 235, 34),
                focus: rgb(29, 78, 216),
                green: rgb(21, 128, 80),
                amber: rgb(163, 98, 8),
                error: rgb(200, 44, 44),
                violet: rgb(109, 74, 212),
                changed: rgb(192, 38, 211),
                lock: rgb(52, 60, 74),
                edge: pick(rgb(160, 167, 180), rgb(96, 104, 118)),
                shadow: Color32::from_rgba_unmultiplied(16, 24, 40, 34),
                backdrop: Color32::from_black_alpha(80),
            }
        }
    }

    /// Install fonts and apply this palette to egui's visuals and spacing.
    pub fn install(self, ctx: &egui::Context) {
        install_fonts(ctx);
        let theme = if self.dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        ctx.set_theme(if self.dark {
            egui::ThemePreference::Dark
        } else {
            egui::ThemePreference::Light
        });
        let mut style = (*ctx.global_style()).clone();
        style.visuals = self.visuals();
        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(SPACE, 6.0);
        spacing.button_padding = egui::vec2(10.0, 5.0);
        spacing.interact_size.y = 28.0;
        spacing.window_margin = Margin::same(SPACE_XL as i8);
        spacing.menu_margin = Margin::same(6);
        spacing.menu_spacing = SPACE_XS;
        spacing.indent = SPACE_XL;
        spacing.icon_width = 15.0;
        spacing.icon_width_inner = 9.0;
        spacing.icon_spacing = 6.0;
        spacing.scroll = egui::style::ScrollStyle::floating();
        spacing.tooltip_width = 360.0;
        use egui::TextStyle;
        style.text_styles = [
            (TextStyle::Small, regular(CAPTION)),
            (TextStyle::Body, regular(BODY)),
            (TextStyle::Button, regular(BODY)),
            (TextStyle::Heading, semibold(TITLE)),
            (TextStyle::Monospace, code(CODE)),
        ]
        .into();
        ctx.set_style_of(theme, style);
    }

    /// egui visuals built from this palette.
    pub fn visuals(self) -> egui::Visuals {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let widget = |fill: Color32, border: Color32| egui::style::WidgetVisuals {
            bg_fill: fill,
            weak_bg_fill: fill,
            bg_stroke: Stroke::new(HAIRLINE, border),
            corner_radius: CornerRadius::from(RADIUS),
            fg_stroke: Stroke::new(1.5, self.text),
            expansion: 0.0,
        };
        visuals.override_text_color = None;
        visuals.weak_text_color = Some(self.muted);
        visuals.widgets.noninteractive = egui::style::WidgetVisuals {
            fg_stroke: Stroke::new(HAIRLINE, self.text),
            ..widget(self.surface, self.border)
        };
        visuals.widgets.inactive = widget(self.elevated, self.border);
        visuals.widgets.hovered = widget(self.hover, self.border_strong);
        visuals.widgets.active = widget(self.hover, self.accent);
        visuals.widgets.open = widget(self.hover, self.border_strong);
        visuals.selection.bg_fill = self.selection;
        visuals.selection.stroke = Stroke::new(HAIRLINE, self.focus);
        visuals.hyperlink_color = self.accent;
        visuals.panel_fill = self.surface;
        visuals.window_fill = self.elevated;
        visuals.window_stroke = Stroke::new(HAIRLINE, self.border);
        visuals.window_corner_radius = CornerRadius::from(RADIUS_XL);
        visuals.menu_corner_radius = CornerRadius::from(RADIUS_L);
        visuals.window_shadow = Shadow {
            offset: [0, 12],
            blur: 36,
            spread: 0,
            color: self.shadow,
        };
        visuals.popup_shadow = Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: self.shadow,
        };
        visuals.window_highlight_topmost = false;
        visuals.faint_bg_color = self.surface.lerp_to_gamma(self.elevated, 0.5);
        visuals.extreme_bg_color = self.canvas;
        visuals.text_edit_bg_color = Some(if self.dark {
            self.canvas
        } else {
            self.elevated
        });
        visuals.code_bg_color = self.elevated;
        visuals.warn_fg_color = self.amber;
        visuals.error_fg_color = self.error;
        visuals.text_cursor.stroke = Stroke::new(2.0, self.accent);
        visuals
    }

    /// A section overline in a panel: small, spaced, muted capitals.
    pub fn section(self, ui: &mut egui::Ui, text: &str) {
        ui.add_space(SPACE_XL);
        ui.label(self.overline(text));
        ui.add_space(SPACE_S);
    }

    /// Overline text style for section and group headings.
    pub fn overline(self, text: &str) -> RichText {
        RichText::new(text.to_uppercase())
            .font(semibold(CAPTION - 0.5))
            .extra_letter_spacing(0.6)
            .color(self.muted)
    }

    /// Fill of a container on the Surface at nesting `depth`; each level
    /// steps away from the canvas so nesting reads without extra borders.
    pub fn containment(self, depth: usize) -> Color32 {
        let step = (depth.min(3) * 3) as u8;
        if self.dark {
            Color32::from_rgb(19 + step, 20 + step, 24 + step)
        } else {
            Color32::from_rgb(249 - step, 250 - step, 251 - step)
        }
    }

    /// Draw the lock mark, a padlock on a small badge, centred at `center` and
    /// `size` points tall. Readable from 12 points upward.
    pub fn lock_mark(self, painter: &Painter, center: Pos2, size: f32) {
        let badge = Rect::from_center_size(center, Vec2::splat(size));
        painter.rect(
            badge,
            CornerRadius::from(size * 0.3),
            self.elevated,
            Stroke::new(HAIRLINE, self.border_strong),
            egui::StrokeKind::Inside,
        );
        let unit = size / 16.0;
        let body = Rect::from_center_size(
            center + Vec2::new(0.0, 2.0 * unit),
            Vec2::new(8.0 * unit, 6.0 * unit),
        );
        let radius = 2.6 * unit;
        let points: Vec<Pos2> = (0..=12)
            .map(|i| {
                let angle = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
                Pos2::new(
                    center.x + angle.cos() * radius,
                    body.top() + angle.sin() * radius * 1.25,
                )
            })
            .collect();
        painter.add(egui::Shape::line(
            points,
            Stroke::new((1.4 * unit).max(1.0), self.lock),
        ));
        painter.rect_filled(body, CornerRadius::from(1.5 * unit), self.lock);
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(true, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color32) -> f32 {
        let channel = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }
    fn contrast(a: Color32, b: Color32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// `over` composited onto an opaque `under`, as egui blends (gamma space).
    fn composite(over: Color32, under: Color32) -> Color32 {
        let keep = 1.0 - over.a() as f32 / 255.0;
        let channel = |o: u8, u: u8| (o as f32 + u as f32 * keep).round() as u8;
        Color32::from_rgb(
            channel(over.r(), under.r()),
            channel(over.g(), under.g()),
            channel(over.b(), under.b()),
        )
    }

    #[test]
    fn text_meets_wcag_aa_on_every_background_in_both_themes() {
        for dark in [true, false] {
            for high in [false, true] {
                let theme = Theme::new(dark, high);
                let context = format!("dark {dark}, high contrast {high}");
                for background in [theme.canvas, theme.surface, theme.elevated] {
                    assert!(contrast(theme.accent, background) >= 4.5, "{context}");
                }
                // Hovered rows keep their text colours.
                let rows = [theme.canvas, theme.surface, theme.elevated, theme.hover];
                for background in rows {
                    assert!(contrast(theme.text, background) >= 7.0, "{context}");
                    assert!(
                        contrast(theme.text_secondary, background) >= 4.5,
                        "{context}"
                    );
                    assert!(contrast(theme.muted, background) >= 4.5, "{context}");
                }
                // Selected rows: egui draws their text in `focus`; secondary
                // lines use `text_secondary` (never `muted`).
                for under in [theme.surface, theme.elevated] {
                    let selected = composite(theme.selection, under);
                    assert!(contrast(theme.text, selected) >= 7.0, "{context}");
                    assert!(contrast(theme.focus, selected) >= 4.5, "{context}");
                    assert!(contrast(theme.text_secondary, selected) >= 4.5, "{context}");
                }
                assert!(contrast(theme.on_accent, theme.accent) >= 4.5, "{context}");
            }
        }
    }

    #[test]
    fn changed_highlight_rises_quickly_then_fades_to_nothing() {
        assert_eq!(changed_intensity(-0.1), 0.0);
        assert!(changed_intensity(CHANGED_RISE_SECONDS) > 0.99);
        assert!(changed_intensity(0.75) < changed_intensity(0.3));
        assert_eq!(changed_intensity(CHANGED_SECONDS), 0.0);
    }

    #[test]
    fn inter_weights_come_from_the_variable_font() {
        let axes = FontData::from_static(INTER).variation_axes();
        let weight = axes
            .iter()
            .find(|axis| axis.tag == "wght")
            .expect("Inter's variable font has a weight axis");
        for value in [WEIGHT_REGULAR, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD] {
            assert!(
                weight.range.contains(value),
                "{value} in {:?}",
                weight.range
            );
        }
        // Inter widens as it gets heavier, so the same text laid out at the
        // three weights must measure three different widths, at every scale.
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        for scale in [1.0, 1.5, 2.0] {
            ctx.set_pixels_per_point(scale);
            let mut widths = Vec::new();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                for font in [regular(BODY), medium(BODY), semibold(BODY)] {
                    let galley = ui.painter().layout_no_wrap(
                        "Agentique Studio: the Surface".into(),
                        font,
                        Color32::WHITE,
                    );
                    widths.push(galley.size().x);
                }
            })
            .textures_delta
            .clear();
            assert!(
                widths[0] < widths[1] && widths[1] < widths[2],
                "at {scale}x: {widths:?}"
            );
        }
    }
}
