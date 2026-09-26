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
use agq_modeling_workspace::{ProjectRevisionCheckpoint, WorkingProjectRevision};
use std::{collections::BTreeMap, fs::File, path::Path, sync::Arc, time::Instant};

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
        left.checkpoint(),
        right.checkpoint(),
        "exact source/arena/retirement checkpoint"
    );
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
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.name, b.name);
        assert_eq!(a.origin, b.origin);
        assert_eq!(a.alias(), b.alias());
        assert_eq!(a.visibility(), b.visibility());
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
    // Only the fresh kernel revision differs. In particular the enclosing
    // SysML dependency contract, bindings, profiles and metamodel stay exact.
    let mut context = r.context.clone();
    context.kerml.revision = l.context.kerml.revision;
    assert_eq!(l.context, context);
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

/// Malformed reuse requests must fail before they can become candidate graphs.
/// The ordinary cold restore tests remain independent of this new entry point.
fn assert_restore_rejections(
    predecessor: &WorkingProjectRevision,
    checkpoint: &ProjectRevisionCheckpoint,
    sources: &BTreeMap<agq_kernel::DocumentId, String>,
) -> usize {
    let mut checked = 0;
    let mut rejects = |label: &str,
                       changed: &ProjectRevisionCheckpoint,
                       bytes: &BTreeMap<agq_kernel::DocumentId, String>| {
        let result = changed.restore_sharing_dependency(predecessor, bytes);
        assert!(result.is_err(), "reuse accepted invalid {label}");
        eprintln!("shared_mount_negative {label}: {}", result.unwrap_err());
        checked += 1;
    };
    let mut changed = checkpoint.clone();
    changed.format_version += 1;
    rejects("checkpoint version", &changed, sources);
    changed = checkpoint.clone();
    changed.parent_revision_id = None;
    rejects("missing parent", &changed, sources);
    changed.parent_revision_id = Some(agq_modeling_workspace::ProjectRevisionId::new());
    rejects("foreign parent", &changed, sources);
    changed = checkpoint.clone();
    changed.project_revision_id = predecessor.revision();
    rejects("reused project revision", &changed, sources);
    changed = checkpoint.clone();
    changed.source.format_version += 1;
    rejects("source checkpoint version", &changed, sources);
    changed = checkpoint.clone();
    changed.source.project_id = agq_kerml_text::ProjectId::new();
    rejects("foreign source project", &changed, sources);
    changed = checkpoint.clone();
    changed.source.root = ElementId::new();
    rejects("foreign canonical root", &changed, sources);
    changed = checkpoint.clone();
    changed.source.accepted_kerml[0] ^= 1;
    rejects("foreign accepted KerML", &changed, sources);
    changed = checkpoint.clone();
    changed.source.accepted_sysml[0] ^= 1;
    rejects("foreign accepted Systems", &changed, sources);
    let mut bytes = sources.clone();
    bytes.values_mut().next().unwrap().push(' ');
    rejects("changed source digest", checkpoint, &bytes);
    bytes = sources.clone();
    bytes.pop_first();
    rejects("missing source", checkpoint, &bytes);
    bytes = sources.clone();
    let (_, source) = bytes.pop_first().unwrap();
    bytes.insert(agq_kernel::DocumentId::new(), source);
    rejects("substituted source identity", checkpoint, &bytes);
    changed = checkpoint.clone();
    changed.source.documents[1].document_id = changed.source.documents[0].document_id;
    rejects("duplicate document identity", &changed, sources);
    changed = checkpoint.clone();
    changed.source.documents[1].path = changed.source.documents[0].path.clone();
    rejects("duplicate document path", &changed, sources);
    changed = checkpoint.clone();
    let syntax = &mut changed.source.documents[0].syntax_nodes;
    syntax[1].id = syntax[0].id;
    rejects("duplicate syntax identity", &changed, sources);
    changed = checkpoint.clone();
    changed.source.documents[0].syntax_nodes.pop();
    rejects("incomplete syntax arena", &changed, sources);
    checked
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
    let ancestor = named(base.revision(), &["PlatformArchitecture"]);
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
    let command_full = candidate.prepared().revision().clone();
    assert!(
        agq_modeling_workspace::testing::shares_accepted_dependency(base.revision(), &command_full),
        "source command reuses the exact authenticated dependency mount"
    );
    assert_eq!(
        named(&command_full, &["PlatformArchitecture", "ModelingPlatform"]),
        owner,
        "the command preserves its owner's canonical identity"
    );
    assert_eq!(
        named(&command_full, &["PlatformArchitecture"]),
        ancestor,
        "the command preserves containing architecture identity"
    );
    let added = named(
        &command_full,
        &["PlatformArchitecture", "ModelingPlatform", "alphaObserver"],
    );
    let ownership = command_full.kerml_queries().unwrap().owner(added);
    assert_eq!(ownership.completeness, Completeness::Complete);
    assert_eq!(ownership.value, Some(owner));
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
    // Reusing candidate.inputs via full_rebuild is not an independent mount
    // oracle: it would reuse the same optimization being qualified. Cold restore
    // must mint its own authenticated mount over identical sources/identities.
    let checkpoint = command_full.checkpoint();
    let sources: BTreeMap<_, _> = command_full
        .documents()
        .map(|(_, document)| (document.id(), document.source().to_owned()))
        .collect();
    let negative_cases = assert_restore_rejections(base.revision(), &checkpoint, &sources);
    eprintln!("oracle: independent cold checkpoint restore");
    let started = Instant::now();
    let full = checkpoint
        .restore(command_full.accepted_sysml().clone(), &sources)
        .unwrap();
    let cold_checkpoint_restore_ms = started.elapsed().as_millis();
    assert!(
        !agq_modeling_workspace::testing::shares_accepted_dependency(&command_full, &full),
        "cold oracle must authenticate a separate mount"
    );
    assert_equivalent(&command_full, &full);
    let started = Instant::now();
    candidate.validate(&AgentPolicy::operator()).unwrap();
    let validation_ms = started.elapsed().as_millis();
    let started = Instant::now();
    full.validate().unwrap();
    let cold_validation_ms = started.elapsed().as_millis();
    assert!(!command_full.compilation_work().semantic_cache_used);
    assert!(!full.compilation_work().semantic_cache_used);
    assert_eq!(
        command_full.compilation_work().documents_reparsed,
        command_full.documents().count()
    );
    assert_eq!(
        full.compilation_work().documents_reparsed,
        full.documents().count()
    );
    // Keep semantic compile timings separate from command source mapping and service preparation.
    println!(
        "CREATE_PART_PERFORMANCE {}",
        serde_json::json!({
            "format": "agentique-create-part-performance/3",
            "model": "models/agentique",
            "command_reconstruction_mode": "full-source-reconstruction-with-verified-insertion-identities-and-shared-authenticated-mount",
            "comparison_reconstruction_mode": "independent-cold-checkpoint-restore-with-identical-source-and-canonical-identities",
            "runtime_restore_ms": runtime_restore_ms,
            "command_prepare_ms": command_prepare_ms,
            "command_compile_ms": command_full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "cold_checkpoint_restore_ms": cold_checkpoint_restore_ms,
            "full_compile_ms": full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "validation_ms": validation_ms,
            "cold_validation_ms": cold_validation_ms,
            "command_noncompile_residual_ms": command_prepare_ms as f64 - command_full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "cold_noncompile_residual_ms": cold_checkpoint_restore_ms as f64 - full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "residual_scope": "mixed source/checkpoint/proof/postcheck/serialization overhead; not a measurement of mount time",
            "command_phases": command_full.compilation_timings(),
            "full_phases": full.compilation_timings(),
            "command_work": command_full.compilation_work(),
            "full_work": full.compilation_work(),
            "command_closure_counters": command_full.producer_status().map(|status| &status.counters),
            "full_closure_counters": full.producer_status().map(|status| &status.counters),
            "command_certificate_build_ms": command_full.producer_status().map(|status| status.counters.certificate_build_micros as f64 / 1000.0),
            "full_certificate_build_ms": full.producer_status().map(|status| status.counters.certificate_build_micros as f64 / 1000.0),
            "closure_counter_scope": "final closure invocation cumulative counters; latest-round-only metrics intentionally omitted; certificate work is part of final_closure, not additional wall time",
            "canonical_elements": command_full.semantic_model().unwrap().elements().count(),
            "authored_documents": command_full.documents().count(),
            "exact_equivalence": true,
            "owner_and_ancestor_identity_preserved": true,
            "same_authenticated_mount_observed": true,
            "independent_cold_mount_observed": true,
            "negative_reuse_cases": negative_cases,
            "incremental_speedup_claimed": false,
            "peak_memory_bytes": null,
            "memory_scope": "measure the process externally; no per-edit allocation claim",
            "comparison_scope": "two full reconstructions over identical source/checkpoint identities; cold restore independently authenticates its mount; hot command additionally includes source proof and continuity postchecks; no incremental semantic work-elimination claim",
        })
    );
}
