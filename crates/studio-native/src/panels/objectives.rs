//! The Objectives panel (C-53, C-54, ROADMAP §4.16): a dashboard of the
//! objectives the Conversation shows, on the same records and commands.
//! Before one starts it holds the start form (unless the form is open in
//! the Conversation); then the objective's record (phase, budgets and
//! spend per role, each role's model and why a fallback was taken), the
//! tree of its child objectives (who asked whom), Pause, Step, Resume,
//! Stop and Continue (the application's commands), a message to its agents
//! (the same path as a reply in its thread), and its latest steps; its
//! whole thread is in the Conversation.

use super::InspectorColumn;
use crate::{
    commands::CommandId,
    objective_form::ObjectiveForm,
    objectives::state_word,
    studio::Studio,
    ui::{self, ActiveTheme, Button, TextField, Tone, r, theme},
    workspace::StudioExt,
};
use agq_orchestrator::record::{Access, Objective, RoleModel};
use agq_orchestrator::thread::Kind;
use gpui::{
    AppContext, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};
use gpui_base::input::InputState;

/// The steps of the thread the panel lists; the Conversation has them all.
const LATEST: usize = 8;

/// What the Operator types in the panel: a message to the objective's
/// agents. The start form is [`ObjectiveForm`], shared with the
/// Conversation.
pub struct Fields {
    message: Entity<InputState>,
    form: Entity<ObjectiveForm>,
}

impl Fields {
    pub fn new(
        form: Entity<ObjectiveForm>,
        window: &mut Window,
        cx: &mut Context<InspectorColumn>,
    ) -> Fields {
        let message =
            cx.new(|cx| InputState::new(window, cx).placeholder("A message for the agent at work"));
        Fields { message, form }
    }

    pub fn render(
        &mut self,
        studio: &Entity<Studio>,
        _: &mut Window,
        cx: &mut Context<InspectorColumn>,
    ) -> gpui::AnyElement {
        let theme = cx.theme().clone();
        let state = studio.read(cx);
        let objectives = &state.objectives;
        let running = objectives.running();
        let paused = objectives.paused();
        let unfinished = objectives.unfinished();
        let current = objectives.current.clone();
        let children = objectives.children.clone();
        let form_here = !running && !unfinished && !objectives.form.in_conversation;
        let form_there = !running && !unfinished && objectives.form.in_conversation;
        let message = objectives.message.clone().filter(|_| !form_here);
        let latest: Vec<(String, String, String)> = current
            .as_ref()
            .and_then(|o| objectives.threads.get(&o.id))
            .into_iter()
            .flatten()
            .filter(|e| e.kind != Kind::Activity)
            .rev()
            .take(LATEST)
            .map(|e| {
                (
                    e.at.get(11..19).unwrap_or(&e.at).to_string(),
                    crate::conversation_view::thread::author(&e.author),
                    e.text.clone(),
                )
            })
            .collect();
        let command = |id: CommandId| {
            let studio = studio.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
                studio.act(cx, |s| s.execute(id))
            }
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
        if form_here {
            body = body.child(self.form.clone());
        }
        if form_there {
            body = body.child(super::note(
                "The start form is open in the Conversation, beside the message it came from.",
                cx,
            ));
        }
        if let Some(objective) = &current {
            body = body.child(summary(objective, running, &theme));
            if !children.is_empty() {
                body = body
                    .child(super::group("Child objectives", Some(children.len()), cx))
                    .children(
                        children
                            .iter()
                            .map(|child| child_line(studio, child, running, &theme)),
                    );
            }
            body = body.child(
                div().flex().child(
                    Button::new(
                        "show-objective-thread",
                        "Show its thread in the Conversation",
                    )
                    .small()
                    .ghost()
                    .icon(ui::IconName::Conversation)
                    .on_click({
                        let studio = studio.clone();
                        move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |s| {
                                s.conversation.shown = true;
                                s.mark(crate::studio::Dirty::LAYOUT);
                            })
                        }
                    }),
                ),
            );
        }
        if running {
            body = body.child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(if paused {
                        Button::new("objective-resume", "Resume")
                            .small()
                            .primary()
                            .on_click(command(CommandId::ResumeObjective))
                    } else {
                        Button::new("objective-pause", "Pause")
                            .small()
                            .tooltip("The agents hold at their next tool call; the Orchestrator before its next phase", None)
                            .on_click(command(CommandId::PauseObjective))
                    })
                    .child(
                        Button::new("objective-step", "Step")
                            .small()
                            .disabled(!paused)
                            .tooltip("One tool call, or one phase, then hold again", None)
                            .on_click(command(CommandId::StepObjective)),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("objective-stop", "Stop")
                            .small()
                            .danger()
                            .tooltip("Ends the objective and its children: their sessions stop, their records stay", None)
                            .on_click(command(CommandId::StopObjective)),
                    ),
            );
        }
        if let Some(objective) = current.as_ref().filter(|_| running || unfinished) {
            let field = self.message.clone();
            let id = objective.id.clone();
            let send = {
                let studio = studio.clone();
                let field = field.clone();
                move |_: &ClickEvent, window: &mut Window, cx: &mut gpui::App| {
                    let text = field.read(cx).value().to_string();
                    if text.trim().is_empty() {
                        return;
                    }
                    let sent = studio.act(cx, |s| {
                        let sent = s.message_objective(&id, &text);
                        if let Err(problem) = &sent {
                            s.objectives.message = Some(format!("Not sent: {problem}"));
                        }
                        sent
                    });
                    if sent.is_ok() {
                        field.update(cx, |state, cx| state.set_value("", window, cx));
                    }
                }
            };
            body = body.child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(
                        div().flex_1().child(
                            TextField::new(&field)
                                .target("Message to the agents")
                                .control_id("objective-message"),
                        ),
                    )
                    .child(Button::new("objective-send", "Send").small().on_click(send)),
            );
        }
        if unfinished {
            body = body.child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(
                        Button::new("objective-continue", "Continue")
                            .small()
                            .primary()
                            .tooltip("Goes on from the phase it reached", None)
                            .on_click(command(CommandId::ContinueObjective)),
                    )
                    .child(
                        Button::new("objective-stop-idle", "Stop")
                            .small()
                            .danger()
                            .on_click(command(CommandId::StopObjective)),
                    ),
            );
        }
        if !latest.is_empty() {
            body = body.child(super::group("Latest", None, cx)).children(
                latest
                    .into_iter()
                    .enumerate()
                    .map(|(i, (at, author, text))| {
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
                                    .child(at),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .w(r(74.0))
                                    .text_color(theme.text_muted)
                                    .font_weight(theme::MEDIUM)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(author),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(theme.text_secondary)
                                    .child(text),
                            )
                    }),
            );
        } else if current.is_none() && !form_here {
            body = body.child(super::note(
                "Agents propose one improvement, implement it in a worktree, run Agentique's checks on a clean checkout, try it in a test instance, and get an independent review; a change that passes is merged, built, tried and adopted, and Agentique goes on in the new build. Every step shows in the Conversation.",
                cx,
            ));
        }
        body.into_any_element()
    }
}

/// A child objective in the tree: what it does, who asked for it, where it
/// stands and what it spent of its budget.
fn child_line(
    studio: &Entity<Studio>,
    child: &Objective,
    running: bool,
    theme: &ui::Theme,
) -> gpui::AnyElement {
    let asked = child
        .requested_by
        .as_ref()
        .map(|r| format!("asked by the {}", r.role))
        .unwrap_or_else(|| "delegated".into());
    // The Operator's own: refused to agents like every `objective-` control.
    let stop = child.active().then(|| {
        let studio = studio.clone();
        let id = child.id.clone();
        Button::new(
            gpui::SharedString::from(format!("objective-stop-child-{}", child.id)),
            "Stop",
        )
        .small()
        .danger()
        .tooltip(
            "Ends this child objective alone; its parent's lead goes on with that",
            None,
        )
        .on_click(move |_: &ClickEvent, _: &mut Window, cx: &mut gpui::App| {
            let id = id.clone();
            studio.act(cx, move |s| s.stop_child(&id))
        })
    });
    div()
        .pl(r(10.0 * f32::from(child.depth.max(1))))
        .flex()
        .items_start()
        .gap(r(6.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .text_size(r(theme::text::XS))
                .line_height(r(16.0))
                .child(
                    div()
                        .text_color(theme.text_secondary)
                        .font_weight(theme::MEDIUM)
                        .child(format!("↳ {}", child.intent)),
                )
                .child(div().text_color(theme.text_muted).child(format!(
                    "{asked} · {} · {}",
                    state_word(child, running),
                    child.budgets.spent_text(child.spent.usd)
                ))),
        )
        .children(stop)
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
    let state = state_word(objective, running);
    let mut lines = vec![
        line(objective.intent.clone(), true),
        line(
            format!(
                "{state} · {}{} · {} tokens · {:.0} min",
                objective.budgets.spent_text(objective.spent.usd),
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
    use agq_orchestrator::record::{Budgets, Cost, Permissions, Store};
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
