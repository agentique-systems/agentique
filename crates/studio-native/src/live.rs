//! Live evaluation of agents through the provider layer (ROADMAP §4.11,
//! §4.14): the model client a live run is handed, only after the Operator
//! confirmed the provider, model, samples and cost. Simulation never reaches
//! a provider by itself; this is the one place a run's agent calls do.
use crate::studio::Studio;
use agq_providers::{
    AssistantPart, ChatRequest, Event, Message, ModelRef, Provider, Providers, StopReason, UserPart,
};
use agq_simulation::Outcome;
use agq_simulation::agents::{AgentRequest, LiveAnswer, LiveModel};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A provider's model answering an agent's calls.
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

/// The live model for the selected scenario's agent: the agent's `model`
/// attribute when its provider has a key, else the Assistant's model.
pub fn live_model(studio: &Studio) -> Option<Arc<dyn LiveModel>> {
    let (provider, model) = live_choice(studio)?;
    let providers = Providers::new();
    if !providers.has_key(provider) {
        return None;
    }
    Some(Arc::new(ProviderLive {
        providers,
        model: ModelRef::new(provider, model),
    }))
}

/// The provider and model a live evaluation of the selected scenario would use.
pub fn live_choice(studio: &Studio) -> Option<(Provider, String)> {
    let scenario = studio.runs.selected?;
    let project = studio.project.as_ref()?;
    let tree = project.state().tree();
    // The first agent in the scenario's model slice with a model attribute.
    let semantics = agq_language::Semantics::new(tree);
    let agent = semantics.resolve("Agents::Agent");
    let model_feature = agent.and_then(|a| {
        semantics
            .features(a)
            .into_iter()
            .find(|f| semantics.name(*f) == Some("model"))
    });
    let mut chosen = None;
    for id in agq_simulation::digest::closure(tree, scenario) {
        let is_agent = agent.is_some_and(|a| {
            tree[id].kind == agq_language::ElementKind::PartDef && semantics.specializes(id, a)
        });
        if !is_agent {
            continue;
        }
        let model = semantics.features(id).into_iter().find_map(|f| {
            let redefines =
                model_feature.is_some_and(|m| semantics.redefined(f).contains(&m) || f == m);
            match (&tree.get(f)?.value, redefines) {
                (Some(agq_language::Literal::String(model)), true) => Some(model.clone()),
                _ => None,
            }
        });
        if let Some(model) = model {
            chosen = Some(model);
            break;
        }
    }
    match chosen.and_then(|m| provider_for(&m).map(|p| (p, m))) {
        Some(found) => Some(found),
        None => {
            let choice = studio.settings.model_choice();
            Some((choice.model.provider, choice.model.model.clone()))
        }
    }
}

impl LiveModel for ProviderLive {
    fn label(&self) -> String {
        format!("{}/{}", self.model.provider.id(), self.model.model)
    }

    fn answer(&self, request: &AgentRequest, cancel: &AtomicBool) -> LiveAnswer {
        let system = format!(
            "{}\n\nYou are one component of a larger system. The user message is the input, as JSON. Answer with exactly one JSON object and nothing else, with these fields:\n{}",
            request.instructions,
            answer_template(&request.output)
        );
        // The agent's mode, as the provider's reasoning effort: a fast agent
        // asks for the least reasoning the model offers (its capability
        // table, lowest first); a deliberate one uses the model's own.
        let effort = match request.mode.as_deref() {
            Some("fast") => agq_providers::capabilities(&self.model)
                .efforts
                .first()
                .map(|e| e.to_string()),
            _ => None,
        };
        let chat = ChatRequest {
            model: self.model.clone(),
            effort,
            max_output_tokens: 1_024,
            system,
            tools: Vec::new(),
            messages: vec![Message::User(vec![UserPart::Text {
                text: serde_json::to_string(&request.input).unwrap_or_default(),
            }])],
        };
        let started = Instant::now();
        let mut handle = self.providers.chat(chat);
        loop {
            if cancel.load(Ordering::SeqCst) {
                handle.cancel();
                return LiveAnswer {
                    outcome: Outcome::Timeout,
                    output: None,
                    latency_ms: started.elapsed().as_millis() as u64,
                    cost_usd: None,
                    error: Some("cancelled".into()),
                };
            }
            let Some(event) = handle.next_event(Duration::from_millis(50)) else {
                continue;
            };
            let Event::Finished(result) = event else {
                continue;
            };
            let latency_ms = started.elapsed().as_millis() as u64;
            return match result {
                Err(error) => LiveAnswer {
                    outcome: Outcome::Timeout,
                    output: None,
                    latency_ms,
                    cost_usd: None,
                    error: Some(error.to_string()),
                },
                Ok(reply) => {
                    let cost_usd = reply.usage.cost_usd(&self.model);
                    if reply.stop == StopReason::Refusal {
                        return LiveAnswer {
                            outcome: Outcome::Refusal,
                            output: None,
                            latency_ms,
                            cost_usd,
                            error: None,
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
                    match json_in(&text) {
                        Some(output) => LiveAnswer {
                            outcome: Outcome::Answer,
                            output: Some(output),
                            latency_ms,
                            cost_usd,
                            error: None,
                        },
                        None => LiveAnswer {
                            outcome: Outcome::InvalidOutput,
                            output: None,
                            latency_ms,
                            cost_usd,
                            error: None,
                        },
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
        let live: Arc<dyn LiveModel> = Arc::new(ProviderLive {
            providers: Providers::new(),
            model: ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
        });
        let mut request = agq_simulation::Request::new(agq_simulation::Mode::Live);
        request.samples = 5;
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
        let replay = agq_simulation::run(
            &program,
            digest,
            &agq_simulation::Request::new(agq_simulation::Mode::Replay),
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
