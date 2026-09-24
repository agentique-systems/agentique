mod jobs;
mod model_api;
use agq_application::{Actor, Application, Command, Operation};
use agq_model::{Error, Result, new_id};
use axum::{
    Json, Router,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = ".workspaces/local.db")]
    workspace: PathBuf,
    #[arg(long, default_value = "models")]
    models: PathBuf,
    #[arg(long, default_value_t = 7331)]
    port: u16,
    #[arg(long, default_value = "console/dist")]
    console: PathBuf,
}
#[derive(Clone)]
struct Server {
    app: Arc<Mutex<Application>>,
    token: String,
    jobs: jobs::Jobs,
    workers: Arc<tokio::sync::Semaphore>,
}
struct ApiError(Error);
impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code.as_str() {
            "permission_denied" => StatusCode::FORBIDDEN,
            "not_found" => StatusCode::NOT_FOUND,
            "revision_conflict"
            | "control_conflict"
            | "idempotency_conflict"
            | "approval_mismatch" => StatusCode::CONFLICT,
            "provider_unavailable" => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        };
        (status, Json(self.0)).into_response()
    }
}
type ApiResult<T> = std::result::Result<Json<T>, ApiError>;
async fn work<T: Send + 'static>(
    s: Server,
    f: impl FnOnce(&mut Application) -> Result<T> + Send + 'static,
) -> std::result::Result<T, ApiError> {
    let permit = s.workers.clone().try_acquire_owned().map_err(|_| {
        ApiError(Error::new(
            "resource_limit",
            "Application background work queue is full",
        ))
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut app = s
            .app
            .lock()
            .map_err(|_| Error::new("internal_error", "Application lock poisoned"))?;
        f(&mut app)
    })
    .await
    .map_err(|_| ApiError(Error::new("internal_error", "Background operation failed")))?
    .map_err(ApiError)
}
async fn auth(State(s): State<Server>, request: Request, next: Next) -> Response {
    let bearer = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let equal = bearer.len() == s.token.len()
        && bearer
            .bytes()
            .zip(s.token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0;
    if !equal {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"code":"unauthenticated","message":"Local session credential required"})),
        )
            .into_response();
    }
    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|x| x.to_str().ok())
    {
        let host = request
            .headers()
            .get(header::HOST)
            .and_then(|x| x.to_str().ok())
            .unwrap_or("");
        if origin != format!("http://{host}") {
            return (StatusCode::FORBIDDEN, "Origin does not match local session").into_response();
        }
    }
    next.run(request).await
}
async fn state(State(s): State<Server>) -> ApiResult<Value> {
    let mode = std::env::var("AGENTIQUE_ASSISTANT").unwrap_or_else(|_| "test".into());
    let mut value = work(s, |a| a.snapshot()).await?;
    value["assistant_mode"] = json!(mode);
    Ok(Json(value))
}
async fn command(State(s): State<Server>, Json(c): Json<Command>) -> ApiResult<Value> {
    Ok(Json(work(s, move |a| a.command(Actor::Operator, c)).await?))
}
#[derive(Deserialize)]
struct InspectQuery {
    revision_id: String,
    selection: Option<String>,
}
async fn inspect(State(s): State<Server>, Query(q): Query<InspectQuery>) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| {
            a.inspect(&q.revision_id, q.selection.as_deref())
        })
        .await?,
    ))
}
#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    from: usize,
    #[serde(default = "page_size")]
    limit: usize,
}
fn page_size() -> usize {
    100
}
async fn run(
    State(s): State<Server>,
    Path(id): Path<String>,
    Query(p): Query<Page>,
) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| a.read_run(&id, p.from, p.limit)).await?,
    ))
}
#[derive(Deserialize)]
struct Cursor {
    #[serde(default)]
    after: u64,
    #[serde(default = "page_size")]
    limit: usize,
}
async fn events(State(s): State<Server>, Query(q): Query<Cursor>) -> ApiResult<Value> {
    Ok(Json(work(s,move|a|{let events=a.store.events(q.after,q.limit.min(1000))?;Ok(json!({"events":events,"next_cursor":events.last().map(|e|e.sequence).unwrap_or(q.after)}))}).await?))
}
async fn export(
    State(s): State<Server>,
    Path(id): Path<String>,
) -> std::result::Result<Response, ApiError> {
    let bytes = work(s, move |a| a.export(&id)).await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/zip".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        "attachment; filename=Agentique.kpar".parse().unwrap(),
    );
    Ok((headers, bytes).into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chat {
    message: String,
    context: agq_assistant::Context,
}
async fn chat(State(s): State<Server>, Json(c): Json<Chat>) -> ApiResult<agq_assistant::Reply> {
    if c.message.len() > 8000 {
        return Err(Error::new("resource_limit", "Message too long").into());
    }
    let tool = agq_assistant::choose(&c.message, &c.context).await?;
    Ok(Json(
        work(s, move |a| agq_assistant::execute(a, c.context, tool)).await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteReview {
    request_id: String,
    approval_id: String,
    command_id: String,
    base_revision_id: String,
}
async fn execute_review(State(s): State<Server>, Json(c): Json<ExecuteReview>) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| {
            let r: agq_application::Review = serde_json::from_value(
                a.store
                    .get("review", &c.request_id)?
                    .ok_or_else(|| Error::new("not_found", "Unknown review"))?,
            )
            .map_err(|e| Error::new("corrupt_data", e.to_string()))?;
            a.command(
                Actor::Assistant,
                Command {
                    command_id: c.command_id,
                    project_id: a.project_id.clone(),
                    base_revision_id: c.base_revision_id,
                    payload: Operation::Act {
                        action: r.action,
                        approval_id: Some(c.approval_id),
                    },
                },
            )
        })
        .await?,
    ))
}
async fn templates() -> Json<Value> {
    Json(
        json!({"accepted":serde_json::from_str::<Value>(include_str!("../../../scenarios/accepted.json")).unwrap(),"rejected":serde_json::from_str::<Value>(include_str!("../../../scenarios/rejected.json")).unwrap()}),
    )
}

fn require_project(a: &Application, id: &str) -> Result<()> {
    if a.project_id == id {
        Ok(())
    } else {
        Err(Error::new("not_found", "Unknown project"))
    }
}
fn sources(dir: &PathBuf) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for e in std::fs::read_dir(dir).map_err(|e| Error::new("io_error", e.to_string()))? {
        let p = e.map_err(|e| Error::new("io_error", e.to_string()))?.path();
        if p.extension().is_some_and(|x| x == "sysml" || x == "kerml") {
            result.insert(
                format!("models/{}", p.file_name().unwrap().to_string_lossy()),
                std::fs::read_to_string(&p).map_err(|e| Error::new("io_error", e.to_string()))?,
            );
        }
    }
    Ok(result)
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let token = std::env::var("AGENTIQUE_SESSION_TOKEN").unwrap_or_else(|_| new_id() + &new_id());
    if token.len() < 16 {
        return Err("Session credential must contain at least 16 characters".into());
    }
    let app = Application::open(
        Box::new(agq_storage::SqliteStore::open(&args.workspace)?),
        sources(&args.models)?,
    )?;
    let s = Server {
        app: Arc::new(Mutex::new(app)),
        token: token.clone(),
        jobs: Arc::new(Mutex::new(BTreeMap::new())),
        workers: Arc::new(tokio::sync::Semaphore::new(16)),
    };
    let worker = s.clone();
    tokio::spawn(async move {
        loop {
            let state = worker.clone();
            if let Err(error) = work(state, |a| {
                let ids: Vec<_> = a
                    .store
                    .list("run")?
                    .iter()
                    .filter(|r| r["status"] == "running")
                    .filter_map(|r| r["id"].as_str().map(str::to_string))
                    .collect();
                for id in ids {
                    a.tick(&id)?;
                }
                Ok(())
            })
            .await
            {
                eprintln!(
                    "Run checkpoint could not commit: {}: {}",
                    error.0.code, error.0.message
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });
    let internal = Router::new()
        .merge(jobs::routes())
        .route("/state", get(state))
        .route("/inspect", get(inspect))
        .route("/commands", post(command))
        .route("/runs/{id}", get(run))
        .route("/events", get(events))
        .route("/export/{id}", get(export))
        .route("/assistant", post(chat))
        .route("/assistant/execute", post(execute_review))
        .route("/templates", get(templates));
    let standard = model_api::routes();
    let api = Router::new()
        .nest("/api/agentique", internal)
        .nest("/api/model", standard)
        .route_layer(middleware::from_fn_with_state(s.clone(), auth));
    let router = api
        .fallback_service(
            tower_http::services::ServeDir::new(&args.console)
                .append_index_html_on_directories(true)
                .fallback(tower_http::services::ServeFile::new(
                    args.console.join("index.html"),
                )),
        )
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(s);
    let listener =
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, args.port)).await?;
    println!(
        "Agentique Console: http://127.0.0.1:{}/expert#token={}\nWorkspace: {}\nAssistant: {}",
        args.port,
        token,
        args.workspace.display(),
        std::env::var("AGENTIQUE_ASSISTANT").unwrap_or_else(|_| "test (deterministic)".into())
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
