//! The capability table (ROADMAP §4.8): what each provider and model family
//! can do, as data. Code outside this crate asks it, never a provider's name
//! (§8.7). Values follow the §4.8 matrix; the evaluation set confirms them
//! per provider (A-8). "Not verified" values are marked in comments and
//! chosen conservatively.

use crate::{ModelRef, Provider};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
    /// The model answers a conversation (streamed chat); an agent's live
    /// evaluation can ask it through an answer template.
    pub chat: bool,
    /// The model makes typed decisions (a choice, yes or no, a score) and is
    /// a known pinned version (C-35, C-52): an agent's live evaluation asks
    /// it a typed question. Aliases and unknown versions are not admitted.
    pub decisions: bool,
    /// The model can call tools; without it the Assistant cannot use it.
    pub tools: bool,
    /// Tool input arrives while it is generated, so a tool card can show it.
    pub tool_input_streaming: bool,
    /// Effort levels the model accepts, lowest first; empty when effort
    /// cannot be set (the effort control is hidden).
    pub efforts: &'static [&'static str],
    /// The effort used when none is chosen.
    pub default_effort: Option<&'static str>,
    pub reasoning_text: ReasoningText,
    pub prompt_cache: PromptCache,
    /// Usage separates cache reads (and writes) from full-price input.
    pub cache_counts: bool,
    /// The provider reruns a declined request on another model (C-27).
    pub refusal_fallbacks: bool,
    /// The Claude Agent runtime can run the model: Anthropic's own API, or
    /// the provider's Anthropic-compatible endpoint (C-53, §4.8).
    pub agent_runtime: bool,
    /// Context window in tokens, when known.
    pub context_window: Option<u64>,
    /// Largest output per call in tokens, when known.
    pub max_output_tokens: Option<u64>,
}

/// What the thinking row can show (§4.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReasoningText {
    /// Nothing readable: "Thinking…" with the elapsed time.
    None,
    /// Summaries of the reasoning.
    Summary,
    /// The reasoning itself.
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptCache {
    None,
    /// The provider caches on its own.
    Automatic,
    /// Explicit cache breakpoints.
    Manual,
}

const ANTHROPIC_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
const DEEPSEEK_EFFORTS: &[&str] = &["low", "high", "max"];
/// The DeepSeek models this table knows.
const DEEPSEEK_MODELS: &[&str] = &["deepseek-flash", "deepseek-v4-pro"];
// OpenAI and OpenRouter effort levels for this phase's models: not verified.
const OPENAI_EFFORTS: &[&str] = &["low", "medium", "high"];

/// The typed-decision models this layer knows, as pinned versions (C-52).
/// Only these are admitted for an agent: an alias such as `jev-latest` can
/// move under a scenario's feet, and an unknown version has no verified
/// behaviour here. Add a version after checking it.
pub const DECISION_MODELS: &[&str] = &[crate::jev::DEFAULT_MODEL];

/// The model a model id names: the provider whose own table knows the id
/// (its default model, a model it prices, or a decision model it knows),
/// never a guess from the id's spelling (§8.7). `None` when no table knows
/// it.
pub fn resolve_model(id: &str) -> Option<ModelRef> {
    let id = id.trim();
    Provider::ALL
        .into_iter()
        .map(|provider| ModelRef::new(provider, id))
        .find(|model| {
            model.provider.default_model() == id
                || price(model).is_some()
                || capabilities(model).decisions
        })
}

/// A model's documented base id. Anthropic names a dated snapshot of a
/// model `<id>-YYYYMMDD` (`claude-haiku-4-5-20251001` is `claude-haiku-4-5`),
/// and usage may report it by either; that date is the only suffix taken
/// off, and only for Anthropic, so the tables price and describe a snapshot
/// as its model. Every other id is taken as written.
pub fn base_id(model: &ModelRef) -> &str {
    let id = model.model.as_str();
    if model.provider == Provider::Anthropic
        && let Some((base, date)) = id.rsplit_once('-')
        && date.len() == 8
        && date.bytes().all(|b| b.is_ascii_digit())
    {
        return base;
    }
    id
}

/// The capabilities of `model`.
pub fn capabilities(model: &ModelRef) -> Capabilities {
    match model.provider {
        Provider::Anthropic => Capabilities {
            chat: true,
            decisions: false,
            tools: true,
            // rig 0.43 hands a tool call over whole when it closes.
            tool_input_streaming: false,
            efforts: ANTHROPIC_EFFORTS,
            // Claude Opus 5.5 thinks at medium unless asked (ROADMAP [109]).
            default_effort: Some(if base_id(model) == "claude-opus-5-5" {
                "medium"
            } else {
                "high"
            }),
            reasoning_text: ReasoningText::Summary,
            // Automatic caching only: rig's cache breakpoints are not used
            // yet (W5.7).
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            // On Claude Opus 5 (C-27), through the Q-18 adapter; the
            // Assistant's hand-written client asks for them on it alone too.
            refusal_fallbacks: base_id(model) == "claude-opus-5",
            agent_runtime: true,
            context_window: None,
            max_output_tokens: None,
        },
        Provider::OpenAi => Capabilities {
            chat: true,
            decisions: false,
            tools: true,
            // rig 0.43 hands a tool call over whole when it closes.
            tool_input_streaming: false,
            efforts: OPENAI_EFFORTS,
            default_effort: Some("medium"),
            reasoning_text: ReasoningText::Summary,
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            refusal_fallbacks: false,
            agent_runtime: false,
            context_window: None,
            max_output_tokens: None,
        },
        Provider::OpenRouter => Capabilities {
            chat: true,
            decisions: false,
            tools: true,
            // rig 0.43 hands a tool call over whole when it closes.
            tool_input_streaming: false,
            efforts: OPENAI_EFFORTS,
            default_effort: None,
            reasoning_text: ReasoningText::Full,
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            refusal_fallbacks: false,
            agent_runtime: false,
            context_window: None,
            max_output_tokens: None,
        },
        // Typed decisions only: no conversation and no tools, so never the
        // Assistant's model (§4.8, Jev column).
        Provider::TypeSafe => Capabilities {
            chat: false,
            decisions: DECISION_MODELS.contains(&model.model.as_str()),
            tools: false,
            tool_input_streaming: false,
            efforts: &[],
            default_effort: None,
            reasoning_text: ReasoningText::None,
            prompt_cache: PromptCache::None,
            cache_counts: false,
            refusal_fallbacks: false,
            agent_runtime: false,
            context_window: Some(64_000),
            max_output_tokens: None,
        },
        Provider::DeepSeek => Capabilities {
            chat: true,
            decisions: false,
            tools: true,
            // rig 0.43 hands a tool call over whole when it closes.
            tool_input_streaming: false,
            efforts: DEEPSEEK_EFFORTS,
            default_effort: Some("high"),
            reasoning_text: ReasoningText::Full,
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            refusal_fallbacks: false,
            agent_runtime: true,
            // `deepseek-flash` (GET /models, 2026-09-27) and `deepseek-v4-pro`
            // (the pricing page, 2026-10-03): 1M context, 384K output.
            context_window: DEEPSEEK_MODELS
                .contains(&model.model.as_str())
                .then_some(1_048_576),
            max_output_tokens: DEEPSEEK_MODELS
                .contains(&model.model.as_str())
                .then_some(393_216),
        },
    }
}

/// List prices in US dollars per million tokens, from a dated table; an
/// estimate, never a bill (C-37, R-42).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Price {
    pub input: f64,
    pub cache_write: f64,
    pub cache_read: f64,
    pub output: f64,
    /// When the price was read from the provider's page.
    pub as_of: &'static str,
}

/// The list price of `model`, if known.
pub fn price(model: &ModelRef) -> Option<Price> {
    let price = |input: f64, cache_write: f64, cache_read: f64, output: f64| {
        Some(Price {
            input,
            cache_write,
            cache_read,
            output,
            as_of: "2026-09-27",
        })
    };
    match (model.provider, base_id(model)) {
        // Anthropic's pricing page (ROADMAP [10]; claude-opus-5 from Stage 3's
        // README); cache writes at 1.25 times and reads at a tenth of the
        // input price (not verified per model).
        (Provider::Anthropic, "claude-opus-5") => price(5.0, 6.25, 0.5, 25.0),
        // Claude Opus 5.5 and Sonnet 5.5: input, output and cache reads read
        // 2026-10-04 (ROADMAP [109]); cache writes derived at 1.25 times the
        // input price, as above (not read there).
        (Provider::Anthropic, "claude-opus-5-5") => Some(Price {
            input: 4.0,
            cache_write: 5.0,
            cache_read: 0.20,
            output: 20.0,
            as_of: "2026-10-04",
        }),
        (Provider::Anthropic, "claude-sonnet-5-5") => Some(Price {
            input: 2.0,
            cache_write: 2.50,
            cache_read: 0.20,
            output: 10.0,
            as_of: "2026-10-04",
        }),
        (Provider::Anthropic, "claude-fable-5-1") => price(10.0, 12.5, 1.0, 50.0),
        (Provider::Anthropic, "claude-haiku-4-5") => price(1.0, 1.25, 0.1, 5.0),
        // DeepSeek's pricing page (ROADMAP [105], read 2026-09-27): the
        // peak-hour price, so the estimate is never low; off-peak costs half.
        (Provider::DeepSeek, "deepseek-flash") => price(0.30, 0.30, 0.006, 1.20),
        // The same page, read 2026-10-03 for the Claude Agent runtime on
        // DeepSeek's Anthropic-compatible endpoint (C-53); peak price.
        (Provider::DeepSeek, "deepseek-v4-pro") => Some(Price {
            input: 1.32,
            cache_write: 1.32,
            cache_read: 0.044,
            output: 3.96,
            as_of: "2026-10-03",
        }),
        // TypeSafe AI's model page (ROADMAP [98]): input only, output free;
        // for the known pinned versions only, never by a name's prefix.
        (Provider::TypeSafe, model) if DECISION_MODELS.contains(&model) => {
            price(0.042, 0.0, 0.0, 0.0)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_resolve_by_the_tables_never_by_their_spelling() {
        assert_eq!(
            resolve_model("deepseek-flash"),
            Some(ModelRef::new(Provider::DeepSeek, "deepseek-flash"))
        );
        assert_eq!(
            resolve_model("claude-haiku-4-5"),
            Some(ModelRef::new(Provider::Anthropic, "claude-haiku-4-5"))
        );
        let jev = resolve_model("jev-1.13.0").unwrap();
        assert_eq!(jev.provider, Provider::TypeSafe);
        let caps = capabilities(&jev);
        assert!(caps.decisions && !caps.chat && !caps.tools);
        // Aliases, unknown versions and look-alikes are known to nobody.
        for unknown in [
            "jev-latest",
            "jev-preview",
            "jev-1.14.0",
            "jev",
            "jevious",
            "mystery",
        ] {
            assert_eq!(resolve_model(unknown), None, "{unknown}");
            let as_typesafe = ModelRef::new(Provider::TypeSafe, unknown);
            assert!(!capabilities(&as_typesafe).decisions, "{unknown}");
            assert_eq!(price(&as_typesafe), None, "{unknown}");
        }
        for chat in [
            Provider::Anthropic,
            Provider::OpenAi,
            Provider::OpenRouter,
            Provider::DeepSeek,
        ] {
            let caps = capabilities(&ModelRef::new(chat, chat.default_model()));
            assert!(caps.chat && !caps.decisions, "{chat:?}");
        }
    }

    /// C-54, ROADMAP [109]: the Claude 5.5 models' list prices, their effort
    /// levels and defaults; both are known to the tables, and the Claude
    /// Agent runtime reaches Anthropic's and DeepSeek's models only.
    #[test]
    fn the_claude_5_5_models_are_priced_with_their_efforts() {
        let opus = ModelRef::new(Provider::Anthropic, "claude-opus-5-5");
        let sonnet = ModelRef::new(Provider::Anthropic, "claude-sonnet-5-5");
        let p = price(&opus).unwrap();
        assert_eq!(
            (p.input, p.cache_write, p.cache_read, p.output, p.as_of),
            (4.0, 5.0, 0.20, 20.0, "2026-10-04")
        );
        let p = price(&sonnet).unwrap();
        assert_eq!(
            (p.input, p.cache_write, p.cache_read, p.output, p.as_of),
            (2.0, 2.50, 0.20, 10.0, "2026-10-04")
        );
        for model in [&opus, &sonnet] {
            assert_eq!(resolve_model(&model.model).as_ref(), Some(model));
            let caps = capabilities(model);
            assert_eq!(caps.efforts, ["low", "medium", "high", "xhigh", "max"]);
            assert!(caps.chat && caps.tools && caps.agent_runtime);
        }
        assert_eq!(capabilities(&opus).default_effort, Some("medium"));
        assert_eq!(capabilities(&sonnet).default_effort, Some("high"));
        // The Assistant's Anthropic default (C-54).
        assert_eq!(Provider::Anthropic.default_model(), "claude-sonnet-5-5");
        let runtime: Vec<Provider> = Provider::ALL
            .into_iter()
            .filter(|p| capabilities(&ModelRef::new(*p, p.default_model())).agent_runtime)
            .collect();
        assert_eq!(runtime, [Provider::Anthropic, Provider::DeepSeek]);
        // A thousand cache reads of Opus 5.5 cost $0.0002, not $0.0004.
        let usage = crate::Usage {
            cache_read_tokens: 1000,
            ..crate::Usage::default()
        };
        assert!((usage.cost_usd(&opus).unwrap() - 0.0002).abs() < 1e-12);
    }

    /// A dated Anthropic snapshot (as usage may report a subagent's model)
    /// is priced as its documented base id; nothing else is guessed.
    #[test]
    fn a_dated_snapshot_is_its_model() {
        let dated = ModelRef::new(Provider::Anthropic, "claude-haiku-4-5-20251001");
        assert_eq!(base_id(&dated), "claude-haiku-4-5");
        assert_eq!(
            price(&dated),
            price(&ModelRef::new(Provider::Anthropic, "claude-haiku-4-5"))
        );
        assert_eq!(resolve_model("claude-haiku-4-5-20251001"), Some(dated));
        assert_eq!(
            capabilities(&ModelRef::new(
                Provider::Anthropic,
                "claude-opus-5-5-20261001"
            ))
            .default_effort,
            Some("medium")
        );
        // Not a date, another provider, or an unknown model: as written.
        for (provider, id) in [
            (Provider::Anthropic, "claude-haiku-4-5-2025"),
            (Provider::Anthropic, "claude-mystery-20251001"),
            (Provider::DeepSeek, "deepseek-flash-20251001"),
        ] {
            assert_eq!(price(&ModelRef::new(provider, id)), None, "{id}");
        }
    }

    #[test]
    fn a_model_is_written_and_read_as_provider_slash_model() {
        let opus = ModelRef::new(Provider::Anthropic, "claude-opus-5-5");
        assert_eq!(opus.to_string(), "anthropic/claude-opus-5-5");
        assert_eq!(ModelRef::parse(&opus.to_string()), Some(opus));
        // A model id may hold a slash itself (OpenRouter's do).
        assert_eq!(
            ModelRef::parse("openrouter/anthropic/claude-opus-5"),
            Some(ModelRef::new(
                Provider::OpenRouter,
                "anthropic/claude-opus-5"
            ))
        );
        for bad in ["", "deepseek", "deepseek/", "nobody/x", "/x"] {
            assert_eq!(ModelRef::parse(bad), None, "{bad}");
        }
    }
}
