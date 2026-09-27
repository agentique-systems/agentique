//! The tool-use loop with a scripted model and an executor that works like
//! the Studio's: prepare the call, apply a change to the System State as the
//! Assistant, and ask the Operator (here: a fixed answer) about locks and
//! questions.

use agq_assistant::tools::{self, Prepared};
use agq_assistant::turn::{self, MAX_MODEL_CALLS};
use agq_assistant::{
    BackgroundEvent, BackgroundTurn, ClaudeModel, Conversation, Entry, Model, ModelError, Reply,
    Request, ScriptedModel, StreamEvent, ToolCall, ToolResult, TurnEvent, system_prompt,
};
use agq_language::{Source, parse, print};
use agq_system_state::{Actor, Change, Operation, Rejection, SystemState};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn text(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

fn tool(id: &str, name: &str, input: Value) -> Value {
    json!({ "type": "tool_use", "id": id, "name": name, "input": input })
}

fn reply(content: Vec<Value>, stop_reason: &str) -> Reply {
    Reply {
        content,
        stop_reason: stop_reason.to_string(),
    }
}

fn model_of(text: &str) -> SystemState {
    SystemState::new(
        parse(&[Source::new("UrlShortener.sysml", text)]),
        BTreeSet::new(),
    )
}

fn asked(text: &str) -> Conversation {
    Conversation {
        entries: vec![Entry::Operator {
            text: text.to_string(),
        }],
    }
}

/// Carries out tool calls the way the Studio does.
struct Studio {
    state: SystemState,
    /// The Operator's answer when a change touches a locked element.
    allow_locked: bool,
    /// The Operator's answer to questions.
    answer: String,
    /// Every call that reached the executor.
    calls: Vec<ToolCall>,
}

impl Studio {
    fn new(state: SystemState) -> Self {
        Studio {
            state,
            allow_locked: false,
            answer: String::new(),
            calls: Vec::new(),
        }
    }

    fn execute(&mut self, call: &ToolCall) -> ToolResult {
        self.calls.push(call.clone());
        match tools::prepare(&self.state, &call.name, &call.input) {
            Prepared::Answer(text) => ToolResult::answer(text),
            Prepared::Invalid(message) => ToolResult::error(message),
            Prepared::Question { .. } => ToolResult::answer(self.answer.clone()),
            Prepared::Change(mut change) => loop {
                assert_eq!(change.actor, Actor::Assistant);
                match self.state.apply(change.clone()) {
                    Ok(event) => break ToolResult::applied(&self.state, &event),
                    Err(Rejection::Locked { elements })
                        if self.allow_locked && change.confirmed.is_empty() =>
                    {
                        change.confirmed = elements;
                    }
                    Err(rejection) => break ToolResult::rejected(&self.state, &rejection),
                }
            },
        }
    }
}

struct Run {
    conversation: Conversation,
    events: Vec<TurnEvent>,
}

fn run(model: &mut dyn Model, studio: &mut Studio, conversation: Conversation) -> Run {
    run_with_stop(model, studio, conversation, &AtomicBool::new(false))
}

fn run_with_stop(
    model: &mut dyn Model,
    studio: &mut Studio,
    mut conversation: Conversation,
    stop: &AtomicBool,
) -> Run {
    let mut events = Vec::new();
    turn::run(
        model,
        &mut conversation,
        &mut |call| studio.execute(call),
        &mut |event| events.push(event),
        stop,
    );
    Run {
        conversation,
        events,
    }
}

fn results(entry: &Entry) -> &[ToolResult] {
    match entry {
        Entry::ToolResults { results } => results,
        other => panic!("expected tool results, got {other:?}"),
    }
}

fn notices(conversation: &Conversation) -> Vec<&str> {
    conversation
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Notice { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

/// Roles alternate, starting with the user, and every tool call is answered
/// by a result in the next message.
fn assert_well_formed(messages: &[Value]) {
    assert_eq!(messages.first().unwrap()["role"], "user");
    for pair in messages.windows(2) {
        assert_ne!(pair[0]["role"], pair[1]["role"], "{messages:#?}");
    }
    for (index, message) in messages.iter().enumerate() {
        for block in message["content"].as_array().unwrap() {
            if block["type"] == "tool_use" {
                assert!(block["input"].is_object(), "{block}");
                let answered = messages.get(index + 1).is_some_and(|next| {
                    next["content"].as_array().unwrap().iter().any(|result| {
                        result["type"] == "tool_result" && result["tool_use_id"] == block["id"]
                    })
                });
                assert!(answered, "{} is not answered", block["id"]);
            }
        }
    }
}

fn build_operations() -> Value {
    json!([
        { "op": "create", "parent": "UrlShortener", "kind": "item def", "name": "ShortLink" },
        { "op": "create", "parent": "UrlShortener::ShortLink", "kind": "attribute", "name": "code", "type": "ScalarValues::String" },
        { "op": "create", "parent": "UrlShortener", "kind": "port def", "name": "LinkStorePort" },
        { "op": "create", "parent": "UrlShortener::LinkStorePort", "kind": "item", "name": "save", "direction": "in", "type": "ShortLink" },
        { "op": "create", "parent": "UrlShortener::LinkStorePort", "kind": "item", "name": "found", "direction": "out", "type": "ShortLink" },
        { "op": "create", "parent": "UrlShortener", "kind": "interface def", "name": "LinkStorage" },
        { "op": "create", "parent": "UrlShortener::LinkStorage", "kind": "port", "name": "client", "type": "~LinkStorePort", "end": true },
        { "op": "create", "parent": "UrlShortener::LinkStorage", "kind": "port", "name": "store", "type": "LinkStorePort", "end": true },
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "HttpApi", "doc": "Accepts shorten and resolve requests over HTTP." },
        { "op": "create", "parent": "UrlShortener::HttpApi", "kind": "port", "name": "storage", "type": "~LinkStorePort" },
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "LinkStore", "doc": "Persists short links." },
        { "op": "create", "parent": "UrlShortener::LinkStore", "kind": "port", "name": "links", "type": "LinkStorePort" },
        { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "UrlShortenerService" },
        { "op": "create", "parent": "UrlShortener::UrlShortenerService", "kind": "part", "name": "api", "type": "HttpApi" },
        { "op": "create", "parent": "UrlShortener::UrlShortenerService", "kind": "part", "name": "store", "type": "LinkStore" },
        { "op": "connect", "parent": "UrlShortener::UrlShortenerService", "kind": "interface", "name": "storage",
          "definition": "LinkStorage", "from": "api.storage", "to": "store.links" },
        { "op": "create", "parent": "UrlShortener", "kind": "requirement def", "name": "UniqueCodes",
          "doc": "Every short code maps to exactly one long URL." },
        { "op": "create", "parent": "UrlShortener::UniqueCodes", "kind": "subject", "name": "store", "type": "LinkStore" },
        { "op": "create", "parent": "UrlShortener", "kind": "part", "name": "shortener", "type": "UrlShortenerService" },
        { "op": "create", "parent": "UrlShortener", "kind": "requirement", "name": "uniqueCodes", "type": "UniqueCodes" },
        { "op": "create", "parent": "UrlShortener", "kind": "satisfy", "requirement": "uniqueCodes", "by": "shortener.store" }
    ])
}

#[test]
fn a_turn_builds_an_architecture_and_ends() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new([
        reply(
            vec![
                text("I'll read the model first."),
                tool("t1", "read_model", json!({})),
            ],
            "tool_use",
        ),
        reply(
            vec![tool(
                "t2",
                "apply_changes",
                json!({ "description": "Add the API, the link store and the storage interface", "operations": build_operations() }),
            )],
            "tool_use",
        ),
        reply(vec![tool("t3", "get_problems", json!({}))], "tool_use"),
        reply(
            vec![text("The service now has an API and a link store.")],
            "end_turn",
        ),
    ]);
    let run = run(
        &mut model,
        &mut studio,
        asked("Design a URL shortener with an API and storage."),
    );

    // The architecture exists and is valid.
    assert!(
        studio.state.diagnostics().is_empty(),
        "{:?}",
        studio.state.diagnostics()
    );
    let printed = &print(studio.state.tree())[0].text;
    for expected in [
        "end port client : ~LinkStorePort;",
        "interface storage : LinkStorage connect api.storage to store.links;",
        "subject store : LinkStore;",
        "satisfy uniqueCodes by shortener.store;",
        "Persists short links.",
    ] {
        assert!(printed.contains(expected), "{expected} in\n{printed}");
    }

    // The conversation holds each reply and each set of results, and nothing else.
    let kinds: Vec<&str> = run
        .conversation
        .entries
        .iter()
        .map(|entry| match entry {
            Entry::Operator { .. } => "operator",
            Entry::Assistant { .. } => "assistant",
            Entry::ToolResults { .. } => "results",
            Entry::Notice { .. } => "notice",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "operator",
            "assistant",
            "results",
            "assistant",
            "results",
            "assistant",
            "results",
            "assistant"
        ]
    );
    let read = &results(&run.conversation.entries[2])[0];
    assert!(
        read.content.contains("package UrlShortener"),
        "{}",
        read.content
    );
    let applied = &results(&run.conversation.entries[4])[0];
    assert!(!applied.is_error, "{}", applied.content);
    assert!(
        applied
            .content
            .contains("Created: UrlShortener::ShortLink (item def)")
    );
    assert!(
        applied
            .content
            .contains("No problems at the changed elements.")
    );
    let summary = applied.change.as_ref().unwrap();
    assert!(!summary.created.is_empty());
    assert_eq!(summary.problems, 0);
    assert_eq!(
        results(&run.conversation.entries[6])[0].content,
        "No problems."
    );

    // Each request carried the skills, the tools and the conversation so far.
    assert_eq!(model.requests.len(), 4);
    assert_eq!(model.requests[0].system, system_prompt());
    assert_eq!(model.requests[0].tools, tools::definitions());
    let second = &model.requests[1].messages;
    assert_eq!(second.len(), 3);
    assert_eq!(second[2]["content"][0]["tool_use_id"], "t1");
    assert_well_formed(&run.conversation.api_messages());

    // Live events: every tool call finished, every entry was reported.
    let finished = run
        .events
        .iter()
        .filter(|event| matches!(event, TurnEvent::ToolFinished(_)))
        .count();
    assert_eq!(finished, 3);
    let entries: Vec<&Entry> = run
        .events
        .iter()
        .filter_map(|event| match event {
            TurnEvent::Entry(entry) => Some(entry),
            _ => None,
        })
        .collect();
    assert_eq!(entries.len(), 7);
    assert!(run.events.contains(&TurnEvent::Stream(StreamEvent::Text(
        "I'll read the model first.".into()
    ))));

    // The whole build is one undo step.
    studio.state.undo().unwrap();
    assert!(
        studio
            .state
            .tree()
            .find("UrlShortener::LinkStore")
            .is_none()
    );
}

fn locked_state() -> SystemState {
    let mut state = model_of(
        "package UrlShortener {
    item def Link;
    port def LinkStorePort { in item save : Link; out item found : Link; }
    part def LinkStore { port links : LinkStorePort; }
    part def HttpApi { port storage : ~LinkStorePort; }
}",
    );
    let store = state.tree().find("UrlShortener::LinkStore").unwrap();
    state
        .apply(Change::new(
            Actor::Operator,
            "Lock the link store",
            vec![Operation::Lock { element: store }],
        ))
        .unwrap();
    state
}

fn rename_locked_port() -> Vec<Reply> {
    vec![
        reply(
            vec![
                text("The store's port must be renamed for expiring links."),
                tool(
                    "t1",
                    "apply_changes",
                    json!({ "description": "Rename the store port", "operations": [
                        { "op": "rename", "element": "UrlShortener::LinkStore::links", "name": "incoming" }
                    ] }),
                ),
            ],
            "tool_use",
        ),
        reply(
            vec![text("Understood; LinkStore stays as it is.")],
            "end_turn",
        ),
    ]
}

#[test]
fn a_locked_part_is_not_changed_when_the_operator_says_no() {
    let mut studio = Studio::new(locked_state());
    let revision = studio.state.revision();
    let mut model = ScriptedModel::new(rename_locked_port());
    let run = run(&mut model, &mut studio, asked("Add expiring links."));

    let result = &results(&run.conversation.entries[2])[0];
    assert!(result.is_error);
    assert!(
        result.content.contains("did not allow"),
        "{}",
        result.content
    );
    assert!(
        result.content.contains("UrlShortener::LinkStore"),
        "{}",
        result.content
    );
    // Nothing changed, and the loop went on to the model's answer.
    assert_eq!(studio.state.revision(), revision);
    assert!(
        studio
            .state
            .tree()
            .find("UrlShortener::LinkStore::links")
            .is_some()
    );
    assert_eq!(model.requests.len(), 2);
    let sent = &model.requests[1].messages[2]["content"][0];
    assert_eq!(sent["is_error"], true);
    assert!(notices(&run.conversation).is_empty());
}

#[test]
fn a_locked_part_changes_when_the_operator_confirms() {
    let mut studio = Studio::new(locked_state());
    studio.allow_locked = true;
    let mut model = ScriptedModel::new(rename_locked_port());
    let run = run(&mut model, &mut studio, asked("Add expiring links."));
    let result = &results(&run.conversation.entries[2])[0];
    assert!(!result.is_error, "{}", result.content);
    assert!(
        studio
            .state
            .tree()
            .find("UrlShortener::LinkStore::incoming")
            .is_some()
    );
}

#[test]
fn invalid_tool_input_is_answered_with_an_error_and_never_run() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new([
        reply(
            vec![
                // Not JSON at all (the model's text, as streamed).
                tool(
                    "t1",
                    "apply_changes",
                    json!("{\"description\": \"Add \"fast\" links\""),
                ),
                // A field the tool does not have.
                tool("t2", "read_model", json!({ "elementName": "UrlShortener" })),
                // A kind that does not exist.
                tool(
                    "t3",
                    "apply_changes",
                    json!({ "description": "Add", "operations": [{ "op": "create", "kind": "partdef", "name": "X" }] }),
                ),
                // A tool that does not exist.
                tool("t4", "delete_everything", json!({})),
            ],
            "tool_use",
        ),
        reply(vec![text("Sorry, I will fix that.")], "end_turn"),
    ]);
    let run = run(&mut model, &mut studio, asked("Build it."));

    assert!(studio.calls.is_empty(), "{:?}", studio.calls);
    assert_eq!(studio.state.revision(), 0);
    let results = results(&run.conversation.entries[2]);
    assert!(results.iter().all(|result| result.is_error));
    assert!(results[0].content.contains("not a valid JSON object"));
    assert!(results[0].content.contains("Add \"fast\" links"));
    assert!(
        results[1].content.contains("elementName"),
        "{}",
        results[1].content
    );
    assert!(
        results[2].content.contains("must be one of"),
        "{}",
        results[2].content
    );
    assert!(
        results[3].content.contains("no tool called"),
        "{}",
        results[3].content
    );
    // The stored reply can be sent back: the unreadable input became `{}`.
    let Entry::Assistant { content } = &run.conversation.entries[1] else {
        panic!()
    };
    assert_eq!(content[0]["input"], json!({}));
    assert_well_formed(&run.conversation.api_messages());
}

#[test]
fn a_question_gets_the_operators_answer() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    studio.answer = "Part of the API".into();
    let mut model = ScriptedModel::new([
        reply(
            vec![tool(
                "t1",
                "ask_operator",
                json!({ "question": "Should click statistics be a separate service?", "options": ["Separate service", "Part of the API"] }),
            )],
            "tool_use",
        ),
        reply(vec![text("Statistics stay in the API.")], "end_turn"),
    ]);
    let run = run(&mut model, &mut studio, asked("Add click statistics."));
    let result = &results(&run.conversation.entries[2])[0];
    assert_eq!(result.content, "Part of the API");
    assert!(!result.is_error);
    assert_eq!(
        model.requests[1].messages[2]["content"][0]["content"],
        "Part of the API"
    );
}

fn create(id: &str, name: &str) -> Value {
    tool(
        id,
        "apply_changes",
        json!({ "description": format!("Add {name}"), "operations": [
            { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": name }
        ] }),
    )
}

#[test]
fn a_stop_during_tool_calls_keeps_the_work_done_and_a_valid_conversation() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new([
        reply(
            vec![create("t1", "LinkStore"), create("t2", "HttpApi")],
            "tool_use",
        ),
        reply(vec![text("never sent")], "end_turn"),
    ]);
    let mut conversation = asked("Build it.");
    let mut events = Vec::new();
    turn::run(
        &mut model,
        &mut conversation,
        &mut |call| {
            let outcome = studio.execute(call);
            // The Operator presses stop while the first change is applied.
            stop.store(true, Ordering::SeqCst);
            outcome
        },
        &mut |event| events.push(event),
        &stop,
    );

    // The first change stays and can be undone; the second never ran.
    assert!(
        studio
            .state
            .tree()
            .find("UrlShortener::LinkStore")
            .is_some()
    );
    assert!(studio.state.tree().find("UrlShortener::HttpApi").is_none());
    assert_eq!(studio.calls.len(), 1);
    assert_eq!(model.requests.len(), 1);
    let results = results(&conversation.entries[2]);
    assert!(!results[0].is_error);
    assert!(results[1].is_error);
    assert!(results[1].content.contains("stopped by the Operator"));
    assert!(notices(&conversation)[0].starts_with("Stopped by the Operator"));

    // The conversation can go on.
    conversation.entries.push(Entry::Operator {
        text: "Carry on.".into(),
    });
    assert_well_formed(&conversation.api_messages());
    studio.state.undo().unwrap();
    assert!(
        studio
            .state
            .tree()
            .find("UrlShortener::LinkStore")
            .is_none()
    );
}

/// Streams some text, then finds the stop flag set.
struct StoppedWhileWriting;

impl Model for StoppedWhileWriting {
    fn send(
        &mut self,
        _: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        _: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        on_event(StreamEvent::Text("I will add the link ".into()));
        Err(ModelError::Stopped)
    }
}

#[test]
fn a_stop_while_the_reply_streams_keeps_the_text_shown() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let run = run(&mut StoppedWhileWriting, &mut studio, asked("Build it."));
    assert_eq!(
        run.conversation.entries[1],
        Entry::Assistant {
            content: vec![text("I will add the link")]
        }
    );
    assert!(notices(&run.conversation)[0].starts_with("Stopped by the Operator"));
    let mut conversation = run.conversation;
    conversation.entries.push(Entry::Operator {
        text: "Go on.".into(),
    });
    assert_well_formed(&conversation.api_messages());
}

#[test]
fn a_reply_cut_off_at_the_output_limit_runs_no_tools() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new([reply(vec![create("t1", "LinkStore")], "max_tokens")]);
    let run = run(&mut model, &mut studio, asked("Build it."));
    assert!(studio.calls.is_empty());
    assert!(results(&run.conversation.entries[2])[0].is_error);
    assert!(notices(&run.conversation)[0].contains("output limit"));
    assert_well_formed(&run.conversation.api_messages());
}

#[test]
fn a_refusal_ends_the_turn_and_runs_nothing() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new([reply(
        vec![text("Sure, "), create("t1", "LinkStore")],
        "refusal",
    )]);
    let run = run(&mut model, &mut studio, asked("Build it."));
    assert!(studio.calls.is_empty());
    assert_eq!(run.conversation.entries.len(), 2);
    assert!(notices(&run.conversation)[0].contains("declined"));
    // The card of the call shown as started is closed.
    assert!(run.events.iter().any(|event| matches!(
        event,
        TurnEvent::ToolFinished(result) if result.tool_use_id == "t1" && result.is_error
    )));
}

/// Starts a tool call on the stream, then ends without it: stopped, or with
/// a reply that no longer contains it (as after a switch to a fallback model).
struct DropsItsCall {
    stopped: bool,
}

impl Model for DropsItsCall {
    fn send(
        &mut self,
        _: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        _: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        on_event(StreamEvent::ToolCallStarted {
            id: "t0".into(),
            name: "read_model".into(),
        });
        if self.stopped {
            return Err(ModelError::Stopped);
        }
        Ok(reply(vec![text("Done.")], "end_turn"))
    }
}

#[test]
fn every_tool_call_shown_as_started_ends() {
    for stopped in [false, true] {
        let mut studio = Studio::new(model_of("package UrlShortener;"));
        let run = run(
            &mut DropsItsCall { stopped },
            &mut studio,
            asked("Read it."),
        );
        let finished: Vec<&ToolResult> = run
            .events
            .iter()
            .filter_map(|event| match event {
                TurnEvent::ToolFinished(result) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(finished.len(), 1, "stopped: {stopped}");
        assert_eq!(finished[0].tool_use_id, "t0");
        assert!(finished[0].content.starts_with("Not run"));
        assert!(studio.calls.is_empty());
    }
}

#[test]
fn unusual_endings_are_explained() {
    for (reason, expected) in [
        (
            "model_context_window_exceeded",
            "The conversation is too long for the model",
        ),
        ("pause_turn", "Send a message to let the Assistant continue"),
    ] {
        let mut studio = Studio::new(model_of("package UrlShortener;"));
        let mut model = ScriptedModel::new([reply(vec![text("Working on")], reason)]);
        let run = run(&mut model, &mut studio, asked("Build it."));
        assert!(notices(&run.conversation)[0].contains(expected), "{reason}");
    }
}

#[test]
fn a_turn_that_never_ends_pauses() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new((0..MAX_MODEL_CALLS + 5).map(|index| {
        reply(
            vec![tool(&format!("t{index}"), "get_problems", json!({}))],
            "tool_use",
        )
    }));
    let run = run(&mut model, &mut studio, asked("Check."));
    assert_eq!(model.requests.len(), MAX_MODEL_CALLS);
    assert!(notices(&run.conversation)[0].starts_with("Paused after 40 steps"));
    assert_well_formed(&run.conversation.api_messages());
}

#[test]
fn a_missing_api_key_is_a_clear_notice_and_the_message_is_kept() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ClaudeModel::from_env();
    model.key = None;
    let run = run(&mut model, &mut studio, asked("Design a URL shortener."));
    assert_eq!(
        run.conversation.entries,
        [
            Entry::Operator {
                text: "Design a URL shortener.".into()
            },
            Entry::Notice {
                text: ModelError::MissingKey.to_string()
            }
        ]
    );
    // After a failed request the next message still forms a valid request.
    let mut conversation = run.conversation;
    conversation.entries.push(Entry::Operator {
        text: "Try again.".into(),
    });
    let messages = conversation.api_messages();
    assert_eq!(messages.len(), 1);
    assert_well_formed(&messages);
}

#[test]
fn nothing_is_sent_without_a_message_to_answer() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ScriptedModel::new([]);
    let run = run(&mut model, &mut studio, Conversation::default());
    assert!(model.requests.is_empty());
    assert_eq!(notices(&run.conversation).len(), 1);
}

/// Runs a background turn to its end, carrying out tool calls on this
/// thread like the Studio's UI thread, and returns the Studio's copy of the
/// conversation.
fn drive(
    background: &BackgroundTurn,
    studio: &mut Studio,
    mut conversation: Conversation,
) -> Conversation {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "the turn did not finish");
        match background.next_event() {
            Some(BackgroundEvent::Turn(TurnEvent::Entry(entry))) => {
                conversation.entries.push(entry)
            }
            Some(BackgroundEvent::Turn(_)) => {}
            Some(BackgroundEvent::ToolCall { call, reply }) => {
                reply.send(studio.execute(&call)).unwrap();
            }
            Some(BackgroundEvent::Finished) => return conversation,
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

#[test]
fn the_assistant_runs_a_turn_in_the_background() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let model = ScriptedModel::new([
        reply(
            vec![text("Adding the store."), create("t1", "LinkStore")],
            "tool_use",
        ),
        reply(vec![text("Done.")], "end_turn"),
    ]);
    let conversation = asked("Add a link store.");
    let background = BackgroundTurn::start(Box::new(model), conversation.clone());
    let conversation = drive(&background, &mut studio, conversation);

    assert!(
        studio
            .state
            .tree()
            .find("UrlShortener::LinkStore")
            .is_some()
    );
    assert_eq!(conversation.entries.len(), 4);
    assert_eq!(
        conversation.entries[3],
        Entry::Assistant {
            content: vec![text("Done.")]
        }
    );
}

#[test]
fn without_an_api_key_the_background_turn_ends_with_a_notice() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let mut model = ClaudeModel::from_env();
    model.key = None;
    let conversation = asked("Design a URL shortener.");
    let background = BackgroundTurn::start(Box::new(model), conversation.clone());
    let conversation = drive(&background, &mut studio, conversation);
    assert_eq!(
        notices(&conversation),
        [ModelError::MissingKey.to_string().as_str()]
    );
    assert_eq!(studio.state.revision(), 0);
}

/// Thinks until it is stopped.
struct Thinking;

impl Model for Thinking {
    fn send(
        &mut self,
        _: &Request,
        on_event: &mut dyn FnMut(StreamEvent),
        stop: &AtomicBool,
    ) -> Result<Reply, ModelError> {
        on_event(StreamEvent::Thinking(String::new()));
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        Err(ModelError::Stopped)
    }
}

#[test]
fn stopping_the_assistant_ends_the_turn_at_once() {
    let mut studio = Studio::new(model_of("package UrlShortener;"));
    let conversation = asked("Build it.");
    let background = BackgroundTurn::start(Box::new(Thinking), conversation.clone());
    // Wait until the model is at work, then stop.
    loop {
        if let Some(BackgroundEvent::Turn(TurnEvent::Stream(StreamEvent::Thinking(_)))) =
            background.next_event()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let stopped = Instant::now();
    background.stop();
    let conversation = drive(&background, &mut studio, conversation);
    assert!(
        stopped.elapsed() < Duration::from_millis(500),
        "{:?}",
        stopped.elapsed()
    );
    assert!(notices(&conversation)[0].starts_with("Stopped by the Operator"));
}

#[test]
fn a_stop_while_a_tool_call_waits_for_the_studio_ends_the_turn() {
    let model = ScriptedModel::new([
        reply(
            vec![tool(
                "t1",
                "ask_operator",
                json!({ "question": "Separate statistics service?" }),
            )],
            "tool_use",
        ),
        reply(vec![text("never sent")], "end_turn"),
    ]);
    let background = BackgroundTurn::start(Box::new(model), asked("Add statistics."));
    // The question is open in the Studio: the reply channel is held, unanswered.
    let pending = loop {
        if let Some(BackgroundEvent::ToolCall { reply, .. }) = background.next_event() {
            break reply;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    let stopped = Instant::now();
    background.stop();
    let mut entries = Vec::new();
    loop {
        assert!(
            stopped.elapsed() < Duration::from_millis(500),
            "the turn did not end"
        );
        match background.next_event() {
            Some(BackgroundEvent::Turn(TurnEvent::Entry(entry))) => entries.push(entry),
            Some(BackgroundEvent::Finished) => break,
            _ => std::thread::sleep(Duration::from_millis(1)),
        }
    }
    drop(pending);
    let Entry::ToolResults { results } = &entries[0] else {
        panic!("expected tool results, got {entries:?}");
    };
    assert_eq!(results[0].content, "Not run: stopped by the Operator.");
    assert!(
        matches!(&entries[1], Entry::Notice { text } if text.starts_with("Stopped by the Operator"))
    );
}

#[test]
fn the_modelling_example_is_valid_as_written() {
    let skill = include_str!("../skills/modelling.md");
    let example = skill
        .split("```sysml")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the modelling skill has an example");
    let state = model_of(example);
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
}

#[test]
fn the_skills_explain_every_tool() {
    let prompt = system_prompt();
    for definition in tools::definitions().as_array().unwrap() {
        let name = definition["name"].as_str().unwrap();
        assert!(prompt.contains(name), "the skills do not mention {name}");
    }
    for topic in [
        "Map ideas onto the architecture first",
        "Ask on major decisions",
        "Locks",
        "slop",
    ] {
        assert!(prompt.contains(topic), "{topic}");
    }
}
