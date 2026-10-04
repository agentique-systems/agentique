//! Typed decisions in operation (C-53, ROADMAP §4.16; W11.6): when a dialog
//! stands in an application-control journey's way, which of its controls
//! continues toward the goal without losing work is one atomic question with
//! typed options (System 1). Where the answer is known, deterministic code
//! decides and no model is asked: a dialog asking for the Operator's
//! approval waits, and the Orchestrator, whose dialogs in the way are never
//! its goal, cancels them by rule ([`Way::Cancel`]). Otherwise Jev answers
//! under a deadline; a confident answer in time is used; low confidence, an
//! invalid answer, a timeout or an unavailable provider escalates to the
//! reasoning model (System 2). A decision only chooses what to press: it
//! never turns a failed check into a pass. The ways are compared live in
//! `tests/decisions.rs` and `tests/workflow.rs`.

use agq_providers::jev::{Answer, DecisionRequest, Question, QuestionKind};
use agq_providers::{
    AssistantPart, ChatRequest, Event, Message, ModelRef, Provider, Providers, UserPart,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

/// The option meaning "press nothing; the Operator decides".
pub const WAIT: &str = "wait";

/// A control of the dialog an agent could act on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub role: String,
    #[serde(default)]
    pub value: Option<String>,
    /// An option that is already chosen.
    #[serde(default)]
    pub selected: bool,
}

/// What a decision is about: the journey's goal and the dialog in its way.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Situation {
    pub goal: String,
    /// The dialog's kind (`Checkpoint`, `Confirm`, …).
    pub dialog: String,
    /// Set when the dialog asks for the Operator's approval.
    #[serde(default)]
    pub approval: Option<String>,
    pub controls: Vec<Choice>,
    #[serde(default)]
    pub status: String,
}

impl Situation {
    /// The situation an observation shows, if a dialog is open: its
    /// buttons, options and fields.
    pub fn from_observation(goal: &str, observation: &Value) -> Option<Situation> {
        let dialog = observation["dialog"].as_str()?.to_string();
        let controls = observation["controls"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|c| c["region"] == "dialog")
            .filter(|c| matches!(c["role"].as_str(), Some("button" | "option" | "field")))
            .filter(|c| c["enabled"] != false)
            // The system's folder picker is not operable through the
            // control interface.
            .filter(|c| !c["id"].as_str().unwrap_or_default().starts_with("browse-"))
            .map(|c| Choice {
                id: c["id"].as_str().unwrap_or_default().to_string(),
                label: c["label"].as_str().unwrap_or_default().to_string(),
                role: c["role"].as_str().unwrap_or_default().to_string(),
                value: c["value"].as_str().map(str::to_string),
                selected: c["selected"] == true,
            })
            .collect();
        Some(Situation {
            goal: goal.to_string(),
            dialog,
            approval: observation["approval"].as_str().map(str::to_string),
            controls,
            status: observation["status"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// The typed options: each control by id, and waiting.
    fn options(&self) -> BTreeMap<String, Option<String>> {
        let mut options: BTreeMap<String, Option<String>> = self
            .controls
            .iter()
            .map(|c| {
                let value = match (&c.value, c.role.as_str()) {
                    (Some(v), _) if !v.is_empty() => format!(", holding “{v}”"),
                    (_, "field") => ", empty".to_string(),
                    _ if c.selected => ", already chosen".to_string(),
                    _ => String::new(),
                };
                (
                    c.id.clone(),
                    Some(format!("{} “{}”{value}", c.role, c.label)),
                )
            })
            .collect();
        options.insert(
            WAIT.into(),
            Some("press nothing: the Operator must decide".into()),
        );
        options
    }

    fn state(&self) -> Value {
        json!({
            "goal": self.goal,
            "dialog": self.dialog,
            "asksForTheOperatorsApproval": self.approval,
            "status": self.status,
            "controls": self.controls,
        })
    }
}

/// Who decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Deterministic code.
    Rules,
    /// Jev, confident in time.
    Jev,
    /// The reasoning model, after Jev escalated.
    Escalated,
    /// The reasoning model alone.
    Model,
}

/// A decision and what it cost.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// The control to act on, or [`WAIT`].
    pub choice: String,
    pub source: Source,
    pub confidence: Option<f64>,
    pub millis: u64,
    /// US dollars; `None` when a call's usage or price is unknown.
    pub usd: Option<f64>,
    /// Why it escalated, or what failed.
    #[serde(default)]
    pub note: String,
}

const INSTRUCTIONS: &str = "An agent operates the Agentique application toward the goal. A dialog is open. Which control should it act on next so the journey moves toward the goal without losing work and without doing what the goal does not ask for? Act on an empty field the goal needs filled before confirming; choose a choice the goal names; confirm only when the dialog does what the goal asks; cancel a dialog the goal does not need. If the dialog asks for the Operator's approval, choose wait: an agent never answers it.";

/// A decision that failed, with what it cost: a request that was sent may
/// be billed, so its cost is unknown, never zero.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub error: String,
    pub millis: u64,
    pub usd: Option<f64>,
}

/// Cancel a dialog in the way (the Orchestrator's rule): an approval waits;
/// otherwise its Cancel; a dialog without one waits.
pub fn cancel(situation: &Situation) -> Decision {
    let choice = if situation.approval.is_none()
        && situation.controls.iter().any(|c| c.id == "dialog-cancel")
    {
        "dialog-cancel".to_string()
    } else {
        WAIT.to_string()
    };
    Decision {
        choice,
        source: Source::Rules,
        confidence: None,
        millis: 0,
        usd: Some(0.0),
        note: String::new(),
    }
}

/// The deterministic baseline: an approval waits; otherwise the journey
/// goes forward (the first empty field, else the dialog's confirm).
pub fn rules(situation: &Situation) -> Decision {
    let started = Instant::now();
    let choice = if situation.approval.is_some() {
        WAIT.to_string()
    } else if let Some(field) = situation
        .controls
        .iter()
        .find(|c| c.role == "field" && c.value.as_deref().is_none_or(str::is_empty))
    {
        field.id.clone()
    } else if situation.controls.iter().any(|c| c.id == "dialog-confirm") {
        "dialog-confirm".into()
    } else {
        WAIT.into()
    };
    Decision {
        choice,
        source: Source::Rules,
        confidence: None,
        millis: started.elapsed().as_millis() as u64,
        usd: Some(0.0),
        note: String::new(),
    }
}

/// Typed decisions with escalation, as configured.
pub struct Decider {
    pub providers: Providers,
    pub jev_model: String,
    pub model: ModelRef,
    /// Jev's confidence at or above which its answer is used.
    pub threshold: f64,
    /// How long Jev may take.
    pub deadline: Duration,
}

impl Default for Decider {
    fn default() -> Self {
        Decider {
            providers: Providers::new(),
            jev_model: agq_providers::jev::DEFAULT_MODEL.into(),
            model: ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro"),
            threshold: 0.6,
            deadline: Duration::from_secs(4),
        }
    }
}

impl Decider {
    /// Jev alone: its choice, with its confidence.
    pub fn jev(&self, situation: &Situation) -> Result<Decision, Failure> {
        if let Some(only) = only_waiting(situation) {
            return Ok(only);
        }
        let started = Instant::now();
        let request = DecisionRequest {
            model: self.jev_model.clone(),
            state: situation.state(),
            questions: BTreeMap::from([(
                "next".to_string(),
                Question {
                    instructions: INSTRUCTIONS.into(),
                    kind: QuestionKind::Choice {
                        options: situation.options(),
                    },
                },
            )]),
        };
        let mut handle = self
            .providers
            .decide_start(request, Instant::now() + self.deadline);
        let result = loop {
            if let Some(result) = handle.next_result(Duration::from_millis(50)) {
                break result;
            }
        };
        let millis = started.elapsed().as_millis() as u64;
        let model = ModelRef::new(Provider::TypeSafe, &self.jev_model);
        match result {
            Ok(reply) => {
                let usd = reply.usage.cost_usd(&model);
                match reply.answers.get("next") {
                    Some(Answer::Choice {
                        choice, confidence, ..
                    }) => Ok(Decision {
                        choice: choice.clone(),
                        source: Source::Jev,
                        confidence: Some(*confidence),
                        millis,
                        usd,
                        note: String::new(),
                    }),
                    _ => Err(Failure {
                        error: "Jev's answer is not a choice".into(),
                        millis,
                        usd,
                    }),
                }
            }
            Err(failure) => Err(Failure {
                error: format!("Jev: {failure}"),
                millis,
                usd: if failure.attempts == 0 {
                    Some(0.0)
                } else {
                    failure.usage.and_then(|u| u.cost_usd(&model))
                },
            }),
        }
    }

    /// The reasoning model alone.
    pub fn model(&self, situation: &Situation) -> Result<Decision, Failure> {
        if let Some(only) = only_waiting(situation) {
            return Ok(only);
        }
        let started = Instant::now();
        let options: Vec<String> = situation
            .options()
            .into_iter()
            .map(|(id, about)| format!("- {id}: {}", about.unwrap_or_default()))
            .collect();
        let text = format!(
            "{INSTRUCTIONS}\n\nThe state:\n{}\n\nThe options:\n{}\n\nAnswer with JSON only: {{\"choice\": \"<one option id>\"}}",
            serde_json::to_string_pretty(&situation.state()).unwrap_or_default(),
            options.join("\n")
        );
        // One more call when the answer cannot be read (the reasoning can
        // run past the limit and leave no answer); both are counted.
        let mut usd = Some(0.0);
        let mut problem = String::new();
        for _ in 0..2 {
            let request = ChatRequest {
                model: self.model.clone(),
                effort: None,
                // Room for the model's reasoning before its one-line answer.
                max_output_tokens: 8000,
                system: "You choose one action for an agent operating an application. You answer with JSON only.".into(),
                tools: Vec::new(),
                messages: vec![Message::User(vec![UserPart::Text { text: text.clone() }])],
            };
            let mut handle = self.providers.chat(request);
            let deadline = Instant::now() + Duration::from_secs(120);
            // A request that was sent may be billed: its cost is unknown.
            let failed = |error: String| Failure {
                error,
                millis: started.elapsed().as_millis() as u64,
                usd: None,
            };
            let reply = loop {
                match handle.next_event(Duration::from_millis(100)) {
                    Some(Event::Finished(Ok(reply))) => break reply,
                    Some(Event::Finished(Err(error))) => return Err(failed(error.to_string())),
                    Some(_) => {}
                    None if Instant::now() >= deadline => {
                        handle.cancel();
                        return Err(failed("the model did not answer in time".into()));
                    }
                    None => {}
                }
            };
            usd = usd.and_then(|total| Some(total + reply.usage.cost_usd(&self.model)?));
            let said: String = reply
                .content
                .iter()
                .filter_map(|p| match p {
                    AssistantPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            match parse_choice(&said, situation) {
                Ok(choice) => {
                    return Ok(Decision {
                        choice,
                        source: Source::Model,
                        confidence: None,
                        millis: started.elapsed().as_millis() as u64,
                        usd,
                        note: problem,
                    });
                }
                Err(error) => problem = error,
            }
        }
        Err(Failure {
            error: problem,
            millis: started.elapsed().as_millis() as u64,
            usd,
        })
    }

    /// Known answers by rule; otherwise Jev, escalating to the model when
    /// it is unsure, wrong in form, late or unavailable; waiting when
    /// nothing answers.
    pub fn decide(&self, situation: &Situation) -> Decision {
        if situation.approval.is_some() {
            return rules(situation);
        }
        if let Some(only) = only_waiting(situation) {
            return only;
        }
        let started = Instant::now();
        let mut usd = Some(0.0);
        let add = |total: Option<f64>, part: Option<f64>| Some(total? + part?);
        let note = match self.jev(situation) {
            Ok(decision)
                if decision.confidence.unwrap_or(0.0) >= self.threshold
                    && (decision.choice == WAIT
                        || situation.controls.iter().any(|c| c.id == decision.choice)) =>
            {
                return decision;
            }
            Ok(decision) => {
                usd = add(usd, decision.usd);
                format!(
                    "Jev chose {} with confidence {:.2}",
                    decision.choice,
                    decision.confidence.unwrap_or(0.0)
                )
            }
            Err(failure) => {
                usd = add(usd, failure.usd);
                failure.error
            }
        };
        match self.model(situation) {
            Ok(decision) => Decision {
                source: Source::Escalated,
                millis: started.elapsed().as_millis() as u64,
                usd: add(usd, decision.usd),
                note,
                ..decision
            },
            Err(failure) => Decision {
                choice: WAIT.into(),
                source: Source::Escalated,
                confidence: None,
                millis: started.elapsed().as_millis() as u64,
                usd: add(usd, failure.usd),
                note: format!("{note}; the model failed too: {}", failure.error),
            },
        }
    }
}

/// The ways of deciding compared in W11.6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Way {
    /// The forward rule: an empty field, else confirm.
    Rules,
    /// The cancelling rule: what the Orchestrator uses for a dialog in the
    /// way, which is never its goal.
    Cancel,
    Jev,
    Model,
    /// Rules where known, Jev, then the model.
    Escalating,
}

impl Decider {
    pub fn decide_with(&self, way: Way, situation: &Situation) -> Result<Decision, Failure> {
        match way {
            Way::Rules => Ok(rules(situation)),
            Way::Cancel => Ok(cancel(situation)),
            Way::Jev => self.jev(situation),
            Way::Model => self.model(situation),
            Way::Escalating => Ok(self.decide(situation)),
        }
    }
}

/// A dialog with nothing an agent may press: waiting, by rule (no model is
/// paid to choose the one option).
fn only_waiting(situation: &Situation) -> Option<Decision> {
    situation.controls.is_empty().then(|| Decision {
        choice: WAIT.into(),
        source: Source::Rules,
        confidence: None,
        millis: 0,
        usd: Some(0.0),
        note: "no control to press".into(),
    })
}

/// Clears the dialogs standing in the way of `goal` in a test instance: for
/// each, a decision by `decide` and the action it names. It stops at a
/// dialog asking for the Operator's approval (the journey waits), at a field
/// the goal gives no text for, or after `limit` decisions. With `guarded`,
/// only a dialog's Cancel is pressed: a dialog in the way of a goal is not
/// the goal's to answer, whatever a model judges (deterministic code has the
/// last word). Returns each dialog with the decision made about it (a failed
/// decision as waiting, with its time and cost), also when it stops, and
/// whether the way is clear.
pub fn clear_dialogs(
    client: &mut crate::control::Client,
    decide: &dyn Fn(&Situation) -> Result<Decision, Failure>,
    goal: &str,
    limit: usize,
    guarded: bool,
) -> (Vec<(String, Decision)>, Result<(), String>) {
    let mut made = Vec::new();
    for _ in 0..limit {
        let observation = match client.observe(true) {
            Ok(observation) => observation,
            Err(error) => return (made, Err(error)),
        };
        let Some(situation) = Situation::from_observation(goal, &observation) else {
            return (made, Ok(()));
        };
        let decision = match decide(&situation) {
            Ok(decision) => decision,
            Err(failure) => {
                made.push((
                    situation.dialog.clone(),
                    Decision {
                        choice: WAIT.into(),
                        source: Source::Model,
                        confidence: None,
                        millis: failure.millis,
                        usd: failure.usd,
                        note: failure.error.clone(),
                    },
                ));
                return (made, Err(failure.error));
            }
        };
        made.push((situation.dialog.clone(), decision.clone()));
        if decision.choice == WAIT {
            return (
                made,
                Err(format!(
                    "the {} dialog waits for the Operator",
                    situation.dialog
                )),
            );
        }
        let Some(control) = situation.controls.iter().find(|c| c.id == decision.choice) else {
            return (
                made,
                Err(format!("`{}` is not on the dialog", decision.choice)),
            );
        };
        if guarded && decision.choice != "dialog-cancel" {
            return (
                made,
                Err(format!(
                    "the decision was to press `{}` on the {} dialog, which is in the way, not the goal; nothing was pressed",
                    decision.choice, situation.dialog
                )),
            );
        }
        if control.role == "field" {
            return (
                made,
                Err(format!(
                    "the {} dialog needs “{}”, which the goal does not give",
                    situation.dialog, control.label
                )),
            );
        }
        let why = format!("a {} dialog is in the way of: {goal}", situation.dialog);
        let action = json!({ "kind": "click", "control": decision.choice });
        match client.act("orchestrator", &why, action) {
            Ok(answer) if answer["ok"] == false => {
                return (
                    made,
                    Err(format!("pressing {} failed: {answer}", decision.choice)),
                );
            }
            Err(error) => return (made, Err(error)),
            Ok(_) => {}
        }
    }
    (
        made,
        Err(format!("a dialog was still open after {limit} decisions")),
    )
}

/// The option the model's JSON names, if it is one of the options.
fn parse_choice(said: &str, situation: &Situation) -> Result<String, String> {
    // Every JSON object in the text that names a choice: they must agree.
    let named: BTreeSet<String> = said
        .char_indices()
        .filter(|(_, c)| *c == '{')
        .filter_map(|(at, _)| {
            let value = serde_json::Deserializer::from_str(&said[at..])
                .into_iter::<Value>()
                .next()?
                .ok()?;
            value["choice"].as_str().map(str::to_string)
        })
        .collect();
    let mut named = named.into_iter();
    let choice = named
        .next()
        .ok_or("the model gave no JSON naming a choice")?;
    if let Some(other) = named.next() {
        return Err(format!(
            "the model named more than one choice (`{choice}`, `{other}`)"
        ));
    }
    if choice == WAIT || situation.controls.iter().any(|c| c.id == choice) {
        Ok(choice)
    } else {
        Err(format!(
            "the model chose `{choice}`, which is not an option"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn situation(approval: Option<&str>, name: &str) -> Situation {
        Situation {
            goal: "Record a checkpoint".into(),
            dialog: "Checkpoint".into(),
            approval: approval.map(str::to_string),
            controls: vec![
                Choice {
                    id: "Checkpoint message".into(),
                    label: "Checkpoint message".into(),
                    role: "field".into(),
                    value: Some(name.into()),
                    selected: false,
                },
                Choice {
                    id: "dialog-cancel".into(),
                    label: "Cancel".into(),
                    role: "button".into(),
                    value: None,
                    selected: false,
                },
                Choice {
                    id: "dialog-confirm".into(),
                    label: "Record checkpoint".into(),
                    role: "button".into(),
                    value: None,
                    selected: false,
                },
            ],
            status: String::new(),
        }
    }

    #[test]
    fn the_rules_wait_for_approvals_fill_empty_fields_then_confirm() {
        assert_eq!(
            rules(&situation(Some("lock confirmation"), "x")).choice,
            WAIT
        );
        assert_eq!(rules(&situation(None, "")).choice, "Checkpoint message");
        assert_eq!(
            rules(&situation(None, "Before the cache")).choice,
            "dialog-confirm"
        );
        // An approval is decided by rule, without asking a model.
        let decider = Decider::default();
        let decided = decider.decide(&situation(Some("live run"), "x"));
        assert_eq!(
            (decided.choice.as_str(), decided.source),
            (WAIT, Source::Rules)
        );
    }

    #[test]
    fn the_cancelling_rule_cancels_unless_approval_is_asked() {
        assert_eq!(cancel(&situation(None, "x")).choice, "dialog-cancel");
        assert_eq!(cancel(&situation(Some("live run"), "x")).choice, WAIT);
        let mut bare = situation(None, "x");
        bare.controls.clear();
        assert_eq!(cancel(&bare).choice, WAIT);
        // Nothing to press: no model is paid to choose waiting.
        let decided = Decider::default().decide(&bare);
        assert_eq!(
            (decided.choice.as_str(), decided.source),
            (WAIT, Source::Rules)
        );
    }

    #[test]
    fn a_models_choice_must_be_an_option() {
        let s = situation(None, "x");
        assert_eq!(
            parse_choice("Sure: {\"choice\": \"dialog-cancel\"}", &s).unwrap(),
            "dialog-cancel"
        );
        // Braces in the reasoning before the answer do not hide it.
        assert_eq!(
            parse_choice(
                "Options {a, b} considered. {\"choice\": \"dialog-confirm\"} done {x}",
                &s
            )
            .unwrap(),
            "dialog-confirm"
        );
        // Answers that disagree are no answer.
        assert!(
            parse_choice(
                "{\"choice\": \"dialog-cancel\"}, not {\"choice\": \"dialog-confirm\"}",
                &s
            )
            .is_err()
        );
        assert!(parse_choice("{\"choice\": \"delete-everything\"}", &s).is_err());
        assert!(parse_choice("no json", &s).is_err());
    }

    #[test]
    fn a_situation_is_read_from_an_observation() {
        let observation = json!({
            "dialog": "Checkpoint",
            "approval": null,
            "status": "Ready",
            "controls": [
                { "id": "checkpoint", "label": "Checkpoint", "role": "dialog", "region": "dialog" },
                { "id": "Checkpoint message", "label": "Checkpoint message", "role": "field", "value": "", "region": "dialog" },
                { "id": "dialog-confirm", "label": "Record checkpoint", "role": "button", "region": "dialog" },
                { "id": "fit", "label": "Fit", "role": "button", "region": "title" }
            ]
        });
        let s = Situation::from_observation("Record a checkpoint", &observation).unwrap();
        assert_eq!(s.controls.len(), 2);
        assert!(s.options().contains_key(WAIT));
        assert!(Situation::from_observation("x", &json!({ "dialog": null })).is_none());
    }
}
