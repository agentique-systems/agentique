//! The Conversation driven with a scripted model (no network), on a real
//! project: tool calls change the System State through the Studio's apply
//! path, locks ask, questions wait, stop and undo, retry and edit, and the
//! conversation is kept per project.
use super::*;
use crate::edit::app_tests::{Folder, studio};
use crate::studio::{Studio, System};
use agq_assistant::Reply;
use agq_system_state::Operation;
use serde_json::{Value, json};
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

fn create(name: &str) -> Value {
    json!({ "op": "create", "parent": "P", "kind": "part", "name": name })
}

fn changes(id: &str, description: &str, operations: Vec<Value>) -> Value {
    tool(
        id,
        tools::APPLY_CHANGES,
        json!({ "description": description, "operations": operations }),
    )
}

/// A Studio with project `P` whose Assistant follows the script.
fn assisted(name: &str, replies: Vec<Reply>) -> (Studio, Folder) {
    let (mut app, folder) = studio(name);
    app.conversation.new_runtime = scripted(replies);
    app.conversation.key_missing = None;
    (app, folder)
}

fn say(app: &mut Studio, message: &str) {
    app.conversation.input = message.to_string();
    app.send_message();
}

/// Takes the turn's events until `done` holds.
fn wait(app: &mut Studio, done: impl Fn(&Studio) -> bool) {
    let started = Instant::now();
    while !done(app) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed out; the conversation is {:#?}",
            app.conversation.conversation.entries
        );
        app.poll_conversation();
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn finished(app: &Studio) -> bool {
    !app.conversation.running()
}

fn find(app: &Studio, name: &str) -> Option<ElementId> {
    app.project.as_ref().unwrap().state().tree().find(name)
}

fn result<'a>(app: &'a Studio, id: &str) -> &'a ToolResult {
    &app.conversation.results[id]
}

/// Roles alternate, starting with the user, and every tool call is answered
/// in the next message.
fn assert_valid(app: &Studio) {
    let messages = app.conversation.conversation.api_messages();
    assert_eq!(messages.first().unwrap()["role"], "user");
    for pair in messages.windows(2) {
        assert_ne!(pair[0]["role"], pair[1]["role"], "{messages:#?}");
    }
    for (index, message) in messages.iter().enumerate() {
        for block in message["content"].as_array().unwrap() {
            if block["type"] == "tool_use" {
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

#[test]
fn a_turn_builds_parts_that_appear_on_the_surface_as_cards_with_links() {
    let (mut app, _folder) = assisted(
        "assistant-build",
        vec![
            reply(
                vec![
                    text("I'll read the model."),
                    tool("t1", tools::READ_MODEL, json!({})),
                ],
                "tool_use",
            ),
            reply(
                vec![changes(
                    "t2",
                    "Add the API and the store",
                    vec![create("api"), create("store")],
                )],
                "tool_use",
            ),
            reply(vec![text("Created `P::api` and `store`.")], "end_turn"),
        ],
    );
    say(&mut app, "Build a URL shortener");
    assert!(app.conversation.input.is_empty());
    wait(&mut app, finished);
    let api = find(&app, "P::api").expect("the Assistant created api");
    let store = find(&app, "P::store").expect("and store");
    // On the Surface and highlighted, like an Operator edit; one undo step.
    assert!(app.lookup.node(&app.scene, api).is_some());
    assert!(app.highlights.contains_key(&api));
    let state = app.project.as_ref().unwrap().state();
    assert_eq!(state.undo_description(), Some("Add the API and the store"));
    // The card's result names the new elements for links.
    assert!(!result(&app, "t1").is_error);
    let change = result(&app, "t2").change.clone().unwrap();
    assert_eq!(change.created, vec![api.raw(), store.raw()]);
    assert_eq!(change.problems, 0);
    assert_valid(&app);
    assert_eq!(app.conversation.undoable(state), Some(Undo::Assistant(1)));
    // Element names in the reply are links: clicking one selects it.
    app.selection.clear();
    app.reveal(store);
    assert_eq!(app.inspected_element(), Some(store), "{}", app.status);
}

#[test]
fn a_locked_part_asks_the_operator_and_a_refusal_changes_nothing() {
    let rename = |id: &str| {
        changes(
            id,
            "Rename api to gateway",
            vec![json!({ "op": "rename", "element": "P::api", "name": "gateway" })],
        )
    };
    let (mut app, _folder) = assisted(
        "assistant-lock",
        vec![
            reply(vec![rename("t1")], "tool_use"),
            reply(vec![text("I left `P::api` as it is.")], "end_turn"),
            reply(vec![rename("t2")], "tool_use"),
            reply(vec![text("Renamed.")], "end_turn"),
        ],
    );
    let package = find(&app, "P").unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "api",
        agq_language::Parent::Element(package),
    );
    let api = find(&app, "P::api").unwrap();
    app.operation("Lock api", Operation::Lock { element: api });
    let revision = app.project.as_ref().unwrap().state().revision();

    say(&mut app, "Call the API the gateway");
    wait(&mut app, |app| app.dialog.is_some());
    assert!(matches!(
        &app.dialog,
        Some(crate::edit::Dialog::Confirm { change, locked, .. })
            if change.actor == Actor::Assistant && locked == &vec![api]
    ));
    app.answer(false);
    wait(&mut app, finished);
    assert_eq!(
        find(&app, "P::api"),
        Some(api),
        "a refusal leaves it as it was"
    );
    assert_eq!(app.project.as_ref().unwrap().state().revision(), revision);
    let refused = result(&app, "t1");
    assert!(
        refused.is_error && refused.content.contains("did not allow"),
        "{refused:?}"
    );
    assert_eq!(refused.change.as_ref().unwrap().refused, vec![api.raw()]);
    assert_valid(&app);

    // Confirming applies it, and the lock stays.
    say(&mut app, "Please rename it anyway");
    wait(&mut app, |app| app.dialog.is_some());
    app.answer(true);
    wait(&mut app, finished);
    assert_eq!(find(&app, "P::gateway"), Some(api));
    assert!(app.project.as_ref().unwrap().state().locks().contains(&api));
    assert!(!result(&app, "t2").is_error);
}

#[test]
fn a_question_waits_for_the_operator_and_the_answer_returns_to_the_model() {
    let (mut app, _folder) = assisted(
        "assistant-question",
        vec![
            reply(
                vec![tool(
                    "t1",
                    tools::ASK_OPERATOR,
                    json!({ "question": "Separate statistics service?", "options": ["Separate", "Inside the API"] }),
                )],
                "tool_use",
            ),
            reply(vec![text("Thanks.")], "end_turn"),
            reply(
                vec![tool(
                    "t2",
                    tools::ASK_OPERATOR,
                    json!({ "question": "Cache?" }),
                )],
                "tool_use",
            ),
            reply(vec![text("Noted.")], "end_turn"),
        ],
    );
    say(&mut app, "Add click statistics");
    wait(&mut app, |app| app.conversation.waiting.is_some());
    assert!(app.conversation.running(), "the turn waits for the answer");
    // The visible phase says so, and ends with the turn.
    assert_eq!(app.conversation.phase, Some("Waiting for your answer"));
    // The options are buttons in the conversation.
    let option = match &app.conversation.waiting {
        Some(Waiting {
            kind: WaitingFor::Question { options, .. },
            ..
        }) => options[1].clone(),
        _ => panic!("a question is open"),
    };
    app.answer_question(&option);
    wait(&mut app, finished);
    assert_eq!(result(&app, "t1").content, "Inside the API");
    assert_eq!(app.conversation.phase, None);

    // A message typed while a question is open answers it.
    say(&mut app, "Should it cache?");
    wait(&mut app, |app| app.conversation.waiting.is_some());
    say(&mut app, "No cache for now");
    assert!(app.conversation.input.is_empty());
    wait(&mut app, finished);
    assert_eq!(result(&app, "t2").content, "No cache for now");
    assert_valid(&app);
}

#[test]
fn stop_keeps_the_partial_work_and_undo_removes_the_turns_changes() {
    let (mut app, _folder) = assisted(
        "assistant-stop",
        vec![
            reply(
                vec![changes("t1", "Add api", vec![create("api")])],
                "tool_use",
            ),
            reply(
                vec![changes("t2", "Add store", vec![create("store")])],
                "tool_use",
            ),
            reply(
                vec![
                    text("One question first."),
                    tool(
                        "t3",
                        tools::ASK_OPERATOR,
                        json!({ "question": "Statistics?" }),
                    ),
                    changes("t4", "Add stats", vec![create("stats")]),
                ],
                "tool_use",
            ),
        ],
    );
    say(&mut app, "Build it");
    wait(&mut app, |app| app.conversation.waiting.is_some());
    app.stop_assistant();
    assert!(
        app.conversation.waiting.is_none(),
        "the question is closed at once"
    );
    wait(&mut app, finished);
    assert!(find(&app, "P::api").is_some() && find(&app, "P::store").is_some());
    assert!(
        find(&app, "P::stats").is_none(),
        "nothing runs after a stop"
    );
    assert!(result(&app, "t3").content.starts_with("Not run"));
    assert!(matches!(
        app.conversation.conversation.entries.last(),
        Some(Entry::Notice { text }) if text.starts_with("Stopped")
    ));
    assert_valid(&app);

    let state = app.project.as_ref().unwrap().state();
    assert_eq!(app.conversation.undoable(state), Some(Undo::Assistant(2)));
    // The list follows new content to its end on the next frame.
    app.undo_assistant_changes();
    assert!(find(&app, "P::api").is_none() && find(&app, "P::store").is_none());
    let state = app.project.as_ref().unwrap().state();
    assert_eq!(app.conversation.undoable(state), None);
    // Undone work can be redone like any undo.
    app.execute(crate::commands::CommandId::Redo);
    assert!(find(&app, "P::api").is_some() && find(&app, "P::store").is_none());
}

#[test]
fn retry_and_edit_and_resend_keep_the_conversation_valid() {
    // The script is empty: the request fails, as when the network is down.
    let (mut app, _folder) = assisted("assistant-retry", Vec::new());
    say(&mut app, "Build it");
    wait(&mut app, finished);
    assert!(app.conversation.can_retry());
    assert!(matches!(
        app.conversation.conversation.entries.as_slice(),
        [Entry::Operator { text }, Entry::Notice { .. }] if text == "Build it"
    ));

    app.conversation.new_runtime = scripted(vec![reply(vec![text("Done.")], "end_turn")]);
    app.retry();
    wait(&mut app, finished);
    assert!(matches!(
        app.conversation.conversation.entries.as_slice(),
        [Entry::Operator { .. }, Entry::Assistant { .. }]
    ));
    assert!(!app.conversation.can_retry());
    assert_valid(&app);

    app.conversation.input = "a draft".into();
    app.edit_last_message();
    assert_eq!(app.conversation.input, "Build it");
    app.conversation.input = "Build it smaller".into();
    app.conversation.new_runtime = scripted(vec![
        reply(
            vec![changes("t1", "Add api", vec![create("api")])],
            "tool_use",
        ),
        reply(vec![text("Smaller.")], "end_turn"),
    ]);
    app.send_message();
    assert_eq!(app.conversation.input, "a draft", "the draft comes back");
    wait(&mut app, finished);
    let entries = &app.conversation.conversation.entries;
    assert!(matches!(&entries[0], Entry::Operator { text } if text == "Build it smaller"));
    assert_eq!(entries.len(), 4, "{entries:#?}");
    assert_valid(&app);
}

#[test]
fn the_conversation_is_kept_per_project_and_survives_a_restart() {
    let (mut app, folder) = assisted(
        "assistant-saved",
        vec![reply(vec![text("Hello.")], "end_turn")],
    );
    say(&mut app, "Hi");
    wait(&mut app, finished);
    let project = app.project.as_ref().unwrap().folder().to_path_buf();
    let saved = app.conversation.conversation.clone();
    assert_eq!(saved.entries.len(), 2);
    let path = conversation_path(&app.session_path, &project);
    assert!(
        path.starts_with(folder.0.join("projects")),
        "{}",
        path.display()
    );
    assert!(!path.starts_with(&project), "never in the project folder");

    // Another project has its own conversation.
    app.create_project(&folder.0.join("Q"), "Q");
    assert!(app.conversation.conversation.entries.is_empty());
    app.open_project(&project);
    assert_eq!(app.conversation.conversation, saved);

    // A new Studio (a restart) shows it again.
    let session = app.session_path.clone();
    drop(app);
    let args = <crate::Args as clap::Parser>::parse_from([
        "studio",
        "--no-restore",
        "--session",
        session.to_str().unwrap(),
    ]);
    let mut again = Studio::new(
        args,
        System {
            dark: true,
            reduced_motion: false,
        },
    );
    again.open_project(&project);
    assert_eq!(again.conversation.conversation, saved);
    again.new_conversation();
    assert!(Conversation::load(&path).unwrap().entries.is_empty());
}

#[test]
fn a_conversation_from_stage_4_is_shown_as_a_transcript_and_never_sent() {
    let (mut app, folder) = assisted(
        "assistant-format-1",
        vec![reply(vec![text("Starting afresh.")], "end_turn")],
    );
    let project = app.project.as_ref().unwrap().folder().to_path_buf();
    // Where Stages 3 and 4 kept it, in format 1.
    let name = conversation_path(&app.session_path, &project)
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let earlier = folder.0.join("conversations").join(format!("{name}.json"));
    std::fs::create_dir_all(earlier.parent().unwrap()).unwrap();
    let format_1 = json!({ "entries": [
        { "type": "operator", "text": "Build the store" },
        { "type": "assistant", "content": [text("Built it.")] }
    ] })
    .to_string();
    std::fs::write(&earlier, &format_1).unwrap();
    app.open_project(&project);
    let conversation = &app.conversation.conversation;
    assert_eq!(conversation.transcript, 3, "{:#?}", conversation.entries);
    assert!(conversation.api_messages().is_empty());
    // The transcript's message cannot be edited and sent again.
    assert_eq!(app.conversation.last_operator(), None);

    say(&mut app, "Carry on");
    wait(&mut app, finished);
    let messages = app.conversation.conversation.api_messages();
    assert_eq!(messages.len(), 2, "only the new exchange: {messages:#?}");
    assert_eq!(messages[0]["content"][0]["text"], "Carry on");
    // Saved in the new place, as format 2; the earlier file is left as it was.
    let saved = std::fs::read_to_string(conversation_path(&app.session_path, &project)).unwrap();
    assert!(saved.contains("\"format\":2"), "{saved}");
    assert_eq!(std::fs::read_to_string(&earlier).unwrap(), format_1);
}

#[test]
fn a_later_versions_conversation_is_left_as_it_is() {
    let (mut app, _folder) = assisted(
        "assistant-later-format",
        vec![reply(vec![text("Hello.")], "end_turn")],
    );
    let project = app.project.as_ref().unwrap().folder().to_path_buf();
    let path = conversation_path(&app.session_path, &project);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let later =
        json!({ "format": 3, "entries": [{ "type": "operator", "text": "Hi" }] }).to_string();
    std::fs::write(&path, &later).unwrap();
    app.open_project(&project);
    assert!(app.conversation.read_error.is_some());
    assert!(app.conversation.conversation.entries.is_empty());
    // Working on does not write over it, and it is not renamed.
    say(&mut app, "Hi");
    wait(&mut app, finished);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), later);
    assert!(!path.with_extension("unreadable.json").exists());
}

#[test]
fn insert_selection_puts_the_selected_names_into_the_message() {
    let (mut app, _folder) = assisted("assistant-insert", Vec::new());
    let package = find(&app, "P").unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "api",
        agq_language::Parent::Element(package),
    );
    app.conversation.input = "Split".into();
    app.execute(crate::commands::CommandId::InsertSelection);
    assert_eq!(app.conversation.input, "Split `P::api` ");
    assert!(app.conversation.focus_input);
    app.selection.clear();
    app.inspected = None;
    app.execute(crate::commands::CommandId::InsertSelection);
    assert_eq!(
        app.conversation.input, "Split `P::api` ",
        "nothing selected"
    );
}

#[test]
fn stop_carries_out_no_tool_call_after_it() {
    let (mut app, _folder) = assisted(
        "assistant-stop-queued",
        vec![
            reply(
                vec![changes("t1", "Add api", vec![create("api")])],
                "tool_use",
            ),
            reply(
                vec![
                    changes("t2", "Add store", vec![create("store")]),
                    changes("t3", "Add stats", vec![create("stats")]),
                ],
                "tool_use",
            ),
        ],
    );
    say(&mut app, "Build it");
    wait(&mut app, |app| find(app, "P::api").is_some());
    // The turn asks for the next change while the Operator presses Stop,
    // before the Studio has taken that call.
    std::thread::sleep(Duration::from_millis(150));
    app.stop_assistant();
    wait(&mut app, finished);
    assert!(find(&app, "P::store").is_none(), "no change after Stop");
    assert!(find(&app, "P::stats").is_none());
    for id in ["t2", "t3"] {
        let card = result(&app, id);
        assert!(
            card.is_error && card.content.starts_with("Not run"),
            "{card:?}"
        );
    }
    assert!(!result(&app, "t1").is_error);
    assert_valid(&app);
    let state = app.project.as_ref().unwrap().state();
    assert_eq!(app.conversation.undoable(state), Some(Undo::Assistant(1)));
}

#[test]
fn a_question_left_open_by_a_finished_turn_keeps_the_typed_answer() {
    let (mut app, _folder) = assisted(
        "assistant-gone",
        vec![reply(
            vec![tool(
                "t1",
                tools::ASK_OPERATOR,
                json!({ "question": "Cache?" }),
            )],
            "tool_use",
        )],
    );
    say(&mut app, "Build it");
    wait(&mut app, |app| app.conversation.waiting.is_some());
    // The turn ends (here: stopped) before the Studio hears of it.
    app.conversation.turn.as_ref().unwrap().stop();
    std::thread::sleep(Duration::from_millis(200));
    say(&mut app, "No cache");
    assert_eq!(
        app.conversation.input, "No cache",
        "an answer nobody receives is not lost"
    );
    wait(&mut app, finished);
    assert!(app.conversation.waiting.is_none(), "nothing waits any more");
}

#[test]
fn undo_says_so_when_the_operator_also_changed_the_model_during_the_turn() {
    let (mut app, _folder) = assisted(
        "assistant-undo-mixed",
        vec![
            reply(
                vec![changes("t1", "Add api", vec![create("api")])],
                "tool_use",
            ),
            reply(
                vec![tool(
                    "t2",
                    tools::ASK_OPERATOR,
                    json!({ "question": "Anything else?" }),
                )],
                "tool_use",
            ),
            reply(vec![text("Done.")], "end_turn"),
        ],
    );
    say(&mut app, "Build it");
    wait(&mut app, |app| app.conversation.waiting.is_some());
    // While the question waits, the Operator adds a part by hand.
    let package = find(&app, "P").unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "cache",
        agq_language::Parent::Element(package),
    );
    say(&mut app, "No");
    wait(&mut app, finished);
    let state = app.project.as_ref().unwrap().state();
    assert_eq!(app.conversation.undoable(state), Some(Undo::All(2)));
    let label = "Undo all changes since the Assistant started";
    assert_eq!(label, "Undo all changes since the Assistant started");
    app.undo_assistant_changes();
    assert!(find(&app, "P::api").is_none() && find(&app, "P::cache").is_none());
    assert_eq!(app.status, "Undid 2 changes since the Assistant started");
}

#[test]
fn a_change_that_waited_for_an_edited_dialog_goes_back_to_the_model() {
    let (mut app, _folder) = assisted(
        "assistant-stale",
        vec![
            reply(
                vec![changes("t1", "Add api", vec![create("api")])],
                "tool_use",
            ),
            reply(vec![text("I'll read the model again.")], "end_turn"),
        ],
    );
    // The Operator has a dialog open; the change waits for it.
    app.dialog = Some(crate::edit::Dialog::Checkpoint {
        message: String::new(),
    });
    say(&mut app, "Build it");
    wait(&mut app, |app| {
        matches!(
            app.conversation.waiting.as_ref().map(|w| &w.kind),
            Some(WaitingFor::Dialog(_))
        )
    });
    // Closing it after an edit: the change was prepared for the model before.
    app.dialog = None;
    let package = find(&app, "P").unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "cache",
        agq_language::Parent::Element(package),
    );
    wait(&mut app, finished);
    assert!(find(&app, "P::api").is_none(), "not applied over the edit");
    assert!(find(&app, "P::cache").is_some());
    let stale = result(&app, "t1");
    assert!(
        stale.is_error && stale.content.contains("model changed"),
        "{stale:?}"
    );
}

// The Library (C-49): the Assistant searches, reads and uses building blocks
// through the Studio's one change path, and saves to My Library only after
// the Operator says so.

#[test]
fn the_assistant_uses_a_library_block_as_one_visible_undoable_change() {
    let (mut app, _folder) = assisted(
        "assistant-library",
        vec![
            reply(
                vec![
                    text("Let me check the Library first."),
                    tool(
                        "t1",
                        tools::SEARCH_LIBRARY,
                        json!({ "query": "cached store" }),
                    ),
                ],
                "tool_use",
            ),
            reply(
                vec![tool(
                    "t2",
                    tools::READ_LIBRARY_BLOCK,
                    json!({ "block": "built-in:Library::Storage::CachedStore" }),
                )],
                "tool_use",
            ),
            reply(
                vec![changes(
                    "t3",
                    "Add the system",
                    vec![
                        json!({ "op": "create", "parent": "P", "kind": "part def", "name": "System" }),
                    ],
                )],
                "tool_use",
            ),
            reply(
                vec![tool(
                    "t4",
                    tools::USE_LIBRARY_BLOCK,
                    json!({ "block": "built-in:Library::Storage::CachedStore", "parent": "P::System",
                             "name": "sessions", "values": { "cache.ttlSeconds": 60 } }),
                )],
                "tool_use",
            ),
            reply(
                vec![text(
                    "`built-in:Library::Storage::CachedStore` fits: I used it as `P::System::sessions`.",
                )],
                "end_turn",
            ),
        ],
    );
    say(
        &mut app,
        "Keep sessions in a store with a cache in front of it.",
    );
    wait(&mut app, finished);
    assert!(
        result(&app, "t1").content.contains("CachedStore"),
        "{}",
        result(&app, "t1").content
    );
    assert!(
        result(&app, "t2")
            .content
            .contains("Purpose: A store with a cache")
    );
    let sessions = find(&app, "P::System::sessions").expect("the block was used");
    let definition = find(&app, "Library::Storage::CachedStore").unwrap();
    let state = app.project.as_ref().unwrap().state();
    assert_eq!(
        state.tree()[sessions].typed_by[0].target(),
        Some(definition)
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    assert_eq!(
        state.undo_description(),
        Some("Add sessions : CachedStore from the Library")
    );
    // On the Surface like any edit, and highlighted as the Assistant's.
    assert!(app.lookup.node(&app.scene, sessions).is_some());
    assert_eq!(
        app.highlights[&sessions].1,
        agq_system_state::Actor::Assistant
    );
    let change = result(&app, "t4").change.clone().unwrap();
    assert!(change.created.contains(&sessions.raw()));
    assert_valid(&app);
    // "Undo the Assistant's changes" takes it all back.
    assert_eq!(app.conversation.undoable(state), Some(Undo::Assistant(2)));
    app.undo_assistant_changes();
    assert!(find(&app, "P::System").is_none());
    assert!(find(&app, "Library").is_none());
}

#[test]
fn saving_to_my_library_waits_for_the_operator() {
    let save = |id: &str| {
        tool(
            id,
            tools::SAVE_TO_LIBRARY,
            json!({ "definition": "P::Checkout", "category": "Payments" }),
        )
    };
    let (mut app, _folder) = assisted(
        "assistant-save",
        vec![
            reply(
                vec![changes(
                    "t0",
                    "Add checkout",
                    vec![
                        json!({ "op": "create", "parent": "P", "kind": "part def", "name": "Checkout" }),
                    ],
                )],
                "tool_use",
            ),
            reply(vec![save("t1")], "tool_use"),
            reply(vec![text("Not saved, as you chose.")], "end_turn"),
            reply(vec![save("t2")], "tool_use"),
            reply(vec![text("Saved.")], "end_turn"),
        ],
    );
    say(&mut app, "Add a checkout and save it to My Library.");
    wait(&mut app, |app| {
        matches!(
            app.conversation.waiting,
            Some(Waiting {
                kind: WaitingFor::SaveToLibrary { .. },
                ..
            })
        )
    });
    // Nothing is saved while the Operator decides.
    let mine = app.library.source.mine_path().unwrap().to_path_buf();
    assert!(!mine.exists());
    assert!(app.answer_question("Don't save"));
    wait(&mut app, finished);
    assert!(!mine.exists());
    assert!(result(&app, "t1").content.starts_with("Not saved"));
    say(&mut app, "Save it after all.");
    wait(&mut app, |app| app.conversation.waiting.is_some());
    assert!(app.answer_question(crate::conversation::SAVE_OPTIONS[0]));
    wait(&mut app, finished);
    assert!(
        std::fs::read_to_string(&mine)
            .unwrap()
            .contains("part def Checkout")
    );
    assert!(
        result(&app, "t2")
            .content
            .contains("mine:Library::Payments::Checkout")
    );
    assert_valid(&app);
}
