//! The objective's start form (C-54, as the Operator amended it; ROADMAP
//! §4.16): one form for the Conversation and the Objectives panel. The
//! Operator writes the intent; it is read (a typed decision: Jev, escalating
//! when unsure) for what the objective does, which the form shows with who
//! read it: whether it explores first, how many improvements, and whether
//! reviewed changes are merged, and then built, tried and adopted. The
//! Operator may change whether it explores, merges or adopts. There is no
//! spend or time limit to set. It is drawn in the Conversation when the
//! composer's message or the Assistant's proposal opened it there,
//! otherwise in the Objectives panel; nothing starts until the Operator
//! presses Start, and starting is the Operator's own (its controls are
//! `objective-…`).

use crate::{
    objectives::StartRequest,
    studio::Studio,
    ui::{self, ActiveTheme, Button, Switch, TextArea, r, theme},
    workspace::StudioExt,
};
use agq_orchestrator::decide::Shape;
use gpui::{
    AppContext, ClickEvent, Context, Entity, IntoElement, ParentElement, Render, Styled, Task,
    Window, div, prelude::FluentBuilder,
};
use gpui_base::input::TextareaState;
use std::time::Duration;

/// How long the intent rests unchanged before it is read.
const REST: Duration = Duration::from_millis(700);

pub struct ObjectiveForm {
    studio: Entity<Studio>,
    intent: Entity<TextareaState>,
    /// What it does: as read from the intent, then as the Operator set it.
    shape: Shape,
    /// The intent whose reading the switches show.
    applied: Option<String>,
    /// The intent as last drawn, and the wait before it is read.
    seen: String,
    waiting: Option<Task<()>>,
    /// The Assistant proposed what the form shows.
    by_assistant: bool,
    /// The Studio's proposals taken so far (`StartForm::given`).
    given: u64,
}

impl ObjectiveForm {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        ObjectiveForm {
            intent: cx.new(|cx| {
                let mut state = TextareaState::new(window, cx)
                    .placeholder("What should Agentique improve in itself?");
                state.set_auto_grow(3, 10, cx);
                state
            }),
            shape: Shape::DEFAULT,
            applied: None,
            seen: String::new(),
            waiting: None,
            by_assistant: false,
            given: 0,
            studio,
        }
    }

    fn intent(&self, cx: &gpui::App) -> String {
        self.intent.read(cx).value().trim().to_string()
    }

    /// Takes the proposal the Studio gave since it was last drawn (the
    /// composer's message, or the Assistant's proposal), and reads it at
    /// once.
    fn follow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let form = &self.studio.read(cx).objectives.form;
        if form.given == self.given {
            return;
        }
        self.given = form.given;
        let Some(proposal) = form.proposal.clone() else {
            return;
        };
        self.intent.update(cx, |state, cx| {
            state.set_value(proposal.intent.clone(), window, cx)
        });
        self.by_assistant = proposal.by_assistant;
        self.seen = proposal.intent.clone();
        self.waiting = None;
        if !proposal.intent.is_empty() {
            self.studio
                .act(cx, |studio| studio.read_intent(&proposal.intent));
        }
    }

    /// Reads the intent once it has rested unchanged (the Operator stopped
    /// typing); takes what it was read as when that arrives.
    fn read_when_rested(&mut self, cx: &mut Context<Self>) {
        let intent = self.intent(cx);
        if intent != self.seen {
            self.seen = intent.clone();
            self.waiting = (!intent.is_empty()).then(|| {
                let rested = intent.clone();
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(REST).await;
                    let _ = this.update(cx, |form, cx| {
                        if form.intent(cx) == rested {
                            form.studio.act(cx, |studio| studio.read_intent(&rested));
                        }
                        form.waiting = None;
                    });
                })
            });
        }
        let read = self
            .studio
            .read(cx)
            .objectives
            .form
            .reading
            .as_ref()
            .filter(|r| r.intent == intent)
            .and_then(|r| r.inferred.as_ref().map(|i| i.shape));
        if let Some(shape) = read
            && self.applied.as_deref() != Some(intent.as_str())
        {
            self.shape = shape;
            self.applied = Some(intent);
        }
    }

    fn start(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let intent = self.intent(cx);
        // The switches show this intent's reading, or nothing starts.
        if self.applied.as_deref() != Some(intent.as_str()) {
            return;
        }
        let inferred = self
            .studio
            .read(cx)
            .objectives
            .form
            .reading
            .as_ref()
            .filter(|r| r.intent == intent)
            .and_then(|r| r.inferred.clone());
        let request = StartRequest::new(&intent, self.shape, inferred);
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
            self.clear(window, cx);
        }
        cx.notify();
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.intent
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.by_assistant = false;
        self.seen.clear();
        self.applied = None;
        self.waiting = None;
        self.shape = Shape::DEFAULT;
    }

    /// Closes the form shown in the Conversation, forgetting the intent it
    /// took from the message: the panel shows the form empty.
    fn cancel(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.clear(window, cx);
        self.studio.act(cx, |studio| studio.close_start_form());
        cx.notify();
    }
}

impl Render for ObjectiveForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.follow(window, cx);
        self.read_when_rested(cx);
        let theme = cx.theme().clone();
        let intent = self.intent(cx);
        let studio = self.studio.read(cx);
        let in_conversation = studio.objectives.form.in_conversation;
        let message = studio.objectives.message.clone();
        let reading = studio
            .objectives
            .form
            .reading
            .as_ref()
            .filter(|r| r.intent == intent);
        let inferred = reading.and_then(|r| r.inferred.clone());
        let read = inferred.is_some();
        // What the reading says, and who read it; or why Start waits.
        let (said, by) = match (&inferred, reading) {
            _ if intent.is_empty() => (
                "Say what to improve. It is read for whether to explore first, how many improvements to make, and whether to merge and adopt reviewed changes."
                    .to_string(),
                None,
            ),
            (Some(inferred), _) => (
                format!("It {}.", inferred.shape.describe()),
                Some(inferred.by()),
            ),
            (None, Some(_)) => ("Reading the intent…".to_string(), None),
            (None, None) => (
                "The intent is read when you stop typing…".to_string(),
                None,
            ),
        };
        let changed = inferred
            .as_ref()
            .is_some_and(|inferred| inferred.shape != self.shape);
        let line = |text: String, color| {
            div()
                .text_size(r(theme::text::XS))
                .line_height(r(15.0))
                .text_color(color)
                .child(text)
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
            move |on: bool, _: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |form, cx| {
                    match which {
                        0 => form.shape.explore = on,
                        1 => form.shape.merge = on,
                        _ => form.shape.adopt = on,
                    }
                    cx.notify();
                })
            }
        };
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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(line(said, theme.text_secondary))
                    .when_some(by, |this, by| this.child(line(by, theme.text_muted)))
                    .when(changed, |this| {
                        this.child(line(
                            format!("As you set it, it {}.", self.shape.describe()),
                            theme.accent.text,
                        ))
                    }),
            )
            .when(read, |this| {
                this.child(switch(
                    Switch::new(
                        "objective-explore",
                        self.shape.explore,
                        "Explore the running application first",
                    )
                    .on_toggle(toggle(0)),
                    "Explore the running application first",
                ))
                .child(switch(
                    Switch::new(
                        "objective-merge",
                        self.shape.merge,
                        "Merge reviewed changes that pass every check",
                    )
                    .on_toggle(toggle(1)),
                    "Merge reviewed changes that pass every check",
                ))
                .child(switch(
                    Switch::new(
                        "objective-adopt",
                        self.shape.merge && self.shape.adopt,
                        "Build, try and restart in the result",
                    )
                    .disabled(!self.shape.merge)
                    .on_toggle(toggle(2)),
                    "Build, try and restart in the result",
                ))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .child(div().flex_1().child(line(
                        "No spend or time limit: it ends after its improvements (each made or not) or when two explorations in a row reproduce nothing new, and you can pause or stop it at any time."
                            .into(),
                        theme.text_muted,
                    )))
                    .when(in_conversation, |this| {
                        this.child(
                            Button::new("objective-cancel", "Cancel")
                                .small()
                                .ghost()
                                .tooltip("Close the form; nothing starts", None)
                                .on_click(cx.listener(Self::cancel)),
                        )
                    })
                    .child(
                        Button::new("objective-start", "Start")
                            .primary()
                            .disabled(!read)
                            .when(in_conversation, |this| this.small())
                            .tooltip(
                                if read {
                                    "Starts the objective on Agentique's own repository; its thread follows in the Conversation"
                                } else {
                                    "Waits until the intent is read"
                                },
                                None,
                            )
                            .on_click(cx.listener(Self::start)),
                    ),
            )
    }
}
