//! The Conversation (§3.6): the Operator's messages and the Assistant's
//! streamed replies as Markdown with element links, its thinking, its tool
//! calls as compact live cards with what they changed, its questions, and
//! the composer with Send and Stop.
//!
//! The list is virtualised (GPUI's `list`): only messages in view are laid
//! out, and a reply streaming in re-measures only itself. Text is selected by
//! dragging across paragraphs, code and messages; the selection is kept by
//! place in the text, so it survives scrolling and streaming, and Ctrl+C
//! copies it in order.
mod cards;
pub mod markdown;

use crate::{
    conversation::{Live, Undo, WaitingFor},
    settings::Section,
    studio::{Dirty, Studio, StudioEvent},
    ui::{
        self, ActiveTheme, Button, IconName, KeyCaps, Menu, MenuItem, TextArea, Tone, icon, r,
        theme,
    },
    workspace::StudioExt,
};
use agq_assistant::Entry;
use agq_language::{ElementId, ElementKind};
use agq_providers::AssistantPart;
use cards::{Outcome, Tool};
use gpui::{
    AnyElement, App, AppContext, ClickEvent, ClipboardItem, Context, DismissEvent, Entity,
    FocusHandle, Focusable, Font, FontStyle, FontWeight, InteractiveElement, InteractiveText,
    IntoElement, KeyBinding, ListAlignment, ListState, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Render, SharedString, StatefulInteractiveElement,
    StrikethroughStyle, Styled, StyledText, Subscription, TextLayout, TextRun, Window, actions,
    anchored, deferred, div, font, list, prelude::FluentBuilder, px,
};
use gpui_base::input::{InputEvent, TextareaState};
use markdown::{Block, BlockKey, TextPoint, TextSelection};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    rc::Rc,
};

actions!(conversation, [CopySelection]);

pub fn bind(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(
        "ctrl-c",
        CopySelection,
        Some("Conversation && !Input"),
    )]);
}

/// A text block drawn this frame, for placing a drag.
struct Drawn {
    key: BlockKey,
    layout: TextLayout,
}

/// What the list's items share for one frame.
pub struct Ctx {
    pub studio: Entity<Studio>,
    pub view: Entity<ConversationView>,
    pub expanded: HashSet<String>,
    selection: Option<(TextPoint, TextPoint)>,
    drawn: Rc<RefCell<Vec<Drawn>>>,
}

/// One row of the list.
enum Item {
    Empty,
    Operator {
        place: (usize, usize),
        blocks: Rc<Vec<Block>>,
        editable: bool,
    },
    Assistant {
        place: (usize, usize),
        blocks: Rc<Vec<Block>>,
        /// The whole reply as Markdown, under its last text: "Copy".
        copy: Option<String>,
        streaming: bool,
    },
    Thinking {
        key: String,
        text: String,
        live: bool,
    },
    Tool(Box<Tool>),
    Notice {
        text: String,
        retry: bool,
    },
    Working(String),
    Undo(Undo),
}

impl Item {
    /// Identity and version: an item whose key changes is measured again.
    fn key(&self, expanded: &HashSet<String>) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match self {
            Item::Empty => 0u8.hash(&mut hasher),
            Item::Operator {
                place,
                blocks,
                editable,
            } => (1u8, place, Rc::as_ptr(blocks) as usize, editable).hash(&mut hasher),
            Item::Assistant {
                place,
                blocks,
                copy,
                streaming,
            } => (
                2u8,
                place,
                Rc::as_ptr(blocks) as usize,
                copy.is_some(),
                streaming,
            )
                .hash(&mut hasher),
            Item::Thinking { key, text, live } => {
                (3u8, key, text.len(), live, expanded.contains(key)).hash(&mut hasher)
            }
            Item::Tool(tool) => (
                4u8,
                &tool.id,
                tool.status(),
                tool.input.to_string().len(),
                tool.options.is_some(),
                tool.outcome
                    .as_ref()
                    .map(|o| (o.created.len(), o.changed.len(), o.gone, o.problems)),
                expanded.contains(&tool.id),
            )
                .hash(&mut hasher),
            Item::Notice { text, retry } => (5u8, text, retry).hash(&mut hasher),
            Item::Working(text) => (6u8, text).hash(&mut hasher),
            Item::Undo(undo) => (7u8, format!("{undo:?}")).hash(&mut hasher),
        }
        hasher.finish()
    }
}

/// A parsed message, and the element names its links resolved to.
struct Parsed {
    blocks: Rc<Vec<Block>>,
    links: Vec<(String, Option<ElementId>)>,
    checked: u64,
}

pub struct ConversationView {
    studio: Entity<Studio>,
    focus: FocusHandle,
    composer: Entity<TextareaState>,
    list: ListState,
    items: Rc<Vec<Item>>,
    keys: Vec<u64>,
    pub(crate) expanded: HashSet<String>,
    /// Something shown changed without the Studio saying so (a card opened).
    pub(crate) dirty_items: bool,
    parsed: HashMap<u64, Parsed>,
    /// Element names for links, per model revision (`None`: not unique).
    names: HashMap<String, Option<ElementId>>,
    names_revision: Option<u64>,
    selection: Option<TextSelection>,
    drawn: Rc<RefCell<Vec<Drawn>>>,
    /// The Studio's `input_set` and `epoch` the view last followed.
    input_set: u64,
    epoch: u64,
    model_menu: Option<(Entity<Menu>, Subscription)>,
    _subscriptions: Vec<Subscription>,
}

impl Focusable for ConversationView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl ConversationView {
    pub fn new(studio: Entity<Studio>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let composer = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .placeholder("Ask the Assistant, or describe a change")
                .submit_on_enter(true);
            state.set_auto_grow(1, 10, cx);
            state
        });
        let subscriptions = vec![
            cx.subscribe(&studio, |this, _, event: &StudioEvent, cx| {
                if event.0.intersects(
                    Dirty::CONVERSATION
                        | Dirty::MODEL
                        | Dirty::APPEARANCE
                        | Dirty::OVERLAY
                        | Dirty::SELECTION,
                ) {
                    if event
                        .0
                        .intersects(Dirty::CONVERSATION | Dirty::MODEL | Dirty::APPEARANCE)
                    {
                        this.dirty_items = true;
                    }
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &composer,
                window,
                |this, composer, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => {
                        // A message the Studio set (sent, inserted, taken back to
                        // edit) and the view has not shown yet wins.
                        let set = this.input_set;
                        let text = composer.read(cx).value().to_string();
                        this.studio.update(cx, |studio, _| {
                            if studio.conversation.input_set == set {
                                studio.conversation.input = text;
                            }
                        });
                        cx.notify();
                    }
                    InputEvent::PressEnter { shift: false, .. } => this.send(window, cx),
                    _ => {}
                },
            ),
        ];
        let list = ListState::new(0, ListAlignment::Bottom, px(600.0));
        list.set_follow_mode(gpui::FollowMode::Tail);
        ConversationView {
            studio,
            focus: cx.focus_handle(),
            composer,
            list,
            items: Rc::new(Vec::new()),
            keys: Vec::new(),
            expanded: HashSet::new(),
            dirty_items: true,
            parsed: HashMap::new(),
            names: HashMap::new(),
            names_revision: None,
            selection: None,
            drawn: Rc::new(RefCell::new(Vec::new())),
            input_set: 0,
            epoch: 0,
            model_menu: None,
            _subscriptions: subscriptions,
        }
    }

    /// Gives the composer the keyboard (Ctrl+L, a question arriving).
    pub fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.composer.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.studio
            .update(cx, |studio, _| studio.conversation.focus_input = false);
    }

    fn send(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.studio.act(cx, |studio| {
            studio.send_message();
            studio.mark(Dirty::CONVERSATION | Dirty::STATUS);
        });
    }

    /// Follows what the Studio did to the message being written: an inserted
    /// selection, a message taken back to edit, a sent message.
    fn sync_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let studio = self.studio.read(cx);
        let (input_set, epoch, focus) = (
            studio.conversation.input_set,
            studio.conversation.epoch,
            studio.conversation.focus_input,
        );
        if epoch != self.epoch {
            self.epoch = epoch;
            // Messages are named by position; earlier ones were replaced.
            self.selection = None;
        }
        if input_set != self.input_set {
            self.input_set = input_set;
            let text = studio.conversation.input.clone();
            self.composer.update(cx, |state, cx| {
                state.set_value(text, window, cx);
                // The caret after the text.
                let end = state.value().len();
                state.set_selected_range(end..end, cx);
            });
        }
        if focus {
            self.focus_input(window, cx);
        }
    }

    /// Parses a message once per text, again only when an element it names
    /// appears or goes.
    fn parse(&mut self, text: &str, revision: u64) -> Rc<Vec<Block>> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        let key = hasher.finish();
        let names = &self.names;
        let resolve = |name: &str| names.get(name).copied().flatten();
        if let Some(parsed) = self.parsed.get_mut(&key) {
            if parsed.checked != revision {
                parsed.checked = revision;
                if parsed
                    .links
                    .iter()
                    .any(|(name, element)| resolve(name) != *element)
                {
                    let links = RefCell::new(Vec::new());
                    parsed.blocks = Rc::new(markdown::parse(text, &|name| {
                        let element = resolve(name);
                        links.borrow_mut().push((name.to_string(), element));
                        element
                    }));
                    parsed.links = links.into_inner();
                }
            }
            return parsed.blocks.clone();
        }
        let links = RefCell::new(Vec::new());
        let blocks = Rc::new(markdown::parse(text, &|name| {
            let element = resolve(name);
            links.borrow_mut().push((name.to_string(), element));
            element
        }));
        self.parsed.insert(
            key,
            Parsed {
                blocks: blocks.clone(),
                links: links.into_inner(),
                checked: revision,
            },
        );
        blocks
    }

    /// Builds the list's items from the conversation and tells the list
    /// which of them changed.
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let studio = self.studio.read(cx);
        let state = studio.project.as_ref().map(|p| p.state());
        let revision = state.map_or(0, |s| s.revision());
        if self.names_revision != Some(revision) {
            self.names_revision = Some(revision);
            self.names.clear();
            if let Some(tree) = state.map(|s| s.tree()) {
                for id in tree.walk() {
                    if let Some(name) = tree.effective_name(id) {
                        self.names
                            .entry(name.to_string())
                            .and_modify(|found| *found = None)
                            .or_insert(Some(id));
                        self.names.insert(tree.qualified_name(id), Some(id));
                    }
                }
            }
        }
        let panel = &studio.conversation;
        let running = panel.running();
        let failed = panel.last_turn.as_ref().is_some_and(|turn| turn.failed);
        let last_operator = panel.last_operator();
        let entries = panel.conversation.entries.clone();
        let live: Vec<Live> = panel
            .live
            .iter()
            .map(|live| match live {
                Live::Text(text) => Live::Text(text.clone()),
                Live::Thinking(text) => Live::Thinking(text.clone()),
                Live::Tool { id, name, input } => Live::Tool {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                },
            })
            .collect();
        let results = panel.results.clone();
        let editing = panel.editing.is_some();
        let question = match &panel.waiting {
            Some(waiting) => match &waiting.kind {
                WaitingFor::Question { options, .. } => {
                    Some((waiting.call.id.clone(), options.clone()))
                }
                WaitingFor::SaveToLibrary { .. } => Some((
                    waiting.call.id.clone(),
                    crate::conversation::SAVE_OPTIONS
                        .iter()
                        .map(|option| option.to_string())
                        .collect(),
                )),
                _ => None,
            },
            None => None,
        };
        let assistant_asks = matches!(
            &studio.dialog,
            Some(crate::edit::Dialog::Confirm { change, .. }) if change.actor == agq_system_state::Actor::Assistant
        );
        let waiting_text = match panel.waiting.as_ref().map(|w| &w.kind) {
            Some(WaitingFor::Confirmation) if assistant_asks => {
                Some("Waiting for your answer about the locked element")
            }
            Some(WaitingFor::Dialog(_)) => Some("Waiting until you close the open dialog"),
            _ => None,
        };
        let thinking = panel.thinking;
        let phase = panel.phase;
        let undoable = state.and_then(|state| panel.undoable(state));
        let tree = state.map(|s| s.tree().clone());
        let mut items = Vec::new();
        let outcome = |result: &agq_assistant::ToolResult| -> Option<Outcome> {
            let change = result.change.as_ref()?;
            let tree = tree.as_ref();
            let ids = |raw: &[u64]| -> Vec<ElementId> {
                raw.iter().map(|r| ElementId::from_raw(*r)).collect()
            };
            let owner = |id: &ElementId| tree.and_then(|t| t.get(*id)).and_then(|e| e.owner());
            let comment = |id: &ElementId| {
                tree.and_then(|t| t.get(*id))
                    .is_some_and(|e| matches!(e.kind, ElementKind::Doc | ElementKind::Comment))
            };
            let name = |id: ElementId| -> Option<(ElementId, Option<String>, String)> {
                let tree = tree?;
                tree.contains(id).then(|| {
                    (
                        id,
                        Some(tree.qualified_name(id)),
                        crate::edit::display_name(tree, id),
                    )
                })
            };
            let created = ids(&change.created);
            let owners: HashSet<ElementId> = created.iter().filter_map(owner).collect();
            let kind = |id: &ElementId| tree.and_then(|t| t.get(*id)).map(|e| e.kind);
            let created_package = |id: &ElementId| {
                owner(id)
                    .is_some_and(|o| created.contains(&o) && kind(&o) == Some(ElementKind::Package))
            };
            let gone = !created.is_empty()
                && tree.is_none_or(|t| created.iter().all(|id| !t.contains(*id)));
            Some(Outcome {
                created: created
                    .iter()
                    .filter(|id| !comment(id) && owner(id).is_none_or(|o| !created.contains(&o)))
                    .filter_map(|id| name(*id))
                    .collect(),
                imported: created
                    .iter()
                    .filter(|id| {
                        kind(id).is_some_and(|k| k.is_definition())
                            && (owner(id).is_none_or(|o| !created.contains(&o))
                                || created_package(id))
                    })
                    .filter_map(|id| name(*id))
                    .collect(),
                added: created
                    .iter()
                    .filter(|id| {
                        kind(id).is_some_and(|k| {
                            k.is_usage()
                                && !matches!(k, ElementKind::Connection | ElementKind::Interface)
                        }) && owner(id).is_none_or(|o| !created.contains(&o))
                    })
                    .filter_map(|id| name(*id))
                    .collect(),
                connected: created
                    .iter()
                    .filter(|id| {
                        matches!(
                            kind(id),
                            Some(ElementKind::Connection | ElementKind::Interface)
                        ) && owner(id).is_none_or(|o| !created.contains(&o))
                    })
                    .filter_map(|id| name(*id))
                    .collect(),
                changed: ids(&change.changed)
                    .into_iter()
                    .filter(|id| !comment(id) && !owners.contains(id))
                    .filter_map(name)
                    .collect(),
                deleted: change.deleted,
                problems: change.problems,
                kept: ids(&change.refused)
                    .into_iter()
                    .map(|id| tree.map_or_else(String::new, |t| crate::edit::display_name(t, id)))
                    .collect(),
                gone,
            })
        };
        let tool = |id: &str, name: &str, input: serde_json::Value| Tool {
            id: id.to_string(),
            name: name.to_string(),
            input,
            result: results.get(id).cloned(),
            running,
            outcome: results.get(id).and_then(outcome),
            options: question
                .as_ref()
                .filter(|(call, _)| call == id)
                .map(|(_, options)| options.clone()),
        };
        if entries.is_empty() && live.is_empty() {
            items.push(Item::Empty);
        }
        for (index, entry) in entries.iter().enumerate() {
            match entry {
                Entry::Operator { text } => {
                    let blocks = self.parse(text, revision);
                    items.push(Item::Operator {
                        place: (index, 0),
                        blocks,
                        editable: Some(index) == last_operator && !running && !editing,
                    });
                }
                Entry::Assistant { parts, .. } => {
                    let markdown = reply_markdown(parts);
                    let last_text = parts.iter().rposition(|part| matches!(part, AssistantPart::Text { text } if !text.trim().is_empty()));
                    for (slot, part) in parts.iter().enumerate() {
                        match part {
                            AssistantPart::Text { text } => {
                                let blocks = self.parse(text, revision);
                                items.push(Item::Assistant {
                                    place: (index, slot),
                                    blocks,
                                    copy: (Some(slot) == last_text).then(|| markdown.clone()),
                                    streaming: false,
                                });
                            }
                            AssistantPart::ToolCall { id, name, input } => {
                                items.push(Item::Tool(Box::new(tool(id, name, input.clone()))))
                            }
                            AssistantPart::Reasoning(reasoning) => items.push(Item::Thinking {
                                key: format!("thinking-{index}-{slot}"),
                                text: reasoning.text(),
                                live: false,
                            }),
                        }
                    }
                }
                Entry::ToolResults { .. } => {}
                Entry::Notice { text } => items.push(Item::Notice {
                    text: text.clone(),
                    retry: index + 1 == entries.len() && failed && !running,
                }),
                // Which runtime took over the turn, and whether its earlier
                // context carried over.
                Entry::Session { event, .. } => items.push(Item::Notice {
                    text: match event.as_str() {
                        "resumed" => "The Claude Agent runtime continued its session.".to_string(),
                        _ => "The Claude Agent runtime started a session: it was given the visible history as text; nothing hidden carried over.".to_string(),
                    },
                    retry: false,
                }),
                // A later version's entry: kept, shown plainly.
                Entry::Other(value) => {
                    let kind = value["type"].as_str().unwrap_or("unknown");
                    items.push(Item::Notice {
                        text: format!("A `{kind}` entry from a later version of Agentique; it is kept as it is."),
                        retry: false,
                    });
                }
            }
        }
        let live_count = live.len();
        for (slot, part) in live.into_iter().enumerate() {
            match part {
                Live::Text(text) => {
                    let blocks = Rc::new(markdown::parse(&text, &|name| {
                        self.names.get(name).copied().flatten()
                    }));
                    items.push(Item::Assistant {
                        place: (entries.len(), slot),
                        blocks,
                        copy: None,
                        streaming: slot + 1 == live_count,
                    });
                }
                Live::Tool { id, name, input } => {
                    let input = serde_json::from_str(&input).unwrap_or(serde_json::Value::Null);
                    items.push(Item::Tool(Box::new(tool(&id, &name, input))));
                }
                Live::Thinking(text) => items.push(Item::Thinking {
                    key: format!("live-thinking-{slot}"),
                    text,
                    live: true,
                }),
            }
        }
        if running && question.is_none() {
            items.push(Item::Working(match (waiting_text, thinking, phase) {
                (Some(waiting), ..) => waiting.to_string(),
                (None, true, _) => "Thinking…".to_string(),
                (None, false, Some(phase)) => format!("{phase}…"),
                (None, false, None) => "Working…".to_string(),
            }));
        }
        if let Some(undo) = undoable {
            items.push(Item::Undo(undo));
        }
        // Tell the list what changed: everything from the first difference.
        let keys: Vec<u64> = items.iter().map(|item| item.key(&self.expanded)).collect();
        let common = self
            .keys
            .iter()
            .zip(&keys)
            .take_while(|(a, b)| a == b)
            .count();
        if common != keys.len() || keys.len() != self.keys.len() {
            self.list
                .splice(common..self.keys.len(), keys.len() - common);
        }
        self.keys = keys;
        self.items = Rc::new(items);
        // Keep the cache to the messages still shown.
        if self.parsed.len() > 4 * self.items.len() + 64 {
            let live: HashSet<usize> = self
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Operator { blocks, .. } | Item::Assistant { blocks, .. } => {
                        Some(Rc::as_ptr(blocks) as usize)
                    }
                    _ => None,
                })
                .collect();
            self.parsed
                .retain(|_, parsed| live.contains(&(Rc::as_ptr(&parsed.blocks) as usize)));
        }
    }

    /// Where a window position is in the text drawn this frame.
    fn point_at(&self, position: gpui::Point<Pixels>) -> Option<TextPoint> {
        let drawn = self.drawn.borrow();
        let mut ordered: Vec<&Drawn> = drawn.iter().collect();
        ordered.sort_by_key(|d| d.key);
        for block in &ordered {
            let bounds = block.layout.bounds();
            if position.y < bounds.top() {
                return Some(TextPoint {
                    message: block.key.0,
                    block: block.key.1,
                    offset: 0,
                });
            }
            if position.y <= bounds.bottom() {
                let offset = match block.layout.index_for_position(position) {
                    Ok(index) | Err(index) => index,
                };
                return Some(TextPoint {
                    message: block.key.0,
                    block: block.key.1,
                    offset,
                });
            }
        }
        ordered.last().map(|block| TextPoint {
            message: block.key.0,
            block: block.key.1,
            offset: block.layout.len(),
        })
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let over_text = self
            .drawn
            .borrow()
            .iter()
            .any(|d| d.layout.bounds().contains(&event.position));
        if !over_text {
            if self.selection.take().is_some() {
                cx.notify();
            }
            return;
        }
        window.focus(&self.focus, cx);
        if let Some(point) = self.point_at(event.position) {
            self.selection = Some(TextSelection {
                anchor: point,
                focus: point,
                dragging: true,
            });
            cx.notify();
        }
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(selection) = self.selection.filter(|s| s.dragging) else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.selection = Some(TextSelection {
                dragging: false,
                ..selection
            });
            return;
        }
        // Past the top or the bottom of the list: scroll towards the pointer.
        let bounds = self.list.viewport_bounds();
        let past = if event.position.y < bounds.top() {
            event.position.y - bounds.top()
        } else if event.position.y > bounds.bottom() {
            event.position.y - bounds.bottom()
        } else {
            px(0.0)
        };
        if past != px(0.0) {
            self.list.scroll_by(past.clamp(px(-24.0), px(24.0)));
        }
        if let Some(point) = self.point_at(event.position) {
            self.selection = Some(TextSelection {
                focus: point,
                ..selection
            });
            cx.notify();
        }
    }

    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selection) = self.selection {
            self.selection = (!selection.is_empty()).then_some(TextSelection {
                dragging: false,
                ..selection
            });
            cx.notify();
        }
    }

    /// The selected text, in order.
    pub fn selected_text(&self) -> Option<String> {
        let selection = self.selection.filter(|s| !s.is_empty())?;
        let messages = self.items.iter().filter_map(|item| match item {
            Item::Operator { place, blocks, .. } | Item::Assistant { place, blocks, .. } => {
                Some((*place, blocks.as_slice()))
            }
            _ => None,
        });
        Some(markdown::selected_text(messages, selection.range()))
    }

    fn copy(&mut self, _: &CopySelection, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn open_model_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let studio = self.studio.read(cx);
        let mut items = Vec::new();
        if std::env::var_os("AGENTIQUE_PROVIDER").is_some() {
            items.push(MenuItem::Note("Set by AGENTIQUE_PROVIDER".into()));
        } else {
            let current = studio.settings.text("assistant.provider");
            items.push(MenuItem::Header("Model".into()));
            let studio_entity = self.studio.clone();
            items.push(
                MenuItem::action("Automatic", move |_, cx| {
                    studio_entity.act(cx, |studio| studio.choose_provider(""))
                })
                .checked(current.is_empty()),
            );
            for provider in agq_assistant::ModelChoice::usable_providers() {
                let studio_entity = self.studio.clone();
                let id = provider.id().to_string();
                items.push(
                    MenuItem::action(
                        format!("{} · {}", provider.name(), provider.default_model()),
                        move |_, cx| {
                            let id = id.clone();
                            studio_entity.act(cx, |studio| studio.choose_provider(&id))
                        },
                    )
                    .checked(current == provider.id()),
                );
            }
        }
        items.push(MenuItem::Separator);
        let studio_entity = self.studio.clone();
        items.push(
            MenuItem::action("More in Settings…", move |_, cx| {
                studio_entity.act(cx, |studio| studio.show_settings(Section::Assistant))
            })
            .icon(IconName::Settings),
        );
        let menu = cx.new(|cx| Menu::new(items, cx).min_width(260.0));
        let subscription = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, _, cx| {
            this.model_menu = None;
            cx.notify();
        });
        let focus = menu.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.model_menu = Some((menu, subscription));
        cx.notify();
    }
}

/// A reply as Markdown, its texts in order: what "Copy" puts on the
/// clipboard.
fn reply_markdown(parts: &[AssistantPart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            AssistantPart::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What a screen reader says for a message: who, then its plain text.
fn message_name(who: &str, blocks: &[Block]) -> String {
    format!("{who}: {}", markdown::plain_text(blocks))
}

/// The runs of a text block: its styles, its links, and the selection.
fn runs(
    block: &markdown::TextBlock,
    selected: Option<std::ops::Range<usize>>,
    theme: &ui::Theme,
) -> Vec<TextRun> {
    let base = if block.quote {
        theme.text_secondary
    } else {
        theme.text
    };
    let mut cuts: Vec<usize> = vec![0, block.text.len()];
    for (range, _) in &block.spans {
        cuts.push(range.start);
        cuts.push(range.end);
    }
    if let Some(selected) = &selected {
        cuts.push(selected.start.min(block.text.len()));
        cuts.push(selected.end.min(block.text.len()));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let style_at = |at: usize| {
        block
            .spans
            .iter()
            .find(|(range, _)| range.start <= at && at < range.end)
            .map(|(_, style)| *style)
            .unwrap_or_default()
    };
    cuts.windows(2)
        .filter(|pair| pair[1] > pair[0])
        .map(|pair| {
            let style = style_at(pair[0]);
            let in_selection = selected
                .as_ref()
                .is_some_and(|s| s.start <= pair[0] && pair[0] < s.end);
            let mut run_font: Font = font(if style.code { theme::MONO } else { theme::SANS });
            run_font.weight = if style.bold {
                FontWeight(crate::tokens::text::SEMIBOLD as f32)
            } else {
                FontWeight(crate::tokens::text::REGULAR as f32)
            };
            run_font.style = if style.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };
            TextRun {
                len: pair[1] - pair[0],
                font: run_font,
                color: if style.link { theme.accent.text } else { base },
                // An element link is a soft accent chip; other code a soft
                // grey one.
                background_color: if in_selection {
                    Some(theme.accent.solid.opacity(0.28))
                } else if style.link {
                    Some(theme.accent.soft)
                } else if style.code {
                    Some(theme.hover)
                } else {
                    None
                },
                underline: None,
                strikethrough: style.strike.then(|| StrikethroughStyle {
                    thickness: px(1.0),
                    color: Some(base),
                }),
            }
        })
        .collect()
}

/// A plain run over `text`, with the selection.
fn plain_runs(
    text: &str,
    family: &'static str,
    colour: gpui::Hsla,
    selected: Option<std::ops::Range<usize>>,
    theme: &ui::Theme,
) -> Vec<TextRun> {
    let run = |len: usize, selected: bool| TextRun {
        len,
        font: font(family),
        color: colour,
        background_color: selected.then(|| theme.accent.solid.opacity(0.28)),
        underline: None,
        strikethrough: None,
    };
    match selected {
        Some(range) if range.start < range.end => {
            let (start, end) = (
                markdown::floor_boundary(text, range.start),
                markdown::floor_boundary(text, range.end),
            );
            [
                (start, false),
                (end - start, true),
                (text.len() - end, false),
            ]
            .into_iter()
            .filter(|(len, _)| *len > 0)
            .map(|(len, selected)| run(len, selected))
            .collect()
        }
        _ => vec![run(text.len(), false)],
    }
}

/// A block's text as a selectable, link-clickable element, registered for
/// placing a drag once it is laid out.
fn text_element(
    ctx: &Rc<Ctx>,
    key: BlockKey,
    text: SharedString,
    text_runs: Vec<TextRun>,
    links: Vec<(std::ops::Range<usize>, ElementId)>,
    blocks: Vec<(std::ops::Range<usize>, agq_library::BlockRef)>,
) -> AnyElement {
    // Where each link ends, for the scripted journeys: its last character is
    // on the link even when it wraps, and in view once the reply is.
    #[cfg(feature = "automation")]
    let last = |range: &std::ops::Range<usize>| {
        text[..range.end]
            .char_indices()
            .last()
            .map_or(range.start, |(i, _)| i)
    };
    #[cfg(feature = "automation")]
    let starts: Vec<(usize, String)> = links
        .iter()
        .map(|(range, id)| (last(range), format!("Link {}", id.raw())))
        .chain(
            blocks
                .iter()
                .map(|(range, block)| (last(range), format!("Block {block}"))),
        )
        .collect();
    let styled = StyledText::new(text).with_runs(text_runs);
    let layout = styled.layout().clone();
    let drawn = ctx.drawn.clone();
    #[cfg(feature = "automation")]
    let drawn = {
        let layout = layout.clone();
        let registry = drawn;
        move || {
            for (start, name) in &starts {
                if let Some(at) = layout.position_for_index(*start) {
                    let height = layout.line_height();
                    let bounds = gpui::Bounds::new(at, gpui::size(gpui::px(8.0), height));
                    ui::target::record(name, bounds);
                }
            }
            registry.clone()
        }
    };
    #[cfg(not(feature = "automation"))]
    let drawn = move || drawn.clone();
    let element = if links.is_empty() && blocks.is_empty() {
        styled.into_any_element()
    } else {
        let studio = ctx.studio.clone();
        let ranges: Vec<_> = links
            .iter()
            .map(|(range, _)| range.clone())
            .chain(blocks.iter().map(|(range, _)| range.clone()))
            .collect();
        let ids: Vec<ElementId> = links.iter().map(|(_, id)| *id).collect();
        let blocks: Vec<agq_library::BlockRef> =
            blocks.into_iter().map(|(_, block)| block).collect();
        InteractiveText::new(
            SharedString::from(format!("text-{}-{}-{}", key.0.0, key.0.1, key.1)),
            styled,
        )
        .on_click(ranges, move |index, _, cx| match ids.get(index) {
            Some(id) => {
                let id = *id;
                studio.act(cx, |studio| studio.reveal(id));
            }
            None => {
                let block = blocks[index - ids.len()].clone();
                studio.act(cx, |studio| studio.show_block(block));
            }
        })
        .into_any_element()
    };
    div()
        .w_full()
        .on_children_prepainted(move |_, _, _| {
            drawn().borrow_mut().push(Drawn {
                key,
                layout: layout.clone(),
            })
        })
        .child(element)
        .into_any_element()
}

/// A message's blocks.
fn message(ctx: &Rc<Ctx>, place: (usize, usize), blocks: &[Block], cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    div()
        .flex()
        .flex_col()
        .gap(r(10.0))
        .text_size(r(theme::text::PROSE))
        .line_height(r(22.0))
        .children(blocks.iter().enumerate().map(|(index, block)| {
            let key = (place, index);
            let selected = |length: usize| {
                ctx.selection
                    .and_then(|range| markdown::block_range(range, key, length))
            };
            match block {
                Block::Text(text) => {
                    let (size, line) = match text.heading {
                        Some(1) => (theme::text::LG + 1.0, 24.0),
                        Some(_) => (theme::text::LG, 22.0),
                        None => (theme::text::PROSE, 22.0),
                    };
                    let element = text_element(
                        ctx,
                        key,
                        SharedString::from(text.text.clone()),
                        runs(text, selected(text.text.len()), &theme),
                        text.links.clone(),
                        text.blocks.clone(),
                    );
                    div()
                        .flex()
                        .gap(r(8.0))
                        .pl(r(16.0
                            * text
                                .indent
                                .saturating_sub(usize::from(text.marker.is_some()))
                                as f32))
                        .text_size(r(size))
                        .line_height(r(line))
                        .when(text.heading.is_some(), |this| this.pt(r(4.0)))
                        .when(text.quote, |this| {
                            this.pl(r(12.0)).border_l_2().border_color(theme.border)
                        })
                        .when_some(text.marker.clone(), |this, marker| {
                            this.child(
                                div()
                                    .flex_none()
                                    .min_w(r(14.0))
                                    .text_color(theme.text_muted)
                                    .child(marker),
                            )
                        })
                        .child(div().flex_1().min_w_0().child(element))
                        .into_any_element()
                }
                Block::Code {
                    language,
                    code,
                    marker,
                    indent,
                } => {
                    let copy = code.clone();
                    let element = text_element(
                        ctx,
                        key,
                        SharedString::from(code.clone()),
                        plain_runs(code, theme::MONO, theme.text, selected(code.len()), &theme),
                        Vec::new(),
                        Vec::new(),
                    );
                    div()
                        .flex()
                        .gap(r(8.0))
                        .pl(r(16.0
                            * indent.saturating_sub(usize::from(marker.is_some()))
                                as f32))
                        .when_some(marker.clone(), |this, marker| {
                            this.child(div().text_color(theme.text_muted).child(marker))
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .rounded(r(crate::tokens::radius::CARD))
                                .bg(theme.inset)
                                .border_1()
                                .border_color(theme.separator)
                                .overflow_hidden()
                                .child(
                                    div()
                                        .h(r(28.0))
                                        .px(r(10.0))
                                        .flex()
                                        .items_center()
                                        .border_b_1()
                                        .border_color(theme.separator)
                                        .text_size(r(theme::text::XS))
                                        .text_color(theme.text_muted)
                                        .child(div().flex_1().min_w_0().child(
                                            if language.is_empty() {
                                                "code".to_string()
                                            } else {
                                                language.clone()
                                            },
                                        ))
                                        .child(
                                            Button::new(
                                                SharedString::from(format!(
                                                    "copy-code-{}-{}-{index}",
                                                    place.0, place.1
                                                )),
                                                "Copy",
                                            )
                                            .small()
                                            .variant(ui::Variant::Subtle)
                                            .icon(IconName::Copy)
                                            .on_click(
                                                move |_: &ClickEvent, _, cx| {
                                                    cx.write_to_clipboard(
                                                        ClipboardItem::new_string(copy.clone()),
                                                    )
                                                },
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .p(r(10.0))
                                        .font_family(theme::MONO)
                                        .text_size(r(12.5))
                                        .line_height(r(19.0))
                                        .child(element),
                                ),
                        )
                        .into_any_element()
                }
                Block::Rule => ui::divider(cx).my(r(4.0)).into_any_element(),
                Block::Table { header, rows } => table(header, rows, cx),
            }
        }))
        .into_any_element()
}

fn table(header: &[String], rows: &[Vec<String>], cx: &App) -> AnyElement {
    let theme = cx.theme();
    let columns = header
        .len()
        .max(rows.iter().map(Vec::len).max().unwrap_or(0))
        .max(1);
    let row = |cells: &[String], head: bool| {
        div()
            .flex()
            .border_b_1()
            .border_color(theme.separator)
            .when(head, |this| this.bg(theme.hover.opacity(0.5)))
            .children((0..columns).map(|column| {
                div()
                    .flex_1()
                    .min_w_0()
                    .px(r(10.0))
                    .py(r(6.0))
                    .text_size(r(theme::text::SM))
                    .line_height(r(18.0))
                    .when(head, |this| this.font_weight(theme::SEMIBOLD))
                    .child(cells.get(column).cloned().unwrap_or_default())
            }))
    };
    div()
        .rounded(r(crate::tokens::radius::CARD))
        .border_1()
        .border_color(theme.border)
        .overflow_hidden()
        .child(row(header, true))
        .children(rows.iter().map(|cells| row(cells, false)))
        .into_any_element()
}

fn render_item(ctx: &Rc<Ctx>, item: &Item, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    let body = match item {
        Item::Empty => div()
            .pt(r(40.0))
            .flex()
            .flex_col()
            .items_center()
            .child(
                ui::EmptyState::new(
                    IconName::Conversation,
                    "Ask the Assistant",
                    "Tell it what to build or change. You see every change on the Surface as it happens, and every change can be undone.",
                )
                .hint("Ctrl+L", "write here")
                .hint("Ctrl+I", "insert the selection"),
            )
            .into_any_element(),
        Item::Operator { place, blocks, editable } => {
            let studio = ctx.studio.clone();
            div()
                .flex()
                .flex_col()
                .items_end()
                .gap(r(4.0))
                .id(SharedString::from(format!("operator-{}", place.0)))
                .role(gpui::Role::Article)
                .aria_label(SharedString::from(message_name("You", blocks)))
                .child(
                    div()
                        .max_w(gpui::relative(0.92))
                        .px(r(12.0))
                        .py(r(9.0))
                        .rounded(r(crate::tokens::radius::CARD + 4.0))
                        .bg(theme.raised)
                        .border_1()
                        .border_color(theme.border)
                        .child(message(ctx, *place, blocks, cx)),
                )
                .when(*editable, |this| {
                    this.child(
                        Button::new(SharedString::from(format!("edit-{}", place.0)), "Edit")
                            .small()
                            .variant(ui::Variant::Subtle)
                            .icon(IconName::Pencil)
                            .tooltip("Edit this message and send it again", None)
                            .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.edit_last_message())),
                    )
                })
                .into_any_element()
        }
        Item::Assistant { place, blocks, copy, streaming } => {
            div()
                .flex()
                .flex_col()
                .gap(r(4.0))
                .id(SharedString::from(format!("assistant-{}-{}", place.0, place.1)))
                .role(gpui::Role::Article)
                .aria_label(SharedString::from(message_name("Assistant", blocks)))
                .child(message(ctx, *place, blocks, cx))
                .when(*streaming, |this| {
                    this.child(
                        div()
                            .w(r(8.0))
                            .h(r(16.0))
                            .rounded(r(2.0))
                            .bg(theme.info.solid.opacity(0.7))
                            .with_animation(
                                "caret",
                                gpui::Animation::new(std::time::Duration::from_millis(900)).repeat(),
                                |this, delta| this.opacity(if delta < 0.5 { 1.0 } else { 0.25 }),
                            ),
                    )
                })
                .when_some(copy.clone(), |this, copy| {
                    this.child(
                        div().flex().child(
                            Button::new(SharedString::from(format!("copy-reply-{}", place.0)), "Copy")
                                .small()
                                .ghost()
                                .icon(IconName::Copy)
                                .tooltip("Copy this reply as Markdown", None)
                                .on_click(move |_: &ClickEvent, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))),
                        ),
                    )
                })
                .into_any_element()
        }
        Item::Thinking { key, text, live } => cards::thinking_row(ctx, key.clone(), text, *live, cx),
        Item::Tool(tool) => cards::tool_card(ctx, tool, cx),
        Item::Notice { text, retry } => cards::notice(ctx, text.clone(), *retry),
        Item::Working(text) => div()
            .flex()
            .items_center()
            .gap(r(8.0))
            .text_size(r(theme::text::SM))
            .text_color(theme.info.text)
            .child(ui::spinner("working", 14.0, theme.info.text))
            .child(text.clone())
            .into_any_element(),
        Item::Undo(undo) => cards::undo_card(ctx, *undo, cx),
    };
    div().px(r(16.0)).pb(r(14.0)).child(body).into_any_element()
}

use gpui::AnimationExt as _;

impl Render for ConversationView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_composer(window, cx);
        if self.dirty_items {
            self.dirty_items = false;
            self.rebuild(cx);
        }
        self.drawn.borrow_mut().clear();
        let theme = cx.theme().clone();
        let studio = self.studio.read(cx);
        let panel = &studio.conversation;
        let running = panel.running();
        let asked = match panel.waiting.as_ref().map(|w| &w.kind) {
            Some(
                WaitingFor::Question { question, .. } | WaitingFor::SaveToLibrary { question, .. },
            ) => Some(question.clone()),
            _ => None,
        };
        let question = asked.is_some();
        let editing = panel.editing.is_some();
        let key_missing = panel.key_missing.clone();
        let errors: Vec<String> = [&panel.read_error, &panel.save_error]
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        // While a steerable turn runs, a message joins it (C-39, C-53).
        let steering = running && panel.steerable && !question;
        let paused = studio.assistant_paused();
        let held_at = panel.held_at.clone();
        let can_send = !panel.input.trim().is_empty()
            && (question || steering || (!running && key_missing.is_none()));
        let usage = panel.usage;
        let mut model = panel.model_name.clone();
        let mut hover = format!(
            "Tokens this session: {} input, {} from cache, {} output",
            usage.input_tokens + usage.cache_creation_input_tokens,
            usage.cache_read_input_tokens,
            usage.output_tokens
        );
        let show_cost = studio
            .settings
            .get("assistant.showCost")
            .as_bool()
            .unwrap_or(true);
        let cost = if show_cost && let Some(turn_model) = &panel.turn_model {
            let turn = panel.turn_usage.cost_usd(turn_model);
            let today = crate::cost::dollars(studio.daily_cost.today());
            let as_of = agq_providers::price(turn_model).map_or_else(
                || "no list price is known for this model".to_string(),
                |price| format!("list prices read {}", price.as_of),
            );
            hover.push_str(&format!(
                "\nEstimated from {as_of}; not a bill. Today is the UTC day."
            ));
            Some(match turn {
                Some(turn) => format!("{} this turn · {today} today", crate::cost::dollars(turn)),
                None => format!("cost not known · {today} today"),
            })
        } else {
            None
        };
        model.push(' ');
        let selection_names: Vec<String> =
            studio.project.as_ref().map_or_else(Vec::new, |project| {
                let tree = project.state().tree();
                studio
                    .working_elements()
                    .into_iter()
                    .filter(|id| tree.contains(*id))
                    .take(3)
                    .map(|id| crate::edit::display_name(tree, id))
                    .collect()
            });
        let ctx = Rc::new(Ctx {
            studio: self.studio.clone(),
            view: cx.entity(),
            expanded: self.expanded.clone(),
            selection: self.selection.map(|s| s.range()),
            drawn: self.drawn.clone(),
        });
        let items = self.items.clone();
        let studio_entity = self.studio.clone();
        let model_menu = self.model_menu.as_ref().map(|(menu, _)| menu.clone());
        let hover_tooltip = ui::tooltip::text(hover, None);
        crate::ui::target::regioned("conversation", div()
            .id("conversation")
            .key_context("Conversation")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::copy))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.chrome)
            .role(gpui::Role::Complementary)
            .aria_label("Conversation")
            // Header: the model, the cost, a new conversation.
            .child(
                div()
                    .flex_none()
                    .h(r(36.0))
                    .pl(r(12.0))
                    .pr(r(6.0))
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .border_b_1()
                    .border_color(theme.separator)
                    .child(icon(IconName::Conversation).size(13.0).color(theme.text_faint))
                    .child(div().text_size(r(theme::text::SM)).font_weight(theme::MEDIUM).child("Conversation"))
                    .child(div().flex_1().min_w_0())
                    .child(
                        div()
                            .id("model-picker")
                            .flex()
                            .items_center()
                            .gap(r(4.0))
                            .h(r(24.0))
                            .px(r(8.0))
                            .rounded(r(crate::tokens::radius::CONTROL))
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.hover).text_color(theme.text_secondary))
                            .role(gpui::Role::Button)
                            .aria_label("Model")
                            .tooltip(move |window, cx| hover_tooltip(window, cx))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.open_model_menu(window, cx)))
                            .child(icon(IconName::Model).size(12.0).color(theme.text_faint))
                            .child(model)
                            .when_some(cost, |this, cost| this.child(div().text_color(theme.text_faint).child(cost)))
                            .child(icon(IconName::ChevronDown).size(11.0).color(theme.text_faint)),
                    )
                    .child(
                        Button::icon_only("new-conversation", IconName::Plus, "New conversation")
                            .small()
                            .tooltip("Start a new conversation; the model is unaffected", None)
                            .on_click({
                                let studio = self.studio.clone();
                                move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.new_conversation())
                            }),
                    ),
            )
            .when_some(model_menu, |this, menu| {
                this.child(deferred(anchored().child(div().mt(r(38.0)).child(menu))).with_priority(2))
            })
            // What to fix before the Assistant can work.
            .when_some(key_missing, |this, message| {
                let studio = self.studio.clone();
                this.child(
                    div().px(r(12.0)).pt(r(10.0)).child(
                        ui::Banner::new(Tone::Warning, message).action(
                            Button::new("open-providers", "Open Settings › Providers")
                                .small()
                                .icon(IconName::Key)
                                .on_click(move |_: &ClickEvent, _, cx| {
                                    studio.act(cx, |studio| studio.show_settings(Section::Providers))
                                }),
                        ),
                    ),
                )
            })
            .children(errors.into_iter().map(|error| div().px(r(12.0)).pt(r(10.0)).child(ui::Banner::new(Tone::Danger, error))))
            // The messages.
            .child(
                div()
                    .id("conversation-list")
                    .flex_1().min_w_0()
                    .min_h_0()
                    .pt(r(12.0))
                    .role(gpui::Role::Log)
                    .aria_label("Conversation")
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
                    .on_mouse_move(cx.listener(Self::mouse_move))
                    .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
                    .cursor(gpui::CursorStyle::IBeam)
                    .child(
                        list(self.list.clone(), move |index, _, cx| match items.get(index) {
                            Some(item) => render_item(&ctx, item, cx),
                            None => div().into_any_element(),
                        })
                        .size_full(),
                    ),
            )
            // The composer.
            .child(
                div()
                    .flex_none()
                    .p(r(10.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .rounded(r(crate::tokens::radius::CARD + 4.0))
                            .bg(theme.raised)
                            .border_1()
                            .border_color(if self.composer.read(cx).focus_handle(cx).is_focused(window) { theme.accent.border } else { theme.border })
                            .shadow(theme.shadow_small())
                            .when(editing, |this| {
                                let studio = self.studio.clone();
                                this.child(
                                    div()
                                        .px(r(12.0))
                                        .pt(r(8.0))
                                        .flex()
                                        .items_center()
                                        .gap(r(6.0))
                                        .text_size(r(theme::text::XS))
                                        .text_color(theme.text_muted)
                                        .child(icon(IconName::Pencil).size(12.0).color(theme.text_muted))
                                        .child(div().flex_1().min_w_0().child("Editing your last message; it and what followed are replaced"))
                                        .child(Button::new("cancel-edit", "Cancel").small().ghost().on_click(move |_: &ClickEvent, _, cx| {
                                            studio.act(cx, |studio| studio.cancel_edit())
                                        })),
                                )
                            })
                            .when_some(asked.filter(|_| !editing), |this, asked| {
                                // Screen readers hear the question itself.
                                this.child(
                                    div()
                                        .id("question-hint")
                                        .role(gpui::Role::Status)
                                        .aria_label(SharedString::from(format!("The Assistant asks: {asked} Type an answer, or pick an option above.")))
                                        .px(r(12.0))
                                        .pt(r(8.0))
                                        .text_size(r(theme::text::XS))
                                        .text_color(theme.accent.text)
                                        .child("Type an answer to the Assistant's question, or pick an option above"),
                                )
                            })
                            .child(div().px(r(12.0)).pt(r(10.0)).pb(r(4.0)).child(TextArea::new(&self.composer).borderless().target("Message")))
                            .child(
                                div()
                                    .px(r(8.0))
                                    .pb(r(8.0))
                                    .flex()
                                    .items_center()
                                    .gap(r(6.0))
                                    // The selection, one click from the message (context chips).
                                    .when(!selection_names.is_empty(), |this| {
                                        let studio = studio_entity.clone();
                                        this.child(
                                            div()
                                                .id("insert-selection")
                                                .min_w_0()
                                                .overflow_hidden()
                                                .flex()
                                                .items_center()
                                                .gap(r(4.0))
                                                .h(r(22.0))
                                                .px(r(6.0))
                                                .rounded(r(crate::tokens::radius::TAG + 2.0))
                                                .border_1()
                                                .border_color(theme.separator)
                                                .text_size(r(theme::text::XS))
                                                .text_color(theme.text_muted)
                                                .cursor_pointer()
                                                .hover(|style| style.bg(theme.hover).text_color(theme.text))
                                                .role(gpui::Role::Button)
                                                .aria_label("Insert selection")
                                                .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.insert_selection()))
                                                .child(icon(IconName::Plus).size(11.0).color(theme.text_faint))
                                                .child(div().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().font_family(theme::MONO).child(selection_names.join(", ")))
                                                .child(div().flex_none().child(KeyCaps::new("Ctrl+I"))),
                                        )
                                    })
                                    .child(div().flex_1().min_w_0())
                                    .when(steering, |this| {
                                        let studio = self.studio.clone();
                                        this.when(can_send, |this| {
                                            this.child(
                                                Button::new("queue", "Add")
                                                    .small()
                                                    .icon(IconName::Send)
                                                    .shortcut("Enter")
                                                    .tooltip("Add this message to the turn that is running; the Assistant reads it at its next step", None)
                                                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.send(window, cx))),
                                            )
                                        })
                                        .child(if paused {
                                            Button::new("resume", "Resume")
                                                .small()
                                                .icon(IconName::Play)
                                                .tooltip(
                                                    match &held_at {
                                                        Some(tool) => format!("Paused before {tool}: let the Assistant go on"),
                                                        None => "Let the Assistant go on".to_string(),
                                                    },
                                                    None,
                                                )
                                                .on_click({
                                                    let studio = studio.clone();
                                                    move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.resume_assistant())
                                                })
                                                .into_any_element()
                                        } else {
                                            Button::new("pause", "Pause")
                                                .small()
                                                .icon(IconName::Pause)
                                                .tooltip("Hold the Assistant before its next tool call", None)
                                                .on_click({
                                                    let studio = studio.clone();
                                                    move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.pause_assistant())
                                                })
                                                .into_any_element()
                                        })
                                        .when(paused, |this| {
                                            this.child(
                                                Button::new("step", "Step")
                                                    .small()
                                                    .tooltip("Let the Assistant take one tool call, then hold it again", None)
                                                    .on_click({
                                                        let studio = studio.clone();
                                                        move |_: &ClickEvent, _, cx| studio.act(cx, |studio| studio.step_assistant())
                                                    }),
                                            )
                                        })
                                    })
                                    .child(if running && !question {
                                        Button::new("stop", "Stop")
                                            .small()
                                            .danger()
                                            .icon(IconName::Stop)
                                            .tooltip("Stop the Assistant now; its changes so far stay and can be undone", None)
                                            .on_click({
                                                let studio = self.studio.clone();
                                                move |_: &ClickEvent, _, cx| studio.act(cx, |studio| if !studio.refused_to_agents("stopping the Assistant") { studio.stop_assistant() })
                                            })
                                            .into_any_element()
                                    } else {
                                        Button::new("send", if question { "Answer" } else { "Send" })
                                            .small()
                                            .primary()
                                            .icon(IconName::Send)
                                            .shortcut("Enter")
                                            .disabled(!can_send)
                                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.send(window, cx)))
                                            .into_any_element()
                                    })
                                    .when(running && question, |this| {
                                        this.child(Button::new("stop-question", "Stop").small().on_click({
                                            let studio = self.studio.clone();
                                            move |_: &ClickEvent, _, cx| studio.act(cx, |studio| if !studio.refused_to_agents("stopping the Assistant") { studio.stop_assistant() })
                                        }))
                                    }),
                            ),
                    ),
            ))
    }
}

#[cfg(test)]
mod tests;
