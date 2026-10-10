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
//!
//! Since C-54 the same typed question ([`Question`]: which of these options,
//! with instructions, a state and what each option means) also chooses the
//! explorer's next action (`explore`); [`Answers`] is where it is answered
//! (Jev and the models through the providers, or a stand-in in tests).

use agq_providers::jev::{self, Answer, DecisionRequest, QuestionKind};
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

    /// The typed question this dialog asks.
    pub fn question(&self) -> Question {
        Question {
            instructions: INSTRUCTIONS.into(),
            state: self.state(),
            options: self.options(),
        }
    }
}

/// One typed question: which of these options, given the instructions and
/// a state (a dialog in the way, or the explorer's next action).
#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub instructions: String,
    pub state: Value,
    /// By id, each with what choosing it means.
    pub options: BTreeMap<String, Option<String>>,
}

impl Question {
    /// The question as a model reads it, ending with the JSON its answer
    /// must be (`format`).
    pub fn prompt(&self, format: &str) -> String {
        let options: Vec<String> = self
            .options
            .iter()
            .map(|(id, about)| format!("- {id}: {}", about.as_deref().unwrap_or_default()))
            .collect();
        format!(
            "{}\n\nThe state:\n{}\n\nThe options:\n{}\n\nAnswer with JSON only: {format}",
            self.instructions,
            serde_json::to_string_pretty(&self.state).unwrap_or_default(),
            options.join("\n")
        )
    }
}

/// The most a model may write in answer to one typed question (its
/// reasoning and its one-line answer).
pub const MODEL_OUTPUT_TOKENS: u64 = 8000;

/// Where typed questions are answered: Jev and the models through the
/// providers ([`Decider`]), or a stand-in in tests.
pub trait Answers {
    /// Jev's model, and the confidence at or above which its answer is
    /// used.
    fn jev_model(&self) -> ModelRef;
    fn threshold(&self) -> f64;
    /// Jev's choice among the question's options, with its confidence,
    /// within its deadline.
    fn ask_jev(&self, question: &Question) -> Result<Decision, Failure>;
    /// One call to `model`: what it said and what it cost (`None` when
    /// unknown), or why it failed (a request that was sent may be billed,
    /// so a failed call's cost is unknown). `stop` is asked while it is
    /// waited for; a stopped call is cancelled and fails.
    fn chat(
        &self,
        model: &ModelRef,
        effort: Option<&str>,
        prompt: &str,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<(String, Option<f64>), String>;
}

/// Reads what a model said into the option it chose and what else it said,
/// or why the answer is not one the question allows.
pub type Read<'a, T> = &'a dyn Fn(&str) -> Result<(String, T), String>;

/// The answer a model gives to `question`, read by `read`. One more call
/// when the answer cannot be read (the reasoning can run past the limit and
/// leave no answer, or the answer is not one the question allows); both
/// are counted.
pub fn ask_model<T>(
    answers: &dyn Answers,
    model: &ModelRef,
    effort: Option<&str>,
    question: &Question,
    format: &str,
    read: Read<T>,
    stop: &mut dyn FnMut() -> bool,
) -> Result<(Decision, T), Failure> {
    let started = Instant::now();
    let prompt = question.prompt(format);
    let mut usd = Some(0.0);
    let mut problem = String::new();
    for _ in 0..2 {
        let (said, cost) = answers
            .chat(model, effort, &prompt, stop)
            .map_err(|error| Failure {
                source: Source::Model,
                error,
                millis: started.elapsed().as_millis() as u64,
                usd: None,
            })?;
        usd = usd.and_then(|total| Some(total + cost?));
        match read(&said) {
            Ok((choice, rest)) => {
                let decision = Decision {
                    choice,
                    source: Source::Model,
                    confidence: None,
                    millis: started.elapsed().as_millis() as u64,
                    usd,
                    note: problem,
                };
                return Ok((decision, rest));
            }
            Err(error) => problem = error,
        }
    }
    Err(Failure {
        source: Source::Model,
        error: problem,
        millis: started.elapsed().as_millis() as u64,
        usd,
    })
}

/// The JSON object in a model's text that names a choice: every object that
/// names one must name the same, and the first of them is the answer.
pub fn answer_object(said: &str) -> Result<Value, String> {
    let named: Vec<Value> = said
        .char_indices()
        .filter(|(_, c)| *c == '{')
        .filter_map(|(at, _)| {
            let value = serde_json::Deserializer::from_str(&said[at..])
                .into_iter::<Value>()
                .next()?
                .ok()?;
            value["choice"].is_string().then_some(value)
        })
        .collect();
    let choices: BTreeSet<&str> = named.iter().filter_map(|v| v["choice"].as_str()).collect();
    let mut choices = choices.into_iter();
    let choice = choices
        .next()
        .ok_or("the model gave no JSON naming a choice")?;
    if let Some(other) = choices.next() {
        return Err(format!(
            "the model named more than one choice (`{choice}`, `{other}`)"
        ));
    }
    Ok(named[0].clone())
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
    /// Who failed to decide.
    pub source: Source,
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
    /// The reasoning model's effort; `None` for its default.
    pub effort: Option<String>,
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
            effort: None,
            threshold: 0.6,
            deadline: Duration::from_secs(4),
        }
    }
}

impl Answers for Decider {
    fn jev_model(&self) -> ModelRef {
        ModelRef::new(Provider::TypeSafe, &self.jev_model)
    }

    fn threshold(&self) -> f64 {
        self.threshold
    }

    fn ask_jev(&self, question: &Question) -> Result<Decision, Failure> {
        let started = Instant::now();
        let request = DecisionRequest {
            model: self.jev_model.clone(),
            state: question.state.clone(),
            questions: BTreeMap::from([(
                "next".to_string(),
                jev::Question {
                    instructions: question.instructions.clone(),
                    kind: QuestionKind::Choice {
                        options: question.options.clone(),
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
                        source: Source::Jev,
                        error: "Jev's answer is not a choice".into(),
                        millis,
                        usd,
                    }),
                }
            }
            Err(failure) => Err(Failure {
                source: Source::Jev,
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

    fn chat(
        &self,
        model: &ModelRef,
        effort: Option<&str>,
        prompt: &str,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<(String, Option<f64>), String> {
        let request = ChatRequest {
            model: model.clone(),
            effort: effort.map(str::to_string),
            // Room for the model's reasoning before its one-line answer.
            max_output_tokens: MODEL_OUTPUT_TOKENS,
            system: "You choose one action for an agent operating an application. You answer with JSON only.".into(),
            tools: Vec::new(),
            messages: vec![Message::User(vec![UserPart::Text {
                text: prompt.to_string(),
            }])],
        };
        let mut handle = self.providers.chat(request);
        let deadline = Instant::now() + Duration::from_secs(120);
        let reply = loop {
            match handle.next_event(Duration::from_millis(100)) {
                Some(Event::Finished(Ok(reply))) => break reply,
                Some(Event::Finished(Err(error))) => return Err(error.to_string()),
                Some(_) => {}
                None if Instant::now() >= deadline => {
                    handle.cancel();
                    return Err("the model did not answer in time".into());
                }
                None if stop() => {
                    handle.cancel();
                    return Err("stopped".into());
                }
                None => {}
            }
        };
        let said: String = reply
            .content
            .iter()
            .filter_map(|p| match p {
                AssistantPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        Ok((said, reply.usage.cost_usd(model)))
    }
}

/// The answer a dialog's question asks of a model.
const CHOICE: &str = "{\"choice\": \"<one option id>\"}";

impl Decider {
    /// Jev alone: its choice, with its confidence.
    pub fn jev(&self, situation: &Situation) -> Result<Decision, Failure> {
        if let Some(only) = only_waiting(situation) {
            return Ok(only);
        }
        self.ask_jev(&situation.question())
    }

    /// The reasoning model alone.
    pub fn model(&self, situation: &Situation) -> Result<Decision, Failure> {
        if let Some(only) = only_waiting(situation) {
            return Ok(only);
        }
        let read = |said: &str| parse_choice(said, situation).map(|choice| (choice, ()));
        ask_model(
            self,
            &self.model,
            self.effort.as_deref(),
            &situation.question(),
            CHOICE,
            &read,
            &mut || false,
        )
        .map(|(decision, ())| decision)
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

/// The ways of deciding compared in W11.6. Exploration (C-54) decides its
/// next action by `Rules`, `Jev`, `Model` or `Escalating` (`explore`); the
/// cancelling rule is for dialogs in the way only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
                        source: failure.source,
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
    let answer = answer_object(said)?;
    let choice = answer["choice"].as_str().unwrap_or_default().to_string();
    if choice == WAIT || situation.controls.iter().any(|c| c.id == choice) {
        Ok(choice)
    } else {
        Err(format!(
            "the model chose `{choice}`, which is not an option"
        ))
    }
}

/// What an objective does, as its intent asks (the Operator's amendment
/// of C-54): whether it explores the running application first, how many
/// improvements (cycles) it makes, and whether a reviewed change that
/// passes every check is merged, and then built, tried and adopted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    pub explore: bool,
    pub cycles: u32,
    pub merge: bool,
    pub adopt: bool,
}

impl Shape {
    /// What an objective does when nothing could read its intent: one
    /// improvement without exploring, merged and adopted.
    pub const DEFAULT: Shape = Shape {
        explore: false,
        cycles: 1,
        merge: true,
        adopt: true,
    };

    /// In the Operator's words, for the start form and the thread.
    pub fn describe(&self) -> String {
        format!(
            "{}; {} at most; {}",
            if self.explore {
                "explores the running application first"
            } else {
                "does not explore first"
            },
            if self.cycles == 1 {
                "one improvement".to_string()
            } else {
                format!("{} improvements", self.cycles)
            },
            match (self.merge, self.adopt) {
                (true, true) =>
                    "merges reviewed changes that pass every check, then builds, tries and restarts in the result",
                (true, false) =>
                    "merges reviewed changes that pass every check, without building or restarting",
                _ => "opens pull requests for you to merge",
            }
        )
    }
}

/// An objective's shape as inferred from its intent, and who inferred it:
/// Jev, the reasoning model it escalated to, or nobody (the defaults, with
/// why).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inferred {
    pub shape: Shape,
    pub source: Source,
    /// Jev's least confidence among its answers, when Jev's were used.
    pub confidence: Option<f64>,
    pub millis: u64,
    /// US dollars; `None` when a call's usage or price is unknown.
    pub usd: Option<f64>,
    /// Why it escalated, or what failed.
    #[serde(default)]
    pub note: String,
}

impl Inferred {
    /// Who inferred it, in the Operator's words.
    pub fn by(&self) -> String {
        match self.source {
            Source::Jev => format!(
                "Jev read the intent (confidence {:.2})",
                self.confidence.unwrap_or(0.0)
            ),
            Source::Escalated | Source::Model => {
                format!(
                    "Jev was unsure ({}), so the escalation model read the intent",
                    self.note
                )
            }
            Source::Rules => format!(
                "Nothing could read the intent ({}): the defaults",
                self.note
            ),
        }
    }
}

/// What the typed questions about an intent are told.
const SHAPE_ABOUT: &str = "Agentique, an application for modelling systems, improves itself through objectives. An objective runs cycles, and each cycle makes one improvement: it may first explore the running application in a test instance to find problems; then a lead proposes a change, an implementer makes it, checks and a reviewer judge it, and a change that passes every check may be merged into the repository, then built, tried and adopted (Agentique restarts in the new build). The Operator's intent says what the objective should achieve.";

/// One of the typed questions about an intent.
struct ShapeQuestion {
    id: &'static str,
    instructions: &'static str,
    /// Each option, with what choosing it means.
    options: &'static [(&'static str, &'static str)],
}

/// The three questions about an intent.
const SHAPE_QUESTIONS: [ShapeQuestion; 3] = [
    ShapeQuestion {
        id: "explore",
        instructions: "Should the objective first explore the running application to find the problems it is about?",
        options: &[
            (
                "explore",
                "Yes: the intent asks to explore, test, find or look for problems, or names an area rather than a change",
            ),
            ("direct", "No: the intent names the change to make"),
        ],
    },
    ShapeQuestion {
        id: "improvements",
        instructions: "How many improvements should the objective make at most, one per cycle? One unless the intent asks for several, for continuing work or for everything it finds.",
        options: &[
            ("1", "One improvement"),
            ("2", "Two improvements"),
            ("3", "Three improvements"),
            ("5", "Five improvements"),
        ],
    },
    ShapeQuestion {
        id: "integrate",
        instructions: "What should happen to a reviewed change that passes every check? Merge, build and adopt unless the intent says otherwise.",
        options: &[
            (
                "adopt",
                "Merge it, then build, try and restart in the result",
            ),
            ("merge", "Merge it, without building or restarting"),
            ("review", "Open a pull request only; the Operator merges"),
        ],
    },
];

/// The shape the three answers name, or why they name none.
pub fn shape_of(explore: &str, improvements: &str, integrate: &str) -> Result<Shape, String> {
    let explore = match explore {
        "explore" => true,
        "direct" => false,
        other => return Err(format!("`{other}` is not an answer to whether it explores")),
    };
    let cycles = match improvements {
        "1" | "2" | "3" | "5" => improvements.parse().unwrap_or(1),
        other => return Err(format!("`{other}` is not a number of improvements")),
    };
    let (merge, adopt) = match integrate {
        "adopt" => (true, true),
        "merge" => (true, false),
        "review" => (false, false),
        other => return Err(format!("`{other}` is not what happens to a change")),
    };
    Ok(Shape {
        explore,
        cycles,
        merge,
        adopt,
    })
}

/// Jev's answers read into a shape with its least confidence, or why they
/// are not used (an answer missing or wrong in form, or one below
/// `threshold`).
pub fn shape_from_jev(
    answers: &BTreeMap<String, Answer>,
    threshold: f64,
) -> Result<(Shape, f64), String> {
    let mut chosen = BTreeMap::new();
    let mut least = 1.0_f64;
    for question in &SHAPE_QUESTIONS {
        let id = question.id;
        match answers.get(id) {
            Some(Answer::Choice {
                choice, confidence, ..
            }) => {
                if *confidence < threshold {
                    return Err(format!(
                        "Jev chose {choice} for {id} with confidence {confidence:.2}"
                    ));
                }
                least = least.min(*confidence);
                chosen.insert(id, choice.as_str());
            }
            _ => return Err(format!("Jev gave no choice for {id}")),
        }
    }
    let shape = shape_of(
        chosen["explore"],
        chosen["improvements"],
        chosen["integrate"],
    )?;
    Ok((shape, least))
}

/// A model's answer about an intent, `{"explore": …, "improvements": …,
/// "integrate": …}`, read into a shape.
pub fn read_shape(said: &str) -> Result<Shape, String> {
    let object = said
        .char_indices()
        .filter(|(_, c)| *c == '{')
        .find_map(|(at, _)| {
            let value: Value = serde_json::Deserializer::from_str(&said[at..])
                .into_iter::<Value>()
                .next()?
                .ok()?;
            value["integrate"].is_string().then_some(value)
        })
        .ok_or("the model gave no JSON naming what happens to a change")?;
    let text = |key: &str| match &object[key] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    };
    shape_of(&text("explore"), &text("improvements"), &text("integrate"))
}

impl Decider {
    /// What an objective with `intent` does (the Operator's amendment of
    /// C-54): Jev answers three typed questions about it in one request; a
    /// confident set is used; otherwise the reasoning model answers them;
    /// when neither does, the defaults, with why. Asked before Start, so
    /// the Operator sees it and may change it.
    pub fn infer(&self, intent: &str) -> Inferred {
        let started = Instant::now();
        let state = json!({ "about": SHAPE_ABOUT, "intent": intent.trim() });
        let mut usd = Some(0.0);
        let add = |total: Option<f64>, part: Option<f64>| Some(total? + part?);
        let note = match self.ask_jev_shape(&state) {
            Ok((answers, cost)) => {
                usd = add(usd, cost);
                match shape_from_jev(&answers, self.threshold) {
                    Ok((shape, confidence)) => {
                        return Inferred {
                            shape,
                            source: Source::Jev,
                            confidence: Some(confidence),
                            millis: started.elapsed().as_millis() as u64,
                            usd,
                            note: String::new(),
                        };
                    }
                    Err(why) => why,
                }
            }
            Err(failure) => {
                usd = add(usd, failure.usd);
                failure.error
            }
        };
        let questions: Vec<String> = SHAPE_QUESTIONS
            .iter()
            .map(
                |ShapeQuestion {
                     id,
                     instructions,
                     options,
                 }| {
                    let options: Vec<String> = options
                        .iter()
                        .map(|(o, about)| format!("  - {o}: {about}"))
                        .collect();
                    format!("{id}: {instructions}\n{}", options.join("\n"))
                },
            )
            .collect();
        let prompt = format!(
            "{SHAPE_ABOUT}\n\nThe Operator's intent:\n{}\n\nAnswer each question with one of its options:\n{}\n\nAnswer with JSON only: {{\"explore\": \"<option>\", \"improvements\": \"<option>\", \"integrate\": \"<option>\"}}",
            intent.trim(),
            questions.join("\n")
        );
        let mut problem = note.clone();
        for _ in 0..2 {
            match self.chat(&self.model, self.effort.as_deref(), &prompt, &mut || false) {
                Ok((said, cost)) => {
                    usd = add(usd, cost);
                    match read_shape(&said) {
                        Ok(shape) => {
                            return Inferred {
                                shape,
                                source: Source::Escalated,
                                confidence: None,
                                millis: started.elapsed().as_millis() as u64,
                                usd,
                                note,
                            };
                        }
                        Err(error) => problem = format!("{note}; the model: {error}"),
                    }
                }
                Err(error) => {
                    // A request that was sent may be billed.
                    usd = None;
                    problem = format!("{note}; the model failed: {error}");
                    break;
                }
            }
        }
        Inferred {
            shape: Shape::DEFAULT,
            source: Source::Rules,
            confidence: None,
            millis: started.elapsed().as_millis() as u64,
            usd,
            note: problem,
        }
    }

    /// Jev's answers to the three questions about an intent, in one
    /// request within its deadline, with what they cost.
    fn ask_jev_shape(
        &self,
        state: &Value,
    ) -> Result<(BTreeMap<String, Answer>, Option<f64>), Failure> {
        let started = Instant::now();
        let questions = SHAPE_QUESTIONS
            .iter()
            .map(
                |ShapeQuestion {
                     id,
                     instructions,
                     options,
                 }| {
                    (
                        id.to_string(),
                        jev::Question {
                            instructions: instructions.to_string(),
                            kind: QuestionKind::Choice {
                                options: options
                                    .iter()
                                    .map(|(o, about)| (o.to_string(), Some(about.to_string())))
                                    .collect(),
                            },
                        },
                    )
                },
            )
            .collect();
        let request = DecisionRequest {
            model: self.jev_model.clone(),
            state: state.clone(),
            questions,
        };
        let mut handle = self
            .providers
            .decide_start(request, Instant::now() + self.deadline);
        let result = loop {
            if let Some(result) = handle.next_result(Duration::from_millis(50)) {
                break result;
            }
        };
        let model = ModelRef::new(Provider::TypeSafe, &self.jev_model);
        match result {
            Ok(reply) => Ok((reply.answers, reply.usage.cost_usd(&model))),
            Err(failure) => Err(Failure {
                source: Source::Jev,
                error: format!("Jev: {failure}"),
                millis: started.elapsed().as_millis() as u64,
                usd: if failure.attempts == 0 {
                    Some(0.0)
                } else {
                    failure.usage.and_then(|u| u.cost_usd(&model))
                },
            }),
        }
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

    /// A stand-in model that answers from a list and keeps what it was asked.
    struct Said(
        std::cell::RefCell<Vec<String>>,
        std::cell::RefCell<Vec<String>>,
    );

    impl Answers for Said {
        fn jev_model(&self) -> ModelRef {
            ModelRef::new(Provider::TypeSafe, "jev-1.13.0")
        }
        fn threshold(&self) -> f64 {
            0.6
        }
        fn ask_jev(&self, _: &Question) -> Result<Decision, Failure> {
            unreachable!("no Jev here")
        }
        fn chat(
            &self,
            _: &ModelRef,
            _: Option<&str>,
            prompt: &str,
            _: &mut dyn FnMut() -> bool,
        ) -> Result<(String, Option<f64>), String> {
            self.1.borrow_mut().push(prompt.to_string());
            Ok((self.0.borrow_mut().remove(0), Some(0.01)))
        }
    }

    #[test]
    fn a_dialogs_question_reads_as_before_and_an_unreadable_answer_is_asked_again() {
        let s = situation(None, "");
        // The W11.6 prompt, word for word.
        let options: Vec<String> = s
            .options()
            .into_iter()
            .map(|(id, about)| format!("- {id}: {}", about.unwrap_or_default()))
            .collect();
        let before = format!(
            "{INSTRUCTIONS}

The state:
{}

The options:
{}

Answer with JSON only: {{\"choice\": \"<one option id>\"}}",
            serde_json::to_string_pretty(&s.state()).unwrap_or_default(),
            options.join(
                "
"
            )
        );
        assert_eq!(s.question().prompt(CHOICE), before);
        let model = ModelRef::new(Provider::DeepSeek, "deepseek-v4-pro");
        let read = |said: &str| parse_choice(said, &s).map(|c| (c, ()));
        let said = Said(
            vec![
                "thinking…".to_string(),
                "{\"choice\": \"dialog-cancel\"}".into(),
            ]
            .into(),
            Vec::new().into(),
        );
        let (decision, ()) = ask_model(
            &said,
            &model,
            None,
            &s.question(),
            CHOICE,
            &read,
            &mut || false,
        )
        .unwrap();
        assert_eq!(decision.choice, "dialog-cancel");
        assert_eq!(decision.source, Source::Model);
        assert_eq!(decision.usd, Some(0.02));
        assert_eq!(decision.note, "the model gave no JSON naming a choice");
        assert_eq!(said.1.borrow().len(), 2);
        assert_eq!(said.1.borrow()[0], before);
        // Two unreadable answers: a failure that keeps both calls' cost.
        let said = Said(
            vec!["?".to_string(), "{\"choice\": \"x\"}".into()].into(),
            Vec::new().into(),
        );
        let failed = ask_model(
            &said,
            &model,
            None,
            &s.question(),
            CHOICE,
            &read,
            &mut || false,
        )
        .unwrap_err();
        assert_eq!(failed.usd, Some(0.02));
        assert!(failed.error.contains("not an option"), "{}", failed.error);
    }

    fn choice(choice: &str, confidence: f64) -> Answer {
        Answer::Choice {
            choice: choice.into(),
            probabilities: BTreeMap::new(),
            confidence,
        }
    }

    /// The Operator's amendment of C-54: Jev's three answers make the
    /// shape when each is confident; otherwise they escalate, with why.
    #[test]
    fn an_intent_is_read_into_a_shape() {
        let answers = BTreeMap::from([
            ("explore".to_string(), choice("explore", 0.9)),
            ("improvements".to_string(), choice("2", 0.7)),
            ("integrate".to_string(), choice("adopt", 0.95)),
        ]);
        let (shape, least) = shape_from_jev(&answers, 0.6).unwrap();
        assert_eq!(
            shape,
            Shape {
                explore: true,
                cycles: 2,
                merge: true,
                adopt: true
            }
        );
        assert!((least - 0.7).abs() < 1e-9);
        let unsure = shape_from_jev(&answers, 0.8).unwrap_err();
        assert!(
            unsure.contains("improvements") && unsure.contains("0.70"),
            "{unsure}"
        );
        let mut missing = answers.clone();
        missing.remove("integrate");
        assert!(
            shape_from_jev(&missing, 0.6)
                .unwrap_err()
                .contains("integrate")
        );
        let mut odd = answers;
        odd.insert("improvements".into(), choice("4", 0.9));
        assert!(shape_from_jev(&odd, 0.6).is_err());

        assert_eq!(
            read_shape("Reasoning... {\"explore\": \"direct\", \"improvements\": 1, \"integrate\": \"review\"}").unwrap(),
            Shape {
                explore: false,
                cycles: 1,
                merge: false,
                adopt: false
            }
        );
        assert!(
            read_shape(
                "{\"explore\": \"maybe\", \"improvements\": \"1\", \"integrate\": \"adopt\"}"
            )
            .is_err()
        );
        assert!(read_shape("no JSON").is_err());
        assert!(
            Shape::DEFAULT
                .describe()
                .contains("does not explore first; one improvement at most; merges")
        );
    }
}
