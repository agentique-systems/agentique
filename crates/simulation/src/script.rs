//! A scenario's steps, run against a target: the model engine or a real
//! implementation's harness (ROADMAP §4.14–§4.15). The same interpreter
//! evaluates the same expressions and checks for both, so a scenario means
//! the same thing wherever it runs; only what the target can see differs,
//! and a check it cannot evaluate is `unsupported`, never guessed.

use crate::compile::{Path, Program, Step, expression_paths};
use crate::eval::{Env, EvalError, eval};
use crate::result::{CheckResult, EventKind, Stop, StopReason, TraceEvent, Verdict};
use crate::value::Value;
use agq_language::ElementId;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// An output that left the subject.
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    /// The port, as `api.shorten`.
    pub port: String,
    pub value: Value,
    pub time_ms: u64,
}

/// The trace being recorded.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trace {
    pub events: Vec<TraceEvent>,
}

impl Trace {
    pub fn push(
        &mut self,
        time_ms: u64,
        kind: EventKind,
        text: impl Into<String>,
        elements: Vec<ElementId>,
    ) -> &mut TraceEvent {
        let seq = self.events.len() as u64;
        self.events.push(TraceEvent {
            seq,
            time_ms,
            kind,
            text: text.into(),
            elements: elements.into_iter().map(ElementId::raw).collect(),
            path: None,
            values: Vec::new(),
            source: None,
        });
        self.events.last_mut().expect("just pushed")
    }
}

/// What a scenario's steps drive.
pub trait Target {
    /// Whether it can read the subject's internal state (checks that do
    /// are `unsupported` elsewhere).
    fn sees_inside(&self) -> bool;
    /// Logical milliseconds since the run started.
    fn now(&self) -> u64;
    /// Sends `value` into the subject through the port at `port`.
    fn send(&mut self, port: &Path, value: Value, trace: &mut Trace) -> Result<(), Stop>;
    /// Waits for a value of type `ty` to leave the subject through `port`;
    /// `None` when the subject goes quiet without one.
    fn accept(
        &mut self,
        port: &Path,
        ty: ElementId,
        trace: &mut Trace,
    ) -> Result<Option<Value>, Stop>;
    /// Lets `ms` of logical time pass.
    fn wait(&mut self, ms: u64, trace: &mut Trace) -> Result<(), Stop>;
    /// Reads internal state along `path` (parts, then an attribute): the
    /// value and how many steps it used.
    fn read(&self, path: &[ElementId]) -> Result<(Value, usize), String>;
    /// Runs until nothing is pending; returns outputs nobody accepted.
    fn finish(&mut self, trace: &mut Trace) -> Result<Vec<Output>, Stop>;
    /// Events processed so far.
    fn events(&self) -> u64;
}

/// What running the steps produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub checks: Vec<CheckResult>,
    pub stop: Option<Stop>,
    pub unexpected: Vec<Output>,
}

/// Runs the scenario's steps in order against `target`.
pub fn run_steps(
    program: &Program,
    target: &mut dyn Target,
    trace: &mut Trace,
    cancel: &AtomicBool,
) -> Outcome {
    let scenario = &program.scenario;
    let mut bindings: HashMap<ElementId, Value> = HashMap::new();
    let mut checks: Vec<CheckResult> = Vec::new();
    let mut stop: Option<Stop> = None;
    let mut missing: Option<ElementId> = None;
    for step in &scenario.steps {
        if stop.is_some() {
            if let Step::Check {
                element,
                name,
                text,
                ..
            } = step
            {
                let blocked = missing.is_some_and(|m| reads(step, m));
                checks.push(CheckResult {
                    element: element.raw(),
                    name: name.clone(),
                    expression: text.clone(),
                    verdict: if blocked {
                        Verdict::Blocked
                    } else {
                        Verdict::NotRun
                    },
                    message: if blocked {
                        "a value it needs never arrived".into()
                    } else {
                        "the run stopped before this check".into()
                    },
                    deterministic: true,
                    samples: None,
                    implicit: false,
                });
            }
            continue;
        }
        if cancel.load(Ordering::SeqCst) {
            stop = Some(Stop {
                reason: StopReason::Cancelled,
                element: Some(step.element().raw()),
                message: "cancelled".into(),
            });
            continue;
        }
        let env = ScriptEnv {
            bindings: &bindings,
            subject: scenario.subject,
            target: &*target,
        };
        let result: Result<(), Stop> = match step {
            Step::Send {
                element,
                port,
                payload,
            } => match eval(payload, &env) {
                Ok(value) => {
                    let text = format!("Send {value} into `{}`", port.text);
                    trace.push(target.now(), EventKind::Step, text, vec![*element]);
                    target.send(port, value, trace)
                }
                Err(error) => Err(eval_stop(*element, &error)),
            },
            Step::Accept {
                element,
                port,
                ty,
                type_name,
                binding,
                name,
            } => {
                trace.push(
                    target.now(),
                    EventKind::Step,
                    format!("Wait for a `{type_name}` from `{}`", port.text),
                    vec![*element],
                );
                match target.accept(port, *ty, trace) {
                    Ok(Some(value)) => {
                        let event = trace.push(
                            target.now(),
                            EventKind::Step,
                            format!("Received {name} = {value}"),
                            vec![*element],
                        );
                        event.values.push((name.clone(), value.to_string()));
                        bindings.insert(*binding, value);
                        Ok(())
                    }
                    Ok(None) => {
                        missing = Some(*binding);
                        Err(Stop {
                            reason: StopReason::AwaitedOutputMissing,
                            element: Some(element.raw()),
                            message: format!(
                                "no `{type_name}` came out of `{}`: the system went quiet without sending one",
                                port.text
                            ),
                        })
                    }
                    Err(stop) => Err(stop),
                }
            }
            Step::Wait { element, duration } => match eval(duration, &env) {
                Ok(Value::Int(ms)) if ms >= 0 => {
                    trace.push(
                        target.now(),
                        EventKind::Step,
                        format!("Wait {ms} ms"),
                        vec![*element],
                    );
                    target.wait(ms as u64, trace)
                }
                Ok(other) => Err(Stop {
                    reason: StopReason::EvaluationError,
                    element: Some(element.raw()),
                    message: format!("a wait takes whole milliseconds, not {other}"),
                }),
                Err(error) => Err(eval_stop(*element, &error)),
            },
            Step::Check {
                element,
                name,
                expr,
                text,
                internal,
            } => {
                let check = if *internal && !target.sees_inside() {
                    CheckResult {
                        element: element.raw(),
                        name: name.clone(),
                        expression: text.clone(),
                        verdict: Verdict::Unsupported,
                        message:
                            "it reads the subject's internal state, which this runner cannot see"
                                .into(),
                        deterministic: true,
                        samples: None,
                        implicit: false,
                    }
                } else {
                    let (verdict, message) = match eval(expr, &env) {
                        Ok(Value::Bool(true)) => (Verdict::Passed, describe(expr, &env, true)),
                        Ok(Value::Bool(false)) => (Verdict::Failed, describe(expr, &env, false)),
                        Ok(other) => (
                            Verdict::Inconclusive,
                            format!("the constraint is {}, not a Boolean", other.kind()),
                        ),
                        Err(error) => (
                            Verdict::Inconclusive,
                            format!("it could not be evaluated: {error}"),
                        ),
                    };
                    CheckResult {
                        element: element.raw(),
                        name: name.clone(),
                        expression: text.clone(),
                        verdict,
                        message,
                        deterministic: true,
                        samples: None,
                        implicit: false,
                    }
                };
                let event = trace.push(
                    target.now(),
                    EventKind::Check,
                    format!("Check `{name}`: {}", check.verdict.label()),
                    vec![*element],
                );
                event.values.push(("message".into(), check.message.clone()));
                checks.push(check);
                Ok(())
            }
        };
        if let Err(s) = result {
            stop = Some(s);
        }
    }
    // After the last step: let everything pending happen, then say what the
    // subject sent that the scenario did not accept.
    let mut unexpected = Vec::new();
    if stop.is_none() {
        match target.finish(trace) {
            Ok(outputs) => unexpected = outputs,
            Err(s) => stop = Some(s),
        }
    }
    let (verdict, message) = match (&stop, unexpected.is_empty()) {
        (Some(_), _) => (
            Verdict::NotRun,
            "the run stopped before it ended".to_string(),
        ),
        (None, true) => (
            Verdict::Passed,
            "the subject sent nothing the scenario did not accept".to_string(),
        ),
        (None, false) => (
            Verdict::Failed,
            format!(
                "the subject also sent {}",
                unexpected
                    .iter()
                    .map(|o| format!("{} through `{}` at {} ms", o.value, o.port, o.time_ms))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        ),
    };
    checks.push(CheckResult {
        element: scenario.element.raw(),
        name: "no unexpected output".into(),
        expression: String::new(),
        verdict,
        message,
        deterministic: true,
        samples: None,
        implicit: true,
    });
    if let Some(stop) = &stop {
        trace.push(
            target.now(),
            EventKind::Stopped,
            format!("Stopped ({}): {}", stop.reason.code(), stop.message),
            stop.element.map(ElementId::from_raw).into_iter().collect(),
        );
    }
    Outcome {
        checks,
        stop,
        unexpected,
    }
}

fn eval_stop(element: ElementId, error: &EvalError) -> Stop {
    Stop {
        reason: StopReason::EvaluationError,
        element: Some(element.raw()),
        message: error.to_string(),
    }
}

/// Whether a check reads the value bound by `binding`.
fn reads(step: &Step, binding: ElementId) -> bool {
    let Step::Check { expr, .. } = step else {
        return false;
    };
    let mut found = false;
    expression_paths(expr, &mut |steps| found |= steps.first() == Some(&binding));
    found
}

/// A check's message: the values its names had.
fn describe(expr: &crate::eval::Expr, env: &dyn Env, held: bool) -> String {
    let mut values = Vec::new();
    collect_paths(expr, &mut |steps, text| {
        if let Ok(value) = eval(
            &crate::eval::Expr::Path {
                steps: steps.to_vec(),
                text: text.to_string(),
            },
            env,
        ) {
            let entry = format!("{text} = {value}");
            if !values.contains(&entry) {
                values.push(entry);
            }
        }
    });
    let outcome = if held { "held" } else { "did not hold" };
    if values.is_empty() {
        format!("the constraint {outcome}")
    } else {
        format!("the constraint {outcome}: {}", values.join(", "))
    }
}

fn collect_paths(expr: &crate::eval::Expr, visit: &mut dyn FnMut(&[ElementId], &str)) {
    use crate::eval::Expr;
    match expr {
        Expr::Const(_) => {}
        Expr::Path { steps, text } => visit(steps, text),
        Expr::Unary(_, operand) => collect_paths(operand, visit),
        Expr::Binary(_, a, b) => {
            collect_paths(a, visit);
            collect_paths(b, visit);
        }
        Expr::Cond(c, a, b) => {
            collect_paths(c, visit);
            collect_paths(a, visit);
            collect_paths(b, visit);
        }
        Expr::New { fields, .. } => {
            for value in fields.iter().filter_map(|(_, v)| v.as_ref()) {
                collect_paths(value, visit);
            }
        }
    }
}

/// Names in a scenario: values its steps received, and (for runners that
/// see inside) the subject's parts and attributes.
struct ScriptEnv<'a> {
    bindings: &'a HashMap<ElementId, Value>,
    subject: ElementId,
    target: &'a dyn Target,
}

impl Env for ScriptEnv<'_> {
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
        let first = steps
            .first()
            .ok_or_else(|| EvalError(format!("`{text}` names nothing")))?;
        if let Some(value) = self.bindings.get(first) {
            return Ok((value.clone(), 1));
        }
        if *first == self.subject {
            return self
                .target
                .read(&steps[1..])
                .map(|(value, used)| (value, used + 1))
                .map_err(|why| EvalError(format!("`{text}`: {why}")));
        }
        Err(EvalError(format!(
            "`{text}` has no value yet: it names a step that has not received anything"
        )))
    }
}
