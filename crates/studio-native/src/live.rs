//! Live evaluation of agents through the provider layer (ROADMAP §4.11,
//! §4.14): the model client a live run is handed, only after the Operator
//! confirmed the provider, model, samples and cost. Simulation never reaches
//! a provider by itself; this is the one place a run's agent calls do.
//!
//! A replay or a live evaluation is prepared first ([`prepare`], C-52):
//! offline and the same for both, it names the one agent configuration the
//! run covers and its binding (provider, model, adapter, mapping, what is
//! asked and sent), which goes into every request and so into the
//! recording key.
use crate::studio::Studio;
use agq_language::ElementId;
use agq_providers::{
    AssistantPart, ChatRequest, Event, Message, ModelRef, Provider, Providers, StopReason, UserPart,
};
use agq_simulation::agents::{AgentRequest, CallLimits, Evidence, LiveAnswer, LiveModel, Tokens};
use agq_simulation::{AgentInfo, Binding, Outcome, RunBinding};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The Studio's mapping of an agent's contract to a chat call, and its
/// revision (C-52): the prompt and answer template, the JSON object read
/// from the reply, a fast agent's lowest effort. Change the revision when
/// any of them changes.
pub const CHAT_MAPPING: &str = "answer-template 1";
/// What the model is told after the agent's instructions.
const CHAT_PROMPT: &str = "You are one component of a larger system. The user message is the input, as JSON. Answer with exactly one JSON object and nothing else, with these fields:";
/// A chat answer's output limit.
const MAX_OUTPUT_TOKENS: u64 = 1_024;

/// A provider's model answering an agent's calls through chat.
pub struct ProviderLive {
    providers: Providers,
    model: ModelRef,
}

/// The provider that serves a model id, as the provider layer's own table
/// knows it (its default model, or a model it prices); never guessed from
/// the id (§8.7). TypeSafe's models make typed decisions, not chat.
pub fn provider_for(model: &str) -> Option<Provider> {
    Provider::ALL
        .into_iter()
        .filter(|p| *p != Provider::TypeSafe)
        .find(|p| {
            p.default_model() == model || agq_providers::price(&ModelRef::new(*p, model)).is_some()
        })
}

/// How a replay or a live evaluation of a scenario is made (C-52).
#[derive(Clone, Debug, PartialEq)]
pub struct Prepared {
    /// The model that answers.
    pub model: ModelRef,
    /// The agent configuration the run covers, and its binding.
    pub run: RunBinding,
    /// The agent's instance, for messages.
    pub path: String,
}

/// Prepares the replay or live evaluation of `scenario`: `Ok(None)` when it
/// asks no agent; `Err` says why it cannot be prepared. Offline and pure:
/// no key, network or provider client.
pub fn prepare(studio: &Studio, scenario: ElementId) -> Result<Option<Prepared>, String> {
    let project = studio.project.as_ref().ok_or("No project is open.")?;
    let tree = project.state().tree();
    let program = agq_simulation::compile(tree, scenario).map_err(|blockers| {
        blockers
            .first()
            .map_or("The scenario cannot run.".to_string(), |b| {
                b.message.clone()
            })
    })?;
    let agents = agq_simulation::describe_agents(&program).map_err(|stop| stop.message)?;
    let mut configurations: Vec<&AgentInfo> = Vec::new();
    for agent in &agents {
        if !configurations
            .iter()
            .any(|c| c.request.same_agent(&agent.request))
        {
            configurations.push(agent);
        }
    }
    let agent = match configurations.as_slice() {
        [] => return Ok(None),
        [one] => *one,
        many => {
            return Err(format!(
                "A replay or a live evaluation is prepared for one agent configuration, and this scenario asks {}: {}.",
                many.len(),
                many.iter()
                    .map(|a| format!(
                        "`{}` (model {})",
                        a.path,
                        a.request.model.as_deref().unwrap_or("not set")
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    };
    let model = match agent
        .request
        .model
        .as_deref()
        .and_then(|m| provider_for(m).map(|p| ModelRef::new(p, m)))
    {
        Some(model) => model,
        None => studio.settings.model_choice().model.clone(),
    };
    let binding = chat_binding(&model, &agent.request);
    Ok(Some(Prepared {
        model,
        run: RunBinding {
            agent: agent.request.clone(),
            binding,
        },
        path: agent.path.clone(),
    }))
}

/// The reasoning effort for an agent's mode: a fast agent asks for the
/// least reasoning the model offers (its capability table, lowest first); a
/// deliberate one uses the model's own.
fn effort(model: &ModelRef, mode: Option<&str>) -> Option<String> {
    match mode {
        Some("fast") => agq_providers::capabilities(model)
            .efforts
            .first()
            .map(|e| e.to_string()),
        _ => None,
    }
}

/// The binding of a chat call: the provider and model, the adapter, the
/// prompt and answer template, what is sent and the effort and output
/// limit.
fn chat_binding(model: &ModelRef, request: &AgentRequest) -> Binding {
    Binding {
        provider: model.provider.id().to_string(),
        model: model.model.clone(),
        adapter: agq_providers::CHAT_ADAPTER.to_string(),
        mapping: CHAT_MAPPING.to_string(),
        question: serde_json::json!({
            "prompt": CHAT_PROMPT,
            "template": answer_template(&request.output),
        }),
        input: serde_json::json!({ "sent": "the input item, as JSON" }),
        policy: serde_json::json!({
            "effort": effort(model, request.mode.as_deref()),
            "maxOutputTokens": MAX_OUTPUT_TOKENS,
        }),
    }
}

/// The live model for a prepared evaluation, when its provider has a key.
pub fn live_model(prepared: &Prepared) -> Option<Arc<dyn LiveModel>> {
    let providers = Providers::new();
    if !providers.has_key(prepared.model.provider) {
        return None;
    }
    Some(Arc::new(ProviderLive {
        providers,
        model: prepared.model.clone(),
    }))
}

/// The provider and model a live evaluation of the selected scenario would
/// use, when it can be prepared.
pub fn live_choice(studio: &Studio) -> Option<(Provider, String)> {
    let scenario = studio.runs.selected?;
    let prepared = prepare(studio, scenario).ok()??;
    Some((prepared.model.provider, prepared.model.model))
}

impl LiveModel for ProviderLive {
    fn label(&self) -> String {
        format!("{}/{}", self.model.provider.id(), self.model.model)
    }

    fn answer(
        &self,
        request: &AgentRequest,
        limits: CallLimits,
        cancel: &AtomicBool,
    ) -> LiveAnswer {
        let system = format!(
            "{}\n\n{CHAT_PROMPT}\n{}",
            request.instructions,
            answer_template(&request.output)
        );
        let chat = ChatRequest {
            model: self.model.clone(),
            effort: effort(&self.model, request.mode.as_deref()),
            max_output_tokens: MAX_OUTPUT_TOKENS,
            system,
            tools: Vec::new(),
            messages: vec![Message::User(vec![UserPart::Text {
                text: serde_json::to_string(&request.input).unwrap_or_default(),
            }])],
        };
        let started = Instant::now();
        let mut handle = self.providers.chat(chat);
        // Once sent, a call that does not finish has an unknown cost.
        let unfinished = |error: Option<String>| LiveAnswer {
            outcome: Outcome::Timeout,
            output: None,
            latency_ms: started.elapsed().as_millis() as u64,
            cost_usd: None,
            error,
            evidence: Some(Evidence {
                attempts: 1,
                ..Evidence::default()
            }),
        };
        loop {
            if cancel.load(Ordering::SeqCst) {
                handle.cancel();
                return unfinished(Some("cancelled".into()));
            }
            // The agent's deadline: the call is stopped, and the engine
            // takes its modelled timeout.
            let left = limits.deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                handle.cancel();
                return unfinished(None);
            }
            let Some(event) = handle.next_event(left.min(Duration::from_millis(50))) else {
                continue;
            };
            let Event::Finished(result) = event else {
                continue;
            };
            let latency_ms = started.elapsed().as_millis() as u64;
            return match result {
                Err(error) => LiveAnswer {
                    latency_ms,
                    ..unfinished(Some(error.to_string()))
                },
                Ok(reply) => {
                    let cost_usd = reply.usage.cost_usd(&self.model);
                    let evidence = Some(Evidence {
                        usage: Some(Tokens {
                            input: Some(
                                reply.usage.input_tokens
                                    + reply.usage.cache_read_tokens
                                    + reply.usage.cache_write_tokens,
                            ),
                            output: Some(reply.usage.output_tokens),
                        }),
                        attempts: 1,
                        ..Evidence::default()
                    });
                    if reply.stop == StopReason::Refusal {
                        return LiveAnswer {
                            outcome: Outcome::Refusal,
                            output: None,
                            latency_ms,
                            cost_usd,
                            error: None,
                            evidence,
                        };
                    }
                    let text: String = reply
                        .content
                        .iter()
                        .filter_map(|part| match part {
                            AssistantPart::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect();
                    let (outcome, output) = match json_in(&text) {
                        Some(output) => (Outcome::Answer, Some(output)),
                        None => (Outcome::InvalidOutput, None),
                    };
                    LiveAnswer {
                        outcome,
                        output,
                        latency_ms,
                        cost_usd,
                        error: None,
                        evidence,
                    }
                }
            };
        }
    }
}

/// The answer an agent gives, as a template the model fills in: one line
/// per field with what it may hold, from the output item's shape (never a
/// schema to copy).
fn answer_template(shape: &serde_json::Value) -> String {
    fn field_line(field: &serde_json::Value, indent: &str) -> String {
        let name = field["name"].as_str().unwrap_or("?");
        let what = if let Some(values) = field["values"].as_array() {
            let values: Vec<&str> = values.iter().filter_map(|v| v.as_str()).collect();
            format!("one of {}", values.join(", "))
        } else if let Some(fields) = field["fields"].as_array() {
            let inner: Vec<String> = fields
                .iter()
                .map(|f| field_line(f, &format!("{indent}  ")))
                .collect();
            format!("an object with:\n{}", inner.join("\n"))
        } else {
            match field["type"].as_str().unwrap_or("") {
                "String" => "text".to_string(),
                "Boolean" => "true or false".to_string(),
                "Real" | "Rational" | "Number" | "Complex" => "a number".to_string(),
                "Integer" | "Natural" | "Positive" => "a whole number".to_string(),
                other => other.to_string(),
            }
        };
        let optional = if field["required"].as_bool() == Some(false) {
            " (may be left out)"
        } else {
            ""
        };
        format!("{indent}- \"{name}\": {what}{optional}")
    }
    let lines: Vec<String> = shape["fields"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| field_line(f, ""))
        .collect();
    let mut text = lines.join("\n");
    text.push_str("\n\"confidence\" is your own confidence in the answer, between 0 and 1.");
    text
}

/// The JSON object in a reply, with or without a code fence.
fn json_in(text: &str) -> Option<serde_json::Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// I3 for real: the screening agent's evaluation cases on DeepSeek, five
    /// samples each, kept as recordings and replayed. It spends money, so it
    /// runs only when asked: `AGQ_LIVE=1 cargo test -p agq-studio-native
    /// live_evaluation -- --ignored --nocapture`, with `DEEPSEEK_API_KEY` set.
    #[test]
    #[ignore = "calls a real model"]
    fn live_evaluation_of_the_screening_agent() {
        if std::env::var("AGQ_LIVE").as_deref() != Ok("1") {
            eprintln!("Set AGQ_LIVE=1 to run the live evaluation.");
            return;
        }
        // AGQ_LIVE_MAX_LATENCY_MS evaluates a changed latency limit (I7).
        let text = match std::env::var("AGQ_LIVE_MAX_LATENCY_MS") {
            Ok(ms) => crate::studio::SCREENING_SAMPLE.replace(
                ":>> maxLatencyMs = 500;",
                &format!(":>> maxLatencyMs = {ms};"),
            ),
            Err(_) => crate::studio::SCREENING_SAMPLE.to_string(),
        };
        let tree = agq_language::parse(&[agq_language::Source::new("UrlShortener.sysml", &text)]);
        let cases = tree.find("UrlShortener::ScreeningCases").unwrap();
        let program = agq_simulation::compile(&tree, cases).unwrap();
        let digest = agq_simulation::digest::model_digest(&tree, cases);
        let model = ModelRef::new(Provider::DeepSeek, "deepseek-flash");
        let agent = agq_simulation::describe_agents(&program).unwrap()[0]
            .request
            .clone();
        let binding = RunBinding {
            binding: chat_binding(&model, &agent),
            agent,
        };
        let live: Arc<dyn LiveModel> = Arc::new(ProviderLive {
            providers: Providers::new(),
            model,
        });
        let mut request = agq_simulation::Request::new(agq_simulation::Mode::Live);
        request.samples = 5;
        request.binding = Some(binding.clone());
        let result = agq_simulation::run(
            &program,
            digest.clone(),
            &request,
            agq_simulation::Answers::Live(live),
            Arc::new(AtomicBool::new(false)),
        );
        println!("LIVE\n{}", result.describe(None, 0));
        for event in result.trace.iter().filter(|e| {
            matches!(
                e.kind,
                agq_simulation::EventKind::AgentFailed | agq_simulation::EventKind::AgentAnswered
            )
        }) {
            println!("  {}", event.text);
        }
        let answers = result
            .live
            .as_ref()
            .map(|l| l.answers.clone())
            .unwrap_or_default();
        let folder = std::env::temp_dir().join(format!("agq-live-{}", std::process::id()));
        let kept = agq_simulation::Recordings::keep(&folder, &answers).unwrap();
        println!("KEPT {kept} recording(s)");
        let mut again = agq_simulation::Request::new(agq_simulation::Mode::Replay);
        again.binding = Some(binding);
        let replay = agq_simulation::run(
            &program,
            digest,
            &again,
            agq_simulation::Answers::Recordings(Arc::new(agq_simulation::Recordings::read(
                &folder,
            ))),
            Arc::new(AtomicBool::new(false)),
        );
        println!("REPLAY\n{}", replay.describe(None, 0));
        let _ = std::fs::remove_dir_all(&folder);
        assert!(result.live.is_some());
    }

    #[test]
    fn providers_come_from_model_ids_and_json_from_replies() {
        assert_eq!(provider_for("deepseek-flash"), Some(Provider::DeepSeek));
        assert_eq!(provider_for("claude-haiku-4-5"), Some(Provider::Anthropic));
        assert_eq!(provider_for("mystery"), None);
        assert_eq!(
            json_in("```json\n{\"decision\": \"allow\"}\n```"),
            Some(serde_json::json!({"decision": "allow"}))
        );
        assert_eq!(json_in("no"), None);
    }

    #[test]
    fn the_answer_template_lists_fields_not_a_schema() {
        let shape = serde_json::json!({
            "type": "Verdict",
            "fields": [
                { "name": "decision", "required": true, "type": "Decision", "values": ["allow", "review", "block"] },
                { "name": "reason", "required": false, "type": "String" },
                { "name": "confidence", "required": false, "type": "Real" }
            ]
        });
        let text = answer_template(&shape);
        assert!(
            text.contains("- \"decision\": one of allow, review, block"),
            "{text}"
        );
        assert!(
            text.contains("- \"reason\": text (may be left out)"),
            "{text}"
        );
        assert!(text.contains("- \"confidence\": a number"), "{text}");
        assert!(!text.contains("\"fields\""), "{text}");
    }
}
