//! Which model the Assistant uses: Settings' choice (W5.8), with these
//! environment variables winning over it:
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

/// Whether the Assistant can use `provider`'s key. The hand-written Claude
/// client reads only `ANTHROPIC_API_KEY`, so until W5.7 moves Anthropic onto
/// the provider layer a key stored in the Credential Manager does not count
/// for Anthropic (a temporary exception to §8.7 rule 2).
fn usable_key(provider: Provider) -> bool {
    match key_status(provider) {
        agq_providers::KeyStatus::FromEnvironment { .. } => true,
        agq_providers::KeyStatus::Stored => provider != Provider::Anthropic,
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelChoice {
    pub model: ModelRef,
    /// The effort sent, if the model offers efforts.
    pub effort: Option<String>,
    /// The provider was named (AGENTIQUE_PROVIDER), not picked by its key.
    pub named: bool,
    /// Something about the configuration the Operator should fix.
    pub problem: Option<String>,
}

impl ModelChoice {
    pub fn from_env() -> ModelChoice {
        ModelChoice::configured(None, None, None)
    }

    /// The model from Settings (provider, model, effort; `None` or empty for
    /// "automatic"), with the environment variables winning over each, as a
    /// key's variable wins over a stored key (R-25):
    /// `AGENTIQUE_PROVIDER`, `AGENTIQUE_MODEL`, `AGENTIQUE_EFFORT`.
    pub fn configured(
        provider: Option<&str>,
        model: Option<&str>,
        effort: Option<&str>,
    ) -> ModelChoice {
        let var = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        };
        ModelChoice::resolve(&var, &usable_key, provider, model, effort)
    }

    /// [`configured`](Self::configured) with the environment and the keys
    /// given, for tests.
    fn resolve(
        var: &dyn Fn(&str) -> Option<String>,
        has_key: &dyn Fn(Provider) -> bool,
        provider: Option<&str>,
        model: Option<&str>,
        effort: Option<&str>,
    ) -> ModelChoice {
        let setting = |value: Option<&str>| {
            value
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        let from_environment = var("AGENTIQUE_PROVIDER");
        let from_settings = setting(provider);
        let chosen = from_environment.clone().or_else(|| from_settings.clone());
        // A provider whose models cannot use tools (TypeSafe AI's Jev) is
        // never the Assistant's (§4.8).
        let named = chosen
            .as_deref()
            .and_then(Provider::from_id)
            .filter(|provider| {
                capabilities(&ModelRef::new(*provider, provider.default_model())).tools
            });
        let provider = named
            .or_else(|| PREFERENCE.into_iter().find(|provider| has_key(*provider)))
            .unwrap_or(Provider::Anthropic);
        // Settings' model and effort belong to Settings' provider: they
        // apply only while it is the one chosen.
        let settings_apply = named.is_some()
            && from_settings.is_some()
            && from_environment.is_none_or(|id| Some(id) == from_settings);
        // AGENTIQUE_MODEL names a model of the chosen provider: it applies
        // when the provider is named too, or to Anthropic as in Stage 3, so a
        // Claude model id left in the environment never goes to DeepSeek.
        let model = var("AGENTIQUE_MODEL")
            .filter(|_| named.is_some() || provider == Provider::Anthropic)
            .or_else(|| setting(model).filter(|_| settings_apply))
            .unwrap_or_else(|| provider.default_model().to_string());
        let effort = var("AGENTIQUE_EFFORT").or_else(|| setting(effort).filter(|_| settings_apply));
        let mut choice = ModelChoice::new(ModelRef::new(provider, model), effort);
        choice.named = chosen.is_some();
        if let Some(id) = chosen.filter(|_| named.is_none()) {
            choice.problem = Some(format!(
                "The provider is set to `{id}`, which is not a provider the Assistant can use (anthropic, deepseek, openai, openrouter)."
            ));
        }
        choice
    }

    /// `effort` is kept only if the model offers it; otherwise the model's
    /// default applies.
    pub fn new(model: ModelRef, effort: Option<String>) -> ModelChoice {
        let offered = capabilities(&model);
        let effort = effort
            .filter(|effort| offered.efforts.contains(&effort.as_str()))
            .or(offered.default_effort.map(str::to_string));
        ModelChoice {
            model,
            effort,
            named: false,
            problem: None,
        }
    }

    /// Shown discreetly in the Conversation: `deepseek-flash · high`.
    pub fn label(&self) -> String {
        match &self.effort {
            Some(effort) => format!("{} · {effort}", self.model.model),
            None => self.model.model.clone(),
        }
    }

    pub fn has_key(&self) -> bool {
        usable_key(self.model.provider)
    }

    /// What the Operator reads when no key is set.
    pub fn missing_key_message(&self) -> String {
        let problem = self
            .problem
            .as_ref()
            .map(|problem| format!("{problem} "))
            .unwrap_or_default();
        let variable = self.model.provider.key_variable();
        let alternatives = if self.named {
            String::new()
        } else {
            let others: Vec<&str> = PREFERENCE
                .iter()
                .map(|provider| provider.key_variable())
                .filter(|other| *other != variable)
                .collect();
            format!(" (or another provider's key: {})", others.join(", "))
        };
        // A key saved in Settings does not count for Anthropic until W5.7
        // (`usable_key`).
        if self.model.provider == Provider::Anthropic
            && key_status(Provider::Anthropic) == agq_providers::KeyStatus::Stored
        {
            return format!(
                "{problem}The Anthropic key saved in Settings is not used by the Assistant yet. Set {variable} and restart Agentique, or choose another provider in Settings (Ctrl+,), to work with the Assistant. Everything else works as usual."
            );
        }
        if self.model.provider == Provider::Anthropic {
            return format!(
                "{problem}No {} key is set. Set {variable}{alternatives} and restart Agentique, or add another provider's key in Settings (Ctrl+,), to work with the Assistant. Everything else works as usual.",
                self.model.provider.name()
            );
        }
        format!(
            "{problem}No {} key is set. Add one in Settings (Ctrl+,) or set {variable}{alternatives} to work with the Assistant. Everything else works as usual.",
            self.model.provider.name()
        )
    }

    /// A model for one turn.
    pub fn start(&self) -> Box<dyn Model + Send> {
        // A temporary exception to §8.7 rule 2 (code outside agq-providers
        // names no provider): the hand-written Claude client stays
        // Anthropic's path until W5.7 moves it onto rig; it is the path
        // tested against the Claude API's stand-in (§7.6).
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

    fn choose(
        environment: &[(&str, &str)],
        keys: &[Provider],
        settings: (&str, &str, &str),
    ) -> ModelChoice {
        let var = |name: &str| {
            environment
                .iter()
                .find(|(variable, _)| *variable == name)
                .map(|(_, value)| value.to_string())
        };
        let has_key = |provider: Provider| keys.contains(&provider);
        let (provider, model, effort) = settings;
        ModelChoice::resolve(&var, &has_key, Some(provider), Some(model), Some(effort))
    }

    #[test]
    fn settings_choose_the_model_and_the_environment_wins() {
        let deepseek = choose(
            &[],
            &[Provider::DeepSeek],
            ("deepseek", "deepseek-flash", "max"),
        );
        assert_eq!(
            deepseek.model,
            ModelRef::new(Provider::DeepSeek, "deepseek-flash")
        );
        assert_eq!(deepseek.effort.as_deref(), Some("max"));
        // The environment's provider wins, and Settings' model does not go
        // to it.
        let openai = choose(
            &[("AGENTIQUE_PROVIDER", "openai")],
            &[Provider::DeepSeek, Provider::OpenAi],
            ("deepseek", "deepseek-pro", "max"),
        );
        assert_eq!(openai.model.provider, Provider::OpenAi);
        assert_eq!(openai.model.model, Provider::OpenAi.default_model());
        // With the provider automatic, a model typed in Settings is not used.
        let automatic = choose(&[], &[Provider::DeepSeek], ("", "some-model", ""));
        assert_eq!(
            automatic.model,
            ModelRef::new(Provider::DeepSeek, Provider::DeepSeek.default_model())
        );
        // AGENTIQUE_MODEL wins over Settings' model for the same provider.
        let named = choose(
            &[("AGENTIQUE_MODEL", "deepseek-flash")],
            &[Provider::DeepSeek],
            ("deepseek", "deepseek-other", ""),
        );
        assert_eq!(named.model.model, "deepseek-flash");
    }

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
