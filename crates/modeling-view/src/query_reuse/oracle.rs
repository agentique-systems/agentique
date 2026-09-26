//! Exact same-revision oracle against the preserved d404a87 implementations.
//! Ignored runtime work is serial and opens only ordinary authenticated assets.
use crate::{
    inspector::reference as old_inspector,
    projection::{connector_edges, has_declared_name, is, reference as old_projection},
    *,
};
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries, QualifiedName, QueryResult};
use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries, sysml::CanonicalSysmlSystemsLibrary,
};
use agq_kernel::value::Value;
use agq_modeling_service::{ModelingService, RevisionSelector};
use agq_modeling_workspace::ProjectRevision;
use agq_sysml::classes as sc;
use std::{
    collections::BTreeSet,
    fmt::Debug,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

fn assert_result<T: Debug + PartialEq + serde::Serialize>(
    baseline: Result<T, ViewError>,
    current: Result<T, ViewError>,
) {
    match (baseline, current) {
        (Ok(a), Ok(b)) => {
            assert_eq!(a, b, "full DTO equality");
            assert_eq!(
                serde_json::to_vec(&a).unwrap(),
                serde_json::to_vec(&b).unwrap()
            );
        }
        (Err(a), Err(b)) => {
            assert_eq!(
                format!("{a:?}"),
                format!("{b:?}"),
                "exact error variant/payload"
            );
            assert_eq!(a.to_string(), b.to_string());
        }
        (a, b) => panic!("Result outcome changed: {a:?} versus {b:?}"),
    }
}

fn assert_query<T: Debug + PartialEq>(a: QueryResult<T>, b: QueryResult<T>) {
    assert_eq!(
        a.context, b.context,
        "same revision requires no normalization"
    );
    assert_eq!(a.value, b.value);
    assert_eq!(a.completeness, b.completeness);
    assert_eq!(a.diagnostics, b.diagnostics);
    assert_eq!(a.positive_dependencies, b.positive_dependencies);
    assert_eq!(a.search_dependencies, b.search_dependencies);
    assert_eq!(a.explanations, b.explanations);
    assert_eq!(a.fact_origins, b.fact_origins);
    assert_eq!(a.declared_fact_origins, b.declared_fact_origins);
    assert_eq!(a.canonical_dependencies, b.canonical_dependencies);
    // Includes private producer/shared-search state as well as public evidence.
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
}

fn local(revision: &ProjectRevision) -> BTreeSet<ElementId> {
    let standard = revision.accepted_sysml().overlay().model();
    revision
        .semantic_model()
        .unwrap()
        .elements()
        .filter(|record| standard.element(record.id()).is_none())
        .map(|record| record.id())
        .collect()
}

fn allowed(revision: &ProjectRevision, local: &BTreeSet<ElementId>) -> BTreeSet<ElementId> {
    let model = revision.semantic_model().unwrap();
    local
        .iter()
        .copied()
        .filter(|id| {
            *id != revision.root()
                && (!is(model, *id, c::RELATIONSHIP) || is(model, *id, c::FEATURE))
                && has_declared_name(model, *id)
        })
        .collect()
}

fn raw_projection_parity(revision: &ProjectRevision, focus: ElementId) {
    let model = revision.semantic_model().unwrap();
    let local = local(revision);
    let allowed = allowed(revision, &local);
    let baseline_connector = revision.kerml_queries().unwrap();
    let shared = revision.kerml_queries().unwrap();
    assert_eq!(baseline_connector.context(), shared.context());
    assert!(std::ptr::eq(baseline_connector.model(), shared.model()));
    for connector in local
        .iter()
        .copied()
        .filter(|id| is(model, *id, c::CONNECTOR))
    {
        assert_query(
            baseline_connector.connector_endpoints(connector),
            shared.connector_endpoints(connector),
        );
    }
    drop(baseline_connector);
    let fresh_focus = revision.kerml_queries().unwrap();
    assert_eq!(fresh_focus.context(), shared.context());
    let effective = fresh_focus.effective_features(focus);
    let features = effective.value.clone();
    assert_query(effective, shared.effective_features(focus));
    for feature in features.into_iter().filter(|id| {
        allowed.contains(id)
            && (is(model, *id, sc::PORT_USAGE) || is(model, *id, sc::INTERFACE_USAGE))
    }) {
        assert_query(fresh_focus.owner(feature), shared.owner(feature));
        assert_query(
            fresh_focus.feature_types(feature),
            shared.feature_types(feature),
        );
    }
}

fn raw_inspector_parity(revision: &ProjectRevision, subject: ElementId) {
    let baseline = revision.kerml_queries().unwrap();
    let sysml = revision.sysml_queries().unwrap();
    let shared = sysml.kerml();
    assert_eq!(baseline.context(), shared.context());
    assert_eq!(baseline.context(), &sysml.context().kerml);
    assert!(std::ptr::eq(baseline.model(), shared.model()));
    assert_query(baseline.owner(subject), shared.owner(subject));
    let model = baseline.model();
    if is(model, subject, c::FEATURE) {
        assert_query(
            baseline.feature_types(subject),
            shared.feature_types(subject),
        );
        assert_query(
            baseline.subsetted_features(subject),
            shared.subsetted_features(subject),
        );
        assert_query(
            baseline.redefined_features(subject),
            shared.redefined_features(subject),
        );
    }
    if is(model, subject, c::TYPE) {
        assert_query(
            baseline.direct_features(subject),
            shared.direct_features(subject),
        );
        assert_query(
            baseline.effective_features(subject),
            shared.effective_features(subject),
        );
        assert_query(baseline.supertypes(subject), shared.supertypes(subject));
    }
    for connector in local(revision)
        .into_iter()
        .filter(|id| is(model, *id, c::CONNECTOR))
    {
        assert_query(
            baseline.connector_endpoints(connector),
            shared.connector_endpoints(connector),
        );
    }
    // The baseline's late profile lookup also carries the exact same full
    // SysML dependency context, not merely the same human profile label.
    let baseline_profile = revision.sysml_queries().unwrap();
    assert_eq!(baseline_profile.context(), sysml.context());
}

fn named(queries: &KerMlQueries<'_>, revision: &ProjectRevision, path: &[&str]) -> ElementId {
    let answer = queries.lookup_path(
        revision.root(),
        &QualifiedName {
            absolute: false,
            segments: path.iter().map(|segment| (*segment).into()).collect(),
        },
    );
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{path:?}: {answer:?}"
    );
    let ids: BTreeSet<_> = answer.value.iter().map(|item| item.element).collect();
    assert_eq!(ids.len(), 1, "{path:?}: {answer:?}");
    *ids.first().unwrap()
}

fn timed<T>(work: impl FnOnce() -> T) -> (f64, T) {
    let started = Instant::now();
    let value = work();
    (started.elapsed().as_secs_f64() * 1000.0, value)
}

fn paired_times<T: Debug + PartialEq + serde::Serialize>(
    label: &str,
    mut baseline: impl FnMut() -> Result<T, ViewError>,
    mut current: impl FnMut() -> Result<T, ViewError>,
) {
    // Correctness work precedes these three paired observations. No p95 claim.
    for round in 0..3 {
        let ((old_ms, a), (new_ms, b)) = if round % 2 == 0 {
            (timed(&mut baseline), timed(&mut current))
        } else {
            let current = timed(&mut current);
            let baseline = timed(&mut baseline);
            (baseline, current)
        };
        assert_result(a, b); // Comparison and serialization are outside timing.
        eprintln!(
            "{}",
            serde_json::json!({
                "format": "agentique-view-query-reuse-pair/1", "case": label, "round": round,
                "order": if round % 2 == 0 { "baseline,current" } else { "current,baseline" },
                "baseline_ms": old_ms, "current_ms": new_ms,
                "contract": "Same restored revision, fresh per-call evaluator caches, profiling disabled; wall time, not disk-cold or UI latency. Three pairs after correctness warm-up."
            })
        );
    }
}

fn context_and_cache_times(revision: &ProjectRevision, focus: ElementId) {
    let local = local(revision);
    let allowed = allowed(revision, &local);
    for (round, order) in [
        ["fresh", "fork", "shared"],
        ["shared", "fork", "fresh"],
        ["fork", "fresh", "shared"],
    ]
    .into_iter()
    .enumerate()
    {
        let mut expected = None;
        for arm in order {
            let primed = revision.kerml_queries().unwrap();
            // Identical actual query priming, outside the focused-stage timing.
            drop(connector_edges(
                &primed,
                revision.revision(),
                &local,
                |_, _| {},
            ));
            let (construction_ms, queries) = match arm {
                "fresh" => {
                    drop(primed);
                    let (time, queries) = timed(|| revision.kerml_queries().unwrap());
                    (Some(time), queries)
                }
                "fork" => {
                    let (time, queries) = timed(|| primed.fork());
                    drop(primed);
                    (Some(time), queries)
                }
                _ => (None, primed),
            };
            let (query_ms, output) =
                timed(|| old_projection::focused_probe(&queries, focus, &allowed));
            if let Some(expected) = &expected {
                assert_eq!(&output, expected);
            } else {
                expected = Some(output);
            }
            eprintln!(
                "{}",
                serde_json::json!({
                    "format": "agentique-view-context-cache-arms/1", "round": round, "arm": arm,
                    "context_or_fork_ms": construction_ms, "focused_queries_ms": query_ms,
                    "contract": "Identical connector priming excluded; context/fork and focused queries independently timed. null means no constructor. Destruction/output comparison excluded; use paired call totals for overall latency."
                })
            );
        }
    }
}

#[test]
fn shared_evaluator_preserves_incomplete_and_invalid_canonical_query_evidence() {
    use crate::projection::tests::semantic_fixture;
    use agq_kerml_semantics::{SemanticContext, SemanticOptions};
    let snapshot = semantic_fixture(
        &[(1, sc::PART_DEFINITION), (2, sc::PORT_USAGE)],
        &[(
            1,
            p::ELEMENT_DECLARED_NAME,
            vec![Value::String("Unclosed canonical type".into())],
        )],
    );
    let queries = || {
        KerMlQueries::new(
            SemanticContext::for_working_snapshot(
                &snapshot,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                    ..Default::default()
                },
                BTreeSet::new(),
                BTreeSet::from([ElementId::from_u128(1)]),
            )
            .unwrap(),
        )
    };
    let baseline = queries();
    let shared = queries();
    let invalid = ElementId::from_u128(999);
    let a = baseline.effective_features(invalid);
    assert_eq!(a.completeness, Completeness::Invalid);
    assert_query(a, shared.effective_features(invalid));
    // Warm unrelated caches before comparing a fresh batch with the shared one.
    drop(shared.owner(ElementId::from_u128(2)));
    let fresh = baseline.fork();
    let a = fresh.effective_features(ElementId::from_u128(1));
    assert_ne!(
        a.completeness,
        Completeness::Complete,
        "fixture has an explicit pending specialization scope"
    );
    assert!(
        a.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "KQ_PENDING_INHERITANCE")
    );
    assert_query(a, shared.effective_features(ElementId::from_u128(1)));
}

#[test]
#[ignore = "requires a closed SQLite backup snapshot and ordinary accepted caches; never publishes standards"]
fn accepted_same_revision_call_local_query_reuse_matches_full_results() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let database = PathBuf::from(
        std::env::var_os("AGENTIQUE_VIEW_ORACLE_DATABASE")
            .expect("closed SQLite backup snapshot required"),
    );
    assert!(database.is_absolute() && database.is_file());
    // Supply a closed SQLite backup snapshot, not a live native repository.
    // Never silently omit a nonempty WAL or rollback journal from this oracle.
    for suffix in ["-wal", "-journal"] {
        let mut sidecar = database.as_os_str().to_owned();
        sidecar.push(suffix);
        match std::fs::metadata(PathBuf::from(sidecar)) {
            Ok(metadata) => assert_eq!(
                metadata.len(),
                0,
                "nonempty {suffix}: supply a closed SQLite backup snapshot"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot establish snapshot sidecar state for {suffix}: {error}"),
        }
    }
    let project = serde_json::from_value(serde_json::Value::String(
        std::env::var("AGENTIQUE_VIEW_ORACLE_PROJECT").expect("exact project ID required"),
    ))
    .unwrap();
    let revision_id = serde_json::from_value(serde_json::Value::String(
        std::env::var("AGENTIQUE_VIEW_ORACLE_REVISION").expect("exact revision ID required"),
    ))
    .unwrap();
    let kerml_file = File::open(
        std::env::var_os("AGENTIQUE_KERML_CACHE").expect("accepted KerML cache required"),
    )
    .unwrap();
    let systems_file = File::open(
        std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").expect("accepted Systems cache required"),
    )
    .unwrap();
    let sources = agq_standard_libraries::VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let (runtime_restore_ms, accepted) = timed(|| {
        let kerml =
            Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml_file, &sources).unwrap());
        Arc::new(
            CanonicalSysmlSystemsLibrary::restore_cache(systems_file, &sources, kerml).unwrap(),
        )
    });
    let scratch = tempfile::tempdir().unwrap();
    let copied = scratch.path().join("view-oracle.sqlite");
    std::fs::copy(&database, &copied).unwrap();
    let service = ModelingService::new(
        Arc::new(agq_modeling_sqlite::SqliteRepository::open(copied).unwrap()),
        accepted,
        1,
    );
    let (revision_restore_ms, bound) = timed(|| {
        service
            .resolve(project, RevisionSelector::Revision(revision_id))
            .unwrap()
    });
    assert!(
        bound.validated().is_some(),
        "baseline must restore as Validated"
    );
    let revision = bound.revision();
    assert_eq!(revision.revision(), revision_id);
    let before = revision.semantic_fingerprint().unwrap();
    eprintln!(
        "{}",
        serde_json::json!({
            "format": "agentique-view-query-reuse-oracle/1", "project": project, "revision": revision_id,
            "runtime_restore_ms": runtime_restore_ms, "revision_restore_ms": revision_restore_ms,
            "load_path": format!("{:?}", bound.load_path()), "fingerprint": before,
            "baseline_commit": "d404a8759891acc5d621dcad655fa4a981e17d63",
            "contract": "Same restored immutable revision; all counterpart calls serial; no identity normalization."
        })
    );
    let names = revision.kerml_queries().unwrap();
    let system = named(&names, revision, &["AgentiqueSystem", "Agentique"]);
    let platform = named(
        &names,
        revision,
        &["PlatformArchitecture", "ModelingPlatform"],
    );
    let repository = named(
        &names,
        revision,
        &["PlatformArchitecture", "ModelRepository"],
    );
    let port = named(
        &names,
        revision,
        &["PlatformArchitecture", "Repository", "repositoryRevisions"],
    );
    let client = named(
        &names,
        revision,
        &["PlatformArchitecture", "ModelingPlatform", "clientQueries"],
    );
    let connector = named(
        &names,
        revision,
        &[
            "PlatformArchitecture",
            "ModelingPlatform",
            "queryConnection",
        ],
    );
    let requirement = named(
        &names,
        revision,
        &["PlatformArchitecture", "ImmutableRevisions"],
    );
    drop(names);

    let mut views = vec![ViewDefinition {
        depth: 1,
        ..ViewDefinition::architecture()
    }];
    for focus in [system, platform, repository] {
        views.push(ViewDefinition {
            focus: Some(focus),
            depth: 1,
            ..ViewDefinition::architecture()
        });
    }
    let mut no_connections = views[2].clone();
    no_connections
        .relationship_families
        .retain(|family| *family != RelationshipFamily::Connection);
    views.push(no_connections);
    let mut standard = views[2].clone();
    standard.include_standard_library = true;
    views.push(standard);
    let mut hidden = views[2].clone();
    hidden.hidden_elements.push(client);
    views.push(hidden);
    for graph_scope in [GraphScope::Neighborhood, GraphScope::DependencyNeighborhood] {
        for depth in [0, 1, 2] {
            views.push(ViewDefinition {
                focus: Some(repository),
                depth,
                graph_scope,
                ..ViewDefinition::semantic_graph()
            });
        }
    }
    let mut graph_without_connections = ViewDefinition::semantic_graph();
    graph_without_connections
        .relationship_families
        .retain(|family| *family != RelationshipFamily::Connection);
    views.push(graph_without_connections);
    views.push(ViewDefinition::requirements());
    views.push(ViewDefinition {
        version: 99,
        ..ViewDefinition::architecture()
    });
    let missing = ElementId::from_u128(u128::MAX);
    assert!(revision.element(missing).is_none());
    views.push(ViewDefinition {
        focus: Some(missing),
        ..ViewDefinition::architecture()
    });
    for view in &views {
        assert_result(
            old_projection::project(revision, view),
            old_projection::current(revision, view),
        );
    }
    for subject in [
        system,
        platform,
        repository,
        port,
        client,
        connector,
        requirement,
        missing,
    ] {
        assert_result(
            old_inspector::inspect(revision, subject),
            old_inspector::current(revision, subject),
        );
        if subject != missing {
            raw_inspector_parity(revision, subject);
        }
    }
    for focus in [platform, repository] {
        raw_projection_parity(revision, focus);
    }
    let focus_view = ViewDefinition {
        focus: Some(platform),
        depth: 1,
        ..ViewDefinition::architecture()
    };
    paired_times(
        "focused ModelingPlatform projection",
        || old_projection::project(revision, &focus_view),
        || old_projection::current(revision, &focus_view),
    );
    paired_times(
        "ModelingPlatform Inspector",
        || old_inspector::inspect(revision, platform),
        || old_inspector::current(revision, platform),
    );
    context_and_cache_times(revision, platform);
    assert_eq!(
        revision.semantic_fingerprint().unwrap(),
        before,
        "read-only oracle changed canonical identity"
    );
    eprintln!(
        "view query reuse oracle passed: {} exact projection cases, 8 Inspector cases, full raw-query evidence, 3 paired timing rounds and 3 A/B/C rounds",
        views.len()
    );
}
