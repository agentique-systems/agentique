//! Requirement evaluation (C-55, ROADMAP §4.14): what a requirement's
//! constraints say about the modelled configuration of what satisfies it.
//!
//! For `satisfy r by x`, the subject of `r` is bound to `x`, and `r` is
//! evaluated on the configuration the model gives `x`: its parts and their
//! attribute values as written, a usage's redefinitions winning over its
//! definitions, attribute expressions (a roll-up such as
//! `frame.mass + battery.mass`) worked out through feature chains. No
//! behaviour runs and no time passes. The effective constraint is the
//! standard's implication (SysML 7.21.2): if every assumed constraint is
//! true, every required constraint and every subrequirement (its subject
//! bound to the container's unless it binds its own) must be true; where it
//! is stricter than the standard is deviation 21. Members count whatever
//! their visibility, and a requirement that contains itself is not
//! evaluable.
//!
//! The result is a calculation on the model, not a test of a built system,
//! and never says more than was calculated: an informal constraint, a value
//! the model does not determine, or a construct this evaluator does not
//! support make it *not evaluable*, never a pass. It is computed from the
//! present model when asked, so it is never stale; the digest of the model
//! slice it read says what it depended on. Nothing is stored.

use crate::compile::ExpressionCompiler;
use crate::digest::{closure, model_digest};
use crate::eval::{Env, EvalError, Expr, eval};
use crate::value::Value;
use agq_language::{
    BinaryOp, Diagnostic, ElementId, ElementKind, Expression, Multiplicity, Role, Semantics, Tree,
    print_expression, printed_reference, validate,
};
use std::cell::RefCell;
use std::collections::HashMap;

/// What a requirement's evaluation concludes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Every assumption is true, and every required constraint and every
    /// subrequirement that applies is true (at least one applies).
    Holds,
    /// The assumptions are true and a required constraint is false.
    Violated,
    /// An assumption is false, or none of what it requires applies: the
    /// requirement claims nothing here.
    AssumptionsNotMet,
    /// It could not be calculated: an informal constraint, a value the model
    /// does not determine, or something unsupported (the reason says which).
    NotEvaluable,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Holds => "holds",
            Status::Violated => "violated",
            Status::AssumptionsNotMet => "assumptions not met",
            Status::NotEvaluable => "not evaluable",
        }
    }
}

/// What a constraint is to its requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintKind {
    /// `assume constraint`
    Assumption,
    /// `require constraint`
    Required,
    /// A nested requirement usage: required with its container.
    Subrequirement,
}

impl ConstraintKind {
    pub fn label(self) -> &'static str {
        match self {
            ConstraintKind::Assumption => "assumption",
            ConstraintKind::Required => "required constraint",
            ConstraintKind::Subrequirement => "subrequirement",
        }
    }
}

/// A constraint's value on the configuration.
#[derive(Clone, Debug, PartialEq)]
pub enum Truth {
    True,
    False,
    /// A subrequirement whose assumptions are not met: it claims nothing
    /// here, so it neither satisfies nor violates its container.
    DoesNotApply,
    /// Why it could not be calculated.
    NotEvaluable(String),
}

/// One assumed or required constraint, or a subrequirement, evaluated.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintResult {
    pub element: ElementId,
    pub kind: ConstraintKind,
    /// The expression as written (`s.mass <= limit`); an informal
    /// constraint's name or doc; a subrequirement's name.
    pub text: String,
    pub truth: Truth,
    /// The values used, as written and as calculated: the operands of a
    /// comparison, then every name read.
    pub values: Vec<(String, String)>,
    /// A subrequirement's own evaluation.
    pub subrequirement: Option<Box<Evaluation>>,
}

/// A requirement evaluated on the modelled configuration of its subject.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub requirement: ElementId,
    /// The `satisfy` that bound the subject (none for a subrequirement).
    pub satisfy: Option<ElementId>,
    /// What the subject is bound to, as written (`drone`, `fleet.scout`).
    pub subject: String,
    pub status: Status,
    /// Why, in one line: which constraint, with its values.
    pub reason: String,
    pub assumptions: Vec<ConstraintResult>,
    pub required: Vec<ConstraintResult>,
    /// The digest of the model slice the evaluation read
    /// ([`crate::digest::model_digest`] of the `satisfy`, or of the part
    /// or part def it is written in when the configuration is built there).
    pub digest: String,
}

/// Every `satisfy` of the model evaluated, in document order.
pub fn evaluate_all(tree: &Tree) -> Vec<Evaluation> {
    let any = tree
        .walk()
        .into_iter()
        .any(|id| tree[id].kind == ElementKind::Satisfy);
    if !any {
        return Vec::new();
    }
    evaluate_satisfies(tree, &validate(tree))
}

/// [`evaluate_all`] with the model's diagnostics already worked out (the
/// System State keeps them).
pub fn evaluate_satisfies(tree: &Tree, diagnostics: &[Diagnostic]) -> Vec<Evaluation> {
    let satisfies: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::Satisfy)
        .collect();
    if satisfies.is_empty() {
        return Vec::new();
    }
    let semantics = Semantics::new(tree);
    satisfies
        .into_iter()
        .map(|satisfy| evaluate_with(tree, &semantics, diagnostics, satisfy))
        .collect()
}

/// One `satisfy` evaluated, or `None` when `satisfy` is not one.
pub fn evaluate(tree: &Tree, satisfy: ElementId) -> Option<Evaluation> {
    if tree.get(satisfy)?.kind != ElementKind::Satisfy {
        return None;
    }
    let diagnostics = validate(tree);
    let semantics = Semantics::new(tree);
    Some(evaluate_with(tree, &semantics, &diagnostics, satisfy))
}

impl Evaluation {
    /// This evaluation and those of its subrequirements, depth first.
    pub fn walk(&self) -> Vec<&Evaluation> {
        let mut out = vec![self];
        for result in &self.required {
            if let Some(sub) = &result.subrequirement {
                out.extend(sub.walk());
            }
        }
        out
    }

    /// The evaluation in words: the conclusion, then each constraint with
    /// its values, subrequirements indented, and the model slice's digest.
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        self.lines(0, &mut lines);
        lines.push(format!(
            "  It read the model slice with digest {}.",
            short_digest(&self.digest)
        ));
        lines.join("\n")
    }

    fn lines(&self, depth: usize, out: &mut Vec<String>) {
        let indent = "  ".repeat(depth);
        out.push(format!(
            "{indent}{}: {} (subject `{}`).",
            self.status.label(),
            self.reason,
            self.subject
        ));
        for result in self.assumptions.iter().chain(&self.required) {
            out.push(format!("{indent}  {}", result.line()));
            if let Some(sub) = &result.subrequirement {
                sub.lines(depth + 2, out);
            }
        }
    }
}

impl ConstraintResult {
    /// `required constraint `s.mass <= limit`: false (s.mass = 7400.0, limit = 7000)`
    pub fn line(&self) -> String {
        let truth = match &self.truth {
            Truth::True => "true".to_string(),
            Truth::False => "false".to_string(),
            Truth::DoesNotApply => "does not apply (its assumptions are not met)".to_string(),
            Truth::NotEvaluable(why) => format!("not evaluable: {why}"),
        };
        let values = values_text(&self.values);
        let values = if values.is_empty()
            || matches!(self.truth, Truth::NotEvaluable(_) | Truth::DoesNotApply)
        {
            String::new()
        } else {
            format!(" ({values})")
        };
        format!("{} `{}`: {truth}{values}", self.kind.label(), self.text)
    }
}

/// The first twelve hex digits, enough to tell slices apart in text.
pub fn short_digest(digest: &str) -> &str {
    &digest[..digest.len().min(12)]
}

fn values_text(values: &[(String, String)]) -> String {
    values
        .iter()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn evaluate_with(
    tree: &Tree,
    semantics: &Semantics,
    diagnostics: &[Diagnostic],
    satisfy: ElementId,
) -> Evaluation {
    let element = &tree[satisfy];
    let subject = element
        .by
        .as_ref()
        .map(|by| printed_reference(tree, satisfy, Role::By, by).to_string())
        .unwrap_or_default();
    let requirement = element
        .target
        .as_ref()
        .and_then(|t| semantics.steps(satisfy, Role::Target, t).ok())
        .and_then(|steps| steps.last().copied());
    let steps = element
        .by
        .as_ref()
        .and_then(|by| semantics.steps(satisfy, Role::By, by).ok())
        .filter(|steps| !steps.is_empty());
    // A feature is configured in its context: the part or part def the
    // satisfy is written in, when the feature belongs to it. The slice read
    // is then the context's.
    let context = element.owner().filter(|owner| {
        matches!(tree[*owner].kind, ElementKind::PartDef | ElementKind::Part)
            && steps
                .as_ref()
                .is_some_and(|steps| semantics.features(*owner).contains(&steps[0]))
    });
    let root = context.unwrap_or(satisfy);
    let digest = model_digest(tree, root);
    let unevaluated = |requirement: ElementId, reason: String| Evaluation {
        requirement,
        satisfy: Some(satisfy),
        subject: subject.clone(),
        status: Status::NotEvaluable,
        reason,
        assumptions: Vec::new(),
        required: Vec::new(),
        digest: digest.clone(),
    };
    let Some(requirement) = requirement else {
        return unevaluated(
            satisfy,
            "the requirement it names cannot be found".to_string(),
        );
    };
    // Problems the language core reports where the evaluation would read
    // keep it from being calculated, as they keep a scenario from running.
    let slice = closure(tree, root);
    let problems: Vec<String> = diagnostics
        .iter()
        .filter(|d| slice.contains(&d.element))
        .map(|d| {
            format!(
                "{}: {} ({})",
                tree.qualified_name(d.element),
                d.message,
                d.code
            )
        })
        .collect();
    if !problems.is_empty() {
        return unevaluated(
            requirement,
            format!(
                "the model has problems where it would be read: {}",
                problems.join("; ")
            ),
        );
    }
    let Some(by) = &element.by else {
        return unevaluated(
            requirement,
            "the satisfy names no satisfying feature (`by`)".to_string(),
        );
    };
    let Some(steps) = steps else {
        return unevaluated(
            requirement,
            format!("the satisfying feature `{by}` cannot be found"),
        );
    };
    let mut compiler = ExpressionCompiler::new(tree, semantics);
    let mut config = Config {
        semantics,
        nodes: Vec::new(),
        memo: RefCell::default(),
        refs: Vec::new(),
    };
    let (start, path) = match context {
        Some(owner) => (config.build(&mut compiler, owner, None, 0), &steps[..]),
        None => {
            // The satisfying feature itself is one part, as each part read
            // through is.
            let aliases = compiler.aliases(steps[0]);
            if let Some(why) = multiplicity_problem(&subject, multiplicity(semantics, &aliases)) {
                return unevaluated(requirement, why);
            }
            (config.build(&mut compiler, steps[0], None, 0), &steps[1..])
        }
    };
    config.bind_refs();
    let node = match config.node_at(start, path) {
        Ok(node) => node,
        Err(why) => return unevaluated(requirement, why),
    };
    let mut evaluator = Evaluator {
        tree,
        semantics,
        compiler,
        digest: &digest,
        subrequirements: HashMap::new(),
        recursive: HashMap::new(),
    };
    // A subject the requirement binds itself must be what the satisfy binds.
    if let Some(why) = evaluator.binding_conflict(requirement, &steps, &subject) {
        return unevaluated(requirement, why);
    }
    let mut evaluation = evaluator.requirement(&config, requirement, Ok(node), &subject, None, 0);
    evaluation.satisfy = Some(satisfy);
    evaluation
}

// ---- members, owned and inherited ----

/// A member of a namespace: owned, or inherited from one of its generals.
struct Member {
    id: ElementId,
    /// The member and every feature it redefines.
    aliases: Vec<ElementId>,
    /// What it is ([`kind_of`]).
    kind: ElementKind,
    /// Where its owner comes in the generals, nearest first (0: owned).
    rank: usize,
}

/// `id` and its generals, transitively, nearest first.
fn generals_in_order(semantics: &Semantics, id: ElementId) -> Vec<ElementId> {
    let mut order = vec![id];
    let mut i = 0;
    while i < order.len() {
        for general in semantics.generals(order[i]) {
            if !order.contains(&general) {
                order.push(general);
            }
        }
        i += 1;
    }
    order
}

/// Every member of a namespace, owned and inherited, whatever its
/// visibility (a private attribute or subrequirement is still part of what
/// is calculated): those of it and its generals, the nearest first, without
/// the ones a nearer member redefines.
fn members(
    semantics: &Semantics,
    compiler: &ExpressionCompiler,
    namespace: ElementId,
) -> Vec<Member> {
    let mut out: Vec<Member> = Vec::new();
    for (rank, owner) in generals_in_order(semantics, namespace)
        .into_iter()
        .enumerate()
    {
        let Some(element) = semantics.element(owner) else {
            continue;
        };
        for child in element.children() {
            if out.iter().any(|m| m.aliases.contains(child)) {
                continue;
            }
            let Some(c) = semantics.element(*child) else {
                continue;
            };
            let aliases = compiler.aliases(*child);
            let kind = kind_of(semantics, &aliases).unwrap_or(c.kind);
            out.push(Member {
                id: *child,
                aliases,
                kind,
                rank,
            });
        }
    }
    out
}

/// The multiplicity a feature has: its own or that of a feature it
/// redefines.
fn multiplicity(semantics: &Semantics, aliases: &[ElementId]) -> Option<Multiplicity> {
    aliases
        .iter()
        .find_map(|a| semantics.element(*a).and_then(|e| e.multiplicity))
}

/// Why a part with this multiplicity cannot be read through: a calculation
/// reads exactly one part.
fn multiplicity_problem(path: &str, multiplicity: Option<Multiplicity>) -> Option<String> {
    match multiplicity {
        None
        | Some(Multiplicity {
            lower: 1,
            upper: Some(1),
        }) => None,
        Some(m) => Some(format!(
            "`{path}` has the multiplicity {m}; a calculation reads through exactly one part"
        )),
    }
}

// ---- the modelled configuration ----

/// A feature that holds a value: an attribute, a reference usage, an item.
struct Slot {
    aliases: Vec<ElementId>,
    /// `drone.battery.mass`, for messages.
    path: String,
    /// The value as written: `Ok(None)` when the model gives it none.
    init: Result<Option<Expr>, String>,
}

/// What a part feature of a node leads to.
enum Child {
    Node(usize),
    /// Why a calculation cannot read through it.
    Unsupported(String),
}

/// One part of the configuration.
struct Node {
    usage: ElementId,
    path: String,
    parent: Option<usize>,
    children: Vec<(Vec<ElementId>, Child)>,
    slots: Vec<Slot>,
}

/// The configuration a feature's usage gives it: its parts, as nodes, with
/// their values worked out on demand, each at most once.
struct Config<'a> {
    semantics: &'a Semantics<'a>,
    nodes: Vec<Node>,
    /// `(node, slot)` to its value; `None` while it is being worked out.
    memo: RefCell<HashMap<(usize, usize), Memo>>,
    /// Referential parts met while building, bound once every node exists.
    refs: Vec<PendingRef>,
}

/// A referential part (`ref part supply : PowerBus = bus;`) of a node,
/// waiting to be bound: it is the part its value leads to, never a part of
/// its own (C-55, as model execution runs it).
struct PendingRef {
    owner: usize,
    aliases: Vec<ElementId>,
    path: String,
    /// The value's feature chain and its text, or why it leads nowhere.
    value: Result<(Vec<ElementId>, String), String>,
}

/// A value worked out once: `None` while it is being worked out.
type Memo = Option<Result<Value, String>>;

/// What a path read from a node found.
enum Found {
    Value(Value, usize),
    /// Its first step is not a feature of the node.
    NotHere,
    Problem(String),
}

/// How deep parts may nest before the configuration stops.
const MAX_DEPTH: usize = 24;

impl Config<'_> {
    fn name(&self, id: ElementId) -> String {
        self.semantics
            .name(id)
            .map(str::to_string)
            .unwrap_or_else(|| id.to_string())
    }

    /// Adds the node for `usage` and, recursively, its parts.
    fn build(
        &mut self,
        compiler: &mut ExpressionCompiler,
        usage: ElementId,
        parent: Option<(usize, String)>,
        depth: usize,
    ) -> usize {
        let index = self.nodes.len();
        let (parent, path) = match parent {
            Some((parent, path)) => (Some(parent), path),
            None => (None, self.name(usage)),
        };
        self.nodes.push(Node {
            usage,
            path: path.clone(),
            parent,
            children: Vec::new(),
            slots: Vec::new(),
        });
        for member in members(self.semantics, compiler, usage) {
            let Some(name) = self.semantics.name(member.id) else {
                continue;
            };
            let feature_path = format!("{path}.{name}");
            match member.kind {
                ElementKind::Part if self.semantics.referential(member.id) == Some(true) => {
                    let value = self.reference_value(&member, &feature_path);
                    self.refs.push(PendingRef {
                        owner: index,
                        aliases: member.aliases,
                        path: feature_path,
                        value,
                    });
                }
                ElementKind::Part => {
                    let multiplicity = multiplicity(self.semantics, &member.aliases);
                    if multiplicity.is_some_and(|m| m.upper == Some(0)) {
                        continue;
                    }
                    let child = match multiplicity_problem(&feature_path, multiplicity) {
                        Some(why) => Child::Unsupported(why),
                        None if depth >= MAX_DEPTH => Child::Unsupported(format!(
                            "`{feature_path}`: parts nest too deeply to calculate"
                        )),
                        None => Child::Node(self.build(
                            compiler,
                            member.id,
                            Some((index, feature_path)),
                            depth + 1,
                        )),
                    };
                    self.nodes[index].children.push((member.aliases, child));
                }
                ElementKind::Attribute | ElementKind::Reference | ElementKind::Item => {
                    let init = compiler.value_of(member.id);
                    self.nodes[index].slots.push(Slot {
                        aliases: member.aliases,
                        path: feature_path,
                        init,
                    });
                }
                _ => {}
            }
        }
        index
    }

    /// What a referential part's value leads to: its own value or that of
    /// the nearest feature it redefines, a feature chain naming a part.
    fn reference_value(
        &self,
        member: &Member,
        path: &str,
    ) -> Result<(Vec<ElementId>, String), String> {
        if let Some(why) = multiplicity_problem(path, multiplicity(self.semantics, &member.aliases))
        {
            return Err(why);
        }
        let Some(holder) = self.semantics.value_holder(member.id) else {
            return Err(format!(
                "`{path}` is a reference bound to nothing in this configuration, so what it refers to is not determined"
            ));
        };
        let Some(Expression::Name(reference)) = self
            .semantics
            .element(holder)
            .and_then(|e| e.expression.clone())
        else {
            return Err(format!(
                "`{path}` is a reference whose value is not a feature chain naming a part"
            ));
        };
        // What it refers to has that part's features; its own would be
        // a second, different description of the same part.
        let own = member.aliases.iter().any(|a| {
            self.semantics.element(*a).is_some_and(|e| {
                e.children().iter().any(|c| {
                    self.semantics
                        .element(*c)
                        .is_some_and(|c| !matches!(c.kind, ElementKind::Doc | ElementKind::Comment))
                })
            })
        });
        if own {
            return Err(format!(
                "`{path}` refers to `{reference}` and declares features of its own; what it refers to has that part's features"
            ));
        }
        let steps = self.semantics.steps(holder, Role::Value, &reference)?;
        Ok((steps, reference.to_string()))
    }

    /// Binds each referential part to the node its value leads to, the
    /// value's first feature looked up from the part's owner outward, as
    /// its name is: the same node, so a shared part counts once. A value
    /// through another reference waits for that one; one that leads
    /// nowhere is not evaluable when read.
    fn bind_refs(&mut self) {
        let mut pending = std::mem::take(&mut self.refs);
        loop {
            let mut progress = false;
            let mut waiting = Vec::new();
            for r in pending {
                let child = match &r.value {
                    Err(why) => Some(Child::Unsupported(why.clone())),
                    Ok((steps, _)) => self.resolve(r.owner, steps).map(Child::Node),
                };
                match child {
                    Some(child) => {
                        self.nodes[r.owner].children.push((r.aliases, child));
                        progress = true;
                    }
                    None => waiting.push(r),
                }
            }
            pending = waiting;
            if !progress || pending.is_empty() {
                break;
            }
        }
        for r in pending {
            let text = r.value.map(|(_, text)| text).unwrap_or_default();
            let why = format!(
                "`{}` is bound to `{text}`, which does not lead to a part in this configuration",
                r.path
            );
            self.nodes[r.owner]
                .children
                .push((r.aliases, Child::Unsupported(why)));
        }
    }

    /// The node a referential part's value leads to.
    fn resolve(&self, owner: usize, steps: &[ElementId]) -> Option<usize> {
        let first = steps.first()?;
        let mut at = Some(owner);
        while let Some(current) = at {
            let node = &self.nodes[current];
            if node
                .children
                .iter()
                .any(|(aliases, _)| aliases.contains(first))
            {
                return self.node_at(current, steps).ok();
            }
            at = node.parent;
        }
        None
    }

    /// The node a path of part features leads to from `from`.
    fn node_at(&self, from: usize, path: &[ElementId]) -> Result<usize, String> {
        let mut current = from;
        for step in path {
            let node = &self.nodes[current];
            match node
                .children
                .iter()
                .find(|(aliases, _)| aliases.contains(step))
            {
                Some((_, Child::Node(child))) => current = *child,
                Some((_, Child::Unsupported(why))) => return Err(why.clone()),
                None => {
                    return Err(format!(
                        "`{}.{}` is not a part of the modelled configuration",
                        node.path,
                        self.name(*step)
                    ));
                }
            }
        }
        Ok(current)
    }

    /// The value of a slot, worked out once.
    fn slot_value(&self, node: usize, slot: usize) -> Result<Value, String> {
        let known = self.memo.borrow().get(&(node, slot)).cloned();
        let s = &self.nodes[node].slots[slot];
        match known {
            Some(Some(result)) => return result,
            Some(None) => return Err(format!("`{}` depends on its own value", s.path)),
            None => {}
        }
        self.memo.borrow_mut().insert((node, slot), None);
        let result = match &s.init {
            Err(why) => Err(format!("the value of `{}` cannot be read: {why}", s.path)),
            Ok(None) => Err(format!(
                "the value of `{}` is not determined (the model gives it none)",
                s.path
            )),
            Ok(Some(expr)) => eval(expr, &NodeEnv { config: self, node })
                .map_err(|e| format!("the value of `{}`: {e}", s.path)),
        };
        self.memo
            .borrow_mut()
            .insert((node, slot), Some(result.clone()));
        result
    }

    /// A path read from a node: through its parts to a value.
    fn find(&self, from: usize, steps: &[ElementId]) -> Found {
        let mut current = from;
        for (i, step) in steps.iter().enumerate() {
            let node = &self.nodes[current];
            if let Some(slot) = node.slots.iter().position(|s| s.aliases.contains(step)) {
                return match self.slot_value(current, slot) {
                    Ok(value) => Found::Value(value, i + 1),
                    Err(why) => Found::Problem(why),
                };
            }
            match node
                .children
                .iter()
                .find(|(aliases, _)| aliases.contains(step))
            {
                Some((_, Child::Node(child))) => current = *child,
                Some((_, Child::Unsupported(why))) => return Found::Problem(why.clone()),
                None if i == 0 => return Found::NotHere,
                None => {
                    return Found::Problem(format!(
                        "`{}` has no feature `{}` in the modelled configuration",
                        node.path,
                        self.name(*step)
                    ));
                }
            }
        }
        Found::Problem(format!(
            "`{}` is a part, not a value",
            self.nodes[current].path
        ))
    }
}

/// Names in a value written in a part: its own features, then those of
/// the parts around it (a usage's value may name its siblings).
struct NodeEnv<'c, 'a> {
    config: &'c Config<'a>,
    node: usize,
}

impl Env for NodeEnv<'_, '_> {
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
        let mut at = Some(self.node);
        while let Some(node) = at {
            match self.config.find(node, steps) {
                Found::Value(value, used) => return Ok((value, used)),
                Found::Problem(why) => return Err(EvalError(why)),
                Found::NotHere => at = self.config.nodes[node].parent,
            }
        }
        Err(EvalError(format!(
            "`{text}` is not a feature of the modelled configuration"
        )))
    }
}

// ---- the requirement ----

/// How deep subrequirements may nest.
const MAX_REQUIREMENT_DEPTH: usize = 16;

/// A requirement being evaluated: its subject, its attributes, and the
/// requirement around it (a subrequirement's names may lead there).
struct Context<'c, 'a> {
    config: &'c Config<'a>,
    /// The node the subject is bound to, or why it is not bound.
    subject: Result<usize, String>,
    subject_aliases: Vec<ElementId>,
    attributes: Vec<Slot>,
    memo: RefCell<HashMap<usize, Memo>>,
    /// Names read and their values, for the constraint being evaluated.
    reads: RefCell<Vec<(String, Value)>>,
    parent: Option<&'c Context<'c, 'a>>,
}

impl Context<'_, '_> {
    fn attribute(&self, index: usize) -> Result<Value, String> {
        let known = self.memo.borrow().get(&index).cloned();
        let slot = &self.attributes[index];
        match known {
            Some(Some(result)) => return result,
            Some(None) => return Err(format!("`{}` depends on its own value", slot.path)),
            None => {}
        }
        self.memo.borrow_mut().insert(index, None);
        let result = match &slot.init {
            Err(why) => Err(format!(
                "the value of `{}` cannot be read: {why}",
                slot.path
            )),
            Ok(None) => Err(format!(
                "the value of `{}` is not determined (the model gives it none)",
                slot.path
            )),
            Ok(Some(expr)) => {
                eval(expr, self).map_err(|e| format!("the value of `{}`: {e}", slot.path))
            }
        };
        self.memo.borrow_mut().insert(index, Some(result.clone()));
        result
    }

    fn read(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), String> {
        let first = steps
            .first()
            .ok_or_else(|| format!("`{text}` names nothing"))?;
        if self.subject_aliases.contains(first) {
            let node = self.subject.clone()?;
            if steps.len() == 1 {
                return Err(format!("`{text}` is the subject, a part, not a value"));
            }
            return match self.config.find(node, &steps[1..]) {
                Found::Value(value, used) => Ok((value, used + 1)),
                Found::Problem(why) => Err(why),
                Found::NotHere => Err(format!(
                    "`{}` has no feature `{}` in the modelled configuration",
                    self.config.nodes[node].path,
                    self.config.name(steps[1])
                )),
            };
        }
        if let Some(index) = self
            .attributes
            .iter()
            .position(|a| a.aliases.contains(first))
        {
            return self.attribute(index).map(|value| (value, 1));
        }
        match self.parent {
            Some(parent) => parent.read(steps, text),
            None => Err(format!(
                "`{text}` is neither the subject nor an attribute of the requirement, so it cannot be read"
            )),
        }
    }
}

impl Env for Context<'_, '_> {
    fn lookup(&self, steps: &[ElementId], text: &str) -> Result<(Value, usize), EvalError> {
        let found = self.read(steps, text).map_err(EvalError)?;
        if found.1 == steps.len() {
            self.reads
                .borrow_mut()
                .push((text.to_string(), found.0.clone()));
        }
        Ok(found)
    }
}

struct Evaluator<'a, 'd> {
    tree: &'a Tree,
    semantics: &'a Semantics<'a>,
    compiler: ExpressionCompiler<'a>,
    digest: &'d str,
    /// The subrequirements of each requirement worked out so far.
    subrequirements: HashMap<ElementId, Vec<ElementId>>,
    /// Whether a requirement contains itself, worked out once.
    recursive: HashMap<ElementId, bool>,
}

impl Evaluator<'_, '_> {
    fn name(&self, id: ElementId) -> String {
        self.semantics
            .name(id)
            .map(str::to_string)
            .unwrap_or_else(|| id.to_string())
    }

    /// The doc of an element as one line.
    fn doc(&self, id: ElementId) -> Option<String> {
        let element = self.semantics.element(id)?;
        element
            .children()
            .iter()
            .filter_map(|c| self.semantics.element(*c))
            .find(|c| c.kind == ElementKind::Doc)
            .and_then(|doc| doc.text.as_deref())
            .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    /// The subrequirements of a requirement, owned and inherited, the
    /// most general definition's first.
    fn subrequirements_of(&mut self, requirement: ElementId) -> Vec<ElementId> {
        if let Some(known) = self.subrequirements.get(&requirement) {
            return known.clone();
        }
        let mut found: Vec<Member> = members(self.semantics, &self.compiler, requirement)
            .into_iter()
            .filter(|m| m.kind == ElementKind::Requirement)
            .collect();
        found.sort_by_key(|m| std::cmp::Reverse(m.rank));
        let ids: Vec<ElementId> = found.into_iter().map(|m| m.id).collect();
        self.subrequirements.insert(requirement, ids.clone());
        ids
    }

    /// Whether a requirement is among its own subrequirements, at any depth:
    /// then evaluating it would never end.
    fn contains_itself(&mut self, requirement: ElementId) -> bool {
        if let Some(known) = self.recursive.get(&requirement) {
            return *known;
        }
        let mut seen = std::collections::HashSet::new();
        let mut pending = self.subrequirements_of(requirement);
        let mut found = false;
        while let Some(next) = pending.pop() {
            if next == requirement {
                found = true;
                break;
            }
            if seen.insert(next) {
                pending.extend(self.subrequirements_of(next));
            }
        }
        self.recursive.insert(requirement, found);
        found
    }

    /// Why the subject a requirement binds itself (`subject s = x;`)
    /// disagrees with what the satisfy binds, if it does.
    fn binding_conflict(
        &mut self,
        requirement: ElementId,
        satisfying: &[ElementId],
        satisfying_text: &str,
    ) -> Option<String> {
        let subject = members(self.semantics, &self.compiler, requirement)
            .into_iter()
            .find(|m| m.kind == ElementKind::Subject)?;
        let tree = self.tree;
        let element = tree.get(subject.id)?;
        let name = self.name(subject.id);
        if element.value.is_some() {
            return Some(format!(
                "its subject `{name}` is bound to a value, but the satisfy binds `{satisfying_text}`"
            ));
        }
        let expression = element.expression.as_ref()?;
        let text = print_expression(tree, subject.id, expression);
        match self.compiler.expr(subject.id, expression) {
            Ok(Expr::Path { steps, .. }) if steps == satisfying => None,
            _ => Some(format!(
                "its subject `{name}` is bound to `{text}` in the model, but the satisfy binds `{satisfying_text}`"
            )),
        }
    }

    /// Evaluates `requirement` with its subject bound to `subject`.
    fn requirement(
        &mut self,
        config: &Config,
        requirement: ElementId,
        subject: Result<usize, String>,
        subject_text: &str,
        parent: Option<&Context>,
        depth: usize,
    ) -> Evaluation {
        let requirement_name = self.name(requirement);
        let mut subject_aliases = Vec::new();
        let mut attributes = Vec::new();
        let mut constraints: Vec<Member> = Vec::new();
        for member in members(self.semantics, &self.compiler, requirement) {
            match member.kind {
                ElementKind::Subject if subject_aliases.is_empty() => {
                    subject_aliases = member.aliases;
                }
                ElementKind::Attribute | ElementKind::Reference | ElementKind::Item => {
                    let Some(name) = self.semantics.name(member.id) else {
                        continue;
                    };
                    attributes.push(Slot {
                        path: format!("{requirement_name}.{name}"),
                        init: self.compiler.value_of(member.id),
                        aliases: member.aliases,
                    });
                }
                kind if kind.is_requirement_constraint() => constraints.push(member),
                _ => {}
            }
        }
        // The most general definition's constraints first.
        constraints.sort_by_key(|m| std::cmp::Reverse(m.rank));
        let subrequirements = self.subrequirements_of(requirement);
        let context = Context {
            config,
            subject,
            subject_aliases,
            attributes,
            memo: RefCell::default(),
            reads: RefCell::default(),
            parent,
        };
        let mut assumptions = Vec::new();
        let mut required = Vec::new();
        for constraint in constraints {
            let result = self.constraint(&context, constraint.id);
            if result.kind == ConstraintKind::Assumption {
                assumptions.push(result);
            } else {
                required.push(result);
            }
        }
        for sub in &subrequirements {
            // Two inherited subrequirements with one name, neither
            // redefining the other: which is meant cannot be told.
            let name = self.semantics.name(*sub);
            let same: Vec<ElementId> = subrequirements
                .iter()
                .copied()
                .filter(|other| name.is_some() && self.semantics.name(*other) == name)
                .collect();
            if same.len() > 1 {
                if same[0] != *sub {
                    continue; // said once, at the first
                }
                let names: Vec<String> = same
                    .iter()
                    .map(|id| format!("`{}`", self.semantics.qualified_name(*id)))
                    .collect();
                required.push(ConstraintResult {
                    element: *sub,
                    kind: ConstraintKind::Subrequirement,
                    text: self.name(*sub),
                    truth: Truth::NotEvaluable(format!(
                        "its name is ambiguous: it names {}, and neither redefines the other",
                        names.join(" and ")
                    )),
                    values: Vec::new(),
                    subrequirement: None,
                });
                continue;
            }
            required.push(self.subrequirement(config, &context, *sub, subject_text, depth));
        }
        let (status, reason) = conclude(&assumptions, &required);
        Evaluation {
            requirement,
            satisfy: None,
            subject: subject_text.to_string(),
            status,
            reason,
            assumptions,
            required,
            digest: self.digest.to_string(),
        }
    }

    /// One assumed or required constraint on the context's configuration.
    fn constraint(&mut self, context: &Context, constraint: ElementId) -> ConstraintResult {
        let tree = self.tree;
        let element = &tree[constraint];
        let kind = if element.kind == ElementKind::AssumeConstraint {
            ConstraintKind::Assumption
        } else {
            ConstraintKind::Required
        };
        let mut result = ConstraintResult {
            element: constraint,
            kind,
            text: String::new(),
            truth: Truth::True,
            values: Vec::new(),
            subrequirement: None,
        };
        let Some(expression) = &element.expression else {
            let doc = self.doc(constraint);
            result.text = match (&element.name, &doc) {
                (Some(name), _) => name.clone(),
                (None, Some(doc)) => doc.clone(),
                (None, None) => "(no text)".into(),
            };
            result.truth = Truth::NotEvaluable(match (&element.name, doc) {
                (Some(_), Some(doc)) => format!(
                    "it is informal (\"{doc}\"): only its text says what it requires, so there is nothing to calculate"
                ),
                (None, Some(_)) => "it is informal: only its text says what it requires, so there is nothing to calculate".into(),
                (_, None) => "it is informal and has no text; there is nothing to calculate".into(),
            });
            return result;
        };
        result.text = print_expression(tree, constraint, expression);
        let compiled = match self.compiler.expr(constraint, expression) {
            Ok(compiled) => compiled,
            Err(why) => {
                result.truth = Truth::NotEvaluable(why);
                return result;
            }
        };
        context.reads.borrow_mut().clear();
        // The operands of a comparison, as written and as calculated.
        if let (Expr::Binary(op, left, right), Expression::Binary(_, left_text, right_text)) =
            (&compiled, expression)
            && is_comparison(*op)
        {
            for (side, text) in [(left, left_text), (right, right_text)] {
                if matches!(**side, Expr::Const(_)) {
                    continue; // a literal says its own value
                }
                if let Ok(value) = eval(side, context) {
                    result
                        .values
                        .push((print_expression(tree, constraint, text), value.to_string()));
                }
            }
        }
        let value = eval(&compiled, context);
        for (name, value) in context.reads.borrow_mut().drain(..) {
            if !result.values.iter().any(|(n, _)| *n == name) {
                result.values.push((name, value.to_string()));
            }
        }
        result.truth = match value {
            Ok(Value::Bool(true)) => Truth::True,
            Ok(Value::Bool(false)) => Truth::False,
            Ok(other) => Truth::NotEvaluable(format!("it is {}, not true or false", other.kind())),
            Err(why) => Truth::NotEvaluable(why.0),
        };
        result
    }

    /// A subrequirement, its subject bound to the container's unless it
    /// binds its own (`subject engine = vehicle.engine;`).
    fn subrequirement(
        &mut self,
        config: &Config,
        context: &Context,
        sub: ElementId,
        subject_text: &str,
        depth: usize,
    ) -> ConstraintResult {
        let mut result = ConstraintResult {
            element: sub,
            kind: ConstraintKind::Subrequirement,
            text: self.name(sub),
            truth: Truth::True,
            values: Vec::new(),
            subrequirement: None,
        };
        if self.contains_itself(sub) {
            result.truth = Truth::NotEvaluable(
                "it contains itself (it is among its own subrequirements), so its evaluation would never end"
                    .into(),
            );
            return result;
        }
        if depth + 1 > MAX_REQUIREMENT_DEPTH {
            result.truth = Truth::NotEvaluable("subrequirements nest too deeply".into());
            return result;
        }
        let subject_feature = members(self.semantics, &self.compiler, sub)
            .into_iter()
            .find(|m| m.kind == ElementKind::Subject)
            .map(|m| m.id);
        let own =
            subject_feature.filter(|f| self.tree.get(*f).is_some_and(|e| e.owner() == Some(sub)));
        let (subject, text) = match own {
            None => (context.subject.clone(), subject_text.to_string()),
            Some(feature) => self.bound_subject(config, context, feature),
        };
        // The subject must be of the subrequirement's subject type.
        let subject = subject.and_then(|node| {
            let Some(feature) = subject_feature else {
                return Ok(node);
            };
            let usage = config.nodes[node].usage;
            let actual: Vec<ElementId> = self
                .semantics
                .types_of(usage)
                .into_iter()
                .map(|(t, _)| t)
                .collect();
            for (expected, _) in self.semantics.types_of(feature) {
                if !actual
                    .iter()
                    .any(|a| self.semantics.specializes(*a, expected))
                {
                    return Err(format!(
                        "its subject `{}` is bound to `{}`, which is not a `{}`",
                        self.name(feature),
                        config.nodes[node].path,
                        self.name(expected)
                    ));
                }
            }
            Ok(node)
        });
        let evaluation = self.requirement(config, sub, subject, &text, Some(context), depth + 1);
        result.truth = match evaluation.status {
            Status::Holds => Truth::True,
            // Its assumptions are not met: it claims nothing here, so it
            // neither satisfies nor violates its container (deviation 21).
            Status::AssumptionsNotMet => Truth::DoesNotApply,
            Status::Violated => Truth::False,
            Status::NotEvaluable => Truth::NotEvaluable(evaluation.reason.clone()),
        };
        result.subrequirement = Some(Box::new(evaluation));
        result
    }

    /// A subrequirement's own subject: bound by its value to a part of the
    /// container's subject, or not bound at all.
    fn bound_subject(
        &mut self,
        config: &Config,
        context: &Context,
        feature: ElementId,
    ) -> (Result<usize, String>, String) {
        let name = self.name(feature);
        let Some(expression) = self.tree[feature].expression.clone() else {
            return (
                Err(format!(
                    "its subject `{name}` is declared but not bound to anything (bind it with `subject {name} = ...;`, or leave it out to share the container's subject)"
                )),
                name,
            );
        };
        let text = print_expression(self.tree, feature, &expression);
        let steps = match self.compiler.expr(feature, &expression) {
            Ok(Expr::Path { steps, .. }) => steps,
            Ok(_) => {
                return (
                    Err(format!(
                        "its subject is bound to `{text}`, which is not a part of the container's subject"
                    )),
                    text,
                );
            }
            Err(why) => return (Err(why), text),
        };
        let node = match steps.split_first() {
            Some((first, rest)) if context.subject_aliases.contains(first) => context
                .subject
                .clone()
                .and_then(|node| config.node_at(node, rest)),
            _ => Err(format!(
                "its subject is bound to `{text}`, which is not a part of the container's subject"
            )),
        };
        (node, text)
    }
}

/// What a feature is: its own kind or, for a usage written without a kind
/// keyword (`:>> x = ...;`), the kind of the feature it redefines.
fn kind_of(semantics: &Semantics, aliases: &[ElementId]) -> Option<ElementKind> {
    let kinds: Vec<ElementKind> = aliases
        .iter()
        .filter_map(|a| semantics.element(*a).map(|e| e.kind))
        .collect();
    kinds
        .iter()
        .copied()
        .find(|k| *k != ElementKind::Reference)
        .or(kinds.first().copied())
}

fn is_comparison(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessOrEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterOrEqual
    )
}

/// The implication (deviation 21 for what is stricter than the standard):
/// assumptions first, then the required constraints and subrequirements.
fn conclude(assumptions: &[ConstraintResult], required: &[ConstraintResult]) -> (Status, String) {
    let described = |result: &ConstraintResult| {
        let values = values_text(&result.values);
        let values = if values.is_empty() {
            String::new()
        } else {
            format!(" ({values})")
        };
        let label = result.kind.label();
        match &result.truth {
            Truth::NotEvaluable(why) => format!("the {label} `{}`: {why}", result.text),
            Truth::False if result.kind == ConstraintKind::Subrequirement => {
                let why = result
                    .subrequirement
                    .as_ref()
                    .map(|sub| sub.reason.clone())
                    .unwrap_or_default();
                format!("the subrequirement `{}` is violated: {why}", result.text)
            }
            Truth::DoesNotApply => format!(
                "the subrequirement `{}` does not apply (its assumptions are not met)",
                result.text
            ),
            _ => format!("the {label} `{}` is false{values}", result.text),
        }
    };
    let joined = |results: Vec<&ConstraintResult>| {
        results
            .into_iter()
            .map(described)
            .collect::<Vec<_>>()
            .join("; ")
    };
    let false_assumptions: Vec<&ConstraintResult> = assumptions
        .iter()
        .filter(|r| r.truth == Truth::False)
        .collect();
    if !false_assumptions.is_empty() {
        return (
            Status::AssumptionsNotMet,
            format!(
                "{}, so the requirement claims nothing for this configuration",
                joined(false_assumptions)
            ),
        );
    }
    let unknown_assumptions: Vec<&ConstraintResult> = assumptions
        .iter()
        .filter(|r| matches!(r.truth, Truth::NotEvaluable(_)))
        .collect();
    if !unknown_assumptions.is_empty() {
        return (
            Status::NotEvaluable,
            format!(
                "whether it applies is not known: {}",
                joined(unknown_assumptions)
            ),
        );
    }
    let violated: Vec<&ConstraintResult> = required
        .iter()
        .filter(|r| r.truth == Truth::False)
        .collect();
    if !violated.is_empty() {
        return (Status::Violated, joined(violated));
    }
    let unknown: Vec<&ConstraintResult> = required
        .iter()
        .filter(|r| matches!(r.truth, Truth::NotEvaluable(_)))
        .collect();
    if !unknown.is_empty() {
        return (Status::NotEvaluable, joined(unknown));
    }
    if required.is_empty() {
        return (
            Status::NotEvaluable,
            "it has no required constraint or subrequirement to calculate (its text alone is informal)"
                .into(),
        );
    }
    let not_applying: Vec<&ConstraintResult> = required
        .iter()
        .filter(|r| r.truth == Truth::DoesNotApply)
        .collect();
    let applying = required.len() - not_applying.len();
    if applying == 0 {
        return (
            Status::AssumptionsNotMet,
            format!(
                "none of what it requires applies: {}, so the requirement claims nothing for this configuration",
                joined(not_applying)
            ),
        );
    }
    let mut reason = if assumptions.is_empty() {
        format!("the {applying} required item(s) that apply are true")
    } else {
        format!(
            "its {} assumption(s) are true and the {applying} required item(s) that apply are true",
            assumptions.len()
        )
    };
    if !not_applying.is_empty() {
        reason.push_str(&format!(
            " ({} subrequirement(s) do not apply: their assumptions are not met)",
            not_applying.len()
        ));
    }
    (Status::Holds, reason)
}
