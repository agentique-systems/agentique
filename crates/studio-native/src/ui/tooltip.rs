//! Tooltips (§3.2 Primitives): a title and, when the action has one, its
//! shortcut, so every control teaches its key (§3.4).
use crate::ui::{
    keycap::KeyCaps,
    theme::{self, ActiveTheme, r},
};
use gpui::{
    AnyView, App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, prelude::FluentBuilder,
};
use std::rc::Rc;

pub struct Tooltip {
    title: SharedString,
    shortcut: Option<&'static str>,
    detail: Option<SharedString>,
}

impl Tooltip {
    pub fn new(title: impl Into<SharedString>) -> Tooltip {
        Tooltip {
            title: title.into(),
            shortcut: None,
            detail: None,
        }
    }
    pub fn shortcut(mut self, shortcut: Option<&'static str>) -> Tooltip {
        self.shortcut = shortcut.filter(|shortcut| !shortcut.is_empty());
        self
    }
    /// A second, quieter line.
    pub fn detail(mut self, detail: impl Into<SharedString>) -> Tooltip {
        self.detail = Some(detail.into());
        self
    }
}

/// What `.tooltip(...)` takes.
pub type Builder = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;

/// A tooltip builder for `.tooltip(...)`.
pub fn text(title: impl Into<SharedString>, shortcut: Option<&'static str>) -> Builder {
    let title = title.into();
    Rc::new(move |_, cx| {
        cx.new(|_| Tooltip::new(title.clone()).shortcut(shortcut))
            .into()
    })
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        // GPUI places the tooltip beside the pointer; the margin keeps it off
        // the cursor.
        div().pl(r(10.0)).pt(r(18.0)).child(
            div()
                .max_w(r(320.0))
                .px(r(8.0))
                .py(r(5.0))
                .rounded(r(crate::tokens::radius::CONTROL))
                .bg(theme.overlay)
                .border_1()
                .border_color(theme.border)
                .shadow(theme.shadow_overlay())
                .text_size(r(theme::text::SM))
                .text_color(theme.text)
                .font_family(theme::SANS)
                .flex()
                .flex_col()
                .gap(r(2.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .child(self.title.clone())
                        .when_some(self.shortcut, |this, shortcut| {
                            this.child(KeyCaps::new(shortcut))
                        }),
                )
                .when_some(self.detail.clone(), |this, detail| {
                    this.child(div().text_color(theme.text_muted).child(detail))
                }),
        )
    }
}
