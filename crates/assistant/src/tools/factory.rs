//! The factory tools (ROADMAP C-50, §4.14, §4.15): reading behaviour and
//! scenarios, the parts of `apply_changes` that write them, and the
//! requests only the Studio can carry out with its own services: running a
//! scenario, reading a result, reading the code links and checking the
//! code. The Assistant never runs anything itself, and a live evaluation
//! (which costs money) is the Operator's to start.

use super::{find, optional_str, required_str};
use agq_language::{
    Element, ElementId, ElementKind, Expression, Parent, Reference, Semantics, StateAction, Tree,
    print_element,
};
use agq_simulation::Mode;
use agq_system_state::{Operation, Property};
use serde_json::Value;

/// What the Studio does for a factory tool.
#[derive(Clone, Debug, PartialEq)]
pub enum StudioRequest {
    /// Run a scenario and answer with its result when it ends.
    Run { scenario: ElementId, mode: Mode },
    /// Stop the run in progress.
    StopRun,
    /// The newest result of a scenario in a mode.
    ReadRun { scenario: ElementId, mode: Mode },
    /// The code links, for one element or all.
    ReadCodeLinks { element: Option<ElementId> },
    /// A part explained from the model and its links.
    Explain { element: ElementId },
    /// Run the implementation checks and answer when they end.
    CheckImplementation,
    /// Ask the Operator to start an implementation task; answer when the
    /// Operator has decided.
    Implement {
        element: ElementId,
        instructions: String,
    },
    /// Show the Operator a start form for an objective (C-54): the
    /// Assistant proposes, only the Operator starts.
    ProposeObjective(ObjectiveProposal),
    /// What the application shows now (C-53): `full` with every command
    /// and the cards in view.
    Observe { full: bool },
    /// An action in the application, as the control interface reads it
    /// (the whole input: action, expect, observed, why).
    Act { input: Value },
}

/// An objective the Assistant proposes (`propose_objective`): its intent,
/// whether it explores, and the budgets it suggests (none: the start
/// form's defaults). The Studio checks the budgets when the Operator starts
/// it; here they are only read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectiveProposal {
    pub intent: String,
    pub explore: bool,
    pub usd: Option<f64>,
    pub cycles: Option<u32>,
    pub attempts: Option<u32>,
    pub hours: Option<f64>,
    pub steps: Option<u32>,
}

impl ObjectiveProposal {
    fn read(input: &Value) -> Result<ObjectiveProposal, String> {
        let intent = required_str(input, "intent")?.trim().to_string();
        let explore = match input.get("explore") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(on)) => *on,
            Some(_) => return Err("`explore` must be true or false".into()),
        };
        let budgets = &input["budgets"];
        let number = |field: &str| budgets.get(field).and_then(Value::as_f64);
        let whole = |field: &str| -> Result<Option<u32>, String> {
            match number(field) {
                None => Ok(None),
                Some(n) if n.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&n) => {
                    Ok(Some(n as u32))
                }
                Some(n) => Err(format!("`budgets.{field}` must be a whole number; got {n}")),
            }
        };
        Ok(ObjectiveProposal {
            intent,
            explore,
            usd: number("usd"),
            cycles: whole("cycles")?,
            attempts: whole("attempts")?,
            hours: number("hours"),
            steps: whole("steps")?,
        })
    }
}

/// The agent settings `inspect_behaviour` lists, as the Agents library
/// names them.
const AGENT_SETTINGS: [&str; 6] = [
    "mode",
    "model",
    "minConfidence",
    "maxLatencyMs",
    "maxCostPerCallUsd",
    "fallback",
];

/// An expression as the Assistant wrote it.
pub(super) fn expression(text: &str) -> Result<Expression, String> {
    agq_language::parse_expression(text)
        .map_err(|reason| format!("the expression `{text}` cannot be read: {reason}"))
}

/// The behaviour fields of a new element.
pub(super) fn behaviour_fields(element: &mut Element, item: &Value) -> Result<(), String> {
    if let Some(text) = optional_str(item, "expression")? {
        element.expression = Some(expression(text)?);
    }
    if let Some(text) = optional_str(item, "via")? {
        element.via = Some(Reference::new(text));
    }
    if item.get("after") == Some(&Value::Bool(true)) {
        element.after = true;
    }
    if let Some(text) = optional_str(item, "guard")? {
        element.guard = Some(expression(text)?);
    }
    if item.get("exhibit") == Some(&Value::Bool(true)) {
        element.exhibit = true;
    }
    if element.kind == ElementKind::Transition {
        let from = required_str(item, "from")?;
        let to = required_str(item, "to")?;
        element.ends = vec![Reference::new(from), Reference::new(to)];
    }
    Ok(())
}

/// What goes inside a new element of `kind` whose id will be `id`: a
/// scenario's subject and objective, a state machine's start, a
/// transition's trigger and effect, and redefined features.
pub(super) fn members(
    tree: &Tree,
    id: ElementId,
    kind: ElementKind,
    item: &Value,
) -> Result<Vec<Operation>, String> {
    let inside = Parent::Element(id);
    let create = |parent: Parent, element: Element| Operation::Create {
        parent,
        element: Box::new(element),
    };
    let mut out = Vec::new();
    match kind {
        ElementKind::VerificationDef => {
            let subject = required_str(item, "subject").map_err(|_| {
                "a verification def needs `subject`, the type of what the scenario is about"
                    .to_string()
            })?;
            let name = match optional_str(item, "subject_name")? {
                Some(name) => name.to_string(),
                None => lower_first(subject.rsplit("::").next().unwrap_or(subject)),
            };
            let mut usage = Element::named(ElementKind::Subject, &name);
            usage.typed_by = vec![Reference::new(subject)];
            out.push(create(inside, usage));
            let verifies = super::optional_list(item, "verifies")?.unwrap_or_default();
            if !verifies.is_empty() {
                let objective = ElementId::from_raw(id.raw() + 2);
                out.push(create(inside, Element::new(ElementKind::Objective)));
                for requirement in verifies {
                    let mut verify = Element::new(ElementKind::Verify);
                    verify.target = Some(Reference::new(requirement));
                    out.push(create(Parent::Element(objective), verify));
                }
            }
        }
        ElementKind::State => {
            if let Some(initial) = optional_str(item, "initial")? {
                let mut entry = Element::new(ElementKind::Action);
                entry.state_action = Some(StateAction::Entry);
                out.push(create(inside, entry));
                let mut then = Element::new(ElementKind::Succession);
                then.target = Some(Reference::new(initial));
                out.push(create(inside, then));
            }
        }
        ElementKind::Transition => {
            if let Some(trigger) = item.get("trigger").filter(|t| !t.is_null()) {
                let mut accept = Element::new(ElementKind::Accept);
                if let Some(after) = optional_str(trigger, "after")? {
                    accept.after = true;
                    accept.expression = Some(expression(after)?);
                } else {
                    accept.name = optional_str(trigger, "name")?.map(str::to_string);
                    accept.typed_by = vec![Reference::new(required_str(trigger, "type").map_err(
                        |_| {
                            "a trigger needs `type` (an item arriving) or `after` (time passing)"
                                .to_string()
                        },
                    )?)];
                    accept.via = optional_str(trigger, "via")?.map(Reference::new);
                }
                out.push(create(inside, accept));
            }
            if let Some(effect) = item.get("effect").filter(|e| !e.is_null()) {
                let node = match (
                    optional_str(effect, "send")?,
                    optional_str(effect, "assign")?,
                ) {
                    (Some(send), None) => {
                        let mut node = Element::new(ElementKind::Send);
                        node.expression = Some(expression(send)?);
                        node.via = optional_str(effect, "via")?.map(Reference::new);
                        node
                    }
                    (None, Some(target)) => {
                        let mut node = Element::new(ElementKind::Assign);
                        node.target = Some(Reference::new(target));
                        node.expression = Some(expression(required_str(effect, "value")?)?);
                        node
                    }
                    _ => {
                        return Err(
                            "an effect is either a `send` (with `via`) or an `assign` (with `value`)"
                                .into(),
                        );
                    }
                };
                out.push(create(inside, node));
            }
        }
        _ => {}
    }
    let _ = tree;
    if let Some(features) = item.get("features") {
        let features = features
            .as_object()
            .ok_or("`features` must be an object of names and values")?;
        for (name, value) in features {
            out.push(create(inside, redefinition(name, value)?));
        }
    }
    Ok(out)
}

/// `:>> name = value`: a value for an inherited feature.
fn redefinition(name: &str, value: &Value) -> Result<Element, String> {
    let mut feature = Element::new(ElementKind::Reference);
    feature.redefines = vec![Reference::new(name)];
    match value {
        Value::String(text) => feature.expression = Some(expression(text)?),
        other => feature.value = Some(super::literal(other)?),
    }
    Ok(feature)
}

/// `set` with `features`: each inherited feature gets its value, in a
/// redefinition that exists or a new one.
pub(super) fn set_features(
    tree: &Tree,
    element: ElementId,
    item: &Value,
) -> Result<Vec<Operation>, String> {
    let features = item
        .get("features")
        .and_then(Value::as_object)
        .ok_or("`features` must be an object of names and values")?;
    let mut out = Vec::new();
    for (name, value) in features {
        let existing = tree[element].children().iter().copied().find(|c| {
            tree[*c]
                .redefines
                .iter()
                .any(|r| r.last_name() == name.as_str())
        });
        match existing {
            Some(child) => {
                let wanted = redefinition(name, value)?;
                out.push(Operation::Set {
                    element: child,
                    property: Property::Value(wanted.value),
                });
                out.push(Operation::Set {
                    element: child,
                    property: Property::Expression(wanted.expression),
                });
            }
            None => out.push(Operation::Create {
                parent: Parent::Element(element),
                element: Box::new(redefinition(name, value)?),
            }),
        }
    }
    Ok(out)
}

/// `inspect_behaviour`.
pub(super) fn inspect_behaviour(tree: &Tree, input: &Value) -> Result<String, String> {
    let element = find(tree, required_str(input, "element")?)?;
    let semantics = Semantics::new(tree);
    let definition = match tree[element].kind {
        ElementKind::PartDef => element,
        ElementKind::Part => semantics
            .types_of(element)
            .first()
            .map(|(d, _)| *d)
            .ok_or("the part has no type to read behaviour from")?,
        other => {
            return Err(format!(
                "`{}` is a {}; behaviour belongs to a part def or part",
                tree.qualified_name(element),
                other.keyword()
            ));
        }
    };
    let name = |id: ElementId| semantics.name(id).unwrap_or("?").to_string();
    let mut lines = vec![format!(
        "{} (part def)",
        semantics.qualified_name(definition)
    )];
    let features = semantics.features(definition);
    // Ports, as a scenario or a neighbour sees them.
    let ports: Vec<ElementId> = features
        .iter()
        .copied()
        .filter(|f| {
            semantics
                .element(*f)
                .is_some_and(|e| e.kind == ElementKind::Port)
        })
        .collect();
    if !ports.is_empty() {
        lines.push("Ports (seen from outside):".into());
        for port in ports {
            let mut takes = Vec::new();
            let mut gives = Vec::new();
            for (_, direction, ty) in semantics.directed_features(port) {
                let ty = ty.map(name).unwrap_or_else(|| "?".into());
                match direction {
                    agq_language::Direction::In => takes.push(ty),
                    agq_language::Direction::Out => gives.push(ty),
                    agq_language::Direction::InOut => {
                        takes.push(ty.clone());
                        gives.push(ty);
                    }
                }
            }
            lines.push(format!(
                "- {}: takes {}; gives {}",
                name(port),
                if takes.is_empty() {
                    "nothing".into()
                } else {
                    takes.join(", ")
                },
                if gives.is_empty() {
                    "nothing".into()
                } else {
                    gives.join(", ")
                }
            ));
        }
    }
    // Its state machine, as written.
    let machine = features
        .iter()
        .copied()
        .chain(tree[definition].children().iter().copied())
        .find(|f| {
            tree.get(*f)
                .is_some_and(|e| e.kind == ElementKind::State && e.exhibit)
        });
    match machine {
        Some(machine) => {
            lines.push("State machine:".into());
            lines.extend(print_element(tree, machine));
        }
        None => lines.push("No state machine: it has no behaviour of its own to run.".into()),
    }
    // Agent settings, with what it inherits.
    if let Some(agent) = semantics.resolve("Agents::Agent")
        && semantics.specializes(definition, agent)
    {
        lines.push("Agent settings:".into());
        for setting in AGENT_SETTINGS {
            let own = features.iter().copied().find(|f| {
                semantics.name(*f) == Some(setting)
                    || semantics
                        .element(*f)
                        .is_some_and(|e| e.redefines.iter().any(|r| r.last_name() == setting))
            });
            let text = own
                .and_then(|f| semantics.element(f).map(|e| (f, e)))
                .map(
                    |(f, e)| match (&e.value, &e.expression, e.typed_by.first()) {
                        (Some(v), _, _) => v.to_string(),
                        (None, Some(x), _) if tree.contains(f) => {
                            agq_language::print_expression(tree, f, x)
                        }
                        (None, None, Some(ty)) => format!(": {ty}"),
                        _ => "default".into(),
                    },
                )
                .unwrap_or_else(|| "default".into());
            lines.push(format!("- {setting}: {text}"));
        }
        if let Some(doc) = doc(tree, definition) {
            lines.push(format!("Instructions (its doc): {doc}"));
        }
    }
    // What a stand-in may replace.
    let mut parts = Vec::new();
    let mut stack = vec![(definition, String::new(), 0)];
    while let Some((holder, prefix, depth)) = stack.pop() {
        if depth > 3 {
            continue;
        }
        for part in semantics.features(holder) {
            if semantics.element(part).map(|e| e.kind) != Some(ElementKind::Part) {
                continue;
            }
            let path = format!("{prefix}{}", name(part));
            let ty = semantics.types_of(part).first().map(|(t, _)| *t);
            parts.push(format!(
                "- {path}{}",
                ty.map(|t| format!(" : {}", name(t))).unwrap_or_default()
            ));
            if let Some(ty) = ty {
                stack.push((ty, format!("{path}."), depth + 1));
            }
        }
    }
    if !parts.is_empty() {
        parts.sort();
        lines.push("Parts inside (a scenario may stand in for them):".into());
        lines.extend(parts);
    }
    Ok(super::cap(lines.join("\n")))
}

/// `list_scenarios`: each scenario as written.
pub(super) fn list_scenarios(tree: &Tree) -> String {
    let scenarios: Vec<String> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::VerificationDef)
        .map(|id| {
            format!(
                "{}:\n{}",
                tree.qualified_name(id),
                print_element(tree, id).unwrap_or_default()
            )
        })
        .collect();
    if scenarios.is_empty() {
        return "The model has no scenarios yet. A scenario is a verification def: create one with apply_changes (kind \"verification def\" with a `subject`), then its stand-ins, steps and checks.".into();
    }
    super::cap(scenarios.join("\n\n"))
}

/// The request for a tool the Studio carries out.
pub(super) fn studio_request(
    tree: &Tree,
    tool: &str,
    input: &Value,
) -> Result<StudioRequest, String> {
    let scenario = || -> Result<ElementId, String> {
        let id = find(tree, required_str(input, "scenario")?)?;
        if tree[id].kind != ElementKind::VerificationDef {
            return Err(format!(
                "`{}` is not a scenario (a verification def)",
                tree.qualified_name(id)
            ));
        }
        Ok(id)
    };
    let mode = |default: Mode| -> Result<Mode, String> {
        match optional_str(input, "mode")? {
            None => Ok(default),
            Some(key) => Mode::from_key(key).ok_or_else(|| format!("unknown mode `{key}`")),
        }
    };
    Ok(match tool {
        super::RUN_SCENARIO => {
            let mode = mode(Mode::Model)?;
            if mode == Mode::Live {
                return Err("a live evaluation costs money: only the Operator starts one, from the Run panel".into());
            }
            StudioRequest::Run {
                scenario: scenario()?,
                mode,
            }
        }
        super::STOP_RUN => StudioRequest::StopRun,
        super::PROPOSE_OBJECTIVE => {
            StudioRequest::ProposeObjective(ObjectiveProposal::read(input)?)
        }
        super::OBSERVE_APP => StudioRequest::Observe {
            full: input["detail"] == "full",
        },
        super::ACT_IN_APP => StudioRequest::Act {
            input: input.clone(),
        },
        super::READ_RUN => StudioRequest::ReadRun {
            scenario: scenario()?,
            mode: mode(Mode::Model)?,
        },
        super::PROPOSE_IMPLEMENTATION => {
            let element = find(tree, required_str(input, "element")?)?;
            if !matches!(tree[element].kind, ElementKind::PartDef | ElementKind::Part) {
                return Err(format!(
                    "`{}` is a {}; a task implements a part def or part",
                    tree.qualified_name(element),
                    tree[element].kind.keyword()
                ));
            }
            StudioRequest::Implement {
                element,
                instructions: optional_str(input, "instructions")?
                    .unwrap_or_default()
                    .to_string(),
            }
        }
        super::EXPLAIN_ELEMENT => {
            let element = find(tree, required_str(input, "element")?)?;
            if !matches!(tree[element].kind, ElementKind::PartDef | ElementKind::Part) {
                return Err(format!(
                    "`{}` is a {}; explain_element explains a part def or a part",
                    tree.qualified_name(element),
                    tree[element].kind.keyword()
                ));
            }
            StudioRequest::Explain { element }
        }
        super::READ_CODE_LINKS => StudioRequest::ReadCodeLinks {
            element: optional_str(input, "element")?
                .map(|name| find(tree, name))
                .transpose()?,
        },
        _ => StudioRequest::CheckImplementation,
    })
}

fn doc(tree: &Tree, id: ElementId) -> Option<String> {
    tree[id]
        .children()
        .iter()
        .find(|c| tree[**c].kind == ElementKind::Doc)
        .and_then(|d| tree[*d].text.clone())
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn lower_first(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

/// Carries out a request without a Studio, for the evaluations and tests:
/// model and walkthrough runs on this thread with the scenario's
/// stand-ins, and a replay with no recordings. Nothing is kept, no code
/// runs and no model is called.
pub fn carry_out_headless(tree: &Tree, request: &StudioRequest) -> Result<String, String> {
    match request {
        StudioRequest::Run { scenario, mode } => {
            let digest = agq_simulation::digest::model_digest(tree, *scenario);
            let answers = match mode {
                Mode::Model | Mode::Walkthrough => agq_simulation::Answers::StandIns,
                Mode::Replay => agq_simulation::Answers::Recordings(std::sync::Arc::new(
                    agq_simulation::Recordings::default(),
                )),
                _ => return Err("Not run: there is no code to run here.".into()),
            };
            let program = agq_simulation::compile(tree, *scenario).map_err(|blockers| {
                let reasons: Vec<String> = blockers.into_iter().map(|b| b.message).collect();
                format!("The scenario cannot start: {}", reasons.join("; "))
            })?;
            let result = agq_simulation::run(
                &program,
                digest,
                &agq_simulation::Request::new(*mode),
                answers,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            );
            Ok(result.describe(None, 12))
        }
        StudioRequest::StopRun => Ok("Nothing is running.".into()),
        StudioRequest::Observe { .. } | StudioRequest::Act { .. } => {
            Err("There is no application to operate here.".into())
        }
        StudioRequest::ReadRun { .. } => Err("No results are kept here; run the scenario.".into()),
        StudioRequest::ReadCodeLinks { .. } => Ok("No code is linked here.".into()),
        StudioRequest::Explain { element } => agq_implementation::responsibility::responsibility(
            tree,
            &agq_implementation::Links::default(),
            &[],
            *element,
        )
        .map(|r| r.describe(tree))
        .ok_or_else(|| "Only a part def or a part can be explained.".into()),
        StudioRequest::CheckImplementation => Err("Not run: there is no code here.".into()),
        StudioRequest::Implement { .. } => Err("Not started: there is no code folder here.".into()),
        StudioRequest::ProposeObjective(_) => {
            Err("Not shown: there is no Operator here to start it; nothing started.".into())
        }
    }
}
