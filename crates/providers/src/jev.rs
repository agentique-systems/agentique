//! TypeSafe AI's Jev: typed decisions for fast agents (C-35), through a thin
//! client until rig's `rig-typesafeai` replaces it (C-34; ROADMAP §4.11).
//!
//! Jev answers typed questions about a state: yes or no (the probability of
//! yes, and no confidence), a choice among 2–255 options, or a score on 2–10
//! ordered levels (each with probabilities and a confidence). It holds no
//! conversation, so it is never the Assistant's model. `POST /v1/systemone`
//! with a bearer key (`TYPESAFE_API_KEY`); the request id comes back in
//! `x-typesafe-request-id` (https://docs.typesafe.ai/api, read 2026-09-27).
//!
//! A reply is read completely, within [`REPLY_LIMIT`], and checked against
//! the request that was sent before anything of it is used: the model, every
//! question id and kind, the options, levels and legend, the probabilities
//! and the selection (the investigation's validation policy, C-52). A reply
//! that does not fit is an error of the provider, never a guessed answer.
//! Usage the reply leaves out is unknown, never zero.

use crate::{Error, ErrorKind, ModelRef, Provider, Usage, runtime};
use serde::de::{Deserializer, Error as _, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

pub const API_URL: &str = "https://api.typesafe.ai";
/// The decision adapter and its revision, as a run's binding names it
/// (C-52): change it when what is sent or how a reply is checked changes.
pub const ADAPTER: &str = "agq-providers decision 1";
/// A fixed version, so answers do not move under a scenario's feet; the
/// API accepts versioned ids whether or not its model list shows them.
pub const DEFAULT_MODEL: &str = "jev-1.13.0";
/// Attempts after the first for rate limits and overload (429, 529).
const RETRIES: u32 = 2;
const MAX_RETRY_WAIT: Duration = Duration::from_secs(10);
/// The longest a blocking decision ([`crate::Providers::decide`]) may take
/// (the vendor claims 70–500 ms per call). A caller with a tighter limit,
/// such as an agent's `maxLatencyMs`, gives its own deadline to
/// [`crate::Providers::decide_start`].
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// The largest successful reply read (a local safeguard, not a vendor
/// limit): a larger one is refused, not cut and parsed.
pub const REPLY_LIMIT: usize = 1 << 20;
/// The largest error reply read; only its start is shown.
const ERROR_LIMIT: usize = 16 * 1024;
/// How much of an error reply the Operator sees, in characters.
const EXCERPT: usize = 300;
/// The largest request sent (a local safeguard below the API's 64k-token
/// limit; bytes do not prove a token count).
pub const REQUEST_LIMIT: usize = 256 * 1024;
/// Probabilities sum to one within this, or, when every probability is
/// rounded to hundredths, within `min(n × 0.005, 0.02)`. A selected option
/// is a most probable one within [`TIE`]. These tolerances follow
/// `rig-typesafeai` 0.43.0's checks (the vendor publishes none); raw values
/// are never changed.
const SUM_TOLERANCE: f64 = 1e-3;
const ROUNDED_STEP: f64 = 0.005;
const ROUNDED_CAP: f64 = 0.02;
const TIE: f64 = 1e-6;
const EPSILON: f64 = 1e-12;

/// Questions about one state.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionRequest {
    /// A versioned id such as `jev-1.13.0`, or an alias such as `jev-latest`.
    pub model: String,
    /// What the questions are about: text, or a JSON object.
    pub state: Value,
    /// By id; the id is not shown to the model.
    pub questions: BTreeMap<String, Question>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub instructions: String,
    pub kind: QuestionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuestionKind {
    /// Yes or no, optionally with what yes and what no mean (both or
    /// neither, as the API requires).
    YesNo { meanings: Option<(String, String)> },
    /// One of these options (2–255), each with an optional description.
    Choice {
        options: BTreeMap<String, Option<String>>,
    },
    /// A level on this ordered scale (2–10 levels, low to high).
    Score { levels: Vec<String> },
}

/// The answers, by question id, checked against the request.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionReply {
    /// The versioned model that answered.
    pub model: String,
    pub answers: BTreeMap<String, Answer>,
    pub usage: DecisionUsage,
    pub request_id: Option<String>,
    /// Requests sent, retries included.
    pub attempts: u32,
}

/// An answer as the model gave it; nothing is rounded or renormalised.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// The probability of yes. Not a confidence: 0.1 favours no.
    YesNo { yes: f64 },
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        /// How concentrated the distribution is, as the vendor computes it;
        /// a claim of the model, not a measured reliability.
        confidence: f64,
    },
    /// `score` is the probability-weighted level (it can fall between
    /// levels); `probabilities` are by level index.
    Score {
        score: f64,
        probabilities: BTreeMap<usize, f64>,
        confidence: f64,
    },
}

/// Tokens one decision reported. A count the reply left out is `None`,
/// never zero, so a call whose usage is unknown never looks free (A4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl DecisionUsage {
    /// Every count is known.
    pub fn is_complete(&self) -> bool {
        self.input_tokens.is_some() && self.output_tokens.is_some()
    }

    /// No count is known.
    pub fn is_unknown(&self) -> bool {
        self.input_tokens.is_none() && self.output_tokens.is_none()
    }

    /// The counts that are known; read [`is_complete`](Self::is_complete)
    /// before taking them as the whole.
    pub fn known(&self) -> Usage {
        Usage {
            input_tokens: self.input_tokens.unwrap_or_default(),
            output_tokens: self.output_tokens.unwrap_or_default(),
            ..Usage::default()
        }
    }

    /// The estimated cost, only when every count is known and the model has
    /// a price; otherwise unknown.
    pub fn cost_usd(&self, model: &ModelRef) -> Option<f64> {
        if self.is_complete() {
            self.known().cost_usd(model)
        } else {
            None
        }
    }
}

/// A decision that failed: why, how many requests were sent (a sent request
/// may be billed even when it failed), and the usage a reply reported, if
/// any reached us.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionFailure {
    pub error: Error,
    /// Requests sent; 0 when it failed before sending anything (so it cost
    /// nothing).
    pub attempts: u32,
    pub usage: Option<DecisionUsage>,
    pub request_id: Option<String>,
}

impl DecisionFailure {
    fn before_sending(kind: ErrorKind, message: String) -> DecisionFailure {
        DecisionFailure {
            error: Error { kind, message },
            attempts: 0,
            usage: None,
            request_id: None,
        }
    }
}

impl std::fmt::Display for DecisionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for DecisionFailure {}

/// Whether `model` is a fixed version (`jev-1.13.0`), not an alias that can
/// move (`jev-latest`, `jev-preview`).
pub fn is_versioned(model: &str) -> bool {
    let Some(version) = model.strip_prefix("jev-") else {
        return false;
    };
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

impl DecisionRequest {
    /// The request body, after checking the limits the API documents and
    /// the local ones.
    pub(crate) fn body(&self) -> Result<Value, String> {
        if self.model.trim().is_empty() {
            return Err("A decision needs a model.".into());
        }
        match &self.state {
            Value::String(text) if !text.trim().is_empty() => {}
            Value::Object(_) => {}
            _ => return Err("The state of a decision must be text or a JSON object.".into()),
        }
        if self.questions.is_empty() {
            return Err("A decision needs at least one question.".into());
        }
        let nonempty = |text: &str| !text.trim().is_empty();
        let mut questions = serde_json::Map::new();
        for (id, question) in &self.questions {
            if !nonempty(id) {
                return Err("A question id must not be empty.".into());
            }
            if !nonempty(&question.instructions) {
                return Err(format!("`{id}`: a question needs instructions."));
            }
            let value = match &question.kind {
                QuestionKind::YesNo { meanings } => {
                    let mut value =
                        json!({ "type": "noul", "instructions": question.instructions });
                    if let Some((yes, no)) = meanings {
                        if !nonempty(yes) || !nonempty(no) {
                            return Err(format!("`{id}`: say what both yes and no mean."));
                        }
                        value["criteria"] = json!({ "true": yes, "false": no });
                    }
                    value
                }
                QuestionKind::Choice { options } => {
                    if !(2..=255).contains(&options.len())
                        || options.keys().any(|option| !nonempty(option))
                        || options
                            .values()
                            .any(|described| described.as_deref().is_some_and(|d| !nonempty(d)))
                    {
                        return Err(format!(
                            "`{id}`: a choice needs 2 to 255 named options, each description not empty."
                        ));
                    }
                    json!({ "type": "choice", "instructions": question.instructions, "criteria": options })
                }
                QuestionKind::Score { levels } => {
                    if !(2..=10).contains(&levels.len()) || levels.iter().any(|l| !nonempty(l)) {
                        return Err(format!("`{id}`: a score needs 2 to 10 named levels."));
                    }
                    json!({ "type": "score", "instructions": question.instructions, "criteria": levels })
                }
            };
            questions.insert(id.clone(), value);
        }
        let body = json!({ "state": self.state, "model": self.model, "questions": questions });
        let size = body.to_string().len();
        if size > REQUEST_LIMIT {
            return Err(format!(
                "The decision request is {size} bytes, over the local limit of {REQUEST_LIMIT}."
            ));
        }
        Ok(body)
    }
}

/// A JSON object whose keys must be unique. serde_json's maps keep the last
/// of two equal keys, which would discard evidence before it is checked.
struct Unique<V>(BTreeMap<String, V>);

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Unique<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Keys<V>(PhantomData<V>);
        impl<'de, V: Deserialize<'de>> Visitor<'de> for Keys<V> {
            type Value = Unique<V>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an object with unique keys")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique<V>, A::Error> {
                let mut out = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, V>()? {
                    if out.contains_key(&key) {
                        return Err(A::Error::custom(format!("the key `{key}` appears twice")));
                    }
                    out.insert(key, value);
                }
                Ok(Unique(out))
            }
        }
        deserializer.deserialize_map(Keys(PhantomData))
    }
}

/// The wire form of a reply. A field named twice is an error (serde's
/// derive); fields this version does not know are ignored.
#[derive(Deserialize)]
struct WireReply {
    model: String,
    answers: Unique<WireAnswer>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Deserialize)]
struct WireAnswer {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    noul: Option<f64>,
    #[serde(default)]
    choice: Option<String>,
    #[serde(default)]
    probabilities: Option<Unique<f64>>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    score: Option<f64>,
    #[serde(default)]
    legend: Option<Unique<String>>,
}

/// Counts must be whole and not negative; a missing count stays missing.
#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
}

fn invalid(message: String) -> Error {
    Error {
        kind: ErrorKind::InvalidReply,
        message: format!("TypeSafe AI's reply could not be used: {message}."),
    }
}

/// Reads a complete reply body and checks it against `request`.
pub(crate) fn read_reply(
    request: &DecisionRequest,
    body: &[u8],
    request_id: Option<String>,
    attempts: u32,
) -> Result<DecisionReply, Error> {
    let text =
        std::str::from_utf8(body).map_err(|error| invalid(format!("it is not UTF-8 ({error})")))?;
    let wire: WireReply = serde_json::from_str(text)
        .map_err(|error| invalid(format!("it is not the expected JSON ({error})")))?;
    let usage = wire
        .usage
        .map_or(DecisionUsage::default(), |u| DecisionUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
        });
    let model = check_model(&request.model, &wire.model).map_err(invalid)?;
    let answers = check_answers(request, wire.answers.0).map_err(invalid)?;
    Ok(DecisionReply {
        model,
        answers,
        usage,
        request_id,
        attempts,
    })
}

/// The usage a body reports, if it reads at all: kept on a failure, so a
/// reply that was billed but not usable still counts.
fn usage_of(body: &[u8]) -> Option<DecisionUsage> {
    #[derive(Deserialize)]
    struct Only {
        usage: Option<WireUsage>,
    }
    let only: Only = serde_json::from_slice(body).ok()?;
    only.usage.map(|u| DecisionUsage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
    })
}

/// The answering model: the pinned one itself, or for an alias a fixed
/// version.
fn check_model(asked: &str, answered: &str) -> Result<String, String> {
    if is_versioned(asked) {
        if answered != asked {
            return Err(format!(
                "`{asked}` was asked for, but `{answered}` answered"
            ));
        }
    } else if !is_versioned(answered) {
        return Err(format!(
            "the alias `{asked}` was answered by `{answered}`, which is not a fixed version"
        ));
    }
    Ok(answered.to_string())
}

fn check_answers(
    request: &DecisionRequest,
    mut answers: BTreeMap<String, WireAnswer>,
) -> Result<BTreeMap<String, Answer>, String> {
    let asked: BTreeSet<&String> = request.questions.keys().collect();
    let got: BTreeSet<&String> = answers.keys().collect();
    if let Some(missing) = asked.difference(&got).next() {
        return Err(format!("question `{missing}` has no answer"));
    }
    if let Some(extra) = got.difference(&asked).next() {
        return Err(format!("it answers `{extra}`, which was not asked"));
    }
    let mut out = BTreeMap::new();
    for (id, question) in &request.questions {
        let answer = answers.remove(id).expect("checked above");
        let checked = check_answer(&question.kind, answer)
            .map_err(|why| format!("question `{id}`: {why}"))?;
        out.insert(id.clone(), checked);
    }
    Ok(out)
}

fn unit(name: &str, value: Option<f64>) -> Result<f64, String> {
    match value {
        None => Err(format!("`{name}` is missing")),
        Some(v) if !v.is_finite() || !(0.0..=1.0).contains(&v) => {
            Err(format!("`{name}` is {v}, not between 0 and 1"))
        }
        Some(v) => Ok(v),
    }
}

fn check_answer(kind: &QuestionKind, answer: WireAnswer) -> Result<Answer, String> {
    let expected = match kind {
        QuestionKind::YesNo { .. } => "noul",
        QuestionKind::Choice { .. } => "choice",
        QuestionKind::Score { .. } => "score",
    };
    if answer.kind != expected {
        return Err(format!(
            "a `{expected}` was asked for, but a `{}` came back",
            answer.kind
        ));
    }
    match kind {
        QuestionKind::YesNo { .. } => Ok(Answer::YesNo {
            yes: unit("noul", answer.noul)?,
        }),
        QuestionKind::Choice { options } => {
            let choice = answer.choice.ok_or("`choice` is missing")?;
            let probabilities = answer.probabilities.ok_or("`probabilities` is missing")?.0;
            let named: BTreeSet<&String> = options.keys().collect();
            let given: BTreeSet<&String> = probabilities.keys().collect();
            if let Some(missing) = named.difference(&given).next() {
                return Err(format!("the option `{missing}` has no probability"));
            }
            if let Some(extra) = given.difference(&named).next() {
                return Err(format!("`{extra}` is not one of the options"));
            }
            for (option, p) in &probabilities {
                unit(&format!("probabilities.{option}"), Some(*p))?;
            }
            check_sum(probabilities.values().copied())?;
            if !options.contains_key(&choice) {
                return Err(format!("the selected `{choice}` is not one of the options"));
            }
            let best = probabilities.values().copied().fold(0.0, f64::max);
            if probabilities[&choice] + TIE < best {
                return Err(format!(
                    "the selected `{choice}` (p = {}) is not the most probable option (p = {best})",
                    probabilities[&choice]
                ));
            }
            let confidence = unit("confidence", answer.confidence)?;
            Ok(Answer::Choice {
                choice,
                probabilities,
                confidence,
            })
        }
        QuestionKind::Score { levels } => {
            let legend = answer.legend.ok_or("`legend` is missing")?.0;
            let legend = indexed("legend", legend, levels.len())?;
            for (index, level) in levels.iter().enumerate() {
                if legend[&index] != *level {
                    return Err(format!(
                        "the legend names level {index} `{}`, but `{level}` was asked",
                        legend[&index]
                    ));
                }
            }
            let probabilities = answer.probabilities.ok_or("`probabilities` is missing")?.0;
            let probabilities = indexed("probabilities", probabilities, levels.len())?;
            for (index, p) in &probabilities {
                unit(&format!("probabilities.{index}"), Some(*p))?;
            }
            let sum = check_sum(probabilities.values().copied())?;
            let top = (levels.len() - 1) as f64;
            let score = match answer.score {
                None => return Err("`score` is missing".into()),
                Some(s) if !s.is_finite() || !(0.0..=top).contains(&s) => {
                    return Err(format!("`score` is {s}, not between 0 and {top}"));
                }
                Some(s) => s,
            };
            // The score is the probability-weighted level; the mean is worked
            // out on a normalised copy, the probabilities stay as given.
            let mean = probabilities
                .iter()
                .map(|(i, p)| *i as f64 * p)
                .sum::<f64>()
                / sum;
            let allowed = if rounded(probabilities.values().copied()) {
                probabilities
                    .keys()
                    .map(|i| (*i as f64 - mean).abs() * ROUNDED_STEP)
                    .sum::<f64>()
                    + ROUNDED_STEP
                    + EPSILON
            } else {
                ROUNDED_STEP + EPSILON
            };
            if (score - mean).abs() > allowed {
                return Err(format!(
                    "`score` is {score}, but its probabilities put it at {mean:.4}"
                ));
            }
            let confidence = unit("confidence", answer.confidence)?;
            Ok(Answer::Score {
                score,
                probabilities,
                confidence,
            })
        }
    }
}

/// Keys `"0"` to `"n-1"`, each exactly once in its canonical form (`"00"`,
/// `"-1"`, `"+1"` and `"x"` are refused, never parsed or dropped).
fn indexed<V>(
    name: &str,
    map: BTreeMap<String, V>,
    n: usize,
) -> Result<BTreeMap<usize, V>, String> {
    let mut out = BTreeMap::new();
    for (key, value) in map {
        let index = key
            .parse::<usize>()
            .ok()
            .filter(|i| *i < n && i.to_string() == key)
            .ok_or_else(|| {
                format!(
                    "`{name}` has the key `{key}`, not a level index 0 to {}",
                    n - 1
                )
            })?;
        out.insert(index, value);
    }
    if let Some(missing) = (0..n).find(|i| !out.contains_key(i)) {
        return Err(format!("`{name}` has no level {missing}"));
    }
    Ok(out)
}

/// Every probability a multiple of 0.01.
fn rounded(probabilities: impl Iterator<Item = f64>) -> bool {
    probabilities
        .into_iter()
        .all(|p| ((p * 100.0) - (p * 100.0).round()).abs() <= 1e-9)
}

/// The sum, if the probabilities sum to one within the tolerance.
fn check_sum(probabilities: impl Iterator<Item = f64> + Clone) -> Result<f64, String> {
    let n = probabilities.clone().count() as f64;
    let sum: f64 = probabilities.clone().sum();
    if sum <= 0.0 {
        return Err("every probability is zero".into());
    }
    let allowed = if rounded(probabilities) {
        (n * ROUNDED_STEP).min(ROUNDED_CAP) + EPSILON
    } else {
        SUM_TOLERANCE
    };
    if (sum - 1.0).abs() > allowed {
        return Err(format!("the probabilities sum to {sum}, not 1"));
    }
    Ok(sum)
}

/// How a body ended.
enum Ending {
    Complete,
    /// Longer than the limit; what was read is kept.
    TooLarge,
    /// The connection failed while reading.
    Cut(String),
}

/// Reads a body up to `limit` bytes, completely when it fits.
async fn read_body(response: &mut reqwest::Response, limit: usize) -> (Vec<u8>, Ending) {
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len() + chunk.len() > limit {
                    let room = limit - bytes.len();
                    bytes.extend_from_slice(&chunk[..room]);
                    return (bytes, Ending::TooLarge);
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => return (bytes, Ending::Complete),
            Err(error) => return (bytes, Ending::Cut(error.without_url().to_string())),
        }
    }
}

/// The start of an error reply, for the Operator: control characters and
/// runs of white space folded to one space, at most [`EXCERPT`] characters,
/// and never the key.
fn excerpt(body: &[u8], key: &str) -> String {
    let text = String::from_utf8_lossy(body);
    let mut folded = text
        .split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !key.trim().is_empty() {
        folded = folded.replace(key.trim(), "…");
    }
    if folded.is_empty() {
        "no details".into()
    } else if folded.chars().count() > EXCERPT {
        let mut short: String = folded.chars().take(EXCERPT).collect();
        short.push('…');
        short
    } else {
        folded
    }
}

/// What a running decision shares with its handle.
pub(crate) struct Running {
    pub(crate) result: std::sync::mpsc::Receiver<Result<DecisionReply, DecisionFailure>>,
    pub(crate) abort: futures::future::AbortHandle,
    pub(crate) cancelled: Arc<AtomicBool>,
    pub(crate) attempts: Arc<AtomicU32>,
}

/// Starts one decision on the background runtime, to finish by `deadline`
/// (monotonic): every request, read, retry and wait shares it. Retries rate
/// limits and overload twice, and only when the wait still fits. Nothing
/// more is sent once `cancelled` is set or the task is aborted.
pub(crate) fn start(
    request: DecisionRequest,
    key: Option<String>,
    endpoint: Option<String>,
    deadline: Instant,
) -> Running {
    let (sender, result) = std::sync::mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let attempts = Arc::new(AtomicU32::new(0));
    let task = {
        let (cancelled, attempts) = (cancelled.clone(), attempts.clone());
        async move {
            let outcome = run(request, key, endpoint, deadline, &cancelled, &attempts).await;
            let _ = sender.send(outcome);
        }
    };
    let abort = runtime::spawn(task);
    Running {
        result,
        abort,
        cancelled,
        attempts,
    }
}

/// The error of a decision that was stopped.
pub(crate) fn cancelled(attempts: u32) -> DecisionFailure {
    DecisionFailure {
        error: Error {
            kind: ErrorKind::Cancelled,
            message: "Stopped.".into(),
        },
        attempts,
        usage: None,
        request_id: None,
    }
}

/// The error of a decision that ran out of time.
pub(crate) fn timed_out(attempts: u32, waited: Duration) -> DecisionFailure {
    DecisionFailure {
        error: Error {
            kind: ErrorKind::TimedOut,
            message: format!(
                "TypeSafe AI did not answer within {} ms, the time this decision had.",
                waited.as_millis()
            ),
        },
        attempts,
        usage: None,
        request_id: None,
    }
}

/// One decision, from checking the request to the checked reply.
async fn run(
    request: DecisionRequest,
    key: Option<String>,
    endpoint: Option<String>,
    deadline: Instant,
    stopped: &AtomicBool,
    sent: &AtomicU32,
) -> Result<DecisionReply, DecisionFailure> {
    let started = Instant::now();
    let key = key.filter(|key| !key.trim().is_empty()).ok_or_else(|| {
        DecisionFailure::before_sending(
            ErrorKind::MissingKey,
            format!(
                "No TypeSafe AI key is set. Set {} and restart Agentique.",
                Provider::TypeSafe.key_variable()
            ),
        )
    })?;
    let body = request
        .body()
        .map_err(|message| DecisionFailure::before_sending(ErrorKind::Rejected, message))?;
    let url = format!(
        "{}/v1/systemone",
        endpoint.as_deref().unwrap_or(API_URL).trim_end_matches('/')
    );
    let client = client();
    let until = tokio::time::Instant::from_std(deadline);
    let attempts = || sent.load(Ordering::SeqCst);
    let failure = |error: Error, usage, request_id| DecisionFailure {
        error,
        attempts: attempts(),
        usage,
        request_id,
    };
    loop {
        // Checked right before each request: nothing is sent once stopped
        // or out of time.
        if stopped.load(Ordering::SeqCst) {
            return Err(cancelled(attempts()));
        }
        if Instant::now() >= deadline {
            return Err(timed_out(attempts(), started.elapsed()));
        }
        sent.fetch_add(1, Ordering::SeqCst);
        let attempt = attempts();
        let response = tokio::time::timeout_at(
            until,
            client.post(&url).bearer_auth(&key).json(&body).send(),
        )
        .await;
        let mut response = match response {
            Err(_) => return Err(timed_out(attempt, started.elapsed())),
            Ok(Ok(response)) => response,
            Ok(Err(error)) => {
                return Err(failure(
                    Error {
                        kind: ErrorKind::Unreachable,
                        message: format!(
                            "Could not reach TypeSafe AI ({}). Check the network connection and try again.",
                            error.without_url()
                        ),
                    },
                    None,
                    None,
                ));
            }
        };
        let status = response.status().as_u16();
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        };
        let request_id = header("x-typesafe-request-id");
        let wait = header("retry-after-ms")
            .and_then(|ms| ms.trim().parse().ok())
            .map(Duration::from_millis)
            .or_else(|| {
                header("retry-after")
                    .and_then(|seconds| seconds.trim().parse().ok())
                    .map(Duration::from_secs)
            });
        if (200..300).contains(&status) {
            // A reply announced as too large is refused before reading.
            let read = if response
                .content_length()
                .is_some_and(|n| n > REPLY_LIMIT as u64)
            {
                Ok((Vec::new(), Ending::TooLarge))
            } else {
                tokio::time::timeout_at(until, read_body(&mut response, REPLY_LIMIT)).await
            };
            let Ok((bytes, ending)) = read else {
                return Err(DecisionFailure {
                    request_id,
                    ..timed_out(attempt, started.elapsed())
                });
            };
            // A reply that arrives after a stop is never delivered.
            if stopped.load(Ordering::SeqCst) {
                return Err(cancelled(attempt));
            }
            return match ending {
                Ending::Complete => read_reply(&request, &bytes, request_id.clone(), attempt)
                    .map_err(|error| failure(error, usage_of(&bytes), request_id)),
                Ending::TooLarge => Err(failure(
                    Error {
                        kind: ErrorKind::InvalidReply,
                        message: format!(
                            "TypeSafe AI's reply is larger than {} KiB, the local limit, so it was not read.",
                            REPLY_LIMIT / 1024
                        ),
                    },
                    None,
                    request_id,
                )),
                Ending::Cut(why) => Err(failure(
                    Error {
                        kind: ErrorKind::Unreachable,
                        message: format!(
                            "TypeSafe AI's reply was cut off while it was read ({why})."
                        ),
                    },
                    None,
                    request_id,
                )),
            };
        }
        // An error body is shown to the Operator: only its start.
        let (bytes, _) = tokio::time::timeout_at(until, read_body(&mut response, ERROR_LIMIT))
            .await
            .unwrap_or((Vec::new(), Ending::Complete));
        if stopped.load(Ordering::SeqCst) {
            return Err(cancelled(attempt));
        }
        let text = excerpt(&bytes, &key);
        let mut no_time = false;
        if matches!(status, 429 | 529) && attempt <= RETRIES {
            let wait = wait.unwrap_or(Duration::from_secs(u64::from(attempt)));
            // A retry only when its wait fits before the deadline, leaving
            // time to ask again.
            if wait <= MAX_RETRY_WAIT && Instant::now() + wait < deadline {
                runtime::sleep(wait).await;
                continue;
            }
            no_time = wait <= MAX_RETRY_WAIT;
        }
        let (kind, mut message) = match status {
            401 => (
                ErrorKind::KeyRefused,
                format!(
                    "TypeSafe AI did not accept the API key (401). Check {}.",
                    Provider::TypeSafe.key_variable()
                ),
            ),
            402 | 403 => (
                ErrorKind::NoAccess,
                format!("This TypeSafe AI key may not make this request ({status}): {text}"),
            ),
            404 => (
                ErrorKind::UnknownModel,
                "TypeSafe AI does not know this model (404).".to_string(),
            ),
            400 | 413 | 422 => (
                ErrorKind::Rejected,
                format!("TypeSafe AI did not accept the questions ({status}): {text}"),
            ),
            429 => (
                ErrorKind::RateLimited,
                "The TypeSafe AI rate limit was reached (429). Try again in a minute.".into(),
            ),
            500..=599 => (
                ErrorKind::Unavailable,
                format!(
                    "TypeSafe AI is overloaded or unavailable ({status}). Try again in a moment."
                ),
            ),
            _ => (
                ErrorKind::Other,
                format!("TypeSafe AI answered with an error ({status}): {text}"),
            ),
        };
        if no_time {
            message.push_str(" No time was left to ask again before the decision's deadline.");
        }
        return Err(failure(Error { kind, message }, None, request_id));
    }
}

/// One HTTP client for every decision, so connections and TLS sessions are
/// reused (a fast agent may decide many times a second).
fn client() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triage() -> DecisionRequest {
        DecisionRequest {
            model: DEFAULT_MODEL.into(),
            state: json!("Short link https://example.test/free-prizes created by a new account."),
            questions: BTreeMap::from([
                (
                    "suspicious".to_string(),
                    Question {
                        instructions: "Is this link likely abuse?".into(),
                        kind: QuestionKind::YesNo { meanings: None },
                    },
                ),
                (
                    "action".to_string(),
                    Question {
                        instructions: "What should happen to the link?".into(),
                        kind: QuestionKind::Choice {
                            options: BTreeMap::from([
                                ("activate".to_string(), None),
                                ("hold".to_string(), Some("Hold for review".to_string())),
                            ]),
                        },
                    },
                ),
                (
                    "risk".to_string(),
                    Question {
                        instructions: "How risky is it?".into(),
                        kind: QuestionKind::Score {
                            levels: vec!["low".into(), "medium".into(), "high".into()],
                        },
                    },
                ),
            ]),
        }
    }

    const NOUL: &str = r#""suspicious":{"type":"noul","noul":0.93}"#;
    const CHOICE: &str = r#""action":{"type":"choice","choice":"hold","probabilities":{"hold":0.9,"activate":0.1},"confidence":0.87}"#;
    const SCORE: &str = r#""risk":{"type":"score","score":1.2,"legend":{"0":"low","1":"medium","2":"high"},"probabilities":{"0":0.1,"1":0.6,"2":0.3},"confidence":0.5}"#;
    const USAGE: &str = r#""usage":{"input_tokens":120,"output_tokens":8}"#;

    /// A reply with these answers (and usage) to [`triage`].
    fn reply(answers: &[&str], usage: &str) -> String {
        let usage = if usage.is_empty() {
            String::new()
        } else {
            format!(",{usage}")
        };
        format!(
            r#"{{"model":"jev-1.13.0","answers":{{{}}}{usage}}}"#,
            answers.join(",")
        )
    }

    fn read(body: &str) -> Result<DecisionReply, Error> {
        read_reply(&triage(), body.as_bytes(), Some("req_1".into()), 1)
    }

    /// The reply with one answer replaced, which must be refused with a
    /// message naming `why`.
    fn refused(choice: &str, score: &str, noul: &str, why: &str) {
        let body = reply(&[noul, choice, score], USAGE);
        match read(&body) {
            Ok(reply) => panic!("accepted {body}: {reply:?}"),
            Err(error) => {
                assert_eq!(error.kind, ErrorKind::InvalidReply, "{error}");
                assert!(error.message.contains(why), "{why:?} not in {error}");
            }
        }
    }

    fn accepted(choice: &str, score: &str, noul: &str) -> DecisionReply {
        let body = reply(&[noul, choice, score], USAGE);
        read(&body).unwrap_or_else(|error| panic!("refused {body}: {error}"))
    }

    #[test]
    fn the_body_follows_the_api() {
        let body = triage().body().unwrap();
        assert_eq!(body["model"], "jev-1.13.0");
        assert_eq!(
            body["questions"]["suspicious"],
            json!({ "type": "noul", "instructions": "Is this link likely abuse?" })
        );
        assert_eq!(
            body["questions"]["action"]["criteria"]["hold"],
            "Hold for review"
        );
        assert_eq!(
            body["questions"]["action"]["criteria"]["activate"],
            Value::Null
        );
        assert_eq!(
            body["questions"]["risk"]["criteria"],
            json!(["low", "medium", "high"])
        );
    }

    #[test]
    fn yes_and_no_are_described_together() {
        let mut request = triage();
        request.questions.insert(
            "urgent".into(),
            Question {
                instructions: "Is it urgent?".into(),
                kind: QuestionKind::YesNo {
                    meanings: Some(("It must be handled now".into(), "It can wait".into())),
                },
            },
        );
        let body = request.body().unwrap();
        assert_eq!(
            body["questions"]["urgent"]["criteria"],
            json!({ "true": "It must be handled now", "false": "It can wait" })
        );
    }

    #[test]
    fn limits_are_checked_before_sending() {
        let refused = |change: &dyn Fn(&mut DecisionRequest), why: &str| {
            let mut request = triage();
            change(&mut request);
            let error = request.body().unwrap_err();
            assert!(error.contains(why), "{why:?} not in {error:?}");
        };
        refused(
            &|r| {
                r.questions.get_mut("risk").unwrap().kind = QuestionKind::Score {
                    levels: vec!["low".into()],
                }
            },
            "2 to 10",
        );
        refused(&|r| r.questions.clear(), "at least one question");
        refused(&|r| r.model = " ".into(), "needs a model");
        refused(&|r| r.state = Value::Null, "text or a JSON object");
        refused(&|r| r.state = json!([1, 2]), "text or a JSON object");
        refused(&|r| r.state = json!("  "), "text or a JSON object");
        refused(
            &|r| r.questions.get_mut("action").unwrap().instructions = "".into(),
            "needs instructions",
        );
        refused(
            &|r| {
                r.questions.get_mut("action").unwrap().kind = QuestionKind::Choice {
                    options: BTreeMap::from([
                        ("a".to_string(), Some(" ".to_string())),
                        ("b".to_string(), None),
                    ]),
                }
            },
            "2 to 255",
        );
        refused(
            &|r| {
                r.questions.get_mut("suspicious").unwrap().kind = QuestionKind::YesNo {
                    meanings: Some(("yes".into(), "".into())),
                }
            },
            "both yes and no",
        );
        refused(
            &|r| r.state = json!("x".repeat(REQUEST_LIMIT)),
            "over the local limit",
        );
    }

    #[test]
    fn a_reply_is_read_against_its_request() {
        let reply = read(&reply(&[NOUL, CHOICE, SCORE], USAGE)).unwrap();
        assert_eq!(reply.answers["suspicious"], Answer::YesNo { yes: 0.93 });
        assert!(
            matches!(&reply.answers["action"], Answer::Choice { choice, confidence, .. } if choice == "hold" && *confidence == 0.87)
        );
        assert!(
            matches!(&reply.answers["risk"], Answer::Score { probabilities, .. } if probabilities[&1] == 0.6)
        );
        assert_eq!(reply.usage.input_tokens, Some(120));
        assert!(reply.usage.is_complete());
        assert_eq!(reply.request_id.as_deref(), Some("req_1"));
        assert_eq!(reply.attempts, 1);
    }

    /// A4: missing usage is unknown, partial usage is partial, a reported
    /// zero is a known zero, and only complete usage has a cost.
    #[test]
    fn usage_says_what_is_known() {
        let model = ModelRef::new(Provider::TypeSafe, DEFAULT_MODEL);
        let usage = |text: &str| read(&reply(&[NOUL, CHOICE, SCORE], text)).map(|r| r.usage);
        let omitted = usage("").unwrap();
        assert!(omitted.is_unknown() && !omitted.is_complete());
        assert_eq!(omitted.cost_usd(&model), None);
        let null = usage(r#""usage":null"#).unwrap();
        assert!(null.is_unknown());
        let partial = usage(r#""usage":{"input_tokens":120}"#).unwrap();
        assert_eq!(partial.input_tokens, Some(120));
        assert_eq!(partial.output_tokens, None);
        assert!(!partial.is_complete() && !partial.is_unknown());
        assert_eq!(partial.cost_usd(&model), None);
        let zero = usage(r#""usage":{"input_tokens":0,"output_tokens":0}"#).unwrap();
        assert!(zero.is_complete());
        assert_eq!(zero.cost_usd(&model), Some(0.0));
        let full = usage(USAGE).unwrap();
        assert!(full.cost_usd(&model).is_some_and(|c| c > 0.0));
        for bad in [
            r#""usage":{"input_tokens":-1,"output_tokens":0}"#,
            r#""usage":{"input_tokens":1.5,"output_tokens":0}"#,
            r#""usage":{"input_tokens":"12","output_tokens":0}"#,
        ] {
            let error = usage(bad).unwrap_err();
            assert_eq!(error.kind, ErrorKind::InvalidReply, "{bad}: {error}");
        }
    }

    #[test]
    fn the_envelope_must_answer_exactly_what_was_asked() {
        // The pinned model, and only it.
        let other = reply(&[NOUL, CHOICE, SCORE], USAGE).replace("jev-1.13.0", "jev-1.14.0");
        assert!(
            read(&other)
                .unwrap_err()
                .message
                .contains("`jev-1.14.0` answered")
        );
        // An alias is answered by a fixed version, never another alias.
        let mut alias = triage();
        alias.model = "jev-latest".into();
        let body = reply(&[NOUL, CHOICE, SCORE], USAGE);
        assert_eq!(
            read_reply(&alias, body.as_bytes(), None, 1).unwrap().model,
            "jev-1.13.0"
        );
        let moving = body.replace("jev-1.13.0", "jev-preview");
        assert!(
            read_reply(&alias, moving.as_bytes(), None, 1)
                .unwrap_err()
                .message
                .contains("not a fixed version")
        );
        // Every question answered once, nothing else.
        let missing = reply(&[NOUL, CHOICE], USAGE);
        assert!(
            read(&missing)
                .unwrap_err()
                .message
                .contains("`risk` has no answer")
        );
        let extra = reply(
            &[NOUL, CHOICE, SCORE, r#""other":{"type":"noul","noul":0.5}"#],
            USAGE,
        );
        assert!(
            read(&extra)
                .unwrap_err()
                .message
                .contains("`other`, which was not asked")
        );
        let twice = reply(&[NOUL, CHOICE, SCORE, NOUL], USAGE);
        assert!(read(&twice).unwrap_err().message.contains("appears twice"));
        let field_twice = reply(&[NOUL, CHOICE, SCORE], USAGE).replacen(
            r#""model":"jev-1.13.0","#,
            r#""model":"jev-1.13.0","model":"jev-1.13.0","#,
            1,
        );
        assert!(
            read(&field_twice)
                .unwrap_err()
                .message
                .contains("duplicate field")
        );
        // The kind that was asked.
        refused(
            CHOICE,
            SCORE,
            r#""suspicious":{"type":"choice","choice":"a","probabilities":{"a":1},"confidence":1}"#,
            "a `noul` was asked for, but a `choice` came back",
        );
        // Not JSON, and numbers JSON cannot carry (NaN, infinity).
        for bad in ["", "{", "[]"] {
            assert_eq!(
                read(bad).unwrap_err().kind,
                ErrorKind::InvalidReply,
                "{bad}"
            );
        }
        for number in ["NaN", "Infinity", "1e400", "-1e400"] {
            let body = reply(&[NOUL, CHOICE, SCORE], USAGE).replace("0.93", number);
            let error = read(&body).unwrap_err();
            assert!(
                error.message.contains("not the expected JSON"),
                "{number}: {error}"
            );
        }
        let error = read_reply(&triage(), &[b'{', 0xff, b'}'], None, 1).unwrap_err();
        assert!(error.message.contains("not UTF-8"), "{error}");
    }

    #[test]
    fn a_yes_or_no_is_a_probability() {
        for yes in ["0", "1", "0.0", "1.0", "-0.0"] {
            let noul = format!(r#""suspicious":{{"type":"noul","noul":{yes}}}"#);
            accepted(CHOICE, SCORE, &noul);
        }
        for (yes, why) in [
            ("1.0000001", "not between 0 and 1"),
            ("-0.01", "not between 0 and 1"),
            ("null", "`noul` is missing"),
        ] {
            let noul = format!(r#""suspicious":{{"type":"noul","noul":{yes}}}"#);
            refused(CHOICE, SCORE, &noul, why);
        }
    }

    fn choice(choice: &str, hold: &str, activate: &str, confidence: &str) -> String {
        format!(
            r#""action":{{"type":"choice","choice":"{choice}","probabilities":{{"hold":{hold},"activate":{activate}}},"confidence":{confidence}}}"#
        )
    }

    #[test]
    fn a_choice_is_one_of_the_options_at_a_maximum() {
        // Exact ties and maxima within 1e-6 are accepted.
        accepted(&choice("hold", "0.5", "0.5", "0.5"), SCORE, NOUL);
        accepted(
            &choice("activate", "0.5000005", "0.4999995", "0.5"),
            SCORE,
            NOUL,
        );
        accepted(&choice("hold", "1", "0", "1"), SCORE, NOUL);
        refused(
            &choice("activate", "0.6", "0.4", "0.6"),
            SCORE,
            NOUL,
            "is not the most probable",
        );
        refused(
            &choice("maybe", "0.6", "0.4", "0.6"),
            SCORE,
            NOUL,
            "the selected `maybe` is not one of the options",
        );
        refused(
            r#""action":{"type":"choice","choice":"hold","probabilities":{"hold":0.9},"confidence":0.9}"#,
            SCORE,
            NOUL,
            "the option `activate` has no probability",
        );
        refused(
            r#""action":{"type":"choice","choice":"hold","probabilities":{"hold":0.8,"activate":0.1,"maybe":0.1},"confidence":0.9}"#,
            SCORE,
            NOUL,
            "`maybe` is not one of the options",
        );
        refused(
            r#""action":{"type":"choice","choice":"hold","probabilities":{"hold":0.9,"activate":0.1,"hold":0.1},"confidence":0.9}"#,
            SCORE,
            NOUL,
            "the key `hold` appears twice",
        );
        // Two spellings of one key collide once unescaped.
        refused(
            r#""action":{"type":"choice","choice":"hold","probabilities":{"hold":0.9,"activate":0.1,"hold":0.1},"confidence":0.9}"#,
            SCORE,
            NOUL,
            "the key `hold` appears twice",
        );
        refused(
            &choice("hold", "0", "0", "0"),
            SCORE,
            NOUL,
            "every probability is zero",
        );
        refused(
            &choice("hold", "1.2", "-0.2", "0.9"),
            SCORE,
            NOUL,
            "not between 0 and 1",
        );
        refused(
            &choice("hold", "0.9", "0.1", "1.5"),
            SCORE,
            NOUL,
            "`confidence` is 1.5",
        );
        refused(
            r#""action":{"type":"choice","choice":"hold","probabilities":{"hold":0.9,"activate":0.1}}"#,
            SCORE,
            NOUL,
            "`confidence` is missing",
        );
        refused(
            r#""action":{"type":"choice","probabilities":{"hold":0.9,"activate":0.1},"confidence":0.9}"#,
            SCORE,
            NOUL,
            "`choice` is missing",
        );
    }

    /// The sum tolerances at their edges (rig-typesafeai 0.43.0's rule):
    /// probabilities in whole hundredths may miss 1 by min(n × 0.005, 0.02),
    /// others by 1e-3. Accepted values are kept exactly as sent.
    #[test]
    fn probabilities_sum_to_one_within_the_documented_tolerance() {
        // n = 2, hundredths: 0.01 allowed.
        let kept = accepted(&choice("hold", "0.9", "0.09", "0.9"), SCORE, NOUL);
        assert!(
            matches!(&kept.answers["action"], Answer::Choice { probabilities, .. } if probabilities["activate"] == 0.09 && probabilities["hold"] == 0.9)
        );
        refused(&choice("hold", "0.9", "0.08", "0.9"), SCORE, NOUL, "sum to");
        accepted(&choice("hold", "0.9", "0.11", "0.9"), SCORE, NOUL);
        refused(&choice("hold", "0.9", "0.12", "0.9"), SCORE, NOUL, "sum to");
        // Not hundredths: 1e-3 allowed.
        accepted(&choice("hold", "0.9", "0.0995", "0.9"), SCORE, NOUL);
        refused(
            &choice("hold", "0.9", "0.0985", "0.9"),
            SCORE,
            NOUL,
            "sum to",
        );
        // n = 3 levels in hundredths: 0.015 allowed; 0.98 is too far.
        accepted(
            CHOICE,
            r#""risk":{"type":"score","score":1.2,"legend":{"0":"low","1":"medium","2":"high"},"probabilities":{"0":0.1,"1":0.6,"2":0.29},"confidence":0.5}"#,
            NOUL,
        );
        refused(
            CHOICE,
            r#""risk":{"type":"score","score":1.2,"legend":{"0":"low","1":"medium","2":"high"},"probabilities":{"0":0.1,"1":0.6,"2":0.28},"confidence":0.5}"#,
            NOUL,
            "sum to",
        );
    }

    fn score(score: &str, legend: &str, probabilities: &str) -> String {
        format!(
            r#""risk":{{"type":"score","score":{score},"legend":{legend},"probabilities":{probabilities},"confidence":0.5}}"#
        )
    }

    const LEGEND: &str = r#"{"0":"low","1":"medium","2":"high"}"#;
    const LEVELS: &str = r#"{"0":0.1,"1":0.6,"2":0.3}"#;

    /// A3: level indices are exactly "0" to "n-1"; nothing is dropped,
    /// parsed loosely or repaired.
    #[test]
    fn score_levels_are_exact_indices() {
        for (probabilities, why) in [
            (r#"{"0":0.1,"1":0.6,"x":0.3}"#, "the key `x`"),
            (r#"{"0":0.1,"1":0.6,"-1":0.3}"#, "the key `-1`"),
            (r#"{"0":0.1,"01":0.6,"2":0.3}"#, "the key `01`"),
            (r#"{"00":0.1,"1":0.6,"2":0.3}"#, "the key `00`"),
            (r#"{"0":0.1,"+1":0.6,"2":0.3}"#, "the key `+1`"),
            (r#"{"1":0.7,"2":0.3}"#, "has no level 0"),
            (r#"{"0":0.1,"1":0.6,"2":0.3,"3":0.0}"#, "the key `3`"),
            (r#"{"0":0.1,"1":0.6,"2":0.3,"1":0.0}"#, "appears twice"),
        ] {
            refused(CHOICE, &score("1.2", LEGEND, probabilities), NOUL, why);
        }
        for (legend, why) in [
            (
                r#"{"0":"low","1":"high","2":"medium"}"#,
                "the legend names level 1 `high`",
            ),
            (r#"{"0":"low","1":"medium"}"#, "has no level 2"),
            (
                r#"{"0":"low","1":"medium","2":"high","3":"extreme"}"#,
                "the key `3`",
            ),
        ] {
            refused(CHOICE, &score("1.2", legend, LEVELS), NOUL, why);
        }
        refused(
            CHOICE,
            r#""risk":{"type":"score","score":1.2,"probabilities":{"0":0.1,"1":0.6,"2":0.3},"confidence":0.5}"#,
            NOUL,
            "`legend` is missing",
        );
    }

    /// The score lies in the scale and agrees with its distribution: within
    /// 0.005 plus, for probabilities in hundredths, Σ|i − mean| × 0.005.
    #[test]
    fn a_score_agrees_with_its_distribution() {
        // Mean 1.2, hundredths: 0.016 allowed.
        accepted(CHOICE, &score("1.21", LEGEND, LEVELS), NOUL);
        accepted(CHOICE, &score("1.216", LEGEND, LEVELS), NOUL);
        refused(
            CHOICE,
            &score("1.25", LEGEND, LEVELS),
            NOUL,
            "its probabilities put it at 1.2000",
        );
        // Mean 1.177, not hundredths: 0.005 allowed.
        let fine = r#"{"0":0.123,"1":0.577,"2":0.3}"#;
        accepted(CHOICE, &score("1.18", LEGEND, fine), NOUL);
        refused(
            CHOICE,
            &score("1.19", LEGEND, fine),
            NOUL,
            "put it at 1.1770",
        );
        // Inside the scale.
        refused(
            CHOICE,
            &score("2.1", LEGEND, LEVELS),
            NOUL,
            "not between 0 and 2",
        );
        refused(
            CHOICE,
            &score("-0.1", LEGEND, LEVELS),
            NOUL,
            "not between 0 and 2",
        );
        refused(
            CHOICE,
            &score("null", LEGEND, LEVELS),
            NOUL,
            "`score` is missing",
        );
        let all_zero = r#"{"0":0,"1":0,"2":0}"#;
        refused(
            CHOICE,
            &score("0", LEGEND, all_zero),
            NOUL,
            "every probability is zero",
        );
    }

    #[test]
    fn versions_are_told_from_aliases() {
        assert!(is_versioned("jev-1.13.0"));
        assert!(is_versioned("jev-10.0.12"));
        for alias in [
            "jev-latest",
            "jev-preview",
            "jev-1.13",
            "jev-1.13.0-rc1",
            "jev-1..0",
            "gpt-1.0.0",
        ] {
            assert!(!is_versioned(alias), "{alias}");
        }
    }

    #[test]
    fn an_excerpt_is_short_single_line_and_keyless() {
        assert_eq!(excerpt(b"", "k"), "no details");
        assert_eq!(excerpt(b"a\n\tb  c\x07", "secret"), "a b c");
        assert_eq!(excerpt(b"bad secret here", "secret"), "bad … here");
        let long = excerpt("é".repeat(400).as_bytes(), "k");
        assert_eq!(long.chars().count(), EXCERPT + 1);
        assert!(long.ends_with('…'));
    }
}
