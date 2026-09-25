//! Run only when an existing accepted bundle/pair is supplied. This never publishes standards.
use agq_modeling_agent::{AgentContext, ModelCommand};
use agq_modeling_view::{RelationshipFamily, ViewDefinition, ViewOrigin};
use agq_studio_platform::{CandidatePhase, NativeConfig, RevisionBinding};

#[test]
#[ignore = "requires existing accepted runtime bundle/pair; never rebuilds publication"]
fn native_in_process_self_model_candidate_commit_and_restore() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = NativeConfig::for_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::env::var_os("AGENTIQUE_RUNTIME_DIR").map(Into::into),
    )
    .unwrap();
    config.database = directory.path().join("native.sqlite");
    config.runtime.bundle = std::env::var_os("AGENTIQUE_STUDIO_BUNDLE").map(Into::into);
    let mut platform = agq_studio_platform::open(&config, |_| {}).unwrap();
    let project = platform
        .projects()
        .unwrap()
        .into_iter()
        .find(|p| p.name == "Agentique")
        .unwrap();
    let history = platform.history(project.id).unwrap();
    let revision = history
        .branches
        .iter()
        .find(|b| b.id == project.default_branch)
        .unwrap()
        .head;
    let binding = RevisionBinding {
        project: project.id,
        revision,
    };
    let definition = ViewDefinition {
        include_standard_library: true,
        ..ViewDefinition::semantic_graph()
    };
    let graph = platform.project(binding, &definition).unwrap();
    let owner = graph
        .nodes
        .iter()
        .find(|node| node.name == "ModelingPlatform" && node.semantic_kind == "PartDefinition")
        .unwrap()
        .id;
    assert_eq!(
        platform.inspect(binding, owner).unwrap().revision_id,
        revision
    );
    let relation = graph
        .edges
        .iter()
        .find(|edge| edge.origin == ViewOrigin::Derived && edge.relationship_id.is_some())
        .unwrap()
        .relationship_id
        .unwrap();
    assert!(
        !platform
            .explain(binding, relation)
            .unwrap()
            .nodes
            .is_empty()
    );
    assert_eq!(
        platform
            .dependencies(binding, owner, RelationshipFamily::all(), 2, false)
            .unwrap()
            .revision_id,
        revision
    );
    let context = AgentContext {
        project: project.id,
        branch: project.default_branch,
        revision,
        selection: vec![owner],
    };
    let candidate = platform
        .propose(
            context.clone(),
            ModelCommand::CreatePartUsage {
                owner,
                name: "nativeObserver".into(),
                definition: None,
            },
            &definition,
        )
        .unwrap();
    assert_eq!(candidate.phase, CandidatePhase::Working);
    let added = candidate
        .projection
        .nodes
        .iter()
        .find(|node| node.name == "nativeObserver")
        .unwrap()
        .id;
    let source = platform.source_candidate(candidate.id, added).unwrap();
    assert_eq!(source.binding.revision, candidate.projection.revision_id);
    assert!(source.source.contains("part nativeObserver;"));
    assert!(
        platform.source(binding, added).is_err(),
        "candidate identity is absent from its parent source"
    );
    assert!(
        platform.commit(candidate.id).is_err(),
        "unvalidated preview cannot commit"
    );
    assert_eq!(
        platform
            .history(project.id)
            .unwrap()
            .branches
            .iter()
            .find(|b| b.id == project.default_branch)
            .unwrap()
            .head,
        revision
    );
    let validated = platform.validate(candidate.id, &definition).unwrap();
    assert_eq!(
        validated.projection.revision_id,
        candidate.projection.revision_id
    );
    let competing = platform
        .propose(
            context,
            ModelCommand::CreatePartUsage {
                owner,
                name: "competingObserver".into(),
                definition: None,
            },
            &definition,
        )
        .unwrap();
    platform.validate(competing.id, &definition).unwrap();
    let receipt = platform.commit(candidate.id).unwrap();
    assert_eq!(receipt, platform.commit(candidate.id).unwrap());
    assert!(platform.cancel(candidate.id).is_err());
    assert!(
        platform.commit(competing.id).is_err(),
        "durable CAS protects the moved head"
    );
    platform
        .cancel(competing.id)
        .expect("explicit CAS refusal permits cancellation");
    assert!(
        !platform
            .compare(project.id, revision, receipt.revision_id, &definition)
            .unwrap()
            .changes
            .declared
            .added
            .is_empty()
    );
    assert_eq!(
        platform.project(binding, &definition).unwrap(),
        graph,
        "old revision remains immutable"
    );
    drop(platform);
    let restored = agq_studio_platform::open(&config, |_| {}).unwrap();
    let history = restored.history(project.id).unwrap();
    assert_eq!(
        history
            .branches
            .iter()
            .find(|b| b.id == project.default_branch)
            .unwrap()
            .head,
        receipt.revision_id
    );
}
