//! Where an agent's answers come from in a run (ROADMAP §4.11, §4.14): the
//! scenario's stand-ins, recordings matched by request digest, or a live
//! model client the Studio hands to an explicit live evaluation. The agent's
//! contract is checked the same way whatever the source.
//!
//! A request is canonical JSON: the agent, its mode and model, its
//! instructions, the input item and the shape of the output it must give.
//! Its SHA-256 is the recording key, so a replay matches exactly the request
//! the model would have been sent, and nothing else.

use crate::compile::{ItemType, Outcome, Types};
use crate::digest::text_digest;
use crate::value::{Field, Item, Value};
use agq_language::ElementId;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// What an agent's model is asked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequest {
    /// The agent's qualified name (its part def).
    pub agent: String,
    pub mode: Option<String>,
    /// The provider's model id, as the agent's `model` attribute gives it.
    pub model: Option<String>,
    pub instructions: String,
    pub input: Json,
    /// The output item: its type and fields, with each field's type and,
    /// for enums, its values.
    pub output: Json,
}

impl AgentRequest {
    /// The canonical text of the request: sorted keys, no spaces.
    pub fn canonical(&self) -> String {
        serde_json::to_string(&serde_json::to_value(self).expect("a request is JSON"))
            .expect("JSON prints")
    }

    /// The recording key.
    pub fn digest(&self) -> String {
        text_digest(&self.canonical())
    }
}

/// The shape of an item type, as a request describes the output.
pub fn shape(types: &Types, ty: ElementId) -> Json {
    let Some(item) = types.items.get(&ty) else {
        return json!({ "type": types.name(ty) });
    };
    shape_of(types, item)
}

fn shape_of(types: &Types, item: &ItemType) -> Json {
    let fields: Vec<Json> = item
        .fields
        .iter()
        .map(|field| {
            let mut f = json!({ "name": field.slot.name, "required": field.required });
            if let Some(ty) = field.ty {
                f["type"] = json!(types.name(ty));
                if let Some(values) = types.enums.get(&ty) {
                    f["values"] = json!(values.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>());
                }
                if let Some(inner) = types.items.get(&ty) {
                    f["fields"] = shape_of(types, inner)["fields"].clone();
                }
            }
            f
        })
        .collect();
    json!({ "type": item.name, "fields": fields })
}

/// An answer, from whatever source.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentAnswer {
    pub outcome: Outcome,
    /// The output as the model gave it: a value from a stand-in, or JSON
    /// from a recording or a live model (checked when it is read).
    pub output: Option<AnswerOutput>,
    pub latency_ms: u64,
    /// Where it came from, for the trace: `stand-in failFirst`,
    /// `recording 3fa2…`, `live deepseek/deepseek-flash`.
    pub source: String,
    pub cost_usd: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnswerOutput {
    Value(Value),
    Json(Json),
}

/// A live model client for live evaluations. The Studio implements it with
/// the provider layer; Simulation never reaches a provider by itself.
pub trait LiveModel: Send + Sync {
    /// `provider/model`, for provenance.
    fn label(&self) -> String;
    /// Asks the model once. `cancel` is set when the run is cancelled.
    fn answer(&self, request: &AgentRequest, cancel: &AtomicBool) -> LiveAnswer;
}

/// What a live model answered.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveAnswer {
    pub outcome: Outcome,
    pub output: Option<Json>,
    pub latency_ms: u64,
    pub cost_usd: Option<f64>,
    /// A provider error (not the model's answer), such as a refused key or
    /// no network: a failure of the evaluation, not of the agent.
    pub error: Option<String>,
}

/// One kept answer: a line of `recordings/<agent>.jsonl`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub digest: String,
    pub request: AgentRequest,
    pub outcome: Outcome,
    #[serde(default)]
    pub output: Option<Json>,
    pub latency_ms: u64,
    /// `provider/model` that answered.
    pub answered_by: String,
    pub recorded_at: String,
    /// The live evaluation it was kept from.
    #[serde(default)]
    pub run: Option<String>,
}

/// The recordings of a project: `<project>/recordings/*.jsonl`, keyed by
/// request digest. Reading never falls back to anything else.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recordings {
    by_digest: HashMap<String, Recording>,
    /// A digest of the files read, for provenance.
    pub digest: String,
    /// Lines that could not be read, with why.
    pub problems: Vec<String>,
}

impl Recordings {
    /// Reads every `.jsonl` file in `folder` (none is fine).
    pub fn read(folder: &Path) -> Recordings {
        let mut recordings = Recordings::default();
        let mut all = String::new();
        let mut files: Vec<PathBuf> = std::fs::read_dir(folder)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for file in files {
            let Ok(text) = std::fs::read_to_string(&file) else {
                recordings
                    .problems
                    .push(format!("{} could not be read", file.display()));
                continue;
            };
            all.push_str(&text);
            for (n, line) in text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Recording>(line) {
                    Ok(recording) => {
                        recordings
                            .by_digest
                            .insert(recording.digest.clone(), recording);
                    }
                    Err(error) => recordings.problems.push(format!(
                        "{} line {}: {error}",
                        file.display(),
                        n + 1
                    )),
                }
            }
        }
        recordings.digest = text_digest(&all);
        recordings
    }

    pub fn get(&self, digest: &str) -> Option<&Recording> {
        self.by_digest.get(digest)
    }

    pub fn len(&self) -> usize {
        self.by_digest.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_digest.is_empty()
    }

    /// Adds recordings to the agent's file in `folder`, one per line,
    /// replacing none: a request already recorded keeps its first answer.
    pub fn keep(folder: &Path, recordings: &[Recording]) -> std::io::Result<usize> {
        std::fs::create_dir_all(folder)?;
        let existing = Recordings::read(folder);
        let mut by_file: HashMap<String, String> = HashMap::new();
        let mut kept = 0;
        for recording in recordings {
            if existing.get(&recording.digest).is_some() {
                continue;
            }
            let file = file_name(&recording.request.agent);
            let line = serde_json::to_string(recording).expect("a recording is JSON");
            let text = by_file.entry(file).or_default();
            text.push_str(&line);
            text.push('\n');
            kept += 1;
        }
        for (file, text) in by_file {
            use std::io::Write;
            let mut out = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(folder.join(file))?;
            out.write_all(text.as_bytes())?;
            out.sync_all()?;
        }
        Ok(kept)
    }
}

/// `UrlShortener::LinkScreening` → `UrlShortener.LinkScreening.jsonl`.
fn file_name(agent: &str) -> String {
    let safe: String = agent
        .replace("::", ".")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}.jsonl")
}

/// Reads a value of type `ty` from JSON: items as `{"type": ..., "fields":
/// {...}}` or as a plain object of fields, enum values by name.
pub fn from_json(types: &Types, ty: ElementId, json: &Json) -> Result<Value, String> {
    if json.is_null() {
        return Ok(Value::Null);
    }
    if let Some(values) = types.enums.get(&ty) {
        let name = json
            .as_str()
            .ok_or_else(|| format!("a `{}` is written as its name", types.name(ty)))?;
        let (value, name) = values
            .iter()
            .find(|(_, n)| n == name)
            .ok_or_else(|| format!("`{name}` is not a value of `{}`", types.name(ty)))?;
        return Ok(Value::Enum {
            def: ty,
            value: *value,
            name: name.clone(),
        });
    }
    if let Some(item) = types.items.get(&ty) {
        let object = json
            .as_object()
            .ok_or_else(|| format!("a `{}` is written as an object", item.name))?;
        if let Some(name) = object.get("type").and_then(Json::as_str)
            && name != item.name
        {
            return Err(format!("expected a `{}`, got a `{name}`", item.name));
        }
        let fields_json = object
            .get("fields")
            .and_then(Json::as_object)
            .unwrap_or(object);
        let mut fields = Vec::new();
        for field in &item.fields {
            let value = match (fields_json.get(&field.slot.name), field.ty) {
                (Some(json), Some(ty)) => from_json(types, ty, json)
                    .map_err(|why| format!("`{}.{}`: {why}", item.name, field.slot.name))?,
                (Some(json), None) => scalar(json),
                (None, _) => field.slot.default.clone().unwrap_or(Value::Null),
            };
            fields.push(Field {
                feature: field.slot.feature,
                aliases: field.slot.aliases.clone(),
                name: field.slot.name.clone(),
                value,
            });
        }
        for key in fields_json.keys() {
            if key != "type" && !item.fields.iter().any(|f| &f.slot.name == key) {
                return Err(format!("`{}` has no field `{key}`", item.name));
            }
        }
        return Ok(Value::Item(Item {
            ty,
            type_name: item.name.clone(),
            fields,
        }));
    }
    let value = scalar(json);
    types.fits(&value, ty)?;
    // A whole number where a Real is expected reads as a Real.
    match (&value, types.scalar.real) {
        (Value::Int(n), Some(real))
            if types.specializes(ty, real)
                && !types
                    .scalar
                    .integer
                    .is_some_and(|i| types.specializes(ty, i)) =>
        {
            Ok(Value::Real(*n as f64))
        }
        _ => Ok(value),
    }
}

fn scalar(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Real(n.as_f64().unwrap_or(f64::NAN)),
        },
        Json::String(s) => Value::Str(s.clone()),
        other => Value::Str(other.to_string()),
    }
}
