//! The Conversation's cards (§3.2 Conversation): a tool call by kind and
//! status with what it changed as links to the Surface, the Assistant's
//! question with its options, a thinking row, a notice with Retry, and the
//! offer to undo the Assistant's changes. Status words are the shared
//! vocabulary: running, done, failed, and kept by you.
use super::Ctx;
use crate::{
    conversation::{Undo, plural},
    ui::{self, ActiveTheme, Button, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_assistant::{ToolResult, tools};
use agq_language::ElementId;
use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};
use serde_json::Value;
use std::rc::Rc;

/// What a change did, in names ready to show.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub created: Vec<(ElementId, Option<String>, String)>,
    pub changed: Vec<(ElementId, Option<String>, String)>,
    pub deleted: usize,
    pub problems: usize,
    /// Locked elements the Operator kept.
    pub kept: Vec<String>,
    /// Everything the change created is gone (undone or deleted since).
    pub gone: bool,
}

/// A tool call, as the card shows it.
#[derive(Clone, Debug)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub input: Value,
    pub result: Option<ToolResult>,
    pub running: bool,
    pub outcome: Option<Outcome>,
    /// The open question: its options, when this call asks one.
    pub options: Option<Vec<String>>,
}

impl Tool {
    /// What a screen reader says for the card.
    pub fn accessible_name(&self) -> String {
        format!(
            "Tool call: {}, {}",
            title(&self.name, &self.input),
            self.status()
        )
    }

    fn read_only(&self) -> bool {
        matches!(
            self.name.as_str(),
            tools::READ_MODEL | tools::FIND_ELEMENTS | tools::GET_PROBLEMS
        )
    }

    /// Where the call stands, in the shared state words (§3.2 principle 4).
    pub fn status(&self) -> &'static str {
        match &self.result {
            None if self.running => "running",
            None => "not run",
            Some(result)
                if result
                    .change
                    .as_ref()
                    .is_some_and(|c| !c.refused.is_empty()) =>
            {
                "kept unchanged by you"
            }
            Some(result) if result.is_error => "failed",
            Some(_) => "done",
        }
    }
}

/// What a tool call is doing, in a few words.
pub fn title(name: &str, input: &Value) -> String {
    let text = |field: &str| input.get(field).and_then(Value::as_str);
    match name {
        tools::READ_MODEL => match text("element") {
            Some(element) => format!("Read {element}"),
            None => "Read the model".into(),
        },
        tools::FIND_ELEMENTS => match (text("name"), text("kind")) {
            (Some(name), _) => format!("Find “{name}”"),
            (None, Some(kind)) => format!("Find every {kind}"),
            _ => "Find elements".into(),
        },
        tools::GET_PROBLEMS => "Check for problems".into(),
        tools::APPLY_CHANGES => text("description")
            .unwrap_or("Change the model")
            .to_string(),
        tools::ASK_OPERATOR => "Question".into(),
        other => other.to_string(),
    }
}

fn status_icon(tool: &Tool, id: SharedString, cx: &App) -> AnyElement {
    let theme = cx.theme();
    match tool.status() {
        "running" => ui::spinner(id, 14.0, theme.info.text).into_any_element(),
        "not run" => icon(IconName::Circle)
            .size(14.0)
            .color(theme.text_faint)
            .into_any_element(),
        "kept unchanged by you" => icon(IconName::Minus)
            .size(14.0)
            .color(theme.text_muted)
            .into_any_element(),
        "failed" => icon(IconName::Alert)
            .size(14.0)
            .color(theme.warning.text)
            .into_any_element(),
        _ => icon(IconName::CircleCheck)
            .size(14.0)
            .color(theme.success.text)
            .into_any_element(),
    }
}

/// An element name that selects it on the Surface.
pub fn element_link(
    ctx: &Rc<Ctx>,
    id: ElementId,
    name: String,
    qualified: Option<String>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let studio = ctx.studio.clone();
    let tooltip = qualified.map(|q| ui::tooltip::text(q, None));
    div()
        .id(SharedString::from(format!("link-{}-{name}", id.raw())))
        .px(r(5.0))
        .h(r(20.0))
        .flex()
        .items_center()
        .rounded(r(crate::tokens::radius::TAG))
        .bg(theme.accent.soft)
        .text_color(theme.accent.text)
        .font_family(theme::MONO)
        .text_size(r(theme::text::XS))
        .cursor_pointer()
        .hover(|style| style.bg(theme.accent.soft_hover))
        .role(gpui::Role::Link)
        .aria_label(SharedString::from(name.clone()))
        .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.reveal(id)))
        .when_some(tooltip, |this, tooltip| {
            this.tooltip(move |window, cx| tooltip(window, cx))
        })
        .relative()
        .child(name)
        .child(ui::target::target(format!("Link {}", id.raw())))
        .into_any_element()
}

pub fn tool_card(ctx: &Rc<Ctx>, tool: &Tool, cx: &App) -> AnyElement {
    if tool.name == tools::ASK_OPERATOR {
        return question_card(ctx, tool, cx);
    }
    let theme = cx.theme().clone();
    let open = ctx.expanded.contains(&tool.id);
    let read_only = tool.read_only();
    let label = title(&tool.name, &tool.input);
    let id = tool.id.clone();
    let view = ctx.view.clone();
    let status = tool.status();
    let aria = SharedString::from(tool.accessible_name());
    let header = div()
        .id(SharedString::from(format!("tool-{}", tool.id)))
        .flex()
        .items_center()
        .gap(r(8.0))
        .min_h(r(if read_only { 24.0 } else { 26.0 }))
        .cursor_pointer()
        .on_click(move |_: &ClickEvent, _, cx| {
            let id = id.clone();
            view.update(cx, |view, cx| {
                if !view.expanded.remove(&id) {
                    view.expanded.insert(id);
                }
                view.dirty_items = true;
                cx.notify();
            })
        })
        .child(status_icon(
            tool,
            SharedString::from(format!("tool-spinner-{}", tool.id)),
            cx,
        ))
        .when(!read_only, |this| {
            this.child(icon(IconName::Tool).size(13.0).color(theme.text_faint))
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .text_size(r(if read_only {
                    theme::text::SM
                } else {
                    theme::text::BASE
                }))
                .font_weight(if read_only {
                    theme::REGULAR
                } else {
                    theme::MEDIUM
                })
                .text_color(if read_only {
                    theme.text_muted
                } else {
                    theme.text
                })
                .child(label),
        )
        .child(
            icon(if open {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .size(13.0)
            .color(theme.text_faint),
        );
    let details = open.then(|| {
        let input = serde_json::to_string_pretty(&tool.input).unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap(r(6.0))
            .pt(r(6.0))
            .child(detail("Input", input, cx))
            .when_some(tool.result.as_ref(), |this, result| {
                this.child(detail("Result", result.content.clone(), cx))
            })
    });
    let summary = (!read_only).then(|| summary(ctx, tool, cx)).flatten();
    div()
        .id(SharedString::from(format!("tool-card-{}", tool.id)))
        .role(gpui::Role::Group)
        .aria_label(aria)
        .when(!read_only, |this| {
            this.px(r(10.0))
                .py(r(7.0))
                .rounded(r(crate::tokens::radius::CARD))
                .bg(theme.raised)
                .border_1()
                .border_color(if status == "running" {
                    theme.info.border
                } else {
                    theme.border
                })
                .when(status == "running", |this| {
                    this.shadow(vec![gpui::BoxShadow {
                        color: theme.info.solid.opacity(0.12),
                        offset: gpui::point(gpui::px(0.0), gpui::px(0.0)),
                        blur_radius: gpui::px(8.0),
                        spread_radius: gpui::px(0.0),
                        inset: false,
                    }])
                })
        })
        .when(read_only, |this| this.px(r(4.0)))
        .flex()
        .flex_col()
        .child(header)
        .children(summary)
        .children(details)
        .into_any_element()
}

/// What a change did: created and changed elements as links, deletions,
/// problems, the locked elements the Operator kept, or why it was not done.
fn summary(ctx: &Rc<Ctx>, tool: &Tool, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    let result = tool.result.as_ref()?;
    let first_line = || {
        result
            .content
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    };
    let Some(outcome) = &tool.outcome else {
        return result.is_error.then(|| {
            div()
                .pt(r(4.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.warning.text)
                .child(first_line())
                .into_any_element()
        });
    };
    if !outcome.kept.is_empty() {
        return Some(
            div()
                .pt(r(4.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.text_secondary)
                .child(format!("You kept {} unchanged.", outcome.kept.join(", ")))
                .into_any_element(),
        );
    }
    if result.is_error {
        return Some(
            div()
                .pt(r(4.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.warning.text)
                .child(first_line())
                .into_any_element(),
        );
    }
    if outcome.gone {
        return Some(
            div()
                .pt(r(4.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.text_muted)
                .child("Undone or deleted since.")
                .into_any_element(),
        );
    }
    let row = |label: &'static str, items: &[(ElementId, Option<String>, String)]| {
        (!items.is_empty()).then(|| {
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(r(4.0))
                .child(
                    div()
                        .w(r(54.0))
                        .text_size(r(theme::text::XS))
                        .text_color(theme.text_muted)
                        .child(label),
                )
                .children(items.iter().map(|(id, qualified, name)| {
                    element_link(ctx, *id, name.clone(), qualified.clone(), cx)
                }))
        })
    };
    Some(
        div()
            .pt(r(6.0))
            .flex()
            .flex_col()
            .gap(r(4.0))
            .children(row("Created", &outcome.created))
            .children(row("Changed", &outcome.changed))
            .when(outcome.deleted > 0, |this| {
                this.child(
                    div()
                        .text_size(r(theme::text::XS))
                        .text_color(theme.text_muted)
                        .child(format!(
                            "Deleted {}",
                            plural(outcome.deleted, "element", "elements")
                        )),
                )
            })
            .when(outcome.problems > 0, |this| {
                this.child(ui::inline_message(
                    Tone::Warning,
                    format!(
                        "{} at the changed elements",
                        plural(outcome.problems, "problem", "problems")
                    ),
                    cx,
                ))
            })
            .into_any_element(),
    )
}

fn detail(label: &'static str, text: String, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap(r(3.0))
        .child(
            div()
                .text_size(r(theme::text::XS))
                .font_weight(theme::SEMIBOLD)
                .text_color(theme.text_muted)
                .child(label.to_uppercase()),
        )
        .child(
            div()
                .p(r(8.0))
                .rounded(r(crate::tokens::radius::CONTROL))
                .bg(theme.inset)
                .border_1()
                .border_color(theme.separator)
                .font_family(theme::MONO)
                .text_size(r(theme::text::XS))
                .line_height(r(16.0))
                .text_color(theme.text_secondary)
                .max_h(r(260.0))
                .overflow_hidden()
                .child(text),
        )
}

/// A question from the Assistant: its options as buttons while it is open,
/// then the answer.
fn question_card(ctx: &Rc<Ctx>, tool: &Tool, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    let question = tool
        .input
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or("The Assistant has a question")
        .to_string();
    let open = tool.options.is_some();
    let answer = match &tool.result {
        Some(result) if !result.is_error => Some(format!("Your answer: {}", result.content)),
        Some(_) => Some("Not answered: the Assistant was stopped.".to_string()),
        None if tool.running && !open => Some("Waiting…".to_string()),
        None if !open => Some("Not answered.".to_string()),
        None => None,
    };
    div()
        .id(SharedString::from(format!("question-{}", tool.id)))
        .role(gpui::Role::Group)
        .aria_label(SharedString::from(format!(
            "The Assistant asks: {question}"
        )))
        .p(r(12.0))
        .rounded(r(crate::tokens::radius::CARD + 2.0))
        .bg(theme.raised)
        .border_1()
        .border_color(if open {
            theme.accent.solid
        } else {
            theme.border
        })
        .when(open, |this| {
            this.shadow(vec![gpui::BoxShadow {
                color: theme.accent.solid.opacity(0.18),
                offset: gpui::point(gpui::px(0.0), gpui::px(0.0)),
                blur_radius: gpui::px(12.0),
                spread_radius: gpui::px(0.0),
                inset: false,
            }])
        })
        .flex()
        .flex_col()
        .gap(r(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(r(6.0))
                .text_size(r(theme::text::XS))
                .font_weight(theme::SEMIBOLD)
                .text_color(if open {
                    theme.accent.text
                } else {
                    theme.text_muted
                })
                .child(icon(IconName::Question).size(13.0).color(if open {
                    theme.accent.text
                } else {
                    theme.text_muted
                }))
                .child("THE ASSISTANT ASKS"),
        )
        .child(
            div()
                .text_size(r(theme::text::PROSE))
                .line_height(r(21.0))
                .font_weight(theme::MEDIUM)
                .child(question),
        )
        .when_some(tool.options.clone(), |this, options| {
            this.child(div().flex().flex_wrap().gap(r(6.0)).children(
                options.into_iter().enumerate().map(|(index, option)| {
                    let studio = ctx.studio.clone();
                    let chosen = option.clone();
                    div()
                        .relative()
                        .child(ui::target::target(format!("Option {}", index + 1)))
                        .child(Button::new(("option", index), option).on_click(
                            move |_: &ClickEvent, _, cx| {
                                let chosen = chosen.clone();
                                studio.act(cx, |studio| {
                                    studio.answer_question(&chosen);
                                })
                            },
                        ))
                }),
            ))
            .child(
                div()
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_muted)
                    .child("Or type your own answer below."),
            )
        })
        .when_some(answer, |this, answer| {
            this.child(
                div()
                    .text_size(r(theme::text::SM))
                    .text_color(theme.text_secondary)
                    .child(answer),
            )
        })
        .into_any_element()
}

/// The model's thinking for one step, collapsed to its first line (R-31);
/// a click opens it. Nothing is shown for thinking without readable text.
pub fn thinking_row(ctx: &Rc<Ctx>, key: String, text: &str, live: bool, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    // Reasoning may quote the model's text; the Operator never sees SysML (C-4).
    let text = agq_assistant::sysml_text::without_sysml(text);
    let text = text.trim().to_string();
    let Some(first) = text.lines().map(str::trim).find(|line| !line.is_empty()) else {
        return div().into_any_element();
    };
    const SUMMARY: usize = 90;
    let mut summary: String = first.chars().take(SUMMARY).collect();
    if first.chars().count() > SUMMARY || text.lines().nth(1).is_some() {
        summary.push('…');
    }
    let open = ctx.expanded.contains(&key);
    let view = ctx.view.clone();
    let toggle = key.clone();
    div()
        .flex()
        .flex_col()
        .gap(r(4.0))
        .child(
            div()
                .id(SharedString::from(format!("thinking-{key}")))
                .flex()
                .items_center()
                .gap(r(6.0))
                .text_size(r(theme::text::SM))
                .text_color(theme.text_muted)
                .cursor_pointer()
                .hover(|style| style.text_color(theme.text_secondary))
                .on_click(move |_: &ClickEvent, _, cx| {
                    let toggle = toggle.clone();
                    view.update(cx, |view, cx| {
                        if !view.expanded.remove(&toggle) {
                            view.expanded.insert(toggle);
                        }
                        view.dirty_items = true;
                        cx.notify();
                    })
                })
                .child(if live {
                    ui::spinner(
                        SharedString::from(format!("thinking-spin-{key}")),
                        13.0,
                        theme.info.text,
                    )
                    .into_any_element()
                } else {
                    icon(IconName::Thinking)
                        .size(13.0)
                        .color(theme.text_faint)
                        .into_any_element()
                })
                .child(div().font_weight(theme::MEDIUM).child(if live {
                    "Thinking"
                } else {
                    "Thought"
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(summary),
                )
                .child(
                    icon(if open {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .size(12.0)
                    .color(theme.text_faint),
                ),
        )
        .when(open, |this| {
            this.child(
                div()
                    .ml(r(6.0))
                    .pl(r(12.0))
                    .border_l_2()
                    .border_color(theme.separator)
                    .text_size(r(theme::text::SM))
                    .line_height(r(19.0))
                    .text_color(theme.text_muted)
                    .child(text),
            )
        })
        .into_any_element()
}

/// A notice (an error, a stop, a limit), with Retry after a failed turn.
pub fn notice(ctx: &Rc<Ctx>, text: String, retry: bool) -> AnyElement {
    let studio = ctx.studio.clone();
    let mut banner = ui::Banner::new(Tone::Warning, text);
    if retry {
        banner = banner.action(
            Button::new("retry", "Retry")
                .small()
                .icon(IconName::Retry)
                .tooltip("Send the last message again", None)
                .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.retry())),
        );
    }
    banner.into_any_element()
}

/// After a turn that changed the model: undo its changes (R-12).
pub fn undo_card(ctx: &Rc<Ctx>, undo: Undo, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let (text, button, hint) = match undo {
        Undo::Assistant(count) => (
            format!(
                "The Assistant made {} in this turn.",
                plural(count, "change", "changes")
            ),
            "Undo the Assistant's changes",
            "Each can be redone with Ctrl+Y",
        ),
        Undo::All(count) => (
            format!(
                "{} since the Assistant started, not all of them the Assistant's.",
                plural(count, "change", "changes")
            ),
            "Undo all changes since the Assistant started",
            "Also undoes your own changes made since then; each can be redone with Ctrl+Y",
        ),
    };
    let studio = ctx.studio.clone();
    div()
        .p(r(10.0))
        .rounded(r(crate::tokens::radius::CARD))
        .bg(theme.info.soft.opacity(0.5))
        .border_1()
        .border_color(theme.info.border.opacity(0.6))
        .flex()
        .flex_col()
        .gap(r(8.0))
        .child(
            div()
                .flex()
                .items_start()
                .gap(r(8.0))
                .child(
                    div()
                        .pt(r(2.0))
                        .child(icon(IconName::Assistant).size(14.0).color(theme.info.text)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(r(theme::text::SM))
                        .text_color(theme.text_secondary)
                        .child(text),
                ),
        )
        .child(
            div().pl(r(22.0)).flex().child(
                Button::new("undo-turn", button)
                    .small()
                    .icon(IconName::Undo)
                    .tooltip(hint, None)
                    .on_click(move |_: &ClickEvent, _, cx| {
                        studio.act(cx, |studio| studio.undo_assistant_changes())
                    }),
            ),
        )
        .into_any_element()
}
