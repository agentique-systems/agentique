//! The Objectives panel (C-53, ROADMAP §4.16): the Operator gives an intent,
//! a spend budget, how many improvements, and whether reviewed changes are
//! merged and the result adopted; then watches the objective's cycles (the
//! phase, the proposal, the checks, the review, the pull request, the
//! spend), pauses, steps, resumes or stops it, sends its agents a message,
//! and reads what every agent did. Before Start it shows each role's model,
//! why a fallback was taken, the credential and who pays; then each role's
//! model and spend (C-54).

use super::InspectorColumn;
use crate::{
    studio::Studio,
    ui::{self, ActiveTheme, Button, Switch, TextArea, TextField, Tone, r, theme},
    workspace::StudioExt,
};
use agq_orchestrator::record::{Access, Budgets, Objective, Permissions, RoleModel, State};
use agq_orchestrator::run::Command;
use gpui::{
    AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};
use gpui_base::input::{InputState, TextareaState};

/// What the Operator types in the panel.
pub struct Fields {
    intent: Entity<TextareaState>,
    usd: Entity<InputState>,
    cycles: Entity<InputState>,
    message: Entity<InputState>,
    merge: bool,
    adopt: bool,
}

impl Fields {
    pub fn new(window: &mut Window, cx: &mut Context<InspectorColumn>) -> Fields {
        let intent = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .placeholder("What should Agentique improve in itself?");
            state.set_auto_grow(3, 10, cx);
            state
        });
        let usd = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("5");
            state.set_value("5", window, cx);
            state
        });
        let cycles = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("1");
            state.set_value("1", window, cx);
            state
        });
        let message =
            cx.new(|cx| InputState::new(window, cx).placeholder("A message for the agent at work"));
        Fields {
            intent,
            usd,
            cycles,
            message,
            merge: true,
            adopt: true,
        }
    }

    pub fn render(
        &mut self,
        studio: &Entity<Studio>,
        _: &mut Window,
        cx: &mut Context<InspectorColumn>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let state = studio.read(cx);
        let running = state.objectives.running();
        let state_paused = state.objectives.paused();
        let current = state.objectives.current.clone();
        let message = state.objectives.message.clone();
        let activity: Vec<_> = state
            .objectives
            .activity
            .iter()
            .rev()
            .take(60)
            .cloned()
            .collect();
        let unfinished = !running && current.as_ref().is_some_and(Objective::active);
        let text = |t: String| {
            div()
                .text_size(r(theme::text::SM))
                .line_height(r(17.0))
                .text_color(theme.text_secondary)
                .child(t)
        };
        let mut body = div()
            .id("objectives")
            .size_full()
            .overflow_y_scroll()
            .p(r(12.0))
            .flex()
            .flex_col()
            .gap(r(10.0));
        if let Some(message) = message {
            body = body.child(ui::inline_message(Tone::Neutral, message, cx));
        }
        if !running && !unfinished {
            body = body.child(self.form(studio, cx));
        }
        if let Some(objective) = &current {
            body = body.child(summary(objective, running, &theme));
        }
        if running {
            let paused = state_paused;
            let act = |command: Command| {
                let studio = studio.clone();
                move |_: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                    let command = command.clone();
                    studio.act(cx, |s| s.objective_command(command))
                }
            };
            body = body.child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(if paused {
                        Button::new("objective-resume", "Resume")
                            .small()
                            .primary()
                            .on_click(act(Command::Resume))
                    } else {
                        Button::new("objective-pause", "Pause")
                            .small()
                            .tooltip("The agents hold at their next tool call; the Orchestrator before its next phase", None)
                            .on_click(act(Command::Pause))
                    })
                    .child(
                        Button::new("objective-step", "Step")
                            .small()
                            .disabled(!paused)
                            .tooltip("One tool call, or one phase, then hold again", None)
                            .on_click(act(Command::Step)),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("objective-stop", "Stop")
                            .small()
                            .danger()
                            .tooltip("Ends the objective: its sessions stop, its record stays", None)
                            .on_click(act(Command::Stop)),
                    ),
            );
            let field = self.message.clone();
            let send = {
                let studio = studio.clone();
                let field = field.clone();
                move |_: &ClickEvent, window: &mut Window, cx: &mut gpui::App| {
                    let text = field.read(cx).value().to_string();
                    if text.trim().is_empty() {
                        return;
                    }
                    field.update(cx, |state, cx| state.set_value("", window, cx));
                    studio.act(cx, |s| s.objective_command(Command::Message(text)));
                }
            };
            body = body.child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(
                        div()
                            .flex_1()
                            .child(TextField::new(&field).target("objective-message")),
                    )
                    .child(Button::new("objective-send", "Send").small().on_click(send)),
            );
        }
        if unfinished {
            let studio_continue = studio.clone();
            let studio_stop = studio.clone();
            body = body.child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(
                        Button::new("objective-continue", "Continue")
                            .small()
                            .primary()
                            .tooltip("Goes on from the phase it reached", None)
                            .on_click(move |_, _, cx| {
                                studio_continue.act(cx, |s| {
                                    if let Err(problem) = s.continue_objective() {
                                        s.objectives.message =
                                            Some(format!("Not continued: {problem}"));
                                    }
                                })
                            }),
                    )
                    .child(
                        Button::new("objective-stop-idle", "Stop")
                            .small()
                            .danger()
                            .on_click(move |_, _, cx| {
                                studio_stop.act(cx, |s| s.stop_idle_objective())
                            }),
                    ),
            );
        }
        if !activity.is_empty() {
            body = body.child(super::group("Activity", None, cx)).children(
                activity.into_iter().enumerate().map(|(i, line)| {
                    div()
                        .id(("objective-line", i))
                        .flex()
                        .gap(r(6.0))
                        .text_size(r(theme::text::XS))
                        .line_height(r(15.0))
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.text_faint)
                                .font_family(theme::MONO)
                                .child(line.at),
                        )
                        .child(
                            div()
                                .flex_none()
                                .w(r(74.0))
                                .text_color(theme.text_muted)
                                .font_weight(theme::MEDIUM)
                                .child(line.role),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(theme.text_secondary)
                                .child(line.text),
                        )
                }),
            );
        } else if current.is_none() {
            body = body.child(text(
                "Agents propose one improvement, implement it in a worktree, run Agentique's checks on a clean checkout, try it in a test instance, and get an independent review; a change that passes is merged, built, tried and adopted, and Agentique goes on in the new build. Every step shows here.".into(),
            ));
        }
        body.into_any_element()
    }

    fn form(
        &mut self,
        studio: &Entity<Studio>,
        cx: &mut Context<InspectorColumn>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let label = |t: &'static str| {
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_muted)
                .child(t)
        };
        let entity = cx.entity();
        let start = {
            let studio = studio.clone();
            let intent = self.intent.clone();
            let usd = self.usd.clone();
            let cycles = self.cycles.clone();
            let entity = entity.clone();
            move |_: &ClickEvent, window: &mut Window, cx: &mut gpui::App| {
                let text = intent.read(cx).value().to_string();
                let usd: f64 = usd.read(cx).value().trim().parse().unwrap_or(-1.0);
                let cycles: u32 = cycles.read(cx).value().trim().parse().unwrap_or(0);
                let (merge, adopt) = entity.read(cx).objective_fields().switches();
                if !(0.05..=100.0).contains(&usd) || !(1..=10).contains(&cycles) {
                    studio.act(cx, |s| {
                        s.objectives.message = Some(
                            "The budget is between $0.05 and $100, and 1 to 10 improvements."
                                .into(),
                        )
                    });
                    return;
                }
                let budgets = Budgets {
                    usd,
                    cycles,
                    ..Budgets::default()
                };
                let permissions = Permissions {
                    push: merge,
                    merge,
                    adopt: merge && adopt,
                    ..Permissions::default()
                };
                let started = studio.act(cx, |s| {
                    let result = s.start_objective(&text, budgets, permissions);
                    if let Err(problem) = &result {
                        s.objectives.message = Some(format!("Not started: {problem}"));
                    }
                    result
                });
                if started.is_ok() {
                    intent.update(cx, |state, cx| state.set_value("", window, cx));
                }
            }
        };
        let models = studio.read(cx).agent_models_each();
        let toggle = |which: bool| {
            let entity = entity.clone();
            move |on: bool, _: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |column, cx| {
                    column.objective_fields_mut().set(which, on);
                    cx.notify();
                })
            }
        };
        div()
            .flex()
            .flex_col()
            .gap(r(8.0))
            .child(super::group("New objective", None, cx))
            .child(TextArea::new(&self.intent).target("objective-intent"))
            .child(
                div()
                    .flex()
                    .gap(r(8.0))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(r(3.0))
                            .child(label("Spend budget (USD)"))
                            .child(TextField::new(&self.usd).target("objective-usd")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(r(3.0))
                            .child(label("Improvements"))
                            .child(TextField::new(&self.cycles).target("objective-cycles")),
                    ),
            )
            .child(
                Switch::new(
                    "objective-merge",
                    self.merge,
                    "Merge reviewed changes that pass every check",
                )
                .on_toggle(toggle(true)),
            )
            .child(
                Switch::new(
                    "objective-adopt",
                    self.merge && self.adopt,
                    "Build, try and restart in the result",
                )
                .disabled(!self.merge)
                .on_toggle(toggle(false)),
            )
            .child(super::group("Models", None, cx))
            .children(
                models
                    .into_iter()
                    .map(|(role, model)| route(role, model, &theme)),
            )
            .child(
                div().flex().child(div().flex_1()).child(
                    Button::new("objective-start", "Start")
                        .primary()
                        .tooltip("Starts the objective on Agentique's own repository", None)
                        .on_click(start),
                ),
            )
            .into_any_element()
    }

    fn switches(&self) -> (bool, bool) {
        (self.merge, self.adopt)
    }

    fn set(&mut self, merge: bool, on: bool) {
        if merge {
            self.merge = on;
        } else {
            self.adopt = on;
        }
    }
}

/// A role's model before Start (C-54): the model, effort and credential,
/// who pays, and why a fallback was taken; or why the role has none, which
/// keeps the objective from starting.
fn route(role: &str, model: Result<RoleModel, String>, theme: &ui::Theme) -> gpui::AnyElement {
    let (head, detail, warn) = match model {
        Ok(model) => (
            format!("{role} · {}", model.label()),
            match &model.fallback {
                Some(why) => format!(
                    "Its fallback: {} {why}. {}; {}.",
                    model.configured, model.credential, model.billed
                ),
                None => format!("{}; {}.", model.credential, model.billed),
            },
            model.fallback.is_some(),
        ),
        Err(problem) => (
            format!("{role} · not available"),
            format!("{problem}. Nothing starts until it has a model."),
            true,
        ),
    };
    div()
        .flex()
        .flex_col()
        .text_size(r(theme::text::XS))
        .line_height(r(15.0))
        .child(
            div()
                .text_color(theme.text_secondary)
                .font_weight(theme::MEDIUM)
                .child(head),
        )
        .child(
            div()
                .text_color(if warn {
                    theme.warning.text
                } else {
                    theme.text_muted
                })
                .child(detail),
        )
        .into_any_element()
}

/// Each role's model and what it spent, by model within it (C-54); at API
/// prices, "API-equivalent" for the Claude subscription.
fn role_lines(objective: &Objective) -> Vec<String> {
    let mut roles: Vec<String> = objective.models.iter().map(|m| m.role.clone()).collect();
    for role in objective.spent.roles.keys() {
        if !roles.contains(role) {
            roles.push(role.clone());
        }
    }
    let mut lines = Vec::new();
    for role in roles {
        let model = objective.models.iter().find(|m| m.role == role);
        let cost = objective.spent.role(&role);
        let equivalent = if model.is_some_and(|m| m.access == Access::Subscription) {
            " API-equivalent"
        } else {
            ""
        };
        let mut line = format!(
            "{role}: {}",
            model
                .map(RoleModel::label)
                .unwrap_or_else(|| "model not recorded".into())
        );
        if cost.tokens > 0 || cost.usd > 0.0 {
            line.push_str(&format!(
                " — ${:.2}{equivalent}{}, {} tokens",
                cost.usd,
                if cost.unknown { " (some unpriced)" } else { "" },
                cost.tokens
            ));
        }
        if let Some(model) = model
            && let Some(why) = &model.fallback
        {
            line.push_str(&format!(" (instead of {}: {why})", model.configured));
        }
        let by_model = objective.spent.roles.get(&role);
        if by_model.is_some_and(|models| models.len() > 1) {
            let parts: Vec<String> = by_model
                .into_iter()
                .flatten()
                .map(|(model, cost)| format!("{model} ${:.2}", cost.usd))
                .collect();
            line.push_str(&format!("; by model: {}", parts.join(", ")));
        }
        lines.push(line);
    }
    lines
}

/// The objective's record, in a few lines.
fn summary(objective: &Objective, running: bool, theme: &ui::Theme) -> gpui::AnyElement {
    let line = |t: String, strong: bool| {
        div()
            .text_size(r(theme::text::SM))
            .line_height(r(18.0))
            .text_color(if strong {
                theme.text
            } else {
                theme.text_secondary
            })
            .when(strong, |this| this.font_weight(theme::MEDIUM))
            .child(t)
    };
    let state = match objective.state {
        State::Running if running => "running",
        State::Running => "interrupted",
        State::Paused => "paused",
        State::Stopped => "stopped",
        State::Done => "done",
        State::Failed => "ended without finishing",
    };
    let mut lines = vec![
        line(objective.intent.clone(), true),
        line(
            format!(
                "{state} · ${:.2} of ${:.2}{} · {} tokens · {:.0} min",
                objective.spent.usd,
                objective.budgets.usd,
                if objective.spent.unknown {
                    " (some usage unpriced)"
                } else {
                    ""
                },
                objective.spent.tokens,
                objective.spent.seconds / 60.0
            ),
            false,
        ),
    ];
    for role in role_lines(objective) {
        lines.push(line(role, false));
    }
    for cycle in &objective.cycles {
        let mut text = format!("Cycle {}: {}", cycle.n, cycle.phase.label());
        if let Some(proposal) = &cycle.proposal {
            text.push_str(&format!(" — {} ({})", proposal.title, proposal.kind));
        }
        lines.push(line(text, false));
        if let Some(attempt) = cycle.attempt() {
            let failures = attempt.failures();
            lines.push(line(
                format!(
                    "  attempt {}: {}",
                    attempt.n,
                    if attempt.checks.is_empty() {
                        "implementing".to_string()
                    } else if failures.is_empty() {
                        "every check and criterion passed".to_string()
                    } else {
                        format!("{} failed: {}", failures.len(), failures.join("; "))
                    }
                ),
                false,
            ));
        }
        if let Some(review) = &cycle.review {
            lines.push(line(
                format!("  review: {}", review.verdict.replace('_', " ")),
                false,
            ));
        }
        if let Some(pr) = &cycle.pull_request {
            lines.push(line(
                format!("  pull request #{}: {}", pr.number, pr.url),
                false,
            ));
        }
        if cycle.adopted {
            lines.push(line("  adopted: Agentique runs the result".into(), false));
        }
        if let Some(blocker) = &cycle.blocker {
            lines.push(line(format!("  stopped: {blocker}"), false));
        }
    }
    if let Some(note) = &objective.note {
        lines.push(line(note.clone(), false));
    }
    div()
        .flex()
        .flex_col()
        .gap(r(2.0))
        .p(r(10.0))
        .rounded(r(crate::tokens::radius::CARD))
        .bg(theme.raised)
        .border_1()
        .border_color(theme.separator)
        .children(lines)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_orchestrator::record::{Cost, Store};
    use agq_providers::{ModelRef, Provider};

    /// C-54: the summary names each role's model, its spend (API-equivalent
    /// on the Claude subscription), why a fallback was taken, and the
    /// models within a role.
    #[test]
    fn each_roles_model_and_spend_are_summarised() {
        let dir = std::env::temp_dir().join(format!("agq-role-lines-{}", std::process::id()));
        let store = Store::new(&dir);
        let mut objective = store
            .create(
                "Fix it",
                std::path::Path::new("C:/agentique"),
                "main",
                Budgets::default(),
                Permissions::default(),
            )
            .unwrap();
        let opus = ModelRef::new(Provider::Anthropic, "claude-opus-5-5");
        let pro = ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro");
        objective.models = vec![
            RoleModel {
                role: "lead".into(),
                model: opus.clone(),
                effort: Some("high".into()),
                access: Access::Subscription,
                configured: opus.clone(),
                fallback: None,
                credential: "the Claude subscription token (CLAUDE_CODE_OAUTH_TOKEN)".into(),
                billed: "your Claude plan".into(),
            },
            RoleModel {
                role: "escalation".into(),
                model: pro.clone(),
                effort: Some("max".into()),
                access: Access::Key,
                configured: opus.clone(),
                fallback: Some("needs an Anthropic API key".into()),
                credential: "DEEPSEEK_API_KEY".into(),
                billed: "DeepSeek".into(),
            },
        ];
        let cost = |usd: f64| Cost {
            usd,
            tokens: 1000,
            unknown: false,
        };
        objective.spent.add("lead", &opus, cost(0.40));
        objective.spent.add(
            "lead",
            &ModelRef::new(Provider::Anthropic, "claude-haiku-4-5"),
            cost(0.02),
        );
        let _ = std::fs::remove_dir_all(&dir);
        let lines = role_lines(&objective);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(
            lines[0].starts_with("lead: claude-opus-5-5 · high · Claude subscription")
                && lines[0].contains("$0.42 API-equivalent, 2000 tokens")
                && lines[0].contains(
                    "by model: anthropic/claude-haiku-4-5 $0.02, anthropic/claude-opus-5-5 $0.40"
                ),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("escalation: deepseek-v4-pro · max · DeepSeek (instead of anthropic/claude-opus-5-5: needs an Anthropic API key)"),
            "{}",
            lines[1]
        );
    }
}
