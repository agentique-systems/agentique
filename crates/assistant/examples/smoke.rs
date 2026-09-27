//! A live turn against the configured model, in the terminal:
//!
//! ```text
//! cargo run -p agq-assistant --example smoke [-- "your request" [max calls]]
//! ```
//!
//! The model is chosen as in the Studio ([`ModelChoice::from_env`]): for
//! example `DEEPSEEK_API_KEY` alone runs `deepseek-flash`. Without a key the
//! example says so and does nothing. Every call passes a spend guard
//! (`AGENTIQUE_SPEND_LOG`, `AGENTIQUE_SPEND_STOP_USD`; see
//! `support/spend.rs`). The Assistant designs into an empty `UrlShortener`
//! package; its questions are answered with "Decide as you think best" and
//! changes to locked elements are refused. Prints the conversation as it
//! happens, then the model and its problems. Exits with 1 if the turn ended
//! with a notice.

#[path = "support/spend.rs"]
mod spend;

use agq_assistant::tools::{self, Prepared};
use agq_assistant::{
    Conversation, Entry, ModelChoice, StreamEvent, ToolCall, ToolResult, TurnEvent, turn,
};
use agq_language::{Source, parse, print};
use agq_system_state::SystemState;
use std::collections::BTreeSet;
use std::io::Write;
use std::sync::atomic::AtomicBool;

fn main() {
    let choice = ModelChoice::from_env();
    if !choice.has_key() {
        println!("{}", choice.missing_key_message());
        return;
    }
    let mut args = std::env::args().skip(1);
    let request = args.next().unwrap_or_else(|| {
        "Design a URL shortener with an HTTP API, link storage and click statistics. Keep it small."
            .to_string()
    });
    let max_calls = args.next().and_then(|n| n.parse().ok()).unwrap_or(12);
    let guard = match spend::SpendGuard::start(
        "smoke",
        &choice.model,
        max_calls,
        agq_assistant::provider_model::MAX_OUTPUT_TOKENS,
    ) {
        Ok(guard) => guard,
        Err(why) => {
            println!("{why}");
            std::process::exit(2);
        }
    };
    let mut model = spend::GuardedModel {
        inner: choice.start(),
        guard: guard.clone(),
        failures: 0,
    };
    println!("Model {}.\nOperator: {request}\n", choice.label());
    let tree = parse(&[Source::new("UrlShortener.sysml", "package UrlShortener;")]);
    let mut state = SystemState::new(tree, BTreeSet::new());
    let mut conversation = Conversation {
        entries: vec![Entry::Operator { text: request }],
    };
    turn::run(
        &mut model,
        &mut conversation,
        &mut |call| execute(&mut state, call),
        &mut show,
        &AtomicBool::new(false),
    );
    for source in print(state.tree()) {
        println!("\n// {}\n{}", source.path, source.text);
    }
    println!("Problems: {}", state.diagnostics().len());
    for problem in state.diagnostics() {
        println!(
            "- {}: {}",
            state.tree().qualified_name(problem.element),
            problem.message
        );
    }
    println!("[spend] this run: ${:.4}", guard.lock().unwrap().run_usd);
    if conversation
        .entries
        .iter()
        .any(|entry| matches!(entry, Entry::Notice { .. }))
    {
        std::process::exit(1);
    }
}

fn execute(state: &mut SystemState, call: &ToolCall) -> ToolResult {
    match tools::prepare(state, &call.name, &call.input) {
        Prepared::Answer(text) => ToolResult::answer(text),
        Prepared::Invalid(message) => ToolResult::error(message),
        Prepared::Question { question, .. } => {
            println!("\n[question] {question}\n[answer] Decide as you think best.");
            ToolResult::answer("Decide as you think best.")
        }
        Prepared::Change(change) => match state.apply(change) {
            Ok(event) => ToolResult::applied(state, &event),
            Err(rejection) => ToolResult::rejected(state, &rejection),
        },
    }
}

fn show(event: TurnEvent) {
    match event {
        TurnEvent::Stream(StreamEvent::Text(text)) => print!("{text}"),
        TurnEvent::Stream(StreamEvent::Thinking(text)) if !text.is_empty() => {
            print!("\x1b[2m{text}\x1b[0m")
        }
        TurnEvent::Stream(StreamEvent::ToolCallStarted { name, .. }) => print!("\n[{name}] "),
        TurnEvent::Stream(StreamEvent::Usage(usage)) => println!(
            "\n[usage] input {} (cache read {}, write {}), output {}",
            usage.input_tokens,
            usage.cache_read_input_tokens,
            usage.cache_creation_input_tokens,
            usage.output_tokens
        ),
        TurnEvent::ToolFinished(result) => {
            let first = result.content.lines().next().unwrap_or_default();
            let mark = if result.is_error { "failed" } else { "ok" };
            println!("{mark}: {first}");
        }
        TurnEvent::Entry(Entry::Notice { text }) => println!("\n[notice] {text}"),
        _ => {}
    }
    let _ = std::io::stdout().flush();
}
