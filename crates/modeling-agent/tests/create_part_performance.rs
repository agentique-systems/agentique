//! Exact accepted-runtime oracle for the command used by Native Studio.
//! Measurements describe this process, never grant semantic acceptance.
use agq_kerml_semantics::{Completeness, QualifiedName};
use agq_kerml_text::{
    ProjectChange, SourceLanguage, library::CanonicalKermlStandardLibraries,
    sysml::CanonicalSysmlSystemsLibrary,
};
use agq_kernel::ElementId;
use agq_modeling_agent::{AgentContext, AgentPolicy, ModelCommand};
use agq_modeling_repository::{ContentDigest, OperationId};
use agq_modeling_service::{ApplyDocumentChanges, ModelingService, RevisionSelector};
use agq_modeling_workspace::{ProjectRevisionCheckpoint, WorkingProjectRevision};
use std::{
    collections::BTreeMap,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

fn oracle_source_root() -> PathBuf {
    let supplied = std::env::var_os("AGENTIQUE_SOURCE_ROOT");
    let selected = supplied.as_ref().map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let root = selected.canonicalize().unwrap_or_else(|error| {
        panic!(
            "oracle source root {} is unavailable: {error}; set AGENTIQUE_SOURCE_ROOT to the Agentique checkout",
            selected.display()
        )
    });
    for relative in [
        "Cargo.toml",
        "standards/normative/sysml-2.0/library-set.json",
    ] {
        assert!(
            root.join(relative).is_file(),
            "oracle source root {} is missing {relative}",
            root.display()
        );
    }
    let models = root.join("models/agentique");
    assert!(
        std::fs::read_dir(&models).is_ok_and(|entries| entries.filter_map(Result::ok).any(
            |entry| {
                let path = entry.path();
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "sysml")
            }
        )),
        "oracle source root {} has no models/agentique/*.sysml inputs",
        root.display()
    );
    eprintln!(
        "oracle source root ({}): {}",
        if supplied.is_some() {
            "AGENTIQUE_SOURCE_ROOT"
        } else {
            "compiled default"
        },
        root.display()
    );
    root
}

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
    // This file remains portable to the baseline checkout, which has only the
    // ordinary command API. The current-only controlled entry imports this
    // module; preserve the correct child test path in either test executable.
    let test_name = "create_part_command_matches_full_self_model_reconstruction";
    let entry = module_path!().split_once("::").map_or_else(
        || test_name.to_owned(),
        |(_, module)| format!("{module}::{test_name}"),
    );
    run_with_command(agq_modeling_agent::propose, &entry, "default-disabled");
}

pub type OracleCommand =
    fn(
        &ModelingService,
        &AgentPolicy,
        AgentContext,
        ModelCommand,
    ) -> Result<agq_modeling_agent::AgentCandidate, agq_modeling_agent::AgentError>;

/// The same independent process oracle can qualify another command entry point.
/// This changes no observation, normalization, invalid-input or equivalence gate.
pub fn run_with_command(command: OracleCommand, test_entry: &str, control_mode: &str) {
    match std::env::var("AGENTIQUE_CREATE_PART_ORACLE_STAGE").as_deref() {
        Ok("command") => command_child(&oracle_directory(), command, control_mode),
        Ok("cold") => cold_child(&oracle_directory()),
        _ => orchestrate(test_entry, control_mode),
    }
}

fn command_child(output: &Path, propose: OracleCommand, control_mode: &str) {
    let root = oracle_source_root();
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
    eprintln!("CREATE_PART_RUNTIME {{\"runtime_restore_ms\":{runtime_restore_ms}}}");
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
    let mut candidate = propose(
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
    eprintln!("command prepared in {command_prepare_ms} ms");
    let command_full = candidate.prepared().revision().clone();
    eprintln!(
        "CREATE_PART_PREPARATION {}",
        serde_json::json!({
            "command_prepare_ms": command_prepare_ms,
            "command_phases": command_full.compilation_timings(),
            "command_work": command_full.compilation_work(),
            "command_closure_counters": command_full.producer_status().map(|status| &status.counters),
            "equivalence_status": "independent cold oracle has not run yet",
        })
    );
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
    let started = Instant::now();
    candidate.validate(&AgentPolicy::operator()).unwrap();
    let validation_ms = started.elapsed().as_millis();
    assert!(!command_full.compilation_work().semantic_cache_used);
    write_json(output, "checkpoint", &checkpoint);
    write_json(output, "sources", &sources);
    write_observations(output, "command", &command_full);
    write_json(
        output,
        "command-metrics",
        &serde_json::json!({
            "runtime_restore_ms": runtime_restore_ms,
            "command_control_mode": control_mode,
            "command_prepare_ms": command_prepare_ms,
            "command_compile_ms": command_full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "validation_ms": validation_ms,
            "command_phases": command_full.compilation_timings(),
            "command_work": command_full.compilation_work(),
            "command_closure_counters": command_full.producer_status().map(|status| &status.counters),
            "canonical_elements": command_full.semantic_model().unwrap().elements().count(),
            "authored_documents": command_full.documents().count(),
            "negative_reuse_cases": negative_cases,
            "owner_and_ancestor_identity_preserved": true,
            "same_authenticated_mount_observed": true,
            "validated": true,
        }),
    );
}

fn oracle_directory() -> std::path::PathBuf {
    std::env::var_os("AGENTIQUE_CREATE_PART_ORACLE_DIR")
        .expect("oracle output directory")
        .into()
}

fn write_json(output: &Path, name: &str, value: &impl serde::Serialize) {
    let file = File::create(output.join(format!("{name}.json"))).unwrap();
    serde_json::to_writer(std::io::BufWriter::new(file), value).unwrap();
}

fn read_json<T: serde::de::DeserializeOwned>(output: &Path, name: &str) -> T {
    serde_json::from_reader(std::io::BufReader::new(
        File::open(output.join(format!("{name}.json"))).unwrap(),
    ))
    .unwrap()
}

/// Exact per-observation hashes keep the independent oracles out of each
/// other's address spaces. Debug includes full query evidence/private status;
/// only the freshly allocated kernel revision label is normalized.
fn write_observations(output: &Path, label: &str, revision: &WorkingProjectRevision) {
    use agq_sysml::classes as sc;
    let kernel_revision = format!("{:?}", revision.kernel_revision().unwrap());
    let mut observations = BTreeMap::new();
    fn observe(
        observations: &mut BTreeMap<String, ContentDigest>,
        revision: &str,
        key: String,
        value: &impl std::fmt::Debug,
    ) {
        let exact = format!("{value:?}").replace(revision, "<fresh-kernel-revision>");
        assert!(
            observations
                .insert(key, ContentDigest::of(exact.as_bytes()))
                .is_none()
        );
    }
    macro_rules! observe {
        ($key:expr, $value:expr) => {
            observe(
                &mut observations,
                &kernel_revision,
                $key.to_string(),
                &$value,
            )
        };
    }
    observe!("source-checkpoint", revision.checkpoint());
    observe!(
        "semantic-fingerprint",
        revision.semantic_fingerprint().unwrap()
    );
    observe!(
        "semantic-closure",
        revision
            .producer_closure()
            .unwrap()
            .semantic_closure_digest()
    );
    observe!("diagnostics", revision.diagnostics());
    observe!("strict-audit", revision.effective_audit().unwrap().report());
    observe!(
        "strict-audit-subjects",
        revision.effective_audit().unwrap().subjects()
    );
    let model = revision.semantic_model().unwrap();
    for record in model.elements() {
        observe!(format!("element/{}", record.id()), record);
        for (property, _) in record.slots() {
            observe!(
                format!("declared-slot/{}/{property}", record.id()),
                model.declared_slot(record.id(), property)
            );
        }
    }
    for occurrence in model.association_occurrences() {
        observe!(format!("occurrence/{}", occurrence.id()), occurrence);
    }
    for (fact, searches) in model.computation_searches() {
        observe!(format!("searches/{fact:?}"), searches);
    }
    for (key, contribution) in model.ordered_reference_contributions() {
        observe!(format!("contribution/{key:?}"), contribution);
    }
    for (key, navigation) in model.derived_navigation_results() {
        observe!(format!("navigation/{key:?}"), navigation);
    }
    for (key, failure) in model.computation_failures() {
        observe!(format!("failure/{key:?}"), failure);
    }
    for reference in revision.references() {
        observe!(format!("reference/{}", reference.relationship), reference);
    }
    let bound = revision.sysml_queries().unwrap();
    let subjects = revision.effective_audit().unwrap().subjects();
    for batch in subjects.chunks(32) {
        let q = bound.fork();
        for &subject in batch {
            let class = model.element(subject).unwrap().metaclass();
            let is = |base| model.registry().is_subtype(class, base).unwrap_or(false);
            macro_rules! query {
                ($operation:ident) => {
                    observe!(
                        format!("query/{subject}/{}", stringify!($operation)),
                        q.$operation(subject)
                    )
                };
            }
            if is(sc::DEFINITION) || is(sc::USAGE) {
                query!(effective_names);
                query!(current_effective_usages);
                query!(effective_usages);
                query!(effective_ports);
                query!(effective_return_parameters);
            }
            if is(sc::DEFINITION) {
                query!(direct_specializations);
                query!(effective_supertypes);
            }
            if is(sc::USAGE) {
                query!(current_usage_types);
                query!(effective_usage_types);
                query!(effective_subsetted_features);
                query!(effective_redefined_features);
            }
            if is(sc::OCCURRENCE_USAGE) {
                query!(current_occurrence_definitions);
                query!(effective_occurrence_definitions);
            }
            if is(sc::CONNECTION_USAGE) {
                query!(current_connection_definitions);
                query!(effective_connection_definitions);
            }
            if is(sc::ATTRIBUTE_USAGE) {
                query!(current_attribute_definitions);
                query!(effective_attribute_definitions);
            }
            if is(sc::ITEM_USAGE) {
                query!(current_item_definitions);
                query!(effective_item_definitions);
            }
            if is(sc::PART_USAGE) {
                query!(current_part_definitions);
                query!(effective_part_definitions);
            }
            if is(sc::PORT_USAGE) {
                query!(current_port_definitions);
                query!(effective_port_definitions);
            }
            if is(sc::CONNECTOR_AS_USAGE) {
                query!(current_connection_related_features);
                query!(effective_connection_related_features);
            }
            if is(sc::CONNECTOR_AS_USAGE) || is(sc::CONNECTION_DEFINITION) {
                query!(effective_connection_ends);
            }
            if is(sc::INTERFACE_DEFINITION) || is(sc::INTERFACE_USAGE) {
                query!(effective_interface_ends);
            }
            if is(sc::OCCURRENCE_DEFINITION) || is(sc::OCCURRENCE_USAGE) {
                query!(effective_parameters);
            }
            if is(sc::ACTION_DEFINITION) || is(sc::ACTION_USAGE) {
                query!(effective_subactions);
            }
        }
    }
    observe!(
        "created-part-identity",
        named(
            revision,
            &["PlatformArchitecture", "ModelingPlatform", "alphaObserver"]
        )
    );
    write_json(output, &format!("{label}-observations"), &observations);
    eprintln!(
        "{label}: exported {} exact semantic observations",
        observations.len()
    );
}

fn cold_child(output: &Path) {
    let root = oracle_source_root();
    let kerml_file = File::open(std::env::var_os("AGENTIQUE_KERML_CACHE").unwrap()).unwrap();
    let systems_file = File::open(std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").unwrap()).unwrap();
    let libraries = agq_standard_libraries::VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let started = Instant::now();
    let kerml =
        Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml_file, &libraries).unwrap());
    let accepted = Arc::new(
        CanonicalSysmlSystemsLibrary::restore_cache(systems_file, &libraries, kerml).unwrap(),
    );
    let runtime_restore_ms = started.elapsed().as_millis();
    let checkpoint: ProjectRevisionCheckpoint = read_json(output, "checkpoint");
    let sources: BTreeMap<agq_kernel::DocumentId, String> = read_json(output, "sources");
    eprintln!("oracle: independent process cold checkpoint restore");
    let started = Instant::now();
    let full = checkpoint.restore(accepted, &sources).unwrap();
    let cold_checkpoint_restore_ms = started.elapsed().as_millis();
    eprintln!(
        "CREATE_PART_COLD {}",
        serde_json::json!({
            "cold_checkpoint_restore_ms": cold_checkpoint_restore_ms,
            "full_phases": full.compilation_timings(), "full_work": full.compilation_work(),
        })
    );
    assert!(!full.compilation_work().semantic_cache_used);
    assert_eq!(
        full.compilation_work().documents_reparsed,
        full.documents().count()
    );
    let started = Instant::now();
    full.validate().unwrap();
    let cold_validation_ms = started.elapsed().as_millis();
    write_observations(output, "cold", &full);
    write_json(
        output,
        "cold-metrics",
        &serde_json::json!({
            "cold_runtime_restore_ms": runtime_restore_ms,
            "cold_checkpoint_restore_ms": cold_checkpoint_restore_ms,
            "full_compile_ms": full.compilation_timings().total_compile_micros as f64 / 1000.0,
            "cold_validation_ms": cold_validation_ms,
            "full_phases": full.compilation_timings(), "full_work": full.compilation_work(),
            "full_closure_counters": full.producer_status().map(|status| &status.counters),
            "validated": true,
        }),
    );
}

fn orchestrate(test_entry: &str, control_mode: &str) {
    let source_root = oracle_source_root();
    let temporary;
    let directory = if let Some(directory) = std::env::var_os("AGENTIQUE_CREATE_PART_ORACLE_OUTPUT")
    {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    } else {
        temporary = tempfile::tempdir().unwrap();
        temporary.path().to_path_buf()
    };
    for stage in ["command", "cold"] {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .env("AGENTIQUE_SOURCE_ROOT", &source_root)
            .args([
                test_entry,
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("AGENTIQUE_CREATE_PART_ORACLE_STAGE", stage)
            .env("AGENTIQUE_CREATE_PART_ORACLE_DIR", &directory)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "{stage} process failed: {status}; retained output: {}",
            directory.display()
        );
    }
    let command: BTreeMap<String, ContentDigest> = read_json(&directory, "command-observations");
    let cold: BTreeMap<String, ContentDigest> = read_json(&directory, "cold-observations");
    assert!(
        command.keys().eq(cold.keys()),
        "exact observation population"
    );
    for (key, expected) in &command {
        assert_eq!(expected, &cold[key], "exact semantic observation: {key}");
    }
    let mut measured: serde_json::Value = read_json(&directory, "command-metrics");
    let cold: serde_json::Value = read_json(&directory, "cold-metrics");
    assert_eq!(measured["command_control_mode"], control_mode);
    assert_eq!(measured["validated"], true);
    assert_eq!(cold["validated"], true);
    measured
        .as_object_mut()
        .unwrap()
        .extend(cold.as_object().unwrap().clone());
    measured["format"] = "agentique-create-part-performance/4".into();
    measured["exact_equivalence"] = true.into();
    measured["exact_observation_count"] = command.len().into();
    measured["comparison_scope"] = "separate-process exact per-record, occurrence, provenance, derivation, search, reference, strict-audit and full applicable local effective-query SHA-256 observations; only fresh kernel revision labels normalized; both revisions independently validate".into();
    measured["independent_cold_mount_observed"] = true.into();
    measured["command_reconstruction_mode"] =
        "identity-preserving-source-command-see-work-counters".into();
    measured["command_vs_cold_wall_speedup"] =
        (measured["cold_checkpoint_restore_ms"].as_f64().unwrap()
            / measured["command_prepare_ms"].as_f64().unwrap())
        .into();
    write_json(&directory, "result", &measured);
    println!("CREATE_PART_PERFORMANCE {measured}");
}
