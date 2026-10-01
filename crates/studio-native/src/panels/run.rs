//! The Run panel (C-50, W7.4): the selected scenario, the execution mode,
//! Run and Stop, what the result may claim (status, freshness, checks kept
//! apart from completion), the live and implementation provenance, and the
//! trace: filtered, virtualised, stepped and played back from recorded
//! events. Playing back never suggests that an external action was undone.
use crate::{
    commands::CommandId,
    runs::TraceFilter,
    studio::{Dirty, Studio},
    ui::{self, ActiveTheme, Button, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_language::ElementId;
use agq_simulation::{
    CheckResult, EventKind, Freshness, Mode, RunResult, RunStatus, TraceEvent, Verdict,
};
use gpui::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, relative,
    uniform_list,
};
use std::rc::Rc;

/// The verdict as a tone and an icon: colour carries state, the icon the
/// meaning, so neither is needed alone.
pub fn verdict_look(verdict: Verdict) -> (Tone, IconName) {
    match verdict {
        Verdict::Passed => (Tone::Success, IconName::CircleCheck),
        Verdict::Failed => (Tone::Danger, IconName::CircleX),
        Verdict::NotRun => (Tone::Neutral, IconName::Circle),
        Verdict::Unsupported => (Tone::Neutral, IconName::CircleDot),
        Verdict::Blocked => (Tone::Warning, IconName::Alert),
        Verdict::Inconclusive => (Tone::Warning, IconName::Question),
    }
}

pub fn event_icon(kind: EventKind) -> IconName {
    match kind {
        EventKind::Step => IconName::ArrowRight,
        EventKind::Sent => IconName::ArrowUp,
        EventKind::Received => IconName::Enter,
        EventKind::Output => IconName::External,
        EventKind::StateEntered => IconName::CircleDot,
        EventKind::Transition => IconName::ArrowRight,
        EventKind::Assigned => IconName::Pencil,
        EventKind::Timer => IconName::Clock,
        EventKind::AgentCalled | EventKind::AgentAnswered => IconName::Agent,
        EventKind::AgentFailed => IconName::Alert,
        EventKind::Fallback => IconName::Retry,
        EventKind::StandIn => IconName::Component,
        EventKind::Check => IconName::Requirement,
        EventKind::Stopped => IconName::Stop,
        EventKind::Note => IconName::Info,
    }
}

/// A choice of one among a few, as toggle buttons that wrap on a narrow
/// column (a segmented control would cut the names short).
fn choices(
    id: &'static str,
    what: &'static str,
    labels: &[&'static str],
    chosen: usize,
    on_choose: impl Fn(usize, &mut App) + Clone + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_wrap()
        .gap(r(4.0))
        .role(gpui::Role::TabList)
        .children(labels.iter().enumerate().map(|(index, label)| {
            let on_choose = on_choose.clone();
            Button::new((id, index), *label)
                .accessible_label(format!("{what}: {label}"))
                .small()
                .ghost()
                .selected(index == chosen)
                .on_click(move |_: &ClickEvent, _, cx| on_choose(index, cx))
        }))
}

/// What a mode runs, in one sentence.
pub fn mode_meaning(mode: Mode) -> &'static str {
    match mode {
        Mode::Model => {
            "Runs the model's own behaviour with the scenario's stand-ins. Deterministic; nothing outside Agentique is called."
        }
        Mode::Replay => {
            "Agents answer from kept recordings, matched exactly. A missing recording stops the run; it never calls a model."
        }
        Mode::Implementation => {
            "Runs the real code through the project's harness, with the scenario's stand-ins in place of the parts they name."
        }
        Mode::Live => {
            "Agents answer from a real model, five samples of the whole scenario. It costs money, so it asks you first."
        }
        Mode::Walkthrough => "Shows the steps in order. Nothing runs and nothing is verified.",
    }
}

fn time(ms: u64) -> String {
    if ms >= 10_000 {
        format!("{:.1} s", ms as f64 / 1000.0)
    } else {
        format!("{ms} ms")
    }
}

pub fn render(studio: &Entity<Studio>, cx: &mut App) -> impl IntoElement {
    let theme = cx.theme().clone();
    let state = studio.read(cx);
    let Some(scenario) = state.runs.selected else {
        let studio = studio.clone();
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(r(20.0))
            .child(
                ui::EmptyState::new(
                    IconName::Scenario,
                    "No scenario chosen",
                    "Choose a scenario in the Scenarios tab to run it and follow its trace here.",
                )
                .action(
                    Button::new("show-scenarios", "Show scenarios")
                        .icon(IconName::Scenario)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |studio| {
                                studio.left = crate::studio::LeftTab::Scenarios;
                                studio.outline_hidden = false;
                                studio.mark(Dirty::LAYOUT);
                            })
                        }),
                ),
            )
            .into_any_element();
    };
    let Some(project) = state.project.as_ref() else {
        return div().into_any_element();
    };
    let tree = project.state().tree();
    if !tree.contains(scenario) {
        return div()
            .p(r(16.0))
            .child(super::note("This scenario no longer exists.", cx))
            .into_any_element();
    }
    let name = tree.effective_name(scenario).unwrap_or("?").to_string();
    let doc = crate::runs::doc_of(tree, scenario);
    let subject = crate::runs::subject_type(tree, scenario).unwrap_or_else(|| "no subject".into());
    let mode = state.runs.mode();
    let running = state.runs.active.is_some();
    let result = state.runs.result.clone();
    let freshness = state.runs.freshness.clone();
    let trusted = state.implementation.choice.is_some_and(|c| c.trusted);
    let editable = state.editable() && state.dialog.is_none() && !running;

    // ---- the top: the scenario, the mode, run, status and checks ----
    let mut top: Vec<AnyElement> = Vec::new();
    top.push(
        div()
            .flex()
            .flex_col()
            .gap(r(4.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(8.0))
                    .child(
                        icon(IconName::Scenario)
                            .size(16.0)
                            .color(theme.text_secondary),
                    )
                    .child(
                        div()
                            .text_size(r(theme::text::LG))
                            .font_weight(theme::SEMIBOLD)
                            .text_color(theme.text)
                            .child(name.clone()),
                    ),
            )
            .when_some(doc, |this, doc| {
                this.child(
                    div()
                        .text_size(r(theme::text::SM))
                        .line_height(r(18.0))
                        .text_color(theme.text_secondary)
                        .child(doc),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(r(4.0))
                    .pt(r(4.0))
                    .child(ui::Chip::new(format!("Subject {subject}")).icon(IconName::Part))
                    .children(verified(tree, scenario).into_iter().map(|(id, label)| {
                        let studio = studio.clone();
                        div()
                            .id(SharedString::from(format!("verifies-{}", id.raw())))
                            .cursor_pointer()
                            .on_click(move |_: &ClickEvent, _, cx| super::show(&studio, id, cx))
                            .child(ui::Chip::new(label).icon(IconName::Requirement))
                    })),
            )
            .into_any_element(),
    );
    // The mode.
    let chosen = Mode::ALL.iter().position(|m| *m == mode).unwrap_or(0);
    let labels = Mode::ALL.map(crate::panels::scenarios::short_mode);
    let chooser = studio.clone();
    top.push(
        div()
            .flex()
            .flex_col()
            .gap(r(6.0))
            .pt(r(14.0))
            .child(choices(
                "run-mode",
                "Mode",
                &labels,
                chosen,
                move |index, cx| {
                    chooser.act(cx, |studio| {
                        studio.runs.mode = Some(Mode::ALL[index]);
                        studio.load_latest_result();
                        studio.mark(Dirty::LAYOUT | Dirty::MODEL);
                    })
                },
            ))
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .line_height(r(16.0))
                    .text_color(theme.text_muted)
                    .child(format!("{}. {}", mode.label(), mode_meaning(mode))),
            )
            .into_any_element(),
    );
    // Run and Stop.
    let run_label = match mode {
        Mode::Live => "Evaluate live…",
        Mode::Walkthrough => "Show the steps",
        _ => "Run",
    };
    let actions = div()
        .flex()
        .items_center()
        .gap(r(6.0))
        .pt(r(10.0))
        .child({
            let studio = studio.clone();
            Button::new("run-scenario", run_label)
                .primary()
                .icon(IconName::Play)
                .shortcut("F5")
                .disabled(running)
                .on_click(move |_: &ClickEvent, _, cx| {
                    studio.act(cx, |studio| studio.execute(CommandId::RunScenario))
                })
        })
        .when(running, |this| {
            let studio = studio.clone();
            this.child(
                Button::new("stop-run", "Stop")
                    .icon(IconName::Stop)
                    .on_click(move |_: &ClickEvent, _, cx| {
                        studio.act(cx, |studio| studio.stop_run())
                    }),
            )
            .child(ui::primitives::spinner(
                "run-spinner",
                14.0,
                theme.info.solid,
            ))
            .children(state.runs.active.as_ref().map(|active| {
                div()
                    .text_size(r(theme::text::SM))
                    .text_color(theme.text_muted)
                    .child(format!(
                        "{} · {:.1} s",
                        active.mode.label(),
                        active.started.elapsed().as_secs_f32()
                    ))
            }))
        })
        .when(
            result.as_ref().is_some_and(|r| {
                r.mode == Mode::Live && r.live.as_ref().is_some_and(|l| !l.answers.is_empty())
            }),
            |this| {
                let studio = studio.clone();
                this.child(
                    Button::new("keep-recordings", "Keep as recordings")
                        .icon(IconName::Save)
                        .tooltip("Keep this run's answers for replay", None)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |studio| studio.keep_recordings())
                        }),
                )
            },
        );
    top.push(actions.into_any_element());
    let editor = scenario_editor(studio, scenario, editable, cx);
    if mode == Mode::Implementation && !trusted {
        let studio = studio.clone();
        top.push(
            div()
                .pt(r(10.0))
                .child(
                    ui::Banner::new(
                        Tone::Warning,
                        "Running the real code needs trusted-local execution, which is off for this project.",
                    )
                    .action(
                        Button::new("trust-local", "Turn on…").small().on_click(move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |studio| {
                                studio.dialog = Some(crate::edit::Dialog::TrustLocal);
                                studio.mark(Dirty::OVERLAY);
                            })
                        }),
                    ),
                )
                .into_any_element(),
        );
    }
    let Some(result) = result else {
        top.push(
            div()
                .pt(r(14.0))
                .child(super::note(
                    format!("Not run in {} yet.", mode.label().to_lowercase()),
                    cx,
                ))
                .into_any_element(),
        );
        top.extend(editor);
        return div()
            .id("run-panel")
            .size_full()
            .overflow_y_scroll()
            .p(r(14.0))
            .children(top)
            .into_any_element();
    };
    top.push(status_block(studio, &result, freshness.as_ref(), cx).into_any_element());
    if !result.blockers.is_empty() {
        top.push(
            super::group("Why it could not start", Some(result.blockers.len()), cx)
                .into_any_element(),
        );
        for (element, message) in &result.blockers {
            top.push(problem_row(
                studio,
                ElementId::from_raw(*element),
                message,
                IconName::Warning,
                cx,
            ));
        }
    }
    if !result.checks.is_empty() {
        top.push(super::group("Checks", Some(result.checks.len()), cx).into_any_element());
        for (index, check) in result.checks.iter().enumerate() {
            top.push(check_row(studio, index, check, cx));
        }
    }
    if let Some(live) = &result.live {
        let failures = if live.failures.is_empty() {
            "no agent failures".to_string()
        } else {
            live.failures
                .iter()
                .map(|(what, n)| format!("{what} {n}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let provenance = result.provenance.live.as_ref().map(|p| {
            format!(
                "{}/{} · instructions {} ({})",
                p.provider,
                p.model,
                &p.instructions_digest[..p.instructions_digest.len().min(10)],
                p.instructions_source
            )
        });
        top.push(super::group("Live evaluation", None, cx).into_any_element());
        top.push(
            detail_lines(
                vec![
                    format!(
                        "{} samples, {} completed; {failures}",
                        live.samples, live.completed
                    ),
                    format!(
                        "Estimated cost {}; median latency {}",
                        live.cost_usd
                            .map_or("unknown".into(), |c| format!("${c:.4}")),
                        live.latency_ms_median
                            .map_or("unknown".into(), |ms| format!("{ms} ms"))
                    ),
                ]
                .into_iter()
                .chain(provenance)
                .collect(),
                cx,
            )
            .into_any_element(),
        );
    }
    if let Some(code) = &result.provenance.implementation {
        top.push(super::group("Code", None, cx).into_any_element());
        top.push(
            detail_lines(
                vec![
                    format!(
                        "Commit {}{}",
                        &code.commit[..code.commit.len().min(10)],
                        if code.dirty {
                            ", with changes not committed"
                        } else {
                            ""
                        }
                    ),
                    format!("Harness: {}", code.harness),
                ],
                cx,
            )
            .into_any_element(),
        );
    }

    top.extend(editor);

    // ---- the trace ----
    let visible = Rc::new(state.runs.visible_events());
    let events: Rc<Vec<TraceEvent>> = Rc::new(result.trace.clone());
    let cursor = state.runs.cursor;
    let playing = state.runs.playing;
    let follow = state.runs.follow;
    let filter = state.runs.filter;
    let drawn = state.result_is_current();
    let trace_header = trace_header(
        studio,
        visible.len(),
        events.len(),
        filter,
        playing,
        follow,
        drawn,
        cx,
    );
    let details = cursor
        .and_then(|c| events.get(c).cloned())
        .map(|e| event_details(&e, cx));
    let list = {
        let studio = studio.clone();
        let visible = visible.clone();
        let events = events.clone();
        uniform_list("trace-rows", visible.len(), move |range, _, cx| {
            let theme = cx.theme().clone();
            range
                .map(|row| {
                    let index = visible[row];
                    let event = &events[index];
                    let chosen = cursor == Some(index);
                    let studio = studio.clone();
                    let failed = matches!(event.kind, EventKind::AgentFailed | EventKind::Stopped)
                        || (event.kind == EventKind::Check && event.text.ends_with("failed"));
                    let colour = if failed {
                        theme.danger.text
                    } else {
                        theme.text_secondary
                    };
                    div()
                        .id(("trace", index))
                        .h(r(26.0))
                        .mx(r(6.0))
                        .px(r(8.0))
                        .flex()
                        .items_center()
                        .gap(r(8.0))
                        .rounded(r(crate::tokens::radius::CONTROL))
                        .cursor_pointer()
                        .when(chosen, |this| this.bg(theme.accent.soft))
                        .when(!chosen, |this| this.hover(|style| style.bg(theme.hover)))
                        .role(gpui::Role::ListItem)
                        .aria_selected(chosen)
                        .aria_label(SharedString::from(format!(
                            "{} at {}: {}",
                            event.kind_label(),
                            time(event.time_ms),
                            event.text
                        )))
                        .on_click(move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |studio| {
                                studio.runs.playing = false;
                                studio.set_cursor(Some(index));
                            })
                        })
                        .child(
                            div()
                                .w(r(52.0))
                                .flex_none()
                                .text_size(r(theme::text::XS))
                                .font_family(theme::MONO)
                                .text_color(theme.text_faint)
                                .child(time(event.time_ms)),
                        )
                        .child(icon(event_icon(event.kind)).size(12.0).color(colour))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(r(theme::text::SM))
                                .text_color(if failed {
                                    theme.danger.text
                                } else {
                                    theme.text
                                })
                                .child(event.text.clone()),
                        )
                })
                .collect()
        })
        .flex_1()
        .min_h_0()
    };
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div()
                .id("run-top")
                .flex_none()
                .max_h(relative(0.58))
                .overflow_y_scroll()
                .p(r(14.0))
                .pb(r(8.0))
                .children(top),
        )
        .child(trace_header)
        .child(list)
        .when_some(details, |this, details| this.child(details))
        .into_any_element()
}

trait KindLabel {
    fn kind_label(&self) -> &'static str;
}

impl KindLabel for TraceEvent {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            EventKind::Step => "Step",
            EventKind::Sent => "Sent",
            EventKind::Received => "Received",
            EventKind::Output => "Output",
            EventKind::StateEntered => "State",
            EventKind::Transition => "Transition",
            EventKind::Assigned => "Assigned",
            EventKind::Timer => "Time passed",
            EventKind::AgentCalled => "Agent asked",
            EventKind::AgentAnswered => "Agent answered",
            EventKind::AgentFailed => "Agent failed",
            EventKind::Fallback => "Fallback",
            EventKind::StandIn => "Stand-in",
            EventKind::Check => "Check",
            EventKind::Stopped => "Stopped",
            EventKind::Note => "Note",
        }
    }
}

/// The requirements a scenario verifies.
fn verified(tree: &agq_language::Tree, scenario: ElementId) -> Vec<(ElementId, String)> {
    let mut out = Vec::new();
    for child in tree[scenario].children() {
        if tree[*child].kind != agq_language::ElementKind::Objective {
            continue;
        }
        for verify in tree[*child].children() {
            if let Some(target) = tree[*verify].target.as_ref().and_then(|t| t.target())
                && tree.contains(target)
            {
                out.push((
                    target,
                    format!("Verifies {}", tree.effective_name(target).unwrap_or("?")),
                ));
            }
        }
    }
    out
}

fn status_block(
    studio: &Entity<Studio>,
    result: &RunResult,
    freshness: Option<&Freshness>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let (tone, label) = match result.status {
        RunStatus::Completed if result.all_passed() => {
            (Tone::Success, "Completed; every check passed".to_string())
        }
        RunStatus::Completed => (
            Tone::Warning,
            "Completed; not every check passed".to_string(),
        ),
        RunStatus::Stopped => (
            Tone::Danger,
            format!(
                "Stopped: {}",
                result.stop.as_ref().map_or("?", |s| s.reason.code())
            ),
        ),
        RunStatus::Cancelled => (Tone::Neutral, "Cancelled".to_string()),
        RunStatus::Blocked => (Tone::Warning, "Could not start".to_string()),
        RunStatus::Walkthrough => (
            Tone::Neutral,
            "Walkthrough: nothing ran, nothing verified".to_string(),
        ),
    };
    let facts = format!(
        "{} · {} logical · {} events · {} · {}",
        result.mode.label(),
        time(result.logical_ms),
        result.events_processed,
        result.started[..result.started.len().min(16)].replace('T', " "),
        result.provenance.runner
    );
    let stop = result.stop.clone();
    let outdated = match freshness {
        Some(Freshness::Outdated(why)) => Some(why.clone()),
        _ => None,
    };
    div()
        .pt(r(14.0))
        .flex()
        .flex_col()
        .gap(r(6.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .child(ui::Badge::new(label).tone(tone))
                .child(match &outdated {
                    Some(_) => ui::Badge::new("Outdated").tone(Tone::Neutral),
                    None => ui::Badge::new("Current").tone(Tone::Info),
                }),
        )
        .child(
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_muted)
                .child(facts),
        )
        .when_some(stop, |this, stop| {
            let studio = studio.clone();
            this.child(
                div()
                    .id("stop-where")
                    .flex()
                    .items_start()
                    .gap(r(6.0))
                    .text_size(r(theme::text::SM))
                    .line_height(r(18.0))
                    .text_color(theme.text)
                    .when_some(stop.element, |this, element| {
                        this.cursor_pointer().on_click(move |_: &ClickEvent, _, cx| {
                            super::show(&studio, ElementId::from_raw(element), cx)
                        })
                    })
                    .child(div().pt(r(2.0)).child(icon(IconName::Alert).size(13.0).color(theme.danger.text)))
                    .child(div().flex_1().min_w_0().child(stop.message)),
            )
        })
        .when_some(outdated, |this, why| {
            let studio = studio.clone();
            this.child(
                ui::Banner::new(
                    Tone::Neutral,
                    format!("This result describes an earlier version: {why}. Its trace is not drawn on the Surface."),
                )
                .action(Button::new("run-again", "Run again").small().on_click(move |_: &ClickEvent, _, cx| {
                    studio.act(cx, |studio| studio.execute(CommandId::RunScenario))
                })),
            )
        })
}

fn problem_row(
    studio: &Entity<Studio>,
    element: ElementId,
    message: &str,
    glyph: IconName,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme().clone();
    let studio = studio.clone();
    div()
        .id(SharedString::from(format!(
            "blocker-{}-{}",
            element.raw(),
            message.len()
        )))
        .flex()
        .gap(r(8.0))
        .py(r(5.0))
        .px(r(6.0))
        .mx(r(-6.0))
        .rounded(r(crate::tokens::radius::CONTROL))
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover))
        .on_click(move |_: &ClickEvent, _, cx| super::show(&studio, element, cx))
        .child(
            div()
                .pt(r(2.0))
                .child(icon(glyph).size(13.0).color(theme.warning.text)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(r(theme::text::SM))
                .line_height(r(18.0))
                .text_color(theme.text)
                .child(message.to_string()),
        )
        .into_any_element()
}

fn check_row(studio: &Entity<Studio>, index: usize, check: &CheckResult, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    let (tone, glyph) = verdict_look(check.verdict);
    let (_, colour, _) = tone.colours(&theme);
    let studio = studio.clone();
    let element = ElementId::from_raw(check.element);
    let samples = check.samples.as_ref().map(|s| {
        format!(
            "{}/{} passed · 95% interval {:.2}–{:.2}",
            s.passed, s.samples, s.interval.0, s.interval.1
        )
    });
    div()
        .id(("check", index))
        .flex()
        .gap(r(8.0))
        .py(r(6.0))
        .px(r(6.0))
        .mx(r(-6.0))
        .rounded(r(crate::tokens::radius::CONTROL))
        .cursor_pointer()
        .hover(|style| style.bg(theme.hover))
        .role(gpui::Role::Button)
        .aria_label(SharedString::from(format!(
            "Check {}: {}. {}",
            check.name,
            check.verdict.label(),
            check.message
        )))
        .on_click(move |_: &ClickEvent, _, cx| super::show(&studio, element, cx))
        .child(div().pt(r(2.0)).child(icon(glyph).size(14.0).color(colour)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(r(2.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .child(
                            div()
                                .text_size(r(theme::text::SM))
                                .font_weight(theme::MEDIUM)
                                .text_color(theme.text)
                                .child(check.name.clone()),
                        )
                        .child(ui::Badge::new(check.verdict.label()).tone(tone))
                        .when(check.implicit, |this| {
                            this.child(ui::Badge::new("implicit"))
                        })
                        .when(!check.deterministic, |this| {
                            this.child(ui::Badge::new("sampled"))
                        }),
                )
                .when(!check.expression.is_empty(), |this| {
                    this.child(
                        div()
                            .text_size(r(theme::text::XS))
                            .font_family(theme::MONO)
                            .text_color(theme.text_muted)
                            .child(check.expression.clone()),
                    )
                })
                .child(
                    div()
                        .text_size(r(theme::text::XS))
                        .line_height(r(16.0))
                        .text_color(theme.text_secondary)
                        .child(check.message.clone()),
                )
                .when_some(samples, |this, samples| {
                    this.child(
                        div()
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_muted)
                            .child(samples),
                    )
                }),
        )
        .into_any_element()
}

fn detail_lines(lines: Vec<String>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(3.0))
        .children(lines.into_iter().map(|line| {
            div()
                .text_size(r(theme::text::XS))
                .line_height(r(16.0))
                .text_color(theme.text_secondary)
                .child(line)
        }))
}

#[allow(clippy::too_many_arguments)]
fn trace_header(
    studio: &Entity<Studio>,
    shown: usize,
    total: usize,
    filter: TraceFilter,
    playing: bool,
    follow: bool,
    drawn: bool,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let control = |id: &'static str,
                   glyph: IconName,
                   label: &'static str,
                   shortcut: Option<&'static str>,
                   command: CommandId| {
        let studio = studio.clone();
        Button::icon_only(id, glyph, label)
            .ghost()
            .small()
            .tooltip(label, shortcut)
            .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.execute(command)))
    };
    let chosen = TraceFilter::ALL
        .iter()
        .position(|f| *f == filter)
        .unwrap_or(0);
    let filters = TraceFilter::ALL.map(TraceFilter::label);
    let chooser = studio.clone();
    let follower = studio.clone();
    div()
        .flex_none()
        .border_t_1()
        .border_color(theme.separator)
        .px(r(12.0))
        .pt(r(8.0))
        .pb(r(6.0))
        .flex()
        .flex_col()
        .gap(r(6.0))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(r(4.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .child(icon(IconName::Trace).size(13.0).color(theme.text_muted))
                        .child(
                            div()
                                .text_size(r(theme::text::XS))
                                .font_weight(theme::SEMIBOLD)
                                .text_color(theme.text_muted)
                                .child(if shown == total {
                                    format!("TRACE {total}")
                                } else {
                                    format!("TRACE {shown} OF {total}")
                                }),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(r(2.0))
                        .child(control(
                            "trace-first",
                            IconName::SkipBack,
                            "First event",
                            None,
                            CommandId::TraceFirst,
                        ))
                        .child(control(
                            "trace-back",
                            IconName::ChevronLeft,
                            "Step back (shows what was recorded; undoes nothing)",
                            Some("["),
                            CommandId::TraceBack,
                        ))
                        .child(control(
                            "trace-play",
                            if playing {
                                IconName::Pause
                            } else {
                                IconName::Play
                            },
                            if playing { "Pause" } else { "Play" },
                            Some("\\"),
                            CommandId::TracePlay,
                        ))
                        .child(control(
                            "trace-forward",
                            IconName::ChevronRight,
                            "Step forward",
                            Some("]"),
                            CommandId::TraceForward,
                        ))
                        .child(control(
                            "trace-last",
                            IconName::SkipForward,
                            "Last event",
                            None,
                            CommandId::TraceLast,
                        ))
                        .child(
                            Button::icon_only(
                                "trace-follow",
                                IconName::Locate,
                                "Follow on the Surface",
                            )
                            .ghost()
                            .small()
                            .selected(follow)
                            .tooltip("Follow on the Surface", None)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                follower.act(cx, |studio| {
                                    studio.runs.follow = !studio.runs.follow;
                                    studio.mark(Dirty::LAYOUT);
                                })
                            }),
                        ),
                ),
        )
        .child(choices(
            "trace-filter",
            "Show",
            &filters,
            chosen,
            move |index, cx| {
                chooser.act(cx, |studio| {
                    studio.runs.filter = TraceFilter::ALL[index];
                    studio.mark(Dirty::LAYOUT);
                })
            },
        ))
        .when(!drawn && total > 0, |this| {
            this.child(
                div()
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_faint)
                    .child("Not drawn on the Surface: the result is not current."),
            )
        })
}

fn event_details(event: &TraceEvent, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    div()
        .flex_none()
        .border_t_1()
        .border_color(theme.separator)
        .bg(theme.raised)
        .px(r(14.0))
        .py(r(10.0))
        .flex()
        .flex_col()
        .gap(r(4.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .child(
                    icon(event_icon(event.kind))
                        .size(13.0)
                        .color(theme.text_secondary),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(r(theme::text::XS))
                        .font_weight(theme::SEMIBOLD)
                        .text_color(theme.text_muted)
                        .child(format!(
                            "{} · {}",
                            event.kind_label().to_uppercase(),
                            time(event.time_ms)
                        )),
                )
                .child(
                    div()
                        .text_size(r(theme::text::XS))
                        .text_color(theme.text_faint)
                        .child("recorded; stepping undoes nothing"),
                ),
        )
        .child(
            div()
                .text_size(r(theme::text::SM))
                .line_height(r(18.0))
                .text_color(theme.text)
                .child(event.text.clone()),
        )
        .when_some(event.path.clone(), |this, path| {
            this.child(
                div()
                    .text_size(r(theme::text::XS))
                    .font_family(theme::MONO)
                    .text_color(theme.text_muted)
                    .child(path),
            )
        })
        .when_some(event.source.clone(), |this, source| {
            this.child(ui::Chip::new(format!("From {source}")).icon(IconName::Agent))
        })
        .children(event.values.iter().map(|(name, value)| {
            div()
                .text_size(r(theme::text::XS))
                .line_height(r(16.0))
                .text_color(theme.text_secondary)
                .child(format!("{name}: {value}"))
        }))
        .into_any_element()
}

/// What the "Add" control offers.
const ADD_KINDS: [&str; 5] = ["Send", "Wait for", "Stand-in", "Check", "Time"];

/// One line per member of the scenario, in order: what it sends, waits for
/// and checks, and the stand-ins it sets up.
pub(crate) fn step_rows(
    tree: &agq_language::Tree,
    scenario: ElementId,
) -> Vec<(ElementId, IconName, &'static str, String)> {
    let mut rows = Vec::new();
    for child in tree[scenario].children().iter().copied() {
        let e = &tree[child];
        let via = e
            .via
            .as_ref()
            .map(|v| format!(" via {v}"))
            .unwrap_or_default();
        let expression = |x: &Option<agq_language::Expression>| {
            x.as_ref()
                .map(|x| agq_language::print_expression(tree, child, x))
                .unwrap_or_default()
        };
        let row = match e.kind {
            agq_language::ElementKind::Send => (
                IconName::ArrowUp,
                "Send",
                format!("{}{via}", expression(&e.expression)),
            ),
            agq_language::ElementKind::Accept if e.after => (
                IconName::Clock,
                "Wait",
                format!("{} ms", expression(&e.expression)),
            ),
            agq_language::ElementKind::Accept => (
                IconName::Enter,
                "Wait for",
                format!(
                    "{} : {}{via}",
                    tree.effective_name(child).unwrap_or("?"),
                    e.typed_by
                        .first()
                        .map(ToString::to_string)
                        .unwrap_or_default()
                ),
            ),
            agq_language::ElementKind::AssertConstraint => (
                IconName::Requirement,
                "Check",
                format!(
                    "{}: {}",
                    tree.effective_name(child).unwrap_or("check"),
                    expression(&e.expression)
                ),
            ),
            agq_language::ElementKind::Part => {
                let Some(features) = crate::runs::stand_in_features(tree, child) else {
                    continue;
                };
                let text = |name: &str| {
                    features
                        .iter()
                        .find(|(n, ..)| *n == name)
                        .map(|(_, _, t)| t.clone())
                        .unwrap_or_default()
                };
                let outcome = text("outcome");
                let outcome = outcome.rsplit("::").next().unwrap_or("answer").to_string();
                (
                    IconName::Component,
                    "Stand-in",
                    format!(
                        "{} {}{}",
                        text("target"),
                        if outcome.is_empty() {
                            "answer".into()
                        } else {
                            outcome
                        },
                        if text("latencyMs").is_empty() {
                            String::new()
                        } else {
                            format!(" after {} ms", text("latencyMs"))
                        }
                    ),
                )
            }
            _ => continue,
        };
        rows.push((child, row.0, row.1, row.2));
    }
    rows
}

/// The scenario's steps and the controls that add to it.
fn scenario_editor(
    studio: &Entity<Studio>,
    scenario: ElementId,
    editable: bool,
    cx: &App,
) -> Vec<AnyElement> {
    let theme = cx.theme().clone();
    let state = studio.read(cx);
    let Some(tree) = state.project.as_ref().map(|p| p.state().tree()) else {
        return Vec::new();
    };
    let rows = step_rows(tree, scenario);
    let mut out: Vec<AnyElement> = Vec::new();
    out.push(super::group("Steps", Some(rows.len()), cx).into_any_element());
    if rows.is_empty() {
        out.push(
            div()
                .text_size(r(theme::text::SM))
                .text_color(theme.text_muted)
                .child("Nothing yet: send something in, wait for what comes out, and check it.")
                .into_any_element(),
        );
    }
    for (index, (id, glyph, label, text)) in rows.into_iter().enumerate() {
        let inspect = studio.clone();
        let remove = studio.clone();
        out.push(
            div()
                .id(("step", index))
                .min_h(r(26.0))
                .px(r(6.0))
                .mx(r(-6.0))
                .flex()
                .items_center()
                .gap(r(8.0))
                .rounded(r(crate::tokens::radius::CONTROL))
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .role(gpui::Role::Button)
                .aria_label(SharedString::from(format!("{label} {text}")))
                .relative()
                .child(ui::target::target(format!("Step {label}")))
                .on_click(move |_: &ClickEvent, _, cx| {
                    inspect.act(cx, |studio| {
                        studio.inspected = Some((studio.selection.primary.clone(), id));
                        studio.panel = crate::studio::Panel::Inspector;
                        studio.mark(Dirty::SELECTION | Dirty::LAYOUT);
                    })
                })
                .child(icon(glyph).size(13.0).color(theme.text_muted))
                .child(
                    div()
                        .w(r(62.0))
                        .flex_none()
                        .text_size(r(theme::text::SM))
                        .text_color(theme.text_muted)
                        .child(label),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .font_family(theme::MONO)
                        .text_size(r(theme::text::SM))
                        .text_color(theme.text_secondary)
                        .child(text),
                )
                .when(editable, |this| {
                    this.child(
                        Button::icon_only(("step-remove", index), IconName::Trash, "Remove")
                            .ghost()
                            .small()
                            .tooltip("Remove", None)
                            .on_click(move |_: &ClickEvent, _, cx| {
                                remove.act(cx, |studio| {
                                    studio.operation(
                                        &format!("Remove a {} step", label.to_lowercase()),
                                        agq_system_state::Operation::Delete { element: id },
                                    );
                                })
                            }),
                    )
                })
                .into_any_element(),
        );
    }
    if !editable {
        return out;
    }
    // Add: what may pass through the subject's ports, the parts a stand-in
    // may replace, a check, a wait.
    let adding = state.runs.adding.min(ADD_KINDS.len() - 1);
    let chooser = studio.clone();
    out.push(
        div()
            .pt(r(10.0))
            .flex()
            .flex_col()
            .gap(r(6.0))
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .font_weight(theme::SEMIBOLD)
                    .text_color(theme.text_muted)
                    .child("ADD"),
            )
            .child(choices(
                "add-kind",
                "Add",
                &ADD_KINDS,
                adding,
                move |index, cx| {
                    chooser.act(cx, |studio| {
                        studio.runs.adding = index;
                        studio.mark(Dirty::LAYOUT);
                    })
                },
            ))
            .into_any_element(),
    );
    let mut options: Vec<(String, crate::runs::NewStep)> = Vec::new();
    match adding {
        0 | 1 => {
            for port in state.scenario_ports(scenario) {
                let list = if adding == 0 {
                    &port.sends
                } else {
                    &port.accepts
                };
                for ty in list {
                    let arrow = if adding == 0 { "→" } else { "←" };
                    let step = if adding == 0 {
                        crate::runs::NewStep::Send {
                            port: port.path.clone(),
                            ty: ty.clone(),
                        }
                    } else {
                        crate::runs::NewStep::Accept {
                            port: port.path.clone(),
                            ty: ty.clone(),
                        }
                    };
                    options.push((format!("{} {arrow} {}", ty.1, port.label), step));
                }
            }
        }
        2 => {
            for (path, label) in state.stand_in_targets(scenario) {
                options.push((
                    label,
                    crate::runs::NewStep::StandIn {
                        target: path,
                        outcome: "answer".into(),
                    },
                ));
            }
        }
        3 => options.push((
            "A condition on what came out (write it in the Inspector)".into(),
            crate::runs::NewStep::Check {
                expression: "true".into(),
            },
        )),
        _ => {
            for ms in [100, 1_000, 5_000, 60_000] {
                let label = if ms >= 1_000 {
                    format!("{} s", ms / 1_000)
                } else {
                    format!("{ms} ms")
                };
                options.push((
                    format!("Let {label} pass"),
                    crate::runs::NewStep::Wait { ms },
                ));
            }
        }
    }
    if options.is_empty() {
        out.push(
            div()
                .text_size(r(theme::text::SM))
                .text_color(theme.text_muted)
                .child(match adding {
                    0 => "The subject has no port that takes an item in.",
                    1 => "The subject has no port that gives an item out.",
                    _ => "The subject has no parts to stand in for.",
                })
                .into_any_element(),
        );
    }
    for (index, (label, step)) in options.into_iter().enumerate() {
        let entity = studio.clone();
        let opens = matches!(step, crate::runs::NewStep::Check { .. });
        out.push(
            div()
                .id(("add-option", index))
                .min_h(r(26.0))
                .px(r(6.0))
                .mx(r(-6.0))
                .flex()
                .items_center()
                .gap(r(8.0))
                .rounded(r(crate::tokens::radius::CONTROL))
                .cursor_pointer()
                .hover(|style| style.bg(theme.hover))
                .role(gpui::Role::Button)
                .aria_label(SharedString::from(format!("Add {label}")))
                .relative()
                .child(ui::target::target(format!("Add {label}")))
                .on_click(move |_: &ClickEvent, _, cx| {
                    let step = step.clone();
                    entity.act(cx, |studio| {
                        let before = studio.project.as_ref().map(|p| p.state().tree().next_id());
                        match studio.add_to_scenario(scenario, step) {
                            Ok(()) if opens => {
                                if let Some(created) = before {
                                    studio.inspected =
                                        Some((studio.selection.primary.clone(), created));
                                    studio.panel = crate::studio::Panel::Inspector;
                                }
                            }
                            Ok(()) => {}
                            Err(error) => studio.status = error,
                        }
                        studio.mark(Dirty::MODEL | Dirty::LAYOUT | Dirty::STATUS);
                    })
                })
                .child(icon(IconName::Plus).size(12.0).color(theme.accent.text))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(r(theme::text::SM))
                        .child(label),
                )
                .into_any_element(),
        );
    }
    out
}
