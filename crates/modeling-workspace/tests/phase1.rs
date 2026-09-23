//! Executable acceptance specification, held until the production crate and
//! Working frontend carrier exist. See README for exact compile blockers.
mod support;
use agq_kerml_semantics::{Completeness, QualifiedName};
use agq_kerml_syntax::production::Production;
use agq_kerml_text::{DocumentStatus, ProjectChange, SourceLanguage};
use agq_kernel::DocumentId;
use std::{collections::BTreeSet, sync::Arc, time::Instant};
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
    let inherited = s3
        .sysml_queries()
        .unwrap()
        .effective_ports(element(&s3, &["Workbench025", "SpecializedWorker"]));
    assert_eq!(inherited.completeness(), Completeness::Complete);
    assert!(inherited.value().contains(&bus));
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
