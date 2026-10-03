//! Overlays (§3.2): dialogs over a backdrop, and the entrance every overlay
//! shares. Overlays take the keyboard at once; the entrance is a brief fade
//! and settle that never delays input, and none under reduced motion.
use crate::ui::theme::{self, ActiveTheme, r};
use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use std::time::Duration;

/// A brief fade and settle for an overlay appearing (100 ms, §3.2 motion).
pub fn entrance<E: IntoElement + Styled + 'static>(
    id: impl Into<ElementId>,
    element: E,
) -> impl IntoElement {
    element.with_animation(
        id,
        Animation::new(Duration::from_millis(u64::from(
            crate::tokens::motion::PRESS_MS,
        )))
        .with_easing(gpui::ease_out_quint()),
        |element, delta| {
            element
                .opacity(0.35 + 0.65 * delta)
                .mt(px(4.0 * (1.0 - delta)))
        },
    )
}

/// A dialog: a title, a line of context, its body and its buttons, centred
/// over a backdrop that dims the Studio behind it.
#[derive(IntoElement)]
pub struct Dialog {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    body: Vec<AnyElement>,
    footer: Vec<AnyElement>,
    width: f32,
    top: f32,
}

impl Dialog {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Dialog {
        Dialog {
            id: id.into(),
            title: title.into(),
            description: None,
            body: Vec::new(),
            footer: Vec::new(),
            width: 440.0,
            // About a sixth of a Studio window down.
            top: 150.0,
        }
    }
    pub fn description(mut self, text: impl Into<SharedString>) -> Dialog {
        self.description = Some(text.into());
        self
    }
    /// Room above the dialog (the gallery shows one in a short frame).
    pub fn top(mut self, top: f32) -> Dialog {
        self.top = top;
        self
    }
    pub fn width(mut self, width: f32) -> Dialog {
        self.width = width;
        self
    }
    pub fn footer(mut self, element: impl IntoElement) -> Dialog {
        self.footer.push(element.into_any_element());
        self
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Dialog {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let record =
            crate::ui::target::Control::new("dialog", self.title.clone()).id(self.id.to_string());
        let record = match &self.description {
            Some(text) => record.value(text.clone()),
            None => record,
        };
        let panel = div()
            .id(self.id.clone())
            .role(gpui::Role::Dialog)
            .aria_label(self.title.clone())
            .relative()
            .child(crate::ui::target::control(record))
            .occlude()
            .w(r(self.width))
            .max_w_full()
            // Never taller than the window: the body scrolls and the
            // footer's buttons stay in view (200% on a small window).
            .max_h_full()
            .min_h_0()
            .rounded(r(crate::tokens::radius::DIALOG))
            .bg(theme.overlay)
            .border_1()
            .border_color(theme.border)
            .shadow(theme.shadow_overlay())
            .font_family(theme::SANS)
            .text_size(r(theme::text::BASE))
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .px(r(20.0))
                    .pt(r(18.0))
                    .pb(r(12.0))
                    .flex()
                    .flex_col()
                    .gap(r(4.0))
                    .child(
                        div()
                            .text_size(r(theme::text::LG))
                            .font_weight(theme::SEMIBOLD)
                            .child(self.title),
                    )
                    .when_some(self.description, |this, text| {
                        this.child(
                            div()
                                .text_size(r(theme::text::SM))
                                .line_height(r(18.0))
                                .text_color(theme.text_muted)
                                .child(text),
                        )
                    }),
            )
            .child(
                div()
                    .id(ElementId::Name(format!("{:?}-body", self.id).into()))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(r(20.0))
                    .pb(r(16.0))
                    .flex()
                    .flex_col()
                    .gap(r(10.0))
                    .children(self.body),
            )
            .when(!self.footer.is_empty(), |this| {
                this.child(
                    div()
                        .flex_none()
                        .px(r(16.0))
                        .py(r(12.0))
                        .flex()
                        .justify_end()
                        .gap(r(8.0))
                        .border_t_1()
                        .border_color(theme.separator)
                        .rounded_b(r(crate::tokens::radius::DIALOG))
                        .bg(theme.chrome)
                        .children(self.footer),
                )
            });
        div()
            .absolute()
            .inset_0()
            .bg(theme.backdrop)
            .flex()
            .justify_center()
            .pt(r(self.top))
            .pb(r(24.0))
            .child(entrance(
                ElementId::Name(format!("{:?}-entrance", self.id).into()),
                div().max_h_full().min_h_0().flex().flex_col().child(panel),
            ))
    }
}
