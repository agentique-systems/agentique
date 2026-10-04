//! Findings (C-54, ROADMAP §4.16 "Exploration"; requirement
//! `FindingsReproduce`): a finding is a deterministic check that failed
//! while an explorer operated a test instance: an invariant of the
//! application, or an expectation the explorer stated before acting. It is
//! **reproduced** by replaying its steps from a fresh start of the same build
//! (each action re-based on a fresh observation, no model asked) twice, both
//! failing the same way after the same step, and **reduced** by dropping
//! steps while it still reproduces. Its **identity** is its check, control and
//! message with numbers, paths, ids and quoted names normalised, so the same
//! problem found again is the same finding. A refusal by rule (the
//! Operator's own, a stale action) is never a finding, and neither is a
//! model's opinion. [`replay`] is what a cycle's criterion runs later: it
//! fails on the build that has the problem and passes on one that fixed it;
//! an instance that ends or hangs in a replay is never a pass.

use crate::explore::{Act, Instance, Step};
use crate::observed::{self, Refusal};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// How long an action may take, from the instance's own `tookMs`. An action
/// is a few frames of input and settling (0.3–0.5 s in a debug build), so
/// 2 s means the window drew nothing new for over a second: far past the
/// 100 ms feedback and 400 ms status budgets (ROADMAP §3.3), with room for a
/// debug build.
pub const ACTION_BUDGET_MS: u64 = 2000;

/// What each character an action types adds to its budget: an agent's fill
/// types its whole text in one step, while each character is a keystroke
/// that must answer within the 100 ms feedback budget (ROADMAP §3.3). (A
/// 300-character fill takes about 9 s in a debug build, 30 ms a keystroke;
/// observer mode types about 12 characters a second.)
pub const TYPED_CHARACTER_MS: u64 = 100;

/// How long a turn of the Assistant may take after a request: a request of
/// the fixed set needs a few tool calls; three minutes is far past what any
/// of them takes, and keeps a turn that never ends from using up a run.
pub const TURN_BUDGET_MS: u64 = 180_000;

/// How long Stop may take to end a running turn: it cancels the model's
/// stream and the turn's tool call, which takes a moment, not ten seconds.
pub const STOP_BUDGET_MS: u64 = 10_000;

/// How often a turn is observed while it runs.
const TURN_POLL: Duration = Duration::from_millis(250);

/// How long an action refused as `held` (another agent holds the window)
/// waits before it is asked again, and how often: an agent releases a
/// window it is done with, and an idle one is released after 30 s, so a
/// few short waits are enough for the usual case; never a finding.
pub const HELD_WAIT: Duration = Duration::from_millis(500);
pub const HELD_TRIES: usize = 4;

/// The part of an explorer's expectation about the Assistant's reply. A
/// model writes its reply differently from run to run, so this part is
/// checked and recorded, never a finding (and not part of the criteria's
/// grammar, `control::expectation`).
pub const REPLY: &str = "replyContains";

/// What a status line or notice says when a program fault reaches it: the
/// Rust runtime's own wording (the Studio has no wording of its own for an
/// internal error), and an error's debug form (`Os { code: 5, … }`).
/// Lowercase. A provider's "internal error (500)" is not one of these.
const INTERNAL_ERRORS: [&str; 6] = [
    "panicked at",
    "called `option::unwrap()`",
    "called `result::unwrap()`",
    "entered unreachable code",
    "index out of bounds: the len is",
    "os { code:",
];

/// The message of a finding that the instance exited or stopped answering:
/// one for both, so a replay that ends the other way is still the same
/// failure (which one it was is in the evidence).
pub const ENDED: &str = "the instance exited or stopped answering";

/// The roles an agent acts on, whose labels must be readable.
pub const INTERACTIVE: [&str; 7] = ["button", "tab", "option", "item", "switch", "link", "field"];

/// The checks: the invariants of the application, and the explorer's
/// expectations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Check {
    /// The instance runs and answers.
    Answers,
    /// A control or command the observation offered, enabled, is not
    /// refused as gone or disabled on a screen that did not change.
    OfferedActs,
    /// Every interactive control has a readable label (not empty, not its
    /// machine id).
    ReadableLabels,
    /// After an action changed the model, undo restores it and redo
    /// re-applies the change.
    UndoRestores,
    /// A dialog other than an approval closes by its Cancel or Escape.
    DialogsClose,
    /// An action finishes within [`ACTION_BUDGET_MS`] (and
    /// [`TYPED_CHARACTER_MS`] for each character it types).
    ActionTime,
    /// The status line and the Conversation's notices report no internal
    /// error.
    NoInternalError,
    /// What the explorer expected before acting holds after it.
    Expectation,
    /// A turn of the Assistant ends within its budget ([`TURN_BUDGET_MS`]).
    TurnEnds,
    /// Stop ends a running turn within its budget ([`STOP_BUDGET_MS`]).
    TurnStops,
}

impl Check {
    pub fn name(self) -> &'static str {
        match self {
            Check::Answers => "answers",
            Check::OfferedActs => "offered-acts",
            Check::ReadableLabels => "readable-labels",
            Check::UndoRestores => "undo-restores",
            Check::DialogsClose => "dialogs-close",
            Check::ActionTime => "action-time",
            Check::NoInternalError => "no-internal-error",
            Check::Expectation => "expectation",
            Check::TurnEnds => "turn-ends",
            Check::TurnStops => "turn-stops",
        }
    }
}

/// Where a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    /// Found, not yet replayed.
    Open,
    /// Two replays from a fresh start failed the same way.
    Reproduced,
    /// A replay passed or could not follow the steps: it proposes nothing.
    NotReproduced,
    /// A merged change fixed it (`fixed_in`).
    Fixed,
    /// It was fixed and fails again: a regression.
    FailingAgain,
}

/// A change of a finding's state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub state: State,
    pub at: String,
    #[serde(default)]
    pub note: String,
}

/// A check that failed.
#[derive(Clone, Debug, PartialEq)]
pub struct Failed {
    pub check: Check,
    pub control: String,
    pub message: String,
    /// The observed values it rests on.
    pub evidence: Value,
}

impl Failed {
    pub fn identity(&self) -> String {
        identity(self.check, &self.control, &self.message)
    }
}

/// That the instance exited or stopped answering, after an action on
/// `control` (or at the start).
pub fn ended(instance: &mut dyn Instance, control: &str, error: &str) -> Failed {
    Failed {
        check: Check::Answers,
        control: control.to_string(),
        message: ENDED.into(),
        evidence: json!({ "exited": !instance.alive(), "error": error }),
    }
}

/// A problem found while exploring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub check: Check,
    /// The control, command or key it is about (empty when none).
    pub control: String,
    /// What failed, as observed.
    pub message: String,
    /// The check, the control and the message normalised.
    pub identity: String,
    pub build: String,
    pub commit: String,
    /// The start state: the project the instance starts with.
    pub start: String,
    /// The actions from a fresh start, each with the screen it was taken
    /// on; the check failed after the last one (at the start, when none).
    pub steps: Vec<Step>,
    /// The observed values it rests on (never pixels).
    pub evidence: Value,
    pub found: String,
    pub state: State,
    /// Why it did not reproduce, or what became of it.
    #[serde(default)]
    pub note: String,
    /// Its states, in order.
    #[serde(default)]
    pub history: Vec<Change>,
    /// The fewest steps found that still reproduce it.
    #[serde(default)]
    pub reduced: Option<Vec<Step>>,
    /// Each replay's outcome, in order.
    #[serde(default)]
    pub replays: Vec<Replay>,
    /// The commit that fixed it.
    #[serde(default)]
    pub fixed_in: Option<String>,
    #[serde(default)]
    pub pull_request: Option<u64>,
    /// The build in which the fix was last replayed (and passed).
    #[serde(default)]
    pub checked_in: Option<String>,
}

impl Finding {
    pub fn new(
        failed: Failed,
        steps: Vec<Step>,
        build: &str,
        commit: &str,
        start: &str,
    ) -> Finding {
        let found = agq_launcher::now();
        Finding {
            identity: failed.identity(),
            check: failed.check,
            control: failed.control,
            message: failed.message,
            build: build.to_string(),
            commit: commit.to_string(),
            start: start.to_string(),
            steps,
            evidence: failed.evidence,
            history: vec![Change {
                state: State::Open,
                at: found.clone(),
                note: String::new(),
            }],
            found,
            state: State::Open,
            note: String::new(),
            reduced: None,
            replays: Vec::new(),
            fixed_in: None,
            pull_request: None,
            checked_in: None,
        }
    }

    /// The steps a replay takes: the reduced ones when there are.
    pub fn replay_steps(&self) -> &[Step] {
        self.reduced.as_deref().unwrap_or(&self.steps)
    }

    /// Moves it to `state`, with why, keeping the change in its history.
    pub fn set_state(&mut self, state: State, note: &str) {
        self.state = state;
        self.note = note.to_string();
        self.history.push(Change {
            state,
            at: agq_launcher::now(),
            note: note.to_string(),
        });
    }
}

/// A finding's identity: its check, its control and its message, with what
/// changes from run to run normalised.
pub fn identity(check: Check, control: &str, message: &str) -> String {
    format!(
        "{}|{}|{}",
        check.name(),
        normalise(control),
        normalise(message)
    )
}

/// `text` with what changes from run to run normalised: quoted names
/// (“…”, `…`, "…"), paths, qualified element names, long hexadecimal ids
/// and numbers.
pub fn normalise(text: &str) -> String {
    const QUOTES: [(char, char); 3] = [('“', '”'), ('`', '`'), ('"', '"')];
    let mut unquoted = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let after = &rest[c.len_utf8()..];
        if let Some((open, close)) = QUOTES.iter().find(|(open, _)| *open == c)
            && let Some(end) = after.find(*close)
        {
            unquoted.push(*open);
            unquoted.push('*');
            unquoted.push(*close);
            rest = &after[end + close.len_utf8()..];
            continue;
        }
        unquoted.push(c);
        rest = after;
    }
    let words: Vec<String> = unquoted
        .split_whitespace()
        .map(|word| {
            let core = word.trim_matches(|c: char| ",.;:()[]{}".contains(c));
            if core.contains(['/', '\\']) && core.chars().any(char::is_alphanumeric) {
                "<path>".to_string()
            } else if core.contains("::") {
                "<name>".to_string()
            } else if core.len() >= 8
                && core.chars().all(|c| c.is_ascii_hexdigit())
                && core.chars().any(|c| c.is_ascii_digit())
            {
                "<id>".to_string()
            } else {
                let mut out = String::new();
                for c in word.chars() {
                    if c.is_ascii_digit() {
                        if !out.ends_with('#') {
                            out.push('#');
                        }
                    } else {
                        out.push(c);
                    }
                }
                out
            }
        })
        .collect();
    words.join(" ").chars().take(160).collect()
}

/// A label that tells nobody what the control does: empty, or the control's
/// own machine id (lowercase words joined by hyphens or underscores). A
/// row's label is its element's name, whatever the name is.
fn unreadable(control: &Value) -> bool {
    let label = control["label"].as_str().unwrap_or_default().trim();
    let id = control["id"].as_str().unwrap_or_default();
    label.is_empty()
        || (control["role"] != "item"
            && label == id
            && label.contains(['-', '_'])
            && !label.contains(' ')
            && label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'))
}

/// An expectation without its part about the reply: what can be checked
/// deterministically, if anything is left.
pub fn deterministic(expect: &Value) -> Option<Value> {
    let mut rest = expect.as_object()?.clone();
    rest.remove(REPLY);
    (!rest.is_empty()).then_some(Value::Object(rest))
}

/// Whether the reply says what the expectation's reply part asks, when it
/// has one.
pub fn reply_holds(observation: &Value, expect: &Value) -> Option<Result<(), String>> {
    let text = expect[REPLY].as_str()?;
    Some(if observed::last_reply(observation).contains(text) {
        Ok(())
    } else {
        Err(format!("the reply does not say “{text}”"))
    })
}

/// What one action did, as the checks read it.
pub struct Outcome<'a> {
    /// The observation the action rested on.
    pub before: &'a Value,
    /// The action (none: the instance has just started).
    pub step: Option<&'a Step>,
    /// The instance's answer to it.
    pub answer: Option<&'a Value>,
    /// The observation after it.
    pub after: &'a Value,
}

/// Whether an observation offers `action`'s target to an agent: its control
/// on screen and enabled, or its command available.
pub fn offered(observation: &Value, action: &Value) -> Result<(), String> {
    if let Some(name) = action["control"].as_str() {
        let control = observation["controls"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["id"] == name);
        return match control {
            None => Err(format!("no control `{name}` is on screen")),
            Some(c) if c["enabled"] == false => Err(format!("`{name}` is disabled")),
            Some(_) => Ok(()),
        };
    }
    if action["kind"] == "command" {
        let id = action["id"].as_str().unwrap_or_default();
        let available = observation["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| c["id"] == id && c["available"] != false);
        return if available {
            Ok(())
        } else {
            Err(format!("`{id}` is not available"))
        };
    }
    Ok(())
}

/// What the status line and the Conversation's notices say, each with
/// where.
fn said(observation: &Value) -> Vec<(&'static str, String)> {
    std::iter::once((
        "the status line",
        observation["status"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    ))
    .chain(
        observed::notices(observation)
            .into_iter()
            .map(|n| ("a notice", n)),
    )
    .collect()
}

/// The invariants an action (or the start) shows to fail, and the
/// expectation stated before it. `answers` and `undo-restores` need the
/// instance, so they are checked where it is used.
pub fn failures(outcome: &Outcome) -> Vec<Failed> {
    let mut failed = Vec::new();
    let after = outcome.after;
    let target = outcome
        .step
        .map(|s| s.target().to_string())
        .unwrap_or_default();
    // Every interactive control has a readable label.
    for control in after["controls"].as_array().into_iter().flatten() {
        if INTERACTIVE.contains(&control["role"].as_str().unwrap_or_default())
            && unreadable(control)
        {
            failed.push(Failed {
                check: Check::ReadableLabels,
                control: control["id"].as_str().unwrap_or_default().to_string(),
                message: format!(
                    "a {} in {} has no readable label",
                    control["role"].as_str().unwrap_or_default(),
                    control["region"].as_str().unwrap_or_default()
                ),
                evidence: control.clone(),
            });
        }
    }
    // An internal error in the status line or a notice, when it appears or
    // changes: one that stays is the same finding, blaming no later action
    // (the control is left out of its identity).
    let earlier = if outcome.step.is_some() {
        said(outcome.before)
    } else {
        Vec::new()
    };
    for (place, text) in said(after) {
        if INTERNAL_ERRORS
            .iter()
            .any(|e| text.to_lowercase().contains(e))
            && !earlier.iter().any(|(_, t)| *t == text)
        {
            failed.push(Failed {
                check: Check::NoInternalError,
                control: String::new(),
                message: format!("{place} reports an internal error: {text}"),
                evidence: json!({ "text": text, "after": target }),
            });
        }
    }
    let (Some(step), Some(answer)) = (outcome.step, outcome.answer) else {
        return failed;
    };
    let before = outcome.before;
    // A turn that did not end in its budget, or a Stop that did not end it.
    if step.action["kind"] == "await-turn" {
        if answer["ok"] == false {
            let stop = step.label == "the stop";
            let budget = step.action["timeoutMs"].as_u64().unwrap_or_default();
            failed.push(Failed {
                check: if stop {
                    Check::TurnStops
                } else {
                    Check::TurnEnds
                },
                control: "conversation".into(),
                message: if stop {
                    format!("Stop did not end the turn within {budget} ms")
                } else {
                    format!("the turn did not end within {budget} ms")
                },
                evidence: json!({ "answer": answer, "conversation": after["conversation"] }),
            });
        }
    } else if answer["ok"] == false {
        let error = answer["error"].as_str().unwrap_or_default();
        match observed::refusal(answer) {
            // Offered, enabled, and refused as gone or disabled while the
            // screen stayed the same and still offers it.
            Refusal::Gone
                if after["screenRevision"] == before["screenRevision"]
                    && offered(after, &step.action).is_ok() =>
            {
                failed.push(Failed {
                    check: Check::OfferedActs,
                    control: target.clone(),
                    message: format!("offered and enabled, but refused: {error}"),
                    evidence: json!({ "answer": answer, "screenRevision": after["screenRevision"] }),
                });
            }
            // Offered, and its handler failed.
            Refusal::Failed => failed.push(Failed {
                check: Check::OfferedActs,
                control: target.clone(),
                message: format!("offered, but its handler failed: {error}"),
                evidence: json!({ "answer": answer }),
            }),
            // Not carried out in time.
            Refusal::Timeout => failed.push(Failed {
                check: Check::ActionTime,
                control: target.clone(),
                message: format!(
                    "{} was not carried out in time ({})",
                    step.action["kind"].as_str().unwrap_or_default(),
                    answer["kind"].as_str().unwrap_or("timeout")
                ),
                evidence: json!({ "answer": answer }),
            }),
            _ => {}
        }
        return failed;
    } else {
        // Within its budget.
        let typed = step.action["text"]
            .as_str()
            .map_or(0, |t| t.chars().count() as u64);
        let budget = ACTION_BUDGET_MS + TYPED_CHARACTER_MS * typed;
        if let Some(took) = answer["tookMs"].as_u64()
            && took > budget
            && step.action["kind"] != "wait"
        {
            failed.push(Failed {
                check: Check::ActionTime,
                control: target.clone(),
                message: format!(
                    "{} took {took} ms (budget {budget} ms)",
                    step.action["kind"].as_str().unwrap_or_default()
                ),
                evidence: json!({ "tookMs": took }),
            });
        }
        // A dialog closes by its Cancel or Escape.
        let closing = (step.action["kind"] == "click"
            && step.action["control"] == observed::CANCEL)
            || (step.action["kind"] == "key" && step.action["keys"] == "escape");
        if closing
            && before["dialog"].is_string()
            && before["approval"].is_null()
            && before["palette"].is_null()
            && after["dialog"] == before["dialog"]
        {
            failed.push(Failed {
                check: Check::DialogsClose,
                control: target.clone(),
                message: format!(
                    "the {} dialog stayed open after {}",
                    before["dialog"].as_str().unwrap_or_default(),
                    if step.action["kind"] == "key" {
                        "Escape"
                    } else {
                        "its Cancel"
                    }
                ),
                evidence: json!({ "dialog": after["dialog"], "status": after["status"] }),
            });
        }
    }
    // What the explorer expected (its part about the reply aside).
    if let Some(expect) = step.expect.as_ref().and_then(deterministic)
        && let Err(problem) = crate::control::holds(after, &expect)
    {
        failed.push(Failed {
            check: Check::Expectation,
            control: target,
            message: format!("expected {expect}: {problem}"),
            evidence: json!({ "expect": expect, "screen": after["screen"], "dialog": after["dialog"], "status": after["status"] }),
        });
    }
    failed
}

/// Whether the action between `before` and `after` changed the open model.
pub fn changed_model(before: &Value, after: &Value) -> bool {
    let (b, a) = (&before["project"], &after["project"]);
    !b.is_null() && b["folder"] == a["folder"] && a["revision"].as_u64() > b["revision"].as_u64()
}

/// What waiting for a turn of the Assistant came to.
pub enum Waited {
    /// It ended (`ok`), or ran past its budget (`ok: false`, timed out): the
    /// answer the checks read, and the observation then.
    Done { answer: Value, after: Value },
    /// The run was stopped while it waited.
    Stopped,
    /// The instance ended or stopped answering.
    Gone(String),
}

/// Waits for the running turn of the Assistant to end, observing it every
/// quarter second for at most `budget_ms`. Only the Conversation's state is
/// waited for, not the runs, tasks, checks or builds the Studio's own
/// `idle` condition includes.
pub fn await_turn(
    instance: &mut dyn Instance,
    budget_ms: u64,
    stop: &mut dyn FnMut() -> bool,
) -> Waited {
    let started = Instant::now();
    loop {
        let now = match instance.observe() {
            Ok(now) => now,
            Err(error) => return Waited::Gone(error),
        };
        let took = started.elapsed().as_millis() as u64;
        if !observed::turn_running(&now) {
            return Waited::Done {
                answer: json!({ "ok": true, "tookMs": took }),
                after: now,
            };
        }
        if took >= budget_ms {
            return Waited::Done {
                answer: json!({ "ok": false, "error": format!("timed out after {took} ms"), "tookMs": took }),
                after: now,
            };
        }
        if stop() {
            return Waited::Stopped;
        }
        std::thread::sleep(TURN_POLL);
    }
}

/// What the undo check did.
pub struct Undone {
    /// The failure, if undo or redo did not do its part.
    pub failed: Option<Failed>,
    /// The actions it took (undo, then redo when undo did its part).
    pub steps: Vec<Step>,
    /// The observation after them.
    pub now: Value,
}

/// Why the undo check was not made.
pub enum Unchecked {
    /// Undo refused (`by_rule`: as the Operator's own, so not tried again in
    /// the run) or not available: never a finding.
    Refused { reason: String, by_rule: bool },
    /// The instance ended or stopped answering on undo or redo, whose step
    /// is the last of `steps`.
    Ended { error: String, steps: Vec<Step> },
}

/// After an action (`step`, between `before` and `after`) changed the model,
/// undo must restore it and redo re-apply the change: a metamorphic check
/// the explorer makes itself in its test instance. With the observation's
/// `project.digest` the model's content is compared; without it, only that
/// undo and redo changed the model at all (`project.revision`, which rises
/// with every change, undo and redo, cannot show that a state came back).
pub fn undo_restores(
    instance: &mut dyn Instance,
    step: &Step,
    before: &Value,
    after: &Value,
    goal: &str,
) -> Result<Undone, Unchecked> {
    let mut steps = Vec::new();
    let control = step.target().to_string();
    let undone = match act_by_check(instance, "undo", after, goal, &mut steps) {
        Ok(undone) => undone,
        Err(Halt::Refused(reason, by_rule)) => return Err(Unchecked::Refused { reason, by_rule }),
        Err(Halt::Ended(error)) => return Err(Unchecked::Ended { error, steps }),
    };
    let failed = |message: String, evidence: Value| Failed {
        check: Check::UndoRestores,
        control: control.clone(),
        message,
        evidence,
    };
    let undid = match (observed::digest(before), observed::digest(&undone)) {
        (Some(was), Some(now)) if was != now => Some(failed(
            "undo did not restore the model".into(),
            json!({ "digestBefore": was, "digestAfterUndo": now }),
        )),
        (Some(_), Some(_)) => None,
        _ if undone["project"]["revision"] == after["project"]["revision"] => Some(failed(
            "undo did not change the model".into(),
            json!({ "revision": after["project"]["revision"] }),
        )),
        _ => None,
    };
    if let Some(problem) = undid {
        return Ok(Undone {
            failed: Some(problem),
            steps,
            now: undone,
        });
    }
    let redone = match act_by_check(instance, "redo", &undone, goal, &mut steps) {
        Ok(redone) => redone,
        Err(Halt::Ended(error)) => return Err(Unchecked::Ended { error, steps }),
        Err(Halt::Refused(problem, _)) => {
            return Ok(Undone {
                failed: Some(failed(
                    format!("redo did not re-apply the change: {problem}"),
                    json!({ "redo": problem }),
                )),
                steps,
                now: undone,
            });
        }
    };
    let redid = match (observed::digest(after), observed::digest(&redone)) {
        (Some(was), Some(now)) if was != now => Some(failed(
            "redo did not re-apply the change".into(),
            json!({ "digestAfterAction": was, "digestAfterRedo": now }),
        )),
        (Some(_), Some(_)) => None,
        _ if redone["project"]["revision"] == undone["project"]["revision"] => Some(failed(
            "redo did not change the model".into(),
            json!({ "revision": undone["project"]["revision"] }),
        )),
        _ => None,
    };
    Ok(Undone {
        failed: redid,
        steps,
        now: redone,
    })
}

/// Why a command of the undo check did not go through.
enum Halt {
    /// Refused or unavailable, and whether by rule.
    Refused(String, bool),
    /// The instance ended or stopped answering.
    Ended(String),
}

/// Runs the command `id` for the undo check; the observation after it. Its
/// step is kept in `steps` from the moment it is sent, so an instance that
/// ends on it is found with it.
fn act_by_check(
    instance: &mut dyn Instance,
    id: &str,
    now: &Value,
    goal: &str,
    steps: &mut Vec<Step>,
) -> Result<Value, Halt> {
    let action = json!({ "kind": "command", "id": id });
    let offered_to_agents = now["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["id"] == id);
    match offered_to_agents {
        Some(c) if observed::operator_only(c) => {
            return Err(Halt::Refused(
                format!("`{id}` is the Operator's here"),
                true,
            ));
        }
        Some(c) if c["available"] != false => {}
        Some(c) => {
            return Err(Halt::Refused(
                format!(
                    "`{id}` is not available: {}",
                    c["why"].as_str().unwrap_or_default()
                ),
                false,
            ));
        }
        None => return Err(Halt::Refused(format!("`{id}` is not offered"), false)),
    }
    steps.push(Step {
        key: format!("{}|command|{id}|command", crate::explore::screen_of(now)),
        screen: crate::explore::screen_of(now),
        label: id.to_string(),
        expect: None,
        by: "check".into(),
        action: action.clone(),
    });
    let why = format!("check that {id} works after the change");
    let answer = instance
        .act(&Act {
            agent: crate::explore::AGENT,
            goal,
            why: &why,
            observed: now["screenRevision"].as_u64().unwrap_or_default(),
            action: &action,
        })
        .map_err(Halt::Ended)?;
    if answer["ok"] == false {
        // Refused: it did nothing, so it is not a step.
        steps.pop();
        let by_rule = observed::refusal(&answer) == Refusal::Rule;
        return Err(Halt::Refused(
            answer["error"].as_str().unwrap_or_default().to_string(),
            by_rule,
        ));
    }
    instance.observe().map_err(Halt::Ended)
}

/// How a replay ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum Replay {
    /// The finding's check failed the same way after the same step: the
    /// problem shows.
    Failed { message: String },
    /// Every step was replayed and the check held after the last.
    Passed,
    /// The replay could not follow the steps (a control gone, the instance
    /// ended before the last step, a refusal, the run stopped): never a
    /// pass.
    Diverged { at: usize, reason: String },
}

impl Replay {
    pub fn failed(&self) -> bool {
        matches!(self, Replay::Failed { .. })
    }

    pub fn passed(&self) -> bool {
        matches!(self, Replay::Passed)
    }
}

/// Replays a finding's steps (the reduced ones, when there are) from a fresh
/// start of the instance's build and checks it after the last: whether the
/// problem shows there. A cycle uses it as a criterion: it fails on the
/// build that has the problem and passes on one that fixed it. `stop` is
/// asked during the replay's waits; a stopped replay diverges.
pub fn replay(
    instance: &mut dyn Instance,
    finding: &Finding,
    stop: &mut dyn FnMut() -> bool,
) -> Replay {
    replay_steps(instance, finding, finding.replay_steps(), stop)
}

fn replay_steps(
    instance: &mut dyn Instance,
    finding: &Finding,
    steps: &[Step],
    stop: &mut dyn FnMut() -> bool,
) -> Replay {
    let diverged = |at: usize, reason: String| Replay::Diverged { at, reason };
    // The instance ended or stopped answering on the step at `at`: the
    // finding itself when it is that, at its place; otherwise the replay
    // could not get there.
    let gone = |at: usize, error: String| {
        if at == steps.len() && finding.check == Check::Answers {
            Replay::Failed {
                message: ENDED.into(),
            }
        } else {
            Replay::Diverged {
                at,
                reason: format!("the instance ended at step {at}: {error}"),
            }
        }
    };
    if stop() {
        return diverged(0, "stopped".into());
    }
    if let Err(error) = instance.restart(stop) {
        return diverged(0, format!("the instance did not start: {error}"));
    }
    let mut now = match instance.observe() {
        Ok(now) => now,
        Err(error) => return gone(0, error),
    };
    if steps.is_empty() {
        let outcome = Outcome {
            before: &now,
            step: None,
            answer: None,
            after: &now,
        };
        return judge(finding, &failures(&outcome));
    }
    let why = format!("replaying {}", finding.identity);
    for (i, step) in steps.iter().enumerate() {
        let at = i + 1;
        let last = at == steps.len();
        if stop() {
            return diverged(at, "stopped".into());
        }
        if step.action["kind"] == "await-turn" {
            let budget = step.action["timeoutMs"].as_u64().unwrap_or(TURN_BUDGET_MS);
            match await_turn(instance, budget, stop) {
                Waited::Stopped => return diverged(at, "stopped".into()),
                Waited::Gone(error) => return gone(at, error),
                Waited::Done { answer, after } => {
                    if last {
                        let outcome = Outcome {
                            before: &now,
                            step: Some(step),
                            answer: Some(&answer),
                            after: &after,
                        };
                        return judge(finding, &failures(&outcome));
                    }
                    now = after;
                    continue;
                }
            }
        }
        let mut answer = None;
        // Re-based on a fresh observation; a stale refusal is observed
        // again once; a window another agent holds is waited for a little.
        let (mut stale, mut held) = (0, 0);
        while answer.is_none() {
            if let Err(reason) = offered(&now, &step.action) {
                return diverged(at, reason);
            }
            let act = Act {
                agent: crate::explore::AGENT,
                goal: "replay a finding",
                why: &why,
                observed: now["screenRevision"].as_u64().unwrap_or_default(),
                action: &step.action,
            };
            let said = match instance.act(&act) {
                Err(error) => return gone(at, error),
                Ok(said) => said,
            };
            match observed::refusal(&said) {
                Refusal::Stale if stale < 1 => stale += 1,
                Refusal::Held if held < HELD_TRIES && !stop() => {
                    held += 1;
                    std::thread::sleep(HELD_WAIT);
                }
                Refusal::Stopped => return diverged(at, "stopped".into()),
                _ => {
                    answer = Some(said);
                    continue;
                }
            }
            now = match instance.observe() {
                Ok(now) => now,
                Err(error) => return gone(at, error),
            };
        }
        let Some(answer) = answer else {
            return diverged(at, "the screen kept changing".into());
        };
        let expected_refusal = last && finding.check == Check::OfferedActs;
        if matches!(
            observed::refusal(&answer),
            Refusal::Stale | Refusal::Gone | Refusal::Rule | Refusal::Held | Refusal::Invalid
        ) && !expected_refusal
        {
            return diverged(at, answer["error"].as_str().unwrap_or_default().to_string());
        }
        let after = match instance.observe() {
            Ok(after) => after,
            Err(error) => return gone(at, error),
        };
        if last {
            if finding.check == Check::UndoRestores {
                return match undo_restores(instance, step, &now, &after, "replay a finding") {
                    Ok(undone) => judge(finding, &undone.failed.into_iter().collect::<Vec<_>>()),
                    Err(Unchecked::Ended { error, .. }) => {
                        diverged(at, format!("the instance ended on the undo check: {error}"))
                    }
                    Err(Unchecked::Refused { reason, .. }) => {
                        diverged(at, format!("the undo check could not be made: {reason}"))
                    }
                };
            }
            let outcome = Outcome {
                before: &now,
                step: Some(step),
                answer: Some(&answer),
                after: &after,
            };
            return judge(finding, &failures(&outcome));
        }
        now = after;
    }
    Replay::Passed
}

fn judge(finding: &Finding, failed: &[Failed]) -> Replay {
    match failed.iter().find(|f| f.identity() == finding.identity) {
        Some(f) => Replay::Failed {
            message: f.message.clone(),
        },
        None => Replay::Passed,
    }
}

fn stopped(replay: &Replay) -> bool {
    matches!(replay, Replay::Diverged { reason, .. } if reason == "stopped")
}

/// Replays a finding twice from a fresh start; it is reproduced only when
/// both fail the same way, and then reduced within what is left of
/// `replays` (at least two are needed). `stop` is asked between and during
/// replays: a stopped reproduction leaves the finding as it was, saying so.
pub fn reproduce(
    instance: &mut dyn Instance,
    finding: &mut Finding,
    replays: usize,
    stop: &mut dyn FnMut() -> bool,
) {
    let mut outcomes = Vec::new();
    for _ in 0..2 {
        let outcome = replay_steps(instance, finding, &finding.steps, stop);
        if stopped(&outcome) {
            finding.note = "its reproduction was stopped".into();
            return;
        }
        outcomes.push(outcome);
    }
    let reproduced = outcomes.iter().all(Replay::failed);
    let summary: String = outcomes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "replay {}: {}",
                i + 1,
                match r {
                    Replay::Failed { .. } => "failed the same way".to_string(),
                    Replay::Passed => "the check held".to_string(),
                    Replay::Diverged { at, reason } => format!("diverged at step {at}: {reason}"),
                }
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    finding.replays.extend(outcomes);
    let model = asks_the_assistant(finding);
    if reproduced {
        finding.set_state(State::Reproduced, &model);
        reduce(instance, finding, replays.saturating_sub(2), stop);
    } else {
        let note = if model.is_empty() {
            summary
        } else {
            format!("{summary}. {model}")
        };
        finding.set_state(State::NotReproduced, &note);
    }
}

/// What a replay cannot promise when the steps ask the Assistant: its
/// model answers differently from run to run.
fn asks_the_assistant(finding: &Finding) -> String {
    let asks = finding.steps.iter().any(|s| {
        s.key.contains("|conversation|")
            && (s.action["keys"] == "enter" || s.action["control"] == observed::SEND)
    });
    if asks {
        format!(
            "Its steps send requests to the Assistant, whose model answers differently from run to run: the replays repeat the same requests and re-check {} only, never the reply's wording",
            finding.check.name()
        )
    } else {
        String::new()
    }
}

/// Drops steps greedily while the finding still reproduces, within
/// `replays` replays (one of them kept to confirm the result): first the
/// longest prefix (the shortest suffix of steps that still fails, trying 0,
/// 1, 2, 4, … steps), then runs of steps of halving length (4, 2, 1 for
/// eight steps), so a dialog opened and cancelled goes as a pair. Reduced
/// steps are kept only when one more replay confirms them; `stop` ends it
/// early.
pub fn reduce(
    instance: &mut dyn Instance,
    finding: &mut Finding,
    replays: usize,
    stop: &mut dyn FnMut() -> bool,
) {
    if replays < 2 {
        return;
    }
    let trials = replays - 1;
    let mut used = 0;
    let mut halted = false;
    let mut steps = finding.steps.clone();
    let mut fails = |instance: &mut dyn Instance, steps: &[Step], used: &mut usize| {
        *used += 1;
        let outcome = replay_steps(instance, finding, steps, stop);
        halted |= stopped(&outcome);
        outcome.failed()
    };
    let n = steps.len();
    let mut keep = 0;
    while keep < n && used < trials {
        if fails(instance, &steps[n - keep..], &mut used) {
            steps = steps[n - keep..].to_vec();
            break;
        }
        keep = if keep == 0 { 1 } else { keep * 2 };
    }
    let mut run = 1;
    while run * 2 <= steps.len() / 2 {
        run *= 2;
    }
    while run >= 1 && used < trials {
        let mut i = 0;
        while i < steps.len() && used < trials && steps.len() > 1 {
            let mut fewer = steps.clone();
            fewer.drain(i..(i + run).min(steps.len()));
            if fails(instance, &fewer, &mut used) {
                steps = fewer;
            } else {
                i += run;
            }
        }
        run /= 2;
    }
    let confirmed = steps.len() < n && fails(instance, &steps, &mut used);
    if halted {
        finding.note = "its reduction was stopped; the full steps are kept".into();
    } else if confirmed {
        // The reduced steps failed twice: on their trial and here.
        finding.reduced = Some(steps);
    } else if steps.len() < n {
        finding.note = "the reduced steps did not fail again; the full steps are kept".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identity_ignores_numbers_paths_ids_and_names() {
        let a = identity(
            Check::NoInternalError,
            "build-use-0",
            "the status line reports an internal error: index out of bounds: the len is 3 but the index is 7 in C:\\work\\a\\model for “LinkStore” (UrlShortener::'part', 9f8e7d6c5b)",
        );
        let b = identity(
            Check::NoInternalError,
            "build-use-12",
            "the status line reports an internal error: index out of bounds: the len is 30 but the index is 31 in /tmp/x/model for “Cache” (Shop::Store, 0123abcdef)",
        );
        assert_eq!(a, b);
        assert_ne!(
            a,
            identity(Check::ActionTime, "build-use-0", "click took 2100 ms")
        );
        assert_eq!(
            normalise("took 2100 ms (budget 2000 ms)"),
            "took # ms (budget # ms)"
        );
        // An apostrophe is not a quote.
        assert_eq!(normalise("the Operator's own"), "the Operator's own");
    }

    #[test]
    fn labels_must_say_what_a_control_does() {
        let label = |id: &str, label: &str, role: &str| {
            unreadable(&json!({ "id": id, "label": label, "role": role }))
        };
        assert!(label("objective-intent", "objective-intent", "field"));
        assert!(label("btn-1", "", "button"));
        assert!(label("x", "   ", "tab"));
        assert!(!label("dialog-confirm", "Create", "button"));
        assert!(!label("Name", "Name", "field"));
        assert!(!label("project", "project", "button"));
        assert!(!label("part def", "part def", "option"));
        // A row is named by its element, whatever the element's name.
        assert!(!label("agq-orchestrator", "agq-orchestrator", "item"));
    }

    fn step(action: Value) -> Step {
        Step {
            action,
            key: String::new(),
            screen: "surface:Create".into(),
            label: String::new(),
            expect: None,
            by: "explorer".into(),
        }
    }

    #[test]
    fn checks_read_the_answer_and_the_observations_around_an_action() {
        let before = json!({
            "screenRevision": 4, "dialog": "Create", "approval": null, "palette": null,
            "status": "", "controls": [
                { "id": "dialog-cancel", "label": "Cancel", "role": "button", "region": "dialog" },
                { "id": "sync", "label": "Sync", "role": "button", "region": "title" }
            ],
            "commands": []
        });
        // Cancel that leaves the dialog open, slowly.
        let cancel = step(json!({ "kind": "click", "control": "dialog-cancel" }));
        let answer = json!({ "ok": true, "tookMs": 2500 });
        let found = failures(&Outcome {
            before: &before,
            step: Some(&cancel),
            answer: Some(&answer),
            after: &before,
        });
        let checks: Vec<Check> = found.iter().map(|f| f.check).collect();
        assert_eq!(checks, vec![Check::ActionTime, Check::DialogsClose]);
        // Typing has a keystroke's budget for each character.
        let long = step(json!({ "kind": "fill", "control": "Name", "text": "x".repeat(300) }));
        let slow_but_typing = json!({ "ok": true, "tookMs": 9000 });
        let mut closed = before.clone();
        closed["dialog"] = Value::Null;
        let within = |s: &Step, a: &Value| {
            failures(&Outcome {
                before: &before,
                step: Some(s),
                answer: Some(a),
                after: &closed,
            })
            .is_empty()
        };
        assert!(within(&long, &slow_but_typing));
        let short = step(json!({ "kind": "fill", "control": "Name", "text": "x" }));
        assert!(!within(&short, &slow_but_typing));
        // Refused as disabled while the same screen still offers it.
        let sync = step(json!({ "kind": "click", "control": "sync" }));
        let refused = json!({ "ok": false, "error": "`Sync` is disabled now" });
        let found = failures(&Outcome {
            before: &before,
            step: Some(&sync),
            answer: Some(&refused),
            after: &before,
        });
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].check, Check::OfferedActs);
        // A refusal by rule, or after the screen changed, is not a finding.
        let rule = json!({ "ok": false, "error": "refused: `undo` is the Operator's to use" });
        let mut moved = before.clone();
        moved["screenRevision"] = json!(5);
        for (answer, after) in [(&rule, &before), (&refused, &moved)] {
            let found = failures(&Outcome {
                before: &before,
                step: Some(&sync),
                answer: Some(answer),
                after,
            });
            assert!(found.is_empty(), "{found:?}");
        }
        // An expectation the observation does not show.
        let mut expecting = step(json!({ "kind": "click", "control": "dialog-cancel" }));
        expecting.expect = Some(json!({ "dialog": null }));
        let ok = json!({ "ok": true, "tookMs": 300 });
        assert!(
            failures(&Outcome {
                before: &before,
                step: Some(&expecting),
                answer: Some(&ok),
                after: &closed,
            })
            .is_empty()
        );
        let found = failures(&Outcome {
            before: &before,
            step: Some(&expecting),
            answer: Some(&ok),
            after: &before,
        });
        assert!(found.iter().any(|f| f.check == Check::Expectation));
    }

    #[test]
    fn an_internal_error_is_found_when_it_appears_not_for_every_action_after() {
        let quiet = json!({ "status": "Ready", "conversation": {} });
        let fault = json!({
            "status": "Validation failed: index out of bounds: the len is 2 but the index is 7",
            "conversation": {}
        });
        let click = |id: &str| step(json!({ "kind": "click", "control": id }));
        let ok = json!({ "ok": true, "tookMs": 100 });
        let errors = |before: &Value, step: &Step, after: &Value| {
            failures(&Outcome {
                before,
                step: Some(step),
                answer: Some(&ok),
                after,
            })
            .into_iter()
            .filter(|f| f.check == Check::NoInternalError)
            .collect::<Vec<_>>()
        };
        let first = errors(&quiet, &click("validate"), &fault);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].control, "", "no action is blamed in its identity");
        // It stays on screen while others act: not found again.
        assert!(errors(&fault, &click("graph"), &fault).is_empty());
        // A provider's wording is not a program fault.
        let provider = json!({
            "status": "Ready",
            "conversation": { "notices": ["The provider had an internal error (500); try again"] }
        });
        assert!(errors(&quiet, &click("send"), &provider).is_empty());
    }
}
