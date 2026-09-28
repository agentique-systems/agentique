//! Design tokens (ROADMAP §3.2, R-26; interface 3 of §6.2): the one place
//! for every colour, size, radius, duration and easing the Studio draws with
//! (§8.5 rule 1). Starting values from §3.2, tuned with the Operator on real
//! models in this file only.
//!
//! Colours are generated, never hand-set: a neutral 12-step scale and five
//! role scales (accent, success, warning, danger, info), each with Radix
//! Colors' step roles (backgrounds 1–5, borders 6–8, solids 9–10, text
//! 11–12) [63], from three inputs (base hue, accent hue, contrast), in
//! OKLCH, so the dark, light and high-contrast themes cannot drift apart
//! (as Linear generates its themes in LCH [49]).
//!
//! This module depends on nothing but `std`; `theme.rs` moves onto it in W5.2.

// ---- Type (§3.2): Inter for UI, a monospace face for names and values.
pub mod text {
    pub const XS: f32 = 11.0;
    pub const SM: f32 = 12.0;
    /// Default UI text.
    pub const BASE: f32 = 13.0;
    /// Conversation prose.
    pub const PROSE: f32 = 14.0;
    pub const LG: f32 = 16.0;
    /// Inter Display from here up.
    pub const XL: f32 = 20.0;
    pub const XXL: f32 = 28.0;
    pub const REGULAR: u16 = 400;
    pub const MEDIUM: u16 = 500;
    pub const SEMIBOLD: u16 = 600;
}

// ---- Spacing (§3.2): a 4 px grid.
pub mod space {
    pub const GRID: [f32; 13] = [
        0.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 40.0, 48.0, 64.0,
    ];
    pub const ROW_COMPACT: f32 = 24.0;
    pub const ROW: f32 = 28.0;
    pub const ROW_COMFORTABLE: f32 = 32.0;
    pub const CONTROL_SM: f32 = 24.0;
    pub const CONTROL: f32 = 28.0;
    pub const CONTROL_LG: f32 = 32.0;
    pub const PANEL_PADDING: f32 = 12.0;
}

// ---- Radii (§3.2).
// Tuned for the GPUI redesign (C-48): one step rounder than §3.2's starting
// values, which read sharp at 150% and 200%.
pub mod radius {
    pub const TAG: f32 = 4.0;
    pub const CONTROL: f32 = 6.0;
    pub const CARD: f32 = 8.0;
    pub const MENU: f32 = 10.0;
    pub const DIALOG: f32 = 14.0;
    /// Pills and ports: half the height.
    pub const FULL: f32 = f32::INFINITY;
}

// ---- Strokes.
pub mod stroke {
    pub const HAIRLINE: f32 = 1.0;
    /// Focus rings: 2 px, outside the focused control (§3.5).
    pub const FOCUS: f32 = 2.0;
}

// ---- Motion (§3.2): Fluent 2 durations and curves [64]; keyboard-invoked
// and frequent UI does not animate (§8.5 rule 5).
pub mod motion {
    pub const INSTANT_MS: u32 = 0;
    pub const PRESS_MS: u32 = 100;
    pub const HOVER_IN_MS: u32 = 100;
    /// Linear's hover fade-out [51].
    pub const HOVER_OUT_MS: u32 = 150;
    pub const PANEL_MS: u32 = 200;
    pub const CAMERA_MS: u32 = 300;
    /// Reduced motion turns moves into jumps with fades no longer than this.
    pub const REDUCED_FADE_MS: u32 = 100;
    /// A change highlight holds, then fades (proposal, tuned with the Operator).
    pub const CHANGE_HOLD_MS: u32 = 1_500;
    pub const CHANGE_FADE_MS: u32 = 400;
    /// Cubic Bézier control points (x1, y1, x2, y2).
    pub type Curve = (f32, f32, f32, f32);
    /// Fluent 2 `curveDecelerateMid`: things arriving.
    pub const DECELERATE: Curve = (0.0, 0.0, 0.0, 1.0);
    /// Fluent 2 `curveAccelerateMid`: things leaving.
    pub const ACCELERATE: Curve = (1.0, 0.0, 1.0, 1.0);
    /// Fluent 2 `curveEasyEase`: things moving on screen.
    pub const EASY_EASE: Curve = (0.33, 0.0, 0.67, 1.0);
    /// A critically damped spring (mass 1) for things that follow a choice
    /// or the pointer (a switch, a sliding selection, a resized panel): it
    /// settles in about 200 ms without overshoot and keeps its velocity
    /// when the target changes.
    pub const SPRING_STIFFNESS: f32 = 520.0;
    pub const SPRING_DAMPING: f32 = 45.6;
}

/// An sRGB colour with alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

/// A 12-step scale: `step(1)` is the app background, `step(12)` the
/// highest-contrast text (Radix roles).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scale(pub [Rgba; 12]);

impl Scale {
    pub fn step(&self, step: usize) -> Rgba {
        self.0[step.clamp(1, 12) - 1]
    }
}

/// The inputs every theme is generated from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeInputs {
    /// Hue of the neutral scale, in degrees (a faint tint).
    pub base_hue: f32,
    /// Hue of the accent (selection, focus), in degrees.
    pub accent_hue: f32,
    /// 1.0 normal; above 1 widens the lightness range (high contrast).
    pub contrast: f32,
}

/// Agentique's starting inputs: a cool neutral and a blue accent.
pub const INPUTS: ThemeInputs = ThemeInputs {
    base_hue: 265.0,
    accent_hue: 258.0,
    contrast: 1.0,
};

/// The high-contrast theme: the same hues, every step further from the
/// background.
pub const HIGH_CONTRAST: ThemeInputs = ThemeInputs {
    contrast: 1.25,
    ..INPUTS
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Dark,
    Light,
}

/// A generated theme: the neutral scale and the five roles (§3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colours {
    pub mode: Mode,
    pub neutral: Scale,
    /// Selection, focus.
    pub accent: Scale,
    /// Added, passed.
    pub success: Scale,
    /// A lock needing confirmation, drift.
    pub warning: Scale,
    /// A validation error, removed.
    pub danger: Scale,
    /// The Assistant's activity.
    pub info: Scale,
}

/// Role hues, fixed; only the accent and the neutral tint are inputs.
const SUCCESS_HUE: f32 = 150.0;
const WARNING_HUE: f32 = 75.0;
const DANGER_HUE: f32 = 25.0;
const INFO_HUE: f32 = 300.0;

impl Colours {
    pub fn generate(inputs: ThemeInputs, mode: Mode) -> Colours {
        let neutral = scale(inputs.base_hue, 0.012, 0.0, mode, inputs.contrast);
        let role = |hue: f32| scale(hue, 0.05, 0.14, mode, inputs.contrast);
        Colours {
            mode,
            neutral,
            accent: role(inputs.accent_hue),
            success: role(SUCCESS_HUE),
            warning: role(WARNING_HUE),
            danger: role(DANGER_HUE),
            info: role(INFO_HUE),
        }
    }
}

/// Lightness of each step, as Radix spaces its steps: close backgrounds,
/// then borders, then the solid colour, then text. Dark first; light mirrors,
/// with its solid steps a little darker so lines on panel backgrounds keep
/// 3:1 (§3.5).
const DARK_L: [f32; 12] = [
    0.17, 0.19, 0.225, 0.25, 0.28, 0.32, 0.37, 0.45, 0.62, 0.66, 0.80, 0.95,
];
const LIGHT_L: [f32; 12] = [
    0.99, 0.975, 0.95, 0.925, 0.90, 0.865, 0.82, 0.74, 0.60, 0.58, 0.45, 0.22,
];

/// One 12-step scale: chroma rises from `soft` in the backgrounds to `solid`
/// at steps 9–10 and eases off for text.
fn scale(hue: f32, soft: f32, solid: f32, mode: Mode, contrast: f32) -> Scale {
    let lightness = match mode {
        Mode::Dark => DARK_L,
        Mode::Light => LIGHT_L,
    };
    // Contrast widens every step's distance from the background (step 1).
    let background = lightness[0];
    let mut steps = [Rgba(0, 0, 0, 255); 12];
    for (index, l) in lightness.iter().enumerate() {
        let l = (background + (l - background) * contrast).clamp(0.0, 1.0);
        let chroma = match index {
            0..=4 => soft,
            5..=7 => soft + (solid.max(soft) - soft) * 0.4,
            8 | 9 => solid.max(soft),
            _ => soft + (solid.max(soft) - soft) * 0.6,
        };
        steps[index] = oklch(l, chroma, hue);
    }
    Scale(steps)
}

/// OKLCH to sRGB, clipping chroma until the colour is in gamut.
pub fn oklch(l: f32, c: f32, h_degrees: f32) -> Rgba {
    let mut c = c;
    loop {
        let h = h_degrees.to_radians();
        let (a, b) = (c * h.cos(), c * h.sin());
        // OKLab to linear sRGB (Björn Ottosson's matrices).
        let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
        let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
        let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
        let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
        let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
        let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
        let b = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;
        let inside = [r, g, b].iter().all(|v| (-0.0005..=1.0005).contains(v));
        if inside || c <= 0.0005 {
            let encode = |v: f32| {
                let v = v.clamp(0.0, 1.0);
                let s = if v <= 0.003_130_8 {
                    12.92 * v
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                };
                (s * 255.0).round() as u8
            };
            return Rgba(encode(r), encode(g), encode(b), 255);
        }
        c *= 0.9;
    }
}

/// Text and icons on a scale's solid colour (steps 9 and 10): white or
/// black, whichever reads better.
pub fn on_solid(scale: Scale) -> Rgba {
    const WHITE: Rgba = Rgba(255, 255, 255, 255);
    const BLACK: Rgba = Rgba(0, 0, 0, 255);
    if contrast(WHITE, scale.step(9)) >= contrast(BLACK, scale.step(9)) {
        WHITE
    } else {
        BLACK
    }
}

/// WCAG contrast ratio between two opaque colours.
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let luminance = |c: Rgba| {
        let linear = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.0) + 0.7152 * linear(c.1) + 0.0722 * linear(c.2)
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// One component of the design system (§3.2).
#[derive(Clone, Copy, Debug)]
pub struct Component {
    /// Shell, Surface, Panels, Command, Conversation or Primitives.
    pub group: &'static str,
    pub name: &'static str,
    /// Kinds or sizes shown side by side.
    pub variants: &'static [&'static str],
    /// States of its own, shown besides the shared ones (`STATES`).
    pub states: &'static [&'static str],
}

const fn component(
    group: &'static str,
    name: &'static str,
    variants: &'static [&'static str],
    states: &'static [&'static str],
) -> Component {
    Component {
        group,
        name,
        variants,
        states,
    }
}

/// Every component of §3.2, each shown in the gallery (`--fixture
/// components`, W5.2) in every state and theme before it is used (§8.5
/// rule 2). Plan, question, lock and note cards and queued messages arrive
/// with Stage 6.
pub const COMPONENTS: &[Component] = &[
    // Shell.
    component("Shell", "title area", &["project name", "branch"], &[]),
    component("Shell", "toolbar", &[], &[]),
    component(
        "Shell",
        "status bar",
        &[
            "save state",
            "problems",
            "locks",
            "background work",
            "frame time",
        ],
        &[],
    ),
    component("Shell", "panel", &["docked"], &["collapsed", "resizing"]),
    component("Shell", "splitter", &[], &[]),
    component("Shell", "focus mode", &[], &[]),
    // Surface.
    component("Surface", "dot grid", &[], &["faded by zoom"]),
    component(
        "Surface",
        "card",
        &["part", "item", "interface", "requirement"],
        &[],
    ),
    component("Surface", "card badge", &["lock", "problem", "agent"], &[]),
    component(
        "Surface",
        "port",
        &["in", "out", "inout"],
        &["unconnected", "connected"],
    ),
    component("Surface", "edge", &["arrowhead by kind"], &[]),
    component("Surface", "edge label pill", &[], &[]),
    component(
        "Surface",
        "container",
        &[],
        &["expanded", "collapsed with boundary ports"],
    ),
    component("Surface", "selection ring", &[], &[]),
    component("Surface", "marquee", &[], &[]),
    component("Surface", "alignment guide", &[], &[]),
    component("Surface", "change mark", &["added", "changed"], &[]),
    component("Surface", "removal ghost", &[], &[]),
    component("Surface", "minimap", &[], &[]),
    component("Surface", "zoom controls", &[], &[]),
    component("Surface", "colour-by overlay", &["legend"], &[]),
    component("Surface", "level-of-detail tiers", &[], &[]),
    // Panels.
    component("Panels", "outline", &[], &[]),
    component(
        "Panels",
        "inspector row",
        &["label and value"],
        &["invalid"],
    ),
    component("Panels", "requirements", &[], &[]),
    component("Panels", "problems", &[], &[]),
    component("Panels", "history", &["checkpoints", "what changed"], &[]),
    // Command.
    component(
        "Command",
        "command palette",
        &["groups", "keywords", "shortcuts", "recent items"],
        &["empty", "loading"],
    ),
    component("Command", "context menu", &[], &[]),
    component("Command", "go to element", &[], &[]),
    component("Command", "shortcut help", &[], &[]),
    // Conversation.
    component("Conversation", "operator message", &[], &[]),
    component(
        "Conversation",
        "assistant message",
        &["element links"],
        &["streaming"],
    ),
    component(
        "Conversation",
        "thinking row",
        &[],
        &["collapsed", "expanded"],
    ),
    component("Conversation", "tool card", &["by kind"], WORK_STATES),
    component("Conversation", "change chip", &[], &[]),
    component("Conversation", "plan card", &[], &[]),
    component(
        "Conversation",
        "question card",
        &["options", "preview"],
        &[],
    ),
    component("Conversation", "lock card", &["before and after"], &[]),
    component("Conversation", "note card", &[], &[]),
    component("Conversation", "queued message", &[], &[]),
    component("Conversation", "turn summary", &[], &[]),
    component("Conversation", "compaction divider", &[], &[]),
    component(
        "Conversation",
        "composer",
        &["context chips", "autonomy mode", "model picker"],
        &["running (Stop)"],
    ),
    // Primitives.
    component(
        "Primitives",
        "button",
        &["primary", "secondary", "ghost", "danger", "24", "28", "32"],
        &[],
    ),
    component("Primitives", "icon button", &[], &[]),
    component("Primitives", "text field", &[], &["invalid"]),
    component("Primitives", "text area", &[], &[]),
    component("Primitives", "select", &[], &["open"]),
    component("Primitives", "toggle", &[], &["on", "off"]),
    component("Primitives", "checkbox", &[], &["checked", "unchecked"]),
    component("Primitives", "radio", &[], &["checked", "unchecked"]),
    component("Primitives", "chip", &[], &[]),
    component("Primitives", "badge", &[], &[]),
    component("Primitives", "key cap", &[], &[]),
    component("Primitives", "tooltip", &["with shortcut"], &[]),
    component("Primitives", "banner", &[], &[]),
    component("Primitives", "inline message", &[], &[]),
    component("Primitives", "toast", &["background event"], &[]),
    component("Primitives", "dialog", &[], &[]),
    component("Primitives", "empty state", &[], &[]),
    component("Primitives", "skeleton", &[], &[]),
    component("Primitives", "spinner", &[], &[]),
    component("Primitives", "progress", &[], &[]),
    component("Primitives", "focus ring", &[], &[]),
];

/// The states every component shows the same way (§3.2 principle 4, §8.5).
pub const STATES: &[&str] = &[
    "rest", "hover", "press", "focus", "disabled", "selected", "changed",
];

/// The state vocabulary shared by the Surface, the Panels and the
/// Conversation (§3.2 principle 4).
pub const WORK_STATES: &[&str] = &[
    "pending",
    "running",
    "waiting for you",
    "done",
    "failed",
    "refused",
];

#[cfg(test)]
mod tests {
    use super::*;

    const THEMES: [(Mode, ThemeInputs); 4] = [
        (Mode::Dark, INPUTS),
        (Mode::Light, INPUTS),
        (Mode::Dark, HIGH_CONTRAST),
        (Mode::Light, HIGH_CONTRAST),
    ];

    fn scales(colours: &Colours) -> [Scale; 6] {
        [
            colours.neutral,
            colours.accent,
            colours.success,
            colours.warning,
            colours.danger,
            colours.info,
        ]
    }

    #[test]
    fn body_text_lines_and_text_on_solids_meet_their_contrast_in_every_theme() {
        for (mode, inputs) in THEMES {
            let colours = Colours::generate(inputs, mode);
            let n = colours.neutral;
            for background in 1..=3 {
                // Body copy at step 11 or 12 on steps 1–3 (§3.5): 4.5:1.
                assert!(
                    contrast(n.step(11), n.step(background)) >= 4.5,
                    "{mode:?} step 11 on {background}"
                );
                assert!(
                    contrast(n.step(12), n.step(background)) >= 7.0,
                    "{mode:?} step 12 on {background}"
                );
                // Lines and focus rings at step 9 on every panel background:
                // 3:1 (§3.2, §3.5).
                for scale in scales(&colours) {
                    assert!(
                        contrast(scale.step(9), n.step(background)) >= 3.0,
                        "{mode:?} {inputs:?} step 9 on {background}"
                    );
                }
            }
            // Text on a solid colour: 4.5:1.
            for scale in scales(&colours) {
                assert!(
                    contrast(on_solid(scale), scale.step(9)) >= 4.5,
                    "{mode:?} on solid"
                );
            }
        }
    }

    #[test]
    fn every_step_moves_further_from_the_background_and_high_contrast_further_still() {
        for (mode, inputs) in THEMES {
            let colours = Colours::generate(inputs, mode);
            let normal = Colours::generate(INPUTS, mode);
            for (scale, normal_scale) in scales(&colours).into_iter().zip(scales(&normal)) {
                let background = colours.neutral.step(1);
                for step in 2..=12 {
                    assert!(
                        contrast(scale.step(step), background)
                            > contrast(scale.step(step - 1), background),
                        "{mode:?} {inputs:?} step {step}"
                    );
                    assert!(
                        contrast(scale.step(step), background)
                            >= contrast(normal_scale.step(step), normal.neutral.step(1)) - 0.01,
                        "{mode:?} {inputs:?} step {step} loses contrast"
                    );
                }
            }
        }
    }

    #[test]
    fn oklch_matches_known_colours() {
        assert_eq!(oklch(1.0, 0.0, 0.0), Rgba(255, 255, 255, 255));
        assert_eq!(oklch(0.0, 0.0, 0.0), Rgba(0, 0, 0, 255));
        // OKLCH(0.628, 0.2577, 29.23) is sRGB red.
        let red = oklch(0.628, 0.2577, 29.23);
        assert!(red.0 >= 250 && red.1 <= 8 && red.2 <= 8, "{red:?}");
    }

    #[test]
    fn the_spacing_grid_is_on_four_pixels_after_the_half_steps() {
        assert!(space::GRID.iter().all(|value| value % 2.0 == 0.0));
        assert!(space::GRID[4..].iter().all(|value| value % 4.0 == 0.0));
    }

    #[test]
    fn the_component_list_covers_every_group_once() {
        for group in [
            "Shell",
            "Surface",
            "Panels",
            "Command",
            "Conversation",
            "Primitives",
        ] {
            assert!(COMPONENTS.iter().any(|c| c.group == group), "{group}");
        }
        let mut names: Vec<_> = COMPONENTS.iter().map(|c| c.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "a component is listed twice");
        let tool_card = COMPONENTS.iter().find(|c| c.name == "tool card").unwrap();
        assert_eq!(tool_card.states, WORK_STATES);
    }
}
