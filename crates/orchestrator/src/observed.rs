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

/// How an instance refused an action, or how it failed: the control
/// interface's refusal `kind` (W12.2), one name each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// It was carried out (or failed partway without saying how).
    None,
    /// `stale`: the screen changed since the observation it rested on.
    Stale,
    /// `gone`, `disabled`, `unavailable`: its control is gone or disabled,
    /// or its command unavailable.
    Gone,
    /// `operator-own`: the Operator's own, by rule.
    Rule,
    /// `held`: another agent holds the window.
    Held,
    /// `stopped`: the Operator pressed Stop in that window.
    Stopped,
    /// `expired`, `timeout`: it was not carried out in time.
    Timeout,
    /// `invalid`: the action itself was malformed (the explorer's fault).
    Invalid,
    /// `failed`: the Studio's handler failed.
    Failed,
    /// Refused for a reason no kind names.
    Other,
}

/// How `answer` refused its action: by its `kind`, or, when it has none or
/// one not known here, by the words its error starts with.
pub fn refusal(answer: &Value) -> Refusal {
    if answer["ok"] != false {
        return Refusal::None;
    }
    let by_kind = match answer["kind"].as_str() {
        Some("operator-own") => Some(Refusal::Rule),
        Some("stale") => Some(Refusal::Stale),
        Some("gone" | "disabled" | "unavailable") => Some(Refusal::Gone),
        Some("held") => Some(Refusal::Held),
        Some("stopped") => Some(Refusal::Stopped),
        Some("expired" | "timeout") => Some(Refusal::Timeout),
        Some("invalid") => Some(Refusal::Invalid),
        Some("failed") => Some(Refusal::Failed),
        _ => None,
    };
    if let Some(refusal) = by_kind {
        return refusal;
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
        let kind = |kind: &str| refusal(&json!({ "ok": false, "kind": kind, "error": "x" }));
        for (name, read) in [
            ("operator-own", Refusal::Rule),
            ("stale", Refusal::Stale),
            ("gone", Refusal::Gone),
            ("disabled", Refusal::Gone),
            ("unavailable", Refusal::Gone),
            ("held", Refusal::Held),
            ("stopped", Refusal::Stopped),
            ("expired", Refusal::Timeout),
            ("timeout", Refusal::Timeout),
            ("invalid", Refusal::Invalid),
            ("failed", Refusal::Failed),
        ] {
            assert_eq!(kind(name), read, "{name}");
        }
        // No synonyms: an unknown kind falls back to the words.
        assert_eq!(kind("operator"), Refusal::Other);
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
    }
}
