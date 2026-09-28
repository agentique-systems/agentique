//! The theme the Studio draws with: the generated colour scales of
//! `tokens.rs` turned into roles (§3.2), plus type. Every view reads it
//! through [`ActiveTheme`]; nothing hard-codes a value the tokens cover.
use crate::tokens::{self, Colours, Mode, Rgba, Scale};
use gpui::{
    App, BoxShadow, FontWeight, Global, Hsla, Pixels, Rems, SharedString, hsla, point, px, rems,
};

/// Inter's variable font (Inter 4.1), at the weights of the type scale.
pub const SANS: &str = "Inter";
/// Names, values and code.
pub const MONO: &str = "JetBrains Mono NL";

/// A size on the 16-point rem, so the UI scale (`window.set_rem_size`)
/// scales it: `r(13.0)` is 13 points at 100%.
pub fn r(points: f32) -> Rems {
    rems(points / 16.0)
}

pub fn hsla_of(colour: Rgba) -> Hsla {
    gpui::Rgba {
        r: f32::from(colour.0) / 255.0,
        g: f32::from(colour.1) / 255.0,
        b: f32::from(colour.2) / 255.0,
        a: f32::from(colour.3) / 255.0,
    }
    .into()
}

/// One role scale in the three uses a view needs.
#[derive(Clone, Copy, Debug)]
pub struct Role {
    /// Step 3: a tinted background (a selected row, a soft chip).
    pub soft: Hsla,
    /// Step 5: a tinted background under the pointer.
    pub soft_hover: Hsla,
    /// Step 7: a tinted border.
    pub border: Hsla,
    /// Step 9: the solid colour (a primary button, a line, a mark).
    pub solid: Hsla,
    /// Step 10: the solid colour under the pointer.
    pub solid_hover: Hsla,
    /// Step 11: text in the role's colour on the app's backgrounds.
    pub text: Hsla,
    /// Text on the solid colour.
    pub on_solid: Hsla,
}

impl Role {
    fn of(scale: Scale) -> Role {
        Role {
            soft: hsla_of(scale.step(3)),
            soft_hover: hsla_of(scale.step(5)),
            border: hsla_of(scale.step(7)),
            solid: hsla_of(scale.step(9)),
            solid_hover: hsla_of(scale.step(10)),
            text: hsla_of(scale.step(11)),
            on_solid: hsla_of(tokens::on_solid(scale)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub dark: bool,
    pub contrast: bool,
    /// The Surface, behind everything the model shows.
    pub canvas: Hsla,
    /// The window's chrome: title bar, status bar, docked panels.
    pub chrome: Hsla,
    /// Controls and cards raised above the chrome or the canvas.
    pub raised: Hsla,
    /// Menus, the palette, dialogs and tooltips.
    pub overlay: Hsla,
    /// A field's inset background.
    pub inset: Hsla,
    pub hover: Hsla,
    pub pressed: Hsla,
    /// Hairlines between regions.
    pub separator: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_muted: Hsla,
    /// Placeholders and disabled text (3:1 at least).
    pub text_faint: Hsla,
    /// Selection, focus.
    pub accent: Role,
    /// Added, passed.
    pub success: Role,
    /// A lock needing confirmation, drift, a problem.
    pub warning: Role,
    /// A validation error, removed.
    pub danger: Role,
    /// The Assistant's activity.
    pub info: Role,
    /// Behind a dialog.
    pub backdrop: Hsla,
    pub shadow: Hsla,
    /// Surface edges and the dot grid.
    pub edge: Hsla,
    pub dots: Hsla,
}

impl Theme {
    pub fn new(dark: bool, contrast: bool) -> Theme {
        let inputs = if contrast {
            tokens::HIGH_CONTRAST
        } else {
            tokens::INPUTS
        };
        let mode = if dark { Mode::Dark } else { Mode::Light };
        let colours = Colours::generate(inputs, mode);
        let n = |step| hsla_of(colours.neutral.step(step));
        // Dark: the canvas is the deepest step and chrome sits one above it;
        // light: the chrome is the palest and the canvas one below it, so the
        // Surface reads as the working area in both.
        let (canvas, chrome, raised, overlay, inset) = if dark {
            (n(1), n(2), n(3), n(3), n(1))
        } else {
            (n(2), n(1), n(1), n(1), n(1))
        };
        Theme {
            dark,
            contrast,
            canvas,
            chrome,
            raised,
            overlay,
            inset,
            hover: n(4),
            pressed: n(5),
            separator: if contrast { n(7) } else { n(5) },
            border: if contrast { n(8) } else { n(6) },
            border_strong: n(8),
            text: n(12),
            text_secondary: n(11),
            text_muted: if contrast { n(11) } else { n(10) },
            text_faint: n(9),
            accent: Role::of(colours.accent),
            success: Role::of(colours.success),
            warning: Role::of(colours.warning),
            danger: Role::of(colours.danger),
            info: Role::of(colours.info),
            backdrop: if dark {
                hsla(0.0, 0.0, 0.0, 0.45)
            } else {
                hsla(0.0, 0.0, 0.0, 0.18)
            },
            shadow: if dark {
                hsla(0.0, 0.0, 0.0, 0.5)
            } else {
                hsla(220.0 / 360.0, 0.3, 0.2, 0.14)
            },
            edge: n(9),
            dots: if dark { n(5) } else { n(6) },
        }
    }

    /// Elevation (§3.2): shadows only for overlays and a dragged card.
    pub fn shadow_overlay(&self) -> Vec<BoxShadow> {
        vec![
            BoxShadow {
                color: self.shadow,
                offset: point(px(0.0), px(12.0)),
                blur_radius: px(32.0),
                spread_radius: px(-4.0),
                inset: false,
            },
            BoxShadow {
                color: self.shadow.opacity(0.5),
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(6.0),
                spread_radius: px(0.0),
                inset: false,
            },
        ]
    }

    pub fn shadow_small(&self) -> Vec<BoxShadow> {
        vec![BoxShadow {
            color: self.shadow.opacity(0.55),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(3.0),
            spread_radius: px(0.0),
            inset: false,
        }]
    }

    /// Colour by who made a change (§3.2 change highlight): the Operator's
    /// edits in an accent tint, the Assistant's in its own colour.
    pub fn actor(&self, actor: agq_system_state::Actor) -> Hsla {
        match actor {
            agq_system_state::Actor::Operator => self.accent.solid,
            agq_system_state::Actor::Assistant => self.info.solid,
        }
    }

    /// Shares the colours with gpui-base, whose text fields and scroll bars
    /// read its own theme.
    pub fn base_theme(&self) -> gpui_base::Theme {
        let mut base = gpui_base::Theme {
            appearance: if self.dark {
                gpui_base::ThemeAppearance::Dark
            } else {
                gpui_base::ThemeAppearance::Light
            },
            ..gpui_base::Theme::default()
        };
        let colors = &mut base.tokens.colors;
        colors.background = self.chrome;
        colors.foreground = self.text;
        colors.surface = self.raised;
        colors.surface_foreground = self.text;
        colors.primary = self.accent.solid;
        colors.primary_foreground = self.accent.on_solid;
        colors.secondary = self.hover;
        colors.secondary_foreground = self.text;
        colors.muted = self.hover;
        colors.muted_foreground = self.text_muted;
        colors.accent = self.accent.soft;
        colors.accent_foreground = self.text;
        colors.destructive = self.danger.solid;
        colors.destructive_foreground = self.danger.on_solid;
        colors.border = self.border;
        colors.input = self.border;
        colors.ring = self.accent.solid;
        colors.selection = self.accent.solid.opacity(0.32);
        base.tokens.typography.sans = SharedString::new_static(SANS);
        base.tokens.typography.mono = SharedString::new_static(MONO);
        base
    }
}

impl Global for Theme {}

/// The active theme, from any context.
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

/// The type scale (§3.2): sizes in points before the UI scale.
pub mod text {
    pub use crate::tokens::text::*;
}

/// Weights of the type scale, as GPUI takes them.
pub const REGULAR: FontWeight = FontWeight(crate::tokens::text::REGULAR as f32);
pub const MEDIUM: FontWeight = FontWeight(crate::tokens::text::MEDIUM as f32);
pub const SEMIBOLD: FontWeight = FontWeight(crate::tokens::text::SEMIBOLD as f32);

/// A px value that does not scale with the UI (hairlines).
pub fn hairline() -> Pixels {
    px(crate::tokens::stroke::HAIRLINE)
}
