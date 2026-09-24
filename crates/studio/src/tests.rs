use super::*;
use axum::{body::Body, http::Request};
use tower::ServiceExt;

fn unavailable_host() -> Host {
    Host {
        runtime: Arc::new(Mutex::new(Err("accepted inputs missing".into()))),
        jobs: Arc::new(Semaphore::new(2)),
        operator_token: Arc::new("operator-test".into()),
        agent_token: Arc::new("agent-test".into()),
    }
}
async fn request(
    app: &Router,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("/api/gen2/studio{path}"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn host_reports_missing_inputs_and_denies_machine_commit() {
    let app = routes(unavailable_host(), PathBuf::from("console/dist"));
    let (status, body) = request(&app, "GET", "/session", None, Value::Null).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(
        body["description"]
            .as_str()
            .unwrap()
            .contains("accepted inputs missing")
    );
    let (status, _) = request(
        &app,
        "POST",
        "/candidates/unknown/commit",
        Some("agent-test"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = request(
        &app,
        "POST",
        "/candidates/unknown/validate",
        Some("agent-test"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = request(
        &app,
        "POST",
        "/candidates/unknown/commit",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/gen2/studio/session")
                .header("sec-fetch-site", "cross-site")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[ignore = "requires accepted KerML and Systems cache files; never rebuilds publication"]
async fn durable_studio_candidate_view_and_history_vertical() {
    let directory = tempfile::tempdir().unwrap();
    let config = StudioConfig {
        root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        database: directory.path().join("studio.sqlite"),
        kerml_cache: std::env::var_os("AGENTIQUE_KERML_CACHE").map(PathBuf::from),
        systems_cache: std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").map(PathBuf::from),
    };
    let runtime = Arc::new(initialize(&config).unwrap());
    let host = Host {
        runtime: Arc::new(Mutex::new(Ok(runtime.clone()))),
        ..unavailable_host()
    };
    let app = routes(host, config.root.join("console/dist"));
    let (status, session) =
        request(&app, "GET", "/session", Some("operator-test"), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let project = &session["project"]["id"];
    let revision = &session["default_revision"];
    let branch = &session["project"]["default_branch"];
    let mut definition = ViewDefinition::semantic_graph();
    definition.include_standard_library = true;
    let (status, graph) = request(
        &app,
        "POST",
        "/view",
        Some("operator-test"),
        json!({"project":project,"revision":revision,"definition":definition}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{graph}");
    assert_eq!(&graph["revision_id"], revision);
    let owner = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "ModelingPlatform" && n["semantic_kind"] == "PartDefinition")
        .unwrap()["id"]
        .clone();
    let (_, inspector) = request(
        &app,
        "POST",
        "/inspect",
        Some("operator-test"),
        json!({"project":project,"revision":revision,"element":owner}),
    )
    .await;
    assert_eq!(&inspector["revision_id"], revision);
    let derived = graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["origin"] == "Derived" && e["relationship_id"].is_string())
        .expect("the real self-model must expose a canonical derived relationship");
    {
        let (status, why) = request(
            &app,
            "POST",
            "/explain",
            Some("operator-test"),
            json!({"project":project,"revision":revision,"element":derived["relationship_id"]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{why}");
        assert!(!why["nodes"].as_array().unwrap().is_empty());
        assert!(why["rule_id"].is_string());
        assert_eq!(&why["revision_id"], revision);
    }
    let (status,agent) = request(&app,"POST","/agent",Some("agent-test"),json!({"project":project,"revision":revision,"selection":[owner],"intent":"Show its dependencies"})).await;
    assert_eq!(status, StatusCode::OK, "{agent}");
    assert_eq!(&agent["view"]["revision_id"], revision);
    let (status,candidate) = request(&app,"POST","/candidates",Some("agent-test"),json!({"project":project,"branch":branch,"revision":revision,"command":{"kind":"CreatePartUsage","owner":owner,"name":"studioObserver","definition":null}})).await;
    assert_eq!(status, StatusCode::OK, "{candidate}");
    assert_eq!(candidate["validation"], "Working");
    let id = candidate["id"].as_str().unwrap();
    let (_, still) = request(&app, "GET", "/session", Some("operator-test"), Value::Null).await;
    assert_eq!(
        &still["default_revision"], revision,
        "proposing must not move the branch"
    );
    let (status, validated) = request(
        &app,
        "POST",
        &format!("/candidates/{id}/validate"),
        Some("operator-test"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{validated}");
    assert_eq!(validated["validation"], "Validated");
    assert_eq!(
        candidate["revision_id"], validated["revision_id"],
        "validation cannot reconstruct another candidate"
    );
    let (status, receipt) = request(
        &app,
        "POST",
        &format!("/candidates/{id}/commit"),
        Some("operator-test"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{receipt}");
    let (status, again) = request(
        &app,
        "POST",
        &format!("/candidates/{id}/validate"),
        Some("operator-test"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["revision_id"], receipt["revision_id"]);
    let (_, retry) = request(
        &app,
        "POST",
        &format!("/candidates/{id}/commit"),
        Some("operator-test"),
        Value::Null,
    )
    .await;
    assert_eq!(retry, receipt, "exact retained operation is idempotent");
    let (status, _) = request(
        &app,
        "DELETE",
        &format!("/candidates/{id}"),
        Some("operator-test"),
        Value::Null,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "reject cannot undo a committed candidate"
    );
    let (_, updated) = request(&app, "GET", "/session", Some("operator-test"), Value::Null).await;
    assert_eq!(updated["default_revision"], receipt["revision_id"]);
    let (_, diff) = request(
        &app,
        "POST",
        "/diff",
        Some("operator-test"),
        json!({"project":project,"from":revision,"to":receipt["revision_id"]}),
    )
    .await;
    assert!(
        !diff["diff"]["declared"]["added"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        runtime
            .service
            .repository()
            .check_integrity()
            .unwrap()
            .is_ok()
    );
    // View metadata persists independently of the model.
    let (_, saved) = request(
        &app,
        "POST",
        "/views",
        Some("operator-test"),
        json!({"project":project,"definition":ViewDefinition::architecture()}),
    )
    .await;
    assert_eq!(saved["version"], 1);
    let metadata =
        rusqlite::Connection::open(config.database.with_extension("views.sqlite")).unwrap();
    assert_eq!(
        metadata
            .query_row("SELECT count(*) FROM studio_views", [], |r| r
                .get::<_, usize>(0))
            .unwrap(),
        1
    );
}
