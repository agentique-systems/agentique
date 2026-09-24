//! Additive `/api/gen2` HTTP adapter for the Systems Modeling API 1.0 subset.
//!
//! This crate starts no listener and imports no Gen1 application or semantic
//! objects. An embedding host provides authentication and authorization before
//! mounting these routes. Semantic work uses a bounded blocking task pool.
#![forbid(unsafe_code)]

use agq_kernel::ElementId;
use agq_modeling_api::{
    ApiError, ErrorKind, ModelingApi, dto,
    paging::{self, PageBinding},
};
use agq_modeling_repository::{BranchId, ProjectId, ProjectRevisionId};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, OriginalUri, Path, Query, State},
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone)]
struct HttpState {
    api: Arc<ModelingApi>,
    jobs: Arc<Semaphore>,
}

/// Build standalone generation-2 routes with bounded body and semantic concurrency.
///
/// The returned router must be wrapped in the host's access-control middleware
/// before it is exposed to untrusted clients. It never changes Gen1 routes.
pub fn router(api: Arc<ModelingApi>, maximum_concurrent_jobs: usize) -> Router {
    let routes = Router::new()
        .route("/projects", get(projects).post(create_project))
        .route(
            "/projects/{project}",
            get(project).put(unsupported).delete(unsupported),
        )
        .route(
            "/projects/{project}/branches",
            get(branches).post(create_branch),
        )
        .route(
            "/projects/{project}/branches/{branch}",
            get(branch).delete(unsupported),
        )
        .route(
            "/projects/{project}/commits",
            get(commits).post(unsupported),
        )
        .route("/projects/{project}/commits/{revision}", get(commit))
        .route(
            "/projects/{project}/commits/{revision}/elements",
            get(elements),
        )
        .route(
            "/projects/{project}/commits/{revision}/elements/{element}",
            get(element),
        )
        .route("/projects/{project}/commits/{revision}/roots", get(roots))
        .route(
            "/projects/{project}/commits/{revision}/elements/{element}/relationships",
            get(relationships),
        )
        .route("/projects/{project}/commits/{revision}/diff", get(diff))
        .route(
            "/projects/{project}/branches/{branch}/merge",
            post(unsupported),
        )
        .route(
            "/projects/{project}/tags",
            get(unsupported).post(unsupported),
        )
        .route(
            "/projects/{project}/tags/{tag}",
            get(unsupported).delete(unsupported),
        )
        .route(
            "/projects/{project}/queries",
            get(unsupported).post(unsupported),
        )
        .route(
            "/projects/{project}/queries/{query}",
            get(unsupported).put(unsupported).delete(unsupported),
        )
        .route(
            "/projects/{project}/queries/{query}/results",
            get(unsupported),
        )
        .route(
            "/projects/{project}/query-results",
            get(unsupported).post(unsupported),
        )
        .route(
            "/projects/{project}/commits/{revision}/changes",
            get(unsupported),
        )
        .route(
            "/projects/{project}/commits/{revision}/changes/{change}",
            get(unsupported),
        )
        .route(
            "/projects/{project}/commits/{revision}/elements/{element}/projectUsage",
            get(unsupported),
        )
        .route("/meta/datatypes", get(unsupported))
        .route("/meta/datatypes/{datatype}", get(unsupported))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(HttpState {
            api,
            jobs: Arc::new(Semaphore::new(maximum_concurrent_jobs.max(1))),
        });
    Router::new().nest("/api/gen2", routes)
}

#[derive(Debug)]
struct HttpError(ApiError);
impl From<ApiError> for HttpError {
    fn from(error: ApiError) -> Self {
        Self(error)
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = match self.0.kind {
            ErrorKind::BadRequest => StatusCode::BAD_REQUEST,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Unsupported => StatusCode::NOT_IMPLEMENTED,
            ErrorKind::Busy => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(serde_json::json!({"@type": "Error", "description": self.0.message})),
        )
            .into_response()
    }
}

async fn unsupported() -> HttpError {
    ApiError::unsupported("operation outside the supported Gen2 profile; source-backed edits and reviewed merge reconciliation are separate contracts").into()
}

async fn run<T: Send + 'static>(
    state: HttpState,
    operation: impl FnOnce(&ModelingApi) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, HttpError> {
    // Refuse excess work instead of accumulating an unbounded queue of closures.
    let permit = state
        .jobs
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError {
            kind: ErrorKind::Busy,
            message: "modeling request concurrency limit reached".into(),
        })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        operation(&state.api)
    })
    .await
    .map_err(|_| ApiError {
        kind: ErrorKind::Internal,
        message: "modeling request task failed".into(),
    })?
    .map_err(Into::into)
}

fn parse<T: std::str::FromStr>(text: &str) -> Result<T, HttpError> {
    text.parse()
        .map_err(|_| ApiError::bad_request("identifier must be a UUID").into())
}
fn element_id(text: &str) -> Result<ElementId, HttpError> {
    Ok(ElementId::from_u128(parse::<Uuid>(text)?.as_u128()))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    #[serde(rename = "page[size]")]
    size: Option<usize>,
    #[serde(rename = "page[after]")]
    after: Option<String>,
    #[serde(rename = "page[before]")]
    before: Option<String>,
    #[serde(rename = "excludeUsed")]
    exclude_used: Option<bool>,
    direction: Option<String>,
    #[serde(rename = "baseCommitId")]
    base_commit_id: Option<String>,
}

fn revision_binding(
    project: &str,
    revision: &str,
    operation: &str,
) -> Result<PageBinding, HttpError> {
    Ok(PageBinding {
        project: parse(project)?,
        revision: parse(revision)?,
        query: operation.into(),
    })
}

fn collection<T: Serialize>(
    values: &[T],
    binding: PageBinding,
    query: PageQuery,
    uri: Uri,
    partial: bool,
) -> Result<Response, HttpError> {
    if query.exclude_used == Some(false) {
        return Err(ApiError::unsupported(
            "ProjectUsage expansion is not implemented; use excludeUsed=true",
        )
        .into());
    }
    let page = paging::page(
        values,
        &binding,
        query.size,
        query.after.as_deref(),
        query.before.as_deref(),
    )
    .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let mut response = Json(page.values).into_response();
    let mut links = Vec::new();
    let extra = query
        .base_commit_id
        .as_ref()
        .map(|id| format!("&baseCommitId={id}"))
        .unwrap_or_default();
    if let Some(cursor) = page.next {
        links.push(format!(
            "<{}?page[after]={cursor}{extra}>; rel=\"next\"",
            uri.path()
        ));
    }
    if let Some(cursor) = page.previous {
        links.push(format!(
            "<{}?page[before]={cursor}{extra}>; rel=\"prev\"",
            uri.path()
        ));
    }
    if !links.is_empty() {
        response.headers_mut().insert(
            header::LINK,
            HeaderValue::from_str(&links.join(", "))
                .map_err(|_| ApiError::bad_request("invalid continuation link"))?,
        );
    }
    if !binding.revision.is_nil() {
        response.headers_mut().insert(
            "x-agentique-revision",
            HeaderValue::from_str(&binding.revision.to_string()).expect("UUID header"),
        );
    }
    if partial {
        response.headers_mut().insert(
            "x-agentique-projection",
            HeaderValue::from_static("partial-current"),
        );
    }
    Ok(response)
}

fn catalog<T: Serialize>(
    values: &[T],
    project: Uuid,
    operation: &str,
    query: PageQuery,
    uri: Uri,
) -> Result<Response, HttpError> {
    let digest = Sha256::digest(
        serde_json::to_vec(values)
            .map_err(|_| ApiError::bad_request("catalog projection failed"))?,
    );
    // Catalog edits cause an explicit cursor mismatch; immutable semantic pages
    // instead carry their actual revision and continue normally after head moves.
    collection(
        values,
        PageBinding {
            project,
            revision: Uuid::nil(),
            query: format!("{operation}:{digest:x}"),
        },
        query,
        uri,
        false,
    )
}

async fn projects(
    State(state): State<HttpState>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    let values = run(state, |api| api.projects()).await?;
    catalog(&values, Uuid::nil(), "getProjects", query, uri)
}
async fn project(
    State(state): State<HttpState>,
    Path(project): Path<String>,
) -> Result<Json<dto::Project>, HttpError> {
    let id = parse::<ProjectId>(&project)?;
    Ok(Json(run(state, move |api| api.project(id)).await?))
}
async fn create_project(
    State(state): State<HttpState>,
    Json(request): Json<dto::ProjectRequest>,
) -> Result<(StatusCode, Json<dto::Project>), HttpError> {
    Ok((
        StatusCode::CREATED,
        Json(run(state, move |api| api.create_project(request)).await?),
    ))
}
async fn branches(
    State(state): State<HttpState>,
    Path(project): Path<String>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    let id = parse::<ProjectId>(&project)?;
    let values = run(state, move |api| api.branches(id)).await?;
    catalog(
        &values,
        parse(&project)?,
        "getBranchesByProject",
        query,
        uri,
    )
}
async fn branch(
    State(state): State<HttpState>,
    Path((project, branch)): Path<(String, String)>,
) -> Result<Json<dto::Branch>, HttpError> {
    let project = parse::<ProjectId>(&project)?;
    let branch = parse::<BranchId>(&branch)?;
    Ok(Json(
        run(state, move |api| api.branch(project, branch)).await?,
    ))
}
async fn create_branch(
    State(state): State<HttpState>,
    Path(project): Path<String>,
    Json(request): Json<dto::BranchRequest>,
) -> Result<(StatusCode, Json<dto::Branch>), HttpError> {
    let project = parse::<ProjectId>(&project)?;
    Ok((
        StatusCode::CREATED,
        Json(run(state, move |api| api.create_branch(project, request)).await?),
    ))
}
async fn commits(
    State(state): State<HttpState>,
    Path(project): Path<String>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    let id = parse::<ProjectId>(&project)?;
    let values = run(state, move |api| api.commits(id)).await?;
    catalog(&values, parse(&project)?, "getCommitsByProject", query, uri)
}
async fn commit(
    State(state): State<HttpState>,
    Path((project, revision)): Path<(String, String)>,
) -> Result<Json<dto::Commit>, HttpError> {
    let project = parse::<ProjectId>(&project)?;
    let revision = parse::<ProjectRevisionId>(&revision)?;
    Ok(Json(
        run(state, move |api| api.commit(project, revision)).await?,
    ))
}
async fn elements(
    State(state): State<HttpState>,
    Path((project, revision)): Path<(String, String)>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    let binding = revision_binding(&project, &revision, "getElementsByProjectCommit:current")?;
    let project = parse::<ProjectId>(&project)?;
    let revision = parse::<ProjectRevisionId>(&revision)?;
    let values = run(state, move |api| api.elements(project, revision)).await?;
    collection(&values, binding, query, uri, true)
}
async fn roots(
    State(state): State<HttpState>,
    Path((project, revision)): Path<(String, String)>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    let binding = revision_binding(&project, &revision, "getRootsByProjectCommit:current")?;
    let project = parse::<ProjectId>(&project)?;
    let revision = parse::<ProjectRevisionId>(&revision)?;
    let values = run(state, move |api| api.roots(project, revision)).await?;
    collection(&values, binding, query, uri, true)
}
async fn element(
    State(state): State<HttpState>,
    Path((project, revision, element)): Path<(String, String, String)>,
) -> Result<Response, HttpError> {
    let project = parse::<ProjectId>(&project)?;
    let revision_id = parse::<ProjectRevisionId>(&revision)?;
    let element = element_id(&element)?;
    let value = run(state, move |api| api.element(project, revision_id, element)).await?;
    let mut response = Json(value).into_response();
    response.headers_mut().insert(
        "x-agentique-projection",
        HeaderValue::from_static("partial-current"),
    );
    response.headers_mut().insert(
        "x-agentique-revision",
        HeaderValue::from_str(&revision).map_err(|_| ApiError::bad_request("revision header"))?,
    );
    Ok(response)
}
async fn relationships(
    State(state): State<HttpState>,
    Path((project, revision, element)): Path<(String, String, String)>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    if query
        .direction
        .as_deref()
        .is_some_and(|direction| direction != "both")
    {
        return Err(ApiError::unsupported(
            "direction filtering is not implemented; only both is supported",
        )
        .into());
    }
    let binding = revision_binding(
        &project,
        &revision,
        &format!("getRelationshipsByProjectCommitRelatedElement:{element}:current"),
    )?;
    let project = parse::<ProjectId>(&project)?;
    let revision = parse::<ProjectRevisionId>(&revision)?;
    let element = element_id(&element)?;
    let values = run(state, move |api| {
        api.relationships(project, revision, element)
    })
    .await?;
    collection(&values, binding, query, uri, true)
}
async fn diff(
    State(state): State<HttpState>,
    Path((project, revision)): Path<(String, String)>,
    Query(query): Query<PageQuery>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, HttpError> {
    let base = query
        .base_commit_id
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("baseCommitId is required"))?;
    let binding = revision_binding(&project, &revision, &format!("diff:{base}:current"))?;
    let project = parse::<ProjectId>(&project)?;
    let revision = parse::<ProjectRevisionId>(&revision)?;
    let base = parse::<ProjectRevisionId>(base)?;
    let values = run(state, move |api| api.diff(project, base, revision)).await?;
    collection(&values, binding, query, uri, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wire_page_is_array_with_revision_bound_links() {
        let binding = PageBinding {
            project: Uuid::from_u128(1),
            revision: Uuid::from_u128(2),
            query: "elements:current".into(),
        };
        let response = collection(
            &[1, 2, 3],
            binding.clone(),
            PageQuery {
                size: Some(1),
                ..Default::default()
            },
            "/api/gen2/projects/p/commits/r/elements".parse().unwrap(),
            true,
        )
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["x-agentique-revision"],
            binding.revision.to_string()
        );
        let link = response.headers()[header::LINK].to_str().unwrap();
        let cursor = link
            .split("page[after]=")
            .nth(1)
            .unwrap()
            .split('>')
            .next()
            .unwrap();
        assert_eq!(paging::continuation_binding(cursor).unwrap(), binding);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), b"[1]");
    }

    #[tokio::test]
    async fn unsupported_operations_return_normative_error_shape() {
        let response = unsupported().await.into_response();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["@type"], "Error");
        assert!(value["description"].is_string());
        assert_eq!(value.as_object().unwrap().len(), 2);
    }
}
