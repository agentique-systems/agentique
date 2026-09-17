//! AGQ-SEQ-01: pure preparation and atomic sequential steps. No I/O capabilities.
use agq_model::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CONTRACT: &str = "AGQ-SEQ-01";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub ordinal: u64,
    pub receiver_path: String,
    pub payload_type: String,
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Limits {
    pub max_input_deliveries: u64,
    pub max_semantic_steps: u64,
    pub max_trace_records: usize,
    #[serde(default = "memory_limit")]
    pub max_memory_bytes: usize,
}
fn memory_limit() -> usize {
    64 * 1024 * 1024
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_deliveries: 100,
            max_semantic_steps: 1000,
            max_trace_records: 5000,
            max_memory_bytes: memory_limit(),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Expected {
    #[serde(default)]
    pub active_states: Vec<String>,
    #[serde(default)]
    pub transition_names: Vec<String>,
    pub stop_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub execution_contract: String,
    pub selected_behaviour_path: String,
    pub inputs: Vec<Input>,
    pub stop_when_active_state: Option<String>,
    #[serde(default)]
    pub bindings: BTreeMap<String, Value>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub expected: Option<Expected>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundInput {
    pub ordinal: u64,
    pub receiver: Id,
    pub occurrence_chain: Vec<Id>,
    pub payload_type: Id,
    pub values: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    pub id: Id,
    pub name: String,
    pub source: Id,
    pub target: Id,
    pub receiver: Id,
    pub payload_type: Id,
    pub payload_name: Option<String>,
    pub guard: Option<Expr>,
    pub span: Span,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    #[serde(default)]
    pub memory_base_bytes: usize,
    pub model_revision_id: Id,
    pub scenario_revision_id: Id,
    pub library_digest: String,
    pub engine_build: String,
    pub contract: String,
    pub input_digest: String,
    pub plan_digest: String,
    pub selected: Vec<Id>,
    pub definition: Id,
    pub initial: Id,
    pub initial_source: Id,
    pub states: BTreeMap<Id, String>,
    pub transitions: Vec<Transition>,
    pub inputs: Vec<BoundInput>,
    pub bindings: BTreeMap<String, Value>,
    pub stop_state: Option<Id>,
    pub limits: Limits,
    pub expected: Option<Expected>,
    pub excluded_interactions: Vec<String>,
}
fn prep_error(e: &Element, code: &str, message: impl Into<String>) -> Error {
    Error::diagnostics(
        code,
        vec![Diagnostic::error(code, message, Some(e.span.clone())).at(&e.id)],
    )
}
pub fn prepare(
    model: &Model,
    revision: &str,
    scenario_id: &str,
    s: &Scenario,
    build: &str,
) -> Result<Plan> {
    if !model.accepted() {
        return Err(Error::diagnostics(
            "invalid_model",
            model.diagnostics.clone(),
        ));
    }
    if s.execution_contract != CONTRACT {
        return Err(Error::new(
            "unsupported_feature",
            "Unknown execution contract",
        ));
    }
    if s.inputs.len() > 100_000
        || s.limits.max_semantic_steps > 1_000_000
        || s.limits.max_trace_records > 3_000_000
        || s.limits.max_memory_bytes > 512 * 1024 * 1024
    {
        return Err(Error::new(
            "resource_limit",
            "Experiment exceeds server hard limits",
        ));
    }
    if s.inputs.len() as u64 > s.limits.max_input_deliveries {
        return Err(Error::new(
            "resource_limit",
            "Input sequence exceeds delivery limit",
        ));
    }
    let selected = agq_semantics::locate(model, &s.selected_behaviour_path)?;
    let usage = model.element(selected.last().unwrap())?;
    let definition = if usage.kind == "StateDefinition"
        || usage.kind == "StateUsage" && usage.target("type").is_none()
    {
        usage
    } else {
        model.element(usage.target("type").ok_or_else(|| {
            prep_error(
                usage,
                "invalid_model",
                "Selection is not a typed state usage",
            )
        })?)?
    };
    if !["StateDefinition", "StateUsage"].contains(&definition.kind.as_str()) {
        return Err(prep_error(
            definition,
            "unsupported_feature",
            "Select one state definition or usage",
        ));
    }
    executable_declaration(usage)?;
    executable_declaration(definition)?;
    for id in &selected {
        let e = model.element(id)?;
        if let Some((lo, hi)) = e.multiplicity
            && (lo != 1 || hi != Some(1))
        {
            return Err(prep_error(
                e,
                "missing_binding",
                "Selected occurrence chain must have exactly one occurrence at each level",
            ));
        }
    }
    if definition.modifiers.contains(&"parallel".into()) || !definition.unsupported.is_empty() {
        return Err(prep_error(
            definition,
            "unsupported_feature",
            "AGQ-SEQ-01 requires a flat exclusive state region",
        ));
    }
    let children: Vec<_> = model.children(&definition.id).collect();
    let states: BTreeMap<_, _> = children
        .iter()
        .filter(|e| e.kind == "StateUsage")
        .map(|e| (e.id.clone(), e.name.clone().unwrap_or_default()))
        .collect();
    if states.is_empty() {
        return Err(prep_error(
            definition,
            "invalid_model",
            "Selected region contains no states",
        ));
    }
    for e in &children {
        executable_declaration(e)?;
        if !e.unsupported.is_empty()
            || e.modifiers.contains(&"parallel".into())
            || ![
                "StateUsage",
                "PortUsage",
                "AttributeUsage",
                "TransitionUsage",
                "EntryActionUsage",
                "SuccessionAsUsage",
            ]
            .contains(&e.kind.as_str())
        {
            return Err(prep_error(
                e,
                "unsupported_feature",
                "Unsupported reachable semantics in selected region",
            ));
        }
        if e.kind == "StateUsage"
            && (e.target("type").is_some()
                || model.children(&e.id).any(|child| {
                    child.kind != "EntryActionUsage"
                        || !child.unsupported.is_empty()
                        || model.children(&child.id).next().is_some()
                }))
        {
            return Err(prep_error(
                e,
                "unsupported_feature",
                "Nested states and nonempty entry/do/exit actions cannot execute",
            ));
        }
        if e.kind == "EntryActionUsage" && model.children(&e.id).next().is_some() {
            return Err(prep_error(
                e,
                "unsupported_feature",
                "Entry action must be empty",
            ));
        }
    }
    let entries: Vec<_> = children
        .iter()
        .filter(|e| e.kind == "EntryActionUsage")
        .collect();
    let initials: Vec<_> = children
        .iter()
        .filter(|e| e.kind == "SuccessionAsUsage")
        .collect();
    if entries.len() != 1 || initials.len() != 1 {
        return Err(prep_error(
            definition,
            "unsupported_feature",
            "Require one empty entry and one explicit initial succession",
        ));
    }
    if !initials[0]
        .modifiers
        .contains(&format!("initial_source:{}", entries[0].id))
    {
        return Err(prep_error(
            initials[0],
            "unsupported_feature",
            "Initial succession must immediately follow the empty entry action (SysML 7.18.3)",
        ));
    }
    if usage.id != definition.id && model.children(&usage.id).next().is_some() {
        return Err(prep_error(
            usage,
            "unsupported_feature",
            "Selected state usage extensions/redefinitions are not executable",
        ));
    }
    let initial = initials[0]
        .target("target")
        .ok_or_else(|| {
            prep_error(
                initials[0],
                "unresolved_reference",
                "Initial target unresolved",
            )
        })?
        .to_string();
    if !states.contains_key(&initial) {
        return Err(prep_error(
            initials[0],
            "invalid_model",
            "Initial target is outside selected region",
        ));
    }
    let mut bindings = BTreeMap::new();
    for e in children.iter().filter(|e| e.kind == "AttributeUsage") {
        let name = e
            .name
            .as_deref()
            .ok_or_else(|| prep_error(e, "unsupported_feature", "Anonymous parameter"))?;
        let value = s
            .bindings
            .get(&e.id)
            .or_else(|| s.bindings.get(name))
            .cloned()
            .or_else(|| {
                if let Some(Expr::Literal { value }) = &e.value {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| prep_error(e, "missing_binding", format!("Missing parameter {name}")))?;
        validate_value(model, e, &value)?;
        bindings.insert(name.into(), value);
    }
    for key in s.bindings.keys() {
        if !children
            .iter()
            .any(|e| e.kind == "AttributeUsage" && (e.name.as_ref() == Some(key) || &e.id == key))
        {
            return Err(Error::new(
                "invalid_binding",
                format!("Unknown binding {key}"),
            ));
        }
    }
    let mut transitions = vec![];
    for e in children.iter().filter(|e| e.kind == "TransitionUsage") {
        let required = |role: &str| {
            e.target(role).map(str::to_string).ok_or_else(|| {
                prep_error(
                    e,
                    "unsupported_feature",
                    format!("Transition requires explicit {role}"),
                )
            })
        };
        let source = required("source")?;
        let target = required("target")?;
        let receiver = required("receiver")?;
        let payload_type = required("payload_type")?;
        if !states.contains_key(&source) || !states.contains_key(&target) {
            return Err(prep_error(
                e,
                "unsupported_feature",
                "Transition endpoints must be states in the same flat region",
            ));
        }
        if model.elements[&receiver].kind != "PortUsage"
            || model.elements[&receiver].owner.as_ref() != Some(&definition.id)
        {
            return Err(prep_error(
                e,
                "unsupported_feature",
                "Receiver must be a port of the selected region",
            ));
        }
        if model.elements[&payload_type].kind != "ItemDefinition" {
            return Err(prep_error(
                e,
                "invalid_model",
                "Transition payload must be an item definition",
            ));
        }
        executable_declaration(&model.elements[&payload_type])?;
        for field in model.children(&payload_type) {
            executable_declaration(field)?;
            if field.kind != "AttributeUsage" {
                return Err(prep_error(
                    field,
                    "unsupported_feature",
                    "Only scalar payload fields can execute",
                ));
            }
        }
        if let Some(g) = &e.guard {
            validate_expr(g, &bindings, &BTreeMap::new(), false)
                .map_err(|err| prep_error(e, &err.code, err.message))?;
        }
        transitions.push(Transition {
            id: e.id.clone(),
            name: e.name.clone().unwrap_or_default(),
            source,
            target,
            receiver,
            payload_type,
            payload_name: e
                .modifiers
                .iter()
                .find_map(|m| m.strip_prefix("payload:").map(str::to_string)),
            guard: e.guard.clone(),
            span: e.span.clone(),
        });
    }
    transitions.sort_by_key(|t| t.span.start);
    let mut inputs = vec![];
    let mut prev = 0;
    for input in &s.inputs {
        if input.ordinal <= prev {
            return Err(Error::new(
                "invalid_binding",
                "Input ordinals must be positive, unique and strictly increasing",
            ));
        }
        prev = input.ordinal;
        let chain = agq_semantics::locate(model, &input.receiver_path)?;
        let receiver = chain.last().unwrap().clone();
        let payload_type = agq_semantics::locate(model, &input.payload_type)?
            .last()
            .unwrap()
            .clone();
        if model.elements[&receiver].owner.as_ref() != Some(&definition.id)
            || model.elements[&receiver].kind != "PortUsage"
        {
            return Err(Error::new(
                "invalid_binding",
                "Input receiver is outside selected behaviour",
            ));
        }
        if selected.len() > 1 && !chain.starts_with(&selected) {
            return Err(Error::new(
                "invalid_binding",
                "Receiver refers to a different occurrence of the behaviour",
            ));
        }
        let p = model.element(&payload_type)?;
        executable_declaration(p)?;
        if p.kind != "ItemDefinition" {
            return Err(prep_error(
                p,
                "invalid_binding",
                "Input payload must be an item definition",
            ));
        }
        let mut values = BTreeMap::new();
        for attribute in model.children(&payload_type) {
            executable_declaration(attribute)?;
            if attribute.kind != "AttributeUsage" {
                return Err(prep_error(
                    attribute,
                    "unsupported_feature",
                    "Only scalar item fields are supported",
                ));
            }
            let name = attribute.name.as_ref().unwrap();
            let value = input
                .values
                .get(name)
                .cloned()
                .or_else(|| {
                    if let Some(Expr::Literal { value }) = &attribute.value {
                        Some(value.clone())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    prep_error(
                        attribute,
                        "missing_binding",
                        format!("Input {} lacks {name}", input.ordinal),
                    )
                })?;
            validate_value(model, attribute, &value)?;
            values.insert(name.clone(), value);
        }
        if input.values.keys().any(|k| !values.contains_key(k)) {
            return Err(prep_error(
                p,
                "invalid_binding",
                "Input contains unknown payload field",
            ));
        }
        inputs.push(BoundInput {
            ordinal: input.ordinal,
            receiver,
            occurrence_chain: chain,
            payload_type,
            values,
        });
    }
    for t in &transitions {
        if let Some(g) = &t.guard {
            let mut params = bindings.clone();
            for attr in model.children(&t.payload_type) {
                if let Some(name) = &attr.name {
                    let scalar = agq_semantics::scalar_name(model.element(
                        attr.target("type").ok_or_else(|| {
                            prep_error(attr, "missing_binding", "Payload scalar type missing")
                        })?,
                    )?)
                    .unwrap_or("");
                    let value = match scalar {
                        "Boolean" => Value::Boolean(false),
                        "Integer" => Value::Integer("0".into()),
                        "String" => Value::String(String::new()),
                        _ => {
                            return Err(prep_error(
                                attr,
                                "unsupported_feature",
                                "Unsupported payload scalar",
                            ));
                        }
                    };
                    if let Some(prefix) = &t.payload_name {
                        params.insert(format!("{prefix}.{name}"), value);
                    }
                }
            }
            // Values here are type witnesses only. They are never runtime bindings or input defaults.
            let value = eval(g, &params)
                .map_err(|err| prep_error(model.element(&t.id).unwrap(), &err.code, err.message))?;
            if !matches!(value, Value::Boolean(_)) {
                return Err(prep_error(
                    model.element(&t.id)?,
                    "invalid_model",
                    "Transition guard must be Boolean",
                ));
            }
        }
    }
    let stop_state = match &s.stop_when_active_state {
        Some(path) => {
            let chain = agq_semantics::locate(model, path)?;
            let id = chain.last().unwrap().clone();
            if !states.contains_key(&id) || (selected.len() > 1 && !chain.starts_with(&selected)) {
                return Err(Error::new(
                    "invalid_binding",
                    "Stop state is outside selected occurrence",
                ));
            }
            Some(id)
        }
        None => None,
    };
    let mut plan=Plan{memory_base_bytes:0,model_revision_id:revision.into(),scenario_revision_id:scenario_id.into(),library_digest:model.library_digest.clone(),engine_build:build.into(),contract:CONTRACT.into(),input_digest:json_digest(&inputs),plan_digest:String::new(),selected,definition:definition.id.clone(),initial,initial_source:initials[0].id.clone(),states,transitions,inputs,bindings,stop_state,limits:s.limits.clone(),expected:s.expected.clone(),excluded_interactions:vec!["Only the selected state occurrence receives ordered scenario inputs; structural connections and other contained behaviours do not execute.".into()]};
    plan.memory_base_bytes = serde_json::to_vec(&plan)
        .unwrap()
        .len()
        .saturating_mul(8)
        .saturating_add(16384);
    plan.plan_digest = json_digest(&plan);
    if plan.memory_base_bytes > plan.limits.max_memory_bytes {
        return Err(Error::new(
            "resource_limit",
            "Prepared plan exceeds declared memory budget",
        ));
    }
    Ok(plan)
}
fn validate_value(model: &Model, e: &Element, v: &Value) -> Result<()> {
    if !v.valid() {
        return Err(prep_error(
            e,
            "evaluation_error",
            "Invalid or oversized exact scalar value",
        ));
    }
    let t = e.target("type").ok_or_else(|| {
        prep_error(
            e,
            "missing_binding",
            "Scalar attribute needs an explicit type",
        )
    })?;
    if agq_semantics::scalar_name(&model.elements[t]) != Some(v.scalar_type()) {
        return Err(prep_error(
            e,
            "invalid_binding",
            "Scalar binding type mismatch",
        ));
    }
    if let Some(Expr::Literal { value }) = &e.value
        && value != v
    {
        return Err(prep_error(
            e,
            "invalid_binding",
            "Scenario binding conflicts with the authored feature value",
        ));
    }
    Ok(())
}
fn executable_declaration(e: &Element) -> Result<()> {
    if !e.unsupported.is_empty()
        || e.references
            .iter()
            .any(|r| ["specialization", "redefinition"].contains(&r.role.as_str()))
        || e.modifiers
            .iter()
            .any(|m| ["all", "abstract", "variation", "parallel"].contains(&m.as_str()))
    {
        return Err(prep_error(
            e,
            "unsupported_feature",
            "Execution of explicit specialization, redefinition, variation or unsupported declaration semantics is not implemented",
        ));
    }
    if e.multiplicity
        .is_some_and(|(lo, hi)| lo != 1 || hi != Some(1))
    {
        return Err(prep_error(
            e,
            "unsupported_feature",
            "Executable fields and states require unit multiplicity",
        ));
    }
    if e.value
        .as_ref()
        .is_some_and(|v| !matches!(v, Expr::Literal { .. }))
    {
        return Err(prep_error(
            e,
            "unsupported_feature",
            "Only literal authored feature values are executable",
        ));
    }
    Ok(())
}
fn validate_expr(
    e: &Expr,
    bindings: &BTreeMap<String, Value>,
    extra: &BTreeMap<String, Value>,
    strict: bool,
) -> Result<()> {
    match e {
        Expr::Unsupported { source } => Err(Error::new(
            "unsupported_feature",
            format!("Unsupported guard operator/expression: {source}"),
        )),
        Expr::Name { path }
            if strict && !bindings.contains_key(path) && !extra.contains_key(path) =>
        {
            Err(Error::new(
                "missing_binding",
                format!("Unbound name {path}"),
            ))
        }
        Expr::Unary { arg, .. } => validate_expr(arg, bindings, extra, strict),
        Expr::Binary { left, right, .. } => {
            validate_expr(left, bindings, extra, strict)?;
            validate_expr(right, bindings, extra, strict)
        }
        _ => Ok(()),
    }
}
pub fn eval(e: &Expr, env: &BTreeMap<String, Value>) -> Result<Value> {
    let err = || Error::new("evaluation_error", "Guard operands have incompatible types");
    match e {
        Expr::Literal { value } => {
            if value.valid() {
                Ok(value.clone())
            } else {
                Err(Error::new("evaluation_error", "Invalid exact scalar"))
            }
        }
        Expr::Name { path } => env
            .get(path)
            .cloned()
            .ok_or_else(|| Error::new("missing_binding", format!("Unbound guard name {path}"))),
        Expr::Unsupported { source } => Err(Error::new(
            "unsupported_feature",
            format!("Unsupported expression {source}"),
        )),
        Expr::Unary { op, arg } => {
            let Value::Boolean(v) = eval(arg, env)? else {
                return Err(err());
            };
            if op == "not" {
                Ok(Value::Boolean(!v))
            } else {
                Err(err())
            }
        }
        Expr::Binary { op, left, right } => {
            let a = eval(left, env)?;
            let b = eval(right, env)?;
            let result = match (&a, &b) {
                (Value::Boolean(a), Value::Boolean(b)) => match op.as_str() {
                    "and" | "&" => *a && *b,
                    "or" | "|" => *a || *b,
                    "xor" => *a ^ *b,
                    "==" => a == b,
                    "!=" => a != b,
                    _ => return Err(err()),
                },
                (Value::Integer(a), Value::Integer(b)) => {
                    let a = a.parse::<num_bigint::BigInt>().map_err(|_| err())?;
                    let b = b.parse::<num_bigint::BigInt>().map_err(|_| err())?;
                    match op.as_str() {
                        "==" => a == b,
                        "!=" => a != b,
                        "<" => a < b,
                        "<=" => a <= b,
                        ">" => a > b,
                        ">=" => a >= b,
                        _ => return Err(err()),
                    }
                }
                (Value::String(a), Value::String(b)) => match op.as_str() {
                    "==" => a == b,
                    "!=" => a != b,
                    _ => return Err(err()),
                },
                _ => return Err(err()),
            };
            Ok(Value::Boolean(result))
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Prepared,
    Running,
    Paused,
    Completed,
    Stopped,
    Blocked,
    Failed,
    Interrupted,
}
impl Status {
    pub fn terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Stopped | Self::Blocked | Self::Failed | Self::Interrupted
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trace {
    pub sequence: u64,
    pub semantic_step: u64,
    pub kind: String,
    pub model_revision_id: Id,
    pub run_id: Id,
    pub occurrence_id: Id,
    pub source_element_id: Id,
    pub input_ordinal: Option<u64>,
    pub before: Option<Id>,
    pub after: Option<Id>,
    pub outcome: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub status: String,
    pub details: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: Id,
    pub plan: std::sync::Arc<Plan>,
    pub status: Status,
    pub control_version: u64,
    pub active: Option<Id>,
    pub occurrence_id: Id,
    pub next_input: usize,
    pub semantic_steps: u64,
    pub trace: std::sync::Arc<Vec<Trace>>,
    pub stop_reason: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub checks: Vec<Check>,
}
impl Run {
    pub fn new(plan: Plan) -> Self {
        let id = new_id();
        Self {
            occurrence_id: derived_id(&id, "state/0"),
            id,
            plan: std::sync::Arc::new(plan),
            status: Status::Prepared,
            control_version: 0,
            active: None,
            next_input: 0,
            semantic_steps: 0,
            trace: std::sync::Arc::new(vec![]),
            stop_reason: None,
            diagnostics: vec![],
            checks: vec![Check {
                name: "documentary_requirements".into(),
                status: "not_run".into(),
                details: "SysML verification declarations have no executable procedure".into(),
            }],
        }
    }
    fn record(
        &mut self,
        kind: &str,
        source: Id,
        ordinal: Option<u64>,
        before: Option<Id>,
        after: Option<Id>,
        outcome: &str,
    ) {
        let sequence = self.trace.len() as u64;
        std::sync::Arc::make_mut(&mut self.trace).push(Trace {
            sequence,
            semantic_step: self.semantic_steps,
            kind: kind.into(),
            model_revision_id: self.plan.model_revision_id.clone(),
            run_id: self.id.clone(),
            occurrence_id: self.occurrence_id.clone(),
            source_element_id: source,
            input_ordinal: ordinal,
            before,
            after,
            outcome: outcome.into(),
        });
    }
    fn finish(&mut self, status: Status, reason: &str) {
        self.status = status;
        self.stop_reason = Some(reason.into());
        if let Some(expected) = &self.plan.expected {
            let states: Vec<_> = self
                .trace
                .iter()
                .filter(|t| t.kind == "initial" || t.kind == "transition")
                .filter_map(|t| t.after.as_ref().and_then(|id| self.plan.states.get(id)))
                .cloned()
                .collect();
            let transitions: Vec<_> = self
                .trace
                .iter()
                .filter(|t| t.kind == "transition")
                .filter_map(|t| {
                    self.plan
                        .transitions
                        .iter()
                        .find(|x| x.id == t.source_element_id)
                        .map(|x| x.name.clone())
                })
                .collect();
            let passed = (expected.active_states.is_empty() || states == expected.active_states)
                && (expected.transition_names.is_empty()
                    || transitions == expected.transition_names)
                && expected.stop_reason.as_deref().is_none_or(|r| r == reason);
            self.checks.push(Check {
                name: "scenario_expectations".into(),
                status: if self.status == Status::Completed {
                    if passed { "pass" } else { "fail" }
                } else {
                    "inconclusive"
                }
                .into(),
                details: format!("Observed stop reason: {reason}"),
            });
        }
    }
    fn block(&mut self, code: &str, message: &str, element: Option<Id>) {
        let status = if ["ambiguous_transition", "unhandled_input"].contains(&code) {
            Status::Blocked
        } else {
            Status::Failed
        };
        self.finish(status, code);
        self.diagnostics.push(Diagnostic {
            code: code.into(),
            message: message.into(),
            severity: "error".into(),
            span: element.as_ref().and_then(|id| {
                self.plan
                    .transitions
                    .iter()
                    .find(|t| &t.id == id)
                    .map(|t| t.span.clone())
            }),
            element_id: element,
        });
    }
    fn budget(&self, records: usize, steps: u64) -> Option<&'static str> {
        if self.semantic_steps + steps > self.plan.limits.max_semantic_steps {
            return Some("step_limit");
        }
        if self.trace.len() + records > self.plan.limits.max_trace_records {
            return Some("trace_limit");
        }
        if self
            .plan
            .memory_base_bytes
            .saturating_add((self.trace.len() + records).saturating_mul(2048))
            > self.plan.limits.max_memory_bytes
        {
            return Some("memory_limit");
        }
        None
    }
    pub fn initialise(&mut self) -> Result<()> {
        if self.status != Status::Prepared {
            return Err(Error::new(
                "invalid_control",
                "Initialise requires prepared status",
            ));
        }
        if let Some(reason) = self.budget(1, 1) {
            self.block(reason, "Initialisation exceeds declared budget", None);
            return Ok(());
        }
        self.active = Some(self.plan.initial.clone());
        self.semantic_steps = 1;
        self.status = Status::Paused;
        self.record(
            "initial",
            self.plan.initial_source.clone(),
            None,
            None,
            self.active.clone(),
            "entered",
        );
        Ok(())
    }
    pub fn step(&mut self) -> Result<()> {
        if !matches!(self.status, Status::Paused | Status::Running) {
            return Err(Error::new(
                "invalid_control",
                "Step requires initialised, nonterminal run",
            ));
        }
        if self.plan.stop_state.is_some() && self.active == self.plan.stop_state {
            self.finish(Status::Completed, "condition_met");
            return Ok(());
        }
        if self.next_input == self.plan.inputs.len() {
            self.finish(Status::Completed, "input_exhausted");
            return Ok(());
        }
        if self.next_input as u64 >= self.plan.limits.max_input_deliveries {
            self.block("input_limit", "Input delivery budget exhausted", None);
            return Ok(());
        }
        if let Some(reason) = self.budget(2, 1) {
            self.block(reason, "Declared experiment budget exhausted", None);
            return Ok(());
        }
        let input = self.plan.inputs[self.next_input].clone();
        let mut enabled = vec![];
        for t in &self.plan.transitions {
            if self.active.as_deref() != Some(&t.source)
                || t.receiver != input.receiver
                || t.payload_type != input.payload_type
            {
                continue;
            }
            let mut env = self.plan.bindings.clone();
            if let Some(prefix) = &t.payload_name {
                for (n, v) in &input.values {
                    env.insert(format!("{prefix}.{n}"), v.clone());
                }
            }
            let result = match &t.guard {
                Some(g) => eval(g, &env),
                None => Ok(Value::Boolean(true)),
            };
            match result {
                Ok(Value::Boolean(true)) => enabled.push(t.clone()),
                Ok(Value::Boolean(false)) => {}
                Ok(_) => {
                    let id = t.id.clone();
                    self.block("evaluation_error", "Guard is not Boolean", Some(id));
                    return Ok(());
                }
                Err(err) => {
                    let id = t.id.clone();
                    self.block("evaluation_error", &err.message, Some(id));
                    return Ok(());
                }
            }
        }
        if enabled.len() != 1 {
            self.block(
                if enabled.is_empty() {
                    "unhandled_input"
                } else {
                    "ambiguous_transition"
                },
                "Exactly one eligible transition is required; state and input cursor retained",
                None,
            );
            return Ok(());
        }
        let t = enabled.remove(0);
        let before = self.active.clone();
        // All fallible evaluation precedes this point. The caller durably commits this candidate as a unit.
        self.semantic_steps += 1;
        self.record(
            "input",
            input.receiver.clone(),
            Some(input.ordinal),
            before.clone(),
            before.clone(),
            "delivered",
        );
        self.next_input += 1;
        self.active = Some(t.target.clone());
        self.occurrence_id = derived_id(&self.id, &format!("state/{}", self.semantic_steps));
        self.record(
            "transition",
            t.id,
            Some(input.ordinal),
            before,
            self.active.clone(),
            "taken",
        );
        if self.plan.stop_state.is_some() && self.active == self.plan.stop_state {
            self.finish(Status::Completed, "condition_met")
        } else if self.next_input == self.plan.inputs.len() {
            self.finish(Status::Completed, "input_exhausted")
        }
        Ok(())
    }
    pub fn control(&mut self, operation: &str) -> Result<()> {
        if self.status.terminal() {
            return Err(Error::new(
                "invalid_control",
                "Terminal runs are immutable; reset creates a new run",
            ));
        }
        match operation {
            "initialise" => self.initialise()?,
            "step" => {
                if self.status != Status::Paused {
                    return Err(Error::new(
                        "invalid_control",
                        "Manual step requires paused status",
                    ));
                }
                self.step()?
            }
            "run" => {
                if self.status != Status::Paused {
                    return Err(Error::new("invalid_control", "Run requires paused status"));
                }
                self.status = Status::Running;
            }
            "pause" => {
                if self.status != Status::Running {
                    return Err(Error::new(
                        "invalid_control",
                        "Pause requires running status",
                    ));
                }
                self.status = Status::Paused;
            }
            "stop" => self.finish(Status::Stopped, "user_stop"),
            _ => return Err(Error::new("invalid_control", "Unknown run control")),
        }
        self.control_version += 1;
        Ok(())
    }
    pub fn interrupt(&mut self) {
        if !self.status.terminal() {
            self.finish(Status::Interrupted, "interrupted");
            self.control_version += 1;
        }
    }
    pub fn normalized_trace(&self) -> serde_json::Value {
        serde_json::Value::Array(self.trace.iter().map(|t|serde_json::json!({"sequence":t.sequence,"semantic_step":t.semantic_step,"kind":t.kind,"source_element_id":t.source_element_id,"input_ordinal":t.input_ordinal,"before":t.before,"after":t.after,"outcome":t.outcome})).collect())
    }
    pub fn trace_digest(&self) -> String {
        json_digest(&self.normalized_trace())
    }
    pub fn execute(&mut self) -> Result<()> {
        if self.status == Status::Prepared {
            self.initialise()?;
        }
        if self.status == Status::Paused {
            self.status = Status::Running;
        }
        while self.status == Status::Running {
            self.step()?;
        }
        Ok(())
    }
}
