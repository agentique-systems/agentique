//! Full HTTP fixture uses accepted cache inputs only; it never publishes standards.
use agq_kerml_text::{
    ProjectChange, SourceLanguage, library::CanonicalKermlStandardLibraries,
    sysml::CanonicalSysmlSystemsLibrary,
};
use agq_modeling_api::ModelingApi;
use agq_modeling_repository::{OperationId, ProjectId};
use agq_modeling_service::{ApplyDocumentChanges, ModelingService};
use agq_modeling_sqlite::SqliteRepository;
use agq_standard_libraries::VerifiedLibrarySet;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{fs::File, path::Path, sync::Arc};
use tower::ServiceExt;

fn accepted() -> Arc<CanonicalSysmlSystemsLibrary> {
    let kerml = File::open(
        std::env::var_os("AGENTIQUE_KERML_CACHE").expect("accepted KerML cache required"),
    )
    .unwrap();
    let systems = File::open(
        std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").expect("accepted Systems cache required"),
    )
    .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(root).unwrap();
    let kerml = Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml, &sources).unwrap());
    Arc::new(CanonicalSysmlSystemsLibrary::restore_cache(systems, &sources, kerml).unwrap())
}

async fn request(
    app: &Router,
    method: &str,
    uri: &str,
    value: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let body = value
        .map(|value| serde_json::to_vec(&value).unwrap())
        .unwrap_or_default();
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"raw": String::from_utf8_lossy(&bytes)}));
    (status, headers, value)
}

#[tokio::test]
#[ignore = "requires accepted KerML and Systems caches; never rebuilds standards"]
async fn durable_project_revision_http_vertical_and_stable_continuation() {
    let publication = accepted();
    let temporary = tempfile::tempdir().unwrap();
    let repository =
        Arc::new(SqliteRepository::open(temporary.path().join("platform.sqlite")).unwrap());
    let service = Arc::new(ModelingService::new(repository, publication, 8));
    let app = agq_modeling_http::router(Arc::new(ModelingApi::new(service.clone())), 2);
    let (status, _, project) = request(
        &app,
        "POST",
        "/api/gen2/projects",
        Some(json!({"name":"HTTP fixture","description":"Normative metadata"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{project}");
    let project_id: ProjectId = project["@id"].as_str().unwrap().parse().unwrap();
    let main = project["defaultBranch"]["@id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let initial = service
        .repository()
        .get_branch(project_id, main)
        .unwrap()
        .head;
    let first = service
        .apply_document_changes(ApplyDocumentChanges {
            project: project_id,
            branch: main,
            expected_head: initial,
            operation_id: OperationId::new(),
            validate: true,
            changes: vec![ProjectChange::Add {
                path: "API.sysml".into(),
                language: SourceLanguage::SysMl,
                source: "package ApiFixture { part def System; part system : System; }".into(),
            }],
        })
        .unwrap()
        .revision_id;
    let prefix = format!("/api/gen2/projects/{project_id}");
    let (status, _, branches) = request(
        &app,
        "POST",
        &format!("{prefix}/branches"),
        Some(json!({"name":"architecture-experiment","head":{"@id":first.to_string()}})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{branches}");
    assert_eq!(branches["head"], branches["referencedCommit"]);
    let experiment = branches["@id"].as_str().unwrap().parse().unwrap();
    let elements = format!("{prefix}/commits/{first}/elements");
    let (status, headers, first_page) =
        request(&app, "GET", &format!("{elements}?page%5Bsize%5D=1"), None).await;
    assert_eq!(status, StatusCode::OK, "{first_page}");
    assert_eq!(first_page.as_array().unwrap().len(), 1);
    assert_eq!(headers["x-agentique-revision"], first.to_string());
    let next = headers["link"]
        .to_str()
        .unwrap()
        .split('>')
        .next()
        .unwrap()
        .trim_start_matches('<')
        .to_owned();
    let second = service
        .apply_document_changes(ApplyDocumentChanges {
            project: project_id,
            branch: experiment,
            expected_head: first,
            operation_id: OperationId::new(),
            validate: true,
            changes: vec![ProjectChange::Add {
                path: "More.sysml".into(),
                language: SourceLanguage::SysMl,
                source: "package Experiment { part def Added; }".into(),
            }],
        })
        .unwrap()
        .revision_id;
    assert_eq!(
        service
            .repository()
            .get_branch(project_id, main)
            .unwrap()
            .head,
        first
    );
    let (status, headers, next_page) = request(&app, "GET", &next, None).await;
    assert_eq!(status, StatusCode::OK, "{next_page}");
    assert_eq!(headers["x-agentique-revision"], first.to_string());
    assert_ne!(first_page[0]["@id"], next_page[0]["@id"]);
    let foreign_cursor = next.replace(&first.to_string(), &second.to_string());
    assert_eq!(
        request(&app, "GET", &foreign_cursor, None).await.0,
        StatusCode::BAD_REQUEST
    );
    let (status, _, roots) = request(
        &app,
        "GET",
        &format!("{prefix}/commits/{first}/roots"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{roots}");
    assert!(
        roots
            .as_array()
            .unwrap()
            .iter()
            .any(|root| root["declaredName"] == "ApiFixture")
    );
    let root_id = roots[0]["@id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("{prefix}/commits/{first}/elements/{root_id}"),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("{prefix}/commits/{first}/elements/{root_id}/relationships"),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, _, diff) = request(
        &app,
        "GET",
        &format!("{prefix}/commits/{second}/diff?baseCommitId={first}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{diff}");
    assert!(
        diff.as_array()
            .unwrap()
            .iter()
            .any(|value| value["baseData"].is_null()
                && value["compareData"]["payload"]["declaredName"] == "Experiment")
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{prefix}/commits"),
            Some(json!({"change":[]}))
        )
        .await
        .0,
        StatusCode::NOT_IMPLEMENTED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{prefix}/branches/{experiment}/merge"),
            None
        )
        .await
        .0,
        StatusCode::NOT_IMPLEMENTED
    );
}
