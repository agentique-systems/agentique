use agq_kerml_semantics::{Completeness, Resolution, SearchDependency};
use agq_kerml_text::{
    DocumentStatus, ProjectChange as Change, SourceLanguage as Language, SourceProject,
    syntax::{ByteRange, ParseLimits, TextEdit},
};
use agq_kernel::{
    ElementId,
    provenance::{DeclaredOrigin, Origin},
};
use std::sync::Arc;

fn add(path: &str, source: &str) -> Change {
    Change::Add {
        path: path.into(),
        language: Language::KerMl,
        source: source.into(),
    }
}
fn named(project: &SourceProject, name: &str) -> ElementId {
    let m = project.current();
    let result = m.queries().lookup_declared_member(m.root(), name);
    assert_eq!(result.completeness, Completeness::Complete);
    assert_eq!(result.value.len(), 1, "{name}: {result:?}");
    result.value[0]
}

#[test]
fn two_documents_share_one_root_and_resolve_in_both_directions() {
    let mut project = SourceProject::new().unwrap();
    let m = project
        .apply(
            project.current().revision(),
            [
                add("b.kerml", "feature B; feature b : A;"),
                add("a.kerml", "feature A; feature a : B;"),
            ],
        )
        .unwrap();
    assert!(m.is_complete_slice(), "{:?}", m.semantic_diagnostics());
    assert_eq!(
        m.documents().map(|(p, _)| p).collect::<Vec<_>>(),
        ["a.kerml", "b.kerml"]
    );
    let (a, b) = (named(&project, "A"), named(&project, "B"));
    assert_eq!(
        m.queries().direct_feature_types(named(&project, "a")).value,
        [b]
    );
    assert_eq!(
        m.queries().direct_feature_types(named(&project, "b")).value,
        [a]
    );
    for r in m.references() {
        assert_eq!(r.resolution.context.revision, m.revision());
        assert!(matches!(r.resolution.value, Resolution::Resolved(_)));
        assert!(m.snapshot().model().element(r.relationship).is_some());
        let doc = m.document(r.origin.document).unwrap();
        assert_eq!(doc.revision(), r.origin.revision);
        assert!(doc.syntax().unwrap().text(r.origin.range).is_some());
    }
}

#[test]
fn edit_preserves_other_document_and_old_revision_and_rebuilds_dependents() {
    let mut project = SourceProject::new().unwrap();
    let old = project
        .apply(
            project.current().revision(),
            [
                add("a.kerml", "feature A;"),
                add("b.kerml", "feature b : A;"),
            ],
        )
        .unwrap();
    let a = named(&project, "A");
    let b = named(&project, "b");
    let b_doc = old.document_at("b.kerml").unwrap();
    let before_record = old.snapshot().model().element(b).unwrap().clone();
    let next = project
        .apply(
            old.revision(),
            [Change::Edit {
                document: old.document_at("a.kerml").unwrap().id(),
                edit: TextEdit {
                    range: ByteRange::new(8, 9).unwrap(),
                    replacement: "Renamed".into(),
                },
            }],
        )
        .unwrap();
    assert_eq!(named(&project, "Renamed"), a);
    assert_eq!(named(&project, "b"), b);
    assert_eq!(
        next.document_at("b.kerml").unwrap().revision(),
        b_doc.revision()
    );
    // The dependent typing disappears; the unchanged declaration and membership survive.
    assert!(next.queries().direct_feature_types(b).value.is_empty());
    assert_eq!(
        next.references()[0].resolution.value,
        Resolution::Unresolved
    );
    assert_eq!(old.queries().direct_feature_types(b).value, [a]);
    assert_eq!(old.snapshot().model().element(b).unwrap(), &before_record);
    assert!(Arc::ptr_eq(project.revision(old.revision()).unwrap(), &old));
}

#[test]
fn deletion_recreation_and_cross_document_move_allocate_new_ids() {
    let mut project = SourceProject::new().unwrap();
    let old = project
        .apply(project.current().revision(), [add("a.kerml", "feature A;")])
        .unwrap();
    let a = named(&project, "A");
    let doc = old.document_at("a.kerml").unwrap().id();
    project
        .apply(
            old.revision(),
            [
                Change::Remove { document: doc },
                add("b.kerml", "feature A;"),
            ],
        )
        .unwrap();
    assert_ne!(a, named(&project, "A"));
    let moved = named(&project, "A");
    let doc = project.current().document_at("b.kerml").unwrap().id();
    project
        .apply(
            project.current().revision(),
            [Change::Remove { document: doc }],
        )
        .unwrap();
    project
        .apply(project.current().revision(), [add("b.kerml", "feature A;")])
        .unwrap();
    assert_ne!(moved, named(&project, "A"));
    assert_ne!(doc, project.current().document_at("b.kerml").unwrap().id());
}

#[test]
fn failed_batch_publishes_no_document_or_semantic_revision() {
    let mut project = SourceProject::with_limits(ParseLimits {
        max_bytes: 100,
        ..ParseLimits::default()
    })
    .unwrap();
    let old = project
        .apply(project.current().revision(), [add("a.kerml", "feature A;")])
        .unwrap();
    assert!(
        project
            .apply(
                old.revision(),
                [
                    add("b.kerml", "feature B;"),
                    add("huge.kerml", &" ".repeat(101))
                ]
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&old, project.current()));
    assert!(project.current().document_at("b.kerml").is_none());
    assert!(
        project
            .apply(old.revision(), [add("a.kerml", "feature Duplicate;")])
            .is_err()
    );
    assert!(Arc::ptr_eq(&old, project.current()));
    project
        .apply(old.revision(), [add("b.kerml", "feature B;")])
        .unwrap();
    assert!(project.apply(old.revision(), []).is_err());
}

#[test]
fn mixed_project_retains_sysml_and_marks_missing_frontend_evidence_incomplete() {
    let mut project = SourceProject::new().unwrap();
    let m = project
        .apply(
            project.current().revision(),
            [
                add("common.kerml", "feature f : Engine;"),
                Change::Add {
                    path: "engine.sysml".into(),
                    language: Language::SysMl,
                    source: "part def Engine;".into(),
                },
                Change::Add {
                    path: "vehicle.sysml".into(),
                    language: Language::SysMl,
                    source: "part def Vehicle;".into(),
                },
            ],
        )
        .unwrap();
    assert_eq!(
        m.document_at("engine.sysml").unwrap().status(),
        DocumentStatus::FrontendUnavailable
    );
    assert_eq!(
        m.document_at("engine.sysml").unwrap().source(),
        "part def Engine;"
    );
    assert_eq!(m.diagnostics().len(), 2);
    assert!(!m.is_complete_slice());
    let r = &m.references()[0];
    assert_eq!(r.resolution.value, Resolution::Incomplete);
    assert_eq!(r.resolution.completeness, Completeness::Incomplete);
    assert!(
        r.resolution
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers {
                namespace: m.root()
            })
    );
    assert!(m.snapshot().model().element(r.relationship).is_none());
    assert_eq!(
        r.resolution
            .context
            .pending_namespace_scopes
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [m.root()]
    );
}

#[test]
fn unchanged_document_origin_and_identity_survive_format_and_path_edits() {
    let mut project = SourceProject::new().unwrap();
    let old = project
        .apply(
            project.current().revision(),
            [add("a.kerml", "feature A;"), add("b.kerml", "feature B;")],
        )
        .unwrap();
    let b = named(&project, "B");
    let old_b = old.snapshot().model().element(b).unwrap().clone();
    let doc = old.document_at("a.kerml").unwrap().id();
    let a = named(&project, "A");
    project
        .apply(
            old.revision(),
            [
                Change::Edit {
                    document: doc,
                    edit: TextEdit {
                        range: ByteRange::new(0, 0).unwrap(),
                        replacement: "\n".into(),
                    },
                },
                Change::RenameDocument {
                    document: doc,
                    path: "renamed.kerml".into(),
                },
            ],
        )
        .unwrap();
    assert_eq!(named(&project, "A"), a);
    assert_eq!(
        project.current().snapshot().model().element(b).unwrap(),
        &old_b
    );
    let Origin::Declared(DeclaredOrigin::Authored {
        source: Some(origin),
    }) = old_b.origin()
    else {
        panic!("source provenance")
    };
    assert_eq!(origin.document, old.document_at("b.kerml").unwrap().id());
    assert!(matches!(
        project
            .current()
            .snapshot()
            .model()
            .element(project.current().root())
            .unwrap()
            .origin(),
        Origin::Declared(DeclaredOrigin::Generated { .. })
    ));
}
