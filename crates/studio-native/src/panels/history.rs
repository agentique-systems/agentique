//! The History panel: checkpoints, newest first, and a visual "what
//! changed" between two of them, or between one and now, on the Surface.
use crate::{
    history::ago,
    studio::{Dirty, Studio},
    ui::{self, ActiveTheme, Button, IconName, KeyCaps, Tone, icon, r, theme},
    workspace::StudioExt,
};
use gpui::{
    App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};

pub fn render(studio: &Entity<Studio>, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme().clone();
    if studio.read(cx).project.is_none() {
        return div().p(r(16.0)).child(super::note("Open a project to see its history.", cx)).into_any_element();
    }
    studio.update(cx, |studio, _| studio.load_history());
    let state = studio.read(cx);
    let history = &state.history;
    let uncommitted = history.uncommitted.unwrap_or(true);
    let error = history.error.clone();
    let checkpoints: Vec<(String, String, String, bool)> = history
        .checkpoints
        .iter()
        .map(|c| (c.id.clone(), c.message.clone(), ago(c.time), history.selected.contains(&c.id)))
        .collect();
    let selected = history.selected.len();
    let comparison = state.comparison.as_ref().map(|c| {
        (
            c.after_is_now,
            c.created.iter().map(|(id, name)| (*id, name.clone())).collect::<Vec<_>>(),
            c.updated.iter().map(|(id, name)| (*id, name.clone())).collect::<Vec<_>>(),
            c.deleted.clone(),
        )
    });
    let checkpoint = studio.clone();
    let compare = studio.clone();
    let close = studio.clone();
    div()
        .id("history")
        .size_full()
        .overflow_y_scroll()
        .px(r(12.0))
        .pb(r(16.0))
        .flex()
        .flex_col()
        .child(
            div()
                .pt(r(12.0))
                .flex()
                .items_center()
                .gap(r(8.0))
                .child(
                    Button::new("checkpoint", "Checkpoint…")
                        .small()
                        .icon(IconName::Checkpoint)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            checkpoint.act(cx, |studio| studio.execute(crate::commands::CommandId::Checkpoint))
                        }),
                )
                .child(KeyCaps::new("Ctrl+S")),
        )
        .child(super::group("Checkpoints · newest first", None, cx))
        // The timeline: now, then each checkpoint on a thread.
        .child(timeline_row(
            None,
            if uncommitted { "Now · changes since the last checkpoint" } else { "Now · same as the last checkpoint" }.into(),
            None,
            false,
            uncommitted,
            None,
            cx,
        ))
        .when_some(error, |this, error| {
            this.child(ui::inline_message(Tone::Danger, format!("The history could not be read: {error}"), cx))
        })
        .when(checkpoints.is_empty(), |this| {
            this.child(div().pl(r(22.0)).pt(r(4.0)).child(super::note("No checkpoints yet. Ctrl+S records one.", cx)))
        })
        .children(checkpoints.into_iter().enumerate().map(|(index, (id, message, when, chosen))| {
            let studio = studio.clone();
            timeline_row(
                Some(index),
                message.into(),
                Some(when.into()),
                chosen,
                false,
                Some(Box::new(move |_: &ClickEvent, _: &mut gpui::Window, cx: &mut App| {
                    let id = id.clone();
                    studio.act(cx, |studio| {
                        studio.history.toggle(&id);
                        studio.mark(Dirty::LAYOUT);
                    })
                })),
                cx,
            )
        }))
        .child(
            div()
                .pt(r(10.0))
                .flex()
                .flex_wrap()
                .gap(r(6.0))
                .child(
                    Button::new(
                        "show-changes",
                        if selected == 1 { "Show changes since then" } else { "Show changes between them" },
                    )
                    .small()
                    .primary()
                    .icon(IconName::Compare)
                    .disabled(selected == 0)
                    .on_click(move |_: &ClickEvent, _, cx| compare.act(cx, |studio| studio.compare_selected())),
                )
                .when(comparison.is_some(), |this| {
                    this.child(
                        Button::new("close-comparison-panel", "Close comparison")
                            .small()
                            .ghost()
                            .on_click(move |_: &ClickEvent, _, cx| close.act(cx, |studio| studio.close_comparison())),
                    )
                }),
        )
        .when_some(comparison, |this, (after_is_now, created, updated, deleted)| {
            let studio = studio.clone();
            this.child(super::group("What changed", None, cx))
                .when(!after_is_now, |this| {
                    this.child(ui::inline_message(Tone::Info, "The Surface shows the later checkpoint. Close the comparison to edit.", cx))
                })
                .when(created.is_empty() && updated.is_empty() && deleted.is_empty(), |this| {
                    this.child(super::note("No differences.", cx))
                })
                .children(created.into_iter().map(|(id, name)| change_row(&studio, Some(id), "+", name, theme.success.text, false, cx)))
                .children(updated.into_iter().map(|(id, name)| change_row(&studio, Some(id), "~", name, theme.info.text, false, cx)))
                .children(deleted.into_iter().map(|name| change_row(&studio, None, "−", name, theme.danger.text, true, cx)))
        })
        .into_any_element()
}

type Click = Box<dyn Fn(&ClickEvent, &mut gpui::Window, &mut App)>;

/// One point on the history's thread.
fn timeline_row(
    index: Option<usize>,
    title: SharedString,
    when: Option<SharedString>,
    chosen: bool,
    now_changed: bool,
    on_click: Option<Click>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let now = index.is_none();
    let dot = if now {
        if now_changed { theme.accent.solid } else { theme.text_faint }
    } else if chosen {
        theme.accent.solid
    } else {
        theme.border_strong
    };
    div()
        .id(match index {
            Some(index) => ("history-row", index).into(),
            None => gpui::ElementId::Name("history-now".into()),
        })
        .flex()
        .items_stretch()
        .gap(r(10.0))
        .min_h(r(40.0))
        .px(r(6.0))
        .mx(r(-6.0))
        .rounded(r(crate::tokens::radius::CONTROL + 2.0))
        .when(chosen, |this| this.bg(theme.accent.soft))
        .when_some(on_click, |this, on_click| {
            this.cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .role(gpui::Role::Button)
                .aria_selected(chosen)
                .on_click(move |event, window, cx| on_click(event, window, cx))
        })
        .child(
            div()
                .w(r(12.0))
                .flex()
                .flex_col()
                .items_center()
                .child(div().w(gpui::px(1.0)).h(r(12.0)).bg(if now { gpui::transparent_black() } else { theme.separator }))
                .child(
                    div()
                        .size(r(9.0))
                        .rounded_full()
                        .border_2()
                        .border_color(dot)
                        .when(chosen || (now && now_changed), |this| this.bg(dot)),
                )
                .child(div().w(gpui::px(1.0)).flex_1().bg(theme.separator)),
        )
        .child(
            div()
                .flex_1()
                .py(r(7.0))
                .flex()
                .flex_col()
                .gap(r(2.0))
                .child(
                    div()
                        .text_size(r(theme::text::SM))
                        .font_weight(if now { theme::REGULAR } else { theme::MEDIUM })
                        .text_color(if now { theme.text_muted } else { theme.text })
                        .child(title),
                )
                .when_some(when, |this, when| {
                    this.child(div().text_size(r(theme::text::XS)).text_color(theme.text_faint).child(when))
                }),
        )
}

fn change_row(
    studio: &Entity<Studio>,
    id: Option<agq_language::ElementId>,
    mark: &'static str,
    name: String,
    colour: gpui::Hsla,
    deleted: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let studio = studio.clone();
    div()
        .id(SharedString::from(format!("change-{mark}-{name}")))
        .h(r(24.0))
        .flex()
        .items_center()
        .gap(r(8.0))
        .text_size(r(theme::text::SM))
        .font_family(theme::MONO)
        .text_color(colour)
        .when(deleted, |this| this.line_through())
        .when_some(id, |this, id| {
            this.cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.show_element(id)))
        })
        .child(div().w(r(10.0)).child(mark))
        .child(name)
        .child(div().flex_1())
        .when(id.is_some(), |this| this.child(icon(IconName::ArrowRight).size(12.0).color(theme.text_faint)))
}
