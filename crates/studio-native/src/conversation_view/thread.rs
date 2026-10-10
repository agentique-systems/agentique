//! Objectives' threads in the Conversation (C-54, ROADMAP §3.6, §4.16 "The
//! Conversation, one place"): the objective's entries, read from its
//! records, in time order with the conversation's own entries (never sent
//! to the Assistant's model). The Operator's messages ("you", with where
//! each went), agents' directives (author → recipient, scope, a status that
//! follows the record), results and Agentique's events are told apart,
//! each agent's entry with its role and model; tool activity folds under
//! the step it belongs to, with its diff or command line one click further;
//! a child objective's thread is nested under the directive that started
//! it.
//!
//! The rows are worked out here without a window ([`rows`], [`merge`],
//! [`routing`], [`label`]), and drawn by [`render_row`].

use super::Ctx;
use crate::{
    ui::{self, ActiveTheme, Button, IconName, Tone, icon, r, theme},
    workspace::StudioExt,
};
use agq_orchestrator::record::{DirectiveStatus, Objective, Recipient};
use agq_orchestrator::thread::{Author, Kind, ThreadEntry};
use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
};
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// One row of an objective's thread as the Conversation shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Its control id: `thread-<objective>-<seq>`, or
    /// `thread-fold-<objective>-<seq>` for the tool calls under a step.
    pub id: String,
    pub objective: String,
    pub seq: u64,
    /// The time it is ordered by among the conversation's entries: its
    /// step's, so a step, its folded activity and a child's thread nested
    /// under it stay together.
    pub at: String,
    /// 0 for the objective the Operator started, 1 for its child, 2 for a
    /// grandchild.
    pub depth: u8,
    pub what: What,
}

#[derive(Clone, Debug, PartialEq)]
pub enum What {
    /// A message, a directive, a result or an event.
    Step(Step),
    /// The tool calls folded under a step, or the updates of an
    /// exploration run (`progress`: the explorer makes no tool calls); with
    /// the latest of them while it is the thread's latest entry (the work
    /// goes on), so what a run does shows without opening the fold.
    Fold {
        count: usize,
        open: bool,
        progress: bool,
        latest: Option<String>,
    },
    /// One tool call or update, shown while its fold is open.
    Activity {
        author: String,
        text: String,
        details: Option<String>,
        open: bool,
        progress: bool,
    },
    /// The objective in one line, where its thread is not shown (it was not
    /// started from this conversation, or ended before this session).
    Summary { intent: String, state: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub kind: Kind,
    /// Who wrote it: `you`, `Agentique`, `lead · claude-opus-5-5`.
    pub author: String,
    /// Its text as it shows now: part of it while it streams in.
    pub text: String,
    pub streaming: bool,
    pub details: Option<String>,
    pub open: bool,
    /// For a directive: whom it is for and where it stands.
    pub directive: Option<DirectiveChip>,
    /// For the Operator's message: where it went.
    pub routing: Option<String>,
    /// Offers Reply: a directive of an objective that still takes messages.
    pub reply: bool,
}

impl Row {
    /// Its identity and version for the list: what can change in a row of
    /// a thread whose entries never change once written (their text grows
    /// only while it streams in), without hashing whole texts.
    pub fn key(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (&self.id, &self.at, self.depth).hash(&mut hasher);
        match &self.what {
            What::Step(step) => {
                (
                    0u8,
                    &step.author,
                    step.text.len(),
                    step.streaming,
                    step.details.as_ref().map(String::len),
                    step.open,
                    &step.routing,
                    step.reply,
                )
                    .hash(&mut hasher);
                if let Some(d) = &step.directive {
                    (
                        &d.from,
                        &d.to,
                        d.status.as_ref().map(|(s, _)| s),
                        &d.scope,
                        &d.stop,
                    )
                        .hash(&mut hasher);
                }
            }
            What::Fold {
                count,
                open,
                progress,
                latest,
            } => (1u8, count, open, progress, latest).hash(&mut hasher),
            What::Activity { text, open, .. } => (2u8, text.len(), open).hash(&mut hasher),
            What::Summary { intent, state } => (3u8, intent, state).hash(&mut hasher),
        }
        hasher.finish()
    }
}

/// An entry's text as it shows now: the part that arrived while it streams
/// in (`shown` characters), else all of it; and whether it is streaming.
pub fn shown_text(entry: &ThreadEntry, shown: Option<usize>) -> (String, bool) {
    match shown {
        Some(shown) => (
            entry.text.chars().take(shown).collect(),
            shown < entry.text.chars().count(),
        ),
        None => (entry.text.clone(), false),
    }
}

/// Whether the Conversation shows `objective`'s thread (C-54): one started
/// from this conversation that is still going, or that ran in this Studio
/// since it started; in a test instance, the recorded objective it is
/// there to show. Otherwise it shows the objective in one line.
pub fn shows_thread(started_here: bool, active: bool, ran_here: bool, test_instance: bool) -> bool {
    test_instance || started_here && (active || ran_here)
}

/// The objective in one line, at its start among the conversation's
/// entries.
pub fn summary(objective: &Objective, state: &str) -> Row {
    Row {
        id: "thread-summary".into(),
        objective: objective.id.clone(),
        seq: 0,
        at: objective.created.clone(),
        depth: 0,
        what: What::Summary {
            intent: objective.intent.clone(),
            state: state.to_string(),
        },
    }
}

/// A directive as its record says: from whom, to whom, where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct DirectiveChip {
    pub from: String,
    /// A role with its model, or a child objective.
    pub to: String,
    /// `running`, `done`, `failed`, `stopped` or `refused: why`; none until
    /// the record has it.
    pub status: Option<(String, Tone)>,
    /// For a child objective: its focus and budgets.
    pub scope: Option<String>,
    /// A child objective still going, which the Operator may stop alone
    /// (its id).
    pub stop: Option<String>,
}

/// Where an Operator's message went, as its chip reads (C-54: "the thread
/// shows where each message went"): one for the lead waits for its next
/// turn until the objective's record says it was `given`
/// (`Objective.delivered`).
pub fn routing(to: Option<&str>, given: bool) -> String {
    match to {
        Some("implementer") => "to the implementer, at its next tool call".into(),
        Some("lead") if given => "given to the lead".into(),
        Some("lead") => "waits for the lead's next turn".into(),
        Some(role) => format!("to the {role}"),
        None => "sent".into(),
    }
}

/// An author by role only: `you`, `Agentique`, `lead`.
pub fn author(author: &Author) -> String {
    match author {
        Author::Agent { role, .. } => role.clone(),
        other => other.label(),
    }
}

/// The kind of an entry, in words.
fn kind_word(kind: Kind) -> &'static str {
    match kind {
        Kind::Human => "Your message",
        Kind::Directive => "Directive",
        Kind::Result => "Result",
        Kind::Activity => "Tool call",
        Kind::Event | Kind::Unknown => "Event",
    }
}

/// A role and the model the objective recorded for it.
fn role_with_model(objective: &Objective, role: &str) -> String {
    match objective.models.iter().find(|m| m.role == role) {
        Some(model) => format!("{role} · {}", model.model.model),
        None => role.to_string(),
    }
}

/// `text`'s first `most` characters, with an ellipsis when cut.
fn short(text: &str, most: usize) -> String {
    let mut cut: String = text.chars().take(most).collect();
    if text.chars().count() > most {
        cut.push('…');
    }
    cut
}

/// What the directive `entry` is, from its objective's record.
fn directive_chip(
    entry: &ThreadEntry,
    objective: &Objective,
    children: &[Objective],
) -> DirectiveChip {
    let record = entry
        .directive
        .as_deref()
        .and_then(|id| objective.directive(id));
    let from = author(&entry.author);
    let Some(record) = record else {
        return DirectiveChip {
            from,
            to: "another agent".into(),
            status: None,
            scope: None,
            stop: None,
        };
    };
    let to = match &record.recipient {
        Recipient::Role(role) => role_with_model(objective, role),
        Recipient::Child(id) => match children.iter().find(|c| &c.id == id) {
            Some(child) => format!("a child objective: {}", short(&child.intent, 60)),
            None => "a child objective".into(),
        },
    };
    let status = match &record.status {
        DirectiveStatus::Running => ("running".to_string(), Tone::Info),
        DirectiveStatus::Done => ("done".into(), Tone::Success),
        DirectiveStatus::Failed => ("failed".into(), Tone::Warning),
        DirectiveStatus::Stopped => ("stopped".into(), Tone::Neutral),
        DirectiveStatus::Refused { reason } => (format!("refused: {reason}"), Tone::Danger),
    };
    // A child still going: the Operator may stop it alone.
    let stop = match &record.recipient {
        Recipient::Child(id) if record.status == DirectiveStatus::Running => children
            .iter()
            .find(|c| &c.id == id && c.active())
            .map(|c| c.id.clone()),
        _ => None,
    };
    let scope = match &record.recipient {
        Recipient::Child(_) => {
            let mut parts = Vec::new();
            if let Some(focus) = &record.scope.focus {
                parts.push(format!("focus: {focus}"));
            }
            if let Some(budgets) = &record.scope.budgets {
                parts.push(format!(
                    "{}, {}, {}",
                    budgets
                        .usd
                        .map_or_else(|| "no spend limit".into(), |usd| format!("${usd:.2}")),
                    crate::conversation::plural(
                        budgets.cycles as usize,
                        "improvement",
                        "improvements"
                    ),
                    crate::conversation::plural(budgets.steps as usize, "step", "steps")
                ));
            }
            (!parts.is_empty()).then(|| parts.join(" · "))
        }
        Recipient::Role(_) => None,
    };
    DirectiveChip {
        from,
        to,
        status: Some(status),
        scope,
        stop,
    }
}

/// Works out the rows of one objective's thread and its children's.
struct Builder<'a> {
    children: &'a [Objective],
    threads: &'a BTreeMap<String, Vec<ThreadEntry>>,
    open: &'a HashSet<String>,
    shown: &'a dyn Fn(&ThreadEntry) -> Option<usize>,
    /// The objective takes messages: its directives offer Reply.
    replies: bool,
    rows: Vec<Row>,
    nested: HashSet<String>,
    /// Entries that could not be kept, counted for their ids.
    unkept: usize,
}

impl Builder<'_> {
    fn objective(&mut self, objective: &Objective, depth: u8, at: Option<&str>) {
        let entries = self
            .threads
            .get(&objective.id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        // Each step's activity: under the step it names, or else (a thread
        // written before steps were named) the entry before it that is not
        // activity; 0 for activity before any step.
        let steps: HashSet<u64> = entries
            .iter()
            .filter(|e| e.kind != Kind::Activity)
            .map(|e| e.seq)
            .collect();
        let mut under: BTreeMap<u64, Vec<&ThreadEntry>> = BTreeMap::new();
        let mut last_step = 0;
        for entry in entries {
            if entry.kind == Kind::Activity {
                let step = entry
                    .under
                    .filter(|s| steps.contains(s))
                    .unwrap_or(last_step);
                under.entry(step).or_default().push(entry);
            } else {
                last_step = entry.seq;
            }
        }
        let last = entries.last().map(|e| e.seq);
        if let Some(activity) = under.get(&0) {
            let at = at.unwrap_or_else(|| activity[0].at.as_str()).to_string();
            self.fold(&objective.id, 0, &at, depth, activity, last);
        }
        for entry in entries.iter().filter(|e| e.kind != Kind::Activity) {
            let at = at.unwrap_or(&entry.at).to_string();
            self.step(objective, entry, &at, depth);
            if entry.seq != 0
                && let Some(activity) = under.get(&entry.seq)
            {
                self.fold(&objective.id, entry.seq, &at, depth, activity, last);
            }
            // A child objective's thread, under the directive that started it.
            let child = entry
                .directive
                .as_deref()
                .filter(|_| entry.kind == Kind::Directive)
                .and_then(|id| objective.directive(id))
                .and_then(|d| match &d.recipient {
                    Recipient::Child(id) => Some(id.clone()),
                    Recipient::Role(_) => None,
                });
            if let Some(child) = child.and_then(|id| self.children.iter().find(|c| c.id == id))
                && self.nested.insert(child.id.clone())
            {
                self.objective(child, depth + 1, Some(&at));
            }
        }
    }

    fn step(&mut self, objective: &Objective, entry: &ThreadEntry, at: &str, depth: u8) {
        // An entry that could not be kept has no number: its id is its own.
        let id = if entry.seq == 0 {
            self.unkept += 1;
            format!("thread-{}-unkept{}", entry.objective, self.unkept)
        } else {
            format!("thread-{}-{}", entry.objective, entry.seq)
        };
        let (text, streaming) = shown_text(entry, (self.shown)(entry));
        // The intent is the thread's first entry, whatever is kept of it.
        let intent = entry.seq == 1;
        let routing = (entry.kind == Kind::Human && entry.author == Author::Operator).then(|| {
            if intent {
                "the objective's intent".to_string()
            } else {
                routing(
                    entry.to.as_deref(),
                    entry.seq != 0 && entry.seq <= objective.delivered,
                )
            }
        });
        let open = self.open.contains(&id);
        self.rows.push(Row {
            id,
            objective: entry.objective.clone(),
            seq: entry.seq,
            at: at.to_string(),
            depth,
            what: What::Step(Step {
                kind: entry.kind,
                author: entry.author.label(),
                text,
                streaming,
                details: entry.details.clone(),
                open,
                directive: (entry.kind == Kind::Directive)
                    .then(|| directive_chip(entry, objective, self.children)),
                routing,
                reply: self.replies && depth == 0 && entry.kind == Kind::Directive,
            }),
        });
    }

    /// `last`: the number of the thread's last entry.
    fn fold(
        &mut self,
        objective: &str,
        step: u64,
        at: &str,
        depth: u8,
        activity: &[&ThreadEntry],
        last: Option<u64>,
    ) {
        let id = format!("thread-fold-{objective}-{step}");
        let open = self.open.contains(&id);
        let latest = activity
            .last()
            .filter(|e| e.seq != 0 && Some(e.seq) == last)
            .map(|e| e.text.clone());
        self.rows.push(Row {
            id,
            objective: objective.to_string(),
            seq: step,
            at: at.to_string(),
            depth,
            what: What::Fold {
                count: activity.len(),
                open,
                progress: activity.iter().all(|e| is_progress(e)),
                latest,
            },
        });
        if !open {
            return;
        }
        for entry in activity {
            let id = format!("thread-{}-{}", entry.objective, entry.seq);
            let open = self.open.contains(&id);
            self.rows.push(Row {
                id,
                objective: objective.to_string(),
                seq: entry.seq,
                at: at.to_string(),
                depth,
                what: What::Activity {
                    author: entry.author.label(),
                    text: entry.text.clone(),
                    details: entry.details.clone(),
                    open,
                    progress: is_progress(entry),
                },
            });
        }
    }
}

/// Whether an activity entry is an exploration run's update (the explorer
/// answers one typed question a step: it makes no tool calls).
fn is_progress(entry: &ThreadEntry) -> bool {
    matches!(&entry.author, Author::Agent { role, .. } if role == "explorer")
}

/// How many tool calls, or updates of a run, a fold holds: `12 tool calls`.
fn fold_noun(count: usize, progress: bool) -> String {
    if progress {
        crate::conversation::plural(count, "update", "updates")
    } else {
        crate::conversation::plural(count, "tool call", "tool calls")
    }
}

/// The rows of `root`'s thread (an objective the Operator started) and its
/// `children`'s, each child's nested under the directive that started it
/// (one with no such directive shown follows the rest); `open` names the
/// rows expanded, `shown` how many characters of an entry show while it
/// streams in, and `replies` whether the objective still takes messages.
pub fn rows(
    root: &Objective,
    children: &[Objective],
    threads: &BTreeMap<String, Vec<ThreadEntry>>,
    open: &HashSet<String>,
    shown: &dyn Fn(&ThreadEntry) -> Option<usize>,
    replies: bool,
) -> Vec<Row> {
    let mut builder = Builder {
        children,
        threads,
        open,
        shown,
        replies,
        rows: Vec::new(),
        nested: HashSet::new(),
        unkept: 0,
    };
    builder.objective(root, 0, None);
    for child in children {
        let parent_shown = child
            .parent
            .as_deref()
            .is_some_and(|p| p == root.id || builder.nested.contains(p));
        if parent_shown && !builder.nested.contains(&child.id) {
            builder.nested.insert(child.id.clone());
            builder.objective(child, child.depth.max(1), None);
        }
    }
    builder.rows
}

/// Where the conversation's entries and a thread's rows go, in time order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Entry(usize),
    Row(usize),
}

/// The conversation's entries (`times`: when each was added, when known)
/// and a thread's rows (`ats`: their times, in order) in time order: a row
/// goes before the first entry added after it; rows never go before an
/// entry whose time is not known (one an earlier build added), and those
/// left go after the last entry.
pub fn merge(times: &[Option<&str>], ats: &[&str]) -> Vec<Slot> {
    let mut slots = Vec::with_capacity(times.len() + ats.len());
    let mut next = 0;
    for (index, time) in times.iter().enumerate() {
        if let Some(time) = time {
            while next < ats.len() && ats[next] <= *time {
                slots.push(Slot::Row(next));
                next += 1;
            }
        }
        slots.push(Slot::Entry(index));
    }
    slots.extend((next..ats.len()).map(Slot::Row));
    slots
}

/// A row as an agent reads it in a test instance: what it is, who wrote
/// it, and the start of its text.
pub fn label(row: &Row) -> String {
    match &row.what {
        What::Step(step) => {
            let head = match (&step.directive, &step.routing) {
                (Some(d), _) => {
                    let status = d
                        .status
                        .as_ref()
                        .map(|(s, _)| format!(", {s}"))
                        .unwrap_or_default();
                    format!("Directive from {} to {}{status}", d.from, d.to)
                }
                (None, Some(routing)) => format!("Your message ({routing})"),
                (None, None) => format!("{} from {}", kind_word(step.kind), step.author),
            };
            super::control_label(format!("{head}: {}", step.text))
        }
        What::Fold {
            count,
            progress,
            latest,
            ..
        } => fold_words(*count, *progress, latest.as_deref(), row.seq),
        What::Activity {
            author,
            text,
            progress,
            ..
        } => super::control_label(format!(
            "{} by {author}: {text}",
            if *progress { "Update" } else { "Tool call" }
        )),
        What::Summary { intent, state } => super::control_label(format!(
            "Objective “{intent}”, {state}: its record is in the Objectives panel"
        )),
    }
}

/// `12 tool calls under entry 4`; `9 updates under entry 6, the latest:
/// Step 4/30: …` while the latest is the thread's.
fn fold_words(count: usize, progress: bool, latest: Option<&str>, step: u64) -> String {
    let calls = fold_noun(count, progress);
    let words = if step == 0 {
        format!("{calls} before the first step")
    } else {
        format!("{calls} under entry {step}")
    };
    match latest {
        Some(latest) => super::control_label(format!("{words}, the latest: {latest}")),
        None => words,
    }
}

/// A thread row's control id read back: whether it is a fold, its
/// objective and its entry's number.
pub fn parse_id(id: &str) -> Option<(bool, &str, u64)> {
    let rest = id.strip_prefix("thread-")?;
    if rest.starts_with("reply-") || rest == "summary" {
        return None;
    }
    let (fold, rest) = match rest.strip_prefix("fold-") {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let (objective, seq) = rest.rsplit_once('-')?;
    Some((fold, objective, seq.parse().ok()?))
}

/// How an observation in the Operator's own window names a control of the
/// Conversation by its id (`thread-…`), if it is a thread's row: what it
/// is and who wrote it, never its text. `entry` finds an entry shown.
pub fn private_name<'a>(
    id: &str,
    entry: impl Fn(&str, u64) -> Option<&'a ThreadEntry>,
) -> Option<String> {
    let rest = id.strip_prefix("thread-")?;
    if rest.starts_with("reply-") {
        // A fixed label, "Reply".
        return None;
    }
    if rest == "summary" {
        return Some("The objective shown, in a line".into());
    }
    Some(match parse_id(id) {
        Some((fold, objective, seq)) => private_label(entry(objective, seq), fold, seq),
        None => "An entry of the objective's thread".into(),
    })
}

/// How an observation in the Operator's own window names a thread's row:
/// what it is and who wrote it, never its text.
pub fn private_label(entry: Option<&ThreadEntry>, fold: bool, seq: u64) -> String {
    if fold {
        return if seq == 0 {
            "Tool calls before the first step".into()
        } else {
            format!("Tool calls under entry {seq}")
        };
    }
    match entry {
        Some(entry) if entry.kind == Kind::Human => {
            format!("Your message, entry {seq} of the objective's thread")
        }
        Some(entry) => format!(
            "{}, entry {seq}, by {}",
            kind_word(entry.kind),
            author(&entry.author)
        ),
        None => format!("Entry {seq} of the objective's thread"),
    }
}

/// Opens or closes a row.
fn toggle(
    ctx: &Rc<Ctx>,
    key: String,
) -> impl Fn(&ClickEvent, &mut gpui::Window, &mut App) + 'static {
    let view = ctx.view.clone();
    move |_: &ClickEvent, _, cx| {
        let key = key.clone();
        view.update(cx, |view, cx| view.toggle(key, cx));
    }
}

/// A row as it is drawn.
pub fn render_row(ctx: &Rc<Ctx>, row: &Row, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    let control = || {
        let open = match &row.what {
            What::Step(step) => step.open,
            What::Fold { open, .. } | What::Activity { open, .. } => *open,
            What::Summary { .. } => false,
        };
        ui::target::control(
            ui::target::Control::new("item", label(row))
                .id(row.id.clone())
                .selected(open),
        )
    };
    let body = match &row.what {
        What::Step(step) => step_card(ctx, row, step, control(), cx),
        What::Summary { intent, state } => {
            let studio = ctx.studio.clone();
            div()
                .id(SharedString::from(row.id.clone()))
                .relative()
                .child(control())
                .flex()
                .items_center()
                .gap(r(6.0))
                .px(r(10.0))
                .py(r(6.0))
                .rounded(r(crate::tokens::radius::CARD))
                .border_1()
                .border_color(theme.separator)
                .text_size(r(theme::text::SM))
                .child(icon(IconName::Agent).size(13.0).color(theme.text_faint))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_color(theme.text_secondary)
                        .child(format!("Objective “{intent}” · {state}")),
                )
                .child(
                    Button::new("show-objectives", "Objectives panel")
                        .small()
                        .ghost()
                        .tooltip("Its record, children and latest steps", None)
                        .on_click(move |_: &ClickEvent, _, cx| {
                            studio.act(cx, |s| {
                                s.panel = crate::studio::Panel::Objectives;
                                s.inspector_hidden = false;
                                s.panels_hidden = false;
                                s.mark(crate::studio::Dirty::LAYOUT);
                            })
                        }),
                )
                .into_any_element()
        }
        What::Fold {
            count,
            open,
            progress,
            latest,
        } => div()
            .id(SharedString::from(row.id.clone()))
            .relative()
            .child(control())
            .flex()
            .items_center()
            .gap(r(6.0))
            .ml(r(12.0))
            .min_w_0()
            .text_size(r(theme::text::XS))
            .text_color(theme.text_muted)
            .cursor_pointer()
            .hover(|style| style.text_color(theme.text_secondary))
            .on_click(toggle(ctx, row.id.clone()))
            .child(
                icon(if *open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size(12.0)
                .color(theme.text_faint),
            )
            .child(
                icon(if *progress {
                    IconName::Play
                } else {
                    IconName::Tool
                })
                .size(12.0)
                .color(theme.text_faint),
            )
            .child(div().flex_none().child(fold_noun(*count, *progress)))
            .when_some(latest.clone().filter(|_| !*open), |this, latest| {
                this.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_color(theme.text_secondary)
                        .child(format!("· {latest}")),
                )
            })
            .into_any_element(),
        What::Activity {
            author,
            text,
            details,
            open,
            ..
        } => div()
            .ml(r(30.0))
            .flex()
            .flex_col()
            .gap(r(4.0))
            .child(
                div()
                    .id(SharedString::from(row.id.clone()))
                    .relative()
                    .child(control())
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_secondary)
                    .when(details.is_some(), |this| {
                        this.cursor_pointer()
                            .hover(|style| style.text_color(theme.text))
                            .on_click(toggle(ctx, row.id.clone()))
                    })
                    .child(
                        icon(if *open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(11.0)
                        .color(if details.is_some() {
                            theme.text_faint
                        } else {
                            theme.chrome
                        }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .font_family(theme::MONO)
                            .child(text.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_color(theme.text_faint)
                            .child(author.clone()),
                    ),
            )
            .when_some(details.clone().filter(|_| *open), |this, details| {
                this.child(diff(&details, cx))
            })
            .into_any_element(),
    };
    div()
        .pl(r(18.0 * f32::from(row.depth)))
        .child(
            div()
                .when(row.depth > 0, |this| {
                    this.pl(r(10.0)).border_l_2().border_color(theme.separator)
                })
                .child(body),
        )
        .into_any_element()
}

/// A tool call's details: a diff's removed lines and added lines marked,
/// or a command line, in a mono block.
fn diff(details: &str, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .p(r(8.0))
        .rounded(r(crate::tokens::radius::CONTROL))
        .bg(theme.inset)
        .border_1()
        .border_color(theme.separator)
        .font_family(theme::MONO)
        .text_size(r(theme::text::XS))
        .line_height(r(16.0))
        .flex()
        .flex_col()
        .children(details.lines().map(|line| {
            let colour = if line.starts_with("+ ") || line == "+" {
                theme.success.text
            } else if line.starts_with("- ") || line == "-" {
                theme.danger.text
            } else {
                theme.text_secondary
            };
            div().text_color(colour).child(if line.is_empty() {
                " ".to_string()
            } else {
                line.to_string()
            })
        }))
        .into_any_element()
}

/// A step: the Operator's message, a directive, a result or an event.
fn step_card(ctx: &Rc<Ctx>, row: &Row, step: &Step, control: AnyElement, cx: &App) -> AnyElement {
    let theme = cx.theme().clone();
    let details_toggle = step.details.as_ref().map(|_| {
        Button::new(
            SharedString::from(format!("{}-details", row.id)),
            if step.open { "Hide details" } else { "Details" },
        )
        .small()
        .ghost()
        .icon(if step.open {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        })
        .on_click(toggle(ctx, row.id.clone()))
    });
    let details = step
        .details
        .clone()
        .filter(|_| step.open)
        .map(|details| diff(&details, cx));
    let text = |size: f32| {
        div()
            .text_size(r(size))
            .line_height(r(size + 7.0))
            .text_color(theme.text)
            .child(step.text.clone())
            .when(step.streaming, |this| this.text_color(theme.text_secondary))
    };
    // Reply on the objective's own directives; the bar's "Write to it"
    // serves the rest.
    let reply = step.reply.then(|| {
        let studio = ctx.studio.clone();
        Button::new(
            SharedString::from(format!("thread-reply-{}-{}", row.objective, row.seq)),
            "Reply",
        )
        .small()
        .ghost()
        .icon(IconName::Enter)
        .tooltip(
            "Write to the objective's agents: your message goes into its thread",
            None,
        )
        .on_click(move |_: &ClickEvent, _, cx| studio.act(cx, |s| s.address_objective()))
    });
    // The Operator's own: refused to agents like every `objective-` control.
    let stop = step
        .directive
        .as_ref()
        .and_then(|d| d.stop.clone())
        .map(|child| {
            let studio = ctx.studio.clone();
            Button::new(
                SharedString::from(format!("objective-stop-child-{child}")),
                "Stop this child",
            )
            .small()
            .danger()
            .tooltip(
                "Ends this child objective alone; the lead goes on with that",
                None,
            )
            .on_click(move |_: &ClickEvent, _, cx| {
                let child = child.clone();
                studio.act(cx, move |s| s.stop_child(&child))
            })
        });
    match step.kind {
        Kind::Human => div()
            .id(SharedString::from(row.id.clone()))
            .relative()
            .child(control)
            .flex()
            .flex_col()
            .items_end()
            .gap(r(4.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(r(6.0))
                    .text_size(r(theme::text::XS))
                    .text_color(theme.text_muted)
                    // The Operator's message, or a child's intent from the
                    // agent that asked for it.
                    .child(if step.routing.is_some() {
                        "you → the objective".to_string()
                    } else {
                        format!("{} → the objective", step.author)
                    })
                    .when_some(step.routing.clone(), |this, routing| {
                        this.child(ui::Chip::new(routing).icon(IconName::ArrowRight))
                    }),
            )
            .child(
                div()
                    .max_w(gpui::relative(0.92))
                    .px(r(12.0))
                    .py(r(9.0))
                    .rounded(r(crate::tokens::radius::CARD + 4.0))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.accent.border.opacity(0.6))
                    .child(text(theme::text::PROSE)),
            )
            .into_any_element(),
        Kind::Directive | Kind::Result => {
            let directive = step.directive.clone();
            let head = match &directive {
                Some(d) => format!("{} → {}", step.author, d.to),
                None => format!("Result · {}", step.author),
            };
            div()
                .flex()
                .flex_col()
                .gap(r(6.0))
                .px(r(10.0))
                .py(r(8.0))
                .rounded(r(crate::tokens::radius::CARD))
                .bg(theme.raised)
                .border_1()
                .border_color(theme.border)
                .when(directive.is_some(), |this| {
                    this.border_l_2().border_color(theme.accent.border)
                })
                .child(
                    div()
                        .id(SharedString::from(row.id.clone()))
                        .relative()
                        .child(control)
                        .flex()
                        .items_center()
                        .gap(r(6.0))
                        .when(step.details.is_some(), |this| {
                            this.cursor_pointer().on_click(toggle(ctx, row.id.clone()))
                        })
                        .child(
                            icon(if directive.is_some() {
                                IconName::Agent
                            } else {
                                IconName::CircleCheck
                            })
                            .size(13.0)
                            .color(theme.text_faint),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .text_size(r(theme::text::XS))
                                .font_weight(theme::MEDIUM)
                                .text_color(theme.text_secondary)
                                .child(head),
                        )
                        .when_some(
                            directive.as_ref().and_then(|d| d.status.clone()),
                            |this, (status, tone)| this.child(ui::Chip::new(status).tone(tone)),
                        ),
                )
                .child(text(theme::text::BASE))
                .when_some(directive.and_then(|d| d.scope), |this, scope| {
                    this.child(
                        div()
                            .text_size(r(theme::text::XS))
                            .text_color(theme.text_muted)
                            .child(scope),
                    )
                })
                .when(
                    details_toggle.is_some() || reply.is_some() || stop.is_some(),
                    |this| {
                        this.child(
                            div()
                                .flex()
                                .gap(r(4.0))
                                .children(details_toggle)
                                .child(div().flex_1())
                                .children(stop)
                                .children(reply),
                        )
                    },
                )
                .children(details)
                .into_any_element()
        }
        Kind::Event | Kind::Activity | Kind::Unknown => div()
            .flex()
            .flex_col()
            .gap(r(4.0))
            .child(
                div()
                    .id(SharedString::from(row.id.clone()))
                    .relative()
                    .child(control)
                    .flex()
                    .items_start()
                    .gap(r(6.0))
                    .text_size(r(theme::text::SM))
                    .line_height(r(18.0))
                    .when(step.details.is_some(), |this| {
                        this.cursor_pointer().on_click(toggle(ctx, row.id.clone()))
                    })
                    .child(
                        div()
                            .pt(r(3.0))
                            .child(icon(IconName::CircleDot).size(11.0).color(theme.text_faint)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_color(theme.text_faint)
                            .child(step.author.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(theme.text_muted)
                            .child(step.text.clone()),
                    ),
            )
            .children(details)
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_orchestrator::record::{Access, Budgets, Permissions, RoleModel, Scope, Store};
    use agq_providers::{ModelRef, Provider};

    /// An objective record in a folder of its own (removed when dropped).
    struct Records {
        dir: std::path::PathBuf,
        store: Store,
    }

    impl Records {
        fn new(name: &str) -> Records {
            let dir =
                std::env::temp_dir().join(format!("agq-thread-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let store = Store::new(&dir);
            Records { dir, store }
        }

        fn objective(&self, id: &str, parent: Option<&str>) -> Objective {
            let mut objective = self
                .store
                .create(
                    "Find and fix problems in the Library panel",
                    std::path::Path::new("C:/agentique"),
                    "main",
                    Budgets::default(),
                    Permissions::default(),
                )
                .unwrap();
            objective.id = id.into();
            objective.parent = parent.map(str::to_string);
            objective.depth = u8::from(parent.is_some());
            objective
        }
    }

    impl Drop for Records {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn scope(instruction: &str) -> Scope {
        Scope {
            instruction: instruction.into(),
            focus: None,
            budgets: None,
            permissions: None,
        }
    }

    fn sonnet() -> ModelRef {
        ModelRef::new(Provider::Anthropic, "claude-sonnet-5-5")
    }

    fn entry(
        objective: &str,
        seq: u64,
        at: u32,
        kind: Kind,
        author: Author,
        text: &str,
    ) -> ThreadEntry {
        let mut entry = ThreadEntry::new(kind, author, text);
        entry.seq = seq;
        entry.objective = objective.into();
        entry.at = format!("2026-10-04T10:{:02}:{:02}Z", at / 60, at % 60);
        entry
    }

    fn steps(rows: &[Row]) -> Vec<(u64, &'static str, u8)> {
        rows.iter()
            .map(|r| {
                let what = match &r.what {
                    What::Step(_) => "step",
                    What::Fold { .. } => "fold",
                    What::Activity { .. } => "activity",
                    What::Summary { .. } => "summary",
                };
                (r.seq, what, r.depth)
            })
            .collect()
    }

    fn step(row: &Row) -> &Step {
        match &row.what {
            What::Step(step) => step,
            other => panic!("not a step: {other:?}"),
        }
    }

    /// C-54: the Operator's messages, directives, results and events are
    /// told apart, each with its author's role and model; a directive says
    /// whom it is for and where it stands, from the record; a message where
    /// it went; tool activity folds under its step (named, or in an older
    /// thread the entry before it), its diff one click further.
    #[test]
    fn entries_are_told_apart_and_activity_folds_under_its_step() {
        let records = Records::new("kinds");
        let mut root = records.objective("objective-1", None);
        root.models = vec![RoleModel {
            role: "implementer".into(),
            model: sonnet(),
            effort: None,
            access: Access::Subscription,
            configured: sonnet(),
            fallback: None,
            credential: "CLAUDE_CODE_OAUTH_TOKEN".into(),
            billed: "your Claude plan".into(),
        }];
        let d1 = root.direct(
            "lead",
            Recipient::Role("implementer".into()),
            scope("Implement the label fix"),
            Some("cycle-1/proposal".into()),
        );
        let lead = Author::agent(
            "lead",
            Some(ModelRef::new(Provider::Anthropic, "claude-opus-5-5")),
        );
        let implementer = Author::agent("implementer", Some(sonnet()));
        let o = "objective-1";
        let mut edit = entry(
            o,
            4,
            4,
            Kind::Activity,
            implementer.clone(),
            "Edit src/a.rs",
        )
        .with_details("- let a = 1;\n+ let a = 2;");
        edit.under = Some(3);
        let mut read = entry(
            o,
            5,
            5,
            Kind::Activity,
            implementer.clone(),
            "Read src/b.rs",
        );
        read.under = Some(3);
        let mut message = entry(o, 6, 6, Kind::Human, Author::Operator, "Keep it small");
        message.to = Some("implementer".into());
        let thread = vec![
            entry(
                o,
                1,
                1,
                Kind::Human,
                Author::Operator,
                "Find and fix problems",
            ),
            entry(
                o,
                2,
                2,
                Kind::Event,
                Author::Agentique,
                "Cycle 1: Proposing an improvement",
            ),
            entry(o, 3, 3, Kind::Directive, lead, "Implement the label fix")
                .with_details("Title: Label the fields")
                .for_directive(Some(&d1)),
            edit,
            read,
            message,
            entry(o, 7, 7, Kind::Result, implementer.clone(), "Implemented"),
            // Written before steps were named: under the entry before it.
            entry(o, 8, 8, Kind::Activity, implementer, "Bash: cargo test"),
            entry(
                o,
                9,
                9,
                Kind::Unknown,
                Author::Unknown,
                "A later build's entry",
            ),
        ];
        let threads = BTreeMap::from([(o.to_string(), thread)]);
        let none = |_: &ThreadEntry| None;
        let closed = rows(&root, &[], &threads, &HashSet::new(), &none, true);
        assert_eq!(
            steps(&closed),
            vec![
                (1, "step", 0),
                (2, "step", 0),
                (3, "step", 0),
                (3, "fold", 0),
                (6, "step", 0),
                (7, "step", 0),
                (7, "fold", 0),
                (9, "step", 0),
            ]
        );
        assert_eq!(
            closed[3].what,
            What::Fold {
                count: 2,
                open: false,
                progress: false,
                latest: None,
            }
        );
        assert_eq!(closed[3].id, "thread-fold-objective-1-3");
        // Kinds and authors.
        assert_eq!(
            step(&closed[0]).routing.as_deref(),
            Some("the objective's intent")
        );
        assert_eq!(step(&closed[0]).author, "you");
        assert_eq!(step(&closed[1]).author, "Agentique");
        let directive = step(&closed[2]);
        assert_eq!(directive.author, "lead · claude-opus-5-5");
        assert_eq!(
            directive.directive,
            Some(DirectiveChip {
                from: "lead".into(),
                to: "implementer · claude-sonnet-5-5".into(),
                status: Some(("running".into(), Tone::Info)),
                scope: None,
                stop: None,
            })
        );
        assert_eq!(
            step(&closed[4]).routing.as_deref(),
            Some("to the implementer, at its next tool call")
        );
        assert_eq!(step(&closed[5]).kind, Kind::Result);
        assert_eq!(step(&closed[5]).author, "implementer · claude-sonnet-5-5");
        // A later build's author is no one's (never credited to Agentique).
        assert_eq!(step(&closed[7]).author, "unknown author");
        // The fold opened, and one tool call in it: its diff.
        let open = HashSet::from([
            "thread-fold-objective-1-3".to_string(),
            "thread-objective-1-4".to_string(),
            "thread-objective-1-3".to_string(),
        ]);
        let opened = rows(&root, &[], &threads, &open, &none, true);
        assert_eq!(
            steps(&opened)[3..6],
            [(3, "fold", 0), (4, "activity", 0), (5, "activity", 0)]
        );
        assert!(step(&opened[2]).open, "the directive's details");
        assert_eq!(
            opened[4].what,
            What::Activity {
                author: "implementer · claude-sonnet-5-5".into(),
                text: "Edit src/a.rs".into(),
                details: Some("- let a = 1;\n+ let a = 2;".into()),
                open: true,
                progress: false,
            }
        );
        // The status follows the record.
        root.settle(&d1, DirectiveStatus::Done, Some("Implemented".into()));
        let done = rows(&root, &[], &threads, &HashSet::new(), &none, true);
        assert_eq!(
            step(&done[2]).directive.as_ref().unwrap().status,
            Some(("done".into(), Tone::Success))
        );
        // A directive streaming in shows the part that arrived.
        let partly = |e: &ThreadEntry| (e.seq == 3).then_some(9);
        let streaming = rows(&root, &[], &threads, &HashSet::new(), &partly, true);
        assert_eq!(step(&streaming[2]).text, "Implement");
        assert!(step(&streaming[2]).streaming);
        assert!(!step(&streaming[1]).streaming);
        // A row's key follows what shows: the text arriving, a fold
        // opening; the same row keeps its key.
        assert_ne!(streaming[2].key(), closed[2].key());
        assert_ne!(opened[3].key(), closed[3].key());
        assert_eq!(
            rows(&root, &[], &threads, &HashSet::new(), &none, true)[2].key(),
            done[2].key()
        );
        // Reply is offered on the objective's directives while it takes
        // messages, never on an ended one's.
        assert!(step(&closed[2]).reply && !step(&closed[0]).reply);
        let ended = rows(&root, &[], &threads, &HashSet::new(), &none, false);
        assert!(!step(&ended[2]).reply);
    }

    /// The intent is the thread's first entry, labelled so even when only
    /// the latest entries are kept; a later message of the Operator's says
    /// where it went; a child's intent is the asking agent's, not "you".
    /// Entries that could not be kept have ids of their own.
    /// The W13.7 repair, after its review: an exploration run's progress
    /// folds as its updates (the explorer makes no tool calls), and while
    /// it goes on the fold's row shows the latest of them; once the run's
    /// result follows, the row is a count again.
    #[test]
    fn a_runs_updates_fold_with_the_latest_shown_while_it_goes_on() {
        let records = Records::new("progress");
        let root = records.objective("objective-1", None);
        let o = "objective-1";
        let explorer = Author::agent(
            "explorer",
            Some(ModelRef::new(Provider::DeepSeek, "deepseek-flash")),
        );
        let update = |seq: u64, text: &str| {
            let mut e = entry(o, seq, seq as u32, Kind::Activity, explorer.clone(), text);
            e.under = Some(2);
            e
        };
        let mut thread = vec![
            entry(
                o,
                1,
                1,
                Kind::Human,
                Author::Operator,
                "Find and fix problems",
            ),
            entry(o, 2, 2, Kind::Event, Author::Agentique, "Explores model"),
            update(
                3,
                "Step 1/30: clicks “History” — ok · rules (the goal names it)",
            ),
            update(4, "Step 2/30: asks deepseek-flash at low · 12 s in"),
            update(5, "Step 2/30: waiting for deepseek-flash at low · 10 s"),
        ];
        let none = |_: &ThreadEntry| None;
        let going = rows(
            &root,
            &[],
            &BTreeMap::from([(o.to_string(), thread.clone())]),
            &HashSet::new(),
            &none,
            true,
        );
        assert_eq!(
            going[2].what,
            What::Fold {
                count: 3,
                open: false,
                progress: true,
                latest: Some("Step 2/30: waiting for deepseek-flash at low · 10 s".into()),
            }
        );
        assert_eq!(
            label(&going[2]),
            "3 updates under entry 2, the latest: Step 2/30: waiting for deepseek-flash at low · 10 s"
        );
        thread.push(entry(
            o,
            6,
            6,
            Kind::Result,
            explorer.clone(),
            "Explored 30 steps",
        ));
        let done = rows(
            &root,
            &[],
            &BTreeMap::from([(o.to_string(), thread)]),
            &HashSet::from(["thread-fold-objective-1-2".to_string()]),
            &none,
            true,
        );
        assert_eq!(label(&done[2]), "3 updates under entry 2");
        assert!(label(&done[3]).starts_with("Update by explorer · deepseek-flash: Step 1/30"));
    }

    #[test]
    fn the_intent_is_the_first_entry_and_unkept_entries_have_their_own_ids() {
        let records = Records::new("intent");
        let root = records.objective("objective-1", None);
        let o = "objective-1";
        let mut later = entry(o, 900, 9, Kind::Human, Author::Operator, "Keep it small");
        later.to = Some("lead".into());
        let tail = BTreeMap::from([(
            o.to_string(),
            vec![
                later,
                entry(o, 0, 10, Kind::Event, Author::Agentique, "not kept"),
                entry(o, 0, 11, Kind::Event, Author::Agentique, "not kept either"),
            ],
        )]);
        let none = |_: &ThreadEntry| None;
        let shown = rows(&root, &[], &tail, &HashSet::new(), &none, true);
        assert_eq!(
            step(&shown[0]).routing.as_deref(),
            Some("waits for the lead's next turn"),
            "not the intent: the first kept message is entry 900"
        );
        // Once the record says the lead was given it (W12.5's delivered).
        let mut given = root.clone();
        given.delivered = 900;
        let shown_after = rows(&given, &[], &tail, &HashSet::new(), &none, true);
        assert_eq!(
            step(&shown_after[0]).routing.as_deref(),
            Some("given to the lead")
        );
        assert_ne!(shown[1].id, shown[2].id);
        assert_eq!(parse_id(&shown[1].id), None);
        assert_eq!(
            private_name(&shown[1].id, |_, _| None).as_deref(),
            Some("An entry of the objective's thread")
        );
        let first = BTreeMap::from([(
            o.to_string(),
            vec![entry(
                o,
                1,
                1,
                Kind::Human,
                Author::Operator,
                "Find problems",
            )],
        )]);
        let shown = rows(&root, &[], &first, &HashSet::new(), &none, true);
        assert_eq!(
            step(&shown[0]).routing.as_deref(),
            Some("the objective's intent")
        );
        let child = records.objective("objective-2", None);
        let asked = BTreeMap::from([(
            "objective-2".to_string(),
            vec![entry(
                "objective-2",
                1,
                1,
                Kind::Human,
                Author::agent("lead", None),
                "Explore History",
            )],
        )]);
        let shown = rows(&child, &[], &asked, &HashSet::new(), &none, true);
        assert_eq!(
            step(&shown[0]).routing,
            None,
            "the lead's, not the Operator's"
        );
        assert_eq!(step(&shown[0]).author, "lead");
    }

    /// C-54: a conversation shows the thread of an objective started from
    /// it while it goes on, or once it ended if it ran in this Studio;
    /// otherwise the objective in one line. A test instance shows the
    /// recorded objective it is there for.
    #[test]
    fn a_conversation_shows_the_threads_started_from_it() {
        // Started here: going on, or ran in this session.
        assert!(shows_thread(true, true, false, false));
        assert!(shows_thread(true, false, true, false));
        // Started here, ended in an earlier session: one line.
        assert!(!shows_thread(true, false, false, false));
        // Started elsewhere (another project, an earlier conversation).
        assert!(!shows_thread(false, true, true, false));
        assert!(shows_thread(false, false, false, true));
        let records = Records::new("summary");
        let root = records.objective("objective-1", None);
        let line = summary(&root, "done");
        assert_eq!(line.id, "thread-summary");
        assert_eq!(line.at, root.created);
        assert_eq!(
            label(&line),
            "Objective “Find and fix problems in the Library panel”, done: its record is in the Objectives panel"
        );
        assert_eq!(
            private_name("thread-summary", |_, _| None).as_deref(),
            Some("The objective shown, in a line")
        );
        assert_eq!(
            private_name("thread-reply-objective-1-3", |_, _| None),
            None
        );
        assert_eq!(private_name("send", |_, _| None), None);
    }

    /// C-54: a child objective's thread is nested under the directive that
    /// started it, in its time; its scope (focus, budgets) and status are
    /// the record's. One whose directive is not shown follows the rest.
    #[test]
    fn a_childs_thread_is_nested_under_its_directive() {
        let records = Records::new("children");
        let mut root = records.objective("objective-1", None);
        let child = records.objective("objective-2", Some("objective-1"));
        let orphan = records.objective("objective-3", Some("objective-1"));
        let mut explore = scope("Explore the History panel");
        explore.focus = Some("History".into());
        explore.budgets = Some(Budgets {
            usd: Some(0.5),
            ..Budgets::default()
        });
        let d1 = root.direct(
            "lead",
            Recipient::Child("objective-2".into()),
            explore,
            None,
        );
        let lead = Author::agent("lead", None);
        let explorer = Author::agent("explorer", None);
        let threads = BTreeMap::from([
            (
                "objective-1".to_string(),
                vec![
                    entry(
                        "objective-1",
                        1,
                        1,
                        Kind::Human,
                        Author::Operator,
                        "Find problems",
                    ),
                    entry(
                        "objective-1",
                        2,
                        2,
                        Kind::Directive,
                        lead,
                        "Explore the History panel",
                    )
                    .for_directive(Some(&d1)),
                    entry(
                        "objective-1",
                        3,
                        50,
                        Kind::Event,
                        Author::Agentique,
                        "The child returned",
                    ),
                ],
            ),
            (
                "objective-2".to_string(),
                vec![
                    entry(
                        "objective-2",
                        1,
                        3,
                        Kind::Event,
                        Author::Agentique,
                        "Exploring",
                    ),
                    entry("objective-2", 2, 40, Kind::Result, explorer, "One finding"),
                ],
            ),
            (
                "objective-3".to_string(),
                vec![entry(
                    "objective-3",
                    1,
                    60,
                    Kind::Event,
                    Author::Agentique,
                    "Started",
                )],
            ),
        ]);
        let none = |_: &ThreadEntry| None;
        let shown = rows(
            &root,
            &[child.clone(), orphan],
            &threads,
            &HashSet::new(),
            &none,
            true,
        );
        let order: Vec<(&str, u64, u8)> = shown
            .iter()
            .map(|r| (r.objective.as_str(), r.seq, r.depth))
            .collect();
        assert_eq!(
            order,
            vec![
                ("objective-1", 1, 0),
                ("objective-1", 2, 0),
                ("objective-2", 1, 1),
                ("objective-2", 2, 1),
                ("objective-1", 3, 0),
                ("objective-3", 1, 1),
            ]
        );
        // Nested rows keep their directive's time, so they stay with it.
        assert_eq!(shown[2].at, shown[1].at);
        assert_eq!(shown[3].at, shown[1].at);
        let chip = step(&shown[1]).directive.clone().unwrap();
        assert_eq!(
            chip.to,
            "a child objective: Find and fix problems in the Library panel"
        );
        assert_eq!(
            chip.scope.as_deref(),
            Some("focus: History · $0.50, 1 improvement, 20 steps")
        );
        // A child still going can be stopped alone, by the Operator (its
        // control's id is an `objective-` one, refused to agents).
        assert_eq!(chip.stop.as_deref(), Some("objective-2"));
        root.settle(
            &d1,
            DirectiveStatus::Refused {
                reason: "over the parent's budget".into(),
            },
            None,
        );
        let refused = rows(&root, &[child], &threads, &HashSet::new(), &none, true);
        assert_eq!(
            step(&refused[1]).directive.as_ref().unwrap().status,
            Some(("refused: over the parent's budget".into(), Tone::Danger))
        );
        assert_eq!(
            step(&refused[1]).directive.as_ref().unwrap().stop,
            None,
            "only a running child's directive offers Stop"
        );
    }

    /// The thread goes among the conversation's entries in time order;
    /// never before an entry whose time is not known.
    #[test]
    fn rows_go_among_the_conversations_entries_in_time_order() {
        let t = |s: u32| format!("2026-10-04T10:00:{s:02}Z");
        let (t0, t1, t2, t5, t6) = (t(0), t(1), t(2), t(5), t(6));
        let slots = merge(
            &[Some(t1.as_str()), None, Some(t5.as_str())],
            &[t0.as_str(), t2.as_str(), t6.as_str()],
        );
        use Slot::*;
        assert_eq!(
            slots,
            vec![Row(0), Entry(0), Entry(1), Row(1), Entry(2), Row(2)]
        );
        // The same second: the thread first.
        assert_eq!(
            merge(&[Some(t1.as_str())], &[t1.as_str()]),
            vec![Row(0), Entry(0)]
        );
        // An older conversation, without times: the thread after it.
        assert_eq!(
            merge(&[None, None], &[t0.as_str()]),
            vec![Entry(0), Entry(1), Row(0)]
        );
        assert_eq!(merge(&[], &[t0.as_str()]), vec![Row(0)]);
    }

    /// Where a message went, as its chip reads; and how agents read rows:
    /// with their text in a test instance, without it in the Operator's
    /// window.
    #[test]
    fn a_message_says_where_it_went_and_rows_are_named_for_agents() {
        assert_eq!(
            routing(Some("implementer"), false),
            "to the implementer, at its next tool call"
        );
        assert_eq!(
            routing(Some("lead"), false),
            "waits for the lead's next turn"
        );
        assert_eq!(routing(Some("lead"), true), "given to the lead");
        assert_eq!(routing(Some("evaluator"), false), "to the evaluator");
        assert_eq!(routing(None, false), "sent");
        let directive = Row {
            id: "thread-objective-1-3".into(),
            objective: "objective-1".into(),
            seq: 3,
            at: String::new(),
            depth: 0,
            what: What::Step(Step {
                kind: Kind::Directive,
                author: "lead · claude-opus-5-5".into(),
                text: "Implement the label fix".into(),
                streaming: false,
                details: None,
                open: false,
                directive: Some(DirectiveChip {
                    from: "lead".into(),
                    to: "implementer".into(),
                    status: Some(("running".into(), Tone::Info)),
                    scope: None,
                    stop: None,
                }),
                routing: None,
                reply: true,
            }),
        };
        assert_eq!(
            label(&directive),
            "Directive from lead to implementer, running: Implement the label fix"
        );
        let fold = Row {
            id: "thread-fold-objective-1-3".into(),
            what: What::Fold {
                count: 12,
                open: false,
                progress: false,
                latest: None,
            },
            ..directive.clone()
        };
        assert_eq!(label(&fold), "12 tool calls under entry 3");
        assert_eq!(
            parse_id("thread-fold-objective-20261004-3"),
            Some((true, "objective-20261004", 3))
        );
        assert_eq!(
            parse_id("thread-objective-20261004-12"),
            Some((false, "objective-20261004", 12))
        );
        assert_eq!(parse_id("thread-reply-objective-1-3"), None);
        assert_eq!(parse_id("thread-objective-1-3-details"), None);
        assert_eq!(parse_id("tool-x"), None);
        let lead = Author::agent("lead", None);
        let written = entry("objective-1", 3, 3, Kind::Directive, lead, "Secret plan");
        let private = private_label(Some(&written), false, 3);
        assert_eq!(private, "Directive, entry 3, by lead");
        assert!(!private.contains("Secret"));
        let mine = entry(
            "objective-1",
            6,
            6,
            Kind::Human,
            Author::Operator,
            "my words",
        );
        assert_eq!(
            private_label(Some(&mine), false, 6),
            "Your message, entry 6 of the objective's thread"
        );
        assert_eq!(private_label(None, true, 3), "Tool calls under entry 3");
    }
}
