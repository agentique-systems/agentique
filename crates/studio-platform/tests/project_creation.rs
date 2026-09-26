//! Explicit accepted-runtime test of the operator project/import lifecycle.
//! This never rebuilds a standards publication or uses a fixture as authority.
use agq_modeling_agent::{AgentContext, AgentPolicy, ModelCommand};
use agq_modeling_repository::{ModelingRepository, ValidationState};
use agq_modeling_service::ModelingService;
use agq_modeling_view::ViewDefinition;
use agq_studio_platform::{CandidatePhase, RevisionBinding, SourceLanguage, StudioPlatform};
use std::{path::PathBuf, sync::Arc};

#[test]
#[ignore = "requires installed accepted runtime; explicit project/import acceptance gate"]
fn operator_project_import_retains_durable_boundaries_on_rejection_cancel_and_commit() {
    // Release test artifacts may be built by CI and exercised on the operator's
    // machine. The accepted runtime still authenticates this explicit checkout's
    // pinned authority; a missing source root is never an acquisition request.
    let root = std::env::var_os("AGENTIQUE_SOURCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .canonicalize()
        .expect("project creation acceptance source root must exist");
    assert!(
        root.join("standards/normative/sysml-2.0/library-set.json")
            .is_file()
    );
    let runtime = agq_runtime_publications::load(
        &agq_runtime_publications::RuntimeConfig {
            runtime_dir: std::env::var_os("AGENTIQUE_RUNTIME_DIR").map(Into::into),
            bundle: std::env::var_os("AGENTIQUE_STUDIO_BUNDLE").map(Into::into),
            ..Default::default()
        },
        &root,
        |_| {},
    )
    .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let original = Arc::new(
        agq_modeling_sqlite::SqliteRepository::open(temporary.path().join("original.sqlite"))
            .unwrap(),
    );
    let service = Arc::new(ModelingService::new(original.clone(), runtime.systems, 8));
    let original_project = service.create_project("Original", None).unwrap();
    let original_head = original
        .get_branch(original_project.id, original_project.default_branch)
        .unwrap()
        .head;
    let mut platform = StudioPlatform::new(service.clone(), AgentPolicy::operator());
    let target = temporary.path().join("new-repository/project.sqlite");

    assert!(platform.create_project_at(" ", &target).is_err());
    assert!(
        platform
            .create_project_at("New", std::path::Path::new("relative.sqlite"))
            .is_err()
    );
    let mut machine = StudioPlatform::new(service, AgentPolicy::agent("test-agent"));
    assert!(machine.create_project_at("Unauthorized", &target).is_err());
    assert!(
        !target.parent().unwrap().exists(),
        "invalid or unauthorized creation must not touch the requested repository location"
    );
    assert_eq!(platform.projects().unwrap(), vec![original_project.clone()]);

    let (project, projects) = platform
        .create_project_at("  Imported design  ", &target)
        .unwrap();
    assert_eq!(project.name, "Imported design");
    assert_eq!(projects, vec![project.clone()]);
    assert_eq!(
        original.list_projects().unwrap(),
        vec![original_project.clone()]
    );
    assert_eq!(
        original
            .get_branch(original_project.id, original_project.default_branch)
            .unwrap()
            .head,
        original_head
    );
    assert!(
        platform.history(original_project.id).is_err(),
        "new repository host must not resolve the old repository's project"
    );
    let store = agq_modeling_sqlite::SqliteRepository::open(&target).unwrap();
    let initial_head = store
        .get_branch(project.id, project.default_branch)
        .unwrap()
        .head;
    let initial = store.load_revision(project.id, initial_head).unwrap();
    assert!(initial.documents.is_empty());
    assert!(matches!(initial.validation, ValidationState::Working));
    let context = || AgentContext {
        project: project.id,
        branch: project.default_branch,
        revision: initial_head,
        selection: vec![],
    };
    let definition = ViewDefinition::semantic_graph();
    let source =
        "// exact imported bytes\r\npackage Imported { part def Assembly { part member; } }\r\n";
    let import = || ModelCommand::AddSourceDocument {
        path: "Assembly.sysml".into(),
        source: source.into(),
        language: SourceLanguage::SysMl,
    };
    let candidate = platform.propose(context(), import(), &definition).unwrap();
    assert_eq!(candidate.phase, CandidatePhase::Working);
    assert_eq!(
        candidate.base,
        RevisionBinding {
            project: project.id,
            revision: initial_head
        }
    );
    assert!(candidate.source_preview.before.is_empty());
    assert_eq!(candidate.source_preview.after.as_bytes(), source.as_bytes());
    assert!(
        platform.commit(candidate.id).is_err(),
        "unvalidated import cannot durably advance head"
    );
    let blocked_repository = temporary.path().join("blocked/project.sqlite");
    assert!(
        platform
            .create_project_at("Cannot abandon candidate", &blocked_repository)
            .is_err()
    );
    assert!(!blocked_repository.parent().unwrap().exists());
    platform.cancel(candidate.id).unwrap();
    assert_eq!(
        store
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        initial_head
    );
    assert_eq!(
        store.load_revision(project.id, initial_head).unwrap(),
        initial
    );
    assert_eq!(store.list_revisions(project.id).unwrap().len(), 1);

    let candidate = platform.propose(context(), import(), &definition).unwrap();
    assert_eq!(
        platform.validate(candidate.id, &definition).unwrap().phase,
        CandidatePhase::Validated
    );
    let committed = platform.commit(candidate.id).unwrap();
    assert_eq!(
        store
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        committed.revision_id
    );
    assert_eq!(store.list_revisions(project.id).unwrap().len(), 2);
    let mut current = context();
    current.revision = committed.revision_id;
    assert!(
        platform.propose(current, import(), &definition).is_err(),
        "an import must not overwrite an existing source path"
    );
    assert!(
        platform
            .propose(
                context(),
                ModelCommand::AddSourceDocument {
                    path: "Stale.sysml".into(),
                    source: source.into(),
                    language: SourceLanguage::SysMl
                },
                &definition
            )
            .is_err(),
        "stale expected head must not become a candidate base"
    );
    assert_eq!(
        store
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        committed.revision_id
    );
    assert_eq!(store.list_revisions(project.id).unwrap().len(), 2);

    // A failed new repository open must retain the current authenticated host.
    let directory = temporary.path().join("is-a-directory.sqlite");
    std::fs::create_dir(&directory).unwrap();
    assert!(
        platform
            .create_project_at("Cannot open directory as SQLite", &directory)
            .is_err()
    );
    assert_eq!(platform.projects().unwrap(), vec![project.clone()]);
    assert_eq!(
        store
            .get_branch(project.id, project.default_branch)
            .unwrap()
            .head,
        committed.revision_id
    );
}
