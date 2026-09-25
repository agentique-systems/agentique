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
    let mut preserved_hierarchy = Vec::new();
    let mut current = Some(owner);
    while let Some(id) = current {
        let Some(node) = graph.nodes.iter().find(|node| node.id == id) else {
            break;
        };
        assert!(
            !preserved_hierarchy
                .iter()
                .any(|(previous, _, _, _)| *previous == id),
            "real source ownership must not cycle"
        );
        preserved_hierarchy.push((
            node.id,
            node.owner,
            node.name.clone(),
            node.semantic_kind.clone(),
        ));
        current = node.owner;
    }
    assert!(
        preserved_hierarchy.len() >= 2,
        "the real model gate must cover an ancestor as well as the selected owner"
    );
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
    for (id, owner, name, kind) in &preserved_hierarchy {
        let node = candidate
            .projection
            .nodes
            .iter()
            .find(|node| node.id == *id)
            .expect("candidate preserves owner and ancestor canonical identities");
        assert_eq!(&node.owner, owner);
        assert_eq!(&node.name, name);
        assert_eq!(&node.semantic_kind, kind);
    }
    assert!(
        candidate
            .projection
            .nodes
            .iter()
            .any(|node| node.id == owner),
        "nested insertion preserves selected owner's canonical identity"
    );
    let added = candidate
        .projection
        .nodes
        .iter()
        .find(|node| node.name == "nativeObserver")
        .unwrap()
        .id;
    assert_eq!(
        candidate
            .projection
            .nodes
            .iter()
            .find(|node| node.id == added)
            .unwrap()
            .owner,
        Some(owner)
    );
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
    let committed = history
        .revisions
        .iter()
        .find(|manifest| manifest.revision_id == receipt.revision_id)
        .unwrap();
    assert!(
        committed
            .metadata
            .alias
            .iter()
            .any(|alias| alias == "agentique-source-identity/part-insertion/1"),
        "validation, commit and restart retain the command identity policy"
    );
    let identity_evidence: serde_json::Value =
        serde_json::from_str(committed.metadata.description.as_deref().unwrap()).unwrap();
    assert_eq!(identity_evidence["mode"], "full-source-reconstruction");
    assert_eq!(identity_evidence["owner"], serde_json::json!(owner));
    assert_eq!(
        identity_evidence["retired_identity_reservations_preserved"],
        true
    );
    assert_eq!(
        history
            .branches
            .iter()
            .find(|b| b.id == project.default_branch)
            .unwrap()
            .head,
        receipt.revision_id
    );
    let restored_binding = RevisionBinding {
        project: project.id,
        revision: receipt.revision_id,
    };
    let restored_graph = restored.project(restored_binding, &definition).unwrap();
    for (id, owner, name, kind) in &preserved_hierarchy {
        let node = restored_graph
            .nodes
            .iter()
            .find(|node| node.id == *id)
            .expect("durable restart preserves owner and ancestor canonical identities");
        assert_eq!(&node.owner, owner);
        assert_eq!(&node.name, name);
        assert_eq!(&node.semantic_kind, kind);
    }
    assert!(restored_graph.nodes.iter().any(|node| node.id == owner));
    assert_eq!(
        restored_graph
            .nodes
            .iter()
            .find(|node| node.id == added)
            .unwrap()
            .owner,
        Some(owner)
    );

    // The normal restart above may use its authenticated project graph cache.
    // Independently force durable source/checkpoint restoration in this isolated
    // test database; accepted standard caches, source blobs and manifests stay intact.
    let cache_digest = committed.semantic_cache.as_ref().unwrap().content_digest;
    assert_ne!(cache_digest, committed.checkpoint_digest);
    assert!(
        committed
            .documents
            .iter()
            .all(|document| document.content_digest != cache_digest)
    );
    drop(restored);
    let database = rusqlite::Connection::open(&config.database).unwrap();
    assert_eq!(
        database
            .execute("DELETE FROM blobs WHERE digest=?1", [cache_digest.hex()])
            .unwrap(),
        1
    );
    drop(database);

    let runtime = agq_runtime_publications::load(&config.runtime, &config.root, |_| {}).unwrap();
    let repository =
        std::sync::Arc::new(agq_modeling_sqlite::SqliteRepository::open(&config.database).unwrap());
    let service = std::sync::Arc::new(agq_modeling_service::ModelingService::new(
        repository,
        runtime.systems,
        8,
    ));
    let from_source = service
        .resolve(
            project.id,
            agq_modeling_service::RevisionSelector::Revision(receipt.revision_id),
        )
        .unwrap();
    assert_eq!(
        from_source.load_path(),
        agq_modeling_service::RevisionLoadPath::DurableSource
    );
    assert!(
        from_source.validated().is_some(),
        "source replay repeats the ordinary validation gate"
    );
    let mut source_platform = agq_studio_platform::StudioPlatform::new(
        service,
        agq_modeling_agent::AgentPolicy::operator(),
    );
    assert_eq!(
        source_platform
            .project(restored_binding, &definition)
            .unwrap(),
        restored_graph,
        "source-only restart retains exact semantic projection identities"
    );

    let second = source_platform
        .propose(
            AgentContext {
                project: project.id,
                branch: project.default_branch,
                revision: receipt.revision_id,
                selection: vec![added],
            },
            ModelCommand::CreatePartUsage {
                // The first new part ended with a semicolon. This second command
                // expands that body, testing both owner forms across a restart.
                owner: added,
                name: "afterRestartObserver".into(),
                definition: Some(
                    graph
                        .nodes
                        .iter()
                        .find(|node| {
                            node.name == "ModelRepository" && node.semantic_kind == "PartDefinition"
                        })
                        .unwrap()
                        .id,
                ),
            },
            &definition,
        )
        .unwrap();
    assert_eq!(second.phase, CandidatePhase::Working);
    for (id, expected_owner, name, kind) in &preserved_hierarchy {
        let node = second
            .projection
            .nodes
            .iter()
            .find(|node| node.id == *id)
            .expect("second insertion preserves the original hierarchy");
        assert_eq!(&node.owner, expected_owner);
        assert_eq!(&node.name, name);
        assert_eq!(&node.semantic_kind, kind);
    }
    let first_part = second
        .projection
        .nodes
        .iter()
        .find(|node| node.id == added)
        .expect("second insertion retains the first inserted identity");
    assert_eq!(first_part.owner, Some(owner));
    let second_added = second
        .projection
        .nodes
        .iter()
        .find(|node| node.name == "afterRestartObserver")
        .unwrap();
    assert_ne!(second_added.id, added);
    assert_eq!(second_added.owner, Some(added));
    source_platform.validate(second.id, &definition).unwrap();
    let second_receipt = source_platform.commit(second.id).unwrap();
    assert_ne!(second_receipt.revision_id, receipt.revision_id);
    assert_eq!(
        source_platform
            .project(restored_binding, &definition)
            .unwrap(),
        restored_graph,
        "the source-restored predecessor remains immutable after a second commit"
    );
    assert_eq!(
        source_platform
            .history(project.id)
            .unwrap()
            .branches
            .iter()
            .find(|branch| branch.id == project.default_branch)
            .unwrap()
            .head,
        second_receipt.revision_id
    );
}
