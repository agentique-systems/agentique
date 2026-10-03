//! Model execution (ROADMAP §4.14): the subject's parts run their explicit
//! behaviour over a fixed snapshot, in logical time.
//!
//! - Events are processed in order of logical time, then of the order they
//!   were scheduled; delivering a message takes no time.
//! - A state machine handles one event at a time to completion: exit the
//!   source state, run the effect, enter the target, then take any enabled
//!   completion transition. Two transitions enabled for one event stop the
//!   run (`ambiguous-transition`); a message nothing accepts stops it
//!   (`unhandled-message`); a message reaching a part without behaviour
//!   stops it (`missing-behaviour`); a message sent through a port that
//!   leads nowhere stops it (`missing-connection`).
//! - An agent's answer comes from the source the run was given, is checked
//!   against its contract, and on failure goes to its fallback, with the
//!   reason in the trace.
//! - Limits on events, logical time, completion depth and wall-clock time,
//!   and cancellation, end a run explicitly.

use crate::agents::{
    AgentAnswer, AgentRequest, AnswerOutput, CallLimits, Recordings, RunBinding, from_json, shape,
};
use crate::compile::{
    Action, Behaviour, Instance, LinkKind, Outcome, Path, Program, StandIn, System, Transition,
    Trigger, find_instance,
};
use crate::eval::{Env, EvalError, eval};
use crate::result::{EventKind, Stop, StopReason};
use crate::script::{Output, Target, Trace};
use crate::value::Value;
use agq_language::{Direction, ElementId};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Bounds on a run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_events: u64,
    pub max_time_ms: u64,
    pub max_depth: u32,
    pub max_wall: Duration,
    /// Live model calls this run may still make (C-52): the evaluation's
    /// allowance. A call beyond it is not made; the run stops
    /// (`budget-exhausted`). `None`: no allowance applies.
    pub max_live_calls: Option<u32>,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_events: 100_000,
            max_time_ms: 24 * 60 * 60 * 1000,
            max_depth: 100,
            max_wall: Duration::from_secs(10),
            max_live_calls: None,
        }
    }
}

/// Where agents' answers come from in this run.
pub enum Answers {
    /// The scenario's stand-ins (model execution).
    StandIns,
    /// Recordings, matched by request digest (replay).
    Recordings(Arc<Recordings>),
    /// A live model (live evaluation).
    Live(Arc<dyn crate::agents::LiveModel>),
}

/// An agent call the run made, for keeping good answers as recordings.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentCall {
    pub request: AgentRequest,
    pub answer: AgentAnswer,
    /// Why the answer was not used, if it was not.
    pub failure: Option<String>,
    /// The provider could not be asked or answered unusably (C-52): no
    /// answer of the agent exists, and the run stopped. Kept so its time and
    /// cost are counted.
    pub provider_error: Option<String>,
}

enum Pending {
    Arrive {
        port: usize,
        value: Value,
        connection: Option<ElementId>,
    },
    Leave {
        port: usize,
        value: Value,
    },
    Timer {
        instance: usize,
        transition: usize,
        generation: u64,
    },
    AgentResult {
        instance: usize,
        port: usize,
        input: Value,
        result: Result<Value, String>,
    },
}

/// A model run in progress.
pub struct Engine<'p> {
    program: &'p Program,
    system: &'p System,
    answers: Answers,
    limits: Limits,
    cancel: Arc<AtomicBool>,
    now: u64,
    seq: u64,
    queue: BinaryHeap<Reverse<(u64, u64)>>,
    pending: HashMap<u64, Pending>,
    slots: Vec<Vec<Value>>,
    states: Vec<Option<usize>>,
    generation: Vec<u64>,
    calls: HashMap<usize, u64>,
    outputs: Vec<(usize, Value, u64)>,
    /// Stand-ins by the instance they answer for, with their outputs.
    stand_ins: HashMap<usize, Vec<(&'p StandIn, Option<Value>)>>,
    events: u64,
    started: Instant,
    /// The binding prepared for the run's agent (replay and live).
    binding: Option<RunBinding>,
    /// Live calls made by this engine.
    live_calls: u32,
    /// The agent calls made, in order.
    pub agent_calls: Vec<AgentCall>,
}

impl<'p> Engine<'p> {
    /// Prepares the system: attribute values, stand-ins, and every state
    /// machine in its first state. `Err` stops the run before its steps.
    pub fn start(
        program: &'p Program,
        answers: Answers,
        limits: Limits,
        binding: Option<RunBinding>,
        cancel: Arc<AtomicBool>,
        trace: &mut Trace,
    ) -> Result<Engine<'p>, Stop> {
        let system = program.system.as_ref().map_err(|blockers| Stop {
            reason: StopReason::Unsupported,
            element: blockers.first().map(|b| b.element.raw()),
            message: blockers
                .iter()
                .map(|b| b.message.clone())
                .collect::<Vec<_>>()
                .join("; "),
        })?;
        let mut engine = Engine {
            program,
            system,
            answers,
            limits,
            cancel,
            now: 0,
            seq: 0,
            queue: BinaryHeap::new(),
            pending: HashMap::new(),
            slots: vec![Vec::new(); system.instances.len()],
            states: vec![None; system.instances.len()],
            generation: vec![0; system.instances.len()],
            calls: HashMap::new(),
            outputs: Vec::new(),
            stand_ins: HashMap::new(),
            events: 0,
            started: Instant::now(),
            binding,
            live_calls: 0,
            agent_calls: Vec::new(),
        };
        for (index, instance) in system.instances.iter().enumerate() {
            let mut values = Vec::with_capacity(instance.slots.len());
            for slot in &instance.slots {
                let value = match &slot.init {
                    Some(init) => {
                        let env = SlotEnv {
                            instance,
                            values: &values,
                        };
                        eval(init, &env).map_err(|e| Stop {
                            reason: StopReason::EvaluationError,
                            element: Some(slot.feature.raw()),
                            message: format!("the value of `{}.{}`: {e}", instance.path, slot.name),
                        })?
                    }
                    None => Value::Null,
                };
                values.push(value);
            }
            engine.slots[index] = values;
        }
        // Stand-in outputs are known when the scenario starts.
        let empty = HashMap::new();
        for stand_in in &program.scenario.stand_ins {
            let Some(instance) = find_instance(system, 0, &stand_in.target.features) else {
                continue;
            };
            let output = match &stand_in.output {
                Some(expr) => Some(eval(expr, &ConstEnv(&empty)).map_err(|e| Stop {
                    reason: StopReason::EvaluationError,
                    element: Some(stand_in.element.raw()),
                    message: format!(
                        "the output of stand-in `{}`: {e} (a stand-in's output can use only values known when the scenario starts)",
                        stand_in.name
                    ),
                })?),
                None => None,
            };
            engine
                .stand_ins
                .entry(instance)
                .or_default()
                .push((stand_in, output));
        }
        for index in 0..system.instances.len() {
            if let Behaviour::Machine(machine) = system.instances[index].behaviour {
                engine.start_machine(index, machine, trace)?;
            }
        }
        engine.settle(trace)?;
        Ok(engine)
    }

    fn instance(&self, index: usize) -> &'p Instance {
        &self.system.instances[index]
    }

    fn schedule(&mut self, at: u64, pending: Pending) {
        self.seq += 1;
        self.queue.push(Reverse((at, self.seq)));
        self.pending.insert(self.seq, pending);
    }

    fn stop(&self, reason: StopReason, element: Option<ElementId>, message: String) -> Stop {
        Stop {
            reason,
            element: element.map(ElementId::raw),
            message,
        }
    }

    /// Processes the next event, if any; `false` when nothing is pending.
    fn step(&mut self, trace: &mut Trace) -> Result<bool, Stop> {
        let Some(Reverse((at, seq))) = self.queue.pop() else {
            return Ok(false);
        };
        if self.cancel.load(Ordering::SeqCst) {
            return Err(self.stop(StopReason::Cancelled, None, "cancelled".into()));
        }
        if at > self.limits.max_time_ms {
            return Err(self.stop(
                StopReason::TimeLimit,
                None,
                format!("logical time would pass {} ms", self.limits.max_time_ms),
            ));
        }
        self.events += 1;
        if self.events > self.limits.max_events {
            return Err(self.stop(
                StopReason::EventLimit,
                None,
                format!("more than {} events", self.limits.max_events),
            ));
        }
        if self.started.elapsed() > self.limits.max_wall {
            return Err(self.stop(
                StopReason::WallClockLimit,
                None,
                format!("the run took longer than {:?}", self.limits.max_wall),
            ));
        }
        self.now = at;
        let pending = self.pending.remove(&seq).expect("scheduled");
        match pending {
            Pending::Arrive {
                port,
                value,
                connection,
            } => self.arrive(port, value, connection, trace)?,
            Pending::Leave { port, value } => self.leave(port, value, trace)?,
            Pending::Timer {
                instance,
                transition,
                generation,
            } => self.timer(instance, transition, generation, trace)?,
            Pending::AgentResult {
                instance,
                port,
                input,
                result,
            } => self.agent_result(instance, port, input, result, trace)?,
        }
        Ok(true)
    }

    /// Processes every event due now.
    fn settle(&mut self, trace: &mut Trace) -> Result<(), Stop> {
        while self
            .queue
            .peek()
            .is_some_and(|Reverse((at, _))| *at <= self.now)
        {
            self.step(trace)?;
        }
        Ok(())
    }

    // ---- ports and routes ----

    /// An item arrives at `port` from outside its part.
    fn arrive(
        &mut self,
        port: usize,
        value: Value,
        connection: Option<ElementId>,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        // Passed on to an inner part through a delegation?
        let inward: Vec<usize> = self
            .system
            .links
            .iter()
            .filter_map(|link| match link.kind {
                LinkKind::Delegation { outer } if outer == port => {
                    Some(if link.a == port { link.b } else { link.a })
                }
                _ => None,
            })
            .collect();
        if !inward.is_empty() {
            for inner in inward {
                self.arrive(inner, value.clone(), connection, trace)?;
            }
            return Ok(());
        }
        let p = &self.system.ports[port];
        let instance = p.instance;
        let usage = self.instance(instance).usage;
        let mut elements = vec![usage, p.feature];
        elements.extend(connection);
        let event = trace.push(
            self.now,
            EventKind::Received,
            format!(
                "`{}` received {value} through `{}`",
                self.instance(instance).path,
                p.name
            ),
            elements,
        );
        event.path = Some(self.instance(instance).path.clone());
        if let Some(stand_ins) = self.stand_ins.get(&instance).cloned()
            && !matches!(self.instance(instance).behaviour, Behaviour::Agent(_))
        {
            return self.stand_in(instance, port, &stand_ins, trace);
        }
        match &self.instance(instance).behaviour {
            Behaviour::Agent(_) => self.agent_call(instance, port, value, trace),
            Behaviour::Machine(machine) => self.machine_event(instance, *machine, port, value, trace),
            Behaviour::None => Err(self.stop(
                StopReason::MissingBehaviour,
                Some(usage),
                format!(
                    "`{}` received {value} through `{}`, but it has no behaviour to handle it: give its definition an exhibit state, or a stand-in in the scenario",
                    self.instance(instance).path,
                    p.name
                ),
            )),
            Behaviour::Unsupported(why) => Err(self.stop(
                StopReason::Unsupported,
                Some(usage),
                format!("`{}` cannot run: {why}", self.instance(instance).path),
            )),
        }
    }

    /// An item leaves a part through `port`.
    fn leave(&mut self, port: usize, value: Value, trace: &mut Trace) -> Result<(), Stop> {
        let p = &self.system.ports[port];
        let instance = p.instance;
        if !self.can_send(port, &value) {
            return Err(self.stop(
                StopReason::EvaluationError,
                Some(p.feature),
                format!("`{}` has no `out` item that carries {value}", p.path),
            ));
        }
        let event = trace.push(
            self.now,
            EventKind::Sent,
            format!(
                "`{}` sent {value} through `{}`",
                self.instance(instance).path,
                p.name
            ),
            vec![self.instance(instance).usage, p.feature],
        );
        event.path = Some(self.instance(instance).path.clone());
        let mut routed = false;
        let mut onward = Vec::new();
        for link in &self.system.links {
            let other = if link.a == port {
                link.b
            } else if link.b == port {
                link.a
            } else {
                continue;
            };
            match link.kind {
                LinkKind::Facing => {
                    routed = true;
                    onward.push((other, true, link.element));
                }
                LinkKind::Delegation { outer } if outer != port => {
                    routed = true;
                    onward.push((outer, false, link.element));
                }
                LinkKind::Fallback { agent_port } if agent_port != port => {
                    routed = true;
                    onward.push((agent_port, false, None));
                }
                _ => {}
            }
        }
        for (next, arrives, connection) in onward {
            if arrives {
                self.schedule(
                    self.now,
                    Pending::Arrive {
                        port: next,
                        value: value.clone(),
                        connection,
                    },
                );
            } else {
                self.leave_quietly(next, value.clone(), trace)?;
            }
        }
        if !routed {
            if instance == 0 {
                let path = self.port_path(port);
                let event = trace.push(
                    self.now,
                    EventKind::Output,
                    format!("{value} came out of `{path}`"),
                    vec![p.feature],
                );
                event.path = Some(self.instance(0).path.clone());
                self.outputs.push((port, value, self.now));
            } else {
                return Err(self.stop(
                    StopReason::MissingConnection,
                    Some(p.feature),
                    format!(
                        "`{}` sent {value} through `{}`, but that port is not connected to anything",
                        self.instance(instance).path,
                        p.name
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Passing out through an outer port (a delegation or an agent's port):
    /// the same as leaving, without a second "sent" line when it goes on.
    fn leave_quietly(&mut self, port: usize, value: Value, trace: &mut Trace) -> Result<(), Stop> {
        self.leave(port, value, trace)
    }

    /// The port as a path from the subject, such as `api.shorten`.
    fn port_path(&self, port: usize) -> String {
        let p = &self.system.ports[port];
        let mut names = vec![p.name.clone()];
        let mut current = p.instance;
        while let Some(parent) = self.instance(current).parent {
            names.push(self.instance(current).name.clone());
            current = parent;
        }
        names.reverse();
        names.join(".")
    }

    /// Whether `port` has an `out` item that carries `value`.
    fn can_send(&self, port: usize, value: &Value) -> bool {
        let types = &self.program.types;
        self.system.ports[port]
            .directed
            .iter()
            .any(|(_, direction, ty)| {
                matches!(direction, Direction::Out | Direction::InOut)
                    && match (ty, value) {
                        (None, _) => true,
                        (Some(ty), Value::Item(item)) => types.specializes(item.ty, *ty),
                        (Some(ty), value) => types.fits(value, *ty).is_ok(),
                    }
            })
    }

    /// The port of `path` (from the subject).
    fn port_at(&self, path: &Path) -> Option<usize> {
        let (parts, port) = path.features.split_at(path.features.len().checked_sub(1)?);
        let instance = find_instance(self.system, 0, parts)?;
        self.instance(instance)
            .ports
            .iter()
            .copied()
            .find(|p| self.system.ports[*p].aliases.contains(&port[0]))
    }

    // ---- stand-ins ----

    fn stand_in(
        &mut self,
        instance: usize,
        port: usize,
        stand_ins: &[(&'p StandIn, Option<Value>)],
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let call = {
            let n = self.calls.entry(instance).or_insert(0);
            *n += 1;
            *n
        };
        let chosen = stand_ins
            .iter()
            .find(|(s, _)| s.call == Some(call))
            .or_else(|| stand_ins.iter().find(|(s, _)| s.call.is_none()));
        let path = self.instance(instance).path.clone();
        let Some((stand_in, output)) = chosen else {
            return Err(self.stop(
                StopReason::MissingStandIn,
                Some(self.instance(instance).usage),
                format!(
                    "`{path}` was called a {} time, and no stand-in answers call {call}",
                    ordinal(call)
                ),
            ));
        };
        let event = trace.push(
            self.now,
            EventKind::StandIn,
            format!(
                "Stand-in `{}` {} for `{path}` (call {call})",
                stand_in.name,
                stand_in.outcome.words()
            ),
            vec![stand_in.element, self.instance(instance).usage],
        );
        event.path = Some(path.clone());
        event.source = Some(format!("stand-in {}", stand_in.name));
        if stand_in.outcome == Outcome::Answer {
            let value = output.clone().expect("an answering stand-in has an output");
            let out = self.output_port(instance, port, &value).ok_or_else(|| {
                self.stop(
                    StopReason::EvaluationError,
                    Some(stand_in.element),
                    format!("`{path}` has no port that can send {value}"),
                )
            })?;
            self.schedule(
                self.now + stand_in.latency_ms,
                Pending::Leave { port: out, value },
            );
        }
        Ok(())
    }

    /// The port an answer leaves through: the one the call came in on if it
    /// can carry it, else another port of the part that can.
    fn output_port(&self, instance: usize, arrived: usize, value: &Value) -> Option<usize> {
        if self.can_send(arrived, value) {
            return Some(arrived);
        }
        self.instance(instance)
            .ports
            .iter()
            .copied()
            .find(|p| self.can_send(*p, value))
    }

    // ---- agents ----

    fn setting(&self, instance: usize, feature: Option<ElementId>) -> Value {
        let Some(feature) = feature else {
            return Value::Null;
        };
        let inst = self.instance(instance);
        inst.slots
            .iter()
            .position(|s| s.aliases.contains(&feature))
            .map(|i| self.slots[instance][i].clone())
            .unwrap_or(Value::Null)
    }

    fn agent_call(
        &mut self,
        instance: usize,
        port: usize,
        input: Value,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let Behaviour::Agent(agent) = &self.instance(instance).behaviour else {
            unreachable!("called for agents")
        };
        let library = &self.program.library;
        let types = &self.program.types;
        let path = self.instance(instance).path.clone();
        let usage = self.instance(instance).usage;
        let call = {
            let n = self.calls.entry(instance).or_insert(0);
            *n += 1;
            *n
        };
        let max_latency = match self.setting(instance, library.max_latency) {
            Value::Int(n) if n >= 0 => Some(n as u64),
            _ => None,
        };
        let min_confidence = self.setting(instance, library.min_confidence).as_f64();
        let agent_name = self
            .instance(instance)
            .types
            .first()
            .map(|t| {
                self.program
                    .names
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| types.name(*t))
            })
            .unwrap_or_else(|| path.clone());
        let request = AgentRequest {
            agent: agent_name,
            mode: match self.setting(instance, library.mode) {
                Value::Enum { name, .. } => Some(name),
                _ => None,
            },
            model: match self.setting(instance, library.model) {
                Value::Str(s) => Some(s),
                _ => None,
            },
            instructions: agent.instructions.clone(),
            input: input.to_json(),
            output: agent
                .output_type
                .map(|t| shape(types, t))
                .unwrap_or(serde_json::Value::Null),
            binding: None,
        };
        // Replay and live calls carry the binding prepared for this agent;
        // another agent or configuration is not covered by it (C-52).
        let mut request = request;
        if !matches!(self.answers, Answers::StandIns)
            && let Some(prepared) = &self.binding
        {
            if !prepared.agent.same_agent(&request) {
                return Err(self.stop(
                    StopReason::Unsupported,
                    Some(usage),
                    format!(
                        "this run was prepared for `{}` with model {}; `{path}` ({}, model {}) is another agent or configuration, which it does not cover",
                        prepared.agent.agent,
                        prepared.agent.model.as_deref().unwrap_or("not set"),
                        request.agent,
                        request.model.as_deref().unwrap_or("not set"),
                    ),
                ));
            }
            request.binding = Some(prepared.binding.clone());
        }
        let answer = self.answer(instance, call, &request, max_latency)?;
        let event = trace.push(
            self.now,
            EventKind::AgentCalled,
            format!(
                "Asked the model of `{path}` (call {call}); it {}",
                answer.outcome.words()
            ),
            vec![usage],
        );
        event.path = Some(path.clone());
        event.source = Some(answer.source.clone());
        event.values.push(("input".into(), input.to_string()));
        // The contract: in time, the right type, valid values, confident enough.
        let (at, result) = match answer.outcome {
            Outcome::Timeout => match max_latency {
                Some(max) => (self.now + max, Err(format!("no answer within {max} ms"))),
                None => {
                    return Err(self.stop(
                        StopReason::Unsupported,
                        Some(usage),
                        format!("the model of `{path}` does not answer, and `maxLatencyMs` does not say when that is noticed"),
                    ));
                }
            },
            _ if max_latency.is_some_and(|max| answer.latency_ms > max) => {
                let max = max_latency.expect("checked");
                (
                    self.now + max,
                    Err(format!(
                        "it answered after {} ms, over maxLatencyMs {max}",
                        answer.latency_ms
                    )),
                )
            }
            Outcome::Refusal => (
                self.now + answer.latency_ms,
                Err("the model refused".into()),
            ),
            Outcome::ToolUnavailable => (
                self.now + answer.latency_ms,
                Err("a tool it needs could not be reached".into()),
            ),
            Outcome::InvalidOutput => (
                self.now + answer.latency_ms,
                Err("its answer broke the contract".into()),
            ),
            Outcome::Answer => (
                self.now + answer.latency_ms,
                self.check_answer(agent.output_type, &answer, min_confidence),
            ),
        };
        self.agent_calls.push(AgentCall {
            request,
            answer,
            failure: result.as_ref().err().cloned(),
            provider_error: None,
        });
        self.schedule(
            at,
            Pending::AgentResult {
                instance,
                port,
                input,
                result,
            },
        );
        Ok(())
    }

    fn answer(
        &mut self,
        instance: usize,
        call: u64,
        request: &AgentRequest,
        max_latency: Option<u64>,
    ) -> Result<AgentAnswer, Stop> {
        let path = self.instance(instance).path.clone();
        let usage = self.instance(instance).usage;
        match &self.answers {
            Answers::StandIns => {
                let stand_ins = self.stand_ins.get(&instance);
                let chosen = stand_ins.and_then(|list| {
                    list.iter()
                        .find(|(s, _)| s.call == Some(call))
                        .or_else(|| list.iter().find(|(s, _)| s.call.is_none()))
                });
                let Some((stand_in, output)) = chosen else {
                    return Err(self.stop(
                        StopReason::MissingStandIn,
                        Some(usage),
                        format!(
                            "the agent `{path}` was asked a {} time, and no stand-in answers call {call}; model execution never calls a real model",
                            ordinal(call)
                        ),
                    ));
                };
                Ok(AgentAnswer {
                    outcome: stand_in.outcome,
                    output: output.clone().map(AnswerOutput::Value),
                    latency_ms: stand_in.latency_ms,
                    source: format!("stand-in {}", stand_in.name),
                    cost_usd: None,
                    evidence: None,
                })
            }
            Answers::Recordings(recordings) => {
                let digest = request.digest();
                let Some(recording) = recordings.get(&digest) else {
                    let unbound = if recordings.has_unbound(request) {
                        "; a recording made before Agentique recorded which provider, model and mapping answered matches it otherwise, but cannot answer it"
                    } else {
                        ""
                    };
                    return Err(self.stop(
                        StopReason::MissingRecording,
                        Some(usage),
                        format!(
                            "no recording answers this request to `{path}` (digest {}…){unbound}; a replay never falls through to a live call",
                            &digest[..12]
                        ),
                    ));
                };
                Ok(AgentAnswer {
                    outcome: recording.outcome,
                    output: recording.output.clone().map(AnswerOutput::Json),
                    latency_ms: recording.latency_ms,
                    source: format!("recording {}… ({})", &digest[..12], recording.answered_by),
                    cost_usd: None,
                    evidence: recording.evidence.clone(),
                })
            }
            Answers::Live(model) => {
                let model = model.clone();
                // The evaluation's allowance is checked before the call:
                // nothing is sent beyond it.
                if self
                    .limits
                    .max_live_calls
                    .is_some_and(|allowed| self.live_calls >= allowed)
                {
                    return Err(self.stop(
                        StopReason::BudgetExhausted,
                        Some(usage),
                        format!(
                            "the evaluation's allowance of {} live call(s) is used up; `{path}` was not asked",
                            self.limits.max_live_calls.unwrap_or_default()
                        ),
                    ));
                }
                self.live_calls += 1;
                // One deadline: the agent's own limit from now, or the run's
                // wall-clock limit if that comes first.
                let wall = self.started + self.limits.max_wall;
                let deadline = max_latency
                    .map(|ms| Instant::now() + Duration::from_millis(ms))
                    .map_or(wall, |agent| agent.min(wall));
                let live = model.answer(request, CallLimits { deadline }, &self.cancel);
                let answer = AgentAnswer {
                    outcome: live.outcome,
                    output: live.output.map(AnswerOutput::Json),
                    latency_ms: live.latency_ms,
                    source: format!("live {}", model.label()),
                    cost_usd: live.cost_usd,
                    evidence: live.evidence,
                };
                // A stop that came during the call is a stop, whatever the
                // client answered or why it failed.
                if self.cancel.load(Ordering::SeqCst) {
                    self.agent_calls.push(AgentCall {
                        request: request.clone(),
                        answer,
                        failure: Some("cancelled".into()),
                        provider_error: None,
                    });
                    return Err(self.stop(StopReason::Cancelled, None, "cancelled".into()));
                }
                if let Some(error) = live.error {
                    // No answer of the agent exists; the call is kept so its
                    // time and cost are counted.
                    self.agent_calls.push(AgentCall {
                        request: request.clone(),
                        answer,
                        failure: Some(error.clone()),
                        provider_error: Some(error.clone()),
                    });
                    return Err(self.stop(
                        StopReason::HarnessFailed,
                        Some(usage),
                        format!("the live model could not be asked: {error}"),
                    ));
                }
                Ok(answer)
            }
        }
    }

    /// An answer's value, if it keeps the contract.
    fn check_answer(
        &self,
        output_type: Option<ElementId>,
        answer: &AgentAnswer,
        min_confidence: Option<f64>,
    ) -> Result<Value, String> {
        let types = &self.program.types;
        let Some(ty) = output_type else {
            return Err("the agent has no `out` item to answer with".into());
        };
        let value = match &answer.output {
            None => return Err("it answered with nothing".into()),
            Some(AnswerOutput::Value(value)) => value.clone(),
            Some(AnswerOutput::Json(json)) => from_json(types, ty, json)
                .map_err(|why| format!("its answer broke the contract: {why}"))?,
        };
        types
            .fits(&value, ty)
            .map_err(|why| format!("its answer broke the contract: {why}"))?;
        let Value::Item(item) = &value else {
            return Err(format!("its answer is {}, not an item", value.kind()));
        };
        if let Some(shape) = types.items.get(&item.ty) {
            for field in &shape.fields {
                if field.required
                    && item
                        .field(field.slot.feature)
                        .is_none_or(|f| f.value == Value::Null)
                {
                    return Err(format!(
                        "its answer broke the contract: `{}` is missing",
                        field.slot.name
                    ));
                }
            }
        }
        let confidence = self
            .program
            .library
            .confidence
            .and_then(|feature| item.field(feature))
            .and_then(|field| field.value.as_f64());
        if confidence.is_none()
            && let Some(min) = min_confidence
        {
            return Err(format!(
                "it gave no confidence, so minConfidence {min} is not met"
            ));
        }
        if let Some(confidence) = confidence {
            if !(0.0..=1.0).contains(&confidence) {
                return Err(format!(
                    "its answer broke the contract: confidence {confidence} is not between 0 and 1"
                ));
            }
            if let Some(min) = min_confidence
                && confidence < min
            {
                return Err(format!(
                    "its confidence {confidence} is below minConfidence {min}"
                ));
            }
        }
        Ok(value)
    }

    fn agent_result(
        &mut self,
        instance: usize,
        port: usize,
        input: Value,
        result: Result<Value, String>,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let Behaviour::Agent(agent) = &self.instance(instance).behaviour else {
            unreachable!("agent results come from agents")
        };
        let path = self.instance(instance).path.clone();
        let usage = self.instance(instance).usage;
        match result {
            Ok(value) => {
                let event = trace.push(
                    self.now,
                    EventKind::AgentAnswered,
                    format!("`{path}` answered {value}"),
                    vec![usage],
                );
                event.path = Some(path.clone());
                let out = self.output_port(instance, port, &value).ok_or_else(|| {
                    self.stop(
                        StopReason::EvaluationError,
                        Some(usage),
                        format!("`{path}` has no port that can send {value}"),
                    )
                })?;
                self.leave(out, value, trace)
            }
            Err(reason) => {
                let event = trace.push(
                    self.now,
                    EventKind::AgentFailed,
                    format!("The model of `{path}` failed: {reason}"),
                    vec![usage],
                );
                event.path = Some(path.clone());
                let Some(fallback) = agent.fallback else {
                    return Err(self.stop(
                        StopReason::AgentFailedWithoutFallback,
                        Some(usage),
                        format!("the model of `{path}` failed ({reason}) and the agent has no fallback; no outcome is guessed"),
                    ));
                };
                let name = self.system.ports[port].name.clone();
                let fallback_port = self
                    .instance(fallback)
                    .ports
                    .iter()
                    .copied()
                    .find(|p| self.system.ports[*p].name == name)
                    .ok_or_else(|| {
                        self.stop(
                            StopReason::Unsupported,
                            Some(usage),
                            format!("the fallback of `{path}` has no port `{name}`"),
                        )
                    })?;
                let event = trace.push(
                    self.now,
                    EventKind::Fallback,
                    format!(
                        "`{path}` handed the call to its fallback `{}` because {reason}",
                        self.instance(fallback).path
                    ),
                    vec![self.instance(fallback).usage, usage],
                );
                event.path = Some(self.instance(fallback).path.clone());
                event.values.push(("reason".into(), reason));
                self.arrive(fallback_port, input, None, trace)
            }
        }
    }

    // ---- state machines ----

    fn machine_env(
        &self,
        instance: usize,
        payload: Option<(ElementId, &Value)>,
    ) -> MachineEnv<'_, 'p> {
        MachineEnv {
            engine: self,
            instance,
            payload: payload.map(|(id, value)| (id, value.clone())),
        }
    }

    fn start_machine(
        &mut self,
        instance: usize,
        machine: usize,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let m = &self.system.machines[machine];
        self.run_actions(instance, &m.entry, None, trace)?;
        if let Some(initial) = m.initial {
            self.enter(instance, machine, initial, 0, trace)?;
        }
        Ok(())
    }

    fn enter(
        &mut self,
        instance: usize,
        machine: usize,
        state: usize,
        depth: u32,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        if depth > self.limits.max_depth {
            return Err(self.stop(
                StopReason::DepthLimit,
                Some(self.instance(instance).usage),
                format!(
                    "`{}` went through more than {} transitions without waiting for anything",
                    self.instance(instance).path,
                    self.limits.max_depth
                ),
            ));
        }
        let m = &self.system.machines[machine];
        let s = &m.states[state];
        self.states[instance] = Some(state);
        self.generation[instance] += 1;
        let event = trace.push(
            self.now,
            EventKind::StateEntered,
            format!("`{}` is {}", self.instance(instance).path, s.name),
            vec![s.element, self.instance(instance).usage],
        );
        event.path = Some(self.instance(instance).path.clone());
        self.run_actions(instance, &s.entry, None, trace)?;
        // Time triggers of the new state start now.
        let generation = self.generation[instance];
        for (index, transition) in m.transitions.iter().enumerate() {
            if transition.source != state {
                continue;
            }
            if let Trigger::After(duration) = &transition.trigger {
                let ms = match eval(duration, &self.machine_env(instance, None)) {
                    Ok(Value::Int(ms)) if ms >= 0 => ms as u64,
                    Ok(other) => {
                        return Err(self.stop(
                            StopReason::EvaluationError,
                            Some(transition.element),
                            format!("`accept after` takes whole milliseconds, not {other}"),
                        ));
                    }
                    Err(e) => return Err(self.eval_stop(transition.element, e)),
                };
                self.schedule(
                    self.now + ms,
                    Pending::Timer {
                        instance,
                        transition: index,
                        generation,
                    },
                );
            }
        }
        // Completion transitions.
        let enabled = self.enabled(instance, machine, state, None, |t| {
            matches!(t.trigger, Trigger::Completion)
        })?;
        match enabled[..] {
            [] => Ok(()),
            [one] => self.fire(instance, machine, one, None, depth + 1, trace),
            _ => Err(self.ambiguous(instance, machine, &enabled)),
        }
    }

    /// Transitions from `state` that `accepts` and whose guards hold.
    fn enabled(
        &self,
        instance: usize,
        machine: usize,
        state: usize,
        payload: Option<(ElementId, &Value)>,
        accepts: impl Fn(&Transition) -> bool,
    ) -> Result<Vec<usize>, Stop> {
        let m = &self.system.machines[machine];
        let mut out = Vec::new();
        for (index, transition) in m.transitions.iter().enumerate() {
            if transition.source != state || !accepts(transition) {
                continue;
            }
            let payload = match (&transition.trigger, payload) {
                (
                    Trigger::Accept {
                        payload: binding, ..
                    },
                    Some((_, value)),
                ) => Some((*binding, value)),
                _ => None,
            };
            let holds = match &transition.guard {
                None => true,
                Some(guard) => match eval(guard, &self.machine_env(instance, payload)) {
                    Ok(Value::Bool(b)) => b,
                    Ok(other) => {
                        return Err(self.stop(
                            StopReason::EvaluationError,
                            Some(transition.element),
                            format!("the guard is {}, not a Boolean", other.kind()),
                        ));
                    }
                    Err(e) => return Err(self.eval_stop(transition.element, e)),
                },
            };
            if holds {
                out.push(index);
            }
        }
        Ok(out)
    }

    fn ambiguous(&self, instance: usize, machine: usize, enabled: &[usize]) -> Stop {
        let m = &self.system.machines[machine];
        let names: Vec<String> = enabled
            .iter()
            .map(|i| {
                let t = &m.transitions[*i];
                format!("{} → {}", m.states[t.source].name, m.states[t.target].name)
            })
            .collect();
        self.stop(
            StopReason::AmbiguousTransition,
            Some(m.transitions[enabled[0]].element),
            format!(
                "`{}` could take {} for the same event; say which with guards that exclude each other",
                self.instance(instance).path,
                names.join(" or ")
            ),
        )
    }

    fn machine_event(
        &mut self,
        instance: usize,
        machine: usize,
        port: usize,
        value: Value,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let Some(state) = self.states[instance] else {
            return Err(self.stop(
                StopReason::Unsupported,
                Some(self.instance(instance).usage),
                format!(
                    "`{}` has no state to receive in",
                    self.instance(instance).path
                ),
            ));
        };
        let types = &self.program.types;
        let port_aliases = &self.system.ports[port].aliases;
        let accepts = |t: &Transition| match &t.trigger {
            Trigger::Accept { ty, port, .. } => {
                port.is_none_or(|p| port_aliases.contains(&p))
                    && ty.is_none_or(|ty| match &value {
                        Value::Item(item) => types.specializes(item.ty, ty),
                        other => types.fits(other, ty).is_ok(),
                    })
            }
            _ => false,
        };
        let enabled = self.enabled(
            instance,
            machine,
            state,
            Some((ElementId::from_raw(0), &value)),
            accepts,
        )?;
        match enabled[..] {
            [] => {
                let m = &self.system.machines[machine];
                Err(self.stop(
                    StopReason::UnhandledMessage,
                    Some(m.states[state].element),
                    format!(
                        "`{}` received {value} through `{}` while {}, and no transition accepts it there",
                        self.instance(instance).path,
                        self.system.ports[port].name,
                        m.states[state].name
                    ),
                ))
            }
            [one] => self.fire(instance, machine, one, Some(value), 0, trace),
            _ => Err(self.ambiguous(instance, machine, &enabled)),
        }
    }

    fn fire(
        &mut self,
        instance: usize,
        machine: usize,
        transition: usize,
        payload: Option<Value>,
        depth: u32,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        let m = &self.system.machines[machine];
        let t = &m.transitions[transition];
        let event = trace.push(
            self.now,
            EventKind::Transition,
            format!(
                "`{}`: {} → {}",
                self.instance(instance).path,
                m.states[t.source].name,
                m.states[t.target].name
            ),
            vec![t.element, self.instance(instance).usage],
        );
        event.path = Some(self.instance(instance).path.clone());
        self.run_actions(instance, &m.states[t.source].exit, None, trace)?;
        let binding = match (&t.trigger, &payload) {
            (Trigger::Accept { payload: id, .. }, Some(value)) => Some((*id, value.clone())),
            _ => None,
        };
        self.run_actions_with(instance, &t.effect, binding, trace)?;
        self.enter(instance, machine, t.target, depth, trace)
    }

    fn timer(
        &mut self,
        instance: usize,
        transition: usize,
        generation: u64,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        if self.generation[instance] != generation {
            return Ok(()); // the state was left: the timer no longer applies
        }
        let Behaviour::Machine(machine) = self.instance(instance).behaviour else {
            return Ok(());
        };
        let m = &self.system.machines[machine];
        let t = &m.transitions[transition];
        if self.states[instance] != Some(t.source) {
            return Ok(());
        }
        let event = trace.push(
            self.now,
            EventKind::Timer,
            format!(
                "`{}`: time passed in {}",
                self.instance(instance).path,
                m.states[t.source].name
            ),
            vec![t.element, self.instance(instance).usage],
        );
        event.path = Some(self.instance(instance).path.clone());
        let holds = match &t.guard {
            None => true,
            Some(guard) => match eval(guard, &self.machine_env(instance, None)) {
                Ok(Value::Bool(b)) => b,
                Ok(other) => {
                    return Err(self.stop(
                        StopReason::EvaluationError,
                        Some(t.element),
                        format!("the guard is {}, not a Boolean", other.kind()),
                    ));
                }
                Err(e) => return Err(self.eval_stop(t.element, e)),
            },
        };
        if holds {
            self.fire(instance, machine, transition, None, 0, trace)
        } else {
            Ok(())
        }
    }

    fn run_actions(
        &mut self,
        instance: usize,
        actions: &'p [Action],
        payload: Option<(ElementId, Value)>,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        self.run_actions_with(instance, actions, payload, trace)
    }

    fn run_actions_with(
        &mut self,
        instance: usize,
        actions: &'p [Action],
        payload: Option<(ElementId, Value)>,
        trace: &mut Trace,
    ) -> Result<(), Stop> {
        for action in actions {
            match action {
                Action::Send {
                    element,
                    port,
                    payload: expr,
                } => {
                    let value = {
                        let env = MachineEnv {
                            engine: self,
                            instance,
                            payload: payload.clone(),
                        };
                        eval(expr, &env).map_err(|e| self.eval_stop(*element, e))?
                    };
                    let out = self
                        .instance(instance)
                        .ports
                        .iter()
                        .copied()
                        .find(|p| self.system.ports[*p].aliases.contains(port))
                        .ok_or_else(|| {
                            self.stop(
                                StopReason::EvaluationError,
                                Some(*element),
                                format!(
                                    "`{}` has no such port to send through",
                                    self.instance(instance).path
                                ),
                            )
                        })?;
                    self.leave(out, value, trace)?;
                }
                Action::Assign {
                    element,
                    target,
                    text,
                    value,
                } => {
                    let new = {
                        let env = MachineEnv {
                            engine: self,
                            instance,
                            payload: payload.clone(),
                        };
                        eval(value, &env).map_err(|e| self.eval_stop(*element, e))?
                    };
                    self.assign(instance, target, text, new.clone(), *element)?;
                    let event = trace.push(
                        self.now,
                        EventKind::Assigned,
                        format!("`{}`: {text} := {new}", self.instance(instance).path),
                        vec![*element, self.instance(instance).usage],
                    );
                    event.path = Some(self.instance(instance).path.clone());
                }
                Action::If {
                    element,
                    condition,
                    then,
                    otherwise,
                } => {
                    let holds = {
                        let env = MachineEnv {
                            engine: self,
                            instance,
                            payload: payload.clone(),
                        };
                        match eval(condition, &env) {
                            Ok(Value::Bool(b)) => b,
                            Ok(other) => {
                                return Err(self.stop(
                                    StopReason::EvaluationError,
                                    Some(*element),
                                    format!("the condition is {}, not a Boolean", other.kind()),
                                ));
                            }
                            Err(e) => return Err(self.eval_stop(*element, e)),
                        }
                    };
                    let branch = if holds { then } else { otherwise };
                    self.run_actions_with(instance, branch, payload.clone(), trace)?;
                }
            }
        }
        Ok(())
    }

    /// `assign a := v`, `assign a.b := v` (a field of an item attribute) or
    /// `assign part.a := v` (an attribute of an inner part).
    fn assign(
        &mut self,
        instance: usize,
        target: &[ElementId],
        text: &str,
        value: Value,
        element: ElementId,
    ) -> Result<(), Stop> {
        let mut current = instance;
        let mut rest = target;
        loop {
            let Some((first, tail)) = rest.split_first() else {
                return Err(self.stop(
                    StopReason::EvaluationError,
                    Some(element),
                    format!("`{text}` names nothing to set"),
                ));
            };
            if let Some(slot) = self
                .instance(current)
                .slots
                .iter()
                .position(|s| s.aliases.contains(first))
            {
                if tail.is_empty() {
                    self.slots[current][slot] = value;
                    return Ok(());
                }
                let mut holder = self.slots[current][slot].clone();
                set_field(&mut holder, tail, value).map_err(|why| {
                    self.stop(
                        StopReason::EvaluationError,
                        Some(element),
                        format!("`{text}`: {why}"),
                    )
                })?;
                self.slots[current][slot] = holder;
                return Ok(());
            }
            let child = self
                .instance(current)
                .children
                .iter()
                .find(|(aliases, _)| aliases.contains(first))
                .map(|(_, c)| *c);
            match child {
                Some(child) => {
                    current = child;
                    rest = tail;
                }
                None => {
                    return Err(self.stop(
                        StopReason::EvaluationError,
                        Some(element),
                        format!(
                            "`{text}` is not an attribute of `{}`",
                            self.instance(instance).path
                        ),
                    ));
                }
            }
        }
    }

    fn eval_stop(&self, element: ElementId, error: EvalError) -> Stop {
        self.stop(
            StopReason::EvaluationError,
            Some(element),
            error.to_string(),
        )
    }

    /// The value of an attribute path from an instance: the value and the
    /// steps used (parts, then one attribute).
    fn read_from(&self, instance: usize, path: &[ElementId]) -> Result<(Value, usize), String> {
        let mut current = instance;
        for (i, step) in path.iter().enumerate() {
            let inst = self.instance(current);
            if let Some(slot) = inst.slots.iter().position(|s| s.aliases.contains(step)) {
                return Ok((self.slots[current][slot].clone(), i + 1));
            }
            match inst
                .children
                .iter()
                .find(|(aliases, _)| aliases.contains(step))
            {
                Some((_, child)) => current = *child,
                None => {
                    return Err(format!("`{}` has no attribute or part here", inst.path));
                }
            }
        }
        Err("it names a part, not a value".into())
    }

    /// Statistics for the result.
    pub fn now_ms(&self) -> u64 {
        self.now
    }
}

fn set_field(holder: &mut Value, path: &[ElementId], value: Value) -> Result<(), String> {
    let Value::Item(item) = holder else {
        return Err(format!("{} has no fields", holder.kind()));
    };
    let (first, rest) = path.split_first().ok_or("no field")?;
    let field = item
        .fields
        .iter_mut()
        .find(|f| f.feature == *first || f.aliases.contains(first))
        .ok_or_else(|| format!("`{}` has no such field", item.type_name))?;
    if rest.is_empty() {
        field.value = value;
        Ok(())
    } else {
        set_field(&mut field.value, rest, value)
    }
}

fn ordinal(n: u64) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

impl Target for Engine<'_> {
    fn sees_inside(&self) -> bool {
        true
    }

    fn now(&self) -> u64 {
        self.now
    }

    fn send(&mut self, port: &Path, value: Value, trace: &mut Trace) -> Result<(), Stop> {
        let index = self.port_at(port).ok_or_else(|| {
            self.stop(
                StopReason::MissingConnection,
                None,
                format!("the subject has no port `{}` that runs", port.text),
            )
        })?;
        self.arrive(index, value, None, trace)?;
        self.settle(trace)
    }

    fn accept(
        &mut self,
        port: &Path,
        ty: ElementId,
        trace: &mut Trace,
    ) -> Result<Option<Value>, Stop> {
        let index = self.port_at(port).ok_or_else(|| {
            self.stop(
                StopReason::MissingConnection,
                None,
                format!("the subject has no port `{}` that runs", port.text),
            )
        })?;
        loop {
            let types = &self.program.types;
            let found = self.outputs.iter().position(|(p, value, _)| {
                *p == index
                    && match value {
                        Value::Item(item) => types.specializes(item.ty, ty),
                        other => types.fits(other, ty).is_ok(),
                    }
            });
            if let Some(position) = found {
                return Ok(Some(self.outputs.remove(position).1));
            }
            if !self.step(trace)? {
                return Ok(None);
            }
        }
    }

    fn wait(&mut self, ms: u64, trace: &mut Trace) -> Result<(), Stop> {
        let until = self.now + ms;
        while self
            .queue
            .peek()
            .is_some_and(|Reverse((at, _))| *at <= until)
        {
            self.step(trace)?;
        }
        self.now = until;
        Ok(())
    }

    fn read(&self, path: &[ElementId]) -> Result<(Value, usize), String> {
        self.read_from(0, path)
    }

    fn finish(&mut self, trace: &mut Trace) -> Result<Vec<Output>, Stop> {
        while self.step(trace)? {}
        let outputs = std::mem::take(&mut self.outputs);
        Ok(outputs
            .into_iter()
            .map(|(port, value, time_ms)| Output {
                port: self.port_path(port),
                value,
                time_ms,
            })
            .collect())
    }

    fn events(&self) -> u64 {
        self.events
    }
}

/// Names in a part's behaviour: an accepted payload, its attributes and the
/// attributes of its inner parts.
struct MachineEnv<'e, 'p> {
    engine: &'e Engine<'p>,
    instance: usize,
    payload: Option<(ElementId, Value)>,
}

impl Env for MachineEnv<'_, '_> {
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
        let first = steps
            .first()
            .ok_or_else(|| EvalError(format!("`{text}` names nothing")))?;
        if let Some((id, value)) = &self.payload
            && id == first
        {
            return Ok((value.clone(), 1));
        }
        self.engine
            .read_from(self.instance, steps)
            .map_err(|why| EvalError(format!("`{text}`: {why}")))
    }
}

/// Names while attribute values are first worked out: earlier attributes.
/// An agent of a compiled scenario as a run would ask it (C-52): its
/// request without input or binding, its effective settings and the fields
/// of its answer and of what it is asked about. Read-only: nothing runs.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentInfo {
    /// The instance path, such as `service.screening`.
    pub path: String,
    /// The usage it stands for.
    pub element: ElementId,
    /// Its requests without input or binding: the agent, its mode and model
    /// (a usage's own values applied), its instructions and output shape.
    pub request: AgentRequest,
    pub min_confidence: Option<f64>,
    pub max_latency_ms: Option<u64>,
    /// The fields of its answer, in order.
    pub fields: Vec<AnswerField>,
    /// What it is asked about: each item type's name and its fields.
    pub inputs: Vec<(String, Vec<String>)>,
}

/// A field of an agent's answer.
#[derive(Clone, Debug, PartialEq)]
pub struct AnswerField {
    pub name: String,
    pub required: bool,
    /// The library's `AgentOutput::confidence`, found by identity, never by
    /// name.
    pub confidence: bool,
    /// For an enum: its values (element, name) in their declared order.
    pub values: Option<Vec<(ElementId, String)>>,
    /// The declared type's name, if any.
    pub type_name: Option<String>,
    /// An item with fields of its own.
    pub nested: bool,
}

/// Every agent of the scenario's system, with its effective settings.
pub fn describe_agents(program: &Program) -> Result<Vec<AgentInfo>, Stop> {
    let Ok(system) = program.system.as_ref() else {
        return Ok(Vec::new());
    };
    let types = &program.types;
    let library = &program.library;
    let mut out = Vec::new();
    for instance in &system.instances {
        let Behaviour::Agent(agent) = &instance.behaviour else {
            continue;
        };
        let mut values = Vec::with_capacity(instance.slots.len());
        for slot in &instance.slots {
            let value = match &slot.init {
                Some(init) => eval(
                    init,
                    &SlotEnv {
                        instance,
                        values: &values,
                    },
                )
                .map_err(|e| Stop {
                    reason: StopReason::EvaluationError,
                    element: Some(slot.feature.raw()),
                    message: format!("the value of `{}.{}`: {e}", instance.path, slot.name),
                })?,
                None => Value::Null,
            };
            values.push(value);
        }
        let setting = |feature: Option<ElementId>| {
            feature
                .and_then(|f| instance.slots.iter().position(|s| s.aliases.contains(&f)))
                .map(|i| values[i].clone())
                .unwrap_or(Value::Null)
        };
        let agent_name = instance
            .types
            .first()
            .map(|t| {
                program
                    .names
                    .get(t)
                    .cloned()
                    .unwrap_or_else(|| types.name(*t))
            })
            .unwrap_or_else(|| instance.path.clone());
        let request = AgentRequest {
            agent: agent_name,
            mode: match setting(library.mode) {
                Value::Enum { name, .. } => Some(name),
                _ => None,
            },
            model: match setting(library.model) {
                Value::Str(s) => Some(s),
                _ => None,
            },
            instructions: agent.instructions.clone(),
            input: serde_json::Value::Null,
            output: agent
                .output_type
                .map(|t| shape(types, t))
                .unwrap_or(serde_json::Value::Null),
            binding: None,
        };
        let fields = agent
            .output_type
            .and_then(|t| types.items.get(&t))
            .map(|item| {
                item.fields
                    .iter()
                    .map(|field| AnswerField {
                        name: field.slot.name.clone(),
                        required: field.required,
                        confidence: library.confidence.is_some_and(|c| {
                            field.slot.feature == c || field.slot.aliases.contains(&c)
                        }),
                        values: field.ty.and_then(|ty| types.enums.get(&ty).cloned()),
                        type_name: field.ty.map(|ty| types.name(ty)),
                        nested: field.ty.is_some_and(|ty| types.items.contains_key(&ty)),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let inputs = instance
            .ports
            .iter()
            .flat_map(|p| system.ports[*p].directed.clone())
            .filter(|(_, direction, _)| *direction == Direction::In)
            .filter_map(|(_, _, ty)| ty)
            .filter_map(|ty| types.items.get(&ty))
            .map(|item| {
                (
                    item.name.clone(),
                    item.fields.iter().map(|f| f.slot.name.clone()).collect(),
                )
            })
            .collect();
        out.push(AgentInfo {
            path: instance.path.clone(),
            element: instance.usage,
            request,
            min_confidence: setting(library.min_confidence).as_f64(),
            max_latency_ms: match setting(library.max_latency) {
                Value::Int(n) if n >= 0 => Some(n as u64),
                _ => None,
            },
            fields,
            inputs,
        });
    }
    Ok(out)
}

struct SlotEnv<'a> {
    instance: &'a Instance,
    values: &'a [Value],
}

impl Env for SlotEnv<'_> {
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
        let first = steps
            .first()
            .ok_or_else(|| EvalError(format!("`{text}` names nothing")))?;
        let slot = self
            .instance
            .slots
            .iter()
            .position(|s| s.aliases.contains(first))
            .filter(|i| *i < self.values.len())
            .ok_or_else(|| {
                EvalError(format!(
                    "`{text}` is not an attribute declared before this one in `{}`",
                    self.instance.path
                ))
            })?;
        Ok((self.values[slot].clone(), 1))
    }
}

/// No names: constants only.
struct ConstEnv<'a>(&'a HashMap<ElementId, Value>);

impl Env for ConstEnv<'_> {
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
        steps
            .first()
            .and_then(|s| self.0.get(s))
            .map(|v| (v.clone(), 1))
            .ok_or_else(|| EvalError(format!("`{text}` is not known when the scenario starts")))
    }
}
