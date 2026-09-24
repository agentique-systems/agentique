//! Accepted-publication integration tests; integration remains gated by language readiness.
mod support;
use agq_kerml_semantics::{
    Completeness, QualifiedName, QueryResult, SearchDependency, SemanticClosureRequirement,
};
use agq_kerml_syntax::production::Production;
use agq_kerml_text::{DocumentStatus, ProjectChange, SourceLanguage};
use agq_kernel::{
    DocumentId, ElementId,
    provenance::{Dependency, FactKey, Origin},
};
use agq_modeling_workspace::{ValidatedProjectRevision, WorkingProjectRevision};
use agq_sysml_semantics::{SysmlQueryResult, SysmlSemanticContextId};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    sync::{Arc, Barrier},
    time::Instant,
};
use support::*;

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn mixed_documents_and_edits_retain_old_revisions_and_inherited_ids() {
    let mut workspace = open();
    let r0 = workspace.head().clone();
    assert_shared(&r0);
    let contracts = workspace
        .add_kerml(r0.revision(), "Contracts.kerml", inputs::CONTRACTS)
        .unwrap();
    let repository = workspace
        .add_sysml(contracts.revision(), "Repository.sysml", inputs::REPOSITORY)
        .unwrap();
    let r1 = workspace
        .add_sysml(repository.revision(), "Workspace.sysml", inputs::WORKSPACE)
        .unwrap();
    assert_valid(&r1);
    assert_eq!(r1.parent(), Some(repository.revision()));
    assert_eq!(
        BTreeSet::from([
            r0.revision(),
            contracts.revision(),
            repository.revision(),
            r1.revision()
        ])
        .len(),
        4
    );
    let r1_signature = immutable_signature(&r1);
    let original_repository = element(&r1, &["Modeling", "Workspace", "repository"]);
    let repository_syntax = syntax_id(
        &r1,
        "Workspace.sysml",
        Production::PartUsage,
        "part repository : Storage::Repository;",
    );
    let insertion = inputs::WORKSPACE.find("    }\n").unwrap();
    let r2 = workspace
        .apply(
            r1.revision(),
            [edit(
                &r1,
                "Workspace.sysml",
                insertion,
                insertion,
                inputs::WORKSPACE_PORT,
            )],
        )
        .unwrap();
    assert_valid(&r2);
    let r2_signature = immutable_signature(&r2);
    let bus = element(&r2, &["Modeling", "Workspace", "bus"]);
    let insertion = r2
        .document_at("Workspace.sysml")
        .unwrap()
        .source()
        .rfind('}')
        .unwrap();
    let r3 = workspace
        .apply(
            r2.revision(),
            [edit(
                &r2,
                "Workspace.sysml",
                insertion,
                insertion,
                inputs::WORKSPACE_SPECIALIZATION,
            )],
        )
        .unwrap();
    assert_valid(&r3);
    for revision in [&r2, &r3] {
        assert_eq!(
            element(revision, &["Modeling", "Workspace", "repository"]),
            original_repository
        );
        assert_eq!(
            syntax_id(
                revision,
                "Workspace.sysml",
                Production::PartUsage,
                "part repository : Storage::Repository;",
            ),
            repository_syntax,
            "an unaffected declaration in an edited document retains syntax identity"
        );
        let document = revision.document_at("Workspace.sysml").unwrap();
        assert_eq!(
            document.id(),
            r1.document_at("Workspace.sysml").unwrap().id()
        );
        assert_ne!(
            document.revision(),
            r1.document_at("Workspace.sysml").unwrap().revision()
        );
    }
    assert_ne!(
        r2.document_at("Workspace.sysml").unwrap().revision(),
        r3.document_at("Workspace.sysml").unwrap().revision()
    );
    assert_eq!(immutable_signature(&r1), r1_signature);
    assert_eq!(immutable_signature(&r2), r2_signature);
    assert!(Arc::ptr_eq(workspace.revision(r1.revision()).unwrap(), &r1));
    assert!(Arc::ptr_eq(workspace.head(), &r3));
    let q1 = r1.sysml_queries().unwrap();
    let old_ports = q1.effective_ports(element(&r1, &["Modeling", "Workspace"]));
    assert_eq!(old_ports.completeness(), Completeness::Complete);
    assert!(!old_ports.value().contains(&bus));
    let q = r3.sysml_queries().unwrap();
    let specialized = element(&r3, &["Modeling", "SpecializedWorkspace"]);
    let ports = q.effective_ports(specialized);
    assert_eq!(ports.completeness(), Completeness::Complete, "{ports:?}");
    assert!(
        ports.value().contains(&bus),
        "inheritance returns the original port ID"
    );
    assert_eq!(
        q.kerml().owning_type(bus).value,
        Some(element(&r3, &["Modeling", "Workspace"]))
    );
    let children = q.effective_usages(specialized);
    assert_eq!(children.completeness(), Completeness::Complete);
    assert!(!children.value().contains(&original_repository));
    assert!(children.value().iter().any(|&child| {
        q.kerml()
            .all_redefined_features(child)
            .value
            .contains(&original_repository)
    }));
    assert!(
        q.model().element(original_repository).is_some(),
        "redefinition suppresses lookup, not original records"
    );
    let unchanged = r1.document_at("Contracts.kerml").unwrap();
    assert!(std::ptr::eq(
        unchanged,
        r3.document_at("Contracts.kerml").unwrap()
    ));
    assert!(std::ptr::eq(
        unchanged.production_syntax().unwrap(),
        r3.document_at("Contracts.kerml")
            .unwrap()
            .production_syntax()
            .unwrap()
    ));
    assert_eq!(
        element(&r1, &["PlatformContracts", "RevisionValue"]),
        element(&r3, &["PlatformContracts", "RevisionValue"])
    );
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn recovered_and_unresolved_edits_are_working_then_repair_to_validated() {
    let mut workspace = open();
    let valid = seed(&mut workspace);
    assert_valid(&valid);
    let retained = valid.validate().unwrap();
    let signature = immutable_signature(&valid);
    let provider = element(&valid, &["Storage", "Repository"]);
    let recovered = workspace
        .apply(
            valid.revision(),
            [replace_by_edit(
                &valid,
                "Repository.sysml",
                inputs::RECOVERED_REPOSITORY,
            )],
        )
        .unwrap();
    assert_ne!(recovered.revision(), valid.revision());
    assert!(recovered.validate().is_err());
    assert!(!recovered.diagnostics().is_empty());
    let document = recovered.document_at("Repository.sysml").unwrap();
    assert_eq!(document.source(), inputs::RECOVERED_REPOSITORY);
    assert_eq!(document.status(), DocumentStatus::Recovered);
    assert_eq!(
        document.id(),
        valid.document_at("Repository.sysml").unwrap().id()
    );
    assert_ne!(
        document.revision(),
        valid.document_at("Repository.sysml").unwrap().revision()
    );
    assert!(
        recovered
            .semantic_model()
            .is_none_or(|model| model.element(provider).is_none())
    );
    if let Ok(q) = recovered.kerml_queries() {
        let lookup = q.lookup_path(
            recovered.root(),
            &QualifiedName {
                absolute: false,
                segments: vec!["Storage".into(), "Repository".into()],
            },
        );
        assert_ne!(
            lookup.completeness,
            Completeness::Complete,
            "omitted recovered provider cannot prove a closed negative"
        );
    }
    assert_eq!(immutable_signature(retained.working()), signature);
    let repaired = workspace
        .apply(
            recovered.revision(),
            [replace_by_edit(
                &recovered,
                "Repository.sysml",
                inputs::REPOSITORY,
            )],
        )
        .unwrap();
    assert_valid(&repaired);
    let missing = inputs::WORKSPACE.replace("Storage::Repository", "Storage::MissingRepository");
    let unresolved = workspace
        .apply(
            repaired.revision(),
            [replace_by_edit(&repaired, "Workspace.sysml", &missing)],
        )
        .unwrap();
    assert_eq!(
        unresolved.document_at("Workspace.sysml").unwrap().source(),
        missing
    );
    assert_eq!(
        unresolved.document_at("Workspace.sysml").unwrap().status(),
        DocumentStatus::Parsed
    );
    assert_unresolved(&unresolved, None);
    let fixed = workspace
        .apply(
            unresolved.revision(),
            [replace_by_edit(
                &unresolved,
                "Workspace.sysml",
                inputs::WORKSPACE,
            )],
        )
        .unwrap();
    assert_valid(&fixed);
    assert_eq!(
        fixed.document_at("Repository.sysml").unwrap().id(),
        valid.document_at("Repository.sysml").unwrap().id(),
        "temporary recovery is not document removal"
    );
    assert_eq!(immutable_signature(&valid), signature);
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn removal_and_readding_a_path_does_not_resurrect_retired_identity() {
    let mut workspace = open();
    let r1 = seed(&mut workspace);
    assert_valid(&r1);
    let document = r1.document_at("Repository.sysml").unwrap().id();
    let provider = element(&r1, &["Storage", "Repository"]);
    let r2 = workspace
        .apply(r1.revision(), [ProjectChange::Remove { document }])
        .unwrap();
    assert!(r2.document_at("Repository.sysml").is_none());
    assert_unresolved(&r2, Some(provider));
    let r3 = workspace
        .add_sysml(r2.revision(), "Repository.sysml", inputs::REPOSITORY)
        .unwrap();
    assert_valid(&r3);
    assert_ne!(r3.document_at("Repository.sysml").unwrap().id(), document);
    assert_ne!(element(&r3, &["Storage", "Repository"]), provider);
    assert_eq!(element(&r1, &["Storage", "Repository"]), provider);
    assert!(r3.semantic_model().unwrap().element(provider).is_none());
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn operational_failures_publish_nothing_and_independent_projects_share_only_standards() {
    let mut workspace = open();
    let before = workspace.head().clone();
    assert_shared(&before);
    let r1 = seed(&mut workspace);
    assert_valid(&r1);
    let signature = immutable_signature(&r1);
    assert!(
        workspace
            .apply(
                before.revision(),
                [add("Stale.sysml", SourceLanguage::SysMl, "package Stale;")]
            )
            .is_err()
    );
    assert!(
        workspace
            .apply(
                r1.revision(),
                [ProjectChange::Remove {
                    document: DocumentId::new()
                }]
            )
            .is_err()
    );
    assert!(
        workspace
            .apply(
                r1.revision(),
                [add(
                    "Workspace.sysml",
                    SourceLanguage::SysMl,
                    inputs::WORKSPACE
                )]
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(workspace.head(), &r1));
    assert_eq!(immutable_signature(workspace.head()), signature);
    let history_count = workspace.revisions().count();
    assert!(
        workspace
            .apply(
                r1.revision(),
                [
                    add("Prepared.sysml", SourceLanguage::SysMl, "package Prepared;"),
                    ProjectChange::Remove {
                        document: DocumentId::new()
                    },
                ]
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(workspace.head(), &r1));
    assert_eq!(workspace.revisions().count(), history_count);
    assert_eq!(immutable_signature(workspace.head()), signature);
    assert!(workspace.head().document_at("Prepared.sysml").is_none());
    let unicode = "package Labels { part def 'révision'; }\n";
    let r2 = workspace
        .add_sysml(r1.revision(), "Labels.sysml", unicode)
        .unwrap();
    let middle_of_utf8 = unicode.find('é').unwrap() + 1;
    assert!(
        workspace
            .apply(
                r2.revision(),
                [edit(
                    &r2,
                    "Labels.sysml",
                    middle_of_utf8,
                    middle_of_utf8,
                    "x"
                )]
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(workspace.head(), &r2));
    let mut other = open();
    assert_shared(other.head());
    let independent = seed(&mut other);
    assert_valid(&independent);
    assert_ne!(
        element(&independent, &["Storage", "Repository"]),
        element(&r1, &["Storage", "Repository"])
    );
    assert!(Arc::ptr_eq(
        independent.accepted_sysml(),
        r1.accepted_sysml()
    ));
    assert!(Arc::ptr_eq(
        independent.accepted_kerml(),
        r1.accepted_kerml()
    ));
    assert_shared(&r1);
    assert_eq!(immutable_signature(&r1), signature);
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn hundred_documents_five_revisions_and_parallel_borrowed_reads() {
    let started = Instant::now();
    let mut workspace = open();
    assert_shared(workspace.head());
    let mut changes = Vec::with_capacity(100);
    for index in 0..50 {
        changes.push(add(
            &format!("Contracts{index:03}.kerml"),
            SourceLanguage::KerMl,
            &inputs::contracts(index),
        ));
        changes.push(add(
            &format!("Worker{index:03}.sysml"),
            SourceLanguage::SysMl,
            &inputs::worker(index),
        ));
    }
    let s1 = workspace
        .apply(workspace.head().revision(), changes)
        .unwrap();
    assert_valid(&s1);
    assert_eq!(s1.documents().count(), 100);
    // Capture before the next mutation, not after all revisions exist.
    let mut baseline = vec![immutable_signature(&s1)];
    let mut projections = vec![[0, 25, 49].map(|group| group_projection(&s1, group))];
    let original_engine = element(&s1, &["Workbench025", "Worker", "engine"]);
    let original_engine_syntax = syntax_id(
        &s1,
        "Worker025.sysml",
        Production::PartUsage,
        "part engine;",
    );
    let original = inputs::worker(25);
    let insertion = original.find("part engine;").unwrap() + "part engine;".len();
    let s2 = workspace
        .apply(
            s1.revision(),
            [edit(
                &s1,
                "Worker025.sysml",
                insertion,
                insertion,
                " port bus;",
            )],
        )
        .unwrap();
    assert_valid(&s2);
    baseline.push(immutable_signature(&s2));
    projections.push([0, 25, 49].map(|group| group_projection(&s2, group)));
    let source = s2.document_at("Worker025.sysml").unwrap().source();
    let insertion = source.rfind('}').unwrap();
    let s3 = workspace
        .apply(
            s2.revision(),
            [edit(
                &s2,
                "Worker025.sysml",
                insertion,
                insertion,
                "part def SpecializedWorker :> Worker { part :>> engine; } ",
            )],
        )
        .unwrap();
    assert_valid(&s3);
    baseline.push(immutable_signature(&s3));
    projections.push([0, 25, 49].map(|group| group_projection(&s3, group)));
    for revision in [&s2, &s3] {
        assert_eq!(
            element(revision, &["Workbench025", "Worker", "engine"]),
            original_engine
        );
        assert_eq!(
            syntax_id(
                revision,
                "Worker025.sysml",
                Production::PartUsage,
                "part engine;"
            ),
            original_engine_syntax
        );
    }
    let bus = element(&s2, &["Workbench025", "Worker", "bus"]);
    {
        let q = s3.sysml_queries().unwrap();
        let specialized = element(&s3, &["Workbench025", "SpecializedWorker"]);
        let inherited = q.effective_ports(specialized);
        assert_eq!(inherited.completeness(), Completeness::Complete);
        assert_eq!(
            inherited.value(),
            &vec![bus],
            "the one inherited port retains its original identity without copies"
        );
        let owner = q.kerml().owning_type(bus);
        assert_eq!(owner.completeness, Completeness::Complete);
        assert_eq!(owner.value, Some(element(&s3, &["Workbench025", "Worker"])));
        let engine = element(&s3, &["Workbench025", "SpecializedWorker", "engine"]);
        assert_ne!(engine, original_engine);
        let redefined = q.effective_redefined_features(engine);
        assert_eq!(redefined.completeness(), Completeness::Complete);
        assert!(redefined.value().contains(&original_engine));
        let children = q.effective_usages(specialized);
        assert_eq!(children.completeness(), Completeness::Complete);
        assert_eq!(
            children.value().iter().filter(|&&id| id == engine).count(),
            1
        );
        assert!(!children.value().contains(&original_engine));
        assert!(
            q.model().element(original_engine).is_some(),
            "redefinition suppresses the inherited member, not its original record"
        );
    }
    let provider = element(&s3, &["Contracts025", "RevisionValue"]);
    let old_document = s3.document_at("Contracts025.kerml").unwrap().id();
    let s4 = workspace
        .apply(
            s3.revision(),
            [ProjectChange::Remove {
                document: old_document,
            }],
        )
        .unwrap();
    assert_unresolved(&s4, Some(provider));
    assert_eq!(s4.documents().count(), 99);
    baseline.push(immutable_signature(&s4));
    projections.push([0, 25, 49].map(|group| group_projection(&s4, group)));
    let s5 = workspace
        .add_kerml(s4.revision(), "Contracts025.kerml", &inputs::contracts(25))
        .unwrap();
    assert_valid(&s5);
    assert_eq!(s5.documents().count(), 100);
    baseline.push(immutable_signature(&s5));
    projections.push([0, 25, 49].map(|group| group_projection(&s5, group)));
    assert_ne!(
        s5.document_at("Contracts025.kerml").unwrap().id(),
        old_document
    );
    let repaired_provider = element(&s5, &["Contracts025", "RevisionValue"]);
    assert_ne!(repaired_provider, provider);
    assert!(s5.references().iter().any(|reference| {
        reference.name.segments == ["Contracts025", "RevisionValue"]
            && reference.resolution.value
                == agq_kerml_semantics::Resolution::Resolved(repaired_provider)
    }));
    let revisions = [s1, s2, s3, s4, s5];
    for consecutive in revisions.windows(2) {
        assert_eq!(consecutive[1].parent(), Some(consecutive[0].revision()));
    }
    let stable = element(&revisions[0], &["Workbench000", "Worker", "engine"]);
    for revision in &revisions {
        assert_shared(revision);
        assert!(std::ptr::eq(
            revisions[0].document_at("Worker000.sysml").unwrap(),
            revision.document_at("Worker000.sysml").unwrap()
        ));
        if revision.validate().is_ok() {
            assert_eq!(
                element(revision, &["Workbench000", "Worker", "engine"]),
                stable
            );
        }
    }
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let revisions = &revisions;
            let baseline = &baseline;
            let projections = &projections;
            scope.spawn(move || {
                for _ in 0..8 {
                    for (index, revision) in revisions.iter().enumerate() {
                        assert_eq!(immutable_signature(revision), baseline[index]);
                        assert_eq!(
                            [0, 25, 49].map(|group| group_projection(revision, group)),
                            projections[index]
                        );
                        if index == 3 {
                            assert_unresolved(revision, Some(provider));
                            continue;
                        }
                        let q = revision.sysml_queries().unwrap();
                        for group in [0, 25, 49] {
                            let owner = path(
                                q.kerml(),
                                revision.root(),
                                &[&format!("Workbench{group:03}"), "Worker"],
                            );
                            let children = q.effective_usages(owner);
                            assert_eq!(children.completeness(), Completeness::Complete);
                            let engine = path(
                                q.kerml(),
                                revision.root(),
                                &[&format!("Workbench{group:03}"), "Worker", "engine"],
                            );
                            assert!(children.value().contains(&engine));
                        }
                    }
                }
            });
        }
    });
    for revision in &revisions {
        let bytes: usize = revision
            .documents()
            .map(|(_, doc)| doc.source().len())
            .sum();
        let nodes: usize = revision
            .documents()
            .filter_map(|(_, doc)| doc.production_syntax())
            .map(|doc| doc.nodes().count())
            .sum();
        eprintln!(
            "revision={:?} documents={} source_bytes={bytes} syntax_nodes={nodes} certificate_bytes={:?} producer_counters={:?}",
            revision.revision(),
            revision.documents().count(),
            revision
                .producer_closure()
                .map(|certificate| certificate.storage_bytes()),
            revision.producer_status().map(|status| &status.counters)
        );
    }
    eprintln!(
        "workspace scaling elapsed={:?}; authored reconstruction remains permitted; no speed threshold",
        started.elapsed()
    );
}

// Retain only evidence anchored at the selected authored owner and engine.
// Accepted proof DAGs remain shared; readers never clone complete query answers
// or serialize their potentially large transitive explanations.
#[derive(Debug, PartialEq, Eq)]
struct ScalePopulationProof {
    context: SysmlSemanticContextId,
    values: Vec<ElementId>,
    facts: BTreeSet<FactKey>,
    canonical_dependencies: BTreeSet<Dependency>,
    origins: BTreeMap<FactKey, Arc<Origin>>,
    searches: BTreeSet<SearchDependency>,
}

fn scale_fact_subject(fact: FactKey) -> Option<ElementId> {
    match fact {
        FactKey::Element(element) | FactKey::Property { element, .. } => Some(element),
        FactKey::AssociationOccurrence(_) => None,
    }
}

fn retain_scale_evidence<T>(
    proof: &mut ScalePopulationProof,
    query: &QueryResult<T>,
    selected: &[ElementId; 2],
) {
    let relevant =
        |fact: FactKey| scale_fact_subject(fact).is_some_and(|subject| selected.contains(&subject));
    proof.facts.extend(
        query
            .positive_dependencies
            .iter()
            .copied()
            .filter(|&fact| relevant(fact)),
    );
    proof
        .canonical_dependencies
        .extend(query.canonical_dependencies.iter().copied().filter(
            |dependency| match dependency {
                Dependency::Declared(fact) | Dependency::Derived(fact) => relevant(*fact),
            },
        ));
    proof.origins.extend(
        query
            .fact_origins
            .iter()
            .filter(|(fact, _)| relevant(**fact))
            .map(|(fact, origin)| (*fact, Arc::clone(origin))),
    );
    proof.searches.extend(
        query
            .search_dependencies
            .iter()
            .filter(|search| {
                let subject = match search {
                    SearchDependency::ProducerClosure { subject, .. }
                    | SearchDependency::Element(subject) => *subject,
                    SearchDependency::PropertySet { element, .. } => *element,
                    SearchDependency::Incoming { target } => *target,
                    SearchDependency::SourceRelationships { source, .. } => *source,
                    SearchDependency::OwnedRelationships { owner, .. }
                    | SearchDependency::OwnedRelationshipsExcluding { owner, .. }
                    | SearchDependency::StructuralFeaturePopulation { owner, .. } => *owner,
                    SearchDependency::NamespaceMembers { namespace } => *namespace,
                    _ => return false,
                };
                selected.contains(&subject)
            })
            .cloned(),
    );
}

fn scale_population_proof(
    answer: SysmlQueryResult<Vec<ElementId>>,
    owner: ElementId,
    engine: ElementId,
) -> ScalePopulationProof {
    assert_eq!(answer.completeness(), Completeness::Complete);
    let mut proof = ScalePopulationProof {
        context: answer.context.clone(),
        values: answer.value().clone(),
        facts: BTreeSet::new(),
        canonical_dependencies: BTreeSet::new(),
        origins: BTreeMap::new(),
        searches: BTreeSet::new(),
    };
    let selected = [owner, engine];
    retain_scale_evidence(&mut proof, &answer.kerml, &selected);
    for query in &answer.supporting_queries {
        retain_scale_evidence(&mut proof, query, &selected);
    }
    for query in &answer.supporting_names {
        retain_scale_evidence(&mut proof, query, &selected);
    }
    for query in answer.observations.values() {
        retain_scale_evidence(&mut proof, query, &selected);
    }
    assert!(proof.facts.contains(&FactKey::Element(owner)));
    assert!(proof.origins.contains_key(&FactKey::Element(owner)));
    assert!(proof.searches.iter().any(|search| matches!(search,
        SearchDependency::ProducerClosure {
            subject,
            requirement: SemanticClosureRequirement::EffectiveMembership,
            certificate_digest: Some(_),
            ..
        } if *subject == owner)));
    assert!(proof.searches.iter().any(|search| matches!(search,
        SearchDependency::NamespaceMembers { namespace } if *namespace == owner)
        || matches!(search, SearchDependency::PropertySet { element, property }
            if *element == owner && *property == agq_kerml::properties::ELEMENT_OWNED_RELATIONSHIP)
        || matches!(search, SearchDependency::OwnedRelationships { owner: observed, .. }
            if *observed == owner)));
    proof
}

#[derive(Debug, PartialEq, Eq)]
struct ScaleGroupProof {
    owner: ElementId,
    engine: ElementId,
    usages: ScalePopulationProof,
    ports: ScalePopulationProof,
}

fn scale_group_proof(revision: &ValidatedProjectRevision, group: usize) -> ScaleGroupProof {
    let q = revision.sysml_queries();
    assert!(std::ptr::eq(q.model(), revision.semantic_model()));
    let package = format!("Workbench{group:03}");
    let root = revision.working().root();
    let owner = path(q.kerml(), root, &[&package, "Worker"]);
    let engine = path(q.kerml(), root, &[&package, "Worker", "engine"]);
    let usages = scale_population_proof(q.effective_usages(owner), owner, engine);
    assert!(usages.values.contains(&engine));
    let ports = scale_population_proof(q.effective_ports(owner), owner, engine);
    ScaleGroupProof {
        owner,
        engine,
        usages,
        ports,
    }
}

fn scale_revision_signature(revision: &WorkingProjectRevision) -> String {
    // The older recovery fixture's signature includes full reference proof
    // Debug output. Keep this larger fixture exact for its source/identity and
    // reference assertions without serializing standard proof DAGs.
    assert!(revision.diagnostics().is_empty());
    let mut signature = String::new();
    writeln!(
        signature,
        "revision={:?} parent={:?} context={:?} closure={:?}",
        revision.revision(),
        revision.parent(),
        revision.kerml_queries().unwrap().context(),
        revision.producer_closure().unwrap().digest()
    )
    .unwrap();
    for (path, document) in revision.documents() {
        writeln!(
            signature,
            "document={path:?} id={:?} revision={:?} language={:?} source={:?}",
            document.id(),
            document.revision(),
            document.language(),
            document.source()
        )
        .unwrap();
        for node in document.production_syntax().unwrap().nodes() {
            writeln!(signature, "node={:?} range={:?}", node.id(), node.range()).unwrap();
        }
    }
    for reference in revision.references() {
        assert_eq!(reference.resolution.completeness, Completeness::Complete);
        assert!(matches!(
            reference.resolution.value,
            agq_kerml_semantics::Resolution::Resolved(_)
        ));
        writeln!(
            signature,
            "reference={:?} specific={:?} kind={:?} name={:?} origin={:?} target={:?} alias={:?} visibility={:?}",
            reference.relationship,
            reference.specific,
            reference.kind,
            reference.name,
            reference.origin,
            reference.resolution.value,
            reference.alias(),
            reference.visibility()
        )
        .unwrap();
    }
    signature
}

fn capture_validated_scale_revision(
    revision: &Arc<WorkingProjectRevision>,
) -> (ValidatedProjectRevision, String, [ScaleGroupProof; 3]) {
    assert_valid(revision);
    assert_eq!(revision.documents().count(), 100);
    for language in [SourceLanguage::KerMl, SourceLanguage::SysMl] {
        assert_eq!(
            revision
                .documents()
                .filter(|(_, document)| document.language() == language)
                .count(),
            50
        );
    }
    let validated = revision.validate().unwrap();
    assert!(Arc::ptr_eq(validated.working(), revision));
    let signature = scale_revision_signature(revision);
    let queries = [0, 25, 49].map(|group| scale_group_proof(&validated, group));
    eprintln!(
        "validated scale revision={:?} documents=100 kerml=50 sysml=50 signature_bytes={} selected_query_fact_keys={} selected_query_searches={}",
        revision.revision(),
        signature.len(),
        queries
            .iter()
            .map(|proof| proof.usages.facts.len() + proof.ports.facts.len())
            .sum::<usize>(),
        queries
            .iter()
            .map(|proof| proof.usages.searches.len() + proof.ports.searches.len())
            .sum::<usize>()
    );
    (validated, signature, queries)
}

#[test]
#[ignore = "requires the accepted publication caches; never rebuilds standards"]
fn hundred_documents_five_validated_revisions_and_four_parallel_readers() {
    let started = Instant::now();
    let mut workspace = open();
    eprintln!(
        "validated scale: restoring accepted dependencies complete; constructing 100 mixed documents"
    );
    let mut changes = Vec::with_capacity(100);
    for group in 0..50 {
        changes.push(add(
            &format!("Contracts{group:03}.kerml"),
            SourceLanguage::KerMl,
            &inputs::contracts(group),
        ));
        changes.push(add(
            &format!("Worker{group:03}.sysml"),
            SourceLanguage::SysMl,
            &inputs::worker(group),
        ));
    }
    let first = workspace
        .apply(workspace.head().revision(), changes)
        .unwrap();
    // Each baseline and validated handle exists before the next source edit.
    let mut retained = vec![capture_validated_scale_revision(&first)];
    let original_engines = [25, 49].map(|group| {
        (
            element(
                &first,
                &[&format!("Workbench{group:03}"), "Worker", "engine"],
            ),
            syntax_id(
                &first,
                &format!("Worker{group:03}.sysml"),
                Production::PartUsage,
                "part engine;",
            ),
        )
    });
    for (index, group) in [25, 49].into_iter().enumerate() {
        let previous = retained.last().unwrap().0.working();
        let document_path = format!("Worker{group:03}.sysml");
        let source = previous.document_at(&document_path).unwrap().source();
        let insertion = source.find("part engine;").unwrap() + "part engine;".len();
        let with_port = workspace
            .apply(
                previous.revision(),
                [edit(
                    previous,
                    &document_path,
                    insertion,
                    insertion,
                    " port bus;",
                )],
            )
            .unwrap();
        retained.push(capture_validated_scale_revision(&with_port));
        let source = with_port.document_at(&document_path).unwrap().source();
        let insertion = source.rfind('}').unwrap();
        let specialized = workspace
            .apply(
                with_port.revision(),
                [edit(
                    &with_port,
                    &document_path,
                    insertion,
                    insertion,
                    "part def SpecializedWorker :> Worker { part :>> engine; } ",
                )],
            )
            .unwrap();
        retained.push(capture_validated_scale_revision(&specialized));
        let package = format!("Workbench{group:03}");
        for revision in [&with_port, &specialized] {
            assert_eq!(
                element(revision, &[&package, "Worker", "engine"]),
                original_engines[index].0
            );
            assert_eq!(
                syntax_id(
                    revision,
                    &document_path,
                    Production::PartUsage,
                    "part engine;"
                ),
                original_engines[index].1
            );
            assert_eq!(
                revision.document_at(&document_path).unwrap().id(),
                first.document_at(&document_path).unwrap().id()
            );
        }
        let q = specialized.sysml_queries().unwrap();
        let original_owner = element(&specialized, &[&package, "Worker"]);
        let owner = element(&specialized, &[&package, "SpecializedWorker"]);
        let bus = element(&with_port, &[&package, "Worker", "bus"]);
        let ports = q.effective_ports(owner);
        assert_eq!(ports.completeness(), Completeness::Complete);
        assert_eq!(ports.value(), &vec![bus]);
        let owning_type = q.kerml().owning_type(bus);
        assert_eq!(owning_type.completeness, Completeness::Complete);
        assert_eq!(owning_type.value, Some(original_owner));
        let replacement = element(&specialized, &[&package, "SpecializedWorker", "engine"]);
        let redefined = q.effective_redefined_features(replacement);
        assert_eq!(redefined.completeness(), Completeness::Complete);
        assert!(redefined.value().contains(&original_engines[index].0));
        let children = q.effective_usages(owner);
        assert_eq!(children.completeness(), Completeness::Complete);
        assert!(children.value().contains(&replacement));
        assert!(!children.value().contains(&original_engines[index].0));
        assert!(q.model().element(original_engines[index].0).is_some());
    }
    assert_eq!(retained.len(), 5);
    for consecutive in retained.windows(2) {
        assert_eq!(
            consecutive[1].0.working().parent(),
            Some(consecutive[0].0.revision())
        );
    }
    for (revision, signature, _) in &retained {
        let working = revision.working();
        assert!(
            scale_revision_signature(working) == *signature,
            "a retained validated revision changed"
        );
        assert!(Arc::ptr_eq(
            workspace.revision(revision.revision()).unwrap(),
            working
        ));
        assert!(std::ptr::eq(
            first.document_at("Worker000.sysml").unwrap(),
            working.document_at("Worker000.sysml").unwrap()
        ));
        assert_shared(working);
    }
    assert!(Arc::ptr_eq(
        workspace.head(),
        retained.last().unwrap().0.working()
    ));
    let barrier = Barrier::new(4);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let retained = &retained;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                for _ in 0..2 {
                    for (revision, signature, proofs) in retained {
                        assert!(
                            scale_revision_signature(revision.working()) == *signature,
                            "a retained validated revision changed"
                        );
                        for (group, proof) in [0, 25, 49].into_iter().zip(proofs) {
                            assert!(
                                scale_group_proof(revision, group) == *proof,
                                "query values, identities, context or bounded evidence changed"
                            );
                        }
                    }
                }
            });
        }
    });
    eprintln!(
        "validated scale: revisions={} documents_each=100 kerml_each=50 sysml_each=50 parallel_readers=4 read_passes=2 elapsed={:?}; physical dependency sharing and zero accepted producer replay asserted",
        retained.len(),
        started.elapsed()
    );
}
