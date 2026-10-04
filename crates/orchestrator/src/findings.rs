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
//! fails on the build that has the problem and passes on one that fixed it.

use crate::explore::{Act, Instance, Step};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// How long an action may take, from the instance's own `tookMs`. An action
/// is a few frames of input and settling (0.3–0.5 s in a debug build), so
/// 2 s means the window drew nothing new for over a second: far past the
/// 100 ms feedback and 400 ms status budgets (ROADMAP §3.3), with room for a
/// debug build.
pub const ACTION_BUDGET_MS: u64 = 2000;

/// What a status line says when a program fault reaches it, rather than a
/// message written for the Operator (lowercase).
const INTERNAL_ERRORS: [&str; 7] = [
    "internal error",
    "panicked",
    "unwrap()",
    "unreachable",
    "should not happen",
    "index out of bounds",
    // An error's debug form, such as `Os { code: 5, kind: … }`.
    "os { code",
];

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
    /// An action finishes within [`ACTION_BUDGET_MS`].
    ActionTime,
    /// The status line reports no internal error.
    NoInternalError,
    /// What the explorer expected before acting holds after it.
    Expectation,
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
            found: agq_launcher::now(),
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

/// Whether an action's refusal says its control or command was gone,
/// disabled or unavailable (and not that the screen changed, or that it is
/// the Operator's: those are refusals by rule).
pub fn refused_as_gone(error: &str) -> bool {
    error.starts_with("stale: no control")
        || error.contains("is disabled now")
        || error.contains("is not available:")
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
    // The status line reports no internal error.
    let status = after["status"].as_str().unwrap_or_default();
    if INTERNAL_ERRORS
        .iter()
        .any(|e| status.to_lowercase().contains(e))
    {
        failed.push(Failed {
            check: Check::NoInternalError,
            control: target.clone(),
            message: format!("the status line reports an internal error: {status}"),
            evidence: json!({ "status": status }),
        });
    }
    let (Some(step), Some(answer)) = (outcome.step, outcome.answer) else {
        return failed;
    };
    let before = outcome.before;
    if answer["ok"] == false {
        // Offered, enabled, and refused as gone or disabled while the screen
        // stayed the same and still offers it.
        let error = answer["error"].as_str().unwrap_or_default();
        if refused_as_gone(error)
            && after["screenRevision"] == before["screenRevision"]
            && offered(after, &step.action).is_ok()
        {
            failed.push(Failed {
                check: Check::OfferedActs,
                control: target.clone(),
                message: format!("offered and enabled, but refused: {error}"),
                evidence: json!({ "answer": answer, "screenRevision": after["screenRevision"] }),
            });
        }
        return failed;
    }
    // Within its budget.
    if let Some(took) = answer["tookMs"].as_u64()
        && took > ACTION_BUDGET_MS
        && step.action["kind"] != "wait"
    {
        failed.push(Failed {
            check: Check::ActionTime,
            control: target.clone(),
            message: format!(
                "{} took {took} ms (budget {ACTION_BUDGET_MS} ms)",
                step.action["kind"].as_str().unwrap_or_default()
            ),
            evidence: json!({ "tookMs": took }),
        });
    }
    // A dialog closes by its Cancel or Escape.
    let closing = (step.action["kind"] == "click" && step.action["control"] == "dialog-cancel")
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
    // What the explorer expected.
    if let Some(expect) = &step.expect
        && let Err(problem) = crate::control::holds(after, expect)
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

/// The model's digest, where the observation publishes one.
fn digest(observation: &Value) -> Option<&Value> {
    observation["project"]
        .get("digest")
        .filter(|d| !d.is_null())
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

/// After an action (`step`, between `before` and `after`) changed the model,
/// undo must restore it and redo re-apply the change: a metamorphic check
/// the explorer makes itself in its test instance. With the observation's
/// `project.digest` the model's content is compared; without it, only that
/// undo and redo changed the model at all (`project.revision`, which rises
/// with every change, undo and redo, cannot show that a state came back).
/// `Err` when the check cannot be made here: undo refused by rule or not
/// available, which is never a finding.
pub fn undo_restores(
    instance: &mut dyn Instance,
    step: &Step,
    before: &Value,
    after: &Value,
    goal: &str,
) -> Result<Undone, String> {
    let mut steps = Vec::new();
    let control = step.target().to_string();
    let undone = act_by_check(instance, "undo", after, goal, &mut steps)?;
    let failed = |message: String, evidence: Value| Failed {
        check: Check::UndoRestores,
        control: control.clone(),
        message,
        evidence,
    };
    let undid = match (digest(before), digest(&undone)) {
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
        Err(problem) => {
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
    let redid = match (digest(after), digest(&redone)) {
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

/// Runs the command `id` for the undo check; the observation after it.
/// `Err` when it is refused, unavailable or fails.
fn act_by_check(
    instance: &mut dyn Instance,
    id: &str,
    now: &Value,
    goal: &str,
    steps: &mut Vec<Step>,
) -> Result<Value, String> {
    let action = json!({ "kind": "command", "id": id });
    let offered_to_agents = now["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["id"] == id);
    match offered_to_agents {
        Some(c) if c["available"] != false && c["operatorOnly"] != true => {}
        Some(c) if c["operatorOnly"] == true => {
            return Err(format!("`{id}` is the Operator's here"));
        }
        Some(c) => {
            return Err(format!(
                "`{id}` is not available: {}",
                c["why"].as_str().unwrap_or_default()
            ));
        }
        None => return Err(format!("`{id}` is not offered")),
    }
    let why = format!("check that {id} works after the change");
    let answer = instance.act(&Act {
        agent: crate::explore::AGENT,
        goal,
        why: &why,
        observed: now["screenRevision"].as_u64().unwrap_or_default(),
        action: &action,
    })?;
    if answer["ok"] == false {
        return Err(answer["error"].as_str().unwrap_or_default().to_string());
    }
    steps.push(Step {
        action,
        key: format!("{}|command|{id}|command", crate::explore::screen_of(now)),
        screen: crate::explore::screen_of(now),
        label: id.to_string(),
        expect: None,
        by: "check".into(),
    });
    instance.observe()
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
    /// ended earlier, a refusal): neither a pass nor a failure.
    Diverged { at: usize, reason: String },
}

impl Replay {
    pub fn failed(&self) -> bool {
        matches!(self, Replay::Failed { .. })
    }
}

/// Replays a finding's steps (the reduced ones, when there are) from a fresh
/// start of the instance's build and checks it after the last: whether the
/// problem shows there. A cycle uses it as a criterion: it fails on the
/// build that has the problem and passes on one that fixed it.
pub fn replay(instance: &mut dyn Instance, finding: &Finding) -> Replay {
    replay_steps(instance, finding, finding.replay_steps())
}

fn replay_steps(instance: &mut dyn Instance, finding: &Finding, steps: &[Step]) -> Replay {
    let diverged = |at: usize, reason: String| Replay::Diverged { at, reason };
    // The instance ended or stopped answering on the step at `at`.
    let gone = |instance: &mut dyn Instance, at: usize, error: String| {
        let message = ended(instance);
        if at == steps.len() && finding.check == Check::Answers {
            judge_message(finding, Check::Answers, &message)
        } else {
            diverged(at, format!("the instance ended at step {at}: {error}"))
        }
    };
    if let Err(error) = instance.restart() {
        return diverged(0, format!("the instance did not start: {error}"));
    }
    let mut now = match instance.observe() {
        Ok(now) => now,
        Err(error) => return gone(instance, 0, error),
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
        let mut answer = None;
        // Re-based on a fresh observation; a stale refusal is observed
        // again once.
        for _ in 0..2 {
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
            match instance.act(&act) {
                Err(error) => return gone(instance, at, error),
                Ok(said)
                    if said["ok"] == false
                        && said["error"]
                            .as_str()
                            .is_some_and(|e| e.starts_with("stale: the screen changed")) =>
                {
                    now = match instance.observe() {
                        Ok(now) => now,
                        Err(error) => return gone(instance, at, error),
                    };
                }
                Ok(said) => {
                    answer = Some(said);
                    break;
                }
            }
        }
        let Some(answer) = answer else {
            return diverged(at, "the screen kept changing".into());
        };
        let error = answer["error"].as_str().unwrap_or_default();
        let last = at == steps.len();
        let expected_refusal = last && finding.check == Check::OfferedActs;
        if answer["ok"] == false
            && !expected_refusal
            && (error.starts_with("stale")
                || error.starts_with("refused")
                || refused_as_gone(error))
        {
            return diverged(at, error.to_string());
        }
        let after = match instance.observe() {
            Ok(after) => after,
            Err(error) => return gone(instance, at, error),
        };
        if last {
            if finding.check == Check::UndoRestores {
                return match undo_restores(instance, step, &now, &after, "replay a finding") {
                    Ok(undone) => judge(finding, &undone.failed.into_iter().collect::<Vec<_>>()),
                    Err(reason) => {
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

/// The message for an instance that ended or stopped answering.
pub fn ended(instance: &mut dyn Instance) -> String {
    if instance.alive() {
        "the instance stopped answering".into()
    } else {
        "the instance exited".into()
    }
}

fn judge_message(finding: &Finding, check: Check, message: &str) -> Replay {
    if identity(check, &finding.control, message) == finding.identity {
        Replay::Failed {
            message: message.to_string(),
        }
    } else {
        Replay::Passed
    }
}

fn judge(finding: &Finding, failed: &[Failed]) -> Replay {
    match failed.iter().find(|f| f.identity() == finding.identity) {
        Some(f) => Replay::Failed {
            message: f.message.clone(),
        },
        None => Replay::Passed,
    }
}

/// Replays a finding twice from a fresh start; it is reproduced only when
/// both fail the same way, and then reduced within what is left of
/// `replays` (at least two are needed).
pub fn reproduce(instance: &mut dyn Instance, finding: &mut Finding, replays: usize) {
    let first = replay_steps(instance, finding, &finding.steps);
    let second = replay_steps(instance, finding, &finding.steps);
    let reproduced = first.failed() && second.failed();
    finding.replays.extend([first, second]);
    if reproduced {
        finding.state = State::Reproduced;
        finding.note = String::new();
        reduce(instance, finding, replays.saturating_sub(2));
    } else {
        finding.state = State::NotReproduced;
        finding.note = finding
            .replays
            .iter()
            .rev()
            .take(2)
            .rev()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "replay {}: {}",
                    i + 1,
                    match r {
                        Replay::Failed { .. } => "failed the same way".to_string(),
                        Replay::Passed => "the check held".to_string(),
                        Replay::Diverged { at, reason } =>
                            format!("diverged at step {at}: {reason}"),
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
    }
}

/// Drops steps greedily while the finding still reproduces, within
/// `replays` replays (one of them kept to confirm the result): first the
/// longest prefix (the shortest suffix of steps that still fails, trying 0,
/// 1, 2, 4, … steps), then runs of steps of halving length (4, 2, 1 for
/// eight steps), so a dialog opened and cancelled goes as a pair. Reduced
/// steps are kept only when one more replay confirms them.
pub fn reduce(instance: &mut dyn Instance, finding: &mut Finding, replays: usize) {
    if replays < 2 {
        return;
    }
    let trials = replays - 1;
    let mut used = 0;
    let mut steps = finding.steps.clone();
    let fails = |instance: &mut dyn Instance, steps: &[Step], used: &mut usize| {
        *used += 1;
        replay_steps(instance, finding, steps).failed()
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
    if steps.len() < n {
        // Confirmed by one more replay, so the reduced steps failed twice.
        if fails(instance, &steps, &mut used) {
            finding.reduced = Some(steps);
        } else {
            finding.note = "the reduced steps did not fail again; the full steps are kept".into();
        }
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
        let step = |action: Value| Step {
            action,
            key: String::new(),
            screen: "surface:Create".into(),
            label: String::new(),
            expect: None,
            by: "explorer".into(),
        };
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
        // Refused as disabled while the same screen still offers it.
        let sync = step(json!({ "kind": "click", "control": "sync" }));
        let refused = json!({ "ok": false, "error": "`Sync` is disabled now" });
        let found = failures(&Outcome {
            before: &before,
            step: Some(&sync),
            answer: Some(&refused),
            after: &before,
        });
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
        let mut closed = before.clone();
        closed["dialog"] = Value::Null;
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
}
