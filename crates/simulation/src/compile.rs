//! Compiling a scenario and the model it runs over into disposable runtime
//! structures (ROADMAP §4.14): the scenario's steps, stand-ins and checks;
//! and, for model execution, the instances of the subject's parts with their
//! attribute slots, ports, routes and state machines.
//!
//! Everything is copied out of the model when the run starts, so the run
//! owns its snapshot and editing the model changes nothing under it. What
//! keeps a scenario from running is reported as [`Blocker`]s at the elements
//! concerned, never guessed around.

use crate::eval::{Expr, FieldSlot};
use crate::value::Value;
use agq_language::{
    Direction, Element, ElementId, ElementKind, Expression, Literal, Reference, Role, Semantics,
    StateAction, Tree, validate,
};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Why a scenario cannot run, at an element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocker {
    pub element: ElementId,
    pub message: String,
}

/// How a stand-in answers one call (`Scenarios::Outcome`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Answer,
    Timeout,
    InvalidOutput,
    Refusal,
    ToolUnavailable,
}

impl Outcome {
    pub fn from_name(name: &str) -> Option<Outcome> {
        Some(match name {
            "answer" => Outcome::Answer,
            "timeout" => Outcome::Timeout,
            "invalidOutput" => Outcome::InvalidOutput,
            "refusal" => Outcome::Refusal,
            "toolUnavailable" => Outcome::ToolUnavailable,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Outcome::Answer => "answer",
            Outcome::Timeout => "timeout",
            Outcome::InvalidOutput => "invalidOutput",
            Outcome::Refusal => "refusal",
            Outcome::ToolUnavailable => "toolUnavailable",
        }
    }

    /// In plain words, for traces.
    pub fn words(self) -> &'static str {
        match self {
            Outcome::Answer => "answers",
            Outcome::Timeout => "does not answer in time",
            Outcome::InvalidOutput => "answers outside its contract",
            Outcome::Refusal => "refuses",
            Outcome::ToolUnavailable => "cannot reach a tool",
        }
    }
}

/// A path of features from the subject: `api.shorten`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    pub features: Vec<ElementId>,
    /// The names, joined with `.`, as the harness and messages name it.
    pub text: String,
}

/// A stand-in declared by the scenario.
#[derive(Clone, Debug, PartialEq)]
pub struct StandIn {
    pub element: ElementId,
    pub name: String,
    /// The part it answers for, from the subject.
    pub target: Path,
    pub call: Option<u64>,
    pub outcome: Outcome,
    pub latency_ms: u64,
    pub output: Option<Expr>,
}

/// One step of a scenario, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    /// Send a value into the subject through a port.
    Send {
        element: ElementId,
        port: Path,
        payload: Expr,
    },
    /// Wait for a value of `ty` to come out of the subject through a port;
    /// `binding` (the accept element) names it for later steps.
    Accept {
        element: ElementId,
        port: Path,
        ty: ElementId,
        type_name: String,
        binding: ElementId,
        name: String,
    },
    /// Let logical time pass.
    Wait { element: ElementId, duration: Expr },
    /// A check: `assert constraint`.
    Check {
        element: ElementId,
        name: String,
        expr: Expr,
        /// The constraint as the printer writes it.
        text: String,
        /// Whether it reads the subject's internal state (so only a runner
        /// that sees inside can evaluate it).
        internal: bool,
    },
}

impl Step {
    pub fn element(&self) -> ElementId {
        match self {
            Step::Send { element, .. }
            | Step::Accept { element, .. }
            | Step::Wait { element, .. }
            | Step::Check { element, .. } => *element,
        }
    }
}

/// A scenario, compiled.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub element: ElementId,
    pub name: String,
    pub qualified_name: String,
    pub doc: Option<String>,
    /// The subject feature, its name and its type.
    pub subject: ElementId,
    pub subject_name: String,
    pub subject_type: ElementId,
    pub subject_type_name: String,
    /// The requirements it verifies.
    pub verifies: Vec<(ElementId, String)>,
    pub stand_ins: Vec<StandIn>,
    pub steps: Vec<Step>,
}

/// What an item type looks like to a run: its fields in order.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemType {
    pub name: String,
    pub fields: Vec<ItemField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemField {
    pub slot: FieldSlot,
    /// The field's declared type, if any.
    pub ty: Option<ElementId>,
    /// Whether a value is required: no multiplicity (one) or a lower bound
    /// of at least one.
    pub required: bool,
}

/// What the run knows of the model's types: every definition's generals
/// (itself included), item types, enum values and names.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Types {
    generals: HashMap<ElementId, HashSet<ElementId>>,
    pub items: HashMap<ElementId, ItemType>,
    /// Enum def to its values (element, name).
    pub enums: HashMap<ElementId, Vec<(ElementId, String)>>,
    pub names: HashMap<ElementId, String>,
    pub kinds: HashMap<ElementId, ElementKind>,
    /// Well-known library elements.
    pub scalar: Scalars,
}

/// The built-in scalar types, by identity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scalars {
    pub boolean: Option<ElementId>,
    pub string: Option<ElementId>,
    pub real: Option<ElementId>,
    pub integer: Option<ElementId>,
    pub natural: Option<ElementId>,
    pub positive: Option<ElementId>,
}

impl Types {
    /// Whether `specific` is `general` or specialises it.
    pub fn specializes(&self, specific: ElementId, general: ElementId) -> bool {
        specific == general
            || self
                .generals
                .get(&specific)
                .is_some_and(|all| all.contains(&general))
    }

    pub fn name(&self, id: ElementId) -> String {
        self.names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    }

    /// Whether `value` fits a feature of type `ty`: the check an agent's
    /// output meets, and what a stand-in's output must be.
    pub fn fits(&self, value: &Value, ty: ElementId) -> Result<(), String> {
        let s = &self.scalar;
        let is = |t: Option<ElementId>| t.is_some_and(|t| self.specializes(ty, t) || ty == t);
        match value {
            Value::Null => Ok(()),
            Value::Bool(_) if is(s.boolean) => Ok(()),
            Value::Str(_) if is(s.string) => Ok(()),
            Value::Int(n) if is(s.positive) => (*n >= 1)
                .then_some(())
                .ok_or_else(|| format!("{n} is not a Positive")),
            Value::Int(n) if is(s.natural) => (*n >= 0)
                .then_some(())
                .ok_or_else(|| format!("{n} is not a Natural")),
            Value::Int(_) if is(s.integer) || is(s.real) => Ok(()),
            Value::Real(_) if is(s.real) && !is(s.integer) => Ok(()),
            Value::Enum { def, name, .. } => {
                if self.specializes(*def, ty) {
                    Ok(())
                } else {
                    Err(format!("`{name}` is not a value of `{}`", self.name(ty)))
                }
            }
            Value::Item(item) => {
                if !self.specializes(item.ty, ty) {
                    return Err(format!(
                        "a `{}` is not a `{}`",
                        item.type_name,
                        self.name(ty)
                    ));
                }
                if let Some(shape) = self.items.get(&item.ty) {
                    for field in &shape.fields {
                        let Some(given) = item.field(field.slot.feature) else {
                            continue;
                        };
                        if let Some(field_ty) = field.ty {
                            self.fits(&given.value, field_ty).map_err(|why| {
                                format!("`{}.{}`: {why}", item.type_name, field.slot.name)
                            })?;
                        }
                    }
                }
                Ok(())
            }
            other => {
                // A type the run does not know (a user attribute def): take it.
                if [
                    s.boolean, s.string, s.real, s.integer, s.natural, s.positive,
                ]
                .into_iter()
                .flatten()
                .any(|t| self.specializes(ty, t) || ty == t)
                {
                    Err(format!("{} is not a `{}`", other.kind(), self.name(ty)))
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// A part instance in a model run.
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    /// The part usage (the subject for the root).
    pub usage: ElementId,
    pub name: String,
    /// `service.dispatcher`; the root is the subject's name.
    pub path: String,
    pub types: Vec<ElementId>,
    pub parent: Option<usize>,
    /// Child instances by the part feature they stand for (and its aliases).
    pub children: Vec<(Vec<ElementId>, usize)>,
    pub slots: Vec<Slot>,
    pub ports: Vec<usize>,
    pub behaviour: Behaviour,
    /// Set for the fallback of an agent: the agent's instance.
    pub fallback_of: Option<usize>,
}

/// An attribute of an instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub feature: ElementId,
    pub aliases: Vec<ElementId>,
    pub name: String,
    pub init: Option<Expr>,
}

/// What an instance does when a message reaches it.
#[derive(Clone, Debug, PartialEq)]
pub enum Behaviour {
    /// Nothing: a message reaching it stops the run (`missing-behaviour`).
    None,
    /// A state machine (index into [`System::machines`]).
    Machine(usize),
    /// An agent: its model answers, checked against its contract.
    Agent(Agent),
    /// Not runnable: why.
    Unsupported(String),
}

/// An agent at run time. Its settings are read from its slots when it is
/// called, so a usage's own values apply.
#[derive(Clone, Debug, PartialEq)]
pub struct Agent {
    /// Its instructions for a live model: the agent's documentation.
    pub instructions: String,
    /// The fallback's instance, when it has one.
    pub fallback: Option<usize>,
    /// The item type its answers must have (the `out` item of its output port).
    pub output_type: Option<ElementId>,
}

/// A port of an instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub instance: usize,
    pub feature: ElementId,
    pub aliases: Vec<ElementId>,
    pub name: String,
    /// `instance path.port name`
    pub path: String,
    /// Directed features as seen from outside the port.
    pub directed: Vec<(String, Direction, Option<ElementId>)>,
}

/// How two ports are joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// Two parts' ports face each other.
    Facing,
    /// `outer` is a port of the part that contains the other port's part:
    /// items pass through in both directions.
    Delegation { outer: usize },
    /// An agent's port and the same port of its fallback: the fallback's
    /// outputs leave through the agent; inputs reach the fallback only when
    /// the agent fails.
    Fallback { agent_port: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub a: usize,
    pub b: usize,
    pub kind: LinkKind,
    /// The connection or interface usage, for highlights.
    pub element: Option<ElementId>,
}

/// A compiled state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Machine {
    pub element: ElementId,
    pub entry: Vec<Action>,
    pub initial: Option<usize>,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub element: ElementId,
    pub name: String,
    pub entry: Vec<Action>,
    pub exit: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub element: ElementId,
    pub source: usize,
    pub target: usize,
    pub trigger: Trigger,
    pub guard: Option<Expr>,
    pub effect: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Trigger {
    /// Taken as soon as the state is entered and its guard holds.
    Completion,
    /// A payload arriving through a port.
    Accept {
        payload: ElementId,
        ty: Option<ElementId>,
        port: Option<ElementId>,
    },
    /// Logical time passing in the source state.
    After(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Send {
        element: ElementId,
        port: ElementId,
        payload: Expr,
    },
    Assign {
        element: ElementId,
        target: Vec<ElementId>,
        text: String,
        value: Expr,
    },
    If {
        element: ElementId,
        condition: Expr,
        then: Vec<Action>,
        otherwise: Vec<Action>,
    },
}

/// The subject's parts, compiled for model execution.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct System {
    pub instances: Vec<Instance>,
    pub ports: Vec<Port>,
    pub links: Vec<Link>,
    pub machines: Vec<Machine>,
}

/// Well-known elements of the built-in libraries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Library {
    pub agent: Option<ElementId>,
    pub agent_output: Option<ElementId>,
    pub confidence: Option<ElementId>,
    pub fallback: Option<ElementId>,
    pub mode: Option<ElementId>,
    pub model: Option<ElementId>,
    pub min_confidence: Option<ElementId>,
    pub max_latency: Option<ElementId>,
    pub stand_in: Option<ElementId>,
    pub stand_in_target: Option<ElementId>,
    pub stand_in_call: Option<ElementId>,
    pub stand_in_outcome: Option<ElementId>,
    pub stand_in_latency: Option<ElementId>,
    pub stand_in_output: Option<ElementId>,
}

/// A scenario ready to run: its steps, its model snapshot and, when the
/// model can run, the system.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub scenario: Scenario,
    pub types: Types,
    pub library: Library,
    /// The subject's parts; `Err` holds what keeps model execution from
    /// running (a scenario can still run against an implementation).
    pub system: Result<System, Vec<Blocker>>,
    /// Qualified names of every element the run may name, for traces.
    pub names: BTreeMap<ElementId, String>,
}

/// Compiles the scenario (a verification def) for running. `Err` lists
/// what keeps the scenario itself from running anywhere; problems that only
/// keep model execution from running are in [`Program::system`].
pub fn compile(tree: &Tree, scenario: ElementId) -> Result<Program, Vec<Blocker>> {
    let semantics = Semantics::new(tree);
    let mut compiler = Compiler {
        tree,
        semantics: &semantics,
        types: Types::default(),
        library: Library::default(),
        blockers: Vec::new(),
        names: BTreeMap::new(),
    };
    compiler.library();
    compiler.scalars();
    let compiled = compiler.scenario(scenario);
    let Some(compiled) = compiled else {
        return Err(compiler.blockers);
    };
    // Problems the language core reports at elements the run depends on
    // keep it from running; problems elsewhere in the model do not.
    let closure = crate::digest::closure(tree, scenario);
    let problems: Vec<Blocker> = validate(tree)
        .into_iter()
        .filter(|d| closure.contains(&d.element))
        .map(|d| Blocker {
            element: d.element,
            message: format!("{} ({})", d.message, d.code),
        })
        .collect();
    if !problems.is_empty() || !compiler.blockers.is_empty() {
        let mut all = compiler.blockers;
        all.extend(problems);
        return Err(all);
    }
    let system = compiler.system(&compiled);
    Ok(Program {
        scenario: compiled,
        types: compiler.types,
        library: compiler.library,
        system,
        names: compiler.names,
    })
}

struct Compiler<'a> {
    tree: &'a Tree,
    semantics: &'a Semantics<'a>,
    types: Types,
    library: Library,
    blockers: Vec<Blocker>,
    names: BTreeMap<ElementId, String>,
}

impl<'a> Compiler<'a> {
    fn block(&mut self, element: ElementId, message: impl Into<String>) {
        self.blockers.push(Blocker {
            element,
            message: message.into(),
        });
    }

    fn element(&self, id: ElementId) -> Option<&'a Element> {
        self.semantics.element(id)
    }

    fn name(&mut self, id: ElementId) -> String {
        let name = self
            .semantics
            .name(id)
            .map(str::to_string)
            .unwrap_or_else(|| id.to_string());
        self.names
            .entry(id)
            .or_insert_with(|| self.semantics.qualified_name(id));
        name
    }

    fn library(&mut self) {
        let s = self.semantics;
        let feature = |def: Option<ElementId>, name: &str| {
            def.and_then(|def| {
                s.features(def)
                    .into_iter()
                    .find(|f| s.name(*f) == Some(name))
            })
        };
        let agent = s.resolve("Agents::Agent");
        let output = s.resolve("Agents::AgentOutput");
        let stand_in = s.resolve("Scenarios::StandIn");
        self.library = Library {
            agent,
            agent_output: output,
            confidence: feature(output, "confidence"),
            fallback: feature(agent, "fallback"),
            mode: feature(agent, "mode"),
            model: feature(agent, "model"),
            min_confidence: feature(agent, "minConfidence"),
            max_latency: feature(agent, "maxLatencyMs"),
            stand_in,
            stand_in_target: feature(stand_in, "target"),
            stand_in_call: feature(stand_in, "call"),
            stand_in_outcome: feature(stand_in, "outcome"),
            stand_in_latency: feature(stand_in, "latencyMs"),
            stand_in_output: feature(stand_in, "output"),
        };
    }

    fn scalars(&mut self) {
        let s = self.semantics;
        let get = |name: &str| s.resolve(&format!("ScalarValues::{name}"));
        self.types.scalar = Scalars {
            boolean: get("Boolean"),
            string: get("String"),
            real: get("Real"),
            integer: get("Integer"),
            natural: get("Natural"),
            positive: get("Positive"),
        };
        for id in [
            self.types.scalar.boolean,
            self.types.scalar.string,
            self.types.scalar.real,
            self.types.scalar.integer,
            self.types.scalar.natural,
            self.types.scalar.positive,
        ]
        .into_iter()
        .flatten()
        {
            self.learn_type(id);
        }
    }

    /// Records a definition's generals, name, kind and (for item, part and
    /// attribute defs) fields, and for enum defs their values.
    fn learn_type(&mut self, id: ElementId) {
        if self.types.kinds.contains_key(&id) {
            return;
        }
        let Some(element) = self.element(id) else {
            return;
        };
        let kind = element.kind;
        self.types.kinds.insert(id, kind);
        let name = self.name(id);
        self.types.names.insert(id, name.clone());
        let mut all = HashSet::new();
        let mut pending = vec![id];
        while let Some(next) = pending.pop() {
            for general in self.semantics.generals(next) {
                if all.insert(general) {
                    pending.push(general);
                }
            }
        }
        for general in all.clone() {
            self.learn_type(general);
        }
        self.types.generals.insert(id, all);
        match kind {
            ElementKind::EnumDef => {
                let values: Vec<(ElementId, String)> = self
                    .semantics
                    .features(id)
                    .into_iter()
                    .filter(|f| {
                        self.element(*f)
                            .is_some_and(|e| e.kind == ElementKind::Enum)
                    })
                    .map(|f| (f, self.semantics.name(f).unwrap_or("?").to_string()))
                    .collect();
                self.types.enums.insert(id, values);
            }
            ElementKind::ItemDef | ElementKind::PartDef | ElementKind::AttributeDef => {
                let mut fields = Vec::new();
                for feature in self.semantics.features(id) {
                    let Some(e) = self.element(feature) else {
                        continue;
                    };
                    if !matches!(
                        e.kind,
                        ElementKind::Attribute | ElementKind::Item | ElementKind::Reference
                    ) {
                        continue;
                    }
                    let ty = self.semantics.types_of(feature).first().map(|(t, _)| *t);
                    if let Some(ty) = ty {
                        self.learn_type(ty);
                    }
                    let default = self.literal_value(e);
                    let required = e.multiplicity.is_none_or(|m| m.lower >= 1);
                    fields.push(ItemField {
                        required,
                        slot: FieldSlot {
                            feature,
                            aliases: self.aliases(feature),
                            name: self.name(feature),
                            default,
                        },
                        ty,
                    });
                }
                // A value type with no features (a scalar) is not an item.
                if kind != ElementKind::AttributeDef || !fields.is_empty() {
                    self.types.items.insert(id, ItemType { name, fields });
                }
            }
            _ => {}
        }
    }

    /// A feature and every feature it redefines, transitively.
    fn aliases(&self, feature: ElementId) -> Vec<ElementId> {
        let mut out = vec![feature];
        let mut i = 0;
        while i < out.len() {
            for redefined in self.semantics.redefined(out[i]) {
                if !out.contains(&redefined) {
                    out.push(redefined);
                }
            }
            i += 1;
        }
        out
    }

    fn literal_value(&self, element: &Element) -> Option<Value> {
        element.value.as_ref().map(literal)
    }

    // ---- expressions ----

    /// Compiles an expression written in `holder`.
    fn expr(&mut self, holder: ElementId, expression: &Expression) -> Result<Expr, String> {
        Ok(match expression {
            Expression::Literal(l) => Expr::Const(literal(l)),
            Expression::Null => Expr::Const(Value::Null),
            Expression::Name(reference) => {
                let steps = self.steps(holder, Role::Value, reference)?;
                if let [only] = steps[..]
                    && let Some(value) = self.enum_value(only)
                {
                    return Ok(Expr::Const(value));
                }
                for step in &steps {
                    self.name(*step);
                }
                Expr::Path {
                    steps,
                    text: reference.to_string(),
                }
            }
            Expression::Unary(op, operand) => {
                Expr::Unary(*op, Box::new(self.expr(holder, operand)?))
            }
            Expression::Binary(op, left, right) => Expr::Binary(
                *op,
                Box::new(self.expr(holder, left)?),
                Box::new(self.expr(holder, right)?),
            ),
            Expression::Conditional(c, a, b) => Expr::Cond(
                Box::new(self.expr(holder, c)?),
                Box::new(self.expr(holder, a)?),
                Box::new(self.expr(holder, b)?),
            ),
            Expression::New { ty, arguments } => {
                let ty = *self
                    .steps(holder, Role::Value, ty)?
                    .last()
                    .ok_or("`new` needs a type")?;
                self.learn_type(ty);
                let shape = self.types.items.get(&ty).cloned().ok_or_else(|| {
                    format!("`new {}` does not make an item", self.types.name(ty))
                })?;
                let mut given: Vec<(ElementId, Expr)> = Vec::new();
                for argument in arguments {
                    let feature = *self
                        .steps(holder, Role::Value, &argument.feature)?
                        .last()
                        .ok_or("an argument needs a feature")?;
                    given.push((feature, self.expr(holder, &argument.value)?));
                }
                let mut fields = Vec::new();
                for field in shape.fields {
                    let value = given
                        .iter()
                        .position(|(f, _)| {
                            field.slot.aliases.contains(f) || *f == field.slot.feature
                        })
                        .map(|i| given.remove(i).1);
                    fields.push((field.slot, value));
                }
                if let Some((feature, _)) = given.first() {
                    return Err(format!(
                        "`{}` is not a field of `{}`",
                        self.types.name(*feature),
                        shape.name
                    ));
                }
                Expr::New {
                    ty,
                    type_name: shape.name,
                    fields,
                }
            }
        })
    }

    fn steps(
        &mut self,
        holder: ElementId,
        role: Role,
        reference: &Reference,
    ) -> Result<Vec<ElementId>, String> {
        self.semantics
            .steps(holder, role, reference)
            .map_err(|why| format!("`{reference}`: {why}"))
    }

    fn enum_value(&mut self, id: ElementId) -> Option<Value> {
        let element = self.element(id)?;
        if element.kind != ElementKind::Enum {
            return None;
        }
        let def = element.owner()?;
        if self.element(def)?.kind != ElementKind::EnumDef {
            return None;
        }
        self.learn_type(def);
        Some(Value::Enum {
            def,
            value: id,
            name: self.name(id),
        })
    }

    /// The value a feature has in its text: a literal or an expression.
    fn value_of(&mut self, feature: ElementId) -> Result<Option<Expr>, String> {
        let Some(element) = self.element(feature) else {
            return Ok(None);
        };
        if let Some(literal_value) = &element.value {
            return Ok(Some(Expr::Const(literal(literal_value))));
        }
        match &element.expression {
            Some(expression) => self.expr(feature, expression).map(Some),
            None => Ok(None),
        }
    }

    // ---- the scenario ----

    fn scenario(&mut self, id: ElementId) -> Option<Scenario> {
        let Some(element) = self.element(id) else {
            self.block(id, "the scenario no longer exists");
            return None;
        };
        if element.kind != ElementKind::VerificationDef {
            self.block(id, "a scenario is a verification def");
            return None;
        }
        let name = self.name(id);
        let qualified_name = self.semantics.qualified_name(id);
        let children = element.children().to_vec();
        let doc = children
            .iter()
            .filter_map(|c| self.element(*c))
            .find(|c| c.kind == ElementKind::Doc)
            .and_then(|d| d.text.clone());
        let subjects: Vec<ElementId> = self
            .semantics
            .features(id)
            .into_iter()
            .filter(|f| {
                self.element(*f)
                    .is_some_and(|e| e.kind == ElementKind::Subject)
            })
            .collect();
        let Some(&subject) = subjects.first() else {
            self.block(
                id,
                "the scenario has no subject: say which system it examines",
            );
            return None;
        };
        let Some(&(subject_type, _)) = self.semantics.types_of(subject).first() else {
            self.block(subject, "the subject has no type: say which part def it is");
            return None;
        };
        if self.element(subject_type).map(|e| e.kind) != Some(ElementKind::PartDef) {
            self.block(subject, "the subject of a scenario that runs is a part");
            return None;
        }
        let subject_name = self.name(subject);
        let subject_type_name = self.name(subject_type);
        let mut verifies = Vec::new();
        let objectives: Vec<ElementId> = children
            .iter()
            .copied()
            .filter(|c| {
                self.element(*c)
                    .is_some_and(|e| e.kind == ElementKind::Objective)
            })
            .collect();
        for objective in objectives {
            for verify in self
                .element(objective)
                .map(|e| e.children().to_vec())
                .unwrap_or_default()
            {
                let Some(target) = self.element(verify).and_then(|e| e.target.clone()) else {
                    continue;
                };
                if let Ok(steps) = self.steps(verify, Role::Target, &target)
                    && let Some(requirement) = steps.last()
                {
                    let name = self.semantics.qualified_name(*requirement);
                    verifies.push((*requirement, name));
                }
            }
        }
        let mut stand_ins = Vec::new();
        let mut steps = Vec::new();
        let mut bindings = HashSet::new();
        for child in children {
            let Some(e) = self.element(child) else {
                continue;
            };
            match e.kind {
                ElementKind::Part => {
                    if let Some(stand_in) = self.stand_in(child, subject) {
                        stand_ins.push(stand_in);
                    }
                }
                ElementKind::Action if e.state_action.is_none() => {
                    self.block(
                        child,
                        "a scenario's steps are written one by one, not grouped in an action",
                    );
                }
                ElementKind::Send | ElementKind::Accept | ElementKind::AssertConstraint => {
                    if let Some(step) = self.step(child, subject, &bindings) {
                        if let Step::Accept { binding, .. } = &step {
                            bindings.insert(*binding);
                        }
                        steps.push(step);
                    }
                }
                ElementKind::If | ElementKind::Assign => {
                    self.block(
                        child,
                        "a scenario step is a send, an accept, a wait or a check",
                    );
                }
                _ => {}
            }
        }
        Some(Scenario {
            element: id,
            name,
            qualified_name,
            doc,
            subject,
            subject_name,
            subject_type,
            subject_type_name,
            verifies,
            stand_ins,
            steps,
        })
    }

    /// A path from the subject to a feature: `service.api.shorten` read as
    /// `api.shorten`.
    fn path(
        &mut self,
        steps: &[ElementId],
        subject: ElementId,
        at: ElementId,
        text: &str,
    ) -> Option<Path> {
        if steps.first() != Some(&subject) {
            self.block(
                at,
                format!("`{text}` does not start at the scenario's subject"),
            );
            return None;
        }
        let features = steps[1..].to_vec();
        let names: Vec<String> = features.iter().map(|f| self.name(*f)).collect();
        Some(Path {
            features,
            text: names.join("."),
        })
    }

    fn step(
        &mut self,
        id: ElementId,
        subject: ElementId,
        bindings: &HashSet<ElementId>,
    ) -> Option<Step> {
        let element = self.element(id)?.clone();
        match element.kind {
            ElementKind::Send => {
                let via = element.via.clone()?;
                let steps = match self.steps(id, Role::Via, &via) {
                    Ok(steps) => steps,
                    Err(why) => {
                        self.block(id, why);
                        return None;
                    }
                };
                let port = self.path(&steps, subject, id, &via.to_string())?;
                let payload = match element.expression.as_ref().map(|e| self.expr(id, e)) {
                    Some(Ok(payload)) => payload,
                    Some(Err(why)) => {
                        self.block(id, why);
                        return None;
                    }
                    None => {
                        self.block(id, "the step does not say what it sends");
                        return None;
                    }
                };
                Some(Step::Send {
                    element: id,
                    port,
                    payload,
                })
            }
            ElementKind::Accept if element.after => {
                let duration = match element.expression.as_ref().map(|e| self.expr(id, e)) {
                    Some(Ok(duration)) => duration,
                    Some(Err(why)) => {
                        self.block(id, why);
                        return None;
                    }
                    None => return None,
                };
                Some(Step::Wait {
                    element: id,
                    duration,
                })
            }
            ElementKind::Accept => {
                let via = element.via.clone()?;
                let steps = match self.steps(id, Role::Via, &via) {
                    Ok(steps) => steps,
                    Err(why) => {
                        self.block(id, why);
                        return None;
                    }
                };
                let port = self.path(&steps, subject, id, &via.to_string())?;
                let ty = element
                    .typed_by
                    .first()
                    .and_then(|r| self.steps(id, Role::TypedBy, r).ok())
                    .and_then(|s| s.last().copied());
                let Some(ty) = ty else {
                    self.block(id, "the step does not say what type it waits for");
                    return None;
                };
                self.learn_type(ty);
                let type_name = self.types.name(ty);
                let name = element.name.clone().unwrap_or_else(|| type_name.clone());
                Some(Step::Accept {
                    element: id,
                    port,
                    ty,
                    type_name,
                    binding: id,
                    name,
                })
            }
            ElementKind::AssertConstraint => {
                let expression = element.expression.clone()?;
                let expr = match self.expr(id, &expression) {
                    Ok(expr) => expr,
                    Err(why) => {
                        self.block(id, why);
                        return None;
                    }
                };
                let mut internal = false;
                expression_paths(&expr, &mut |steps| {
                    if steps.first() == Some(&subject) {
                        internal = true;
                    }
                });
                let _ = bindings;
                let name = element.name.clone().unwrap_or_else(|| "check".into());
                Some(Step::Check {
                    element: id,
                    name,
                    expr,
                    text: agq_language::print_expression(self.tree, id, &expression),
                    internal,
                })
            }
            _ => None,
        }
    }

    /// A usage of `Scenarios::StandIn` (or a specialisation of it).
    fn stand_in(&mut self, id: ElementId, subject: ElementId) -> Option<StandIn> {
        let stand_in_def = self.library.stand_in?;
        let is_stand_in = self
            .semantics
            .types_of(id)
            .iter()
            .any(|(t, _)| self.semantics.specializes(*t, stand_in_def));
        if !is_stand_in {
            return None;
        }
        let name = self.name(id);
        let features = self.semantics.features(id);
        let find = |this: &Self, wanted: Option<ElementId>| {
            let wanted = wanted?;
            features
                .iter()
                .copied()
                .find(|f| *f == wanted || this.aliases(*f).contains(&wanted))
        };
        let target_feature = find(self, self.library.stand_in_target);
        let target_expr = target_feature
            .and_then(|f| self.element(f))
            .and_then(|e| e.expression.clone());
        let Some(Expression::Name(reference)) = target_expr else {
            self.block(
                id,
                format!("the stand-in `{name}` does not say which part it answers for (`target`)"),
            );
            return None;
        };
        let holder = target_feature.expect("found");
        let steps = match self.steps(holder, Role::Value, &reference) {
            Ok(steps) => steps,
            Err(why) => {
                self.block(id, why);
                return None;
            }
        };
        let target = self.path(&steps, subject, id, &reference.to_string())?;
        let number = |this: &mut Self, feature: Option<ElementId>| -> Option<u64> {
            let value = this.value_of(feature?).ok()??;
            match value {
                Expr::Const(Value::Int(n)) if n >= 0 => Some(n as u64),
                _ => None,
            }
        };
        let call_feature = find(self, self.library.stand_in_call);
        let call = number(self, call_feature);
        let latency_feature = find(self, self.library.stand_in_latency);
        let latency_ms = number(self, latency_feature).unwrap_or(0);
        let outcome_feature = find(self, self.library.stand_in_outcome);
        let outcome = outcome_feature
            .and_then(|f| self.value_of(f).ok().flatten())
            .and_then(|expr| match expr {
                Expr::Const(Value::Enum { name, .. }) => Outcome::from_name(&name),
                _ => None,
            });
        let Some(outcome) = outcome else {
            self.block(
                id,
                format!("the stand-in `{name}` does not say how it answers (`outcome`)"),
            );
            return None;
        };
        let output_feature = find(self, self.library.stand_in_output);
        let output = match output_feature.map(|f| self.value_of(f)) {
            Some(Ok(output)) => output,
            Some(Err(why)) => {
                self.block(id, why);
                return None;
            }
            None => None,
        };
        if outcome == Outcome::Answer && output.is_none() {
            self.block(
                id,
                format!("the stand-in `{name}` answers, but does not say with what (`output`)"),
            );
            return None;
        }
        Some(StandIn {
            element: id,
            name,
            target,
            call,
            outcome,
            latency_ms,
            output,
        })
    }

    // ---- the system ----

    fn system(&mut self, scenario: &Scenario) -> Result<System, Vec<Blocker>> {
        let before = self.blockers.len();
        let mut system = System::default();
        let mut machines: HashMap<ElementId, usize> = HashMap::new();
        self.instantiate(
            &mut system,
            &mut machines,
            scenario.subject,
            scenario.subject_name.clone(),
            None,
            0,
        );
        // Stand-ins must name parts of the subject; agent-only outcomes only
        // agents.
        for stand_in in &scenario.stand_ins {
            match find_instance(&system, 0, &stand_in.target.features) {
                None => self.block(
                    stand_in.element,
                    format!("`{}` is not a part of the subject", stand_in.target.text),
                ),
                Some(instance) => {
                    let agent = matches!(system.instances[instance].behaviour, Behaviour::Agent(_));
                    if !agent && !matches!(stand_in.outcome, Outcome::Answer | Outcome::Timeout) {
                        self.block(
                            stand_in.element,
                            format!(
                                "`{}` is not an agent, so it can only answer or time out, not `{}`",
                                stand_in.target.text,
                                stand_in.outcome.name()
                            ),
                        );
                    }
                }
            }
        }
        if self.blockers.len() > before {
            return Err(self.blockers.split_off(before));
        }
        Ok(system)
    }

    /// Creates the instance for `usage` and, recursively, its parts, ports,
    /// connections and behaviour. Returns its index.
    fn instantiate(
        &mut self,
        system: &mut System,
        machines: &mut HashMap<ElementId, usize>,
        usage: ElementId,
        path: String,
        parent: Option<usize>,
        depth: usize,
    ) -> usize {
        let index = system.instances.len();
        let name = self.name(usage);
        let types: Vec<ElementId> = self
            .semantics
            .types_of(usage)
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        for ty in &types {
            self.learn_type(*ty);
        }
        system.instances.push(Instance {
            usage,
            name,
            path: path.clone(),
            types: types.clone(),
            parent,
            children: Vec::new(),
            slots: Vec::new(),
            ports: Vec::new(),
            behaviour: Behaviour::None,
            fallback_of: None,
        });
        if depth > 24 {
            self.block(usage, "parts nest too deeply to run");
            return index;
        }
        let features = self.semantics.features(usage);
        let mut connections = Vec::new();
        for feature in features {
            let Some(element) = self.element(feature) else {
                continue;
            };
            match element.kind {
                ElementKind::Part => {
                    if self.semantics.types_of(feature).is_empty() {
                        continue; // an untyped part (an agent without a fallback)
                    }
                    if let Some(m) = element.multiplicity
                        && (m.upper == Some(0) || m.lower > 1)
                    {
                        if m.lower > 1 {
                            self.block(
                                feature,
                                "a part with more than one instance cannot run yet; give it the multiplicity 1",
                            );
                        }
                        continue;
                    }
                    let child_path = format!("{path}.{}", self.name(feature));
                    let child = self.instantiate(
                        system,
                        machines,
                        feature,
                        child_path,
                        Some(index),
                        depth + 1,
                    );
                    let aliases = self.aliases(feature);
                    system.instances[index].children.push((aliases, child));
                }
                ElementKind::Port => {
                    let port = system.ports.len();
                    let port_name = self.name(feature);
                    let directed = self.semantics.directed_features(feature);
                    for (_, _, ty) in &directed {
                        if let Some(ty) = ty {
                            self.learn_type(*ty);
                        }
                    }
                    system.ports.push(Port {
                        instance: index,
                        feature,
                        aliases: self.aliases(feature),
                        name: port_name.clone(),
                        path: format!("{path}.{port_name}"),
                        directed,
                    });
                    system.instances[index].ports.push(port);
                }
                ElementKind::Attribute | ElementKind::Reference | ElementKind::Item => {
                    let init = match self.value_of(feature) {
                        Ok(init) => init,
                        Err(why) => {
                            self.block(feature, why);
                            None
                        }
                    };
                    let slot = Slot {
                        feature,
                        aliases: self.aliases(feature),
                        name: self.name(feature),
                        init,
                    };
                    system.instances[index].slots.push(slot);
                }
                _ => {}
            }
        }
        // Connections are often unnamed, so they are found as members of the
        // usage and its generals rather than by name.
        for owner in self.generals_in_order(usage) {
            let Some(element) = self.element(owner) else {
                continue;
            };
            for child in element.children() {
                if self.element(*child).is_some_and(|c| {
                    matches!(c.kind, ElementKind::Connection | ElementKind::Interface)
                        && c.ends.len() == 2
                }) && !connections.contains(child)
                {
                    connections.push(*child);
                }
            }
        }
        for connection in connections {
            self.connect(system, index, connection);
        }
        let behaviour = self.behaviour(system, machines, index);
        system.instances[index].behaviour = behaviour;
        index
    }

    /// Joins the ports a connection or interface of `owner` names.
    fn connect(&mut self, system: &mut System, owner: usize, connection: ElementId) {
        let Some(element) = self.element(connection) else {
            return;
        };
        let ends = element.ends.clone();
        let mut ports = Vec::new();
        for end in &ends {
            let steps = match self.steps(connection, Role::End, end) {
                Ok(steps) => steps,
                Err(why) => {
                    self.block(connection, why);
                    return;
                }
            };
            let (instance_steps, port_step) = steps.split_at(steps.len() - 1);
            let Some(instance) = find_instance(system, owner, instance_steps) else {
                self.block(
                    connection,
                    format!("`{end}` does not lead to a part that runs"),
                );
                return;
            };
            let found = system.instances[instance]
                .ports
                .iter()
                .copied()
                .find(|p| system.ports[*p].aliases.contains(&port_step[0]));
            let Some(port) = found else {
                // A connection between parts or items (not ports) carries nothing.
                return;
            };
            ports.push((port, instance_steps.is_empty()));
        }
        let [(a, a_outer), (b, b_outer)] = ports[..] else {
            return;
        };
        let kind = match (a_outer, b_outer) {
            (true, false) => LinkKind::Delegation { outer: a },
            (false, true) => LinkKind::Delegation { outer: b },
            _ => LinkKind::Facing,
        };
        system.links.push(Link {
            a,
            b,
            kind,
            element: Some(connection),
        });
    }

    /// What an instance does: an agent, its exhibit state machine, or nothing.
    fn behaviour(
        &mut self,
        system: &mut System,
        machines: &mut HashMap<ElementId, usize>,
        index: usize,
    ) -> Behaviour {
        let usage = system.instances[index].usage;
        let types = system.instances[index].types.clone();
        if let Some(agent) = self.library.agent
            && types.iter().any(|t| self.semantics.specializes(*t, agent))
        {
            return self.agent(system, index);
        }
        let exhibits = self.exhibit_states(usage);
        match exhibits[..] {
            [] => Behaviour::None,
            [state] => {
                if let Some(machine) = machines.get(&state) {
                    return Behaviour::Machine(*machine);
                }
                match self.machine(state) {
                    Ok(machine) => {
                        system.machines.push(machine);
                        let at = system.machines.len() - 1;
                        machines.insert(state, at);
                        Behaviour::Machine(at)
                    }
                    Err(why) => Behaviour::Unsupported(why),
                }
            }
            _ => Behaviour::Unsupported(
                "it performs several behaviours; running more than one exhibit state per part is not supported".into(),
            ),
        }
    }

    /// The exhibit states a usage performs: its own and those of its types
    /// and their generals, minus any another one redefines.
    /// A usage and its generals (types, specialisations, subsetted and
    /// redefined features), transitively, nearest first.
    fn generals_in_order(&self, usage: ElementId) -> Vec<ElementId> {
        let mut order = vec![usage];
        let mut i = 0;
        while i < order.len() {
            for general in self.semantics.generals(order[i]) {
                if !order.contains(&general) {
                    order.push(general);
                }
            }
            i += 1;
        }
        order
    }

    fn exhibit_states(&self, usage: ElementId) -> Vec<ElementId> {
        let order = self.generals_in_order(usage);
        let mut found: Vec<ElementId> = Vec::new();
        for owner in order {
            let Some(element) = self.element(owner) else {
                continue;
            };
            for child in element.children() {
                if self
                    .element(*child)
                    .is_some_and(|e| e.kind == ElementKind::State && e.exhibit)
                {
                    found.push(*child);
                }
            }
        }
        let redefined: HashSet<ElementId> = found
            .iter()
            .flat_map(|s| self.aliases(*s).into_iter().skip(1))
            .collect();
        found.retain(|s| !redefined.contains(s));
        found
    }

    fn agent(&mut self, system: &mut System, index: usize) -> Behaviour {
        let usage = system.instances[index].usage;
        let types = system.instances[index].types.clone();
        let instructions = types
            .iter()
            .filter_map(|t| self.element(*t))
            .flat_map(|e| e.children().to_vec())
            .filter_map(|c| self.element(c))
            .find(|c| c.kind == ElementKind::Doc)
            .and_then(|d| d.text.clone())
            .unwrap_or_default();
        // The fallback is the child that stands for the (redefined) `fallback`.
        let fallback = self.library.fallback.and_then(|fallback| {
            system.instances[index]
                .children
                .iter()
                .find(|(aliases, _)| aliases.contains(&fallback))
                .map(|(_, child)| *child)
        });
        if let Some(fallback) = fallback {
            system.instances[fallback].fallback_of = Some(index);
            // The fallback's ports stand behind the agent's ports of the same name.
            let agent_ports = system.instances[index].ports.clone();
            for port in system.instances[fallback].ports.clone() {
                let name = system.ports[port].name.clone();
                if let Some(agent_port) = agent_ports
                    .iter()
                    .copied()
                    .find(|p| system.ports[*p].name == name)
                {
                    system.links.push(Link {
                        a: agent_port,
                        b: port,
                        kind: LinkKind::Fallback { agent_port },
                        element: None,
                    });
                }
            }
        }
        let output_type = system.instances[index]
            .ports
            .iter()
            .flat_map(|p| system.ports[*p].directed.clone())
            .find(|(_, direction, _)| *direction == Direction::Out)
            .and_then(|(_, _, ty)| ty);
        let _ = usage;
        Behaviour::Agent(Agent {
            instructions,
            fallback,
            output_type,
        })
    }

    /// Compiles an exhibit state into a flat state machine.
    fn machine(&mut self, id: ElementId) -> Result<Machine, String> {
        let element = self.element(id).ok_or("the behaviour no longer exists")?;
        let children = element.children().to_vec();
        let mut machine = Machine {
            element: id,
            entry: Vec::new(),
            initial: None,
            states: Vec::new(),
            transitions: Vec::new(),
        };
        let mut index: HashMap<ElementId, usize> = HashMap::new();
        for child in &children {
            let Some(e) = self.element(*child) else {
                continue;
            };
            if e.kind == ElementKind::State {
                if e.children().iter().any(|c| {
                    self.element(*c)
                        .is_some_and(|c| c.kind == ElementKind::State)
                }) {
                    return Err(format!(
                        "state `{}` has states inside; nested state machines cannot run yet",
                        self.name(*child)
                    ));
                }
                let mut state = State {
                    element: *child,
                    name: self.name(*child),
                    entry: Vec::new(),
                    exit: Vec::new(),
                };
                for action in e.children().to_vec() {
                    let Some(a) = self.element(action) else {
                        continue;
                    };
                    match a.state_action {
                        Some(StateAction::Entry) => state.entry = self.actions(action)?,
                        Some(StateAction::Exit) => state.exit = self.actions(action)?,
                        Some(StateAction::Do) => {
                            return Err(format!(
                                "state `{}` has a `do` action; ongoing actions cannot run yet",
                                state.name
                            ));
                        }
                        None => {}
                    }
                }
                index.insert(*child, machine.states.len());
                machine.states.push(state);
            }
        }
        for child in &children {
            let Some(e) = self.element(*child).cloned() else {
                continue;
            };
            match (e.kind, e.state_action) {
                (
                    ElementKind::Action | ElementKind::Send | ElementKind::Assign,
                    Some(StateAction::Entry),
                ) => {
                    machine.entry = self.actions(*child)?;
                }
                (_, Some(StateAction::Exit | StateAction::Do)) => {
                    return Err(
                        "the state machine itself has an exit or do action, which cannot run yet"
                            .into(),
                    );
                }
                (ElementKind::Succession, _) => {
                    let target = e.target.clone().ok_or("`then` names no state")?;
                    let steps = self.steps(*child, Role::Target, &target)?;
                    machine.initial = steps.last().and_then(|s| index.get(s).copied());
                }
                (ElementKind::Transition, _) => {
                    let transition = self.transition(*child, &e, &index)?;
                    machine.transitions.push(transition);
                }
                _ => {}
            }
        }
        if machine.initial.is_none() && !machine.states.is_empty() {
            return Err("the state machine does not say which state comes first".into());
        }
        Ok(machine)
    }

    fn transition(
        &mut self,
        id: ElementId,
        element: &Element,
        index: &HashMap<ElementId, usize>,
    ) -> Result<Transition, String> {
        let mut ends = Vec::new();
        for end in &element.ends {
            let steps = self.steps(id, Role::End, end)?;
            let state = steps
                .last()
                .and_then(|s| index.get(s).copied())
                .ok_or_else(|| format!("`{end}` is not a state of this machine"))?;
            ends.push(state);
        }
        let [source, target] = ends[..] else {
            return Err("a transition has a source and a target state".into());
        };
        let mut trigger = Trigger::Completion;
        let mut effect = Vec::new();
        for child in element.children().to_vec() {
            let Some(c) = self.element(child).cloned() else {
                continue;
            };
            match c.kind {
                ElementKind::Accept if c.after => {
                    let duration = c
                        .expression
                        .as_ref()
                        .ok_or("`accept after` needs a duration")?;
                    trigger = Trigger::After(self.expr(child, duration)?);
                }
                ElementKind::Accept => {
                    let ty = match c.typed_by.first() {
                        Some(r) => self.steps(child, Role::TypedBy, r)?.last().copied(),
                        None => None,
                    };
                    if let Some(ty) = ty {
                        self.learn_type(ty);
                    }
                    let port = match &c.via {
                        Some(via) => self.steps(child, Role::Via, via)?.last().copied(),
                        None => None,
                    };
                    trigger = Trigger::Accept {
                        payload: child,
                        ty,
                        port,
                    };
                }
                kind if kind.is_action_node() => effect = self.actions(child)?,
                _ => {}
            }
        }
        let guard = match &element.guard {
            Some(guard) => Some(self.expr(id, guard)?),
            None => None,
        };
        Ok(Transition {
            element: id,
            source,
            target,
            trigger,
            guard,
            effect,
        })
    }

    /// The actions of an action node: itself, or an action's members in order.
    fn actions(&mut self, id: ElementId) -> Result<Vec<Action>, String> {
        let element = self
            .element(id)
            .ok_or("the action no longer exists")?
            .clone();
        Ok(match element.kind {
            ElementKind::Action => {
                let mut out = Vec::new();
                for child in element.children().to_vec() {
                    if self.element(child).is_some_and(|c| c.kind.is_action_node()) {
                        out.extend(self.actions(child)?);
                    }
                }
                out
            }
            ElementKind::Send => {
                let via = element.via.as_ref().ok_or("`send` needs `via port`")?;
                let port = *self
                    .steps(id, Role::Via, via)?
                    .last()
                    .ok_or("`send` needs a port")?;
                let payload = element.expression.as_ref().ok_or("`send` needs a value")?;
                vec![Action::Send {
                    element: id,
                    port,
                    payload: self.expr(id, payload)?,
                }]
            }
            ElementKind::Assign => {
                let target = element.target.as_ref().ok_or("`assign` needs a feature")?;
                let steps = self.steps(id, Role::Target, target)?;
                let value = element
                    .expression
                    .as_ref()
                    .ok_or("`assign` needs a value")?;
                vec![Action::Assign {
                    element: id,
                    target: steps,
                    text: target.to_string(),
                    value: self.expr(id, value)?,
                }]
            }
            ElementKind::If => {
                let condition = element
                    .expression
                    .as_ref()
                    .ok_or("`if` needs a condition")?;
                let condition = self.expr(id, condition)?;
                let branches = element.children().to_vec();
                let then = match branches.first() {
                    Some(b) => self.actions(*b)?,
                    None => Vec::new(),
                };
                let otherwise = match branches.get(1) {
                    Some(b) => self.actions(*b)?,
                    None => Vec::new(),
                };
                vec![Action::If {
                    element: id,
                    condition,
                    then,
                    otherwise,
                }]
            }
            ElementKind::Accept => {
                return Err("waiting (`accept`) inside a part's action cannot run yet; use a state and a transition".into());
            }
            _ => Vec::new(),
        })
    }
}

/// The instance a path of part features leads to from `from`.
pub(crate) fn find_instance(system: &System, from: usize, features: &[ElementId]) -> Option<usize> {
    let mut current = from;
    for feature in features {
        current = system.instances[current]
            .children
            .iter()
            .find(|(aliases, _)| aliases.contains(feature))
            .map(|(_, child)| *child)?;
    }
    Some(current)
}

/// Calls `visit` on the steps of every path in an expression.
pub(crate) fn expression_paths(expr: &Expr, visit: &mut dyn FnMut(&[ElementId])) {
    match expr {
        Expr::Const(_) => {}
        Expr::Path { steps, .. } => visit(steps),
        Expr::Unary(_, operand) => expression_paths(operand, visit),
        Expr::Binary(_, a, b) => {
            expression_paths(a, visit);
            expression_paths(b, visit);
        }
        Expr::Cond(c, a, b) => {
            expression_paths(c, visit);
            expression_paths(a, visit);
            expression_paths(b, visit);
        }
        Expr::New { fields, .. } => {
            for (_, value) in fields {
                if let Some(value) = value {
                    expression_paths(value, visit);
                }
            }
        }
    }
}

/// A literal as a run-time value. Escapes in a string are resolved.
pub(crate) fn literal(literal: &Literal) -> Value {
    match literal {
        Literal::Boolean(b) => Value::Bool(*b),
        Literal::Integer(text) => text
            .parse::<i64>()
            .map(Value::Int)
            .unwrap_or_else(|_| Value::Real(text.parse().unwrap_or(f64::NAN))),
        Literal::Real(text) => Value::Real(text.parse().unwrap_or(f64::NAN)),
        Literal::String(text) => {
            let mut out = String::with_capacity(text.len());
            let mut chars = text.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some(other) => out.push(other),
                        None => {}
                    }
                } else {
                    out.push(c);
                }
            }
            Value::Str(out)
        }
    }
}
