//! The objective's start form (C-54, ROADMAP §3.6, §4.16): one form for
//! the Conversation and the Objectives panel. It shows, before Start, the
//! intent, whether the objective explores, its budgets (spend, improvements,
//! attempts, hours and exploration steps), its permissions (merge, adopt),
//! and each role's model with why a fallback was taken, the credential and
//! who pays. It is drawn in the Conversation when the composer's message or
//! the Assistant's proposal opened it there, otherwise in the Objectives
//! panel; nothing starts until the Operator presses Start, and starting is
//! the Operator's own (its controls are `objective-…`).

use crate::{
    objectives::{Proposal, StartRequest},
    studio::Studio,
    ui::{self, ActiveTheme, Button, Switch, TextArea, TextField, r, theme},
    workspace::StudioExt,
};
use agq_orchestrator::record::{Budgets, Permissions, RoleModel};
use gpui::{
    AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement, Render, Styled, Window,
    div, prelude::FluentBuilder,
};
use gpui_base::input::{InputState, TextareaState};

pub struct ObjectiveForm {
    studio: Entity<Studio>,
    intent: Entity<TextareaState>,
    usd: Entity<InputState>,
    cycles: Entity<InputState>,
    attempts: Entity<InputState>,
    hours: Entity<InputState>,
    steps: Entity<InputState>,
    explore: bool,
    merge: bool,
    adopt: bool,
    /// The Assistant proposed what the form shows.
    by_assistant: bool,
    /// The Studio's proposals taken so far (`StartForm::given`).
    given: u64,
}

impl ObjectiveForm {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = |value: &str, window: &mut Window, cx: &mut Context<Self>| {
            let value = value.to_string();
            cx.new(|cx| {
                let mut state = InputState::new(window, cx).placeholder(value.clone());
                state.set_value(value, window, cx);
                state
            })
        };
        let defaults = Proposal::new("", false);
        let budgets = &defaults.budgets;
        ObjectiveForm {
            intent: cx.new(|cx| {
                let mut state = TextareaState::new(window, cx)
                    .placeholder("What should Agentique improve in itself?");
                state.set_auto_grow(3, 10, cx);
                state
            }),
            usd: field(&number(budgets.usd), window, cx),
            cycles: field(&budgets.cycles.to_string(), window, cx),
            attempts: field(&budgets.attempts.to_string(), window, cx),
            hours: field(&number(budgets.hours), window, cx),
            steps: field(&budgets.steps.to_string(), window, cx),
            explore: false,
            merge: true,
            adopt: true,
            by_assistant: false,
            given: 0,
            studio,
        }
    }

    /// Takes the proposal the Studio gave since it was last drawn (the
    /// composer's message, or the Assistant's proposal).
    fn follow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let form = &self.studio.read(cx).objectives.form;
        if form.given == self.given {
            return;
        }
        self.given = form.given;
        let Some(proposal) = form.proposal.clone() else {
            return;
        };
        let set = |field: &Entity<InputState>,
                   value: String,
                   window: &mut Window,
                   cx: &mut Context<Self>| {
            field.update(cx, |state, cx| state.set_value(value, window, cx));
        };
        self.intent.update(cx, |state, cx| {
            state.set_value(proposal.intent.clone(), window, cx)
        });
        let budgets = &proposal.budgets;
        set(&self.usd, number(budgets.usd), window, cx);
        set(&self.cycles, budgets.cycles.to_string(), window, cx);
        set(&self.attempts, budgets.attempts.to_string(), window, cx);
        set(&self.hours, number(budgets.hours), window, cx);
        set(&self.steps, budgets.steps.to_string(), window, cx);
        self.explore = proposal.explore;
        self.by_assistant = proposal.by_assistant;
    }

    /// What the form asks for, as typed.
    fn request(&self, cx: &gpui::App) -> StartRequest {
        let read = |field: &Entity<InputState>| field.read(cx).value().trim().to_string();
        // A number that does not read is out of every range: the budgets'
        // check names it.
        let decimal = |field: &Entity<InputState>| read(field).parse::<f64>().unwrap_or(-1.0);
        let whole = |field: &Entity<InputState>| read(field).parse::<u32>().unwrap_or(0);
        StartRequest {
            intent: self.intent.read(cx).value().trim().to_string(),
            explore: self.explore,
            budgets: Budgets {
                usd: decimal(&self.usd),
                cycles: whole(&self.cycles),
                attempts: whole(&self.attempts),
                hours: decimal(&self.hours),
                steps: if self.explore {
                    whole(&self.steps)
                } else {
                    Budgets::default().steps
                },
                ..Budgets::default()
            },
            permissions: Permissions {
                push: self.merge,
                merge: self.merge,
                adopt: self.merge && self.adopt,
                ..Permissions::default()
            },
        }
    }

    fn start(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let request = self.request(cx);
        let intent = request.intent.clone();
        let started = self.studio.act(cx, |studio| {
            let result = studio.start_objective(request);
            match &result {
                Err(problem) => {
                    studio.objectives.message = Some(format!("Not started: {problem}"));
                }
                // The message it came from leaves the composer.
                Ok(()) if studio.conversation.input.trim() == intent => {
                    studio.conversation.input.clear();
                    studio.conversation.input_set += 1;
                }
                Ok(()) => {}
            }
            result
        });
        if started.is_ok() {
            self.intent
                .update(cx, |state, cx| state.set_value("", window, cx));
            self.by_assistant = false;
        }
        cx.notify();
    }

    /// Explore on: three improvements unless changed (it explores, fixes
    /// and explores again); off: one.
    fn set_explore(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let cycles = self.cycles.read(cx).value().trim().to_string();
        let (from, to) = if on { ("1", "3") } else { ("3", "1") };
        if cycles == from {
            self.cycles
                .update(cx, |state, cx| state.set_value(to, window, cx));
        }
        self.explore = on;
        cx.notify();
    }
}

/// A decimal as the form shows it: `5`, `0.5`.
fn number(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl Render for ObjectiveForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.follow(window, cx);
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let in_conversation = studio.objectives.form.in_conversation;
        let message = studio.objectives.message.clone();
        let models = studio.agent_models_each();
        let explore = self.explore;
        let label = |t: &'static str| {
            div()
                .text_size(r(theme::text::XS))
                .text_color(theme.text_muted)
                .child(t)
        };
        let field = |title: &'static str, state: &Entity<InputState>, id: &'static str| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(r(3.0))
                .child(label(title))
                .child(TextField::new(state).target(title).control_id(id))
        };
        let switch = |switch: Switch, text: &'static str| {
            div().flex().items_center().gap(r(8.0)).child(switch).child(
                div()
                    .text_size(r(theme::text::SM))
                    .text_color(theme.text_secondary)
                    .child(text),
            )
        };
        let entity = cx.entity();
        let toggle = |which: u8| {
            let entity = entity.clone();
            move |on: bool, window: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |form, cx| match which {
                    0 => form.set_explore(on, window, cx),
                    1 => {
                        form.merge = on;
                        cx.notify();
                    }
                    _ => {
                        form.adopt = on;
                        cx.notify();
                    }
                })
            }
        };
        let studio_entity = self.studio.clone();
        div()
            .flex()
            .flex_col()
            .gap(r(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .child(crate::panels::group("New objective", None, cx))
                    .when(self.by_assistant, |this| {
                        this.child(
                            div()
                                .pt(r(8.0))
                                .text_size(r(theme::text::XS))
                                .text_color(theme.accent.text)
                                .child("Proposed by the Assistant"),
                        )
                    }),
            )
            .when_some(message, |this, message| {
                this.child(ui::inline_message(ui::Tone::Neutral, message, cx))
            })
            .child(
                TextArea::new(&self.intent)
                    .target("What should Agentique improve in itself?")
                    .control_id("objective-intent"),
            )
            .child(switch(
                Switch::new(
                    "objective-explore",
                    explore,
                    "Explore the running application first",
                )
                .on_toggle(toggle(0)),
                "Explore the running application first",
            ))
            .child(
                div()
                    .flex()
                    .gap(r(8.0))
                    .child(field("Spend budget (USD)", &self.usd, "objective-usd"))
                    .child(field("Improvements", &self.cycles, "objective-cycles")),
            )
            .child(
                div()
                    .flex()
                    .gap(r(8.0))
                    .child(field(
                        "Attempts per improvement",
                        &self.attempts,
                        "objective-attempts",
                    ))
                    .child(field("Hours at most", &self.hours, "objective-hours"))
                    .when(explore, |this| {
                        this.child(field("Exploration steps", &self.steps, "objective-steps"))
                    }),
            )
            .child(switch(
                Switch::new(
                    "objective-merge",
                    self.merge,
                    "Merge reviewed changes that pass every check",
                )
                .on_toggle(toggle(1)),
                "Merge reviewed changes that pass every check",
            ))
            .child(switch(
                Switch::new(
                    "objective-adopt",
                    self.merge && self.adopt,
                    "Build, try and restart in the result",
                )
                .disabled(!self.merge)
                .on_toggle(toggle(2)),
                "Build, try and restart in the result",
            ))
            .child(crate::panels::group("Models", None, cx))
            .children(
                models
                    .into_iter()
                    .map(|(role, model)| route(role, model, explore, &theme)),
            )
            .child(
                div()
                    .flex()
                    .gap(r(6.0))
                    .child(div().flex_1())
                    .when(in_conversation, |this| {
                        this.child(
                            Button::new("objective-cancel", "Cancel")
                                .small()
                                .ghost()
                                .tooltip("Close the form; nothing starts", None)
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    studio_entity.act(cx, |studio| studio.close_start_form())
                                }),
                        )
                    })
                    .child(
                        Button::new("objective-start", "Start")
                            .primary()
                            .when(in_conversation, |this| this.small())
                            .tooltip(
                                "Starts the objective on Agentique's own repository; its thread follows in the Conversation",
                                None,
                            )
                            .on_click(cx.listener(Self::start)),
                    ),
            )
    }
}

/// A role's model before Start (C-54): the model, effort and credential,
/// who pays, and why a fallback was taken; or why the role has none, which
/// keeps the objective from starting when it needs that role.
fn route(
    role: &str,
    model: Result<RoleModel, String>,
    explore: bool,
    theme: &ui::Theme,
) -> gpui::AnyElement {
    let (head, detail, warn) = match model {
        Ok(model) => (
            format!("{role} · {}", model.label()),
            match &model.fallback {
                Some(why) => format!(
                    "On its fallback, since {} {why}. {}; {}.",
                    model.configured, model.credential, model.billed
                ),
                None => format!("{}; {}.", model.credential, model.billed),
            },
            model.fallback.is_some(),
        ),
        // Not needed by an objective that does not explore: recorded,
        // never a reason not to start.
        Err(problem) if !agq_orchestrator::models::needed(role, explore) => (
            format!("{role} · no model now"),
            format!("{problem}. Only an objective that explores needs it."),
            false,
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

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn numbers_read_as_typed() {
        assert_eq!(number(5.0), "5");
        assert_eq!(number(0.5), "0.5");
        assert_eq!(number(2.25), "2.25");
        assert_eq!(number(6.0), "6");
    }
}
