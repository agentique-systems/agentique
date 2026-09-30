use agq_assistant::tools::{
    self, APPLY_CHANGES, ASK_OPERATOR, FIND_ELEMENTS, READ_LIBRARY_BLOCK, READ_MODEL,
    SAVE_TO_LIBRARY, SEARCH_LIBRARY, USE_LIBRARY_BLOCK,
};
use agq_assistant::{Conversation, Entry, Prepared, ToolResult};
use agq_language::{Source, parse, print};
use agq_library::Library;
use agq_providers::{AssistantPart, ModelRef, Provider, Reasoning, ReasoningPart};
use agq_system_state::{Actor, Change, Operation, Rejection, SystemState};
use serde_json::json;
use std::collections::BTreeSet;

fn state() -> SystemState {
    let text = "package UrlShortener {
    item def Link;
    port def LinkStorePort { in item save : Link; out item found : Link; }
    part def LinkStore { port links : LinkStorePort; }
    part def HttpApi { port storage : ~LinkStorePort; }
}";
    SystemState::new(parse(&[Source::new("model.sysml", text)]), BTreeSet::new())
}

fn change(prepared: Prepared) -> Change {
    match prepared {
        Prepared::Change(change) => change,
        other => panic!("expected a change, got {other:?}"),
    }
}

#[test]
fn every_tool_has_a_schema() {
    let definitions = tools::definitions();
    let names: Vec<&str> = definitions
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "read_model",
            "find_elements",
            "get_problems",
            "apply_changes",
            "ask_operator",
            "inspect_behaviour",
            "list_scenarios",
            "run_scenario",
            "stop_run",
            "read_run",
            "read_code_links",
            "check_implementation",
            "search_library",
            "read_library_block",
            "use_library_block",
            "save_to_library"
        ]
    );
    for tool in definitions.as_array().unwrap() {
        assert_eq!(tool["input_schema"]["type"], "object");
    }
}

#[test]
fn one_change_can_build_nested_parts_and_connect_them() {
    let mut state = state();
    let prepared = tools::prepare(
        &state,
        &Library::built_in_only(),
        APPLY_CHANGES,
        &json!({
            "description": "Add the service with its API and store",
            "operations": [
                { "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "Service",
                  "doc": "The URL shortener service." },
                { "op": "create", "parent": "UrlShortener::Service", "kind": "part", "name": "api", "type": "HttpApi" },
                { "op": "create", "parent": "UrlShortener::Service", "kind": "part", "name": "store", "type": "LinkStore",
                  "multiplicity": "1" },
                { "op": "connect", "parent": "UrlShortener::Service", "name": "storage",
                  "from": "api.storage", "to": "store.links" }
            ]
        }),
    );
    let event = state
        .apply(change(prepared))
        .expect("applies as one change");
    assert_eq!(event.actor, Actor::Assistant);
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text = &print(state.tree())[0].text;
    assert!(text.contains("part store : LinkStore[1];"), "{text}");
    assert!(
        text.contains("connection storage connect api.storage to store.links;"),
        "{text}"
    );
    assert!(text.contains("The URL shortener service."), "{text}");
    let result = tools::describe_event(&state, &event);
    assert!(
        result.contains("Created: UrlShortener::Service (part def)"),
        "{result}"
    );
    assert!(
        result.contains("No problems at the changed elements."),
        "{result}"
    );
    // One undo step removes all of it.
    state.undo().unwrap();
    assert!(state.tree().find("UrlShortener::Service").is_none());
}

#[test]
fn a_bad_operation_is_explained_and_nothing_is_prepared() {
    let state = state();
    let prepared = tools::prepare(
        &state,
        &Library::built_in_only(),
        APPLY_CHANGES,
        &json!({
            "description": "Rename something that does not exist",
            "operations": [{ "op": "rename", "element": "UrlShortener::Nope", "name": "X" }]
        }),
    );
    let Prepared::Invalid(message) = prepared else {
        panic!("expected an explanation, got {prepared:?}");
    };
    assert!(message.contains("operation 1"), "{message}");
    assert!(message.contains("UrlShortener::Nope"), "{message}");
}

#[test]
fn a_locked_part_is_not_changed_without_the_operator() {
    let mut state = state();
    let store = state.tree().find("UrlShortener::LinkStore").unwrap();
    state
        .apply(Change::new(
            Actor::Operator,
            "Lock",
            vec![Operation::Lock { element: store }],
        ))
        .unwrap();
    let request = json!({
        "description": "Rename the store's port",
        "operations": [{ "op": "rename", "element": "UrlShortener::LinkStore::links", "name": "incoming" }]
    });
    let rejection = state
        .apply(change(tools::prepare(
            &state,
            &Library::built_in_only(),
            APPLY_CHANGES,
            &request,
        )))
        .unwrap_err();
    assert_eq!(
        rejection,
        Rejection::Locked {
            elements: vec![store]
        }
    );
    let result = tools::describe_rejection(&state, &rejection);
    assert!(result.contains("UrlShortener::LinkStore"), "{result}");
    assert!(
        state
            .tree()
            .find("UrlShortener::LinkStore::links")
            .is_some()
    );
}

#[test]
fn a_change_prepared_on_an_older_model_is_stale() {
    let mut state = state();
    let request = json!({
        "description": "Add a part def",
        "operations": [{ "op": "create", "parent": "UrlShortener", "kind": "part def", "name": "Stats" }]
    });
    let prepared = change(tools::prepare(
        &state,
        &Library::built_in_only(),
        APPLY_CHANGES,
        &request,
    ));
    state
        .apply(Change::new(
            Actor::Operator,
            "Meanwhile",
            vec![Operation::Rename {
                element: state.tree().find("UrlShortener::HttpApi").unwrap(),
                name: "Api".into(),
            }],
        ))
        .unwrap();
    let rejection = state.apply(prepared).unwrap_err();
    assert!(tools::describe_rejection(&state, &rejection).contains("Read the model again"));
}

#[test]
fn reading_and_finding_and_asking() {
    let state = state();
    let Prepared::Answer(model) =
        tools::prepare(&state, &Library::built_in_only(), READ_MODEL, &json!({}))
    else {
        panic!()
    };
    // Without an element: an outline, not the whole text (R-34).
    assert!(
        model.contains(
            "
  LinkStore (part def)
"
        ),
        "{model}"
    );
    assert!(model.contains("storage (port : ~LinkStorePort)"), "{model}");
    assert!(model.contains("save (in item : Link)"), "{model}");
    assert!(!model.contains("part def LinkStore {"), "{model}");
    assert!(model.contains("Locked: none."), "{model}");
    // With an element: its full text.
    let Prepared::Answer(store) = tools::prepare(
        &state,
        &Library::built_in_only(),
        READ_MODEL,
        &json!({ "element": "UrlShortener::LinkStore" }),
    ) else {
        panic!()
    };
    assert!(store.contains("part def LinkStore"), "{store}");
    let Prepared::Answer(found) = tools::prepare(
        &state,
        &Library::built_in_only(),
        FIND_ELEMENTS,
        &json!({ "name": "store" }),
    ) else {
        panic!()
    };
    assert!(
        found.contains("UrlShortener::LinkStore (part def)"),
        "{found}"
    );
    assert_eq!(
        tools::prepare(
            &state,
            &Library::built_in_only(),
            ASK_OPERATOR,
            &json!({ "question": "Separate statistics service?", "options": ["Yes", "No"] })
        ),
        Prepared::Question {
            question: "Separate statistics service?".into(),
            options: vec!["Yes".into(), "No".into()]
        }
    );
    assert!(matches!(
        tools::prepare(
            &state,
            &Library::built_in_only(),
            "delete_everything",
            &json!({})
        ),
        Prepared::Invalid(_)
    ));
}

#[test]
fn a_stopped_reply_still_forms_a_valid_exchange() {
    let conversation = Conversation {
        transcript: 0,
        entries: vec![
            Entry::Operator {
                text: "Build it".into(),
            },
            Entry::reply(
                None,
                &[json!({ "type": "tool_use", "id": "t1", "name": "read_model", "input": {} })],
            ),
            Entry::Notice {
                text: "Stopped.".into(),
            },
            Entry::Operator {
                text: "Carry on".into(),
            },
        ],
    };
    // The error result and the Operator's next message form one user message.
    let messages = conversation.api_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[2]["role"], "user");
    assert_eq!(messages[2]["content"][0]["tool_use_id"], "t1");
    assert_eq!(messages[2]["content"][0]["is_error"], true);
    assert_eq!(messages[2]["content"][1]["text"], "Carry on");

    let answered = Conversation {
        transcript: 0,
        entries: vec![
            Entry::reply(
                None,
                &[json!({ "type": "tool_use", "id": "t2", "name": "get_problems", "input": {} })],
            ),
            Entry::ToolResults {
                results: vec![ToolResult {
                    tool_use_id: "t2".into(),
                    content: "No problems.".into(),
                    is_error: false,
                    change: None,
                }],
            },
        ],
    };
    assert_eq!(answered.api_messages().len(), 2);
}

#[test]
fn a_conversation_survives_saving() {
    let folder = std::env::temp_dir().join(format!("agq-conversation-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("conversation.json");
    let deepseek = ModelRef::new(Provider::DeepSeek, "deepseek-flash");
    let conversation = Conversation {
        transcript: 0,
        entries: vec![
            Entry::Operator {
                text: "Hello".into(),
            },
            Entry::Assistant {
                model: Some(deepseek),
                parts: vec![
                    AssistantPart::Reasoning(Reasoning {
                        id: None,
                        parts: vec![ReasoningPart::Text {
                            text: "Greet back.".into(),
                            signature: None,
                        }],
                    }),
                    AssistantPart::Text {
                        text: "Hello.".into(),
                    },
                ],
            },
            Entry::Notice {
                text: "No API key".into(),
            },
        ],
    };
    conversation.save(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"format\":2"), "{text}");
    assert_eq!(Conversation::load(&path).unwrap(), conversation);
    assert_eq!(
        Conversation::load(&folder.join("missing.json")).unwrap(),
        Conversation::default()
    );
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn reasoning_goes_back_only_to_the_model_that_wrote_it() {
    let deepseek = ModelRef::new(Provider::DeepSeek, "deepseek-flash");
    let other = ModelRef::new(Provider::OpenAi, "gpt-5");
    let conversation = Conversation {
        transcript: 0,
        entries: vec![
            Entry::Operator { text: "Hi".into() },
            Entry::Assistant {
                model: Some(deepseek.clone()),
                parts: vec![
                    AssistantPart::Reasoning(Reasoning {
                        id: None,
                        parts: vec![ReasoningPart::Text {
                            text: "Think.".into(),
                            signature: None,
                        }],
                    }),
                    AssistantPart::Text {
                        text: "Hello.".into(),
                    },
                ],
            },
            Entry::Operator {
                text: "Again".into(),
            },
        ],
    };
    let same = conversation.messages_for(Some(&deepseek));
    assert_eq!(same[1]["content"][0]["type"], "reasoning");
    assert_eq!(same[1]["content"][0]["provider"], "deepseek");
    let changed = conversation.messages_for(Some(&other));
    assert_eq!(changed[1]["content"].as_array().unwrap().len(), 1);
    assert_eq!(changed[1]["content"][0]["type"], "text");
}

#[test]
fn a_format_1_file_is_a_read_only_transcript() {
    let format_1 = json!({
        "entries": [
            { "type": "operator", "text": "Build it" },
            { "type": "assistant", "content": [
                { "type": "thinking", "thinking": "Plan.", "signature": "sig" },
                { "type": "text", "text": "Done." }
            ] },
            { "type": "notice", "text": "Stopped." }
        ]
    });
    let conversation = Conversation::parse(&format_1.to_string()).unwrap();
    // Shown: the three entries and a notice that the Assistant starts afresh.
    assert_eq!(conversation.entries.len(), 4);
    assert_eq!(conversation.transcript, 4);
    assert!(matches!(
        &conversation.entries[1],
        Entry::Assistant { model: None, parts } if parts.len() == 2
    ));
    // Never sent to a model.
    assert!(conversation.api_messages().is_empty());
    // Saved again as format 2, it stays a transcript.
    let again = Conversation::parse(&conversation.to_text()).unwrap();
    assert_eq!(again, conversation);
}

#[test]
fn an_unknown_entry_is_kept_shown_and_never_sent_and_an_unknown_format_refused() {
    let text = json!({
        "format": 2,
        "entries": [
            { "type": "operator", "text": "Plan it" },
            { "type": "plan", "steps": ["one", "two"] }
        ]
    })
    .to_string();
    let conversation = Conversation::parse(&text).unwrap();
    assert!(matches!(&conversation.entries[1], Entry::Other(value) if value["type"] == "plan"));
    assert_eq!(conversation.api_messages().len(), 1);
    let again: serde_json::Value = serde_json::from_str(&conversation.to_text()).unwrap();
    assert_eq!(again["entries"][1]["steps"][1], "two");
    assert!(matches!(
        Conversation::parse(&json!({ "format": 3, "entries": [] }).to_string()),
        Err(agq_assistant::conversation::ParseError::LaterFormat(_))
    ));
    assert!(Conversation::parse(&json!({ "format": "2", "entries": [] }).to_string()).is_err());
}

#[test]
fn results_of_a_reply_this_version_cannot_read_are_left_out() {
    let text = json!({
        "format": 2,
        "entries": [
            { "type": "operator", "text": "Build it" },
            { "type": "assistant", "parts": [{ "type": "hologram", "id": "x" }] },
            { "type": "tool_results", "results": [
                { "tool_use_id": "t9", "content": "done", "is_error": false, "change": null }
            ] },
            { "type": "operator", "text": "Carry on" }
        ]
    })
    .to_string();
    let conversation = Conversation::parse(&text).unwrap();
    assert!(matches!(conversation.entries[1], Entry::Other(_)));
    let messages = conversation.api_messages();
    // Only the Operator's two messages, as one user message: no result
    // without its call.
    assert_eq!(messages.len(), 1, "{messages:#?}");
    assert!(
        messages[0]["content"]
            .as_array()
            .unwrap()
            .iter()
            .all(|block| block["type"] == "text")
    );
}

#[test]
fn tool_input_is_checked_against_the_schema() {
    let valid = [
        (READ_MODEL, json!({})),
        (READ_MODEL, json!({ "element": "UrlShortener" })),
        (
            FIND_ELEMENTS,
            json!({ "name": "store", "kind": "part def" }),
        ),
        (
            APPLY_CHANGES,
            json!({ "description": "Add", "operations": [
                { "op": "create", "kind": "attribute", "name": "limit", "value": 5, "end": false }
            ] }),
        ),
        (
            ASK_OPERATOR,
            json!({ "question": "Separate?", "options": ["Yes", "No"] }),
        ),
    ];
    for (tool, input) in valid {
        assert_eq!(tools::check_input(tool, &input), Ok(()), "{tool} {input}");
    }
    let invalid = [
        (READ_MODEL, json!([]), "`input` must be object"),
        (
            READ_MODEL,
            json!({ "elementName": "X" }),
            "no field `elementName`",
        ),
        (
            FIND_ELEMENTS,
            json!({ "kind": "partdef" }),
            "`input.kind` must be one of",
        ),
        (
            APPLY_CHANGES,
            json!({ "description": "Add" }),
            "`input.operations` is required",
        ),
        (
            APPLY_CHANGES,
            json!({ "description": "Add", "operations": [] }),
            "at least 1",
        ),
        (
            APPLY_CHANGES,
            json!({ "description": "Add", "operations": [{ "op": "create", "value": [1] }] }),
            "`input.operations[0].value` must be string or number or boolean",
        ),
        (
            ASK_OPERATOR,
            json!({ "question": "Q", "options": [1] }),
            "`input.options[0]` must be string",
        ),
        ("delete_everything", json!({}), "no tool called"),
    ];
    for (tool, input, expected) in invalid {
        let message = tools::check_input(tool, &input).unwrap_err();
        assert!(message.contains(expected), "{tool} {input}: {message}");
    }
}

#[test]
fn a_long_tool_result_is_cut_with_a_way_to_narrow_it() {
    let long: String = (0..5_000)
        .map(|n| {
            format!(
                "UrlShortener::Part{n} (part def)
"
            )
        })
        .collect();
    let result = agq_assistant::ToolResult::answer(long);
    assert!(result.content.len() <= tools::RESULT_LIMIT + 200);
    assert!(
        result.content.contains("[Cut: "),
        "{}",
        &result.content[result.content.len() - 200..]
    );
    assert!(
        result
            .content
            .contains("read one element by qualified name")
    );
}

// The Library tools (C-49).

fn shop() -> SystemState {
    let text = "package Shop {
    part def System;
    part def Checkout { doc /* Takes payment for an order. */ }
}";
    SystemState::new(parse(&[Source::new("Shop.sysml", text)]), BTreeSet::new())
}

fn answer(prepared: Prepared) -> String {
    match prepared {
        Prepared::Answer(text) => text,
        other => panic!("expected an answer, got {other:?}"),
    }
}

#[test]
fn search_library_lists_blocks_with_what_they_are_and_expose() {
    let state = shop();
    let library = Library::built_in_only();
    let found = answer(tools::prepare(
        &state,
        &library,
        SEARCH_LIBRARY,
        &json!({ "query": "cache" }),
    ));
    assert!(found.contains("building blocks for \"cache\":"), "{found}");
    assert!(
        found.contains("- `built-in:Library::Storage::Cache` — part def, Built-in (Storage): Answers repeated reads"),
        "{found}"
    );
    assert!(
        found.contains("Ports: access : RequestPort, backend : ~RequestPort."),
        "{found}"
    );
    assert!(
        found.contains("`built-in:Library::Storage::CachedStore` — part def, composite"),
        "{found}"
    );
    // The project's own definitions are blocks too.
    let own = answer(tools::prepare(
        &state,
        &library,
        SEARCH_LIBRARY,
        &json!({ "query": "payment", "scope": "project" }),
    ));
    assert!(own.contains("`project:Shop::Checkout`"), "{own}");
    // Kinds narrow it; nothing found says what to do.
    let ports = answer(tools::prepare(
        &state,
        &library,
        SEARCH_LIBRARY,
        &json!({ "kind": "port def" }),
    ));
    assert!(
        ports.starts_with("2 building blocks in the Library:"),
        "{ports}"
    );
    let none = answer(tools::prepare(
        &state,
        &library,
        SEARCH_LIBRARY,
        &json!({ "query": "spaceship" }),
    ));
    assert!(none.contains("model the concept in the project"), "{none}");
}

#[test]
fn read_library_block_describes_a_block_in_words() {
    let state = shop();
    let text = answer(tools::prepare(
        &state,
        &Library::built_in_only(),
        READ_LIBRARY_BLOCK,
        &json!({ "block": "CachedStore" }),
    ));
    assert!(text.contains("Purpose: A store with a cache"), "{text}");
    assert!(
        text.contains("Inner attributes: cache.ttlSeconds"),
        "{text}"
    );
    assert!(!text.contains('{'), "no SysML text: {text}");
    let Prepared::Invalid(message) = tools::prepare(
        &state,
        &Library::built_in_only(),
        READ_LIBRARY_BLOCK,
        &json!({ "block": "Spaceship" }),
    ) else {
        panic!("unknown block")
    };
    assert!(message.contains("search_library"), "{message}");
}

#[test]
fn use_library_block_is_one_undoable_change_through_the_system_state() {
    let mut state = shop();
    let library = Library::built_in_only();
    let prepared = tools::prepare(
        &state,
        &library,
        USE_LIBRARY_BLOCK,
        &json!({
            "block": "built-in:Library::Storage::CachedStore",
            "parent": "Shop::System",
            "name": "sessions",
            "values": {}
        }),
    );
    let first = change(prepared);
    assert_eq!(first.actor, Actor::Assistant);
    assert_eq!(
        first.description,
        "Add sessions : CachedStore from the Library"
    );
    let event = state.apply(first).expect("applies as one change");
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let result = tools::describe_event(&state, &event);
    assert!(
        result.contains("Library::Storage::CachedStore (part def)"),
        "{result}"
    );
    assert!(result.contains("Shop::System::sessions (part)"), "{result}");
    assert!(result.contains("member(s) inside them"), "{result}");
    assert!(!result.contains("(doc)"), "{result}");
    state.undo().unwrap();
    assert!(state.tree().find("Library").is_none());
    // Values and a connection in the same call.
    let prepared = tools::prepare(
        &state,
        &library,
        USE_LIBRARY_BLOCK,
        &json!({ "block": "Gateway", "parent": "Shop::System", "name": "front" }),
    );
    state.apply(change(prepared)).unwrap();
    let prepared = tools::prepare(
        &state,
        &library,
        USE_LIBRARY_BLOCK,
        &json!({
            "block": "Cache",
            "parent": "Shop::System",
            "values": { "ttlSeconds": 60 },
            "connect_to": "front.backend"
        }),
    );
    let cache = change(prepared);
    assert!(
        cache.description.ends_with("connected to front.backend"),
        "{}",
        cache.description
    );
    state.apply(cache).unwrap();
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text = &print(state.tree())[0].text;
    assert!(text.contains("attribute :>> ttlSeconds = 60;"), "{text}");
    assert!(
        text.contains("interface connect front.backend to cache.access;"),
        "{text}"
    );
}

#[test]
fn use_library_block_reports_conflicts_and_never_overwrites() {
    let text = "package Shop { part def System; }
package Library { package Storage { part def Cache { doc /* Ours. */ } } }";
    let state = SystemState::new(parse(&[Source::new("Shop.sysml", text)]), BTreeSet::new());
    let library = Library::built_in_only();
    let Prepared::Invalid(message) = tools::prepare(
        &state,
        &library,
        USE_LIBRARY_BLOCK,
        &json!({ "block": "built-in:Library::Storage::Cache", "parent": "Shop::System" }),
    ) else {
        panic!("a conflict is reported")
    };
    assert!(
        message.contains("already exists with different content"),
        "{message}"
    );
    assert!(message.contains("if_exists"), "{message}");
    let renamed = change(tools::prepare(
        &state,
        &library,
        USE_LIBRARY_BLOCK,
        &json!({ "block": "built-in:Library::Storage::Cache", "parent": "Shop::System",
                 "if_exists": "copy_renamed" }),
    ));
    let mut state = state;
    state.apply(renamed).unwrap();
    assert!(state.tree().find("Library::Storage::Cache2").is_some());
    assert!(print(state.tree())[0].text.contains("Ours."));
}

#[test]
fn save_to_library_waits_for_the_operator() {
    let state = shop();
    let prepared = tools::prepare(
        &state,
        &Library::built_in_only(),
        SAVE_TO_LIBRARY,
        &json!({ "definition": "Shop::Checkout", "category": "Payments" }),
    );
    let Prepared::SaveToLibrary {
        plan,
        question,
        saved,
    } = prepared
    else {
        panic!("a confirmation, got {prepared:?}")
    };
    assert!(
        question.contains("Save `Shop::Checkout` to My Library as `Library::Payments::Checkout`?"),
        "{question}"
    );
    assert!(
        saved.contains("mine:Library::Payments::Checkout"),
        "{saved}"
    );
    assert_eq!(plan.added, ["Library::Payments::Checkout"]);
}

#[test]
fn library_tool_inputs_are_checked_against_their_schemas() {
    assert!(
        tools::check_input(USE_LIBRARY_BLOCK, &json!({ "block": "Cache" })).is_err(),
        "parent is required"
    );
    let error = tools::check_input(
        USE_LIBRARY_BLOCK,
        &json!({ "block": "Cache", "parent": "Shop::System", "if_exists": "overwrite" }),
    )
    .unwrap_err();
    assert!(error.contains("if_exists"), "{error}");
    assert!(
        tools::check_input(
            USE_LIBRARY_BLOCK,
            &json!({ "block": "Cache", "parent": "P", "values": 5 })
        )
        .is_err()
    );
    assert!(tools::check_input(SEARCH_LIBRARY, &json!({ "scope": "cloud" })).is_err());
    assert!(tools::check_input(SEARCH_LIBRARY, &json!({ "query": "cache", "extra": 1 })).is_err());
    assert!(
        tools::check_input(
            SAVE_TO_LIBRARY,
            &json!({ "definition": "Shop::X", "replace": "yes" })
        )
        .is_err()
    );
    assert!(tools::check_input(READ_LIBRARY_BLOCK, &json!({})).is_err());
    assert!(
        tools::check_input(
            SEARCH_LIBRARY,
            &json!({ "query": "cache", "fits_port": "Shop::System::front.backend" })
        )
        .is_ok()
    );
}
