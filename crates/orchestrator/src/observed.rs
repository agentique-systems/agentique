//! What exploration reads from a test instance's observations and from its
//! answers to actions (C-54), in one place: the control interface's fields
//! as the explorer assumes them, each with what it falls back to when a
//! Studio does not publish it yet. The control interface's own work item
//! (W12.2) adds `operatorOnly` marks, `project.digest`, the `conversation`
//! fields and a refusal `kind`; differences in their names are reconciled
//! here only.

use serde_json::Value;

/// The Conversation's composer, its Send and its Stop.
pub const COMPOSER: &str = "Message";
pub const SEND: &str = "send";
pub const STOP: &str = "stop";
/// A dialog's confirm and cancel.
pub const CONFIRM: &str = "dialog-confirm";
pub const CANCEL: &str = "dialog-cancel";

/// How an instance refused an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// It was carried out (or failed partway: not a refusal).
    None,
    /// The screen changed since the observation it rested on.
    Stale,
    /// Its control is gone or disabled, or its command unavailable.
    Gone,
    /// The Operator's own, by rule.
    Rule,
    /// Refused for another reason.
    Other,
}

/// How `answer` refused its action: by its `kind` when it says, otherwise by
/// the words its error starts with.
pub fn refusal(answer: &Value) -> Refusal {
    if answer["ok"] != false {
        return Refusal::None;
    }
    if let Some(kind) = answer["kind"].as_str() {
        return match kind {
            "stale" => Refusal::Stale,
            "gone" | "disabled" | "unavailable" => Refusal::Gone,
            "operator" | "operatorOnly" | "operator-only" | "rule" => Refusal::Rule,
            _ => Refusal::Other,
        };
    }
    let error = answer["error"].as_str().unwrap_or_default();
    if error.starts_with("stale: no control")
        || error.contains("is disabled now")
        || error.contains("is not available:")
    {
        Refusal::Gone
    } else if error.starts_with("stale") {
        Refusal::Stale
    } else if error.starts_with("refused") {
        Refusal::Rule
    } else {
        Refusal::Other
    }
}

/// Whether the observation marks what is the Operator's own (`operatorOnly`
/// on its controls or commands); then its marks decide.
pub fn marks(observation: &Value) -> bool {
    ["controls", "commands"].iter().any(|list| {
        observation[*list]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| c.get("operatorOnly").is_some())
    })
}

/// Whether a control or command is marked the Operator's own.
pub fn operator_only(item: &Value) -> bool {
    item["operatorOnly"] == true
}

/// The model's digest, where the observation publishes one.
pub fn digest(observation: &Value) -> Option<&Value> {
    observation["project"]
        .get("digest")
        .filter(|d| !d.is_null())
}

/// Whether a turn of the Assistant is running.
pub fn turn_running(observation: &Value) -> bool {
    let conversation = &observation["conversation"];
    conversation["running"] == true || conversation["turn"]["running"] == true
}

/// The Assistant's last reply.
pub fn last_reply(observation: &Value) -> &str {
    observation["conversation"]["lastReply"]
        .as_str()
        .unwrap_or_default()
}

/// The Conversation's notices (`conversation.notices`, as text or
/// `{ text }`, and `conversation.error`).
pub fn notices(observation: &Value) -> Vec<String> {
    let conversation = &observation["conversation"];
    conversation["notices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_str().or_else(|| n["text"].as_str()))
        .chain(conversation["error"].as_str())
        .map(str::to_string)
        .collect()
}

/// An environment condition the observation shows: the Assistant needs a
/// key (`conversation.keyMissing`, or a notice saying so). A condition of
/// the run, never a finding.
pub fn needs_key(observation: &Value) -> Option<String> {
    let missing = &observation["conversation"]["keyMissing"];
    if let Some(text) = missing.as_str() {
        return Some(format!("the Assistant needs a key: {text}"));
    }
    if !missing.is_null() {
        return Some("the Assistant needs a key".into());
    }
    notices(observation).into_iter().find(|n| {
        let n = n.to_lowercase();
        n.contains("key") && (n.contains("need") || n.contains("missing") || n.contains("no "))
    })
}

/// What the instance's Assistant has spent so far, where the observation
/// says (`conversation.usd`).
pub fn assistant_spend(observation: &Value) -> Option<f64> {
    observation["conversation"]["usd"].as_f64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_refusal_is_read_by_its_kind_and_otherwise_by_its_words() {
        let answer = |error: &str| json!({ "ok": false, "error": error });
        assert_eq!(refusal(&json!({ "ok": true })), Refusal::None);
        assert_eq!(
            refusal(&answer("stale: the screen changed since you observed it")),
            Refusal::Stale
        );
        assert_eq!(
            refusal(&answer("stale: no control `x` is on screen now")),
            Refusal::Gone
        );
        assert_eq!(refusal(&answer("`Sync` is disabled now")), Refusal::Gone);
        assert_eq!(
            refusal(&answer("refused: `undo` is the Operator's to use")),
            Refusal::Rule
        );
        assert_eq!(refusal(&answer("the project did not open")), Refusal::Other);
        // The kind decides over the words.
        let kinded = json!({ "ok": false, "kind": "operator", "error": "stale: whatever" });
        assert_eq!(refusal(&kinded), Refusal::Rule);
    }
}
