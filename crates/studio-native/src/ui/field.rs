//! Text fields and text areas (§3.2 Primitives): gpui-base's editing engine
//! (selection, IME composition at the caret, undo, clipboard) in the
//! Studio's frame, with focus, invalid and disabled states.
use crate::ui::{
    icon::{IconName, icon},
    theme::{self, ActiveTheme, r},
};
use gpui::{
    AnyElement, App, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_base::input::{Input, InputState, Textarea, TextareaState};

/// The frame every field shares: inset background, hairline, a soft ring
/// while focused, a warning border while invalid.
fn frame(focused: bool, invalid: bool, cx: &App) -> gpui::Div {
    let theme = cx.theme();
    let border = if invalid {
        theme.danger.solid
    } else if focused {
        theme.accent.solid
    } else {
        theme.border
    };
    div()
        .flex()
        .items_center()
        .gap(r(6.0))
        .px(r(8.0))
        .rounded(r(crate::tokens::radius::CONTROL))
        .bg(theme.inset)
        .border_1()
        .border_color(border)
        .text_size(r(theme::text::BASE))
        .font_family(theme::SANS)
        .text_color(theme.text)
        .when(focused, |this| {
            this.shadow(vec![gpui::BoxShadow {
                color: if invalid {
                    theme.danger.solid.opacity(0.25)
                } else {
                    theme.accent.solid.opacity(0.28)
                },
                offset: gpui::point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(3.0),
                inset: false,
            }])
        })
}

/// A single-line field.
#[derive(IntoElement)]
pub struct TextField {
    state: Entity<InputState>,
    invalid: bool,
    leading: Option<IconName>,
    trailing: Option<AnyElement>,
    height: f32,
    mono: bool,
    name: Option<SharedString>,
}

impl TextField {
    pub fn new(state: &Entity<InputState>) -> TextField {
        TextField {
            state: state.clone(),
            invalid: false,
            leading: None,
            trailing: None,
            height: crate::tokens::space::CONTROL,
            mono: false,
            name: None,
        }
    }
    /// The field's name for the scripted journeys.
    pub fn target(mut self, name: impl Into<SharedString>) -> TextField {
        self.name = Some(name.into());
        self
    }
    pub fn invalid(mut self, invalid: bool) -> TextField {
        self.invalid = invalid;
        self
    }
    pub fn leading(mut self, name: IconName) -> TextField {
        self.leading = Some(name);
        self
    }
    pub fn trailing(mut self, element: impl IntoElement) -> TextField {
        self.trailing = Some(element.into_any_element());
        self
    }
    pub fn large(mut self) -> TextField {
        self.height = crate::tokens::space::CONTROL_LG;
        self
    }
    /// For names, keys and values.
    pub fn mono(mut self) -> TextField {
        self.mono = true;
        self
    }
}

impl RenderOnce for TextField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let muted = cx.theme().text_muted;
        let handle = self.state.read(cx).focus_handle(cx);
        frame(focused, self.invalid, cx)
            .h(r(self.height))
            .w_full()
            .relative()
            .when_some(self.name, |this, name| {
                this.child(crate::ui::target::target(name))
            })
            .when(self.mono, |this| this.font_family(theme::MONO))
            .cursor_text()
            .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                window.focus(&handle, cx);
            })
            .when_some(self.leading, |this, name| {
                this.child(icon(name).size(14.0).color(muted))
            })
            .child(div().flex_1().min_w_0().child(Input::new(&self.state)))
            .when_some(self.trailing, |this, trailing| this.child(trailing))
    }
}

/// A multi-line field that grows with its text.
#[derive(IntoElement)]
pub struct TextArea {
    state: Entity<TextareaState>,
    invalid: bool,
    borderless: bool,
    name: Option<SharedString>,
}

impl TextArea {
    pub fn new(state: &Entity<TextareaState>) -> TextArea {
        TextArea {
            state: state.clone(),
            invalid: false,
            borderless: false,
            name: None,
        }
    }
    /// The field's name for the scripted journeys.
    pub fn target(mut self, name: impl Into<SharedString>) -> TextArea {
        self.name = Some(name.into());
        self
    }
    pub fn invalid(mut self, invalid: bool) -> TextArea {
        self.invalid = invalid;
        self
    }
    /// Inside a frame of its own (the composer).
    pub fn borderless(mut self) -> TextArea {
        self.borderless = true;
        self
    }
}

impl RenderOnce for TextArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let area = div()
            .w_full()
            .relative()
            .child(Textarea::new(&self.state))
            .when_some(self.name, |this, name| {
                this.child(crate::ui::target::target(name))
            });
        if self.borderless {
            return div()
                .w_full()
                .text_size(r(theme::text::PROSE))
                .line_height(r(21.0))
                .font_family(theme::SANS)
                .text_color(cx.theme().text)
                .child(area);
        }
        frame(focused, self.invalid, cx)
            .items_start()
            .py(r(6.0))
            .w_full()
            .child(area)
    }
}
