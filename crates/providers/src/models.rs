//! Testing a key and listing a provider's models, for Settings (Scenario E,
//! R-25 point 3, §4.8). A key test calls an endpoint that needs the key and
//! sends no prompt, so it costs nothing; the result maps to plain states.

use crate::{
    Capabilities, Error, ErrorKind, ModelRef, Price, Provider, capabilities, price, runtime,
};
use serde_json::Value;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

/// What a key test found (Settings shows each in plain words, §2.4 E2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyCheck {
    /// "Key works".
    Works,
    /// 401: "Key refused".
    Refused,
    /// 402 or 403: "Key has no access".
    NoAccess,
    /// 429: "Rate limited, try later".
    RateLimited,
    /// "Could not reach <provider>", with what went wrong.
    Unreachable(String),
    /// No key was given or configured.
    Missing,
    /// Anything else, with the status.
    Failed(String),
}

/// One model a provider offers, with what Agentique knows about it.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub model: ModelRef,
    /// The provider's display name, or the id.
    pub name: String,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    /// Effort levels the provider lists for the model, when it does.
    pub efforts: Vec<String>,
    /// From the capability table (§4.8).
    pub capabilities: Capabilities,
    /// From the dated price table: an estimate.
    pub price: Option<Price>,
}

/// The base URL for `provider`'s own endpoints (not rig's).
fn base(provider: Provider, endpoint: Option<&str>) -> String {
    endpoint
        .unwrap_or(match provider {
            Provider::Anthropic => "https://api.anthropic.com",
            Provider::OpenAi => "https://api.openai.com",
            Provider::OpenRouter => "https://openrouter.ai",
            Provider::DeepSeek => "https://api.deepseek.com",
            Provider::TypeSafe => "https://api.typesafe.ai",
        })
        .trim_end_matches('/')
        .to_string()
}

/// The endpoint a key test calls: one that needs the key and runs no model.
fn check_path(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic | Provider::OpenAi | Provider::TypeSafe => "/v1/models",
        // OpenRouter's model list is public; its key endpoint is not (not
        // verified live).
        Provider::OpenRouter => "/api/v1/key",
        Provider::DeepSeek => "/user/balance",
    }
}

fn models_path(provider: Provider) -> &'static str {
    match provider {
        // Anthropic pages its list, 20 models by default.
        Provider::Anthropic => "/v1/models?limit=1000",
        Provider::OpenAi | Provider::TypeSafe => "/v1/models",
        Provider::OpenRouter => "/api/v1/models",
        Provider::DeepSeek => "/models",
    }
}

/// GET with the provider's authentication; the status and body.
fn get(provider: Provider, url: String, key: String) -> Result<(u16, String), String> {
    runtime::run(TIMEOUT, async move {
        let client = reqwest::Client::new();
        let request = match provider {
            Provider::Anthropic => client
                .get(&url)
                .header("x-api-key", &key)
                .header("anthropic-version", "2023-06-01"),
            // OpenRouter's model list is public: no key, no header.
            _ if key.is_empty() => client.get(&url),
            _ => client.get(&url).bearer_auth(&key),
        };
        let response = request
            .send()
            .await
            .map_err(|error| error.without_url().to_string())?;
        let status = response.status().as_u16();
        Ok((status, response.text().await.unwrap_or_default()))
    })
    .unwrap_or_else(|failure| {
        Err(match failure {
            runtime::Failure::TimedOut => format!("no answer within {} seconds", TIMEOUT.as_secs()),
            runtime::Failure::Crashed => "the request failed unexpectedly".to_string(),
        })
    })
}

pub(crate) fn check(provider: Provider, key: Option<String>, endpoint: Option<&str>) -> KeyCheck {
    let Some(key) = key else {
        return KeyCheck::Missing;
    };
    let url = format!("{}{}", base(provider, endpoint), check_path(provider));
    match get(provider, url, key) {
        // DeepSeek answers its balance even when there is none to spend.
        Ok((200..=299, body))
            if provider == Provider::DeepSeek && body.contains("\"is_available\":false") =>
        {
            KeyCheck::NoAccess
        }
        Ok((200..=299, _)) => KeyCheck::Works,
        Ok((401, _)) => KeyCheck::Refused,
        // TypeSafe AI answers 403 when the key is missing or not accepted.
        Ok((403, _)) if provider == Provider::TypeSafe => KeyCheck::Refused,
        Ok((402 | 403, _)) => KeyCheck::NoAccess,
        Ok((429, _)) => KeyCheck::RateLimited,
        Ok((status, body)) => KeyCheck::Failed(format!(
            "{} answered {status}: {}",
            provider.name(),
            body.trim().chars().take(200).collect::<String>()
        )),
        Err(error) => KeyCheck::Unreachable(error),
    }
}

pub(crate) fn list(
    provider: Provider,
    key: Option<String>,
    endpoint: Option<&str>,
) -> Result<Vec<ModelInfo>, Error> {
    // OpenRouter's list is public; the others need the key.
    let key = match (key, provider) {
        (Some(key), _) => key,
        (None, Provider::OpenRouter) => String::new(),
        (None, _) => {
            return Err(Error {
                kind: ErrorKind::MissingKey,
                message: format!(
                    "No {} key is set. Set {}.",
                    provider.name(),
                    provider.key_variable()
                ),
            });
        }
    };
    let url = format!("{}{}", base(provider, endpoint), models_path(provider));
    let (status, body) = get(provider, url, key).map_err(|error| Error {
        kind: ErrorKind::Unreachable,
        message: format!("Could not reach {} ({error}).", provider.name()),
    })?;
    if !(200..300).contains(&status) {
        return Err(Error {
            kind: if status == 401 {
                ErrorKind::KeyRefused
            } else {
                ErrorKind::Other
            },
            message: format!("{} did not list its models ({status}).", provider.name()),
        });
    }
    read_models(provider, &body)
}

/// Reads a model list: `{"data": [...]}` from the chat providers,
/// `{"models": [...]}` from TypeSafe AI.
pub fn read_models(provider: Provider, body: &str) -> Result<Vec<ModelInfo>, Error> {
    let value: Value = serde_json::from_str(body).map_err(|error| Error {
        kind: ErrorKind::Other,
        message: format!(
            "{} sent a model list that could not be read ({error}).",
            provider.name()
        ),
    })?;
    let items = value["data"]
        .as_array()
        .or_else(|| value["models"].as_array())
        .cloned()
        .unwrap_or_default();
    let number =
        |item: &Value, fields: &[&str]| fields.iter().find_map(|field| item[*field].as_u64());
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = item["id"]
                .as_str()
                .or_else(|| item["name"].as_str())?
                .to_string();
            let model = ModelRef::new(provider, id.clone());
            let name = item["display_name"]
                .as_str()
                .or_else(|| item["name"].as_str())
                .unwrap_or(&id)
                .to_string();
            let efforts = item["effort"]["supported_levels"]
                .as_array()
                .map(|levels| {
                    levels
                        .iter()
                        .filter_map(|level| level.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Some(ModelInfo {
                capabilities: capabilities(&model),
                price: price(&model),
                context_window: number(item, &["context_window", "context_length"]),
                max_output_tokens: number(item, &["max_output_tokens"])
                    .or_else(|| item["top_provider"]["max_completion_tokens"].as_u64()),
                efforts,
                name,
                model,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_lists_its_window_and_efforts() {
        let body = r#"{"object":"list","data":[{"id":"deepseek-flash","object":"model","name":"DeepSeek-V4.1-Flash","context_window":1048576,"max_output_tokens":393216,"effort":{"supported_levels":["low","high","max"],"default_level":"high"}}]}"#;
        let models = read_models(Provider::DeepSeek, body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "DeepSeek-V4.1-Flash");
        assert_eq!(models[0].context_window, Some(1_048_576));
        assert_eq!(models[0].efforts, ["low", "high", "max"]);
        assert!(models[0].capabilities.tools);
        assert!(models[0].price.is_some());
    }

    #[test]
    fn typesafe_and_anthropic_lists_are_read() {
        let jev = read_models(
            Provider::TypeSafe,
            r#"{"models":[{"name":"jev-latest","description":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(jev[0].model.model, "jev-latest");
        assert!(!jev[0].capabilities.tools);
        let claude = read_models(
            Provider::Anthropic,
            r#"{"data":[{"id":"claude-opus-5","display_name":"Claude Opus 5","type":"model"}],"has_more":false}"#,
        )
        .unwrap();
        assert_eq!(claude[0].name, "Claude Opus 5");
    }
}
