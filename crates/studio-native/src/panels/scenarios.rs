//! The Scenarios tab (C-50, W7.4): every scenario of the project with the
//! newest result in each execution mode and whether it is still current. A
//! click opens it in the Run panel; the Run panel runs it.
use crate::{
    runs::ScenarioRow,
    studio::{Dirty, Studio, StudioEvent},
    ui::{self, ActiveTheme, Button, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_simulation::{Mode, RunStatus};
use gpui::{
    ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::FluentBuilder, uniform_list,
};
use std::rc::Rc;

pub struct ScenariosView {
    studio: Entity<Studio>,
    _subscription: Subscription,
}

impl ScenariosView {
    pub fn new(studio: Entity<Studio>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(&studio, |_, _, event: &StudioEvent, cx| {
            if event
                .0
                .intersects(Dirty::MODEL | Dirty::LAYOUT | Dirty::APPEARANCE)
            {
                cx.notify();
            }
        });
        ScenariosView {
            studio,
            _subscription: subscription,
        }
    }
}

/// The short word and tone for a mode's newest result.
pub fn result_chip(
    mode: Mode,
    status: RunStatus,
    all_passed: bool,
    current: bool,
) -> (String, Tone) {
    let short = short_mode(mode);
    if !current {
        return (format!("{short} · outdated"), Tone::Neutral);
    }
    match status {
        RunStatus::Walkthrough => (format!("{short} · shown"), Tone::Neutral),
        RunStatus::Blocked => (format!("{short} · cannot start"), Tone::Warning),
        RunStatus::Cancelled => (format!("{short} · cancelled"), Tone::Neutral),
        RunStatus::Stopped => (format!("{short} · stopped"), Tone::Danger),
        RunStatus::Completed if all_passed => (format!("{short} · passed"), Tone::Success),
        RunStatus::Completed => (format!("{short} · failed"), Tone::Danger),
    }
}

/// A mode in one word, for chips.
pub fn short_mode(mode: Mode) -> &'static str {
    match mode {
        Mode::Model => "Model",
        Mode::Replay => "Replay",
        Mode::Implementation => "Code",
        Mode::Live => "Live",
        Mode::Walkthrough => "Walkthrough",
    }
}

impl Render for ScenariosView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let rows: Rc<Vec<ScenarioRow>> =
            Rc::new(self.studio.update(cx, |studio, _| studio.scenario_rows()));
        let studio = self.studio.read(cx);
        let has_project = studio.project.is_some();
        let selected = studio.runs.selected;
        let running = studio.runs.active.as_ref().map(|a| a.scenario);
        let new_button = {
            let studio = self.studio.clone();
            Button::new("new-scenario", "New scenario")
                .small()
                .icon(IconName::Plus)
                .disabled(!has_project)
                .tooltip("New scenario", None)
                .on_click(move |_: &ClickEvent, _, cx| {
                    studio.act(cx, |studio| {
                        studio.execute(crate::commands::CommandId::NewScenario)
                    })
                })
        };
        let header = div()
            .flex_none()
            .h(r(40.0))
            .px(r(12.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .font_weight(theme::SEMIBOLD)
                    .text_color(theme.text_muted)
                    .child(format!("SCENARIOS {}", rows.len())),
            )
            .child(new_button);
        if !has_project {
            return div()
                .size_full()
                .p(r(16.0))
                .child(super::note("Open a project to see its scenarios.", cx))
                .into_any_element();
        }
        if rows.is_empty() {
            let studio = self.studio.clone();
            return div()
                .size_full()
                .flex()
                .flex_col()
                .child(header)
                .child(
                    div().flex_1().flex().items_center().justify_center().p(r(20.0)).child(
                        ui::EmptyState::new(
                            IconName::Scenario,
                            "No scenarios yet",
                            "A scenario says what should happen to a system: what goes in, how its parts or agents answer, and what must come out. It runs against the model, recordings, a live model or the real code.",
                        )
                        .action(
                            Button::new("empty-new-scenario", "New scenario")
                                .primary()
                                .icon(IconName::Plus)
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    studio.act(cx, |studio| {
                                        studio.execute(crate::commands::CommandId::NewScenario)
                                    })
                                }),
                        ),
                    ),
                )
                .into_any_element();
        }
        let count = rows.len();
        let entity = self.studio.clone();
        div()
            .id("scenario-list")
            .size_full()
            .flex()
            .flex_col()
            .role(gpui::Role::List)
            .aria_label(SharedString::from(format!("{count} scenarios")))
            .child(header)
            .child(
                uniform_list("scenario-rows", count, move |range, _, cx| {
                    let theme = cx.theme().clone();
                    range
                        .map(|index| {
                            let row = rows[index].clone();
                            let chosen = selected == Some(row.id);
                            let is_running = running == Some(row.id);
                            let studio = entity.clone();
                            let id = row.id;
                            let summary: Vec<String> = row
                                .latest
                                .iter()
                                .map(|(mode, s, current)| {
                                    result_chip(*mode, s.status, s.all_passed, *current).0
                                })
                                .collect();
                            let name = SharedString::from(format!(
                                "{}, subject {}{}",
                                row.name,
                                row.subject,
                                if summary.is_empty() {
                                    ", not run".to_string()
                                } else {
                                    format!(", {}", summary.join(", "))
                                }
                            ));
                            div()
                                .id(("scenario", index))
                                .h(r(58.0))
                                .mx(r(6.0))
                                .px(r(8.0))
                                .py(r(7.0))
                                .flex()
                                .gap(r(8.0))
                                .rounded(r(crate::tokens::radius::CONTROL + 2.0))
                                .cursor_pointer()
                                .when(chosen, |this| this.bg(theme.pressed))
                                .when(!chosen, |this| this.hover(|style| style.bg(theme.hover)))
                                .role(gpui::Role::ListItem)
                                .aria_selected(chosen)
                                .aria_label(name)
                                .on_click(move |event: &ClickEvent, _, cx| {
                                    let twice = event.click_count() >= 2;
                                    studio.act(cx, |studio| {
                                        studio.select_scenario(id);
                                        if twice {
                                            let mode = studio.runs.mode();
                                            if mode != Mode::Live {
                                                studio.start_run(mode);
                                            }
                                        }
                                    })
                                })
                                .relative()
                                .child(ui::target::target(format!("Scenario {}", row.name)))
                                .child(div().pt(r(2.0)).child(if is_running {
                                    ui::primitives::spinner(
                                        ("scenario-spin", index),
                                        14.0,
                                        theme.info.solid,
                                    )
                                    .into_any_element()
                                } else {
                                    icon(IconName::Scenario)
                                        .size(14.0)
                                        .color(if chosen {
                                            theme.text_secondary
                                        } else {
                                            theme.text_muted
                                        })
                                        .into_any_element()
                                }))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap(r(4.0))
                                        .child(
                                            div()
                                                .flex()
                                                .items_baseline()
                                                .gap(r(6.0))
                                                .min_w_0()
                                                .child(
                                                    div()
                                                        .text_size(r(theme::text::SM))
                                                        .font_weight(theme::MEDIUM)
                                                        .text_color(theme.text)
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .whitespace_nowrap()
                                                        .child(row.name.clone()),
                                                )
                                                .child(
                                                    div()
                                                        .flex_none()
                                                        .text_size(r(theme::text::XS))
                                                        .text_color(theme.text_faint)
                                                        .overflow_hidden()
                                                        .text_ellipsis()
                                                        .whitespace_nowrap()
                                                        .child(row.subject.clone()),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .gap(r(4.0))
                                                .overflow_hidden()
                                                .when(row.latest.is_empty(), |this| {
                                                    this.child(
                                                        div()
                                                            .text_size(r(theme::text::XS))
                                                            .text_color(theme.text_faint)
                                                            .child("Not run yet"),
                                                    )
                                                })
                                                .children(row.latest.iter().map(
                                                    |(mode, s, current)| {
                                                        let (label, tone) = result_chip(
                                                            *mode,
                                                            s.status,
                                                            s.all_passed,
                                                            *current,
                                                        );
                                                        ui::Badge::new(label).tone(tone)
                                                    },
                                                )),
                                        ),
                                )
                        })
                        .collect()
                })
                .flex_1()
                .min_h_0(),
            )
            .into_any_element()
    }
}
