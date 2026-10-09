//! The Requirements panel (C-55): each requirement on one line with what it
//! stands on, strongest evidence first, and never more than that: a
//! `satisfy` alone is a declaration, not evidence. Its ladder opens below:
//! what declares it satisfied, what the model calculates on the modelled
//! configuration, the scenarios that verify it and its linked tests. The
//! selected part can be declared as satisfying a requirement.
use crate::{
    studio::Studio,
    ui::{self, ActiveTheme, Button, Chip, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_implementation::requirements::Ladder;
use agq_simulation::requirements::{Evaluation, Status};
use gpui::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
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
    let open = studio_ref.requirements_open.clone();
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
    let headline = crate::requirements::rows_headline(&rows);
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
                .text_size(r(theme::text::SM))
                .line_height(r(18.0))
                .text_color(theme.text_muted)
                .child(headline),
        )
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            let studio_show = studio.clone();
            let studio_toggle = studio.clone();
            let studio_satisfy = studio.clone();
            let id = row.id;
            let expanded = open.contains(&id);
            let can_satisfy = editable && row.keyword == "requirement" && part.is_some();
            let standing = row.ladder.standing();
            let problems = row.problems.len();
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
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .child(
                            div()
                                .id(("requirement-title", index))
                                .flex_1()
                                .min_w_0()
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
                                ),
                        )
                        .when(problems > 0 && !expanded, |this| {
                            this.child(
                                Chip::new(format!(
                                    "{problems} problem{}",
                                    if problems == 1 { "" } else { "s" }
                                ))
                                .icon(IconName::Warning)
                                .tone(Tone::Warning),
                            )
                        })
                        .child(Chip::new(row.keyword))
                        .child(
                            Button::icon_only(
                                ("requirement-ladder", index),
                                if expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                },
                                if expanded {
                                    "Hide the evidence"
                                } else {
                                    "Show the evidence"
                                },
                            )
                            .small()
                            .on_click(move |_: &ClickEvent, _, cx| {
                                studio_toggle.act(cx, |studio| studio.toggle_requirement(id))
                            }),
                        ),
                )
                // One line: what it stands on, then the ladder in short.
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .min_w_0()
                        .when(!row.ladder.definition, |this| {
                            this.child(
                                Chip::new(standing.label())
                                    .tone(crate::requirements::tone(standing)),
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(r(theme::text::XS))
                                .text_color(theme.text_muted)
                                .child(row.ladder.summary()),
                        ),
                )
                .when(expanded, |this| {
                    this.when_some(row.doc.clone(), |this, doc| {
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
                    .when(!row.ladder.definition, |this| {
                        this.child(ladder(&row.ladder, &format!("panel-{index}"), studio, cx))
                    })
                    .children(
                        row.problems
                            .iter()
                            .map(|problem| ui::inline_message(Tone::Warning, problem.clone(), cx)),
                    )
                })
                .when(can_satisfy, |this| {
                    let label = format!(
                        "Declare satisfied by {}",
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

/// A rung's heading: what it is and what it can claim.
fn rung(title: &'static str, claim: &'static str, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(r(6.0))
        .pt(r(4.0))
        .child(
            div()
                .text_size(r(theme::text::XS))
                .font_weight(theme::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .child(
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_faint)
                .child(claim),
        )
        .into_any_element()
}

/// A quiet line of a rung.
fn line(text: impl Into<SharedString>, mono: bool, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .text_size(r(theme::text::XS))
        .line_height(r(16.0))
        .text_color(theme.text_muted)
        .when(mono, |this| this.font_family(theme::MONO))
        .child(text.into())
        .into_any_element()
}

/// How a calculation's conclusion is coloured.
fn status_tone(status: Status) -> Tone {
    match status {
        Status::Holds => Tone::Success,
        Status::Violated => Tone::Danger,
        Status::AssumptionsNotMet | Status::NotEvaluable => Tone::Neutral,
    }
}

/// One calculation: its conclusion and why, then each constraint with the
/// values used.
fn calculation(evaluation: &Evaluation, within: Option<&str>, cx: &App) -> AnyElement {
    let lines: Vec<AnyElement> = evaluation
        .assumptions
        .iter()
        .chain(&evaluation.required)
        .map(|result| line(result.line(), true, cx))
        .collect();
    let subject = match within {
        Some(within) => format!("for `{}`, within {within}", evaluation.subject),
        None => format!("for `{}`", evaluation.subject),
    };
    div()
        .flex()
        .flex_col()
        .gap(r(2.0))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(r(6.0))
                .child(Chip::new(evaluation.status.label()).tone(status_tone(evaluation.status)))
                .child(line(subject, false, cx)),
        )
        .child(line(evaluation.reason.clone(), false, cx))
        .children(lines)
        .into_any_element()
}

/// A requirement's ladder, rung by rung, each saying what it can claim.
/// The Inspector shows the same. A scenario opens in the Run panel.
pub(crate) fn ladder(ladder: &Ladder, id: &str, studio: &Entity<Studio>, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    let declared: AnyElement = if ladder.declared.is_empty() {
        line("Nothing declares it satisfied.", false, cx)
    } else {
        div()
            .flex()
            .flex_wrap()
            .gap(r(4.0))
            .children(
                ladder
                    .declared
                    .iter()
                    .map(|d| Chip::new(format!("satisfy by {}", d.by)).mono()),
            )
            .into_any_element()
    };
    let calculated: Vec<AnyElement> = if ladder.calculated.is_empty() {
        vec![line(
            "Nothing to calculate: no satisfy binds its subject.",
            false,
            cx,
        )]
    } else {
        ladder
            .calculated
            .iter()
            .map(|c| {
                calculation(
                    &c.evaluation,
                    c.within.as_ref().map(|(_, name)| name.as_str()),
                    cx,
                )
            })
            .collect()
    };
    let scenarios: Vec<AnyElement> = if ladder.scenarios.is_empty() {
        vec![line("No scenario verifies it.", false, cx)]
    } else {
        ladder
            .scenarios
            .iter()
            .enumerate()
            .map(|(index, scenario)| {
                let entity = studio.clone();
                let target = scenario.scenario;
                div()
                    .id(SharedString::from(format!("ladder-{id}-scenario-{index}")))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(r(4.0))
                    .px(r(4.0))
                    .mx(r(-4.0))
                    .rounded(r(crate::tokens::radius::CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.hover))
                    .on_click(move |_: &ClickEvent, _, cx| {
                        entity.act(cx, |studio| studio.select_scenario(target))
                    })
                    .child(icon(IconName::Scenario).size(12.0).color(theme.text_muted))
                    .child(line(scenario.name.clone(), true, cx))
                    .when(scenario.results.is_empty(), |this| {
                        this.child(ui::Badge::new("not run"))
                    })
                    .children(scenario.results.iter().map(|result| {
                        let (label, tone) = super::scenarios::result_chip(
                            result.mode,
                            result.status,
                            result.all_passed,
                            result.current,
                        );
                        ui::Badge::new(label).tone(tone)
                    }))
                    .into_any_element()
            })
            .collect()
    };
    let tests: Vec<AnyElement> = if ladder.tests.is_empty() {
        vec![line("No tests are linked to it.", false, cx)]
    } else {
        ladder
            .tests
            .iter()
            .map(|test| {
                let outcome = match &test.outcome {
                    None => "not run".to_string(),
                    Some((verdict, _, current)) => format!(
                        "{} ({})",
                        verdict.label(),
                        if *current { "current" } else { "outdated" }
                    ),
                };
                line(format!("{}: {outcome}", test.location), true, cx)
            })
            .collect()
    };
    div()
        .id(SharedString::from(format!("ladder-{id}")))
        .flex()
        .flex_col()
        .gap(r(4.0))
        .child(rung("Declared", "a claim, not evidence", cx))
        .child(declared)
        .child(rung(
            "Calculated from the model",
            "on the modelled configuration, not a built system",
            cx,
        ))
        .children(calculated)
        .child(rung(
            "Scenarios",
            "verification by running the model or the code",
            cx,
        ))
        .children(scenarios)
        .child(rung("Implementation", "linked tests", cx))
        .children(tests)
        .into_any_element()
}
