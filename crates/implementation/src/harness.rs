//! Scenarios against the real implementation (ROADMAP §4.15): the
//! implementation runner starts the project's harness (a small program in
//! the implementation repository) and drives it one JSON line at a time.
//! The harness calls the real code; the runner evaluates the scenario's
//! expressions and checks itself, with the same interpreter as model
//! execution. It never replays expected answers.
//!
//! **Protocol** (one JSON object per line; the harness answers each command
//! with any number of `output` and `log` lines, then `{"ready": true}`, or
//! with `{"error": "..."}`):
//!
//! | Runner sends | Meaning |
//! |---|---|
//! | `{"start": {"scenario": Q, "subject": T, "standIns": [...]}}` | Build the system for subject type `T` with these stand-ins in place of the parts they name |
//! | `{"send": {"port": "api.shorten", "value": V}}` | Put `V` into the subject through that port |
//! | `{"wait": ms}` | Let `ms` of the implementation's time pass (a harness with no clock of its own just answers) |
//! | `{"finish": true}` | Let everything pending finish; report what else came out |
//!
//! | Harness answers | Meaning |
//! |---|---|
//! | `{"output": {"port": "api.shorten", "value": V}}` | `V` came out of the subject through that port |
//! | `{"log": "text"}` | A line for the trace |
//! | `{"ready": true}` | Done with the command |
//! | `{"error": "text"}` | It cannot go on (the run stops with `harness-failed`) |
//!
//! A stand-in is `{"target": "screening", "call": 1 | null, "outcome":
//! "answer" | "timeout" | "invalidOutput" | "refusal" | "toolUnavailable",
//! "latencyMs": n, "output": V | null}`. Values are items as `{"type": T,
//! "fields": {...}}`, enum values by name, scalars as JSON.

use crate::links::Links;
use agq_execution::{Executor, Interactive, Program as Command, git};
use agq_language::ElementId;
use agq_simulation::agents::from_json;
use agq_simulation::compile::{Path as PortPath, Step, Types};
use agq_simulation::engine::Answers;
use agq_simulation::eval::{Env, EvalError, eval};
use agq_simulation::result::{ImplementationProvenance, RunStatus};
use agq_simulation::{
    EventKind, Output, Program, Request, RunResult, Stop, StopReason, Target, Trace, Value,
    run_steps,
};
use serde_json::{Value as Json, json};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// How long the harness may take to start (it may build first).
const START_TIMEOUT: Duration = Duration::from_secs(900);
/// How long one command may take.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// A running harness, as a scenario target.
pub struct HarnessTarget {
    process: Interactive,
    types: Types,
    /// Port names to the paths the scenario uses, for trace elements.
    ports: HashMap<String, PortPath>,
    outputs: Vec<(String, Json, u64)>,
    now: u64,
    events: u64,
}

impl HarnessTarget {
    /// Starts the harness and hands it the scenario's stand-ins.
    pub fn start(
        executor: &Executor,
        command: &Command,
        folder: &str,
        program: &Program,
        trace: &mut Trace,
    ) -> Result<HarnessTarget, Stop> {
        let harness_stop = |message: String| Stop {
            reason: StopReason::HarnessFailed,
            element: Some(program.scenario.element.raw()),
            message,
        };
        let process = executor
            .spawn(command, folder)
            .map_err(|refusal| harness_stop(refusal.to_string()))?;
        let mut ports = HashMap::new();
        for step in &program.scenario.steps {
            if let Step::Send { port, .. } | Step::Accept { port, .. } = step {
                ports.insert(port.text.clone(), port.clone());
            }
        }
        let mut target = HarnessTarget {
            process,
            types: program.types.clone(),
            ports,
            outputs: Vec::new(),
            now: 0,
            events: 0,
        };
        let stand_ins = stand_ins(program).map_err(harness_stop)?;
        let start = json!({
            "start": {
                "scenario": program.scenario.qualified_name,
                "subject": program.scenario.subject_type_name,
                "standIns": stand_ins,
            }
        });
        trace.push(
            0,
            EventKind::Note,
            format!("Started the harness: {}", command.display()),
            vec![program.scenario.element],
        );
        target.command(&start, START_TIMEOUT, trace)?;
        Ok(target)
    }

    /// Sends a command and reads until `ready`.
    fn command(
        &mut self,
        command: &Json,
        timeout: Duration,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let stop = |message: String| Stop {
            reason: StopReason::HarnessFailed,
            element: None,
            message,
        };
        self.process
            .send(&command.to_string())
            .map_err(|e| stop(format!("the harness stopped: {e}")))?;
        let started = Instant::now();
        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            let line = self
                .process
                .read(remaining.max(Duration::from_millis(1)))
                .map_err(|e| stop(format!("the harness did not answer: {e}")))?;
            let Ok(message) = serde_json::from_str::<Json>(&line) else {
                // Not protocol: something the code printed. Keep it visible.
                trace.push(
                    self.now,
                    EventKind::Note,
                    format!("harness: {line}"),
                    Vec::new(),
                );
                continue;
            };
            if message.get("ready").is_some() {
                return Ok(());
            }
            if let Some(error) = message.get("error") {
                return Err(stop(format!(
                    "the harness reported: {}",
                    error
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| error.to_string())
                )));
            }
            if let Some(log) = message.get("log") {
                let text = log
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| log.to_string());
                trace.push(
                    self.now,
                    EventKind::Note,
                    format!("implementation: {text}"),
                    Vec::new(),
                );
                continue;
            }
            if let Some(output) = message.get("output") {
                let port = output["port"].as_str().unwrap_or("").to_string();
                let value = output["value"].clone();
                self.events += 1;
                let elements = self
                    .ports
                    .get(&port)
                    .and_then(|p| p.features.last().copied())
                    .into_iter()
                    .collect();
                trace.push(
                    self.now,
                    EventKind::Output,
                    format!("{} came out of `{port}`", describe(&value)),
                    elements,
                );
                self.outputs.push((port, value, self.now));
                continue;
            }
            trace.push(
                self.now,
                EventKind::Note,
                format!("harness: {line}"),
                Vec::new(),
            );
        }
    }
}

fn describe(json: &Json) -> String {
    match (json.get("type").and_then(Json::as_str), json.get("fields")) {
        (Some(ty), Some(fields)) => {
            let fields: Vec<String> = fields
                .as_object()
                .map(|f| f.iter().map(|(k, v)| format!("{k} = {v}")).collect())
                .unwrap_or_default();
            format!("{ty}({})", fields.join(", "))
        }
        _ => json.to_string(),
    }
}

/// The scenario's stand-ins as the harness reads them.
fn stand_ins(program: &Program) -> Result<Vec<Json>, String> {
    struct Nothing;
    impl Env for Nothing {
        fn lookup(&self, _: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
            Err(EvalError(format!(
                "`{text}` is not known when the scenario starts"
            )))
        }
    }
    let mut out = Vec::new();
    for stand_in in &program.scenario.stand_ins {
        let output = match &stand_in.output {
            Some(expr) => eval(expr, &Nothing)
                .map_err(|e| format!("the output of stand-in `{}`: {e}", stand_in.name))?
                .to_json(),
            None => Json::Null,
        };
        out.push(json!({
            "target": stand_in.target.text,
            "call": stand_in.call,
            "outcome": stand_in.outcome.name(),
            "latencyMs": stand_in.latency_ms,
            "output": output,
        }));
    }
    Ok(out)
}

impl Target for HarnessTarget {
    fn sees_inside(&self) -> bool {
        false
    }

    fn now(&self) -> u64 {
        self.now
    }

    fn send(&mut self, port: &PortPath, value: Value, trace: &mut Trace) -> Result<(), Stop> {
        self.events += 1;
        trace.push(
            self.now,
            EventKind::Received,
            format!(
                "The implementation received {value} through `{}`",
                port.text
            ),
            port.features.last().copied().into_iter().collect(),
        );
        let command = json!({ "send": { "port": port.text, "value": value.to_json() } });
        self.command(&command, COMMAND_TIMEOUT, trace)
    }

    fn accept(
        &mut self,
        port: &PortPath,
        ty: ElementId,
        _trace: &mut Trace,
    ) -> Result<Option<Value>, Stop> {
        let position = self
            .outputs
            .iter()
            .position(|(p, json, _)| *p == port.text && from_json(&self.types, ty, json).is_ok());
        Ok(position.map(|i| {
            let (_, json, _) = self.outputs.remove(i);
            from_json(&self.types, ty, &json).expect("checked")
        }))
    }

    fn wait(&mut self, ms: u64, trace: &mut Trace) -> Result<(), Stop> {
        self.command(&json!({ "wait": ms }), COMMAND_TIMEOUT, trace)?;
        self.now += ms;
        Ok(())
    }

    fn read(&self, _path: &[ElementId]) -> Result<(Value, usize), String> {
        Err("the implementation's internal state is not visible to this runner".into())
    }

    fn finish(&mut self, trace: &mut Trace) -> Result<Vec<Output>, Stop> {
        self.command(&json!({ "finish": true }), COMMAND_TIMEOUT, trace)?;
        let types = &self.types;
        Ok(std::mem::take(&mut self.outputs)
            .into_iter()
            .map(|(port, json, time_ms)| {
                let value = json
                    .get("type")
                    .and_then(Json::as_str)
                    .and_then(|name| {
                        types
                            .items
                            .iter()
                            .find(|(_, t)| t.name == name)
                            .map(|(id, _)| *id)
                    })
                    .and_then(|ty| from_json(types, ty, &json).ok())
                    .unwrap_or_else(|| Value::Str(describe(&json)));
                Output {
                    port,
                    value,
                    time_ms,
                }
            })
            .collect())
    }

    fn events(&self) -> u64 {
        self.events
    }
}

/// Runs a compiled scenario against the implementation in `repository`
/// through its linked harness.
pub fn run_implementation(
    program: &Program,
    model_digest: String,
    request: &Request,
    links: &Links,
    repository: &Path,
    executor: &Executor,
    cancel: Arc<AtomicBool>,
) -> RunResult {
    let started = Instant::now();
    let mut result = agq_simulation::begin(program, model_digest, request, &Answers::StandIns);
    let Some(command) = Command::from_list(&links.harness) else {
        result.status = RunStatus::Blocked;
        result.blockers.push((
            program.scenario.element.raw(),
            "no harness is linked: the implementation has no program the runner can drive (links.json `harness`)".into(),
        ));
        return result;
    };
    let head = git::head(repository).ok();
    let changed = git::changed_files(repository).unwrap_or_default();
    result.provenance.implementation = Some(ImplementationProvenance {
        repository: repository.to_string_lossy().into_owned(),
        commit: head.map(|h| h.commit).unwrap_or_else(|| "unknown".into()),
        dirty: !changed.is_empty(),
        tree_digest: git::tree_digest(repository).unwrap_or_default(),
        harness: command.display(),
    });
    let mut trace = Trace::default();
    let outcome = match HarnessTarget::start(executor, &command, "", program, &mut trace) {
        Ok(mut target) => run_steps(program, &mut target, &mut trace, &cancel),
        Err(stop) => {
            trace.push(
                0,
                EventKind::Stopped,
                format!("Stopped ({}): {}", stop.reason.code(), stop.message),
                Vec::new(),
            );
            agq_simulation::script::Outcome {
                checks: agq_simulation::not_run_checks(program),
                stop: Some(stop),
                unexpected: Vec::new(),
            }
        }
    };
    agq_simulation::finish(&mut result, outcome, &mut trace);
    result.wall_ms = started.elapsed().as_millis() as u64;
    result
}

/// Whether an implementation result still describes the code: the same
/// commit and working tree.
pub fn code_is_current(result: &RunResult, repository: &Path) -> Result<(), String> {
    let Some(recorded) = &result.provenance.implementation else {
        return Ok(());
    };
    let digest = git::tree_digest(repository).map_err(|e| e.to_string())?;
    if digest == recorded.tree_digest {
        Ok(())
    } else {
        Err("the code changed since it ran".into())
    }
}
