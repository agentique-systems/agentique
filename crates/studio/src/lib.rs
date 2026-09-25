//! Loopback Studio host: durable Gen2 models, bounded semantic jobs and disposable views.
//! Browser presentation metadata is stored separately from semantic model facts.
#![forbid(unsafe_code)]

mod seed;
mod startup;
#[cfg(test)]
mod tests;

use agq_kernel::ElementId;
use agq_modeling_agent::decision::{
    DecisionModel, DecisionQuestion, DecisionState, DecisionValue, MockViewDecision,
};
use agq_modeling_agent::{AgentCandidate, AgentContext, AgentPolicy, Authority, ModelCommand};
use agq_modeling_repository::{BranchId, ProjectId, ProjectRevisionId};
use agq_modeling_service::{BoundRevision, ModelingService, RevisionSelector, revision_diff};
use agq_modeling_view::ViewDefinition;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::Semaphore;
use tower_http::services::{ServeDir, ServeFile};

/// Paths are explicit deployment inputs; this host never acquires or republishes standards.
#[derive(Clone)]
pub struct StudioConfig {
    pub database: PathBuf,
    pub root: PathBuf,
    pub kerml_cache: Option<PathBuf>,
    pub systems_cache: Option<PathBuf>,
    /// Installed runtime location; defaults to the normal per-user Agentique store.
    pub runtime_dir: Option<PathBuf>,
    /// Explicit accepted runtime bundle override, independently authenticated on load.
    pub bundle: Option<PathBuf>,
}

struct Runtime {
    service: Arc<ModelingService>,
    metadata: Mutex<rusqlite::Connection>,
    candidates: Mutex<BTreeMap<String, Arc<Mutex<StoredCandidate>>>>,
    candidate_slots: Arc<Semaphore>,
}
#[derive(PartialEq, Eq)]
enum CandidatePhase {
    Active,
    CommitAttempted,
    Committed,
    Rejected,
}
struct StoredCandidate {
    candidate: AgentCandidate,
    phase: CandidatePhase,
    slot: Option<tokio::sync::OwnedSemaphorePermit>,
}
impl StoredCandidate {
    fn readable(&self) -> Result<&AgentCandidate, HttpError> {
        if self.phase == CandidatePhase::Rejected {
            Err(HttpError(
                StatusCode::CONFLICT,
                "Candidate was rejected".into(),
            ))
        } else {
            Ok(&self.candidate)
        }
    }
}
#[derive(Clone)]
struct Host {
    runtime: Arc<Mutex<Result<Arc<Runtime>, String>>>,
    jobs: Arc<Semaphore>,
    operator_token: Arc<String>,
    agent_token: Arc<String>,
    config: Arc<StudioConfig>,
    startup: Arc<Mutex<startup::Startup>>,
}

#[derive(Debug)]
struct HttpError(StatusCode, String);
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1,"description":self.1}))).into_response()
    }
}
fn failed(error: impl std::fmt::Display) -> HttpError {
    HttpError(StatusCode::BAD_REQUEST, error.to_string())
}
fn model_error(error: agq_modeling_service::ServiceError) -> HttpError {
    use agq_modeling_repository::RepositoryError;
    use agq_modeling_service::ServiceError;
    let status = match &error {
        ServiceError::Repository(RepositoryError::Conflict { .. }) => StatusCode::CONFLICT,
        ServiceError::Repository(RepositoryError::NotFound(_)) => StatusCode::NOT_FOUND,
        ServiceError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::BAD_REQUEST,
    };
    HttpError(status, error.to_string())
}
fn agent_error(error: agq_modeling_agent::AgentError) -> HttpError {
    match error {
        agq_modeling_agent::AgentError::Service(error) => model_error(error),
        agq_modeling_agent::AgentError::Denied(_) => {
            HttpError(StatusCode::FORBIDDEN, error.to_string())
        }
        _ => failed(error),
    }
}
type HttpResult = Result<Json<Value>, HttpError>;

/// Serve the actual product even when configuration is incomplete; `/session` reports why.
/// This is a local single-operator host. A remote deployment needs its own identity gateway.
pub fn router(config: StudioConfig) -> Router {
    let host = Host {
        runtime: Arc::new(Mutex::new(Err(
            "Restoring accepted standards and durable self-model…".into(),
        ))),
        jobs: Arc::new(Semaphore::new(2)),
        operator_token: Arc::new(
            std::env::var("AGENTIQUE_STUDIO_OPERATOR_TOKEN")
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
        ),
        agent_token: Arc::new(
            std::env::var("AGENTIQUE_STUDIO_AGENT_TOKEN")
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string()),
        ),
        config: Arc::new(config.clone()),
        startup: Arc::new(Mutex::new(startup::Startup::new())),
    };
    let build = config.root.join("console/dist");
    startup::start(host.clone(), None);
    routes(host, build)
}

fn routes(host: Host, build: PathBuf) -> Router {
    Router::new()
        .nest(
            "/api/gen2/studio",
            Router::new()
                .route("/session", get(session))
                .route("/runtime", get(startup::status))
                .route("/runtime/install", post(startup::install))
                .route("/runtime/retry", post(startup::retry))
                .route("/view", post(view))
                .route("/inspect", post(inspect))
                .route("/explain", post(explain))
                .route("/source", post(source))
                .route("/diff", post(diff))
                .route("/agent", post(agent))
                .route("/views", post(save_view))
                .route("/candidates", post(candidate))
                .route("/candidates/{id}/validate", post(validate))
                .route("/candidates/{id}/commit", post(commit))
                .route("/candidates/{id}/inspect", post(candidate_inspect))
                .route("/candidates/{id}/explain", post(candidate_explain))
                .route("/candidates/{id}", axum::routing::delete(reject))
                .layer(DefaultBodyLimit::max(1024 * 1024))
                .layer(middleware::from_fn_with_state(host.clone(), access))
                .with_state(host),
        )
        .fallback_service(ServeDir::new(&build).fallback(ServeFile::new(build.join("index.html"))))
}

async fn access(State(host): State<Host>, request: Request, next: Next) -> Response {
    // Cross-site pages cannot mint/use local operator authority. Native agents use their token.
    if request
        .headers()
        .get("sec-fetch-site")
        .is_some_and(|v| v == "cross-site")
    {
        return HttpError(
            StatusCode::FORBIDDEN,
            "cross-site Studio access denied".into(),
        )
        .into_response();
    }
    if let Some(origin) = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
    {
        let allowed = origin.strip_prefix("http://").is_some_and(|authority| {
            let hostname = authority.split(':').next().unwrap_or_default();
            (hostname == "127.0.0.1" || hostname == "localhost") && !authority.contains('/')
        });
        if !allowed {
            return HttpError(
                StatusCode::FORBIDDEN,
                "Studio is a loopback operator host".into(),
            )
            .into_response();
        }
    }
    if !matches!(
        request.uri().path(),
        "/session" | "/runtime" | "/api/gen2/studio/session" | "/api/gen2/studio/runtime"
    ) && policy(&host, request.headers()).is_err()
    {
        return HttpError(
            StatusCode::UNAUTHORIZED,
            "Studio session token required".into(),
        )
        .into_response();
    }
    next.run(request).await
}
fn policy(host: &Host, headers: &HeaderMap) -> Result<AgentPolicy, HttpError> {
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));
    match token {
        Some(token) if token == host.operator_token.as_str() => Ok(AgentPolicy::operator()),
        Some(token) if token == host.agent_token.as_str() => {
            Ok(AgentPolicy::agent("external-agent"))
        }
        _ => Err(HttpError(
            StatusCode::UNAUTHORIZED,
            "invalid Studio session".into(),
        )),
    }
}
async fn run(
    host: Host,
    operation: impl FnOnce(&Runtime) -> HttpResult + Send + 'static,
) -> HttpResult {
    let runtime = host
        .runtime
        .lock()
        .expect("runtime state")
        .as_ref()
        .map(Arc::clone)
        .map_err(|reason| HttpError(StatusCode::SERVICE_UNAVAILABLE, reason.clone()))?;
    let permit = host.jobs.try_acquire_owned().map_err(|_| {
        HttpError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Semantic work is in progress; the loaded viewport remains available.".into(),
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        operation(&runtime)
    })
    .await
    .map_err(|error| HttpError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
}

fn initialize(config: &StudioConfig, progress: &mut impl FnMut(&str)) -> Result<Runtime, String> {
    let accepted =
        agq_runtime_publications::load(&startup::runtime_config(config), &config.root, |phase| {
            progress(startup::phase_name(phase))
        })
        .map_err(|error| error.to_string())?;
    initialize_with_publications(config, progress, accepted)
}

fn initialize_with_publications(
    config: &StudioConfig,
    progress: &mut impl FnMut(&str),
    accepted: agq_runtime_publications::AuthenticatedRuntime,
) -> Result<Runtime, String> {
    eprintln!("{}", json!({"publication_timings": accepted.timings}));
    progress("opening_repository");
    if let Some(parent) = config.database.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let repository = Arc::new(
        agq_modeling_sqlite::SqliteRepository::open(&config.database).map_err(|e| e.to_string())?,
    );
    let service = Arc::new(ModelingService::new(repository, accepted.systems, 8));
    let metadata = rusqlite::Connection::open(config.database.with_extension("views.sqlite"))
        .map_err(|e| e.to_string())?;
    metadata.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS studio_views (project TEXT NOT NULL, name TEXT NOT NULL, definition TEXT NOT NULL, PRIMARY KEY(project,name));").map_err(|e| e.to_string())?;
    progress("opening_agentique");
    seed::seed(&service, &config.root, &metadata, progress).map_err(|e| e.to_string())?;
    progress("restoring_revision");
    for project in service
        .repository()
        .list_projects()
        .map_err(|e| e.to_string())?
    {
        let head = service
            .repository()
            .get_branch(project.id, project.default_branch)
            .map_err(|error| error.to_string())?
            .head;
        service
            .resolve(project.id, RevisionSelector::Revision(head))
            .map_err(|error| error.to_string())?;
        let view = ViewDefinition::architecture();
        metadata
            .execute(
                "INSERT OR IGNORE INTO studio_views(project,name,definition) VALUES(?1,?2,?3)",
                rusqlite::params![
                    project.id.to_string(),
                    view.name,
                    serde_json::to_string(&view).map_err(|e| e.to_string())?
                ],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(Runtime {
        service,
        metadata: Mutex::new(metadata),
        candidates: Mutex::new(BTreeMap::new()),
        candidate_slots: Arc::new(Semaphore::new(8)),
    })
}

async fn session(State(host): State<Host>, headers: HeaderMap) -> HttpResult {
    // Browser bootstrap receives the local operator capability. Explicit agent credentials
    // never upgrade to operator by calling this endpoint.
    let explicit_agent = headers.contains_key("authorization");
    let actor = if explicit_agent {
        policy(&host, &headers)?
    } else {
        AgentPolicy::operator()
    };
    let token = actor
        .permissions
        .contains(&Authority::Commit)
        .then(|| host.operator_token.as_str().to_owned());
    run(host, move |runtime| {
    let mut projects = runtime.service.repository().list_projects().map_err(failed)?;
    projects.sort_by_key(|project| (project.name != "Agentique",project.id));
    let project = projects.into_iter().next().ok_or_else(|| failed("No durable project available"))?;
        let branches = runtime.service.repository().list_branches(project.id).map_err(failed)?;
        let revision = branches.iter().find(|b| b.id == project.default_branch).ok_or_else(|| failed("Default branch unavailable"))?.head;
        // Validate the selected durable binding before presenting stored Validated status.
        runtime.service.resolve(project.id, RevisionSelector::Revision(revision)).map_err(failed)?;
        let revisions = runtime.service.repository().list_revisions(project.id).map_err(failed)?;
        let mut views = saved_views(runtime, project.id)?;
        if views.is_empty() { views.push(ViewDefinition::architecture()); }
        Ok(Json(json!({"project":project,"branches":branches,"revisions":revisions,"default_revision":revision,"saved_views":views,"session_token":token,"authority":actor.permissions})))
    }).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewRequest {
    project: ProjectId,
    revision: ProjectRevisionId,
    definition: ViewDefinition,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementRequest {
    project: ProjectId,
    revision: ProjectRevisionId,
    element: ElementId,
}
fn bound(
    runtime: &Runtime,
    project: ProjectId,
    revision: ProjectRevisionId,
) -> Result<BoundRevision, HttpError> {
    runtime
        .service
        .resolve(project, RevisionSelector::Revision(revision))
        .map_err(model_error)
}
fn encoded(value: impl Serialize) -> HttpResult {
    Ok(Json(serde_json::to_value(value).map_err(failed)?))
}
async fn view(State(host): State<Host>, Json(request): Json<ViewRequest>) -> HttpResult {
    run(host, move |runtime| {
        encoded(
            agq_modeling_view::project(
                bound(runtime, request.project, request.revision)?.revision(),
                &request.definition,
            )
            .map_err(failed)?,
        )
    })
    .await
}
async fn inspect(State(host): State<Host>, Json(request): Json<ElementRequest>) -> HttpResult {
    run(host, move |runtime| {
        encoded(
            agq_modeling_view::inspect(
                bound(runtime, request.project, request.revision)?.revision(),
                request.element,
            )
            .map_err(failed)?,
        )
    })
    .await
}
async fn explain(State(host): State<Host>, Json(request): Json<ElementRequest>) -> HttpResult {
    run(host, move |runtime| {
        encoded(
            agq_modeling_view::explain(
                bound(runtime, request.project, request.revision)?.revision(),
                request.element,
            )
            .map_err(failed)?,
        )
    })
    .await
}
async fn source(State(host): State<Host>, Json(request): Json<ElementRequest>) -> HttpResult {
    run(host, move |runtime| {
        let revision = bound(runtime, request.project, request.revision)?;
        let origin = revision.source_origin(request.element).ok_or_else(|| failed("No authored source for this element"))?;
        let (path, document) = revision.revision().documents().find(|(_, doc)| doc.id() == origin.document).ok_or_else(|| failed("Source not in authored project"))?;
        Ok(Json(json!({"revision_id":request.revision,"path":path,"source":document.source(),"start":origin.range.start(),"end":origin.range.end()})))
    }).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiffRequest {
    project: ProjectId,
    from: ProjectRevisionId,
    to: ProjectRevisionId,
}
fn diff_value(runtime: &Runtime, request: DiffRequest) -> Result<Value, HttpError> {
    let before = bound(runtime, request.project, request.from)?;
    let after = bound(runtime, request.project, request.to)?;
    let definition = ViewDefinition::semantic_graph();
    Ok(
        json!({"from":request.from,"to":request.to,"before":agq_modeling_view::project(before.revision(),&definition).map_err(failed)?,"after":agq_modeling_view::project(after.revision(),&definition).map_err(failed)?,"diff":revision_diff(&before,&after).map_err(failed)?}),
    )
}
async fn diff(State(host): State<Host>, Json(request): Json<DiffRequest>) -> HttpResult {
    run(host, move |runtime| Ok(Json(diff_value(runtime, request)?))).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentRequest {
    project: ProjectId,
    revision: ProjectRevisionId,
    selection: Vec<ElementId>,
    intent: String,
    compare_to: Option<ProjectRevisionId>,
}
async fn agent(State(host): State<Host>, Json(request): Json<AgentRequest>) -> HttpResult {
    run(host, move |runtime| {
        let revision = bound(runtime, request.project, request.revision)?;
        for id in &request.selection { revision.current_element(*id).map_err(failed)?; }
        let decision = MockViewDecision.decide(&DecisionState { intent: request.intent, revision: request.revision.to_string(), selected_elements: request.selection.iter().map(ToString::to_string).collect() }, &[DecisionQuestion::Choice { id: "operator-view".into(), options: ["Architecture","Requirements","Graph","History"].map(str::to_owned).into() }]).map_err(failed)?;
        let selected = match &decision.answers[0].value { DecisionValue::Choice(value) => value.as_str(), _ => unreachable!() };
        if selected == "History" {
            let from = request.compare_to.or(revision.revision().parent()).ok_or_else(|| failed("Choose an earlier revision for a visual diff"))?;
            return Ok(Json(json!({"message":"Deterministic agent selected a revision comparison.","decision":decision,"diff":diff_value(runtime,DiffRequest { project: request.project, from, to:request.revision })?})));
        }
        let mut definition = match selected { "Requirements" => ViewDefinition::requirements(), "Graph" => ViewDefinition::semantic_graph(), _ => ViewDefinition::architecture() };
        definition.name = "Temporary Agent View".into();
        definition.focus = request.selection.first().copied();
        definition.depth = 2;
        let view = agq_modeling_view::project(revision.revision(), &definition).map_err(failed)?;
        Ok(Json(json!({"message":"Deterministic agent prepared a view of this exact revision.","decision":decision,"view":view})))
    }).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateRequest {
    project: ProjectId,
    branch: BranchId,
    revision: ProjectRevisionId,
    command: ModelCommand,
}
fn candidate_value(
    runtime: &Runtime,
    id: &str,
    candidate: &AgentCandidate,
) -> Result<Value, HttpError> {
    let before = bound(
        runtime,
        candidate.context.project,
        candidate.context.revision,
    )?;
    let after = candidate.prepared().bound_revision();
    Ok(
        json!({"id":id,"revision_id":after.revision().revision(),"base_revision":candidate.context.revision,"validation":if after.validated().is_some(){"Validated"}else{"Working"},"projection":agq_modeling_view::project(after.revision(),&ViewDefinition::semantic_graph()).map_err(failed)?,"changes":revision_diff(&before,&after).map_err(failed)?,"source_preview":candidate.source_preview}),
    )
}
async fn candidate(
    State(host): State<Host>,
    headers: HeaderMap,
    Json(request): Json<CandidateRequest>,
) -> HttpResult {
    let actor = policy(&host, &headers)?;
    run(host, move |runtime| {
        let slot = runtime
            .candidate_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                HttpError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Review or reject retained candidates before proposing more.".into(),
                )
            })?;
        let context = AgentContext {
            project: request.project,
            branch: request.branch,
            revision: request.revision,
            selection: vec![],
        };
        let candidate =
            agq_modeling_agent::propose(&runtime.service, &actor, context, request.command)
                .map_err(agent_error)?;
        let id = uuid::Uuid::new_v4().to_string();
        let value = candidate_value(runtime, &id, &candidate)?;
        let mut candidates = runtime.candidates.lock().expect("candidates");
        // Keep a bounded set of completed handles for exact transport retries. Durable
        // operation receipts remain in the repository after these UI handles expire.
        if candidates.len() >= 32 {
            let expired = candidates.iter().find_map(|(id, entry)| {
                entry
                    .try_lock()
                    .ok()
                    .filter(|entry| entry.phase == CandidatePhase::Committed)
                    .map(|_| id.clone())
            });
            if let Some(expired) = expired {
                candidates.remove(&expired);
            }
        }
        candidates.insert(
            id,
            Arc::new(Mutex::new(StoredCandidate {
                candidate,
                phase: CandidatePhase::Active,
                slot: Some(slot),
            })),
        );
        Ok(Json(value))
    })
    .await
}
fn retained(runtime: &Runtime, id: &str) -> Result<Arc<Mutex<StoredCandidate>>, HttpError> {
    runtime
        .candidates
        .lock()
        .expect("candidates")
        .get(id)
        .cloned()
        .ok_or_else(|| {
            HttpError(
                StatusCode::NOT_FOUND,
                "Candidate no longer available".into(),
            )
        })
}
async fn validate(
    State(host): State<Host>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> HttpResult {
    let actor = policy(&host, &headers)?;
    actor
        .require(Authority::Validate)
        .map_err(|e| HttpError(StatusCode::FORBIDDEN, e.to_string()))?;
    run(host, move |runtime| {
        let retained = retained(runtime, &id)?;
        let mut stored = retained.lock().expect("candidate");
        stored.readable()?;
        if stored.phase == CandidatePhase::CommitAttempted {
            return Err(HttpError(
                StatusCode::CONFLICT,
                "Commit acknowledgement is unresolved; retry the same commit.".into(),
            ));
        }
        stored.candidate.validate(&actor).map_err(agent_error)?;
        Ok(Json(candidate_value(runtime, &id, &stored.candidate)?))
    })
    .await
}
async fn commit(
    State(host): State<Host>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> HttpResult {
    let actor = policy(&host, &headers)?;
    actor
        .require(Authority::Commit)
        .map_err(|e| HttpError(StatusCode::FORBIDDEN, e.to_string()))?;
    run(host, move |runtime| {
        let retained = retained(runtime, &id)?;
        let mut stored = retained.lock().expect("candidate");
        stored.readable()?;
        if stored
            .candidate
            .prepared()
            .bound_revision()
            .validated()
            .is_none()
        {
            return Err(failed("Validate this candidate before commit"));
        }
        stored.phase = CandidatePhase::CommitAttempted;
        match stored.candidate.commit(&runtime.service, &actor) {
            Ok(receipt) => {
                stored.phase = CandidatePhase::Committed;
                stored.slot.take();
                encoded(receipt)
            }
            Err(error) => {
                if matches!(
                    &error,
                    agq_modeling_agent::AgentError::Service(
                        agq_modeling_service::ServiceError::Repository(
                            agq_modeling_repository::RepositoryError::Conflict { .. }
                        )
                    )
                ) {
                    stored.phase = CandidatePhase::Active;
                }
                Err(agent_error(error))
            }
        }
    })
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateElement {
    element: ElementId,
}
async fn candidate_inspect(
    State(host): State<Host>,
    Path(id): Path<String>,
    Json(request): Json<CandidateElement>,
) -> HttpResult {
    run(host, move |runtime| {
        let retained = retained(runtime, &id)?;
        let stored = retained.lock().expect("candidate");
        encoded(
            agq_modeling_view::inspect(stored.readable()?.prepared().revision(), request.element)
                .map_err(failed)?,
        )
    })
    .await
}
async fn candidate_explain(
    State(host): State<Host>,
    Path(id): Path<String>,
    Json(request): Json<CandidateElement>,
) -> HttpResult {
    run(host, move |runtime| {
        let retained = retained(runtime, &id)?;
        let stored = retained.lock().expect("candidate");
        encoded(
            agq_modeling_view::explain(stored.readable()?.prepared().revision(), request.element)
                .map_err(failed)?,
        )
    })
    .await
}
async fn reject(
    State(host): State<Host>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> HttpResult {
    let actor = policy(&host, &headers)?;
    actor.require(Authority::Propose).map_err(failed)?;
    run(host, move |runtime| {
        let retained = retained(runtime, &id)?;
        let mut stored = retained.lock().expect("candidate");
        if stored.phase != CandidatePhase::Active {
            return Err(HttpError(
                StatusCode::CONFLICT,
                "Candidate is committed, rejected, or awaiting commit acknowledgement.".into(),
            ));
        }
        if stored.candidate.actor != actor.actor && actor.require(Authority::Commit).is_err() {
            return Err(HttpError(
                StatusCode::FORBIDDEN,
                "Only the proposing actor or operator may discard this candidate.".into(),
            ));
        }
        stored.phase = CandidatePhase::Rejected;
        stored.slot.take();
        runtime.candidates.lock().expect("candidates").remove(&id);
        Ok(Json(json!({"rejected":id})))
    })
    .await
}

fn saved_views(runtime: &Runtime, project: ProjectId) -> Result<Vec<ViewDefinition>, HttpError> {
    let db = runtime.metadata.lock().expect("metadata");
    let mut statement = db
        .prepare("SELECT definition FROM studio_views WHERE project=? ORDER BY name")
        .map_err(failed)?;
    let rows = statement
        .query_map([project.to_string()], |row| row.get::<_, String>(0))
        .map_err(failed)?;
    rows.map(|row| serde_json::from_str(&row.map_err(failed)?).map_err(failed))
        .collect()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedViewRequest {
    project: ProjectId,
    definition: ViewDefinition,
}
async fn save_view(
    State(host): State<Host>,
    headers: HeaderMap,
    Json(request): Json<SavedViewRequest>,
) -> HttpResult {
    let actor = policy(&host, &headers)?;
    actor.require(Authority::Commit).map_err(failed)?;
    run(host,move |runtime| {
        runtime.service.repository().get_project(request.project).map_err(failed)?;
        if request.definition.version != ViewDefinition::VERSION || request.definition.name.trim().is_empty() || request.definition.name.len() > 120 { return Err(failed("Invalid saved view version or name")); }
        runtime.metadata.lock().expect("metadata").execute("INSERT INTO studio_views(project,name,definition) VALUES(?1,?2,?3) ON CONFLICT(project,name) DO UPDATE SET definition=excluded.definition",rusqlite::params![request.project.to_string(), request.definition.name,serde_json::to_string(&request.definition).map_err(failed)?]).map_err(failed)?;
        encoded(request.definition)
    }).await
}
