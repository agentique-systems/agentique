//! The Problems panel (§3.4): every problem in the model in plain words, at
//! its element; a click selects the element's card and moves there.
use crate::{
    studio::Studio,
    ui::{self, ActiveTheme, IconName, icon, r, theme},
};
use gpui::{
    App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, uniform_list,
};
use std::rc::Rc;

pub fn render(studio: &Entity<Studio>, cx: &mut App) -> impl IntoElement {
    let state = studio.read(cx);
    let Some(project) = &state.project else {
        return div()
            .p(r(16.0))
            .child(super::note("Open a project to see its problems.", cx))
            .into_any_element();
    };
    let system = project.state();
    let tree = system.tree();
    let rows: Rc<Vec<(agq_language::ElementId, SharedString, SharedString)>> = Rc::new(
        system
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                let path = tree.get(diagnostic.element).map_or_else(String::new, |_| {
                    crate::edit::display_path(tree, diagnostic.element)
                });
                (
                    diagnostic.element,
                    path.into(),
                    diagnostic.message.clone().into(),
                )
            })
            .collect(),
    );
    if rows.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(r(20.0))
            .child(ui::EmptyState::new(
                IconName::CircleCheck,
                "No problems",
                "Every element is valid. Problems appear here, and at the element, as they happen.",
            ))
            .into_any_element();
    }
    let count = rows.len();
    let studio = studio.clone();
    uniform_list("problems", count, move |range, _, cx| {
        let theme = cx.theme().clone();
        range
            .map(|index| {
                let (id, path, message) = rows[index].clone();
                let studio = studio.clone();
                div()
                    .id(("problem", index))
                    .h(r(52.0))
                    .mx(r(6.0))
                    .px(r(8.0))
                    .flex()
                    .items_start()
                    .gap(r(8.0))
                    .pt(r(8.0))
                    .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .role(gpui::Role::Button)
                    .aria_label(SharedString::from(format!("{path}: {message}")))
                    .on_click(move |_: &ClickEvent, _, cx| super::show(&studio, id, cx))
                    .child(
                        div()
                            .pt(r(1.0))
                            .child(icon(IconName::Warning).size(14.0).color(theme.warning.text)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(r(2.0))
                            .child(
                                div()
                                    .text_size(r(theme::text::SM))
                                    .text_color(theme.text)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(message),
                            )
                            .child(
                                div()
                                    .text_size(r(theme::text::XS))
                                    .font_family(theme::MONO)
                                    .text_color(theme.text_muted)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(path),
                            ),
                    )
            })
            .collect()
    })
    .size_full()
    .pt(r(6.0))
    .into_any_element()
}
