//! Accepted-cache platform gates. No standard publication acquisition or rebuild.
#[path = "support/repository_probe.rs"]
mod repository_probe;
use agq_kerml_semantics::{Completeness, QualifiedName};
use agq_kerml_text::{
    ProjectChange, SourceLanguage, library::CanonicalKermlStandardLibraries,
    sysml::CanonicalSysmlSystemsLibrary,
};
use agq_kernel::{ElementId, provenance::ByteRange};
use agq_modeling_repository::*;
use agq_modeling_service::*;
use agq_modeling_sqlite::SqliteRepository;
use agq_modeling_workspace::{ProjectRevisionCheckpoint, SemanticFingerprint};
use agq_standard_libraries::VerifiedLibrarySet;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};

fn accepted_catalogue() -> &'static Mutex<Option<Arc<CanonicalSysmlSystemsLibrary>>> {
    static VALUE: OnceLock<Mutex<Option<Arc<CanonicalSysmlSystemsLibrary>>>> = OnceLock::new();
    VALUE.get_or_init(|| Mutex::new(None))
}
fn accepted() -> Arc<CanonicalSysmlSystemsLibrary> {
    accepted_catalogue()
        .lock()
        .unwrap()
        .get_or_insert_with(|| {
            let kerml = File::open(
                std::env::var_os("AGENTIQUE_KERML_CACHE").expect("accepted KerML cache required"),
            )
            .unwrap();
            let systems = File::open(
                std::env::var_os("AGENTIQUE_SYSTEMS_CACHE")
                    .expect("accepted Systems cache required"),
            )
            .unwrap();
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
            let kerml =
                Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml, &sources).unwrap());
            Arc::new(CanonicalSysmlSystemsLibrary::restore_cache(systems, &sources, kerml).unwrap())
        })
        .clone()
}
fn open(path: &Path, capacity: usize) -> ModelingService {
    let start = Instant::now();
    let repository = Arc::new(SqliteRepository::open(path).unwrap());
    eprintln!("cold_repository_open_ms={}", start.elapsed().as_millis());
    ModelingService::new(repository, accepted(), capacity)
}
fn report_restore(bound: &BoundRevision, started: Instant) {
    let class = match bound.load_path() {
        RevisionLoadPath::DurableSource => "cold_revision_restore_from_source",
        RevisionLoadPath::AuthenticatedSemanticCache => "restore_from_authenticated_semantic_cache",
        RevisionLoadPath::ImmutableMemory => "restore_from_immutable_memory",
    };
    eprintln!("{class}_ms={}", started.elapsed().as_millis());
}
fn add(path: &str, language: SourceLanguage, source: &str) -> ProjectChange {
    ProjectChange::Add {
        path: path.into(),
        language,
        source: source.into(),
    }
}
fn apply(
    service: &ModelingService,
    project: &Project,
    branch: BranchId,
    expected: ProjectRevisionId,
    changes: Vec<ProjectChange>,
    validate: bool,
) -> CommitReceipt {
    service
        .apply_document_changes(ApplyDocumentChanges {
            operation_id: OperationId::new(),
            project: project.id,
            branch,
            expected_head: expected,
            changes,
            validate,
        })
        .unwrap()
}
fn named(bound: &BoundRevision, names: &[&str]) -> ElementId {
    let q = bound.revision().kerml_queries().unwrap();
    let answer = q.lookup_path(
        bound.revision().root(),
        &QualifiedName {
            absolute: false,
            segments: names.iter().map(|s| s.to_string()).collect(),
        },
    );
    assert_eq!(answer.completeness, Completeness::Complete);
    let ids: std::collections::BTreeSet<_> =
        answer.value.iter().map(|member| member.element).collect();
    assert_eq!(ids.len(), 1, "{names:?}: {answer:?}");
    *ids.first().unwrap()
}
fn replace(bound: &BoundRevision, path: &str, old: &str, new: &str) -> ProjectChange {
    let doc = bound.revision().document_at(path).unwrap();
    let start = doc.source().find(old).unwrap();
    ProjectChange::Edit {
        document: doc.id(),
        edit: agq_kerml_text::syntax::TextEdit {
            range: ByteRange::new(start as u64, (start + old.len()) as u64).unwrap(),
            replacement: new.into(),
        },
    }
}

#[test]
#[ignore = "accepted-cache service fault and persisted semantic cache authentication gate"]
fn durable_cache_and_failure_authentication() {
    use repository_probe::{BlobOverride, RepositoryProbe};
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteRepository::open(directory.path().join("cache.db")).unwrap());
    let probe = Arc::new(RepositoryProbe::new(store.clone()));
    // Disable immutable-memory reuse so every observation exercises durable reads.
    let service = ModelingService::new(probe.clone(), accepted(), 0);
    let prepared_project = service
        .prepare_project("authenticated cache", None)
        .unwrap();
    probe.lose_next_create_acknowledgement();
    assert!(matches!(
        service.commit_project(&prepared_project),
        Err(ServiceError::Repository(RepositoryError::OutcomeUnknown(_)))
    ));
    let project = service.commit_project(&prepared_project).unwrap();
    assert_eq!(project.id, prepared_project.project().id);
    assert_eq!(store.list_projects().unwrap().len(), 1);
    let initial = prepared_project.revision().revision();
    let prepared = service
        .prepare_changes(ApplyDocumentChanges {
            operation_id: OperationId::new(),
            project: project.id,
            branch: project.default_branch,
            expected_head: initial,
            changes: vec![add(
                "cache.sysml",
                SourceLanguage::SysMl,
                "package Cache { part def Repository; part repository : Repository; }",
            )],
            validate: true,
        })
        .unwrap();
    probe.fail_next_commit_before_durability();
    assert!(matches!(
        service.commit_prepared(&prepared),
        Err(ServiceError::Repository(RepositoryError::Storage(_)))
    ));
    assert_eq!(
        store
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        initial
    );
    assert_eq!(store.list_revisions(project.id).unwrap().len(), 1);
    let started = Instant::now();
    let receipt = service.commit_prepared(&prepared).unwrap();
    eprintln!(
        "durable_revision_commit_ms={}",
        started.elapsed().as_millis()
    );
    let manifest = prepared.request().candidate.manifest.clone();
    let cache = manifest
        .semantic_cache
        .clone()
        .expect("validated candidate persisted its cache");
    eprintln!(
        "semantic_cache_blob_bytes={}",
        prepared.request().candidate.blobs[&cache.content_digest].len()
    );
    let checkpoint = prepared.revision().checkpoint();
    let fingerprint = prepared.revision().semantic_fingerprint().unwrap();
    let started = Instant::now();
    let cached = service
        .resolve(project.id, RevisionSelector::Revision(receipt.revision_id))
        .unwrap();
    eprintln!(
        "restore_from_authenticated_semantic_cache_ms={}",
        started.elapsed().as_millis()
    );
    assert_eq!(
        cached.load_path(),
        RevisionLoadPath::AuthenticatedSemanticCache
    );
    eprintln!(
        "authenticated_cache_restore_work={}",
        serde_json::to_string(cached.revision().compilation_work()).unwrap()
    );
    assert_eq!(cached.revision().checkpoint(), checkpoint);
    assert_eq!(
        cached.revision().semantic_fingerprint().unwrap(),
        fingerprint
    );
    let observation = observe_named(&cached, &["Cache", "repository"]);

    for scenario in [
        "missing",
        "corrupt",
        "stale",
        "unknown_format",
        "no_reference",
    ] {
        probe.clear_blob_overrides();
        probe.clear_manifest_override();
        let mut overridden = manifest.clone();
        match scenario {
            "missing" => probe.override_blob(cache.content_digest, BlobOverride::Missing),
            "corrupt" => probe.override_blob(
                cache.content_digest,
                BlobOverride::Bytes(b"corrupt cache".to_vec()),
            ),
            "stale" => {
                overridden.semantic_cache.as_mut().unwrap().semantic_context =
                    ContentDigest::of(b"stale semantic context")
            }
            "unknown_format" => {
                overridden.semantic_cache.as_mut().unwrap().format = "future-cache/999".into()
            }
            "no_reference" => overridden.semantic_cache = None,
            _ => unreachable!(),
        }
        probe.override_manifest(project.id, receipt.revision_id, overridden);
        let started = Instant::now();
        let restored = service
            .resolve(project.id, RevisionSelector::Revision(receipt.revision_id))
            .unwrap();
        eprintln!(
            "semantic_cache_fallback={scenario} source_restore_ms={}",
            started.elapsed().as_millis()
        );
        eprintln!(
            "source_restore_work={}",
            serde_json::to_string(restored.revision().compilation_work()).unwrap()
        );
        assert_eq!(
            restored.load_path(),
            RevisionLoadPath::DurableSource,
            "{scenario}"
        );
        assert_eq!(restored.revision().checkpoint(), checkpoint);
        assert_eq!(
            restored.revision().semantic_fingerprint().unwrap(),
            fingerprint
        );
        assert_eq!(
            observe_named(&restored, &["Cache", "repository"]),
            observation
        );
        assert!(restored.validated().is_some());
    }
    probe.clear_blob_overrides();
    let mut working = manifest.clone();
    working.validation = ValidationState::Working;
    probe.override_manifest(project.id, receipt.revision_id, working);
    let restored = service
        .resolve(project.id, RevisionSelector::Revision(receipt.revision_id))
        .unwrap();
    assert!(restored.validated().is_none());
    assert!(restored.validate().is_ok());
    assert!(matches!(
        restored.manifest().validation,
        ValidationState::Working
    ));

    let mut forged = manifest.clone();
    let ValidationState::Validated(validation) = &mut forged.validation else {
        unreachable!()
    };
    validation.semantic_digest = ContentDigest::of(b"forged validation receipt");
    probe.override_manifest(project.id, receipt.revision_id, forged);
    assert!(
        service
            .resolve(project.id, RevisionSelector::Revision(receipt.revision_id))
            .is_err()
    );
    probe.clear_manifest_override();
    // A valid cache cannot make missing mandatory source acceptable.
    probe.override_blob(manifest.documents[0].content_digest, BlobOverride::Missing);
    assert!(
        service
            .resolve(project.id, RevisionSelector::Revision(receipt.revision_id))
            .is_err()
    );
    probe.clear_blob_overrides();

    for (projects, revisions) in [(true, false), (false, true)] {
        probe.override_integrity_report(Some(IntegrityReport {
            errors: vec!["original structural finding".into()],
            ..Default::default()
        }));
        probe.fail_enumeration(projects, revisions);
        let report = service.check_integrity().unwrap();
        assert_eq!(report.errors[0], "original structural finding");
        assert!(report.errors.len() > 1);
    }
    probe.fail_enumeration(false, false);
    probe.override_integrity_report(None);
    assert!(store.check_integrity().unwrap().is_ok());
}

/// Test-only observations carry source identities and selected projections, never
/// a graph or standard publication capable of bypassing source reconstruction.
#[derive(Debug, Serialize, Deserialize)]
struct RestartExpectation {
    project: Project,
    manifest: RevisionManifest,
    checkpoint: ProjectRevisionCheckpoint,
    fingerprint: SemanticFingerprint,
    observations: Vec<NamedObservation>,
    root_members: QueryObservation,
    scenario: RestartScenario,
}

#[derive(Debug, Serialize, Deserialize)]
enum RestartScenario {
    SelfModel,
    Scale {
        initial_empty: ProjectRevisionId,
        first_authored: ProjectRevisionId,
        experiment: BranchId,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NamedObservation {
    path: Vec<String>,
    element: ElementDto,
    relationships: Vec<ElementId>,
    effective_names: QueryObservation,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QueryObservation {
    value: String,
    completeness: Completeness,
    evidence_digest: ContentDigest,
}

fn observe_query<T: std::fmt::Debug>(
    query: agq_kerml_semantics::QueryResult<T>,
) -> QueryObservation {
    // This same-binary test witness deliberately excludes the fresh kernel
    // RevisionId. Semantic context and closure digests are compared separately.
    let evidence = format!(
        "{:?}",
        (
            query.diagnostics,
            query.positive_dependencies,
            query.search_dependencies,
            query.explanations,
            query.fact_origins,
            query.declared_fact_origins,
            query.canonical_dependencies,
        )
    );
    QueryObservation {
        value: format!("{:?}", query.value),
        completeness: query.completeness,
        evidence_digest: ContentDigest::of(evidence.as_bytes()),
    }
}

fn observe_named(bound: &BoundRevision, path: &[&str]) -> NamedObservation {
    let element = named(bound, path);
    NamedObservation {
        path: path.iter().map(|segment| (*segment).to_string()).collect(),
        element: bound.current_element(element).unwrap(),
        relationships: bound
            .relationships(element)
            .unwrap()
            .into_iter()
            .map(|record| record.id)
            .collect(),
        effective_names: observe_query(bound.effective_names(element).unwrap().answer),
    }
}

fn expectation(
    project: &Project,
    bound: &BoundRevision,
    paths: &[&[&str]],
    scenario: RestartScenario,
) -> RestartExpectation {
    RestartExpectation {
        project: project.clone(),
        manifest: bound.manifest().clone(),
        checkpoint: bound.revision().checkpoint(),
        fingerprint: bound.revision().semantic_fingerprint().unwrap(),
        observations: paths
            .iter()
            .map(|path| observe_named(bound, path))
            .collect(),
        root_members: observe_query(
            bound
                .effective_members(bound.revision().root())
                .unwrap()
                .answer,
        ),
        scenario,
    }
}

fn restart_in_fresh_process(database: &Path, expected: &RestartExpectation) {
    // Release even the accepted standards before the child starts. This proves
    // complete process-independent restoration without retaining two large
    // publication catalogues in memory during the subprocess gate.
    let publication = accepted_catalogue().lock().unwrap().take().unwrap();
    assert_eq!(Arc::strong_count(&publication), 1);
    drop(publication);
    let expectation_file = database.with_extension("restart-expectation.json");
    std::fs::write(&expectation_file, serde_json::to_vec(expected).unwrap()).unwrap();
    let started = Instant::now();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "internal_cold_repository_restore_child",
            "--ignored",
            "--nocapture",
        ])
        .env("AGENTIQUE_RESTORE_CHILD_DATABASE", database)
        .env("AGENTIQUE_RESTORE_CHILD_EXPECTATION", expectation_file)
        .output()
        .unwrap();
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    eprint!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(
        output.status.success(),
        "fresh-process restore failed: {}",
        output.status
    );
    eprintln!(
        "cold_restart_subprocess_total_ms={}",
        started.elapsed().as_millis()
    );
}

#[test]
#[ignore = "internal subprocess helper; requires explicit database and expectation environment"]
fn internal_cold_repository_restore_child() {
    let database = std::env::var_os("AGENTIQUE_RESTORE_CHILD_DATABASE")
        .expect("internal helper requires a database path");
    let expectation_file = std::env::var_os("AGENTIQUE_RESTORE_CHILD_EXPECTATION")
        .expect("internal helper requires an expectation path");
    let expected: RestartExpectation =
        serde_json::from_slice(&std::fs::read(expectation_file).unwrap()).unwrap();
    let started = Instant::now();
    let publication = accepted();
    eprintln!(
        "cold_accepted_publication_restore_ms={}",
        started.elapsed().as_millis()
    );
    let started = Instant::now();
    let repository = Arc::new(SqliteRepository::open(&database).unwrap());
    eprintln!("cold_repository_open_ms={}", started.elapsed().as_millis());
    let service = ModelingService::new(repository, publication, 2);
    let started = Instant::now();
    let restored = service
        .resolve(
            expected.project.id,
            RevisionSelector::Revision(expected.manifest.revision_id),
        )
        .unwrap();
    report_restore(&restored, started);
    assert_eq!(restored.manifest(), &expected.manifest);
    assert_eq!(restored.revision().checkpoint(), expected.checkpoint);
    assert_eq!(
        restored.revision().semantic_fingerprint().unwrap(),
        expected.fingerprint
    );
    let persisted_validated = matches!(expected.manifest.validation, ValidationState::Validated(_));
    assert_eq!(restored.validated().is_some(), persisted_validated);
    for observation in &expected.observations {
        let path: Vec<_> = observation.path.iter().map(String::as_str).collect();
        assert_eq!(&observe_named(&restored, &path), observation);
    }
    assert_eq!(
        observe_query(
            restored
                .effective_members(restored.revision().root())
                .unwrap()
                .answer
        ),
        expected.root_members
    );
    let started = Instant::now();
    let checked = restored.validate().unwrap();
    eprintln!("standalone_validation_ms={}", started.elapsed().as_millis());
    assert_eq!(checked.revision(), expected.manifest.revision_id);
    assert_eq!(restored.validated().is_some(), persisted_validated);
    assert_eq!(
        service
            .repository()
            .load_revision(expected.project.id, expected.manifest.revision_id)
            .unwrap()
            .validation,
        expected.manifest.validation
    );
    drop(checked);
    match expected.scenario {
        RestartScenario::SelfModel => {
            continue_self_model_after_restart(&service, &expected.project, &restored)
        }
        RestartScenario::Scale {
            initial_empty,
            first_authored,
            experiment,
        } => {
            assert_eq!(restored.revision().documents().count(), 100);
            assert!(
                restored.validated().is_none(),
                "stored Working must remain Working"
            );
            let history = service
                .repository()
                .list_revision_history(expected.project.id, expected.manifest.revision_id)
                .unwrap();
            assert_eq!(history.len(), 11);
            assert_eq!(history.last().unwrap().revision_id, initial_empty);
            assert!(history.last().unwrap().documents.is_empty());
            assert!(
                history[..10]
                    .iter()
                    .all(|manifest| manifest.documents.len() == 100)
            );
            assert_eq!(
                service
                    .repository()
                    .get_branch(expected.project.id, expected.project.default_branch)
                    .unwrap()
                    .head,
                first_authored
            );
            assert_eq!(
                service
                    .repository()
                    .get_branch(expected.project.id, experiment)
                    .unwrap()
                    .head,
                expected.manifest.revision_id
            );
            let started = Instant::now();
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    let revision = &restored;
                    scope.spawn(move || {
                        for _ in 0..2 {
                            named(revision, &["S1", "P1"]);
                            assert_eq!(revision.revision().documents().count(), 100);
                        }
                    });
                }
            });
            eprintln!(
                "parallel_immutable_reads_ms={}",
                started.elapsed().as_millis()
            );
            assert!(service.repository().check_integrity().unwrap().is_ok());
            eprintln!(
                "durable_scale_cold_process_restore=true documents=100 authored_revisions=10 initial_empty_revisions=1 readers=4 stored_working_preserved=true"
            );
        }
    }
}

#[test]
#[ignore = "requires accepted caches; focused gate before durable scale"]
fn durable_restore_branch_binding_diff_and_cas() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("project.db");
    let service = open(&database, 4);
    let project = service.create_project("platform", None).unwrap();
    let initial = service
        .repository()
        .get_branch(project.id, project.default_branch)
        .unwrap()
        .head;
    let first = apply(
        &service,
        &project,
        project.default_branch,
        initial,
        vec![add(
            "platform.sysml",
            SourceLanguage::SysMl,
            "package Platform { part def Repository; part repository : Repository; }",
        )],
        true,
    );
    let bound = service
        .resolve(project.id, RevisionSelector::Branch(project.default_branch))
        .unwrap();
    assert!(bound.validated().is_some());
    let signature = bound.revision().semantic_fingerprint().unwrap();
    let element = named(&bound, &["Platform", "repository"]);
    let source = bound.source_origin(element).unwrap();
    let checkpoint = bound.revision().checkpoint();
    let dto = bound.current_element(element).unwrap();
    assert_eq!(dto.metaclass_name, "PartUsage");
    assert!(!matches!(
        bound
            .elements_at_source(source.document, source.revision, source.range.start())
            .unwrap(),
        SourceMatches::Missing
    ));
    drop(bound);
    drop(service);
    // All project/workspace state has been destroyed. Only authenticated standards
    // survive in the catalogue fixture, as they do in a fresh service process.
    let service = open(&database, 4);
    let started = Instant::now();
    let restored = service
        .resolve(project.id, RevisionSelector::Revision(first.revision_id))
        .unwrap();
    report_restore(&restored, started);
    assert_eq!(restored.revision().checkpoint(), checkpoint);
    assert_eq!(
        restored.revision().semantic_fingerprint().unwrap(),
        signature
    );
    assert_eq!(restored.current_element(element).unwrap(), dto);
    let experiment = service
        .create_branch(project.id, "architecture-experiment", first.revision_id)
        .unwrap();
    let change = replace(
        &restored,
        "platform.sysml",
        "part repository : Repository;",
        "part repository : Repository; part replica : Repository;",
    );
    let a = service
        .prepare_changes(ApplyDocumentChanges {
            operation_id: OperationId::new(),
            project: project.id,
            branch: experiment.id,
            expected_head: first.revision_id,
            changes: vec![change.clone()],
            validate: true,
        })
        .unwrap();
    let b = service
        .prepare_changes(ApplyDocumentChanges {
            operation_id: OperationId::new(),
            project: project.id,
            branch: experiment.id,
            expected_head: first.revision_id,
            changes: vec![change],
            validate: true,
        })
        .unwrap();
    let second = service.commit_prepared(&a).unwrap();
    assert!(
        matches!(service.commit_prepared(&b), Err(ServiceError::Repository(RepositoryError::Conflict { expected, actual })) if expected==first.revision_id && actual==second.revision_id)
    );
    assert!(service.commit_prepared(&a).unwrap().replayed);
    assert_eq!(
        service
            .repository()
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        first.revision_id
    );
    assert_eq!(restored.revision().revision(), first.revision_id);
    assert_eq!(
        restored.revision().semantic_fingerprint().unwrap(),
        signature
    );
    let next = service
        .resolve(project.id, RevisionSelector::Branch(experiment.id))
        .unwrap();
    named(&next, &["Platform", "replica"]);
    let diff = service
        .diff(project.id, first.revision_id, second.revision_id)
        .unwrap();
    assert_eq!(diff.documents.len(), 1);
    assert!(!diff.declared.added.is_empty());
    assert!(!diff.relationships_added.is_empty());
    let start = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let bound = &restored;
            scope.spawn(move || {
                assert_eq!(named(bound, &["Platform", "repository"]), element);
            });
        }
    });
    eprintln!(
        "parallel_immutable_reads_ms={}",
        start.elapsed().as_millis()
    );
    let start = Instant::now();
    let elements = next.current_elements().unwrap();
    eprintln!(
        "element_page_projection_ms={} elements={}",
        start.elapsed().as_millis(),
        elements.len()
    );
    service
        .repository()
        .delete_branch(project.id, experiment.id)
        .unwrap();
    assert_eq!(
        service
            .repository()
            .list_revisions(project.id)
            .unwrap()
            .len(),
        3
    );
    assert!(
        service
            .repository()
            .load_revision(project.id, second.revision_id)
            .is_ok()
    );
    assert!(service.repository().check_integrity().unwrap().is_ok());
}

#[test]
#[ignore = "requires accepted caches; Agentique durable self-model acceptance"]
fn agentique_durable_self_model_two_branches() {
    let directory = tempfile::tempdir().unwrap();
    let database = std::env::var_os("AGENTIQUE_SELF_MODEL_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| directory.path().join("agentique.db"));
    assert!(
        !database.exists(),
        "dogfood output must be a new repository"
    );
    let service = open(&database, 3);
    let project = service
        .create_project("Agentique", Some("Durable architecture dogfooding".into()))
        .unwrap();
    let initial = service
        .repository()
        .get_branch(project.id, project.default_branch)
        .unwrap()
        .head;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../models/agentique");
    let changes = [
        "Contracts.sysml",
        "LanguageEngine.sysml",
        "ModelingPlatform.sysml",
        "ExecutionRuntime.sysml",
        "Agentique.sysml",
    ]
    .into_iter()
    .map(|path| {
        add(
            path,
            SourceLanguage::SysMl,
            &std::fs::read_to_string(root.join(path)).unwrap(),
        )
    })
    .collect();
    let first = apply(
        &service,
        &project,
        project.default_branch,
        initial,
        changes,
        true,
    );
    let bound = service
        .resolve(project.id, RevisionSelector::Revision(first.revision_id))
        .unwrap();
    let expected = expectation(
        &project,
        &bound,
        &[
            &["PlatformArchitecture", "ModelRepository"],
            &["PlatformArchitecture", "ModelingService"],
            &["PlatformArchitecture", "SystemsModelingApiAdapter"],
        ],
        RestartScenario::SelfModel,
    );
    drop(bound);
    drop(service);
    restart_in_fresh_process(&database, &expected);
    let repository = SqliteRepository::open(&database).unwrap();
    eprintln!(
        "dogfood_repository={} project={} main={} revision_1={}",
        database.display(),
        project.id,
        project.default_branch,
        first.revision_id
    );
    assert_eq!(repository.list_branches(project.id).unwrap().len(), 2);
    assert_eq!(repository.list_revisions(project.id).unwrap().len(), 3);
    assert_eq!(
        repository
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        first.revision_id
    );
    assert!(repository.check_integrity().unwrap().is_ok());
}

fn continue_self_model_after_restart(
    service: &ModelingService,
    project: &Project,
    restored: &BoundRevision,
) {
    let first = restored.revision().revision();
    let standard_ids = restored.manifest().accepted_publications.clone();
    let fingerprint = restored.revision().semantic_fingerprint().unwrap();
    let branch = service
        .create_branch(project.id, "architecture-experiment", first)
        .unwrap();
    let second = apply(
        service,
        project,
        branch.id,
        first,
        vec![replace(
            restored,
            "ModelingPlatform.sysml",
            "part modelingService : ModelingService;",
            "part modelingService : ModelingService; part experimentalService : ModelingService;",
        )],
        true,
    );
    let edited = service
        .resolve(project.id, RevisionSelector::Revision(second.revision_id))
        .unwrap();
    named(
        &edited,
        &[
            "PlatformArchitecture",
            "ModelingPlatform",
            "experimentalService",
        ],
    );
    assert_eq!(edited.manifest().accepted_publications, standard_ids);
    assert_eq!(
        service
            .repository()
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        first
    );
    assert_eq!(
        restored.revision().semantic_fingerprint().unwrap(),
        fingerprint
    );
    assert!(service.repository().check_integrity().unwrap().is_ok());
    assert!(restored.validated().is_some());
    assert!(edited.validated().is_some());
    assert_eq!(
        service
            .repository()
            .get_branch(project.id, branch.id)
            .unwrap()
            .head,
        second.revision_id
    );
    assert_eq!(
        service
            .resolve(project.id, RevisionSelector::Revision(first))
            .unwrap()
            .revision()
            .semantic_fingerprint()
            .unwrap(),
        fingerprint
    );
    eprintln!(
        "self_model_documents=5 validated_authored_revisions=2 branches=2 cold_process_restart=true"
    );
}

#[test]
#[ignore = "heavy accepted-cache durable scale; run only after focused lifecycle gate"]
fn durable_scale_100_mixed_documents_10_revisions_four_readers() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("scale.db");
    let service = open(&database, 2);
    let project = service.create_project("durable scale", None).unwrap();
    let initial = service
        .repository()
        .get_branch(project.id, project.default_branch)
        .unwrap()
        .head;
    let documents = (0..100)
        .map(|i| {
            if i % 2 == 0 {
                add(
                    &format!("doc{i}.kerml"),
                    SourceLanguage::KerMl,
                    &format!("package K{i} {{ datatype C{i}; }}"),
                )
            } else {
                add(
                    &format!("doc{i}.sysml"),
                    SourceLanguage::SysMl,
                    &if i == 1 {
                        "package S1 { part def P1 { attribute count : ScalarValues::Integer = 1; } }"
                            .into()
                    } else {
                        format!("package S{i} {{ part def P{i}; }}")
                    },
                )
            }
        })
        .collect();
    let first = apply(
        &service,
        &project,
        project.default_branch,
        initial,
        documents,
        true,
    );
    let branch = service
        .create_branch(project.id, "architecture-experiment", first.revision_id)
        .unwrap();
    let mut last = first.revision_id;
    let mut history = vec![last];
    for index in 2..=10 {
        let started = Instant::now();
        let bound = service
            .resolve(project.id, RevisionSelector::Revision(last))
            .unwrap();
        let changed = replace(
            &bound,
            "doc1.sysml",
            &format!("= {};", index - 1),
            &format!("= {index};"),
        );
        last = if index == 10 {
            // Both writers construct against R9. Exactly one candidate becomes
            // R10; the other remains inspectable without registering a revision.
            let left = service
                .prepare_changes(ApplyDocumentChanges {
                    operation_id: OperationId::new(),
                    project: project.id,
                    branch: branch.id,
                    expected_head: last,
                    changes: vec![changed.clone()],
                    validate: false,
                })
                .unwrap();
            let right = service
                .prepare_changes(ApplyDocumentChanges {
                    operation_id: OperationId::new(),
                    project: project.id,
                    branch: branch.id,
                    expected_head: last,
                    changes: vec![changed],
                    validate: false,
                })
                .unwrap();
            assert_ne!(left.revision().revision(), right.revision().revision());
            let barrier = std::sync::Barrier::new(2);
            let results = std::thread::scope(|scope| {
                let a = scope.spawn(|| {
                    barrier.wait();
                    service.commit_prepared(&left)
                });
                let b = scope.spawn(|| {
                    barrier.wait();
                    service.commit_prepared(&right)
                });
                [a.join().unwrap(), b.join().unwrap()]
            });
            assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
            let winner = results
                .iter()
                .find_map(|result| result.as_ref().ok())
                .unwrap();
            let loser = results
                .iter()
                .find_map(|result| result.as_ref().err())
                .unwrap();
            assert!(matches!(
                loser,
                ServiceError::Repository(RepositoryError::Conflict { expected, actual })
                    if *expected == last && *actual == winner.revision_id
            ));
            assert_eq!(left.revision().documents().count(), 100);
            assert_eq!(right.revision().documents().count(), 100);
            eprintln!("durable_scale_concurrent_cas_winners=1 conflicts=1");
            winner.revision_id
        } else {
            apply(
                &service,
                &project,
                branch.id,
                last,
                vec![changed],
                index % 3 != 0,
            )
            .revision_id
        };
        history.push(last);
        eprintln!(
            "durable_scale_revision={index} edit_and_commit_ms={}",
            started.elapsed().as_millis()
        );
    }
    let bound = service
        .resolve(project.id, RevisionSelector::Revision(last))
        .unwrap();
    let expected = expectation(
        &project,
        &bound,
        &[&["S1", "P1"], &["K0", "C0"]],
        RestartScenario::Scale {
            initial_empty: initial,
            first_authored: first.revision_id,
            experiment: branch.id,
        },
    );
    assert!(bound.validated().is_none());
    drop(bound);
    drop(service);
    restart_in_fresh_process(&database, &expected);
    let repository = SqliteRepository::open(&database).unwrap();
    assert_eq!(
        repository
            .list_revision_history(project.id, last)
            .unwrap()
            .len(),
        11
    );
    assert_eq!(
        repository
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        first.revision_id
    );
    let revisions: Vec<_> = history
        .iter()
        .map(|id| repository.load_revision(project.id, *id).unwrap())
        .collect();
    let blobs: std::collections::BTreeSet<_> = revisions
        .iter()
        .flat_map(|m| m.documents.iter().map(|d| d.content_digest))
        .collect();
    assert_eq!(blobs.len(), 109);
    assert!(
        revisions
            .iter()
            .any(|r| matches!(r.validation, ValidationState::Working))
    );
    assert!(
        revisions
            .iter()
            .any(|r| matches!(r.validation, ValidationState::Validated(_)))
    );
    assert_eq!(revisions.len(), 10);
    assert!(
        revisions
            .iter()
            .all(|manifest| manifest.documents.len() == 100)
    );
    assert!(
        repository
            .load_revision(project.id, initial)
            .unwrap()
            .documents
            .is_empty()
    );
    assert_eq!(repository.list_revisions(project.id).unwrap().len(), 11);
    assert_eq!(repository.list_branches(project.id).unwrap().len(), 2);
    assert!(repository.check_integrity().unwrap().is_ok());
    let cache_digests: std::collections::BTreeSet<_> = revisions
        .iter()
        .filter_map(|manifest| {
            manifest
                .semantic_cache
                .as_ref()
                .map(|cache| cache.content_digest)
        })
        .collect();
    let cache_sizes: Vec<_> = cache_digests
        .iter()
        .map(|digest| repository.read_blob(*digest).unwrap().len())
        .collect();
    eprintln!(
        "durable_scale_cache_blobs={} cache_total_bytes={} cache_max_bytes={} sqlite_database_bytes={}",
        cache_sizes.len(),
        cache_sizes.iter().sum::<usize>(),
        cache_sizes.iter().max().copied().unwrap_or(0),
        std::fs::metadata(&database).unwrap().len()
    );
    eprintln!(
        "durable_scale_documents=100 authored_revisions=10 initial_empty_revisions=1 branches=2 readers=4 unique_source_blobs=109 cold_process_restart=true"
    );
}
