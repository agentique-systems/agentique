//! The Requirements panel: each requirement with its subject, what
//! satisfies it and its problems; the selected part can be recorded as
//! satisfying a requirement.
use crate::{
    studio::Studio,
    ui::{self, ActiveTheme, Button, Chip, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use gpui::{
    App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};

pub fn render(studio: &Entity<Studio>, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme().clone();
    if studio.read(cx).project.is_none() {
        return div()
            .p(r(16.0))
            .child(super::note("Open a project to see its requirements.", cx))
            .into_any_element();
    }
    let rows = studio.update(cx, |studio, _| studio.requirements());
    let studio_ref = studio.read(cx);
    let editable = studio_ref.editable();
    let part = studio_ref.satisfying_part();
    let part_name = part.and_then(|part| {
        studio_ref
            .project
            .as_ref()
            .and_then(|p| p.state().tree().effective_name(part).map(str::to_string))
    });
    if rows.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(r(20.0))
            .child(
                ui::EmptyState::new(
                    IconName::Requirements,
                    "No requirements yet",
                    "A requirement says what the system must do; parts satisfy it.",
                )
                .hint("R", "create a requirement"),
            )
            .into_any_element();
    }
    let satisfied = rows
        .iter()
        .filter(|row| !row.satisfied_by.is_empty())
        .count();
    let total = rows.len();
    div()
        .id("requirements")
        .size_full()
        .overflow_y_scroll()
        .px(r(12.0))
        .pb(r(16.0))
        .flex()
        .flex_col()
        .gap(r(8.0))
        .child(
            div()
                .pt(r(12.0))
                .flex()
                .items_center()
                .gap(r(8.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.text_muted)
                .child(format!("{satisfied} of {total} satisfied"))
                .child(div().flex_1().min_w_0().child(ui::primitives::progress(
                    Some(satisfied as f32 / total.max(1) as f32),
                    cx,
                ))),
        )
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            let studio_show = studio.clone();
            let studio_satisfy = studio.clone();
            let id = row.id;
            let unsatisfied = row.satisfied_by.is_empty();
            let can_satisfy = editable && row.keyword == "requirement" && part.is_some();
            div()
                .id(("requirement", index))
                .p(r(10.0))
                .rounded(r(crate::tokens::radius::CARD))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border)
                .flex()
                .flex_col()
                .gap(r(6.0))
                .child(
                    div()
                        .id(("requirement-title", index))
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .cursor_pointer()
                        .on_click(move |_: &ClickEvent, _, cx| {
                            studio_show.act(cx, |studio| studio.show_element(id))
                        })
                        .child(
                            icon(IconName::Requirement)
                                .size(14.0)
                                .color(theme.warning.text),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_weight(theme::MEDIUM)
                                .font_family(theme::MONO)
                                .text_size(r(theme::text::SM))
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(SharedString::from(row.name.clone())),
                        )
                        .child(Chip::new(row.keyword)),
                )
                .when_some(row.doc.clone(), |this, doc| {
                    this.child(
                        div()
                            .text_size(r(theme::text::SM))
                            .line_height(r(18.0))
                            .text_color(theme.text_secondary)
                            .child(doc),
                    )
                })
                .when(!row.subjects.is_empty(), |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(r(4.0))
                            .items_center()
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_muted)
                            .child("Subject")
                            .children(
                                row.subjects
                                    .iter()
                                    .map(|subject| Chip::new(subject.clone()).mono()),
                            ),
                    )
                })
                .child(if unsatisfied {
                    ui::inline_message(Tone::Warning, "Not satisfied by anything yet", cx)
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(r(4.0))
                        .items_center()
                        .text_size(r(theme::text::XS))
                        .text_color(theme.success.text)
                        .child(
                            icon(IconName::CircleCheck)
                                .size(12.0)
                                .color(theme.success.text),
                        )
                        .child("Satisfied by")
                        .children(
                            row.satisfied_by
                                .iter()
                                .map(|by| Chip::new(by.clone()).mono().tone(Tone::Success)),
                        )
                        .into_any_element()
                })
                .children(
                    row.problems
                        .iter()
                        .map(|problem| ui::inline_message(Tone::Warning, problem.clone(), cx)),
                )
                .when(can_satisfy, |this| {
                    let label = format!(
                        "Satisfied by {}",
                        part_name
                            .clone()
                            .unwrap_or_else(|| "the selected part".into())
                    );
                    this.child(
                        div().child(
                            Button::new(("satisfy", index), label)
                                .small()
                                .icon(IconName::Satisfy)
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    if let Some(part) = part {
                                        studio_satisfy.act(cx, |studio| studio.satisfy(id, part));
                                    }
                                }),
                        ),
                    )
                })
        }))
        .into_any_element()
}
