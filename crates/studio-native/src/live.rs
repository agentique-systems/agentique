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

/// The provider for a model id, by the ids the providers use.
pub fn provider_for(model: &str) -> Option<Provider> {
    let model = model.to_ascii_lowercase();
    Some(if model.starts_with("deepseek") {
        Provider::DeepSeek
    } else if model.starts_with("claude") {
        Provider::Anthropic
    } else if model.starts_with("gpt") || model.starts_with("o3") || model.starts_with("o4") {
        Provider::OpenAi
    } else if model.contains('/') {
        Provider::OpenRouter
    } else {
        return None;
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
            "{}\n\nYou are one component of a larger system. Answer every request with exactly one JSON object and nothing else, in this form:\n{}\nUse only the values listed for a field that lists values. confidence is a number between 0 and 1.",
            request.instructions,
            serde_json::to_string_pretty(&request.output).unwrap_or_default()
        );
        let chat = ChatRequest {
            model: self.model.clone(),
            effort: None,
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

/// The JSON object in a reply, with or without a code fence.
fn json_in(text: &str) -> Option<serde_json::Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_come_from_model_ids_and_json_from_replies() {
        assert_eq!(provider_for("deepseek-flash"), Some(Provider::DeepSeek));
        assert_eq!(provider_for("claude-haiku-4-5"), Some(Provider::Anthropic));
        assert_eq!(
            provider_for("anthropic/claude-opus-5"),
            Some(Provider::OpenRouter)
        );
        assert_eq!(provider_for("mystery"), None);
        assert_eq!(
            json_in("```json\n{\"decision\": \"allow\"}\n```"),
            Some(serde_json::json!({"decision": "allow"}))
        );
        assert_eq!(json_in("no"), None);
    }
}
