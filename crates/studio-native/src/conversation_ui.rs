//! The Conversation column: messages rendered as Markdown with element
//! links, the Assistant's tool calls as compact live cards, its questions
//! as answerable prompts, and the message input with Send and Stop.
use crate::{
    app::StudioApp,
    conversation::{ConversationPanel, Live, Undo, WaitingFor, plural},
    markdown::{self, Block},
    targets::{Target, record},
    theme::{self, Theme},
};
use agq_assistant::{Entry, ToolResult, tools};
use agq_language::{ElementId, ElementKind, Tree};
use eframe::egui::{self, Key, Modifiers, RichText, Stroke, Vec2};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Automation names for the options of a question, in order.
pub const OPTIONS: [&str; 6] = [
    "Option 1", "Option 2", "Option 3", "Option 4", "Option 5", "Option 6",
];

const INPUT: &str = "conversation-input";

/// Parsed messages and their heights: a message is parsed once per text
/// and theme, again only when an element it names appears or goes, and laid
/// out again only when that or the width changes.
#[derive(Default)]
pub struct ViewCache {
    messages: HashMap<u64, Message>,
    used: HashSet<u64>,
    /// Element names by simple name for links, for one revision of the model
    /// (`None`: the name is not unique).
    names: HashMap<String, Option<ElementId>>,
    names_revision: Option<u64>,
}

struct Message {
    blocks: Vec<Block>,
    /// Its inline code spans and the elements they named when parsed.
    links: Vec<(String, Option<ElementId>)>,
    /// The model revision the links were checked at.
    checked: u64,
    /// Height at a width, to skip messages outside the view.
    height: Option<(f32, f32)>,
}

/// What the Operator asked for while the conversation was drawn.
enum Action {
    Reveal(ElementId),
    Edit,
    Retry,
    Undo,
    Answer(String),
    Send,
    Stop,
    InsertSelection,
    CancelEdit,
    NewConversation,
}

impl StudioApp {
    /// The Conversation column, on the right of the Studio.
    pub fn conversation_column(&mut self, ctx: &egui::Context) {
        let theme = self.theme;
        let mut actions = Vec::new();
        egui::SidePanel::right("conversation")
            .default_width(360.0)
            .min_width(260.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme.surface)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme.overline("Conversation"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let new = ui
                            .add(egui::Button::new(RichText::new("New").small()).frame(false))
                            .on_hover_text("Start a new conversation; the model is unaffected");
                        if new.clicked() {
                            actions.push(Action::NewConversation);
                        }
                        let usage = self.conversation.usage;
                        ui.label(
                            RichText::new(&self.conversation.model_name)
                                .font(theme::regular(theme::CAPTION))
                                .color(theme.muted),
                        )
                        .on_hover_text(format!(
                            "Tokens this session: {} input, {} from cache, {} output",
                            usage.input_tokens + usage.cache_creation_input_tokens,
                            usage.cache_read_input_tokens,
                            usage.output_tokens
                        ));
                    });
                });
                if self.conversation.key_missing {
                    banner(
                        ui,
                        theme,
                        "No Claude API key is set. Set ANTHROPIC_API_KEY and restart Agentique to work with the Assistant. Everything else works as usual.",
                    );
                }
                for error in [&self.conversation.read_error, &self.conversation.save_error]
                    .into_iter()
                    .flatten()
                {
                    banner(ui, theme, error);
                }
                ui.add_space(theme::SPACE_S);
                egui::TopBottomPanel::bottom("conversation-input")
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin {
                        top: 8,
                        ..Default::default()
                    }))
                    .show_separator_line(false)
                    .show_inside(ui, |ui| self.message_input(ui, &mut actions));
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        let state = self.project.as_ref().map(|p| p.state());
                        // What the running turn waits for, if the Operator is to act.
                        let assistant_asks = matches!(
                            &self.dialog,
                            Some(crate::edit::Dialog::Confirm { change, .. })
                                if change.actor == agq_system_state::Actor::Assistant
                        );
                        let waiting = match self.conversation.waiting.as_ref().map(|w| &w.kind) {
                            Some(WaitingFor::Confirmation) if assistant_asks => {
                                Some("Waiting for your answer about the locked element")
                            }
                            Some(WaitingFor::Dialog(_)) => {
                                Some("Waiting until you close the open dialog")
                            }
                            _ => None,
                        };
                        messages(ui, &mut self.conversation, state, waiting, theme, &mut actions);
                    });
            });
        for action in actions {
            match action {
                Action::Reveal(id) => self.reveal(id),
                Action::Edit => self.edit_last_message(),
                Action::Retry => self.retry(),
                Action::Undo => self.undo_assistant_changes(),
                Action::Answer(answer) => {
                    self.answer_question(&answer);
                }
                Action::Send => self.send_message(),
                Action::Stop => self.stop_assistant(),
                Action::InsertSelection => self.insert_selection(),
                Action::CancelEdit => self.cancel_edit(),
                Action::NewConversation => self.new_conversation(),
            }
        }
    }

    fn message_input(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        let theme = self.theme;
        let panel = &mut self.conversation;
        let question = matches!(
            panel.waiting,
            Some(crate::conversation::Waiting {
                kind: WaitingFor::Question { .. },
                ..
            })
        );
        if panel.editing.is_some() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Editing your last message; it and what followed are replaced")
                        .font(theme::regular(theme::CAPTION))
                        .color(theme.muted),
                );
                if ui.small_button("Cancel").clicked() {
                    actions.push(Action::CancelEdit);
                }
            });
        } else if question {
            ui.label(
                RichText::new(
                    "Type an answer to the Assistant's question, or pick an option above",
                )
                .font(theme::regular(theme::CAPTION))
                .color(theme.accent),
            );
        }
        let id = egui::Id::new(INPUT);
        let focused = ui.memory(|memory| memory.has_focus(id));
        if focused {
            // Enter sends; Shift+Enter is a new line. Ctrl+I inserts the selection.
            let enter = ui.input_mut(|input| {
                !input.modifiers.shift && input.consume_key(Modifiers::NONE, Key::Enter)
            });
            if enter {
                actions.push(Action::Send);
            }
            if ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::I)) {
                actions.push(Action::InsertSelection);
            }
        }
        let hint = if question {
            "Your answer"
        } else {
            "Ask the Assistant (Enter to send, Shift+Enter for a new line)"
        };
        let response = ui.add(
            egui::TextEdit::multiline(&mut panel.input)
                .id(id)
                .hint_text(hint)
                .desired_rows(3)
                .margin(theme::INPUT_MARGIN)
                .desired_width(f32::INFINITY),
        );
        record(ui.ctx(), Target::Field("Message"), response.rect);
        if std::mem::take(&mut panel.focus_input) {
            response.request_focus();
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                let end = egui::text::CCursor::new(panel.input.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), id);
            }
        }
        ui.add_space(theme::SPACE_S);
        let running = panel.running();
        let can_send =
            !panel.input.trim().is_empty() && (question || (!running && !panel.key_missing));
        ui.horizontal(|ui| {
            let insert = ui
                .add(egui::Button::new(RichText::new("Insert selection").small()).frame(false))
                .on_hover_text("Put the selected element's name into the message (Ctrl+I)");
            record(ui.ctx(), Target::Button("Insert selection"), insert.rect);
            if insert.clicked() {
                actions.push(Action::InsertSelection);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if running && !question {
                    let stop = ui
                        .add(
                            egui::Button::new(RichText::new("■  Stop").color(theme.on_accent))
                                .fill(theme.error),
                        )
                        .on_hover_text(
                            "Stop the Assistant now; its changes so far stay and can be undone",
                        );
                    record(ui.ctx(), Target::Button("Stop"), stop.rect);
                    if stop.clicked() {
                        actions.push(Action::Stop);
                    }
                } else {
                    let label = if question { "Answer" } else { "Send" };
                    let send = ui.add_enabled(can_send, crate::edit::primary_button(theme, label));
                    let send = if panel.key_missing && !question {
                        send.on_disabled_hover_text("Set ANTHROPIC_API_KEY and restart Agentique")
                    } else {
                        send
                    };
                    record(ui.ctx(), Target::Button("Send"), send.rect);
                    if send.clicked() {
                        actions.push(Action::Send);
                    }
                }
                if running && question {
                    let stop = ui.add(egui::Button::new("Stop"));
                    record(ui.ctx(), Target::Button("Stop"), stop.rect);
                    if stop.clicked() {
                        actions.push(Action::Stop);
                    }
                }
            });
        });
    }
}

fn banner(ui: &mut egui::Ui, theme: Theme, text: &str) {
    ui.add_space(theme::SPACE_S);
    egui::Frame::new()
        .fill(theme.elevated)
        .stroke(Stroke::new(theme::HAIRLINE, theme.amber))
        .corner_radius(theme::RADIUS)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(text).color(theme.text_secondary));
        });
}

fn messages(
    ui: &mut egui::Ui,
    panel: &mut ConversationPanel,
    state: Option<&agq_system_state::SystemState>,
    waiting: Option<&str>,
    theme: Theme,
    actions: &mut Vec<Action>,
) {
    let running = panel.running();
    let undoable = state.and_then(|state| panel.undoable(state));
    let failed = panel.last_turn.as_ref().is_some_and(|turn| turn.failed);
    let tree = state.map(|state| state.tree());
    let revision = state.map_or(0, |state| state.revision());
    let view = &mut panel.view;
    if view.names_revision != Some(revision) {
        // Names for element links: qualified names, and simple names that
        // are unique (`None` otherwise). Built once per model revision.
        view.names_revision = Some(revision);
        view.names.clear();
        if let Some(tree) = tree {
            for id in tree.walk() {
                if let Some(name) = tree.effective_name(id) {
                    view.names
                        .entry(name.to_string())
                        .and_modify(|found| *found = None)
                        .or_insert(Some(id));
                    view.names.insert(tree.qualified_name(id), Some(id));
                }
            }
        }
    }
    view.used.clear();
    let entries = &panel.conversation.entries;
    let last_operator = entries
        .iter()
        .rposition(|entry| matches!(entry, Entry::Operator { .. }));
    let waiting_question = match &panel.waiting {
        Some(waiting) => match &waiting.kind {
            WaitingFor::Question { question, options } => Some((
                waiting.call.id.as_str(),
                question.as_str(),
                options.as_slice(),
            )),
            _ => None,
        },
        None => None,
    };
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        // Pressing on a message is a click (a link, a button), never a scroll.
        .scroll_source(egui::scroll_area::ScrollSource {
            drag: false,
            ..Default::default()
        })
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Room for the floating scroll bar, so it never covers a card.
            ui.set_max_width(ui.available_width() - 10.0);
            ui.spacing_mut().item_spacing.y = theme::SPACE;
            if entries.is_empty() && panel.live.is_empty() {
                ui.add_space(theme::SPACE_XL);
                ui.label(
                    RichText::new(
                        "Tell the Assistant what to build or change. You see every change on the Surface as it happens, and every change can be undone.",
                    )
                    .color(theme.muted),
                );
            }
            let ctx = Context {
                tree,
                revision,
                theme,
                running,
                results: &panel.results,
                question: waiting_question,
            };
            for (index, entry) in entries.iter().enumerate() {
                match entry {
                    Entry::Operator { text } => {
                        let editable = Some(index) == last_operator && !running && panel.editing.is_none();
                        operator_message(ui, view, &ctx, index, text, editable, actions);
                    }
                    Entry::Assistant { content } => {
                        for (slot, block) in content.iter().enumerate() {
                            match block["type"].as_str() {
                                Some("text") => {
                                    let text = block["text"].as_str().unwrap_or_default();
                                    let id = egui::Id::new(("message", index, slot));
                                    if let Some(id) = markdown_message(ui, view, &ctx, id, text) {
                                        actions.push(Action::Reveal(id));
                                    }
                                }
                                Some("tool_use") => {
                                    let id = block["id"].as_str().unwrap_or_default();
                                    let name = block["name"].as_str().unwrap_or_default();
                                    tool_card(ui, &ctx, id, name, &block["input"], actions);
                                }
                                _ => {}
                            }
                        }
                    }
                    Entry::ToolResults { .. } => {}
                    Entry::Notice { text } => {
                        let retry = index + 1 == entries.len() && failed && !running;
                        notice(ui, theme, text, retry, actions);
                    }
                }
            }
            for (slot, live) in panel.live.iter().enumerate() {
                match live {
                    Live::Text(text) => {
                        let id = egui::Id::new(("live", slot));
                        if let Some(id) = markdown_message(ui, view, &ctx, id, text) {
                            actions.push(Action::Reveal(id));
                        }
                    }
                    Live::Tool { id, name, input } => {
                        let input = serde_json::from_str(input).unwrap_or(Value::Null);
                        tool_card(ui, &ctx, id, name, &input, actions);
                    }
                }
            }
            if running && waiting_question.is_none() {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(12.0).color(theme.muted));
                    let text = if let Some(waiting) = waiting {
                        waiting
                    } else if panel.thinking {
                        "Thinking…"
                    } else {
                        "Working…"
                    };
                    ui.label(RichText::new(text).color(theme.muted));
                });
            }
            if let Some(undo) = undoable {
                let (text, button, hint) = match undo {
                    Undo::Assistant(count) => (
                        format!("The Assistant made {} in this turn.", plural(count, "change", "changes")),
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
                egui::Frame::new()
                    .fill(theme.elevated)
                    .stroke(Stroke::new(theme::HAIRLINE, theme.border))
                    .corner_radius(theme::RADIUS_L)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(text).color(theme.text_secondary));
                        let undo = ui.button(button).on_hover_text(hint);
                        record(ui.ctx(), Target::Button(button), undo.rect);
                        if undo.clicked() {
                            actions.push(Action::Undo);
                        }
                    });
            }
            ui.add_space(theme::SPACE);
        });
    view.messages.retain(|key, _| view.used.contains(key));
}

/// What a tool card needs to know.
struct Context<'a> {
    tree: Option<&'a Tree>,
    revision: u64,
    theme: Theme,
    running: bool,
    results: &'a HashMap<String, ToolResult>,
    /// The open question: its call id, text and options.
    question: Option<(&'a str, &'a str, &'a [String])>,
}

fn operator_message(
    ui: &mut egui::Ui,
    view: &mut ViewCache,
    ctx: &Context,
    index: usize,
    text: &str,
    editable: bool,
    actions: &mut Vec<Action>,
) {
    let theme = ctx.theme;
    ui.add_space(theme::SPACE_S);
    egui::Frame::new()
        .fill(theme.elevated)
        .stroke(Stroke::new(theme::HAIRLINE, theme.border))
        .corner_radius(theme::RADIUS_L)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(id) =
                markdown_message(ui, view, ctx, egui::Id::new(("message", index)), text)
            {
                actions.push(Action::Reveal(id));
            }
        });
    if editable {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            let edit = ui
                .add(
                    egui::Button::new(RichText::new("Edit").small().color(theme.muted))
                        .frame(false),
                )
                .on_hover_text("Edit this message and send it again");
            record(ui.ctx(), Target::Button("Edit message"), edit.rect);
            if edit.clicked() {
                actions.push(Action::Edit);
            }
        });
    }
}

/// A message's text as Markdown, parsed once per text and model revision.
/// A message outside the view takes its last height and is not laid out.
fn markdown_message(
    ui: &mut egui::Ui,
    view: &mut ViewCache,
    ctx: &Context,
    id: egui::Id,
    text: &str,
) -> Option<ElementId> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (text, ctx.theme.dark, ctx.theme.contrast).hash(&mut hasher);
    let key = hasher.finish();
    view.used.insert(key);
    let width = ui.available_width();
    let names = &view.names;
    let resolve = |name: &str| names.get(name).copied().flatten();
    let parse = || {
        let links = std::cell::RefCell::new(Vec::new());
        let blocks = markdown::parse(text, ctx.theme, &|name: &str| {
            let element = resolve(name);
            links.borrow_mut().push((name.to_string(), element));
            element
        });
        (blocks, links.into_inner())
    };
    let message = view.messages.entry(key).or_insert_with(|| {
        let (blocks, links) = parse();
        Message {
            blocks,
            links,
            checked: ctx.revision,
            height: None,
        }
    });
    if message.checked != ctx.revision {
        message.checked = ctx.revision;
        if message
            .links
            .iter()
            .any(|(name, element)| resolve(name) != *element)
        {
            (message.blocks, message.links) = parse();
        }
    }
    if let Some((at, height)) = message.height
        && (at - width).abs() < 0.5
    {
        let rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(width, height));
        if !ui.is_rect_visible(rect) {
            ui.allocate_space(Vec2::new(width, height));
            return None;
        }
    }
    let shown = ui.scope(|ui| markdown::show(ui, id, &message.blocks, ctx.theme));
    message.height = Some((width, shown.response.rect.height()));
    shown.inner
}

fn notice(ui: &mut egui::Ui, theme: Theme, text: &str, retry: bool, actions: &mut Vec<Action>) {
    egui::Frame::new()
        .fill(theme.elevated)
        .stroke(Stroke::new(theme::HAIRLINE, theme.border))
        .corner_radius(theme::RADIUS)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.label(
                    RichText::new("!")
                        .font(theme::semibold(theme::BODY))
                        .color(theme.amber),
                );
                ui.add(egui::Label::new(RichText::new(text).color(theme.text_secondary)).wrap());
            });
            if retry {
                let button = ui
                    .button("Retry")
                    .on_hover_text("Send the last message again");
                record(ui.ctx(), Target::Button("Retry"), button.rect);
                if button.clicked() {
                    actions.push(Action::Retry);
                }
            }
        });
}

/// What a tool call is doing, in a few words.
fn title(name: &str, input: &Value) -> String {
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

fn tool_card(
    ui: &mut egui::Ui,
    ctx: &Context,
    id: &str,
    name: &str,
    input: &Value,
    actions: &mut Vec<Action>,
) {
    let theme = ctx.theme;
    let result = ctx.results.get(id);
    if name == tools::ASK_OPERATOR {
        return question_card(ui, ctx, id, input, result, actions);
    }
    let read_only = matches!(
        name,
        tools::READ_MODEL | tools::FIND_ELEMENTS | tools::GET_PROBLEMS
    );
    let open_id = egui::Id::new(("tool-card", id));
    let mut open = ui
        .data(|data| data.get_temp::<bool>(open_id))
        .unwrap_or(false);
    let frame = if read_only {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(2, 0))
    } else {
        egui::Frame::new()
            .fill(theme.elevated)
            .stroke(Stroke::new(theme::HAIRLINE, theme.border))
            .corner_radius(theme::RADIUS_L)
            .inner_margin(egui::Margin::symmetric(10, 7))
    };
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            status_icon(ui, theme, result, ctx.running);
            let size = if read_only { theme::LABEL } else { theme::BODY };
            let font = if read_only {
                theme::regular(size)
            } else {
                theme::medium(size)
            };
            let color = if read_only { theme.muted } else { theme.text };
            ui.add(
                egui::Label::new(RichText::new(title(name, input)).font(font).color(color))
                    .truncate(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let arrow = if open { "▾" } else { "▸" };
                if ui
                    .add(egui::Button::new(RichText::new(arrow).color(theme.muted)).frame(false))
                    .on_hover_text("Details")
                    .clicked()
                {
                    open = !open;
                }
            });
        });
        if let Some(result) = result
            && !read_only
        {
            summary(ui, ctx, result, actions);
        }
        if open {
            let text = serde_json::to_string_pretty(input).unwrap_or_default();
            details(ui, theme, "Input", &text);
            if let Some(result) = result {
                details(ui, theme, "Result", &result.content);
            }
        }
    });
    ui.data_mut(|data| data.insert_temp(open_id, open));
}

fn status_icon(ui: &mut egui::Ui, theme: Theme, result: Option<&ToolResult>, running: bool) {
    let (text, color) = match result {
        None if running => {
            ui.add(egui::Spinner::new().size(12.0).color(theme.accent));
            return;
        }
        None => ("○", theme.muted),
        // A locked element the Operator kept: their decision, not a failure.
        Some(result)
            if result
                .change
                .as_ref()
                .is_some_and(|c| !c.refused.is_empty()) =>
        {
            ("–", theme.muted)
        }
        Some(result) if result.is_error => ("!", theme.amber),
        Some(_) => ("✓", theme.green),
    };
    ui.label(
        RichText::new(text)
            .font(theme::semibold(theme::BODY))
            .color(color),
    );
}

/// The outcome of a change: the elements it created or changed, as links,
/// what it deleted and problems at those elements; the locked elements the
/// Operator kept; or why it was not done. The model's own wording is in the
/// card's details.
fn summary(ui: &mut egui::Ui, ctx: &Context, result: &ToolResult, actions: &mut Vec<Action>) {
    let theme = ctx.theme;
    let tree = ctx.tree;
    let caption = |text: String, color| {
        RichText::new(text)
            .font(theme::regular(theme::CAPTION))
            .color(color)
    };
    let ids =
        |raw: &[u64]| -> Vec<ElementId> { raw.iter().map(|r| ElementId::from_raw(*r)).collect() };
    let Some(change) = &result.change else {
        if result.is_error {
            let first = result.content.lines().next().unwrap_or_default();
            ui.add(egui::Label::new(RichText::new(first).color(theme.amber)).wrap());
        }
        return;
    };
    if !change.refused.is_empty() {
        let names: Vec<String> = ids(&change.refused)
            .into_iter()
            .map(|id| tree.map_or_else(String::new, |tree| crate::edit::display_name(tree, id)))
            .collect();
        ui.label(caption(
            format!("You kept {} unchanged.", names.join(", ")),
            theme.text_secondary,
        ));
        return;
    }
    if result.is_error {
        let first = result.content.lines().next().unwrap_or_default();
        ui.add(egui::Label::new(RichText::new(first).color(theme.amber)).wrap());
        return;
    }
    let created = ids(&change.created);
    if !created.is_empty() && tree.is_none_or(|tree| created.iter().all(|id| !tree.contains(*id))) {
        ui.label(caption("Undone or deleted since.".into(), theme.muted));
        return;
    }
    let owner = |id: &ElementId| tree.and_then(|tree| tree.get(*id)).and_then(|e| e.owner());
    let comment = |id: &ElementId| {
        tree.and_then(|tree| tree.get(*id))
            .is_some_and(|e| matches!(e.kind, ElementKind::Doc | ElementKind::Comment))
    };
    // What was created, outermost first; what changed, without the owners
    // that only gained members (as the Surface highlights them).
    let owners: HashSet<ElementId> = created.iter().filter_map(owner).collect();
    let shown_created: Vec<ElementId> = created
        .iter()
        .filter(|id| !comment(id) && owner(id).is_none_or(|o| !created.contains(&o)))
        .copied()
        .collect();
    let shown_changed: Vec<ElementId> = ids(&change.changed)
        .into_iter()
        .filter(|id| !comment(id) && !owners.contains(id))
        .collect();
    for (label, shown) in [("Created", shown_created), ("Changed", shown_changed)] {
        if shown.is_empty() {
            continue;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(caption(label.into(), theme.muted));
            for id in shown {
                let Some(tree) = tree.filter(|tree| tree.contains(id)) else {
                    ui.label(caption("(undone or deleted)".into(), theme.muted));
                    continue;
                };
                let name = crate::edit::display_name(tree, id);
                let link = ui
                    .add(egui::Link::new(
                        RichText::new(name)
                            .font(theme::code(theme::CODE - 1.0))
                            .color(theme.accent),
                    ))
                    .on_hover_text(tree.qualified_name(id));
                record(ui.ctx(), Target::Link(id.raw()), link.rect);
                if link.clicked() {
                    actions.push(Action::Reveal(id));
                }
            }
        });
    }
    if change.deleted > 0 {
        ui.label(caption(
            format!(
                "Deleted {}",
                crate::conversation::plural(change.deleted, "element", "elements")
            ),
            theme.muted,
        ));
    }
    if change.problems > 0 {
        ui.label(caption(
            format!(
                "{} at the changed elements",
                crate::conversation::plural(change.problems, "problem", "problems")
            ),
            theme.amber,
        ));
    }
}

fn details(ui: &mut egui::Ui, theme: Theme, label: &str, text: &str) {
    ui.add_space(theme::SPACE_S);
    ui.label(theme.overline(label));
    egui::Frame::new()
        .fill(theme.surface)
        .corner_radius(theme::RADIUS)
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(text)
                        .font(theme::code(theme::CODE - 1.0))
                        .color(theme.text_secondary),
                )
                .selectable(true)
                .wrap(),
            );
        });
}

/// A question from the Assistant: its options as buttons while it is open,
/// then the answer.
fn question_card(
    ui: &mut egui::Ui,
    ctx: &Context,
    id: &str,
    input: &Value,
    result: Option<&ToolResult>,
    actions: &mut Vec<Action>,
) {
    let theme = ctx.theme;
    let open = ctx.question.filter(|(call, _, _)| *call == id);
    let question = input
        .get("question")
        .and_then(Value::as_str)
        .or(open.map(|(_, question, _)| question))
        .unwrap_or("The Assistant has a question");
    let stroke = if open.is_some() {
        theme.accent
    } else {
        theme.border
    };
    egui::Frame::new()
        .fill(theme.elevated)
        .stroke(Stroke::new(
            if open.is_some() { 1.5 } else { theme::HAIRLINE },
            stroke,
        ))
        .corner_radius(theme::RADIUS_L)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(theme.overline("The Assistant asks"));
            ui.label(
                RichText::new(question)
                    .font(theme::medium(theme::BODY))
                    .color(theme.text),
            );
            if let Some((_, _, options)) = open {
                ui.add_space(theme::SPACE_S);
                ui.horizontal_wrapped(|ui| {
                    for (index, option) in options.iter().enumerate() {
                        let button = ui.add(
                            egui::Button::new(RichText::new(option).color(theme.accent))
                                .stroke(Stroke::new(theme::HAIRLINE, theme.accent)),
                        );
                        if let Some(name) = OPTIONS.get(index) {
                            record(ui.ctx(), Target::Button(name), button.rect);
                        }
                        if button.clicked() {
                            actions.push(Action::Answer(option.clone()));
                        }
                    }
                });
                ui.label(
                    RichText::new("Or type your own answer below.")
                        .font(theme::regular(theme::CAPTION))
                        .color(theme.muted),
                );
            } else {
                let (text, color) = match result {
                    Some(result) if !result.is_error => (
                        format!("Your answer: {}", result.content),
                        theme.text_secondary,
                    ),
                    Some(_) => (
                        "Not answered: the Assistant was stopped.".to_string(),
                        theme.muted,
                    ),
                    None if ctx.running => ("Waiting…".to_string(), theme.muted),
                    None => ("Not answered.".to_string(), theme.muted),
                };
                ui.label(RichText::new(text).color(color));
            }
        });
}
