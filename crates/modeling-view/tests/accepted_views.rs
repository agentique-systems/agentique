//! Opt-in bounded authored acceptance over the exact already accepted caches.
//! No fallback acquires sources or rebuilds/publishes a standard library.
use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries, sysml::CanonicalSysmlSystemsLibrary,
};
use agq_modeling_view::*;
use agq_modeling_workspace::ProjectWorkspace;
use agq_standard_libraries::VerifiedLibrarySet;
use std::{collections::BTreeSet, fs::File, path::Path, sync::Arc};

fn accepted() -> Arc<CanonicalSysmlSystemsLibrary> {
    // Fail immediately if explicitly requested without both authenticated inputs.
    let kerml = File::open(
        std::env::var_os("AGENTIQUE_KERML_CACHE").expect("AGENTIQUE_KERML_CACHE required"),
    )
    .unwrap();
    let systems = File::open(
        std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").expect("AGENTIQUE_SYSTEMS_CACHE required"),
    )
    .unwrap();
    let sources = VerifiedLibrarySet::load_from_directory(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    )
    .unwrap();
    let kerml = Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml, &sources).unwrap());
    Arc::new(CanonicalSysmlSystemsLibrary::restore_cache(systems, &sources, kerml).unwrap())
}

#[test]
#[ignore = "requires accepted KerML and Systems caches; never rebuilds publications"]
fn authored_view_inspector_proof_and_second_revision_keep_exact_semantic_identity() {
    let mut workspace = ProjectWorkspace::open(accepted()).unwrap();
    let first = workspace.add_sysml(workspace.head().revision(),"Studio.sysml",
        "package StudioFixture { part def Engine; part def System { part engine : Engine; port control; } }").unwrap();
    first.validate().unwrap();
    let architecture = project(&first, &ViewDefinition::architecture()).unwrap();
    assert!(!architecture.nodes.is_empty());
    assert!(
        architecture
            .nodes
            .iter()
            .all(|node| matches!(node.semantic_kind.as_str(), "PartDefinition" | "PartUsage"))
    );
    let system = architecture
        .nodes
        .iter()
        .find(|node| node.name == "System")
        .unwrap()
        .id;
    let engine = architecture
        .nodes
        .iter()
        .find(|node| node.name == "engine")
        .unwrap()
        .id;
    assert_eq!(architecture.metadata.suggested_focus, Some(system));
    let inspector = inspect(&first, system).unwrap();
    assert_eq!(inspector.revision_id, architecture.revision_id);
    assert!(
        inspector
            .owned_features
            .iter()
            .any(|feature| feature.id == engine)
    );
    assert!(
        inspector
            .queries
            .iter()
            .all(|query| query.completeness == "Complete")
    );
    let location = inspector.source.unwrap();
    let source = first.document(location.document_id).unwrap().source();
    let owner_source = &source[location.start as usize..location.end as usize];
    assert!(owner_source.contains("part def System"));
    assert!(owner_source.contains("part engine : Engine"));
    let mut definition = ViewDefinition::semantic_graph();
    definition.include_standard_library = true;
    let graph = project(&first, &definition).unwrap();
    let identities: BTreeSet<_> = graph.nodes.iter().map(|node| node.id).collect();
    for edge in &graph.edges {
        assert_eq!(edge.revision_id, first.revision());
        assert!(identities.contains(&edge.source) && identities.contains(&edge.target));
        assert!(first.element(edge.relationship_id.unwrap()).is_some());
    }
    let derived = graph
        .edges
        .iter()
        .find(|edge| edge.origin == ViewOrigin::Derived)
        .expect("effective SysML derived relationship");
    let explanation = explain(&first, derived.relationship_id.unwrap()).unwrap();
    assert_eq!(explanation.revision_id, first.revision());
    assert_eq!(explanation.rule_id, derived.rule_id);
    assert!(explanation.evidence_count > 0);
    assert!(explanation.edges.iter().all(|edge| edge.presentation_only));
    let requirements = project(&first, &ViewDefinition::requirements()).unwrap();
    assert!(requirements.nodes.is_empty()); // no fabricated requirements
    let encoded = serde_json::to_vec(&architecture).unwrap();
    let second = workspace
        .add_sysml(first.revision(), "Notes.sysml", "package DesignNotes;")
        .unwrap();
    assert_ne!(first.revision(), second.revision());
    assert_eq!(
        serde_json::to_vec(&project(&first, &ViewDefinition::architecture()).unwrap()).unwrap(),
        encoded
    );
    let later = project(&second, &ViewDefinition::architecture()).unwrap();
    assert!(
        later
            .nodes
            .iter()
            .all(|node| node.revision_id == second.revision())
    );
    assert!(later.nodes.iter().any(|node| node.id == system));
    let mut hidden = ViewDefinition::architecture();
    hidden.hidden_elements.push(engine);
    assert!(
        !project(&first, &hidden)
            .unwrap()
            .nodes
            .iter()
            .any(|node| node.id == engine)
    );
    assert!(first.element(engine).is_some());
}
