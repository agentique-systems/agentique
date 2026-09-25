//! Exact accepted-runtime oracle for the command used by Native Studio.
//! Measurements describe this process, never grant semantic acceptance.
use agq_kerml_semantics::{Completeness, QualifiedName, QueryResult};
use agq_kerml_text::{
    ProjectChange, SourceLanguage, library::CanonicalKermlStandardLibraries,
    sysml::CanonicalSysmlSystemsLibrary,
};
use agq_kernel::ElementId;
use agq_modeling_agent::{AgentContext, AgentPolicy, ModelCommand};
use agq_modeling_repository::OperationId;
use agq_modeling_service::{ApplyDocumentChanges, ModelingService, RevisionSelector};
use agq_modeling_workspace::WorkingProjectRevision;
use std::{fs::File, path::Path, sync::Arc, time::Instant};

fn named(revision: &WorkingProjectRevision, segments: &[&str]) -> ElementId {
    let answer = revision.kerml_queries().unwrap().lookup_path(
        revision.root(),
        &QualifiedName {
            absolute: false,
            segments: segments.iter().map(|s| (*s).into()).collect(),
        },
    );
    assert_eq!(answer.completeness, Completeness::Complete);
    let ids: std::collections::BTreeSet<_> = answer.value.iter().map(|m| m.element).collect();
    assert_eq!(ids.len(), 1, "{segments:?}: {answer:?}");
    *ids.first().unwrap()
}

fn assert_query<T: std::fmt::Debug + PartialEq>(left: &QueryResult<T>, right: &QueryResult<T>) {
    let mut context = right.context.clone();
    context.revision = left.context.revision;
    assert_eq!(left.context, context);
    assert_eq!(left.value, right.value);
    assert_eq!(left.completeness, right.completeness);
    assert_eq!(left.diagnostics, right.diagnostics);
    assert_eq!(left.positive_dependencies, right.positive_dependencies);
    assert_eq!(left.search_dependencies, right.search_dependencies);
    assert_eq!(left.explanations, right.explanations);
    assert_eq!(left.fact_origins, right.fact_origins);
    assert_eq!(left.declared_fact_origins, right.declared_fact_origins);
    assert_eq!(left.canonical_dependencies, right.canonical_dependencies);
}

fn assert_equivalent(left: &WorkingProjectRevision, right: &WorkingProjectRevision) {
    assert_eq!(
        left.semantic_fingerprint().unwrap(),
        right.semantic_fingerprint().unwrap()
    );
    let a = left.semantic_model().unwrap();
    let b = right.semantic_model().unwrap();
    assert!(
        a.elements().eq(b.elements()),
        "canonical records, IDs and provenance"
    );
    assert!(
        a.association_occurrences().eq(b.association_occurrences()),
        "relationship identity and order"
    );
    assert_eq!(
        left.producer_closure().unwrap().semantic_closure_digest(),
        right.producer_closure().unwrap().semantic_closure_digest(),
    );
    assert_eq!(left.references().len(), right.references().len());
    for (a, b) in left.references().iter().zip(right.references()) {
        assert_eq!(a.relationship, b.relationship);
        assert_eq!(a.specific, b.specific);
        assert_eq!(a.name, b.name);
        assert_eq!(a.origin, b.origin);
        assert_query(&a.resolution, &b.resolution);
    }
    // Only the fresh kernel revision label differs; source and canonical identities do not.
    let normalized = format!("{:?}", right.diagnostics()).replace(
        &format!("{:?}", right.kernel_revision().unwrap()),
        &format!("{:?}", left.kernel_revision().unwrap()),
    );
    assert_eq!(format!("{:?}", left.diagnostics()), normalized);
    let path = ["PlatformArchitecture", "ModelingPlatform", "alphaObserver"];
    let a = named(left, &path);
    assert_eq!(a, named(right, &path));
    let l = left.sysml_queries().unwrap().effective_usages(a);
    let r = right.sysml_queries().unwrap().effective_usages(a);
    assert_query(&l.kerml, &r.kerml);
    assert_eq!(l.completeness(), r.completeness());
    assert_eq!(l.pending, r.pending);
    assert_eq!(l.diagnostics, r.diagnostics);
    assert_eq!(l.rejected_targets, r.rejected_targets);
    assert_eq!(l.filtered_targets, r.filtered_targets);
    assert_eq!(l.supporting_queries.len(), r.supporting_queries.len());
    for (a, b) in l.supporting_queries.iter().zip(&r.supporting_queries) {
        assert_query(a, b);
    }
    assert_eq!(l.supporting_names.len(), r.supporting_names.len());
    for (a, b) in l.supporting_names.iter().zip(&r.supporting_names) {
        assert_query(a, b);
    }
    assert!(l.observations.keys().eq(r.observations.keys()));
    for (fact, a) in &l.observations {
        assert_query(a, &r.observations[fact]);
    }
}

#[test]
#[ignore = "requires exact accepted runtime caches; never acquires or rebuilds standards"]
fn create_part_command_matches_full_self_model_reconstruction() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Open both before expensive restoration: unavailable input fails immediately.
    let kerml_file = File::open(
        std::env::var_os("AGENTIQUE_KERML_CACHE").expect("accepted KerML cache required"),
    )
    .unwrap();
    let systems_file = File::open(
        std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").expect("accepted Systems cache required"),
    )
    .unwrap();
    let sources = agq_standard_libraries::VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let started = Instant::now();
    let kerml =
        Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml_file, &sources).unwrap());
    let accepted = Arc::new(
        CanonicalSysmlSystemsLibrary::restore_cache(systems_file, &sources, kerml).unwrap(),
    );
    let runtime_restore_ms = started.elapsed().as_millis();
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(
        agq_modeling_sqlite::SqliteRepository::open(directory.path().join("command.sqlite"))
            .unwrap(),
    );
    let service = ModelingService::new(store, accepted, 8);
    let project = service
        .create_project("Agentique command oracle", None)
        .unwrap();
    let initial = service
        .repository()
        .get_branch(project.id, project.default_branch)
        .unwrap()
        .head;
    let mut documents: Vec<_> = std::fs::read_dir(root.join("models/agentique"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sysml"))
        .collect();
    documents.sort();
    let seed = service
        .apply_document_changes(ApplyDocumentChanges {
            operation_id: OperationId::new(),
            project: project.id,
            branch: project.default_branch,
            expected_head: initial,
            changes: documents
                .into_iter()
                .map(|path| ProjectChange::Add {
                    path: path.file_name().unwrap().to_str().unwrap().into(),
                    language: SourceLanguage::SysMl,
                    source: std::fs::read_to_string(path).unwrap(),
                })
                .collect(),
            validate: true,
        })
        .unwrap();
    let base = service
        .resolve(project.id, RevisionSelector::Revision(seed.revision_id))
        .unwrap();
    assert!(
        base.validated().is_some(),
        "real self-model must be Validated first"
    );
    let before = base.revision().semantic_fingerprint().unwrap();
    let owner = named(
        base.revision(),
        &["PlatformArchitecture", "ModelingPlatform"],
    );
    let started = Instant::now();
    let mut candidate = agq_modeling_agent::propose(
        &service,
        &AgentPolicy::operator(),
        AgentContext {
            project: project.id,
            branch: project.default_branch,
            revision: seed.revision_id,
            selection: vec![owner],
        },
        ModelCommand::CreatePartUsage {
            owner,
            name: "alphaObserver".into(),
            definition: None,
        },
    )
    .unwrap();
    let command_prepare_ms = started.elapsed().as_millis();
    let incremental = candidate.prepared().revision().clone();
    assert!(
        candidate
            .source_preview
            .after
            .contains("part alphaObserver;")
    );
    assert_eq!(
        service
            .repository()
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        seed.revision_id
    );
    assert_eq!(
        base.revision().semantic_fingerprint().unwrap(),
        before,
        "candidate preserves current revision"
    );
    let started = Instant::now();
    let full = agq_modeling_workspace::testing::full_rebuild(&incremental).unwrap();
    let full_rebuild_ms = started.elapsed().as_millis();
    assert_equivalent(&incremental, &full);
    let started = Instant::now();
    candidate.validate(&AgentPolicy::operator()).unwrap();
    let validation_ms = started.elapsed().as_millis();
    full.validate().unwrap();
    // Keep semantic compile timings separate from command source mapping and service preparation.
    println!(
        "CREATE_PART_PERFORMANCE {}",
        serde_json::json!({
            "format": "agentique-create-part-performance/1",
            "model": "models/agentique",
            "runtime_restore_ms": runtime_restore_ms,
            "command_prepare_ms": command_prepare_ms,
            "incremental_compile_ms": incremental.compilation_timings().total_compile_micros as f64 / 1000.0,
            "full_rebuild_ms": full_rebuild_ms,
            "full_compile_ms": full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "validation_ms": validation_ms,
            "incremental_phases": incremental.compilation_timings(),
            "full_phases": full.compilation_timings(),
            "incremental_work": incremental.compilation_work(),
            "full_work": full.compilation_work(),
            "canonical_elements": incremental.semantic_model().unwrap().elements().count(),
            "authored_documents": incremental.documents().count(),
            "exact_equivalence": true,
            "peak_memory_bytes": null,
            "memory_scope": "measure the process externally; no per-edit allocation claim",
            "comparison_scope": "identical parsed source and canonical identity inputs; parsing occurs before compilation",
        })
    );
}
