//! Two-process, source-only recovery gate over an isolated operator-created project.
//! Usage: source_recovery create|restore ROOT RUNTIME DATABASE EXPECTATION.json
use agq_modeling_agent::{AgentContext, AgentPolicy, ModelCommand};
use agq_modeling_repository::{ModelingRepository, Project, RevisionManifest};
use agq_modeling_service::{BoundRevision, ModelingService, RevisionLoadPath, RevisionSelector};
use agq_modeling_view::ViewDefinition;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

#[derive(Serialize, Deserialize)]
struct Expected {
    project: Project,
    observations: Vec<serde_json::Value>,
    manifests: Vec<RevisionManifest>,
}

fn observe(bound: &BoundRevision) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let revision = bound.revision();
    let graph = agq_modeling_view::project(revision, &ViewDefinition::semantic_graph())?;
    let inspectors = graph
        .nodes
        .iter()
        .map(|node| agq_modeling_view::inspect(revision, node.id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::json!({
        "fingerprint": revision.semantic_fingerprint().map_err(|e| format!("{e:?}"))?,
        "checkpoint": revision.checkpoint(),
        "graph": graph,
        "system": agq_modeling_view::project(revision, &ViewDefinition::architecture())?,
        "requirements": agq_modeling_view::project(revision, &ViewDefinition::requirements())?,
        "inspectors": inspectors,
        "validated": bound.validated().is_some(),
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 5 {
        return Err("expected create|restore ROOT RUNTIME DATABASE EXPECTATION.json".into());
    }
    let mode = args[0].to_str().ok_or("invalid mode")?;
    let root = PathBuf::from(&args[1]);
    let runtime_dir = PathBuf::from(&args[2]);
    let database = PathBuf::from(&args[3]);
    let expectation = PathBuf::from(&args[4]);
    if !database.is_absolute()
        || !expectation.is_absolute()
        || !matches!(mode, "create" | "restore")
    {
        return Err(
            "explicit mode and absolute isolated database/expectation paths required".into(),
        );
    }
    if mode == "create" && (database.exists() || expectation.exists()) {
        return Err("creation gate refuses existing database/evidence".into());
    }
    if mode == "restore" && (!database.is_file() || !expectation.is_file()) {
        return Err("restore requires the completed creation process".into());
    }
    let runtime = agq_runtime_publications::load(
        &agq_runtime_publications::RuntimeConfig {
            runtime_dir: Some(runtime_dir),
            ..Default::default()
        },
        &root,
        |phase| eprintln!("{phase:?}"),
    )?;
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = Arc::new(agq_modeling_sqlite::SqliteRepository::open(&database)?);
    let service = ModelingService::new(store.clone(), runtime.systems, 8);
    if mode == "create" {
        let project = service.create_project("Source recovery acceptance", None)?;
        let mut head = store.get_branch(project.id, project.default_branch)?.head;
        let initial = service.resolve(project.id, RevisionSelector::Revision(head))?;
        assert!(initial.manifest().documents.is_empty() && initial.validated().is_none());
        let mut expected = Expected {
            project: project.clone(),
            observations: vec![],
            manifests: vec![],
        };
        for (path, source, language) in [
            (
                "Recovery.sysml",
                "package Recovery { part def Assembly { part member; } }",
                agq_kerml_text::SourceLanguage::SysMl,
            ),
            (
                "Types.kerml",
                "package Types { class Marker; }",
                agq_kerml_text::SourceLanguage::KerMl,
            ),
        ] {
            let mut candidate = agq_modeling_agent::propose(
                &service,
                &AgentPolicy::operator(),
                AgentContext {
                    project: project.id,
                    branch: project.default_branch,
                    revision: head,
                    selection: vec![],
                },
                ModelCommand::AddSourceDocument {
                    path: path.into(),
                    source: source.into(),
                    language,
                },
            )?;
            assert_eq!(
                store.get_branch(project.id, project.default_branch)?.head,
                head
            );
            candidate.validate(&AgentPolicy::operator())?;
            let receipt = candidate.commit(&service, &AgentPolicy::operator())?;
            head = receipt.revision_id;
            let bound = service.resolve(project.id, RevisionSelector::Revision(head))?;
            assert!(bound.validated().is_some());
            expected.observations.push(observe(&bound)?);
            expected.manifests.push(bound.manifest().clone());
        }
        std::fs::write(&expectation, serde_json::to_vec_pretty(&expected)?)?;
        println!(
            "{}",
            serde_json::json!({"phase":"created_and_committed", "revisions":2, "source_only_recovery":"pending separate process"})
        );
    } else {
        let expected: Expected = serde_json::from_slice(&std::fs::read(&expectation)?)?;
        for (manifest, observation) in expected.manifests.iter().zip(&expected.observations) {
            let started = std::time::Instant::now();
            let bound = service.resolve(
                expected.project.id,
                RevisionSelector::Revision(manifest.revision_id),
            )?;
            assert_eq!(bound.load_path(), RevisionLoadPath::DurableSource);
            assert!(!bound.revision().compilation_work().semantic_cache_used);
            assert_eq!(bound.manifest(), manifest);
            assert_eq!(&observe(&bound)?, observation);
            println!(
                "{}",
                serde_json::json!({"phase":"source_only_restore", "revision":manifest.revision_id, "elapsed_ms":started.elapsed().as_secs_f64()*1000.0, "exact_identity_projection_inspector_equivalence":true})
            );
        }
        assert_eq!(
            store
                .get_branch(expected.project.id, expected.project.default_branch)?
                .head,
            expected.manifests.last().unwrap().revision_id
        );
        println!(
            "{}",
            serde_json::json!({"outcome":"passed", "separate_process_source_only_recovery":true})
        );
    }
    Ok(())
}
