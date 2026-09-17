//! Shared application operations. Authority comes from adapter-authenticated Actor values.
use agq_model::*;
use agq_simulation::{Run, Scenario, Status};
use agq_workspace::{Candidate, Edit, Revision};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;

pub const BUILD_ID: &str = concat!(env!("CARGO_PKG_VERSION"), "/", env!("AGQ_BUILD_ID"));
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Operator,
    Assistant,
}
impl Actor {
    pub fn id(&self) -> &str {
        match self {
            Self::Operator => "local-operator",
            Self::Assistant => "assistant-1",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub sequence: u64,
    pub actor_id: String,
    pub command_id: String,
    pub project_id: String,
    pub payload_digest: String,
    pub kind: String,
    pub result: Json,
    pub time: String,
}
#[derive(Clone, Debug)]
pub struct Write {
    pub kind: String,
    pub id: Id,
    pub value: Json,
}
impl Write {
    pub fn new<T: Serialize>(kind: &str, id: &str, value: &T) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            value: serde_json::to_value(value).unwrap(),
        }
    }
}
pub struct Transaction {
    pub writes: Vec<Write>,
    pub event: Event,
    pub receipt: Option<(String, String, Json)>,
}
pub trait Store: Send {
    fn get(&self, kind: &str, id: &str) -> Result<Option<Json>>;
    fn list(&self, kind: &str) -> Result<Vec<Json>>;
    fn transact(&mut self, tx: Transaction) -> Result<u64>;
    fn events(&self, after: u64, limit: usize) -> Result<Vec<Event>>;
    fn receipt(&self, key: &str) -> Result<Option<(String, Json)>>;
    fn latest_sequence(&self) -> Result<u64> {
        Ok(self
            .events(0, usize::MAX)?
            .last()
            .map(|e| e.sequence)
            .unwrap_or(0))
    }
}
#[derive(Default)]
pub struct MemoryStore {
    objects: BTreeMap<(String, String), Json>,
    events: Vec<Event>,
    receipts: BTreeMap<String, (String, Json)>,
}
impl Store for MemoryStore {
    fn get(&self, k: &str, id: &str) -> Result<Option<Json>> {
        Ok(self.objects.get(&(k.into(), id.into())).cloned())
    }
    fn list(&self, k: &str) -> Result<Vec<Json>> {
        Ok(self
            .objects
            .iter()
            .filter(|((kind, _), _)| kind == k)
            .map(|(_, v)| v.clone())
            .collect())
    }
    fn transact(&mut self, mut tx: Transaction) -> Result<u64> {
        tx.event.sequence = self.events.len() as u64 + 1;
        let seq = tx.event.sequence;
        for w in tx.writes {
            self.objects.insert((w.kind, w.id), w.value);
        }
        if let Some((key, digest, result)) = tx.receipt {
            self.receipts.insert(key, (digest, result));
        }
        self.events.push(tx.event);
        Ok(seq)
    }
    fn events(&self, after: u64, limit: usize) -> Result<Vec<Event>> {
        Ok(self
            .events
            .iter()
            .filter(|e| e.sequence > after)
            .take(limit)
            .cloned()
            .collect())
    }
    fn receipt(&self, key: &str) -> Result<Option<(String, Json)>> {
        Ok(self.receipts.get(key).cloned())
    }
    fn latest_sequence(&self) -> Result<u64> {
        Ok(self.events.len() as u64)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    CommitChange {
        proposal_id: Id,
    },
    SaveScenario {
        scenario: Scenario,
    },
    PrepareRun {
        scenario_revision_id: Id,
    },
    ControlRun {
        run_id: Id,
        expected_control_version: u64,
        operation: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    ProposeChange {
        edits: Vec<Edit>,
    },
    RequestApproval {
        action: Action,
    },
    Approve {
        request_id: Id,
        payload_digest: String,
        expires_in_seconds: u64,
    },
    Act {
        action: Action,
        approval_id: Option<Id>,
    },
    SaveDraft {
        file: String,
        source: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub command_id: String,
    pub project_id: Id,
    pub base_revision_id: Id,
    pub payload: Operation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Id,
    pub actor_id: String,
    pub payload_digest: String,
    pub candidate: Candidate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Review {
    pub id: Id,
    pub actor_id: String,
    pub base_revision_id: Id,
    pub action: Action,
    pub payload: Json,
    pub payload_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Approval {
    pub id: Id,
    pub request_id: Id,
    pub actor_id: String,
    pub approver_id: String,
    pub base_revision_id: Id,
    pub payload_digest: String,
    pub expires_at: i64,
    pub consumed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioRevision {
    pub id: Id,
    pub model_revision_id: Id,
    pub digest: String,
    pub scenario: Scenario,
}
pub struct Application {
    pub store: Box<dyn Store>,
    pub head: Revision,
    pub project_id: Id,
    runs: BTreeMap<Id, Run>,
}
fn checkpoint_id(run: &Run) -> String {
    format!(
        "{}:{}:{}:{:?}",
        run.id, run.control_version, run.semantic_steps, run.status
    )
    .to_lowercase()
}
fn decode<T: serde::de::DeserializeOwned>(value: Json) -> Result<T> {
    serde_json::from_value(value).map_err(|e| Error::new("corrupt_data", e.to_string()))
}
impl Application {
    pub fn open(store: Box<dyn Store>, sources: BTreeMap<String, String>) -> Result<Self> {
        Self::open_session(store, sources, true)
    }
    /// CLI invocations attach to durable paused experiments; server startup recovers all unfinished runs.
    pub fn open_session(
        store: Box<dyn Store>,
        sources: BTreeMap<String, String>,
        recover_all: bool,
    ) -> Result<Self> {
        Self::open_revision_session(store, sources, None, recover_all)
    }
    /// Create a workspace from a fully validated interchange revision in one durable transaction.
    pub fn import(store: Box<dyn Store>, revision: Revision) -> Result<Self> {
        if store.get("workspace", "local")?.is_some() {
            return Err(Error::new(
                "workspace_exists",
                "Import requires a new workspace",
            ));
        }
        if !revision.model.accepted() {
            return Err(Error::new(
                "invalid_model",
                "Cannot import an invalid revision",
            ));
        }
        Self::open_revision_session(store, BTreeMap::new(), Some(revision), false)
    }
    fn open_revision_session(
        mut store: Box<dyn Store>,
        sources: BTreeMap<String, String>,
        initial: Option<Revision>,
        recover_all: bool,
    ) -> Result<Self> {
        let metadata = store.get("workspace", "local")?;
        let (project_id, head) = if let Some(meta) = metadata {
            let id = meta["head"]
                .as_str()
                .ok_or_else(|| Error::new("corrupt_data", "Missing head"))?;
            let head = decode(
                store
                    .get("revision", id)?
                    .ok_or_else(|| Error::new("corrupt_data", "Missing accepted revision"))?,
            )?;
            (
                meta["project_id"].as_str().unwrap_or("local").to_string(),
                head,
            )
        } else {
            let revision = match initial {
                Some(r) => r,
                None => Revision::import(sources, &BTreeMap::new())?,
            };
            let project = new_id();
            let result = json!({"head":revision.id,"project_id":project});
            store.transact(Transaction {
                writes: vec![
                    Write::new("revision", &revision.id, &revision),
                    Write::new("workspace", "local", &result),
                ],
                event: Event {
                    sequence: 0,
                    actor_id: "local-operator".into(),
                    command_id: new_id(),
                    project_id: project.clone(),
                    payload_digest: revision.model.source_digest(),
                    kind: "workspace_created".into(),
                    result,
                    time: chrono::Utc::now().to_rfc3339(),
                },
                receipt: None,
            })?;
            (project, revision)
        };
        let mut app = Self {
            store,
            head,
            project_id,
            runs: BTreeMap::new(),
        };
        for mut value in app.store.list("run")? {
            if value.get("plan").is_none() {
                value["plan"] = app
                    .store
                    .get(
                        "manifest",
                        value["id"]
                            .as_str()
                            .ok_or_else(|| Error::new("corrupt_data", "Run identity missing"))?,
                    )?
                    .ok_or_else(|| Error::new("corrupt_data", "Run manifest missing"))?;
            }
            let mut run: Run = decode(value)?;
            if run.trace.is_empty() {
                run.trace = std::sync::Arc::new(
                    app.store
                        .list(&format!("trace:{}", run.id))?
                        .into_iter()
                        .map(decode)
                        .collect::<Result<_>>()?,
                );
            }
            app.runs.insert(run.id.clone(), run.clone());
            if !run.status.terminal() && (recover_all || run.status == Status::Running) {
                run.interrupt();
                app.persist_system_run(&run, "interrupted")?;
            }
        }
        Ok(app)
    }
    pub fn revision(&self, id: &str) -> Result<Revision> {
        if id == self.head.id {
            return Ok(self.head.clone());
        }
        decode(
            self.store
                .get("revision", id)?
                .ok_or_else(|| Error::new("not_found", "Unknown model revision"))?,
        )
    }
    pub fn run(&self, id: &str) -> Result<Run> {
        if let Some(run) = self.runs.get(id) {
            return Ok(run.clone());
        }
        decode(
            self.store
                .get("run", id)?
                .ok_or_else(|| Error::new("not_found", "Unknown run"))?,
        )
    }
    pub fn inspect(&self, revision: &str, selection: Option<&str>) -> Result<Json> {
        let r = self.revision(revision)?;
        let mut elements: Vec<_> = r
            .model
            .elements
            .values()
            .filter(|e| !e.library && !e.is_implied)
            .cloned()
            .collect();
        elements.sort_by(|a, b| (&a.span.file, a.span.start).cmp(&(&b.span.file, b.span.start)));
        let selected = selection
            .map(|s| agq_semantics::locate(&r.model, s))
            .transpose()?;
        Ok(
            json!({"project_id":self.project_id,"revision_id":r.id,"parent_revision_id":r.parent,"source_digest":r.model.source_digest(),"library_digest":r.model.library_digest,"elements":elements,"implied_elements":r.model.elements.values().filter(|e|e.is_implied).collect::<Vec<_>>(),"relationships":r.model.relationships,"diagnostics":r.model.diagnostics,"selected":selected,"sources":r.model.sources,"provenance":"authored_model"}),
        )
    }
    pub fn read_run(&self, id: &str, from: usize, limit: usize) -> Result<Json> {
        let run = self.run(id)?;
        let end = (from.saturating_add(limit.min(1000))).min(run.trace.len());
        let trace = run.trace.get(from..end).unwrap_or(&[]);
        Ok(
            json!({"id":run.id,"model_revision_id":run.plan.model_revision_id,"scenario_revision_id":run.plan.scenario_revision_id,"status":run.status,"control_version":run.control_version,"active":run.active,"active_name":run.active.as_ref().and_then(|s|run.plan.states.get(s)),"occurrence_id":run.occurrence_id,"next_input":run.next_input,"semantic_steps":run.semantic_steps,"stop_reason":run.stop_reason,"trace_page":trace,"next_sequence":end,"trace_count":run.trace.len(),"trace_digest":run.trace_digest(),"checks":run.checks,"diagnostics":run.diagnostics,"plan":run.plan,"provenance":"run"}),
        )
    }
    pub fn snapshot(&self) -> Result<Json> {
        let mut runs = vec![];
        for run in self.runs.values() {
            runs.push(json!({"id":run.id,"model_revision_id":run.plan.model_revision_id,"status":run.status,"control_version":run.control_version,"active":run.active,"stop_reason":run.stop_reason}));
        }
        Ok(
            json!({"project_id":self.project_id,"head_revision_id":self.head.id,"model":self.inspect(&self.head.id,None)?,"runs":runs,"scenarios":self.store.list("scenario")?,"reviews":self.store.list("review")?,"proposals":self.store.list("proposal")?.iter().map(|p|json!({"id":p["id"],"actor_id":p["actor_id"],"payload_digest":p["payload_digest"],"base_revision_id":p["candidate"]["base_revision_id"],"diff":p["candidate"]["diff"],"diagnostics":p["candidate"]["model"]["diagnostics"]})).collect::<Vec<_>>(),"drafts":self.store.list("draft")?,"event_cursor":self.store.latest_sequence()?}),
        )
    }
    fn review_payload(&self, action: &Action) -> Result<Json> {
        match action {
            Action::CommitChange { proposal_id } => {
                let p: Proposal = decode(
                    self.store
                        .get("proposal", proposal_id)?
                        .ok_or_else(|| Error::new("not_found", "Unknown proposal"))?,
                )?;
                Ok(
                    json!({"action":action,"edits":p.candidate.edits,"diff":p.candidate.diff,"candidate_digest":p.candidate.model.source_digest(),"base_revision_id":p.candidate.base_revision_id}),
                )
            }
            _ => Ok(json!({"action":action})),
        }
    }
    pub fn command(&mut self, actor: Actor, command: Command) -> Result<Json> {
        self.command_controlled(actor, command, &WorkControl::default())
    }
    pub fn command_controlled(
        &mut self,
        actor: Actor,
        command: Command,
        work: &WorkControl,
    ) -> Result<Json> {
        work.check()?;
        if command.command_id.is_empty() || command.command_id.len() > 128 {
            return Err(Error::new(
                "invalid_command",
                "Command ID must contain 1–128 bytes",
            ));
        }
        if command.project_id != self.project_id {
            return Err(Error::new("permission_denied", "Wrong project scope"));
        }
        let payload_digest = json_digest(&command);
        let receipt_key = format!("{}:{}", actor.id(), command.command_id);
        if let Some((old, result)) = self.store.receipt(&receipt_key)? {
            if old != payload_digest {
                return Err(Error::new(
                    "idempotency_conflict",
                    "Command ID reused with a different payload",
                ));
            }
            return Ok(result);
        }
        if command.base_revision_id != self.head.id {
            return Err(Error::new("revision_conflict", "Base revision is stale"));
        }
        let mut writes = vec![];
        let mut new_head = None;
        let mut changed_run = None;
        let result = match &command.payload {
            Operation::ProposeChange { edits } => {
                let candidate = agq_workspace::propose_controlled(&self.head, edits.clone(), work)?;
                let p = Proposal {
                    id: new_id(),
                    actor_id: actor.id().into(),
                    payload_digest: json_digest(edits),
                    candidate,
                };
                let result = json!({"proposal_id":p.id,"base_revision_id":command.base_revision_id,"payload_digest":p.payload_digest,"diff":p.candidate.diff,"diagnostics":p.candidate.model.diagnostics,"valid":p.candidate.model.accepted(),"provenance":"proposal"});
                writes.push(Write::new("proposal", &p.id, &p));
                result
            }
            Operation::RequestApproval { action } => {
                let payload = self.review_payload(action)?;
                let review = Review {
                    id: new_id(),
                    actor_id: actor.id().into(),
                    base_revision_id: self.head.id.clone(),
                    action: action.clone(),
                    payload_digest: json_digest(&payload),
                    payload,
                };
                let result = serde_json::to_value(&review).unwrap();
                writes.push(Write::new("review", &review.id, &review));
                result
            }
            Operation::Approve {
                request_id,
                payload_digest,
                expires_in_seconds,
            } => {
                if actor != Actor::Operator {
                    return Err(Error::new(
                        "permission_denied",
                        "Only the Human Operator can approve",
                    ));
                }
                let review: Review = decode(
                    self.store
                        .get("review", request_id)?
                        .ok_or_else(|| Error::new("not_found", "Unknown review"))?,
                )?;
                if &review.payload_digest != payload_digest
                    || review.base_revision_id != self.head.id
                {
                    return Err(Error::new(
                        "approval_mismatch",
                        "Reviewed payload or revision does not match",
                    ));
                }
                if *expires_in_seconds == 0 || *expires_in_seconds > 3600 {
                    return Err(Error::new(
                        "invalid_command",
                        "Approval expiry must be 1–3600 seconds",
                    ));
                }
                let approval = Approval {
                    id: new_id(),
                    request_id: request_id.clone(),
                    actor_id: review.actor_id,
                    approver_id: actor.id().into(),
                    base_revision_id: self.head.id.clone(),
                    payload_digest: payload_digest.clone(),
                    expires_at: chrono::Utc::now().timestamp() + *expires_in_seconds as i64,
                    consumed: false,
                };
                let result = serde_json::to_value(&approval).unwrap();
                writes.push(Write::new("approval", &approval.id, &approval));
                result
            }
            Operation::SaveDraft { file, source } => {
                if !agq_workspace::safe_source_name(file) {
                    return Err(Error::new("invalid_edit", "Unsafe draft path"));
                }
                let mut sources = self.head.model.sources.clone();
                sources.insert(file.clone(), source.clone());
                let model = agq_semantics::compile_controlled(
                    sources,
                    &agq_semantics::identity_map(&self.head.model),
                    work,
                );
                let draft = json!({"file":file,"source":source,"base_revision_id":self.head.id,"diagnostics":model.diagnostics,"accepted_unchanged":true});
                writes.push(Write::new("draft", file, &draft));
                draft
            }
            Operation::Act {
                action,
                approval_id,
            } => {
                if actor == Actor::Assistant {
                    let approval_id = approval_id.as_ref().ok_or_else(|| {
                        Error::new(
                            "approval_required",
                            "Assistant mutation/execution requires Human Operator approval",
                        )
                    })?;
                    let mut approval: Approval = decode(
                        self.store
                            .get("approval", approval_id)?
                            .ok_or_else(|| Error::new("approval_mismatch", "Unknown approval"))?,
                    )?;
                    if approval.actor_id != actor.id()
                        || approval.base_revision_id != self.head.id
                        || approval.payload_digest != json_digest(&self.review_payload(action)?)
                        || approval.consumed
                        || approval.expires_at <= chrono::Utc::now().timestamp()
                    {
                        return Err(Error::new(
                            "approval_mismatch",
                            "Approval is stale, altered, expired, consumed, or bound to another actor",
                        ));
                    }
                    approval.consumed = true;
                    writes.push(Write::new("approval", approval_id, &approval));
                }
                match action {
                    Action::CommitChange { proposal_id } => {
                        let proposal: Proposal = decode(
                            self.store
                                .get("proposal", proposal_id)?
                                .ok_or_else(|| Error::new("not_found", "Unknown proposal"))?,
                        )?;
                        let r = agq_workspace::commit(&self.head, &proposal.candidate)?;
                        let result = json!({"new_revision_id":r.id,"changed_element_ids":r.model.elements.values().filter(|e|!e.library).map(|e|e.id.clone()).collect::<Vec<_>>()});
                        writes.push(Write::new("revision", &r.id, &r));
                        writes.push(Write::new(
                            "workspace",
                            "local",
                            &json!({"head":r.id,"project_id":self.project_id}),
                        ));
                        new_head = Some(r);
                        result
                    }
                    Action::SaveScenario { scenario } => {
                        let r = ScenarioRevision {
                            id: new_id(),
                            model_revision_id: self.head.id.clone(),
                            digest: json_digest(scenario),
                            scenario: scenario.clone(),
                        };
                        let result = serde_json::to_value(&r).unwrap();
                        writes.push(Write::new("scenario", &r.id, &r));
                        result
                    }
                    Action::PrepareRun {
                        scenario_revision_id,
                    } => {
                        let scenario: ScenarioRevision = decode(
                            self.store
                                .get("scenario", scenario_revision_id)?
                                .ok_or_else(|| {
                                    Error::new("not_found", "Unknown scenario revision")
                                })?,
                        )?;
                        if scenario.model_revision_id != self.head.id {
                            return Err(Error::new(
                                "revision_conflict",
                                "Scenario is bound to another model revision; save a new scenario",
                            ));
                        }
                        let plan = agq_simulation::prepare(
                            &self.head.model,
                            &self.head.id,
                            &scenario.id,
                            &scenario.scenario,
                            BUILD_ID,
                        )?;
                        let run = Run::new(plan);
                        let result = json!({"prepared_run_id":run.id,"control_version":run.control_version,"status":run.status,"plan":run.plan});
                        writes.extend(self.run_writes(&run));
                        changed_run = Some(run);
                        result
                    }
                    Action::ControlRun {
                        run_id,
                        expected_control_version,
                        operation,
                    } => {
                        let mut run = self.run(run_id)?;
                        if ["initialise", "run", "step", "reset_as_new_run"]
                            .contains(&operation.as_str())
                            && run.plan.engine_build != BUILD_ID
                        {
                            return Err(Error::new(
                                "dependency_conflict",
                                "Execution requires the pinned Engine build; the original run remains inspectable",
                            ));
                        }
                        if run.control_version != *expected_control_version {
                            return Err(Error::new(
                                "control_conflict",
                                "Run control version is stale",
                            ));
                        }
                        if operation == "reset_as_new_run" {
                            if run.plan.engine_build != BUILD_ID
                                || run.plan.library_digest != self.head.model.library_digest
                            {
                                return Err(Error::new(
                                    "dependency_conflict",
                                    "Reset requires the pinned Engine build and libraries",
                                ));
                            }
                            let reset = Run::new((*run.plan).clone());
                            let result = json!({"run_id":reset.id,"control_version":0,"run_status":reset.status});
                            writes.extend(self.run_writes(&reset));
                            changed_run = Some(reset);
                            result
                        } else {
                            run.control(operation)?;
                            let result = json!({"run_id":run.id,"control_version":run.control_version,"run_status":run.status,"checkpoint_id":checkpoint_id(&run)});
                            writes.extend(self.run_writes(&run));
                            changed_run = Some(run);
                            result
                        }
                    }
                }
            }
        };
        work.begin_commit()?;
        self.store.transact(Transaction {
            writes,
            event: Event {
                sequence: 0,
                actor_id: actor.id().into(),
                command_id: command.command_id.clone(),
                project_id: self.project_id.clone(),
                payload_digest: payload_digest.clone(),
                kind: "command_committed".into(),
                result: result.clone(),
                time: chrono::Utc::now().to_rfc3339(),
            },
            receipt: Some((receipt_key, payload_digest, result.clone())),
        })?;
        if let Some(head) = new_head {
            self.head = head
        }
        if let Some(run) = changed_run {
            self.runs.insert(run.id.clone(), run);
        }
        Ok(result)
    }
    fn persist_system_run(&mut self, run: &Run, kind: &str) -> Result<()> {
        let checkpoint = checkpoint_id(run);
        let writes = self.run_writes(run);
        self.store.transact(Transaction{writes,event:Event{sequence:0,actor_id:"engine".into(),command_id:new_id(),project_id:self.project_id.clone(),payload_digest:json_digest(&json!({"run":run.id,"step":run.semantic_steps,"cursor":run.next_input,"status":run.status})),kind:kind.into(),result:json!({"run_id":run.id,"control_version":run.control_version,"status":run.status,"checkpoint_id":checkpoint}),time:chrono::Utc::now().to_rfc3339()},receipt:None})?;
        self.runs.insert(run.id.clone(), run.clone());
        Ok(())
    }
    fn run_writes(&self, run: &Run) -> Vec<Write> {
        let record = json!({"id":run.id,"status":run.status,"control_version":run.control_version,"active":run.active,"occurrence_id":run.occurrence_id,"next_input":run.next_input,"semantic_steps":run.semantic_steps,"trace":[],"stop_reason":run.stop_reason,"diagnostics":run.diagnostics,"checks":run.checks});
        let mut checkpoint = record.clone();
        checkpoint["trace_cursor"] = json!(run.trace.len());
        let mut writes = vec![
            Write::new("run", &run.id, &record),
            Write::new("checkpoint", &checkpoint_id(run), &checkpoint),
        ];
        if !self.runs.contains_key(&run.id) {
            writes.push(Write::new("manifest", &run.id, &run.plan));
        }
        let from = self.runs.get(&run.id).map(|r| r.trace.len()).unwrap_or(0);
        for t in &run.trace[from..] {
            writes.push(Write::new(
                &format!("trace:{}", run.id),
                &format!("{:012}", t.sequence),
                t,
            ));
        }
        writes
    }
    pub fn tick(&mut self, run_id: &str) -> Result<bool> {
        let mut run = self.run(run_id)?;
        if run.status != Status::Running {
            return Ok(false);
        }
        run.step()?;
        self.persist_system_run(&run, "run_step")?;
        Ok(run.status == Status::Running)
    }
    pub fn export(&self, revision_id: &str) -> Result<Vec<u8>> {
        agq_workspace::export_kpar(&self.revision(revision_id)?)
    }
}
