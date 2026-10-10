//! Small primitives of the design system (§3.2): chip, badge, switch,
//! segmented control, spinner, skeleton, progress, banner, inline message,
//! empty state, section header and divider. Each takes its colours from the
//! theme and its sizes from the tokens.
use crate::ui::{
    icon::{IconName, icon},
    theme::{self, ActiveTheme, Theme, r},
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, ElementId, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, SpringAnimation,
    SpringConfig, StatefulInteractiveElement, Styled, Transformation, Window, div,
    prelude::FluentBuilder, px, relative, svg,
};
use std::{rc::Rc, time::Duration};

/// A spring for things that follow the pointer or a choice: critically
/// damped, settling in about 200 ms (§3.2 motion, 200 ms panels).
pub fn spring() -> SpringAnimation<()> {
    SpringAnimation::new(SpringConfig::new(
        crate::tokens::motion::SPRING_STIFFNESS,
        crate::tokens::motion::SPRING_DAMPING,
        1.0,
    ))
}

/// Which role colours a chip or badge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
    Info,
}

impl Tone {
    /// Background, text and border.
    pub fn colours(self, theme: &Theme) -> (Hsla, Hsla, Hsla) {
        let role = match self {
            Tone::Neutral => return (theme.hover, theme.text_secondary, theme.separator),
            Tone::Accent => theme.accent,
            Tone::Success => theme.success,
            Tone::Warning => theme.warning,
            Tone::Danger => theme.danger,
            Tone::Info => theme.info,
        };
        (role.soft, role.text, role.border.opacity(0.6))
    }
}

/// A small labelled token: a kind, a state, a change chip.
#[derive(IntoElement)]
pub struct Chip {
    label: SharedString,
    icon: Option<IconName>,
    tone: Tone,
    mono: bool,
}

impl Chip {
    pub fn new(label: impl Into<SharedString>) -> Chip {
        Chip {
            label: label.into(),
            icon: None,
            tone: Tone::Neutral,
            mono: false,
        }
    }
    pub fn icon(mut self, name: IconName) -> Chip {
        self.icon = Some(name);
        self
    }
    pub fn tone(mut self, tone: Tone) -> Chip {
        self.tone = tone;
        self
    }
    /// The label is a name or a value.
    pub fn mono(mut self) -> Chip {
        self.mono = true;
        self
    }
}

impl RenderOnce for Chip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (bg, fg, border) = self.tone.colours(theme);
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(r(4.0))
            .h(r(20.0))
            .px(r(6.0))
            .rounded(r(crate::tokens::radius::TAG + 2.0))
            .bg(bg)
            .border_1()
            .border_color(border)
            .text_color(fg)
            .text_size(r(theme::text::XS))
            .font_weight(theme::MEDIUM)
            .when(self.mono, |this| this.font_family(theme::MONO))
            .when_some(self.icon, |this, name| {
                this.child(icon(name).size(12.0).color(fg))
            })
            .child(div().whitespace_nowrap().child(self.label))
    }
}

/// A count or a short state beside a label.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    tone: Tone,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Badge {
        Badge {
            label: label.into(),
            tone: Tone::Neutral,
        }
    }
    pub fn tone(mut self, tone: Tone) -> Badge {
        self.tone = tone;
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (bg, fg, _) = self.tone.colours(theme);
        div()
            .flex_none()
            .min_w(r(18.0))
            .h(r(18.0))
            .px(r(5.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(bg)
            .text_color(fg)
            .text_size(r(theme::text::XS))
            .font_weight(theme::SEMIBOLD)
            .child(self.label)
    }
}

type Toggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A switch: on or off, applied at once (§3.7 Apply).
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    on: bool,
    disabled: bool,
    label: SharedString,
    on_toggle: Option<Toggle>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>, on: bool, label: impl Into<SharedString>) -> Switch {
        Switch {
            id: id.into(),
            on,
            disabled: false,
            label: label.into(),
            on_toggle: None,
        }
    }
    pub fn disabled(mut self, disabled: bool) -> Switch {
        self.disabled = disabled;
        self
    }
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Switch {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let on = self.on;
        let track = if on {
            theme.accent.solid
        } else {
            theme.pressed
        };
        let knob_id = ElementId::Name(format!("{:?}-knob", self.id).into());
        // For agents (C-54): what it switches, whether it is on, and whether
        // it has the focus (its own handle, so Space is known to reach it).
        let focus = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let control = crate::ui::target::Control::new("switch", self.label.clone())
            .id(self.id.to_string())
            .value(if on { "on" } else { "off" })
            .enabled(!self.disabled)
            .selected(on)
            .focused(focus.is_focused(window));
        let mut button = gpui_base::Button::new(self.id)
            .track_focus(&focus)
            .role(gpui::Role::Switch)
            .accessibility_label(self.label)
            .disabled(self.disabled)
            .relative()
            .child(crate::ui::target::control(control))
            .w(r(30.0))
            .h(r(18.0))
            .p(px(2.0))
            .rounded_full()
            .bg(track)
            .border_1()
            .border_color(if on { track } else { theme.border })
            .when(!self.disabled, |this| this.cursor_pointer())
            .when(self.disabled, |this| this.opacity(0.45))
            .focus_visible(move |style| {
                style.shadow(crate::ui::button::focus_ring(theme.accent.solid))
            })
            .child(
                div().size_full().relative().child(
                    div()
                        .absolute()
                        .top_0()
                        .size(r(12.0))
                        .rounded_full()
                        .bg(if on {
                            theme.accent.on_solid
                        } else {
                            theme.text_secondary
                        })
                        .shadow(theme.shadow_small())
                        .with_spring(
                            knob_id,
                            spring().to(if on { 1.0 } else { 0.0 }),
                            |knob, t: f32| knob.left(relative(t * 0.52)),
                        ),
                ),
            );
        if let Some(toggle) = self.on_toggle {
            button = button.on_click(move |_, window, cx| toggle(!on, window, cx));
        }
        button
    }
}

type Choose = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A segmented control: one of a few choices, with a sliding selection.
#[derive(IntoElement)]
pub struct Segmented {
    id: ElementId,
    choices: Vec<(Option<IconName>, SharedString)>,
    selected: usize,
    on_choose: Option<Choose>,
    tooltips: Vec<Option<(SharedString, Option<&'static str>)>>,
    /// Icons only: the labels stay the choices' names and tooltips.
    compact: bool,
}

impl Segmented {
    pub fn new(id: impl Into<ElementId>, selected: usize) -> Segmented {
        Segmented {
            id: id.into(),
            choices: Vec::new(),
            selected,
            on_choose: None,
            tooltips: Vec::new(),
            compact: false,
        }
    }
    /// Shows the choices by their icons only (where room is short); each
    /// label stays its name and leads its tooltip.
    pub fn compact(mut self, compact: bool) -> Segmented {
        self.compact = compact;
        self
    }
    pub fn choice(mut self, icon: Option<IconName>, label: impl Into<SharedString>) -> Segmented {
        self.choices.push((icon, label.into()));
        self.tooltips.push(None);
        self
    }
    /// The last choice's tooltip, with its shortcut.
    pub fn tooltip(
        mut self,
        title: impl Into<SharedString>,
        shortcut: Option<&'static str>,
    ) -> Segmented {
        if let Some(last) = self.tooltips.last_mut() {
            *last = Some((title.into(), shortcut));
        }
        self
    }
    pub fn on_choose(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Segmented {
        self.on_choose = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Segmented {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme().clone();
        let count = self.choices.len().max(1) as f32;
        let selected = self.selected.min(self.choices.len().saturating_sub(1));
        let indicator = ElementId::Name(format!("{:?}-indicator", self.id).into());
        div()
            .id(self.id.clone())
            .role(gpui::Role::TabList)
            .relative()
            .flex()
            .items_center()
            .h(r(28.0))
            .p(px(2.0))
            .rounded(r(crate::tokens::radius::CONTROL + 1.0))
            .bg(theme.inset)
            .border_1()
            .border_color(theme.separator)
            .child(
                div()
                    .absolute()
                    .top(px(2.0))
                    .bottom(px(2.0))
                    .w(relative(1.0 / count))
                    .px(px(2.0))
                    .child(
                        div()
                            .size_full()
                            .rounded(r(crate::tokens::radius::CONTROL))
                            .bg(theme.raised)
                            .border_1()
                            .border_color(theme.border)
                            .shadow(theme.shadow_small()),
                    )
                    .with_spring(
                        indicator,
                        spring().to(selected as f32),
                        move |this, at: f32| this.left(relative(at / count)),
                    ),
            )
            .children(
                self.choices
                    .into_iter()
                    .enumerate()
                    .map(|(index, (glyph, label))| {
                        let chosen = index == selected;
                        let iconic = glyph.is_some();
                        let label_shown = label.clone();
                        let on_choose = self.on_choose.clone();
                        let tooltip = self.tooltips.get(index).cloned().flatten();
                        div()
                            .id(ElementId::NamedInteger("segment".into(), index as u64))
                            .role(gpui::Role::Tab)
                            .aria_selected(chosen)
                            .aria_label(label.clone())
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .px(r(10.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(r(6.0))
                            .rounded(r(crate::tokens::radius::CONTROL))
                            .text_size(r(theme::text::SM))
                            .font_weight(theme::MEDIUM)
                            .text_color(if chosen { theme.text } else { theme.text_muted })
                            .cursor_pointer()
                            .when(!chosen, |this| {
                                this.hover(|style| style.text_color(theme.text_secondary))
                            })
                            .when_some(glyph, |this, name| {
                                this.child(icon(name).size(14.0).color(if chosen {
                                    theme.text
                                } else {
                                    theme.text_muted
                                }))
                            })
                            .child(crate::ui::target::control(
                                crate::ui::target::Control::new("option", label.clone())
                                    .selected(chosen),
                            ))
                            .when(!self.compact || !iconic, |this| {
                                this.child(
                                    div()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .whitespace_nowrap()
                                        .child(label),
                                )
                            })
                            .when_some(tooltip, |this, (title, shortcut)| {
                                // Icons only: the tooltip names the choice.
                                let title: SharedString = if self.compact && iconic {
                                    format!("{label_shown}: {title}").into()
                                } else {
                                    title
                                };
                                let tooltip = crate::ui::tooltip::text(title, shortcut);
                                this.tooltip(move |window, cx| tooltip(window, cx))
                            })
                            .when_some(on_choose, |this, on_choose| {
                                this.on_click(move |_: &ClickEvent, window, cx| {
                                    on_choose(index, window, cx)
                                })
                            })
                    }),
            )
    }
}

/// A spinner for work that takes longer than about 400 ms (§3.3); still
/// under reduced motion.
pub fn spinner(id: impl Into<ElementId>, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .path(IconName::Loader.path())
        .size(r(size))
        .flex_none()
        .text_color(color)
        .with_animation(
            id,
            Animation::new(Duration::from_millis(900)).repeat(),
            |svg, delta| svg.with_transformation(Transformation::rotate(gpui::percentage(delta))),
        )
}

/// A placeholder bar while content loads: it breathes gently, and stays
/// still under reduced motion.
pub fn skeleton(
    id: impl Into<ElementId>,
    width: gpui::DefiniteLength,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .h(r(10.0))
        .w(width)
        .rounded_full()
        .bg(theme.hover)
        .with_animation(
            id,
            Animation::new(Duration::from_millis(1400))
                .repeat()
                .with_easing(gpui::ease_in_out),
            |bar, delta| bar.opacity(0.55 + 0.45 * (1.0 - (2.0 * delta - 1.0).abs())),
        )
}

/// A thin progress bar; `None` while the amount is not known.
pub fn progress(value: Option<f32>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .h(px(3.0))
        .w_full()
        .rounded_full()
        .bg(theme.hover)
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .rounded_full()
                .bg(theme.accent.solid)
                .w(relative(value.unwrap_or(0.3).clamp(0.0, 1.0))),
        )
}

/// A banner across a panel: something to fix, with the action that fixes
/// it (§3.4: every error in plain words, with the action that fixes it).
#[derive(IntoElement)]
pub struct Banner {
    tone: Tone,
    icon: IconName,
    message: SharedString,
    actions: Vec<AnyElement>,
}

impl Banner {
    pub fn new(tone: Tone, message: impl Into<SharedString>) -> Banner {
        Banner {
            tone,
            icon: match tone {
                Tone::Danger => IconName::CircleX,
                Tone::Warning => IconName::Warning,
                Tone::Success => IconName::CircleCheck,
                _ => IconName::Info,
            },
            message: message.into(),
            actions: Vec::new(),
        }
    }
    pub fn action(mut self, action: impl IntoElement) -> Banner {
        self.actions.push(action.into_any_element());
        self
    }
}

impl RenderOnce for Banner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (bg, fg, border) = self.tone.colours(theme);
        div()
            .flex()
            .flex_col()
            .gap(r(8.0))
            .p(r(10.0))
            .rounded(r(crate::tokens::radius::CARD))
            .bg(bg.opacity(0.6))
            .border_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(r(8.0))
                    .child(
                        div()
                            .pt(px(1.0))
                            .child(icon(self.icon).size(14.0).color(fg)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(r(theme::text::SM))
                            .line_height(r(18.0))
                            .text_color(theme.text_secondary)
                            .child(self.message),
                    ),
            )
            .when(!self.actions.is_empty(), |this| {
                this.child(div().flex().gap(r(6.0)).pl(r(22.0)).children(self.actions))
            })
    }
}

/// A one-line message under a field or a row: invalid, or a note.
pub fn inline_message(tone: Tone, message: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (_, fg, _) = tone.colours(theme);
    div()
        .flex()
        .items_start()
        .gap(r(6.0))
        .text_size(r(theme::text::SM))
        .text_color(if tone == Tone::Neutral {
            theme.text_muted
        } else {
            fg
        })
        .when(tone == Tone::Danger || tone == Tone::Warning, |this| {
            this.child(
                div()
                    .mt(r(2.0))
                    .child(icon(IconName::Alert).size(12.0).color(fg)),
            )
        })
        // Text in a flex row wraps only when it may shrink: a long
        // message stays within its panel.
        .child(div().flex_1().min_w_0().child(message.into()))
}

/// An empty state that teaches (§3.4): what this place is for and the one
/// action that starts it.
#[derive(IntoElement)]
pub struct EmptyState {
    icon: IconName,
    title: SharedString,
    body: SharedString,
    actions: Vec<AnyElement>,
    hints: Vec<(&'static str, &'static str)>,
}

impl EmptyState {
    pub fn new(
        icon: IconName,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> EmptyState {
        EmptyState {
            icon,
            title: title.into(),
            body: body.into(),
            actions: Vec::new(),
            hints: Vec::new(),
        }
    }
    pub fn action(mut self, action: impl IntoElement) -> EmptyState {
        self.actions.push(action.into_any_element());
        self
    }
    /// A key and what it does.
    pub fn hint(mut self, key: &'static str, what: &'static str) -> EmptyState {
        self.hints.push((key, what));
        self
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(r(12.0))
            .w_full()
            .max_w(r(360.0))
            .px(r(16.0))
            .child(
                div()
                    .size(r(40.0))
                    .rounded(r(crate::tokens::radius::MENU + 2.0))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border)
                    .flex()
                    .items_center()
                    .justify_center()
                    .shadow(theme.shadow_small())
                    .child(icon(self.icon).size(20.0).color(theme.text_muted)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(r(4.0))
                    .child(
                        div()
                            .text_size(r(theme::text::PROSE))
                            .font_weight(theme::SEMIBOLD)
                            .text_color(theme.text)
                            .child(self.title),
                    )
                    .child(
                        div()
                            .text_size(r(theme::text::SM))
                            .line_height(r(18.0))
                            .w_full()
                            .text_color(theme.text_muted)
                            .text_center()
                            .child(self.body),
                    ),
            )
            .when(!self.actions.is_empty(), |this| {
                this.child(div().flex().gap(r(8.0)).pt(r(4.0)).children(self.actions))
            })
            .when(!self.hints.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_center()
                        .gap_x(r(14.0))
                        .gap_y(r(6.0))
                        .pt(r(4.0))
                        .children(self.hints.into_iter().map(|(key, what)| {
                            div()
                                .flex()
                                .items_center()
                                .gap(r(6.0))
                                .text_size(r(theme::text::XS))
                                .text_color(theme.text_muted)
                                .child(crate::ui::keycap::KeyCaps::new(key))
                                .child(what)
                        })),
                )
            })
    }
}

/// An overline heading for a group of rows.
pub fn section_header(title: impl Into<SharedString>, cx: &App) -> gpui::Div {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap(r(6.0))
        .h(r(24.0))
        .text_size(r(theme::text::XS))
        .font_weight(FontWeight(crate::tokens::text::SEMIBOLD as f32))
        .text_color(theme.text_muted)
        .child(title.into().to_uppercase())
}

/// A hairline between regions.
pub fn divider(cx: &App) -> gpui::Div {
    div().h(theme::hairline()).w_full().bg(cx.theme().separator)
}
