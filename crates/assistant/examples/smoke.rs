//! A live turn against the Claude API, in the terminal:
//!
//! ```text
//! cargo run -p agq-assistant --example smoke [-- "your request"]
//! ```
//!
//! Needs `ANTHROPIC_API_KEY`; without it the example says so and does
//! nothing. The Assistant designs into an empty `UrlShortener` package; its
//! questions are answered with "Decide as you think best" and changes to
//! locked elements are refused. Prints the conversation as it happens, then
//! the model and its problems. Exits with 1 if the turn ended with a notice.

use agq_assistant::tools::{self, Prepared};
use agq_assistant::{
    ClaudeModel, Conversation, Entry, StreamEvent, ToolCall, ToolResult, TurnEvent, turn,
};
use agq_language::{Source, parse, print};
use agq_system_state::SystemState;
use std::collections::BTreeSet;
use std::io::Write;
use std::sync::atomic::AtomicBool;

fn main() {
    let mut model = ClaudeModel::from_env();
    if !model.has_key() {
        println!("ANTHROPIC_API_KEY is not set, so there is nothing to try.");
        return;
    }
    let request = std::env::args().nth(1).unwrap_or_else(|| {
        "Design a URL shortener with an HTTP API, link storage and click statistics. Keep it small."
            .to_string()
    });
    println!(
        "Model {} at effort {}.\nOperator: {request}\n",
        model.model, model.effort
    );
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
        TurnEvent::Stream(StreamEvent::ToolCallStarted { name, .. }) => print!("\n[{name}] "),
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
