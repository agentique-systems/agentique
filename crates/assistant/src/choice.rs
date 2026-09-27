//! Which model the Assistant uses. Until Settings exists (W5.8) it comes
//! from the environment:
//!
//! - `AGENTIQUE_PROVIDER`: `anthropic`, `deepseek`, `openai` or `openrouter`.
//!   Without it, the first provider with a key, in that order (C-27 keeps
//!   Claude the default; C-35 lets the product run with only a DeepSeek key).
//! - `AGENTIQUE_MODEL`: the provider's model id; its default otherwise.
//! - `AGENTIQUE_EFFORT`: an effort level the model offers; its default
//!   otherwise.

use crate::claude::ClaudeModel;
use crate::model::Model;
use crate::provider_model::ProviderModel;
use agq_providers::{ModelRef, Provider, capabilities, key_status};

/// The order in which providers with a key are chosen.
const PREFERENCE: [Provider; 4] = [
    Provider::Anthropic,
    Provider::DeepSeek,
    Provider::OpenAi,
    Provider::OpenRouter,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelChoice {
    pub model: ModelRef,
    /// The effort sent, if the model offers efforts.
    pub effort: Option<String>,
}

impl ModelChoice {
    pub fn from_env() -> ModelChoice {
        let var = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        let provider = var("AGENTIQUE_PROVIDER")
            .and_then(|id| Provider::from_id(&id))
            .or_else(|| {
                PREFERENCE
                    .into_iter()
                    .find(|provider| key_status(*provider) != agq_providers::KeyStatus::Missing)
            })
            .unwrap_or(Provider::Anthropic);
        let model = ModelRef::new(
            provider,
            var("AGENTIQUE_MODEL").unwrap_or_else(|| provider.default_model().to_string()),
        );
        ModelChoice::new(model, var("AGENTIQUE_EFFORT"))
    }

    /// `effort` is kept only if the model offers it; otherwise the model's
    /// default applies.
    pub fn new(model: ModelRef, effort: Option<String>) -> ModelChoice {
        let offered = capabilities(&model);
        let effort = effort
            .filter(|effort| offered.efforts.contains(&effort.as_str()))
            .or(offered.default_effort.map(str::to_string));
        ModelChoice { model, effort }
    }

    /// Shown discreetly in the Conversation: `deepseek-flash · high`.
    pub fn label(&self) -> String {
        match &self.effort {
            Some(effort) => format!("{} · {effort}", self.model.model),
            None => self.model.model.clone(),
        }
    }

    pub fn has_key(&self) -> bool {
        key_status(self.model.provider) != agq_providers::KeyStatus::Missing
    }

    /// What the Operator reads when no key is set.
    pub fn missing_key_message(&self) -> String {
        format!(
            "No {} key is set. Set {} (or another provider's key, such as DEEPSEEK_API_KEY) and restart Agentique to work with the Assistant. Everything else works as usual.",
            self.model.provider.name(),
            self.model.provider.key_variable()
        )
    }

    /// A model for one turn.
    pub fn start(&self) -> Box<dyn Model + Send> {
        // The hand-written Claude client stays Anthropic's path until W5.7
        // moves it onto rig; it is the path tested against the Claude API's
        // stand-in (§7.6).
        if self.model.provider == Provider::Anthropic {
            let mut claude = ClaudeModel::from_env();
            claude.model = self.model.model.clone();
            if let Some(effort) = &self.effort {
                claude.effort = effort.clone();
            }
            return Box::new(claude);
        }
        Box::new(ProviderModel::new(self.model.clone(), self.effort.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_effort_the_model_lacks_falls_back_to_its_default() {
        let choice = ModelChoice::new(
            ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
            Some("medium".into()),
        );
        assert_eq!(choice.effort.as_deref(), Some("high"));
        assert_eq!(choice.label(), "deepseek-flash · high");
        let choice = ModelChoice::new(
            ModelRef::new(Provider::DeepSeek, "deepseek-flash"),
            Some("max".into()),
        );
        assert_eq!(choice.effort.as_deref(), Some("max"));
    }
}
