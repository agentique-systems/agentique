//! The Assistant's skills: what it knows about working with the Operator and
//! modelling in Agentique, written as short Markdown files in `skills/` and
//! compiled in as one system prompt.
//!
//! The prompt is the same text for every request, so it is cached by the
//! API; nothing that changes per project or per turn belongs in it.

/// The system prompt: every skill, in a fixed order.
pub fn system_prompt() -> &'static str {
    concat!(
        include_str!("../skills/assistant.md"),
        "\n",
        include_str!("../skills/modelling.md"),
        "\n",
        include_str!("../skills/simplicity.md"),
        "\n",
        include_str!("../skills/ideas.md"),
        "\n",
        include_str!("../skills/decisions.md"),
        "\n",
        include_str!("../skills/locks.md"),
        "\n",
        include_str!("../skills/tools.md"),
    )
}
