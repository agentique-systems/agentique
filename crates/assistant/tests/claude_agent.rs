//! The Claude Agent runtime's side of protocol 2 (ROADMAP §4.7, W10.3, C-53),
//! against a scripted stand-in companion (`fixtures/fake-companion.mjs`, no
//! SDK, no network, no key): tool calls go through the Studio's executor
//! with their tool use ids and the input check; a refused key, a crash, a
//! protocol mismatch, a stop and an SDK that did not start as configured
//! each end the turn with a plain reason and a well-formed conversation; the
//! session is resumed or handed over; the companion's environment holds the
//! key and nothing else. Needs Node.js on the PATH.

use agq_assistant::claude_agent::{
    ClaudeAgent, Development, Endpoint, Gate, Installation, Node, RUNTIME, Undecided,
};
use agq_assistant::conversation::{Conversation, Entry, ToolResult};
use agq_assistant::policy::{Permissions, Place, Policy};
use agq_assistant::runtime::{Runtime, checked};
use agq_assistant::turn::{ToolCall, Toolset, TurnEvent};
use agq_assistant::{Activity, TaskEvent};
use agq_providers::{AssistantPart, Secret};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn node() -> Option<Node> {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    let path = std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|folder| folder.join(name))
        .find(|candidate| candidate.is_file())?;
    Some(Node {
        path,
        version: "test".into(),
    })
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-companion.mjs")
}

fn agent(data: &Path, script: PathBuf) -> Option<ClaudeAgent> {
    let mut agent = ClaudeAgent::new(
        node()?,
        Installation {
            root: data.join("runtime"),
        },
        data.join("agent"),
        Some("claude-opus-5-5".into()),
        None,
        Secret::new("sk-test-key"),
    );
    agent.script = Some(script);
    Some(agent)
}

fn asked(text: &str) -> Conversation {
    Conversation {
        entries: vec![Entry::Operator { text: text.into() }],
        transcript: 0,
    }
}

struct Turn {
    conversation: Conversation,
    events: Vec<TurnEvent>,
    calls: Vec<ToolCall>,
}

/// Runs one turn; `stop_on_call` stops it as soon as a call arrives.
fn run(agent: &mut ClaudeAgent, conversation: Conversation, stop_on_call: bool) -> Turn {
    let mut conversation = conversation;
    let toolset = Toolset::assistant();
    let stop = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    let flag = stop.clone();
    let mut executor = move |call: &ToolCall| {
        seen.lock().unwrap().push(call.clone());
        if stop_on_call {
            flag.store(true, Ordering::SeqCst);
        }
        ToolResult::answer("the outline")
    };
    let mut execute = checked(&toolset.definitions, &mut executor);
    let mut events = Vec::new();
    agent.run(
        &mut conversation,
        &toolset,
        40,
        &mut execute,
        &mut |event| events.push(event),
        &stop,
    );
    let calls = calls.lock().unwrap().clone();
    Turn {
        conversation,
        events,
        calls,
    }
}

fn notices(conversation: &Conversation) -> Vec<String> {
    conversation
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Notice { text } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn texts(conversation: &Conversation) -> Vec<String> {
    conversation
        .entries
        .iter()
        .flat_map(|e| match e {
            Entry::Assistant { parts, .. } => parts
                .iter()
                .filter_map(|p| match p {
                    AssistantPart::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// Every started tool call ends exactly once.
fn calls_balance(events: &[TurnEvent]) {
    let started = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                TurnEvent::Stream(agq_assistant::StreamEvent::ToolCallStarted { .. })
            )
        })
        .count();
    let finished = events
        .iter()
        .filter(|e| matches!(e, TurnEvent::ToolFinished(_)))
        .count();
    assert_eq!(started, finished, "{events:#?}");
}

#[test]
fn a_tool_call_goes_through_the_studio_with_its_tool_use_id() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        eprintln!("Node.js is not on the PATH: skipped");
        return;
    };
    let turn = run(&mut agent, asked("scenario:tool Outline the model."), false);
    assert_eq!(turn.calls.len(), 1);
    assert_eq!(turn.calls[0].id, "toolu_1");
    assert_eq!(turn.calls[0].name, "read_model");
    calls_balance(&turn.events);
    // The conversation: the message, the session, the reply with its call,
    // the call's result, the final reply.
    let kinds: Vec<&str> = turn
        .conversation
        .entries
        .iter()
        .map(|e| match e {
            Entry::Operator { .. } => "operator",
            Entry::Session { .. } => "session",
            Entry::Assistant { .. } => "assistant",
            Entry::ToolResults { .. } => "results",
            Entry::Notice { .. } => "notice",
            Entry::Other(_) => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        ["operator", "session", "assistant", "results", "assistant"],
        "{:#?}",
        turn.conversation.entries
    );
    match &turn.conversation.entries[1] {
        Entry::Session {
            runtime, id, event, ..
        } => {
            assert_eq!(runtime, RUNTIME);
            assert_eq!(id, "session-1");
            assert_eq!(event, "started");
        }
        other => panic!("{other:?}"),
    }
    assert!(
        texts(&turn.conversation)
            .iter()
            .any(|t| t == "The Studio said: the outline")
    );
    // The reply names the tool without the MCP prefix.
    assert!(matches!(
        &turn.conversation.entries[2],
        Entry::Assistant { parts, .. } if parts.iter().any(|p| matches!(p, AssistantPart::ToolCall { name, .. } if name == "read_model"))
    ));
    assert!(notices(&turn.conversation).is_empty());
    assert!(agent.effective.as_ref().unwrap().problems(None).is_empty());
}

#[test]
fn a_call_that_does_not_fit_its_schema_never_reaches_the_studio() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:unknown Change it."), false);
    assert!(turn.calls.is_empty(), "{:?}", turn.calls);
    let answer = texts(&turn.conversation).join(" ");
    assert!(answer.contains("Answer: true Not run:"), "{answer}");
    calls_balance(&turn.events);
}

#[test]
fn a_refused_key_ends_the_turn_with_where_to_fix_it() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:auth Hello."), false);
    let notices = notices(&turn.conversation);
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(
        notices[0].contains("Anthropic refused the API key"),
        "{notices:?}"
    );
    assert!(notices[0].contains("Settings"), "{notices:?}");
}

#[test]
fn a_crash_keeps_what_was_shown_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:crash Hello."), false);
    assert!(
        texts(&turn.conversation)
            .iter()
            .any(|t| t == "Half a thought")
    );
    let notices = notices(&turn.conversation);
    assert!(
        notices.iter().any(|n| n.contains("stopped unexpectedly")),
        "{notices:?}"
    );
}

#[test]
fn another_protocol_is_refused_before_anything_runs() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("old.mjs");
    std::fs::write(
        &script,
        "process.stdout.write(JSON.stringify({type:'ready',protocol:1})+'\\n'); setTimeout(()=>process.exit(0),2000);",
    )
    .unwrap();
    let Some(mut agent) = agent(dir.path(), script) else {
        return;
    };
    let turn = run(&mut agent, asked("Hello."), false);
    let notices = notices(&turn.conversation);
    assert!(
        notices.iter().any(|n| n.contains("another protocol")),
        "{notices:?}"
    );
    assert!(turn.calls.is_empty());
}

#[test]
fn a_stop_ends_the_turn_and_its_waiting_call_is_not_run() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:interrupt Work."), true);
    calls_balance(&turn.events);
    let notices = notices(&turn.conversation);
    assert!(
        notices
            .iter()
            .any(|n| n.starts_with("Stopped by the Operator")),
        "{notices:?}"
    );
    // The conversation stays well formed: the call has a result.
    assert!(
        turn.conversation
            .entries
            .iter()
            .any(|e| matches!(e, Entry::ToolResults { .. }))
    );
}

#[test]
fn an_sdk_that_did_not_start_as_configured_is_stopped_before_anything_runs() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:policy Hello."), false);
    assert!(turn.calls.is_empty());
    let notices = notices(&turn.conversation);
    assert!(
        notices
            .iter()
            .any(|n| n.contains("did not start as Agentique configures it") && n.contains("`Bash`")),
        "{notices:?}"
    );
}

#[test]
fn a_session_is_resumed_and_otherwise_the_visible_history_is_handed_over() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    // First turn: a new session, nothing to hand over.
    let first = run(&mut agent, asked("scenario:resume Begin."), false);
    assert!(
        texts(&first.conversation)
            .iter()
            .any(|t| t.starts_with("resume=null; continues=false; handover=false"))
    );
    // Second turn in the same conversation: the session is resumed.
    let mut conversation = first.conversation.clone();
    conversation.entries.push(Entry::Operator {
        text: "scenario:resume Go on.".into(),
    });
    let second = run(&mut agent, conversation, false);
    assert!(
        texts(&second.conversation)
            .iter()
            .any(|t| t.starts_with("resume=session-new; continues=true")),
        "{:?}",
        texts(&second.conversation)
    );
    // A turn on another runtime in between: a new session, handed the
    // visible history.
    let mut conversation = second.conversation.clone();
    conversation.entries.push(Entry::Operator {
        text: "On the loop.".into(),
    });
    conversation.entries.push(Entry::Assistant {
        model: None,
        parts: vec![AssistantPart::Text {
            text: "From the loop.".into(),
        }],
    });
    conversation.entries.push(Entry::Operator {
        text: "scenario:resume Back again.".into(),
    });
    let third = run(&mut agent, conversation, false);
    assert!(
        texts(&third.conversation)
            .iter()
            .any(|t| t.starts_with("resume=null; continues=false; handover=true")),
        "{:?}",
        texts(&third.conversation)
    );
}

#[test]
fn the_companion_gets_the_key_and_nothing_else_of_the_environment() {
    // SAFETY of the test: variables set for this process only, to show they
    // do not reach the companion.
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), fixture()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:environment Hello."), false);
    let said = texts(&turn.conversation).join(" ");
    assert!(said.contains("key=sk-test-key"), "{said}");
    // Whatever the Studio's own environment holds (a token, a parent
    // Claude Code session), the companion never sees it.
    assert!(said.contains("token=none"), "{said}");
    assert!(said.contains("parent=none"), "{said}");
}

// --- Protocol 2: development sessions (C-53) ---------------------------------

fn development(dir: &Path) -> Development {
    let project = dir.join("project");
    std::fs::create_dir_all(&project).unwrap();
    Development {
        cwd: project.clone(),
        policy: Policy::development(
            &project,
            &[],
            &[],
            Place::WorkingCopy,
            Permissions {
                commands: true,
                network: false,
                push: false,
                mcp_servers: Vec::new(),
                undecided: Undecided::Ask,
            },
        ),
        setting_sources: vec!["project".into()],
        agents: serde_json::json!({ "reviewer": { "description": "Reviews.", "prompt": "Review." } }),
        preset: true,
    }
}

fn development_agent(dir: &Path) -> Option<ClaudeAgent> {
    let mut agent = agent(dir, fixture())?
        .with_development(development(dir))
        .with_endpoint(Endpoint::deepseek());
    agent.model = Some("deepseek-v4-pro".into());
    Some(agent)
}

/// Runs one turn, answering every call with `answer`.
fn run_answering(
    agent: &mut ClaudeAgent,
    conversation: Conversation,
    answer: impl Fn(&ToolCall) -> ToolResult,
) -> Turn {
    let mut conversation = conversation;
    let toolset = Toolset::assistant();
    let stop = AtomicBool::new(false);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    let mut executor = move |call: &ToolCall| {
        seen.lock().unwrap().push(call.clone());
        answer(call)
    };
    let mut execute = checked(&toolset.definitions, &mut executor);
    let mut events = Vec::new();
    agent.run(
        &mut conversation,
        &toolset,
        40,
        &mut execute,
        &mut |event| events.push(event),
        &stop,
    );
    let calls = calls.lock().unwrap().clone();
    Turn {
        conversation,
        events,
        calls,
    }
}

#[test]
fn a_development_session_carries_its_policy_endpoint_and_an_environment_without_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = development_agent(dir.path()) else {
        return;
    };
    let turn = run(
        &mut agent,
        asked("scenario:development Fix the gap."),
        false,
    );
    let said = texts(&turn.conversation).join(" ");
    for expected in [
        "undecided=ask",
        "protectsModel=true",
        "preset=true",
        "endpoint=https://api.deepseek.com/anthropic",
        "fast=deepseek-flash",
        "sources=project",
        "cwdIsProject=true",
        "agents=reviewer",
        "key=sk-test-key",
        "path=true",
        "manifest=kept",
        "parent=none",
    ] {
        assert!(said.contains(expected), "{expected} in {said}");
    }
    // The SDK's own tools are what a development session is for.
    assert!(
        notices(&turn.conversation)
            .iter()
            .all(|n| !n.contains("did not start as Agentique configures it")),
        "{:?}",
        notices(&turn.conversation)
    );
    assert_eq!(
        agent.label(),
        "Claude Agent · deepseek-v4-pro (through DeepSeek)"
    );
    // Costed at DeepSeek's price, not Claude's.
    assert_eq!(
        agent.model().map(|m| m.provider),
        Some(agq_providers::Provider::DeepSeek)
    );
}

#[test]
fn the_sdks_own_tools_subagents_and_compaction_reach_the_conversation() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = development_agent(dir.path()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:builtin Run the tests."), false);
    calls_balance(&turn.events);
    assert!(turn.calls.is_empty(), "the SDK ran its own tool");
    let finished = turn.events.iter().find_map(|e| match e {
        TurnEvent::ToolFinished(result) if result.tool_use_id == "toolu_b1" => Some(result.clone()),
        _ => None,
    });
    let finished = finished.expect("the Bash call ended with its result");
    assert!(finished.is_error);
    assert!(finished.content.contains("1 failed"));
    let tasks: Vec<_> = turn
        .events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::Activity(Activity::Task { event, agent, .. }) => {
                Some((*event, agent.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        tasks,
        vec![
            (TaskEvent::Started, Some("reviewer".into())),
            (TaskEvent::Done, Some("reviewer".into()))
        ]
    );
    assert!(turn.events.iter().any(|e| matches!(
        e,
        TurnEvent::Activity(Activity::Compacted {
            before: 180000,
            after: Some(12000),
            ..
        })
    )));
    assert!(
        notices(&turn.conversation)
            .iter()
            .any(|n| n.contains("summarised its context"))
    );
    // The conversation keeps the call and its result, well formed.
    assert!(turn.conversation.entries.iter().any(|e| matches!(
        e,
        Entry::ToolResults { results } if results.iter().any(|r| r.tool_use_id == "toolu_b1")
    )));
}

#[test]
fn a_call_outside_the_policy_is_asked_of_the_operator_and_the_answer_applies() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = development_agent(dir.path()) else {
        return;
    };
    let allowed = run_answering(
        &mut agent,
        asked("scenario:permission Read my notes."),
        |call| {
            assert_eq!(call.name, "ask_operator");
            assert!(
                call.input["question"]
                    .as_str()
                    .unwrap()
                    .contains("read D:\\notes.txt")
            );
            ToolResult::answer("Allow")
        },
    );
    assert!(
        texts(&allowed.conversation)
            .join(" ")
            .contains("allow=true")
    );
    assert!(
        notices(&allowed.conversation)
            .iter()
            .any(|n| n.starts_with("You allowed the Assistant to read"))
    );
    calls_balance(&allowed.events);
    let refused = run_answering(
        &mut agent,
        asked("scenario:permission Read my notes."),
        |_| ToolResult::answer("Don't allow"),
    );
    let said = texts(&refused.conversation).join(" ");
    assert!(said.contains("allow=false"), "{said}");
    assert!(said.contains("The Operator did not allow it"), "{said}");
}

#[test]
fn queued_messages_and_the_pause_gate_reach_the_running_session() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = development_agent(dir.path()) else {
        return;
    };
    agent.steering.queue("Also update the README.");
    agent.steering.set_gate(Gate::Pause);
    let turn = run(&mut agent, asked("scenario:steer Work."), false);
    let said = texts(&turn.conversation).join(" ");
    assert!(
        said.contains("seen=gate:pause|message:Also update the README."),
        "{said}"
    );
    assert!(turn.events.iter().any(|e| matches!(
        e,
        TurnEvent::Activity(Activity::Paused { tool }) if tool == "Bash"
    )));
    assert_eq!(agent.steering.held_at().as_deref(), Some("Bash"));
}

/// The real companion with the real SDK and Claude Code binary (installed in
/// `claude-agent/node_modules` by `npm ci`), and a key Anthropic refuses:
/// the SDK starts with Agentique's tools only, through Agentique's server
/// alone, in `dontAsk`, and the refused key ends the turn before any model
/// runs (it costs nothing). Skipped when the packages are not installed;
/// needs the network to be refused.
#[test]
fn the_real_sdk_starts_with_agentiques_tools_only_and_a_refused_key_stops_it() {
    let companion = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../claude-agent");
    if !companion
        .join("node_modules/@anthropic-ai/claude-agent-sdk/package.json")
        .is_file()
    {
        eprintln!("The companion's packages are not installed (npm ci in claude-agent): skipped");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = agent(dir.path(), companion.join("src/main.ts")) else {
        return;
    };
    agent.model = None;
    let stop = Arc::new(AtomicBool::new(false));
    let guard = stop.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(120));
        guard.store(true, Ordering::SeqCst);
    });
    let mut conversation = asked("Hello.");
    let toolset = Toolset::assistant();
    let mut executor = |_: &ToolCall| ToolResult::answer("never");
    let mut execute = checked(&toolset.definitions, &mut executor);
    let mut events = Vec::new();
    agent.run(
        &mut conversation,
        &toolset,
        2,
        &mut execute,
        &mut |event| events.push(event),
        &stop,
    );
    let effective = agent
        .effective
        .clone()
        .expect("the SDK reported how it started");
    assert!(effective.problems(None).is_empty(), "{effective:?}");
    let names: Vec<String> = toolset
        .definitions
        .as_array()
        .unwrap()
        .iter()
        .map(|d| format!("mcp__agentique__{}", d["name"].as_str().unwrap()))
        .collect();
    let mut tools = effective.tools.clone();
    tools.sort();
    let mut expected = names.clone();
    expected.sort();
    assert_eq!(tools, expected, "exactly Agentique's tools");
    assert_eq!(effective.permission_mode, "dontAsk");
    let notices = notices(&conversation);
    assert!(
        notices
            .iter()
            .any(|n| n.contains("Anthropic refused the API key")),
        "{notices:?}"
    );
    assert!(
        !stop.load(Ordering::SeqCst),
        "it ended on the refused key, not the guard"
    );
}

/// Gate B (ROADMAP §6.6): a live, authenticated turn on the real SDK. The
/// agent reads the model through Agentique's own tool, answers from what it
/// read, and a second turn continues it (a fork of the first turn's
/// session) and remembers it. Costs a few cents:
/// runs only with `AGQ_LIVE=1`, the companion's packages installed, and an
/// Anthropic key in `ANTHROPIC_API_KEY` or the credential store.
/// `cargo test -p agq-assistant --test claude_agent -- --ignored --nocapture`
#[test]
#[ignore = "live: costs money; set AGQ_LIVE=1 and an Anthropic key"]
fn live_the_sdk_reads_the_model_through_agentique_and_resumes_its_session() {
    if std::env::var("AGQ_LIVE").as_deref() != Ok("1") {
        eprintln!("AGQ_LIVE is not 1: skipped");
        return;
    }
    let companion = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../claude-agent");
    let key = agq_providers::claude_agent_key()
        .expect("the credential store can be read")
        .expect("an Anthropic key");
    let dir = tempfile::tempdir().unwrap();
    let mut agent = ClaudeAgent::new(
        node().expect("Node.js on the PATH"),
        Installation {
            root: dir.path().join("runtime"),
        },
        dir.path().join("agent"),
        Some(std::env::var("AGQ_LIVE_MODEL").unwrap_or_else(|_| "claude-haiku-4-5".into())),
        None,
        key,
    );
    agent.script = Some(companion.join("src/main.ts"));
    let toolset = Toolset::assistant();
    let mut conversation =
        asked("Use read_model once, then tell me in one sentence which part defs the model has.");
    let calls = Arc::new(Mutex::new(Vec::<ToolCall>::new()));
    for turn in 0..2 {
        let stop = AtomicBool::new(false);
        let seen = calls.clone();
        let mut executor = move |call: &ToolCall| {
            seen.lock().unwrap().push(call.clone());
            ToolResult::answer("package Shop\n  part def LinkStore\n  part def HttpApi\n")
        };
        let mut execute = checked(&toolset.definitions, &mut executor);
        let mut events = Vec::new();
        agent.run(
            &mut conversation,
            &toolset,
            6,
            &mut execute,
            &mut |event| events.push(event),
            &stop,
        );
        eprintln!("turn {turn}: {:#?}", conversation.entries);
        assert!(
            notices(&conversation).is_empty(),
            "{:?}",
            notices(&conversation)
        );
        if turn == 0 {
            conversation.entries.push(Entry::Operator {
                text: "Which of those two did you name first? One word.".into(),
            });
        }
    }
    let calls = calls.lock().unwrap();
    assert!(
        calls.iter().any(|c| c.name == "read_model"),
        "the agent read the model through Agentique: {calls:?}"
    );
    let replies = texts(&conversation);
    assert!(replies.join("\n").contains("LinkStore"), "{replies:?}");
    // The second turn remembers the first (it names neither part).
    let last = replies.last().cloned().unwrap_or_default();
    assert!(
        last.contains("LinkStore") || last.contains("HttpApi"),
        "{replies:?}"
    );
    let sessions: Vec<(&str, &str)> = conversation
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Session { id, event, .. } => Some((id.as_str(), event.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(sessions.len(), 2, "{sessions:?}");
    assert_eq!(sessions[1].1, "resumed", "{sessions:?}");
    // Each turn forks the session it continues.
    assert_ne!(sessions[0].0, sessions[1].0, "a fork of the first session");
}

/// W11.2 (C-53): a live development session through Agentique's runtime on
/// DeepSeek's Anthropic-compatible endpoint. In a scratch repository the
/// agent reads and edits a file with the SDK's own tools, is refused a write
/// to a model file (the reason names apply_changes), runs a command, calls an
/// Agentique tool through the Studio's executor, and a second turn resumes
/// its session. Costs a few cents: runs only with `AGQ_LIVE=1`, the
/// companion's packages installed, and a DeepSeek key
/// (`DEEPSEEK_API_KEY` or the credential store).
#[test]
#[ignore = "live: costs money; set AGQ_LIVE=1 and a DeepSeek key"]
fn live_a_development_session_on_deepseek_works_in_the_repository_within_its_policy() {
    if std::env::var("AGQ_LIVE").as_deref() != Ok("1") {
        eprintln!("AGQ_LIVE is not 1: skipped");
        return;
    }
    let companion = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../claude-agent");
    let key = agq_providers::runtime_key(agq_providers::Provider::DeepSeek)
        .expect("the credential store can be read")
        .expect("a DeepSeek key");
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(project.join("model")).unwrap();
    std::fs::write(project.join("notes.txt"), "The build number is 41.\n").unwrap();
    let model = "package Shop {\n    part def Store;\n}\n";
    std::fs::write(project.join("model/Shop.sysml"), model).unwrap();
    let mut development = development(dir.path());
    development.cwd = project.clone();
    let mut agent = ClaudeAgent::new(
        node().expect("Node.js on the PATH"),
        Installation {
            root: dir.path().join("runtime"),
        },
        dir.path().join("agent"),
        Some(std::env::var("AGQ_LIVE_MODEL").unwrap_or_else(|_| "deepseek-v4-pro".into())),
        None,
        key,
    )
    .with_development(development)
    .with_endpoint(Endpoint::deepseek());
    agent.script = Some(companion.join("src/main.ts"));
    let toolset = Toolset::assistant();
    let mut conversation = asked(
        "Do these steps in order, with your tools, and then report briefly: \
         1) Change the number in notes.txt from 41 to 42 with your Edit tool. \
         2) Try to add the line `// checked` at the end of model/Shop.sysml with your Edit tool, and tell me exactly what happened. \
         3) Run `node -e \"console.log(require('fs').readFileSync('notes.txt','utf8').trim())\"` and quote its output. \
         4) Call Agentique's read_model tool once and name the part defs it lists.",
    );
    let calls = Arc::new(Mutex::new(Vec::<ToolCall>::new()));
    let mut all_events = Vec::new();
    for turn in 0..2 {
        let stop = AtomicBool::new(false);
        let seen = calls.clone();
        let mut executor = move |call: &ToolCall| {
            seen.lock().unwrap().push(call.clone());
            ToolResult::answer("package Shop\n  part def Store\n  part def Ledger\n")
        };
        let mut execute = checked(&toolset.definitions, &mut executor);
        let mut events = Vec::new();
        agent.run(
            &mut conversation,
            &toolset,
            30,
            &mut execute,
            &mut |event| events.push(event),
            &stop,
        );
        eprintln!("turn {turn}: {:#?}", conversation.entries);
        let problems: Vec<String> = notices(&conversation)
            .into_iter()
            .filter(|n| !n.starts_with("Subagent") && !n.contains("summarised"))
            .collect();
        assert!(problems.is_empty(), "{problems:?}");
        all_events.extend(events);
        if turn == 0 {
            conversation.entries.push(Entry::Operator {
                text: "What number is in notes.txt now? Answer with the number only, from memory."
                    .into(),
            });
        }
    }
    let effective = agent
        .effective
        .clone()
        .expect("the SDK reported how it started");
    assert!(
        effective.problems(agent.development.as_ref()).is_empty(),
        "{effective:?}"
    );
    assert!(effective.tools.iter().any(|t| t == "Edit"), "{effective:?}");
    // The file tool's edit happened; the model file did not change.
    let notes = std::fs::read_to_string(project.join("notes.txt")).unwrap();
    assert!(notes.contains("42"), "{notes}");
    assert_eq!(
        std::fs::read_to_string(project.join("model/Shop.sysml")).unwrap(),
        model
    );
    // The refusal reached the agent with its reason.
    let refused = all_events.iter().any(|e| {
        matches!(
            e,
            TurnEvent::ToolFinished(r) if r.is_error && r.content.contains("apply_changes")
        )
    });
    assert!(refused, "the model file's refusal names apply_changes");
    // An Agentique tool went through the Studio's executor.
    assert!(calls.lock().unwrap().iter().any(|c| c.name == "read_model"));
    let replies = texts(&conversation).join("\n");
    assert!(replies.contains("Ledger"), "{replies}");
    assert!(
        texts(&conversation)
            .last()
            .is_some_and(|t| t.contains("42")),
        "{replies}"
    );
    let sessions: Vec<&str> = conversation
        .entries
        .iter()
        .filter_map(|e| match e {
            Entry::Session { event, .. } => Some(event.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(sessions, vec!["started", "resumed"]);
}

/// Review of PR #98: the SDK sends a reply's content blocks as separate
/// messages with the reply's id; calls of one reply that run side by side
/// keep their real results instead of being recorded as not run.
#[test]
fn parallel_calls_of_one_reply_keep_their_results() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = development_agent(dir.path()) else {
        return;
    };
    let turn = run(&mut agent, asked("scenario:parallel Read both."), false);
    calls_balance(&turn.events);
    let results: Vec<(String, String, bool)> = turn
        .events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::ToolFinished(r) => {
                Some((r.tool_use_id.clone(), r.content.clone(), r.is_error))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        results,
        vec![
            ("toolu_p1".to_string(), "fn a() {}".to_string(), false),
            ("toolu_p2".to_string(), "fn b() {}".to_string(), false)
        ]
    );
}

/// Second review of PR #98: after a turn's first result the companion may
/// keep it open (queued messages, background work); the pause gate still
/// reaches it.
#[test]
fn the_pause_gate_reaches_a_turn_kept_open_after_its_result() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut agent) = development_agent(dir.path()) else {
        return;
    };
    let steering = agent.steering.clone();
    let pauser = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        steering.set_gate(Gate::Pause);
    });
    let turn = run(&mut agent, asked("scenario:lingering Work."), false);
    pauser.join().unwrap();
    let said = texts(&turn.conversation).join(" ");
    assert!(
        said.contains("gate=pause"),
        "{said} {:?}",
        turn.conversation.entries
    );
}

/// Second review of PR #98: without trusted-local execution the project's
/// hooks are disabled (`disableAllHooks`); Agentique's own policy hook must
/// still hold. Live, on `deepseek-flash` (a fraction of a cent): a file edit
/// works, a model-file edit and a command are refused with their reasons.
#[test]
#[ignore = "live: costs money; set AGQ_LIVE=1 and a DeepSeek key"]
fn live_without_commands_the_policy_hook_still_holds() {
    if std::env::var("AGQ_LIVE").as_deref() != Ok("1") {
        eprintln!("AGQ_LIVE is not 1: skipped");
        return;
    }
    let companion = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../claude-agent");
    let key = agq_providers::runtime_key(agq_providers::Provider::DeepSeek)
        .expect("the credential store can be read")
        .expect("a DeepSeek key");
    let dir = tempfile::tempdir().unwrap();
    let mut development = development(dir.path());
    development.policy.commands = false;
    let project = development.cwd.clone();
    std::fs::create_dir_all(project.join("model")).unwrap();
    std::fs::write(project.join("notes.txt"), "The build number is 41.\n").unwrap();
    let model = "package Shop {\n    part def Store;\n}\n";
    std::fs::write(project.join("model/Shop.sysml"), model).unwrap();
    let mut agent = ClaudeAgent::new(
        node().expect("Node.js on the PATH"),
        Installation {
            root: dir.path().join("runtime"),
        },
        dir.path().join("agent"),
        Some("deepseek-flash".into()),
        None,
        key,
    )
    .with_development(development)
    .with_endpoint(Endpoint::deepseek());
    agent.script = Some(companion.join("src/main.ts"));
    let turn = run_answering(
        &mut agent,
        asked(
            "Do these three steps with your tools, then report what happened in each: \
             1) Change 41 to 42 in notes.txt with your Edit tool. \
             2) Try to add the line `// checked` at the end of model/Shop.sysml with your Edit tool. \
             3) Try to run `echo hi` with your Bash tool.",
        ),
        |_| ToolResult::answer("not used"),
    );
    eprintln!("{:#?}", turn.conversation.entries);
    let notes = std::fs::read_to_string(project.join("notes.txt")).unwrap();
    assert!(notes.contains("42"), "{notes}");
    assert_eq!(
        std::fs::read_to_string(project.join("model/Shop.sysml")).unwrap(),
        model
    );
    let refusals: Vec<String> = turn
        .events
        .iter()
        .filter_map(|e| match e {
            TurnEvent::ToolFinished(r) if r.is_error => Some(r.content.clone()),
            _ => None,
        })
        .collect();
    assert!(
        refusals.iter().any(|r| r.contains("apply_changes")),
        "{refusals:?}"
    );
    assert!(
        refusals.iter().any(|r| r.contains("trusted-local")),
        "{refusals:?}"
    );
}
