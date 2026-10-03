//! Execution identity (C-52): a run's agent calls carry the binding the
//! Studio prepared; the recording key covers it; a recording answers only
//! the binding it was made with, legacy recordings never answer a bound
//! request, keys are checked again when read, results say whether their
//! binding and recordings are still those of the present; live calls get a
//! deadline, failed and stopped calls are counted, and an allowance bounds
//! the calls made.
use agq_language::{Source, Tree, parse};
use agq_simulation::agents::{CallLimits, Evidence, LiveAnswer, LiveModel, Tokens};
use agq_simulation::digest::model_digest;
use agq_simulation::{
    AgentRequest, Answers, Binding, Mode, Outcome, Present, Recording, Recordings, Request,
    RunBinding, RunResult, RunStatus, StopReason, compile, describe_agents, freshness, run,
};
use serde_json::{Value as Json, json};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SHORTENER: &str = include_str!("../../../models/link-screening/UrlShortener.sysml");

fn tree() -> Tree {
    parse(&[Source::new("UrlShortener.sysml", SHORTENER)])
}

fn binding() -> Binding {
    Binding {
        provider: "typesafe".into(),
        model: "jev-1.13.0".into(),
        adapter: "agq-providers jev 2".into(),
        mapping: "choice 1".into(),
        question: json!({"id": "decision", "options": {"allow": "a", "review": "r", "block": "b"}}),
        input: json!({"fields": ["longUrl", "host"]}),
        policy: json!({"confidence": "choice"}),
    }
}

/// The binding for the screening agent of `scenario`, as the Studio
/// prepares it.
fn prepared(tree: &Tree, scenario: &str, binding: Binding) -> RunBinding {
    let id = tree.find(&format!("UrlShortener::{scenario}")).unwrap();
    let program = compile(tree, id).unwrap();
    let agents = describe_agents(&program).unwrap();
    assert_eq!(agents.len(), 1, "{agents:#?}");
    RunBinding {
        agent: agents[0].request.clone(),
        binding,
    }
}

fn run_with(
    tree: &Tree,
    scenario: &str,
    mode: Mode,
    answers: Answers,
    binding: Option<RunBinding>,
    samples: u32,
    allowance: Option<u32>,
) -> RunResult {
    let id = tree.find(&format!("UrlShortener::{scenario}")).unwrap();
    let program = compile(tree, id).unwrap();
    let mut request = Request::new(mode);
    request.samples = samples;
    request.binding = binding;
    request.limits.max_live_calls = allowance;
    run(
        &program,
        model_digest(tree, id),
        &request,
        answers,
        Arc::new(AtomicBool::new(false)),
    )
}

/// What a live client does per call.
#[derive(Clone, Copy)]
enum Act {
    Allow,
    Fail,
    StopAndFail,
    UnknownCost,
}

struct Scripted {
    acts: Vec<Act>,
    calls: AtomicU32,
    seen: Mutex<Vec<(AgentRequest, Duration)>>,
}

impl Scripted {
    fn new(acts: Vec<Act>) -> Arc<Scripted> {
        Arc::new(Scripted {
            acts,
            calls: AtomicU32::new(0),
            seen: Mutex::new(Vec::new()),
        })
    }
}

impl LiveModel for Scripted {
    fn label(&self) -> String {
        "typesafe/jev-1.13.0".into()
    }

    fn answer(
        &self,
        request: &AgentRequest,
        limits: CallLimits,
        cancel: &AtomicBool,
    ) -> LiveAnswer {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) as usize;
        let left = limits.deadline.saturating_duration_since(Instant::now());
        self.seen.lock().unwrap().push((request.clone(), left));
        let act = self.acts.get(n).copied().unwrap_or(Act::Allow);
        let answer = LiveAnswer {
            outcome: Outcome::Answer,
            output: Some(json!({"decision": "allow", "confidence": 0.93})),
            latency_ms: 120,
            cost_usd: Some(0.00001),
            error: None,
            evidence: Some(Evidence {
                model: Some("jev-1.13.0".into()),
                estimates: Some(
                    json!({"choice": "allow", "probabilities": {"allow": 0.93, "review": 0.05, "block": 0.02}, "confidence": 0.93}),
                ),
                usage: Some(Tokens {
                    input: Some(40),
                    output: Some(1),
                }),
                request_id: Some(format!("req_{n}")),
                attempts: 1,
            }),
        };
        match act {
            Act::Allow => answer,
            Act::UnknownCost => LiveAnswer {
                cost_usd: None,
                ..answer
            },
            Act::Fail => LiveAnswer {
                outcome: Outcome::Timeout,
                output: None,
                cost_usd: None,
                error: Some("TypeSafe AI's reply could not be used".into()),
                ..answer
            },
            Act::StopAndFail => {
                cancel.store(true, Ordering::SeqCst);
                LiveAnswer {
                    outcome: Outcome::Timeout,
                    output: None,
                    cost_usd: None,
                    error: Some("Stopped.".into()),
                    ..answer
                }
            }
        }
    }
}

#[test]
fn a_request_without_a_binding_keeps_its_legacy_key() {
    let request = AgentRequest {
        agent: "P::A".into(),
        mode: Some("fast".into()),
        model: None,
        instructions: "x".into(),
        input: json!({"b": 1, "a": 2}),
        output: Json::Null,
        binding: None,
    };
    // The exact bytes recordings were keyed by before bindings existed.
    assert_eq!(
        request.canonical(),
        r#"{"agent":"P::A","input":{"a":2,"b":1},"instructions":"x","mode":"fast","model":null,"output":null}"#
    );
    let bound = AgentRequest {
        binding: Some(binding()),
        ..request.clone()
    };
    assert!(bound.canonical().contains(r#""binding":{"adapter":"agq-providers jev 2","input":{"fields":["longUrl","host"]},"mapping":"choice 1","model":"jev-1.13.0","policy":{"confidence":"choice"},"provider":"typesafe","question":{"#));
    assert_ne!(bound.digest(), request.digest());
    assert_eq!(bound.unbound().digest(), request.digest());
}

/// Every part of the binding is part of the key; the order of an object's
/// keys is not, the order of a list is.
#[test]
fn every_part_of_a_binding_changes_the_key() {
    let base = AgentRequest {
        agent: "UrlShortener::LinkScreening".into(),
        mode: Some("fast".into()),
        model: Some("jev-1.13.0".into()),
        instructions: "Screen it.".into(),
        input: json!({"type": "LinkCandidate", "fields": {"longUrl": "https://a.example/", "host": "a.example"}}),
        output: json!({"type": "Verdict"}),
        binding: Some(binding()),
    };
    let key = base.digest();
    type Change = Box<dyn Fn(&mut Binding)>;
    let changed: Vec<(&str, Change)> = vec![
        ("provider", Box::new(|b| b.provider = "deepseek".into())),
        ("model", Box::new(|b| b.model = "jev-1.14.0".into())),
        (
            "adapter",
            Box::new(|b| b.adapter = "agq-providers jev 3".into()),
        ),
        ("mapping", Box::new(|b| b.mapping = "choice 2".into())),
        (
            "question",
            Box::new(|b| b.question["options"]["allow"] = json!("an ordinary link")),
        ),
        (
            "input",
            Box::new(|b| b.input = json!({"fields": ["longUrl"]})),
        ),
        (
            "policy",
            Box::new(|b| b.policy = json!({"confidence": "derived"})),
        ),
        (
            "list order",
            Box::new(|b| b.input = json!({"fields": ["host", "longUrl"]})),
        ),
    ];
    for (what, change) in changed {
        let mut request = base.clone();
        change(request.binding.as_mut().unwrap());
        assert_ne!(request.digest(), key, "{what} did not change the key");
        assert_ne!(
            request.binding.as_ref().unwrap().digest(),
            base.binding.as_ref().unwrap().digest(),
            "{what}"
        );
    }
    // The same binding written with its keys in another order is the same.
    let mut reordered = base.clone();
    reordered.binding.as_mut().unwrap().question = serde_json::from_str(
        r#"{"options": {"block": "b", "review": "r", "allow": "a"}, "id": "decision"}"#,
    )
    .unwrap();
    assert_eq!(reordered.digest(), key);
}

#[test]
fn agents_are_described_with_their_effective_settings() {
    let tree = tree();
    let id = tree.find("UrlShortener::ScreeningCases").unwrap();
    let agents = describe_agents(&compile(&tree, id).unwrap()).unwrap();
    assert_eq!(agents.len(), 1);
    let agent = &agents[0];
    assert_eq!(agent.request.agent, "UrlShortener::LinkScreening");
    assert_eq!(agent.request.model.as_deref(), Some("deepseek-flash"));
    assert_eq!(agent.request.mode.as_deref(), Some("fast"));
    assert_eq!(agent.min_confidence, Some(0.8));
    assert_eq!(agent.max_latency_ms, Some(500));
    assert_eq!(
        agent.inputs,
        [(
            "LinkCandidate".to_string(),
            vec!["longUrl".to_string(), "host".to_string()]
        )]
    );
    let names: Vec<(&str, bool, bool)> = agent
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.required, f.confidence))
        .collect();
    assert!(names.contains(&("decision", true, false)), "{names:?}");
    assert!(names.contains(&("reason", false, false)), "{names:?}");
    assert!(names.contains(&("confidence", false, true)), "{names:?}");
    let decision = agent.fields.iter().find(|f| f.name == "decision").unwrap();
    let values: Vec<&str> = decision
        .values
        .as_ref()
        .unwrap()
        .iter()
        .map(|(_, n)| n.as_str())
        .collect();
    assert_eq!(values, ["allow", "review", "block"]);
    // A usage's own value is the effective one.
    let tree = parse(&[Source::new(
        "UrlShortener.sysml",
        SHORTENER.replace(":>> maxLatencyMs = 500;", ":>> maxLatencyMs = 750;"),
    )]);
    let id = tree.find("UrlShortener::ScreeningCases").unwrap();
    let agents = describe_agents(&compile(&tree, id).unwrap()).unwrap();
    assert_eq!(agents[0].max_latency_ms, Some(750));
}

/// Live calls carry the binding and a deadline of the agent's own
/// `maxLatencyMs`; their answers keep the provider's evidence, and the
/// recording says which model answered.
#[test]
fn live_calls_carry_the_binding_and_a_deadline() {
    let tree = tree();
    let live = Scripted::new(Vec::new());
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live.clone()),
        Some(prepared(&tree, "ScreeningCases", binding())),
        2,
        None,
    );
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    let seen = live.seen.lock().unwrap();
    assert_eq!(seen.len(), 8);
    for (request, left) in seen.iter() {
        assert_eq!(request.binding.as_ref(), Some(&binding()));
        assert!(*left <= Duration::from_millis(500), "{left:?}");
        assert!(*left > Duration::from_millis(300), "{left:?}");
    }
    assert_eq!(
        result.provenance.binding.as_deref(),
        Some(binding().digest().as_str())
    );
    let summary = result.live.as_ref().unwrap();
    assert_eq!(summary.calls, 8);
    assert_eq!(summary.unknown_cost, 0);
    assert!(summary.cost_usd.is_some());
    let kept = &summary.answers[0];
    assert_eq!(kept.answered_by, "typesafe/jev-1.13.0");
    assert_eq!(kept.request.binding.as_ref(), Some(&binding()));
    let evidence = kept.evidence.as_ref().unwrap();
    assert_eq!(evidence.usage.unwrap().input, Some(40));
    assert!(evidence.estimates.as_ref().unwrap()["probabilities"]["allow"] == 0.93);
}

/// A run prepared for one agent configuration refuses another.
#[test]
fn a_binding_covers_only_the_agent_it_was_prepared_for() {
    let tree = tree();
    let mut other = prepared(&tree, "ScreeningCases", binding());
    other.agent.model = Some("jev-1.14.0".into());
    let live = Scripted::new(Vec::new());
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live.clone()),
        Some(other),
        1,
        None,
    );
    assert_eq!(live.calls.load(Ordering::SeqCst), 0, "no call was made");
    let summary = result.live.as_ref().unwrap();
    assert!(
        summary.failures.iter().any(|(c, _)| c == "unsupported"),
        "{:?}",
        summary.failures
    );
}

/// A provider failure is counted (its time and unknown cost) under its own
/// category, and stops the evaluation; a stop during a call is a stop, even
/// when the client also reported an error.
#[test]
fn failed_and_stopped_calls_are_counted_and_told_apart() {
    let tree = tree();
    let live = Scripted::new(vec![Act::Allow, Act::Fail]);
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live.clone()),
        Some(prepared(&tree, "ScreeningCases", binding())),
        1,
        None,
    );
    let summary = result.live.as_ref().unwrap();
    assert_eq!(summary.calls, 2);
    assert_eq!(summary.unknown_cost, 1);
    assert_eq!(summary.cost_usd, None, "never a smaller total");
    assert!(summary.known_cost_usd.is_some_and(|c| c > 0.0));
    assert_eq!(summary.failures, [("providerError".to_string(), 1)]);
    assert_eq!(result.status, RunStatus::Stopped);
    assert_eq!(
        live.calls.load(Ordering::SeqCst),
        2,
        "no sample after the failure"
    );
    assert!(
        result
            .describe(None, 0)
            .contains("plus 1 call(s) of unknown cost")
    );

    let live = Scripted::new(vec![Act::Allow, Act::StopAndFail]);
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live.clone()),
        Some(prepared(&tree, "ScreeningCases", binding())),
        3,
        None,
    );
    assert_eq!(result.status, RunStatus::Cancelled);
    let summary = result.live.as_ref().unwrap();
    assert_eq!(
        live.calls.load(Ordering::SeqCst),
        2,
        "nothing after the stop"
    );
    assert_eq!(summary.calls, 2);
    assert_eq!(summary.failures, [("cancelled".to_string(), 1)]);
    assert!(
        !summary
            .failures
            .iter()
            .any(|(c, _)| c == "providerError" || c == "harness-failed")
    );
}

/// An allowance bounds the calls of the whole evaluation: the call beyond
/// it is not made.
#[test]
fn the_allowance_is_shared_by_every_sample() {
    let tree = tree();
    let live = Scripted::new(vec![Act::Allow, Act::UnknownCost]);
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live.clone()),
        Some(prepared(&tree, "ScreeningCases", binding())),
        5,
        Some(6),
    );
    assert_eq!(live.calls.load(Ordering::SeqCst), 6);
    let summary = result.live.as_ref().unwrap();
    assert_eq!(summary.calls, 6);
    assert!(
        summary
            .failures
            .contains(&("budget-exhausted".to_string(), 1)),
        "{:?}",
        summary.failures
    );
    assert_eq!(summary.unknown_cost, 1);
    assert_eq!(result.status, RunStatus::Stopped);
    assert_eq!(result.stop.unwrap().reason, StopReason::BudgetExhausted);
}

/// Recordings answer only the binding they were made with. A recording
/// from before bindings never answers a bound request, and the stop says
/// why; a damaged key is never used.
#[test]
fn recordings_answer_only_their_binding() {
    let tree = tree();
    let folder = tempfile::tempdir().unwrap();
    let live = Scripted::new(Vec::new());
    let bound = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live),
        Some(prepared(&tree, "ScreeningCases", binding())),
        1,
        None,
    );
    let answers = bound.live.unwrap().answers;
    assert_eq!(answers.len(), 4);
    // The same answers as a recording from before bindings existed.
    let legacy: Vec<Recording> = answers
        .iter()
        .map(|r| {
            let request = r.request.unbound();
            Recording {
                digest: request.digest(),
                request,
                evidence: None,
                ..r.clone()
            }
        })
        .collect();
    Recordings::keep(folder.path(), &legacy).unwrap();
    let replay = |recordings: Arc<Recordings>, binding: Option<Binding>| {
        run_with(
            &tree,
            "ScreeningCases",
            Mode::Replay,
            Answers::Recordings(recordings),
            binding.map(|b| prepared(&tree, "ScreeningCases", b)),
            1,
            None,
        )
    };
    let read = Arc::new(Recordings::read(folder.path()));
    // Unbound, the legacy recordings still replay as before.
    assert_eq!(replay(read.clone(), None).status, RunStatus::Completed);
    // Bound, they never answer, and the stop says why.
    let stopped = replay(read.clone(), Some(binding()));
    let stop = stopped.stop.unwrap();
    assert_eq!(stop.reason, StopReason::MissingRecording);
    assert!(
        stop.message.contains("made before Agentique recorded"),
        "{}",
        stop.message
    );
    assert!(
        stop.message.contains("never falls through"),
        "{}",
        stop.message
    );
    // Kept bound, they answer that binding and no other.
    Recordings::keep(folder.path(), &answers).unwrap();
    let read = Arc::new(Recordings::read(folder.path()));
    let replayed = replay(read.clone(), Some(binding()));
    assert_eq!(
        replayed.status,
        RunStatus::Completed,
        "{:#?}",
        replayed.stop
    );
    assert!(
        replayed
            .trace
            .iter()
            .filter_map(|e| e.source.as_ref())
            .all(|s| s.starts_with("recording ") && s.contains("typesafe/jev-1.13.0"))
    );
    assert_eq!(replayed.provenance.binding, Some(binding().digest()));
    let mut changed = binding();
    changed.mapping = "choice 2".into();
    assert_eq!(
        replay(read, Some(changed)).stop.unwrap().reason,
        StopReason::MissingRecording
    );
    // A line whose key does not match its request is reported, not used.
    let file = std::fs::read_dir(folder.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = std::fs::read_to_string(&file).unwrap();
    let tampered = text.replace("\"allow\"", "\"block\"");
    std::fs::write(&file, tampered).unwrap();
    let read = Recordings::read(folder.path());
    assert!(
        read.problems
            .iter()
            .any(|p| p.contains("does not match its request")),
        "{:?}",
        read.problems
    );
}

/// A replay or live result is current only while its binding (and, for a
/// replay, the recordings) are those of the present; one without a binding
/// is never current.
#[test]
fn freshness_compares_the_binding_and_the_recordings() {
    let tree = tree();
    let live = Scripted::new(Vec::new());
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live),
        Some(prepared(&tree, "ScreeningCases", binding())),
        1,
        None,
    );
    let digest = binding().digest();
    let mut other = binding();
    other.model = "jev-1.14.0".into();
    let other = other.digest();
    let present = |binding: Result<Option<&str>, &str>, recordings: Option<&str>| {
        freshness(
            &result,
            &Present {
                tree: &tree,
                binding,
                recordings,
            },
        )
    };
    assert!(present(Ok(Some(&digest)), None).is_current());
    assert!(!present(Ok(Some(&other)), None).is_current());
    assert!(!present(Ok(None), None).is_current());
    assert!(!present(Err("unknown model"), None).is_current());
    assert!(!freshness(&result, &Present::model(&tree)).is_current());
    let mut legacy = result.clone();
    legacy.provenance.binding = None;
    let why = freshness(
        &legacy,
        &Present {
            tree: &tree,
            binding: Ok(Some(&digest)),
            recordings: None,
        },
    );
    assert!(!why.is_current(), "{why:?}");
    // A scenario that asks no agent has no binding, then or now.
    assert!(
        freshness(
            &legacy,
            &Present {
                tree: &tree,
                binding: Ok(None),
                recordings: None,
            },
        )
        .is_current()
    );
    let mut older = result.clone();
    older.provenance.runner = "agq-simulation 0.0.9 live".into();
    assert!(
        !freshness(
            &older,
            &Present {
                tree: &tree,
                binding: Ok(Some(&digest)),
                recordings: None
            }
        )
        .is_current()
    );
    // A replay also depends on the recordings it read.
    let mut replay = result.clone();
    replay.mode = Mode::Replay;
    replay.provenance.runner = replay.provenance.runner.replace(" live", " replay");
    replay.provenance.recordings = Some("r1".into());
    let at = |recordings: &str| {
        freshness(
            &replay,
            &Present {
                tree: &tree,
                binding: Ok(Some(&digest)),
                recordings: Some(recordings),
            },
        )
        .is_current()
    };
    assert!(at("r1"));
    assert!(!at("r2"));
}

/// What an older build reads (C-52, §7.6): the format number is unchanged,
/// so the shape of a result without the new fields must be exactly the old
/// one, and a bound recording, read by the old request type (the fields it
/// had), parses but can never be matched: its key is not the key of any
/// request an older build makes.
#[test]
fn an_older_reader_ignores_or_never_matches_what_is_new() {
    let tree = tree();
    let model = run_with(
        &tree,
        "ShortenAllowed",
        Mode::Model,
        Answers::StandIns,
        None,
        1,
        None,
    );
    let text = serde_json::to_string(&model).unwrap();
    for new in [
        "binding",
        "unknownCost",
        "knownCostUsd",
        "evidence",
        "\"calls\"",
    ] {
        assert!(!text.contains(new), "a model run writes {new}");
    }
    assert_eq!(model.format, 1);

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OldRequest {
        agent: String,
        mode: Option<String>,
        model: Option<String>,
        instructions: String,
        input: Json,
        output: Json,
    }
    #[derive(serde::Deserialize)]
    struct OldRecording {
        digest: String,
        request: OldRequest,
    }
    let live = Scripted::new(Vec::new());
    let result = run_with(
        &tree,
        "ScreeningCases",
        Mode::Live,
        Answers::Live(live),
        Some(prepared(&tree, "ScreeningCases", binding())),
        1,
        None,
    );
    for recording in &result.live.as_ref().unwrap().answers {
        let line = serde_json::to_string(recording).unwrap();
        let old: OldRecording = serde_json::from_str(&line).expect("an older build reads it");
        let asked = AgentRequest {
            agent: old.request.agent,
            mode: old.request.mode,
            model: old.request.model,
            instructions: old.request.instructions,
            input: old.request.input,
            output: old.request.output,
            binding: None,
        };
        assert_ne!(old.digest, asked.digest(), "an older build would match it");
    }
    // A stop reason an older build does not know makes it skip the result.
    let text = serde_json::to_string(&StopReason::BudgetExhausted).unwrap();
    assert_eq!(text, "\"budget-exhausted\"");
}
