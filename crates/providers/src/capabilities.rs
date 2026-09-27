//! The capability table (ROADMAP §4.8): what each provider and model family
//! can do, as data. Code outside this crate asks it, never a provider's name
//! (§8.7). Values follow the §4.8 matrix; the evaluation set confirms them
//! per provider (A-8). "Not verified" values are marked in comments and
//! chosen conservatively.

use crate::{ModelRef, Provider};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities {
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
// OpenAI and OpenRouter effort levels for this phase's models: not verified.
const OPENAI_EFFORTS: &[&str] = &["low", "medium", "high"];

/// The capabilities of `model`.
pub fn capabilities(model: &ModelRef) -> Capabilities {
    match model.provider {
        Provider::Anthropic => Capabilities {
            tools: true,
            tool_input_streaming: true,
            efforts: ANTHROPIC_EFFORTS,
            default_effort: Some("high"),
            reasoning_text: ReasoningText::Summary,
            // Automatic caching only: rig's cache breakpoints are not used
            // yet (W5.7).
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            // On the default model (C-27), through the Q-18 adapter; the
            // Assistant's hand-written client asks for them on it alone too.
            refusal_fallbacks: model.model == "claude-opus-5",
            context_window: None,
            max_output_tokens: None,
        },
        Provider::OpenAi => Capabilities {
            tools: true,
            tool_input_streaming: true,
            efforts: OPENAI_EFFORTS,
            default_effort: Some("medium"),
            reasoning_text: ReasoningText::Summary,
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            refusal_fallbacks: false,
            context_window: None,
            max_output_tokens: None,
        },
        Provider::OpenRouter => Capabilities {
            tools: true,
            tool_input_streaming: true,
            efforts: OPENAI_EFFORTS,
            default_effort: None,
            reasoning_text: ReasoningText::Full,
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            refusal_fallbacks: false,
            context_window: None,
            max_output_tokens: None,
        },
        Provider::DeepSeek => Capabilities {
            tools: true,
            tool_input_streaming: true,
            efforts: DEEPSEEK_EFFORTS,
            default_effort: Some("high"),
            reasoning_text: ReasoningText::Full,
            prompt_cache: PromptCache::Automatic,
            cache_counts: true,
            refusal_fallbacks: false,
            // `deepseek-flash` (GET /models, 2026-09-27).
            context_window: (model.model == "deepseek-flash").then_some(1_048_576),
            max_output_tokens: (model.model == "deepseek-flash").then_some(393_216),
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
    match (model.provider, model.model.as_str()) {
        // Anthropic's pricing page (ROADMAP [10]; claude-opus-5 from Stage 3's
        // README); cache writes at 1.25 times and reads at a tenth of the
        // input price (not verified per model).
        (Provider::Anthropic, "claude-opus-5") => price(5.0, 6.25, 0.5, 25.0),
        (Provider::Anthropic, "claude-opus-5-5") => price(4.0, 5.0, 0.4, 20.0),
        (Provider::Anthropic, "claude-fable-5-1") => price(10.0, 12.5, 1.0, 50.0),
        (Provider::Anthropic, "claude-haiku-4-5") => price(1.0, 1.25, 0.1, 5.0),
        // DeepSeek's pricing page (ROADMAP [105], read 2026-09-27): the
        // peak-hour price, so the estimate is never low; off-peak costs half.
        (Provider::DeepSeek, "deepseek-flash") => price(0.30, 0.30, 0.006, 1.20),
        _ => None,
    }
}
