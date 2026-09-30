//! The Assistant's tools (ROADMAP §4.2, §4.10).
//!
//! A tool either reads the System State or turns the Assistant's request into
//! one System State [`Change`]. The Assistant never changes the model itself:
//! the Studio applies the change through the same path as the Operator's own
//! edits, which asks the Operator before a locked element changes. Tool input
//! is untrusted: every name is resolved and every operation is tried on a copy
//! of the model before the change is handed over.
//!
//! The Library tools (`library`) find, read and use building blocks through
//! the same Library service the Operator's Studio uses (C-49); saving to My
//! Library waits for the Operator's confirmation.
//!
//! The factory tools (`factory`, C-50) read behaviour and scenarios, and
//! ask the Studio to run scenarios, read their results, read the code
//! links and check the implementation, through the services the Operator
//! uses. Scenarios and state machines are written with `apply_changes`.

mod factory;
mod library;

pub use factory::{StudioRequest, carry_out_headless};

use agq_language::{
    Direction, Element, ElementId, ElementKind, Literal, Multiplicity, Parent, Reference, Tree,
    print_element,
};
use agq_library::Library;
use agq_system_state::{Actor, Change, ChangeEvent, Operation, Property, Rejection, SystemState};
use serde_json::{Value, json};

pub const READ_MODEL: &str = "read_model";
pub const FIND_ELEMENTS: &str = "find_elements";
pub const GET_PROBLEMS: &str = "get_problems";
pub const APPLY_CHANGES: &str = "apply_changes";
pub const ASK_OPERATOR: &str = "ask_operator";
pub const SEARCH_LIBRARY: &str = "search_library";
pub const READ_LIBRARY_BLOCK: &str = "read_library_block";
pub const USE_LIBRARY_BLOCK: &str = "use_library_block";
pub const SAVE_TO_LIBRARY: &str = "save_to_library";
pub const INSPECT_BEHAVIOUR: &str = "inspect_behaviour";
pub const LIST_SCENARIOS: &str = "list_scenarios";
pub const RUN_SCENARIO: &str = "run_scenario";
pub const STOP_RUN: &str = "stop_run";
pub const READ_RUN: &str = "read_run";
pub const READ_CODE_LINKS: &str = "read_code_links";
pub const CHECK_IMPLEMENTATION: &str = "check_implementation";

/// The longest tool result the model reads, in characters: about 8,000
/// tokens (R-34). Longer results are cut with a note on narrowing the
/// request.
pub const RESULT_LIMIT: usize = 32_000;

/// `text`, cut at a line boundary to [`RESULT_LIMIT`], with a note that says
/// how to narrow the request.
pub fn cap(text: String) -> String {
    if text.len() <= RESULT_LIMIT {
        return text;
    }
    let mut end = RESULT_LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let end = text[..end].rfind('\n').unwrap_or(end);
    let rest = text[end..]
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    format!(
        "{}\n[Cut: {rest} more lines. Narrow the request: read one element by qualified name, or find elements by name or kind.]",
        &text[..end]
    )
}

/// The element kinds the Assistant may create, by their SysML keyword.
const KINDS: &[ElementKind] = &[
    ElementKind::Package,
    ElementKind::PartDef,
    ElementKind::Part,
    ElementKind::PortDef,
    ElementKind::Port,
    ElementKind::ItemDef,
    ElementKind::Item,
    ElementKind::AttributeDef,
    ElementKind::Attribute,
    ElementKind::InterfaceDef,
    ElementKind::Interface,
    ElementKind::ConnectionDef,
    ElementKind::Connection,
    ElementKind::RequirementDef,
    ElementKind::Requirement,
    ElementKind::Subject,
    ElementKind::Satisfy,
    ElementKind::EnumDef,
    ElementKind::Enum,
    // Behaviour and scenarios (C-50).
    ElementKind::State,
    ElementKind::Transition,
    ElementKind::VerificationDef,
    ElementKind::Send,
    ElementKind::Accept,
    ElementKind::AssertConstraint,
    ElementKind::Reference,
];

/// What the Studio should do with a tool call.
#[derive(Clone, Debug, PartialEq)]
pub enum Prepared {
    /// A read-only answer: send it back as the tool result.
    Answer(String),
    /// A change to apply like any Operator edit (lock confirmation included),
    /// then describe with [`describe_event`] or [`describe_rejection`].
    Change(Change),
    /// A question for the Operator; the answer is the tool result.
    Question {
        question: String,
        options: Vec<String>,
    },
    /// Saving to My Library, which changes the Operator's own library for
    /// every project: shown as a question with Save and Don't save. Only on
    /// Save does the Studio save `plan`, and `saved` is the tool result.
    SaveToLibrary {
        plan: Box<agq_library::SavePlan>,
        question: String,
        saved: String,
    },
    /// Something only the Studio can do with its own services (C-50): run
    /// a scenario, read a result, read the code links, check the code.
    Studio(StudioRequest),
    /// The input cannot be used: send the message back as an error result.
    Invalid(String),
}

/// The tool definitions sent to the model (JSON Schema per tool).
pub fn definitions() -> Value {
    let kinds: Vec<&str> = KINDS.iter().map(|kind| kind.keyword()).collect();
    let library_kinds: Vec<&str> = library::KINDS.iter().map(|kind| kind.keyword()).collect();
    let name = |description: &str| json!({ "type": "string", "description": description });
    let operation_fields = json!({
        "op": { "type": "string", "enum": ["create", "delete", "rename", "move", "connect", "set"] },
        "element": name("delete, rename, move, set: qualified name of the element, e.g. \"Shop::Store\"."),
        "parent": name("create, move, connect: qualified name of the new owner. Omit for the top level of the model."),
        "kind": { "type": "string", "enum": kinds, "description": "create: what to create." },
        "name": name("create: the new element's name. rename: the new name."),
        "type": name("create, set: the type of a usage, by qualified or visible name, e.g. \"LinkStore\" or \"ScalarValues::String\". Prefix with ~ for a conjugated port type."),
        "specializes": { "type": "array", "items": { "type": "string" }, "description": "create, set: general definitions (:>) of a definition, or subsetted features of a usage." },
        "redefines": { "type": "array", "items": { "type": "string" }, "description": "create, set: inherited features this usage redefines (:>>)." },
        "multiplicity": name("create, set: e.g. \"1\", \"0..1\", \"1..*\", \"*\"."),
        "direction": { "type": "string", "enum": ["in", "out", "inout"], "description": "create, set: direction of an item or port feature." },
        "end": { "type": "boolean", "description": "create: true for an end of an interface or connection def, e.g. a port `client` with type \"~LinkStorePort\" becomes `end port client : ~LinkStorePort;`." },
        "value": { "type": ["string", "number", "boolean"], "description": "create, set: the value of an attribute." },
        "doc": name("create, set: documentation in plain words."),
        "from": name("connect: the first end, a feature chain relative to the parent, e.g. \"api.storage\"."),
        "to": name("connect: the second end, e.g. \"store.links\"."),
        "definition": name("connect: the interface or connection definition that types it, e.g. \"LinkStorage\"."),
        "requirement": name("create satisfy: the requirement being satisfied."),
        "by": name("create satisfy: the feature that satisfies it, e.g. \"shortener.store\"."),
        "expression": name("create, set: an expression, as KerML writes it. send: what is sent, e.g. \"new ShortenRequest(longUrl = \\\"https://a.example/x\\\", host = \\\"a.example\\\")\"; accept with after: the time in ms; assert constraint: the condition, e.g. \"link.status == LinkStatus::held\"; ref or attribute: a value that is not a plain literal, e.g. \"service.screening\"."),
        "via": name("create send or accept: the port, as a feature chain from the scenario or the part, e.g. \"service.shorten\"."),
        "after": { "type": "boolean", "description": "create accept: wait for the time in `expression` to pass instead of for an item." },
        "guard": name("create or set transition: the condition, e.g. \"attempts < 3\"."),
        "exhibit": { "type": "boolean", "description": "create state: the state machine the owning part or part def exhibits (one per owner)." },
        "initial": name("create exhibited state: the state it enters first (created with `then`)."),
        "trigger": {
            "type": "object",
            "description": "create transition: what fires it, an item arriving (`type` and `via`, optionally a `name` for the guard and effect to use) or time passing (`after`, an expression in ms).",
            "properties": { "name": name("The payload's name."), "type": name("The item type."), "via": name("The port."), "after": name("Milliseconds, as an expression.") },
            "additionalProperties": false
        },
        "effect": {
            "type": "object",
            "description": "create transition: what it does, a `send` (an expression, with `via`) or an `assign` (a feature and its new `value` expression).",
            "properties": { "send": name("What to send."), "via": name("The port."), "assign": name("The feature to set."), "value": name("Its new value.") },
            "additionalProperties": false
        },
        "subject": name("create verification def: the type of the scenario's subject, e.g. \"UrlShortenerService\"; the subject is named after it (urlShortenerService) unless `subject_name` says otherwise."),
        "subject_name": name("create verification def: the subject's name, e.g. \"service\"."),
        "verifies": { "type": "array", "items": { "type": "string" }, "description": "create verification def: requirements (usages) it verifies." },
        "features": { "type": "object", "description": "create part, create or set any usage: inherited features it redefines with values (:>>), e.g. a stand-in {\"target\": \"service.screening\", \"outcome\": \"Scenarios::Outcome::timeout\", \"latencyMs\": 50} or an agent's {\"minConfidence\": 0.8}. A number or true/false is a value; text is an expression (write text values in quotes).", "additionalProperties": { "type": ["string", "number", "boolean"] } }
    });
    json!([
        {
            "name": READ_MODEL,
            "description": "Without an element: an outline of the architecture, one line per element (name, kind, type, direction, ends), indented by owner, with locks and problem counts. With an element's qualified name: that element's full text. Read the outline first, then the elements you will change.",
            "input_schema": {
                "type": "object",
                "properties": { "element": name("Optional qualified name of one element.") },
                "additionalProperties": false
            }
        },
        {
            "name": FIND_ELEMENTS,
            "description": "Find elements by part of their name and/or by kind. Returns qualified names, kinds and locks.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "name": name("Part of a name, matched without case."),
                    "kind": { "type": "string", "enum": kinds }
                },
                "additionalProperties": false
            }
        },
        {
            "name": GET_PROBLEMS,
            "description": "List every problem in the model (validation errors and unsupported constructs) with the element it is reported at.",
            "input_schema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": APPLY_CHANGES,
            "description": "Change the architecture. All operations apply together as one change (one undo step) or not at all. Later operations can refer to elements created by earlier ones by qualified name. Changing a locked element asks the Operator first.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "description": name("What the change does, in plain words; shown in history and undo."),
                    "operations": {
                        "type": "array",
                        "minItems": 1,
                        "items": { "type": "object", "properties": operation_fields, "required": ["op"], "additionalProperties": false }
                    }
                },
                "required": ["description", "operations"],
                "additionalProperties": false
            }
        },
        {
            "name": ASK_OPERATOR,
            "description": "Ask the Operator a question and wait for the answer. Use it for major decisions (for example whether statistics are a separate service) instead of guessing.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "question": name("The question, short and specific."),
                    "options": { "type": "array", "items": { "type": "string" }, "description": "Suggested answers; the Operator may also answer freely." }
                },
                "required": ["question"],
                "additionalProperties": false
            }
        },
        {
            "name": INSPECT_BEHAVIOUR,
            "description": "How a part or part def behaves: what may go in and come out of each port (seen from outside), its state machine (states and transitions with their triggers, guards and effects), its agent settings if it is an agent, and the parts inside it a scenario may stand in for. Read it before writing a scenario or a state machine.",
            "input_schema": {
                "type": "object",
                "properties": { "element": name("Qualified name of a part def or part usage.") },
                "required": ["element"],
                "additionalProperties": false
            }
        },
        {
            "name": LIST_SCENARIOS,
            "description": "The model's scenarios (verification defs): subject, the requirements they verify, stand-ins and steps, one block per scenario.",
            "input_schema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": RUN_SCENARIO,
            "description": "Run a scenario and wait for its result: model (the model's own behaviour with the scenario's stand-ins; deterministic, offline), replay (agents answer from kept recordings; stops at a missing one), walkthrough (shows the steps; verifies nothing) or implementation (the real code through the project's harness; only when the Operator allowed trusted-local execution). Live evaluation costs money and is started only by the Operator. The result says how it ended and why each check passed or failed.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "scenario": name("Qualified name of the verification def."),
                    "mode": { "type": "string", "enum": ["model", "replay", "walkthrough", "implementation"], "description": "Default model." }
                },
                "required": ["scenario"],
                "additionalProperties": false
            }
        },
        {
            "name": STOP_RUN,
            "description": "Stop the scenario run in progress; it ends as cancelled.",
            "input_schema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": READ_RUN,
            "description": "The newest result of a scenario in a mode, with whether it still describes the model, each check's verdict and reason, and the events before it ended.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "scenario": name("Qualified name of the verification def."),
                    "mode": { "type": "string", "enum": ["model", "replay", "walkthrough", "implementation", "live"], "description": "Default model." }
                },
                "required": ["scenario"],
                "additionalProperties": false
            }
        },
        {
            "name": READ_CODE_LINKS,
            "description": "The code linked to the model (model/links.json): for an element, or all of them. Each link says which file (and symbol) implements, defines or tests the element. Also the newest implementation checks and the drift they found, if any.",
            "input_schema": {
                "type": "object",
                "properties": { "element": name("Optional qualified name of one element.") },
                "additionalProperties": false
            }
        },
        {
            "name": CHECK_IMPLEMENTATION,
            "description": "Run the implementation checks and wait for them: module boundaries against the model's dependencies, contract shapes (Rust types against item and enum defs) and the linked tests. Builds and tests run only when the Operator allowed trusted-local execution; otherwise the tests are reported as not run.",
            "input_schema": { "type": "object", "properties": {}, "additionalProperties": false }
        },
        {
            "name": SEARCH_LIBRARY,
            "description": "Search the Library of building blocks: reusable definitions from the built-in library (neutral software concepts such as services, gateways, stores, caches, queues, workers, retries), the project's own definitions and the Operator's My Library. Use it before modelling a common concept by hand. With fits_port, only blocks with a port that can connect to that port by the model's rules. Returns one line per block: its reference, kind, source, purpose and ports.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": name("Words to find, matched against names, purposes and ports, e.g. \"rate limit\" or \"cache\". Empty lists blocks by category."),
                    "kind": { "type": "string", "enum": library_kinds, "description": "Only this kind of definition." },
                    "scope": { "type": "string", "enum": ["all", "built-in", "project", "mine"], "description": "Only the built-in library, the project's definitions or My Library. Default all." },
                    "fits_port": name("A port as a feature chain from its owner's qualified name, e.g. \"Shop::System::front.backend\": only blocks that can connect to it."),
                    "limit": { "type": "number", "description": "At most this many results (default 12, at most 40)." }
                },
                "additionalProperties": false
            }
        },
        {
            "name": READ_LIBRARY_BLOCK,
            "description": "Read one building block in words: its purpose, ports and the items they carry, parts, connections, attributes and inner attributes you can give values, requirements, the definitions it needs, and whether the project already has it. Read a block before using it, to check that its meaning fits.",
            "input_schema": {
                "type": "object",
                "properties": { "block": name("The block's reference from search_library, e.g. \"built-in:Library::Storage::CachedStore\", or its qualified name.") },
                "required": ["block"],
                "additionalProperties": false
            }
        },
        {
            "name": USE_LIBRARY_BLOCK,
            "description": "Use a building block, as one change (one undo step): copies the definitions it needs into the project's Library package (identical copies already there are reused; a different definition with the same name is reported, never overwritten) and adds a usage typed by it inside parent. Optionally gives inherited attributes values and connects one of its ports. The usage shows the block's ports; its inside stays in the definition. Changing a locked element asks the Operator first.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "block": name("The block's reference or qualified name."),
                    "parent": name("Qualified name of the part or part def that gets the usage, e.g. \"Shop::System\"."),
                    "name": name("The usage's name in lowerCamelCase; defaults to the block's name."),
                    "values": { "type": "object", "description": "Inherited attributes to give values in this usage, e.g. {\"ttlSeconds\": 60}.", "additionalProperties": { "type": ["string", "number", "boolean"] } },
                    "connect_to": name("A port to connect the new usage to, as a feature chain from parent, e.g. \"front.backend\"; the first of the block's ports that fits is used."),
                    "connect_with": name("With connect_to: the block's port to connect, by name."),
                    "if_exists": { "type": "string", "enum": ["ask", "use_existing", "copy_renamed"], "description": "When the project has a different definition with the same name: stop and report (ask, the default), use the project's, or copy the block's under another name." }
                },
                "required": ["block", "parent"],
                "additionalProperties": false
            }
        },
        {
            "name": SAVE_TO_LIBRARY,
            "description": "Save a project definition, with what it needs, to the Operator's My Library for use in other projects. Only when the Operator asked for it: the Operator sees and confirms the save. It changes no project.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "definition": name("Qualified name of the definition to save."),
                    "category": name("The package it is filed under in My Library, e.g. \"Payments\"; defaults to its package's name."),
                    "replace": { "type": "boolean", "description": "Replace a different block of the same name in My Library (only when the Operator agreed)." }
                },
                "required": ["definition"],
                "additionalProperties": false
            }
        }
    ])
}

/// Turns one tool call into what the Studio should do. The input has been
/// checked with [`check_input`] (the turn does that before the call reaches
/// the Studio). `library` is the Library of building blocks (the built-in
/// blocks and the Operator's My Library).
pub fn prepare(state: &SystemState, library: &Library, tool: &str, input: &Value) -> Prepared {
    let result = match tool {
        READ_MODEL => read_model(state, input).map(Prepared::Answer),
        FIND_ELEMENTS => find_elements(state.tree(), input).map(Prepared::Answer),
        GET_PROBLEMS => Ok(Prepared::Answer(problems(state, None))),
        APPLY_CHANGES => prepare_change(state, input).map(Prepared::Change),
        ASK_OPERATOR => ask_operator(input),
        SEARCH_LIBRARY => library::search(state, library, input).map(Prepared::Answer),
        READ_LIBRARY_BLOCK => library::read(state, library, input).map(Prepared::Answer),
        USE_LIBRARY_BLOCK => library::use_block(state, library, input).map(Prepared::Change),
        SAVE_TO_LIBRARY => library::save(state, library, input),
        INSPECT_BEHAVIOUR => factory::inspect_behaviour(state.tree(), input).map(Prepared::Answer),
        LIST_SCENARIOS => Ok(Prepared::Answer(factory::list_scenarios(state.tree()))),
        RUN_SCENARIO | STOP_RUN | READ_RUN | READ_CODE_LINKS | CHECK_IMPLEMENTATION => {
            factory::studio_request(state.tree(), tool, input).map(Prepared::Studio)
        }
        other => Err(format!("there is no tool called `{other}`")),
    };
    result.unwrap_or_else(Prepared::Invalid)
}

/// Checks a tool call's input against the tool's input schema from
/// [`definitions`]: field names, types, allowed values and required fields.
/// The API does not check inputs that stream in as they are generated, so
/// the input is checked here before anything runs.
pub fn check_input(tool: &str, input: &Value) -> Result<(), String> {
    let definitions = definitions();
    let schema = definitions
        .as_array()
        .into_iter()
        .flatten()
        .find(|definition| definition["name"] == tool)
        .map(|definition| &definition["input_schema"])
        .ok_or_else(|| format!("there is no tool called `{tool}`"))?;
    check(schema, input, "input")
}

/// Checks `value` (found at `at`) against the parts of JSON Schema the tool
/// definitions use.
fn check(schema: &Value, value: &Value, at: &str) -> Result<(), String> {
    let types: Vec<&str> = match &schema["type"] {
        Value::String(name) => vec![name.as_str()],
        Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let fits = |name: &&str| match *name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        _ => false,
    };
    if !types.is_empty() && !types.iter().any(fits) {
        return Err(format!("`{at}` must be {}", types.join(" or ")));
    }
    if let Value::Array(allowed) = &schema["enum"]
        && !allowed.contains(value)
    {
        let names: Vec<String> = allowed.iter().map(Value::to_string).collect();
        return Err(format!("`{at}` must be one of {}", names.join(", ")));
    }
    match value {
        Value::Object(fields) => {
            for name in schema["required"].as_array().into_iter().flatten() {
                if let Some(name) = name.as_str()
                    && !fields.contains_key(name)
                {
                    return Err(format!("`{at}.{name}` is required"));
                }
            }
            for (name, field) in fields {
                match schema["properties"].get(name) {
                    Some(field_schema) => check(field_schema, field, &format!("{at}.{name}"))?,
                    None if schema["additionalProperties"] == false => {
                        return Err(format!("`{at}` has no field `{name}`"));
                    }
                    None => {}
                }
            }
        }
        Value::Array(items) => {
            if let Some(least) = schema["minItems"].as_u64()
                && (items.len() as u64) < least
            {
                return Err(format!("`{at}` needs at least {least} item(s)"));
            }
            for (index, item) in items.iter().enumerate() {
                check(&schema["items"], item, &format!("{at}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// The tool result after a change was applied: what changed and any problems
/// at the changed elements. Of the new elements, those inside a new
/// definition or usage are counted, not listed (a copied building block
/// brings many), and documentation is left out.
pub fn describe_event(state: &SystemState, event: &ChangeEvent) -> String {
    let tree = state.tree();
    let list = |ids: &[ElementId]| {
        ids.iter()
            .filter_map(|id| {
                let element = tree.get(*id)?;
                Some(format!(
                    "{} ({})",
                    tree.qualified_name(*id),
                    element.kind.keyword()
                ))
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![format!("Applied: {}", event.description)];
    if !event.created.is_empty() {
        let inside_new = |id: &ElementId| {
            tree.get(*id).and_then(Element::owner).is_some_and(|owner| {
                event.created.contains(&owner) && tree[owner].kind != ElementKind::Package
            })
        };
        let shown: Vec<ElementId> = event
            .created
            .iter()
            .copied()
            .filter(|id| {
                !inside_new(id)
                    && tree
                        .get(*id)
                        .is_some_and(|e| !matches!(e.kind, ElementKind::Doc | ElementKind::Comment))
            })
            .collect();
        let members = event
            .created
            .iter()
            .filter(|id| {
                inside_new(id) && tree.get(**id).is_some_and(|e| e.kind != ElementKind::Doc)
            })
            .count();
        let mut line = format!("Created: {}", list(&shown));
        if members > 0 {
            line.push_str(&format!(" (with {members} member(s) inside them)"));
        }
        lines.push(line);
    }
    if !event.updated.is_empty() {
        lines.push(format!("Changed: {}", list(&event.updated)));
    }
    if !event.deleted.is_empty() {
        lines.push(format!("Deleted {} element(s).", event.deleted.len()));
    }
    let touched: Vec<ElementId> = event
        .created
        .iter()
        .chain(&event.updated)
        .copied()
        .collect();
    lines.push(problems(state, Some(&touched)));
    lines.join("\n")
}

/// The tool result after a change was not applied.
pub fn describe_rejection(state: &SystemState, rejection: &Rejection) -> String {
    match rejection {
        Rejection::Locked { elements } => {
            let names: Vec<String> = elements
                .iter()
                .map(|id| state.tree().qualified_name(*id))
                .collect();
            format!(
                "Not applied: the Operator did not allow changes to the locked element(s) {}. Leave them as they are, or explain to the Operator why they must change.",
                names.join(", ")
            )
        }
        Rejection::Stale { .. } => {
            "Not applied: the model changed while you were preparing this change. Read the model again and retry.".to_string()
        }
        other => format!("Not applied: {other}."),
    }
}

fn read_model(state: &SystemState, input: &Value) -> Result<String, String> {
    let tree = state.tree();
    let text = match optional_str(input, "element")? {
        Some(name) => {
            let id = find(tree, name)?;
            print_element(tree, id).ok_or_else(|| format!("`{name}` cannot be printed"))?
        }
        None => outline(state),
    };
    let locked: Vec<String> = state
        .locks()
        .iter()
        .map(|id| tree.qualified_name(*id))
        .collect();
    let locks = if locked.is_empty() {
        "Locked: none.".to_string()
    } else {
        format!("Locked (ask before changing): {}.", locked.join(", "))
    };
    Ok(format!(
        "{text}\n{locks}\nProblems: {}.",
        state.diagnostics().len()
    ))
}

/// The outline `read_model` returns without an element (R-34): one line per
/// element, indented by owner, with its kind and what it refers to, its lock
/// and its problems. Documentation, comments and imports are left out.
fn outline(state: &SystemState) -> String {
    let tree = state.tree();
    let mut problems: std::collections::HashMap<ElementId, usize> = Default::default();
    for diagnostic in state.diagnostics() {
        *problems.entry(diagnostic.element).or_default() += 1;
    }
    let mut lines = vec![
        "Outline (indented by owner; read an element by qualified name for its full text):"
            .to_string(),
    ];
    let mut stack: Vec<(ElementId, usize)> = tree.roots().map(|root| (root, 0)).collect();
    stack.reverse();
    while let Some((id, depth)) = stack.pop() {
        let element = &tree[id];
        if matches!(
            element.kind,
            ElementKind::Doc | ElementKind::Comment | ElementKind::Import
        ) {
            continue;
        }
        let mut line = format!("{}{}", "  ".repeat(depth), outline_line(tree, id, element));
        if state.locks().contains(&id) {
            line.push_str(" [locked]");
        }
        match problems.get(&id) {
            Some(1) => line.push_str(" — 1 problem"),
            Some(count) => line.push_str(&format!(" — {count} problems")),
            None => {}
        }
        lines.push(line);
        for child in element.children().iter().rev() {
            stack.push((*child, depth + 1));
        }
    }
    lines.join("\n")
}

/// One element in the outline: `name (kind : Type [m] :> General, a.b to c.d)`.
fn outline_line(tree: &Tree, id: ElementId, element: &Element) -> String {
    let names = |references: &[Reference]| {
        references
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    match element.kind {
        ElementKind::Satisfy => {
            return match (&element.target, &element.by) {
                (Some(target), Some(by)) => format!("satisfy {target} by {by}"),
                (Some(target), None) => format!("satisfy {target}"),
                _ => "satisfy".to_string(),
            };
        }
        ElementKind::Unsupported => {
            return format!(
                "unsupported {}",
                element.note.as_deref().unwrap_or("construct")
            );
        }
        ElementKind::SyntaxError => return "text that could not be read".to_string(),
        _ => {}
    }
    let name = tree.effective_name(id).unwrap_or("(unnamed)");
    let mut detail = String::new();
    if element.is_abstract {
        detail.push_str("abstract ");
    }
    if element.is_end {
        detail.push_str("end ");
    }
    if let Some(direction) = element.direction {
        detail.push_str(direction.keyword());
        detail.push(' ');
    }
    detail.push_str(element.kind.keyword());
    if !element.typed_by.is_empty() {
        let tilde = if element.conjugated { "~" } else { "" };
        detail.push_str(&format!(" : {tilde}{}", names(&element.typed_by)));
    }
    if let Some(multiplicity) = element.multiplicity {
        detail.push_str(&format!(" {multiplicity}"));
    }
    if !element.specializes.is_empty() {
        detail.push_str(&format!(" :> {}", names(&element.specializes)));
    }
    if !element.redefines.is_empty() && element.name.is_some() {
        detail.push_str(&format!(" :>> {}", names(&element.redefines)));
    }
    if let Some(value) = &element.value {
        detail.push_str(&format!(" = {value}"));
    }
    if element.ends.len() == 2 {
        detail.push_str(&format!(", {} to {}", element.ends[0], element.ends[1]));
    }
    format!("{name} ({detail})")
}

fn find_elements(tree: &Tree, input: &Value) -> Result<String, String> {
    let name = optional_str(input, "name")?.map(str::to_lowercase);
    let kind = optional_str(input, "kind")?.map(kind_from).transpose()?;
    let found: Vec<String> = tree
        .walk()
        .into_iter()
        .filter(|id| kind.is_none_or(|kind| tree[*id].kind == kind))
        .filter_map(|id| {
            let element_name = tree.effective_name(id)?;
            if let Some(name) = &name
                && !element_name.to_lowercase().contains(name.as_str())
            {
                return None;
            }
            Some(format!(
                "{} ({})",
                tree.qualified_name(id),
                tree[id].kind.keyword()
            ))
        })
        .collect();
    Ok(if found.is_empty() {
        "No matching elements.".to_string()
    } else {
        found.join("\n")
    })
}

/// Problems at `only` (or everywhere), one per line.
fn problems(state: &SystemState, only: Option<&[ElementId]>) -> String {
    let tree = state.tree();
    let lines: Vec<String> = state
        .diagnostics()
        .iter()
        .filter(|diagnostic| only.is_none_or(|ids| ids.contains(&diagnostic.element)))
        .map(|diagnostic| {
            format!(
                "- {}: {} [{}]",
                tree.qualified_name(diagnostic.element),
                diagnostic.message,
                diagnostic.code
            )
        })
        .collect();
    match (lines.is_empty(), only.is_some()) {
        (true, true) => "No problems at the changed elements.".to_string(),
        (true, false) => "No problems.".to_string(),
        (false, _) => format!("Problems:\n{}", lines.join("\n")),
    }
}

fn ask_operator(input: &Value) -> Result<Prepared, String> {
    let question = required_str(input, "question")?.to_string();
    let options = match input.get("options") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or("each option must be text")
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err("`options` must be a list of text".to_string()),
    };
    Ok(Prepared::Question { question, options })
}

/// Resolves every operation against a copy of the model, applying each one
/// there so later operations can name elements created by earlier ones. New
/// elements get the same ids when the change is applied to the real state,
/// because the change is pinned to the current revision.
fn prepare_change(state: &SystemState, input: &Value) -> Result<Change, String> {
    let description = required_str(input, "description")?;
    let items = match input.get("operations") {
        Some(Value::Array(items)) if !items.is_empty() => items,
        _ => return Err("`operations` must be a non-empty list".to_string()),
    };
    let mut copy = SystemState::new(state.tree().clone(), state.locks().clone());
    let mut operations = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let failed = |reason: String| format!("operation {}: {reason}", index + 1);
        let mut steps = operations_of(copy.tree(), item).map_err(failed)?;
        let event = try_on(&mut copy, description, &steps).map_err(failed)?;
        // A new element's doc comment is set once the element exists.
        if item.get("op").and_then(Value::as_str) == Some("create")
            && let Some(doc) = optional_str(item, "doc").map_err(failed)?
            && let Some(created) = event.created.first()
        {
            let set = Operation::Set {
                element: *created,
                property: Property::Doc(Some(doc.to_string())),
            };
            try_on(&mut copy, description, std::slice::from_ref(&set)).map_err(failed)?;
            steps.push(set);
        }
        operations.extend(steps);
    }
    let mut change = Change::new(Actor::Assistant, description, operations);
    change.base = Some(state.revision());
    Ok(change)
}

/// Applies operations to the copy of the model; locks are the Operator's to
/// confirm when the whole change is applied for real.
fn try_on(
    copy: &mut SystemState,
    description: &str,
    operations: &[Operation],
) -> Result<ChangeEvent, String> {
    let mut step = Change::new(Actor::Assistant, description, operations.to_vec());
    step.confirmed = copy.locks().iter().copied().collect();
    copy.apply(step).map_err(|rejection| rejection.to_string())
}

fn operations_of(tree: &Tree, item: &Value) -> Result<Vec<Operation>, String> {
    let op = required_str(item, "op")?;
    match op {
        "create" => {
            let parent = parent(tree, item)?;
            let kind = kind_from(required_str(item, "kind")?)?;
            let mut element = Element::new(kind);
            element.name = optional_str(item, "name")?.map(str::to_string);
            element.is_end = item.get("end") == Some(&Value::Bool(true));
            if kind == ElementKind::Satisfy {
                element.target = Some(Reference::new(required_str(item, "requirement")?));
                element.by = optional_str(item, "by")?.map(Reference::new);
            }
            set_fields(&mut element, item)?;
            factory::behaviour_fields(&mut element, item)?;
            let mut operations = vec![Operation::Create {
                parent,
                element: Box::new(element),
            }];
            // What goes inside it: a subject and objective, a state
            // machine's start, a transition's trigger and effect,
            // redefined features.
            operations.extend(factory::members(tree, tree.next_id(), kind, item)?);
            Ok(operations)
        }
        "delete" => Ok(vec![Operation::Delete {
            element: find(tree, required_str(item, "element")?)?,
        }]),
        "rename" => Ok(vec![Operation::Rename {
            element: find(tree, required_str(item, "element")?)?,
            name: required_str(item, "name")?.to_string(),
        }]),
        "move" => Ok(vec![Operation::Move {
            element: find(tree, required_str(item, "element")?)?,
            parent: parent(tree, item)?,
        }]),
        "connect" => {
            let Parent::Element(parent) = parent(tree, item)? else {
                return Err("a connection needs a parent part".to_string());
            };
            let kind = match optional_str(item, "kind")? {
                None | Some("connection") => ElementKind::Connection,
                Some("interface") => ElementKind::Interface,
                Some(other) => {
                    return Err(format!(
                        "connect makes a connection or an interface, not a {other}"
                    ));
                }
            };
            Ok(vec![Operation::Connect {
                parent,
                kind,
                name: optional_str(item, "name")?.map(str::to_string),
                definition: optional_str(item, "definition")?.map(Reference::new),
                from: Reference::new(required_str(item, "from")?),
                to: Reference::new(required_str(item, "to")?),
            }])
        }
        "set" => {
            let element = find(tree, required_str(item, "element")?)?;
            let mut changed = tree[element].clone();
            set_fields(&mut changed, item)?;
            let old = &tree[element];
            let mut properties: Vec<Property> = Vec::new();
            if changed.typed_by != old.typed_by || changed.conjugated != old.conjugated {
                properties.push(Property::TypedBy(changed.typed_by.clone()));
                properties.push(Property::Conjugated(changed.conjugated));
            }
            if changed.specializes != old.specializes {
                properties.push(Property::Specializes(changed.specializes.clone()));
            }
            if changed.redefines != old.redefines {
                properties.push(Property::Redefines(changed.redefines.clone()));
            }
            if changed.multiplicity != old.multiplicity {
                properties.push(Property::Multiplicity(changed.multiplicity));
            }
            if changed.direction != old.direction {
                properties.push(Property::Direction(changed.direction));
            }
            if changed.value != old.value {
                properties.push(Property::Value(changed.value.clone()));
            }
            if let Some(text) = optional_str(item, "expression")? {
                properties.push(Property::Expression(Some(factory::expression(text)?)));
            }
            if let Some(text) = optional_str(item, "guard")? {
                properties.push(Property::Guard(Some(factory::expression(text)?)));
            }
            if item.get("features").is_some() {
                let features = factory::set_features(tree, element, item)?;
                if properties.is_empty() && optional_str(item, "doc")?.is_none() {
                    return Ok(features);
                }
                let mut operations: Vec<Operation> = properties
                    .into_iter()
                    .map(|property| Operation::Set { element, property })
                    .collect();
                operations.extend(features);
                return Ok(operations);
            }
            if let Some(doc) = optional_str(item, "doc")? {
                properties.push(Property::Doc(Some(doc.to_string())));
            }
            if properties.is_empty() {
                return Err("`set` needs at least one property to change".to_string());
            }
            Ok(properties
                .into_iter()
                .map(|property| Operation::Set { element, property })
                .collect())
        }
        other => Err(format!("unknown operation `{other}`")),
    }
}

/// Copies the optional property fields of a create or set operation.
fn set_fields(element: &mut Element, item: &Value) -> Result<(), String> {
    if let Some(ty) = optional_str(item, "type")? {
        let (conjugated, name) = match ty.strip_prefix('~') {
            Some(rest) => (true, rest.trim()),
            None => (false, ty.trim()),
        };
        element.conjugated = conjugated;
        element.typed_by = vec![Reference::new(name)];
    }
    if let Some(names) = optional_list(item, "specializes")? {
        element.specializes = names.iter().map(|name| Reference::new(name)).collect();
    }
    if let Some(names) = optional_list(item, "redefines")? {
        element.redefines = names.iter().map(|name| Reference::new(name)).collect();
    }
    if let Some(text) = optional_str(item, "multiplicity")? {
        element.multiplicity = Some(multiplicity(text)?);
    }
    if let Some(text) = optional_str(item, "direction")? {
        element.direction = Some(match text {
            "in" => Direction::In,
            "out" => Direction::Out,
            "inout" => Direction::InOut,
            other => return Err(format!("unknown direction `{other}`")),
        });
    }
    if let Some(value) = item.get("value") {
        element.value = Some(literal(value)?);
    }
    Ok(())
}

fn parent(tree: &Tree, item: &Value) -> Result<Parent, String> {
    match optional_str(item, "parent")? {
        Some(name) => Ok(Parent::Element(find(tree, name)?)),
        None if !tree.documents().is_empty() => Ok(Parent::Document(0)),
        None => Err("the model has no document to add to".to_string()),
    }
}

fn find(tree: &Tree, name: &str) -> Result<ElementId, String> {
    tree.find(name).ok_or_else(|| {
        format!("there is no element `{name}`; use its qualified name, e.g. `Package::Part`")
    })
}

fn kind_from(keyword: &str) -> Result<ElementKind, String> {
    KINDS
        .iter()
        .copied()
        .find(|kind| kind.keyword() == keyword)
        .ok_or_else(|| format!("unknown kind `{keyword}`"))
}

fn multiplicity(text: &str) -> Result<Multiplicity, String> {
    let number = |text: &str| {
        text.trim()
            .parse::<u64>()
            .map_err(|_| format!("`{text}` is not a multiplicity; use e.g. 1, 0..1 or 1..*"))
    };
    let upper = |text: &str| match text.trim() {
        "*" => Ok(None),
        text => number(text).map(Some),
    };
    Ok(match text.split_once("..") {
        Some((lower, high)) => Multiplicity {
            lower: number(lower)?,
            upper: upper(high)?,
        },
        None if text.trim() == "*" => Multiplicity {
            lower: 0,
            upper: None,
        },
        None => {
            let exact = number(text)?;
            Multiplicity {
                lower: exact,
                upper: Some(exact),
            }
        }
    })
}

fn literal(value: &Value) -> Result<Literal, String> {
    match value {
        Value::Bool(value) => Ok(Literal::Boolean(*value)),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Ok(Literal::Integer(number.to_string()))
        }
        Value::Number(number) => Ok(Literal::Real(number.to_string())),
        Value::String(text) => Ok(Literal::String(text.clone())),
        _ => Err("`value` must be text, a number or true/false".to_string()),
    }
}

fn required_str<'a>(item: &'a Value, field: &str) -> Result<&'a str, String> {
    optional_str(item, field)?.ok_or_else(|| format!("`{field}` is required"))
}

fn optional_str<'a>(item: &'a Value, field: &str) -> Result<Option<&'a str>, String> {
    match item.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(Some(text)),
        Some(_) => Err(format!("`{field}` must be non-empty text")),
    }
}

fn optional_list<'a>(item: &'a Value, field: &str) -> Result<Option<Vec<&'a str>>, String> {
    match item.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| format!("`{field}` must be a list of names"))
            })
            .collect::<Result<_, _>>()
            .map(Some),
        Some(_) => Err(format!("`{field}` must be a list of names")),
    }
}
