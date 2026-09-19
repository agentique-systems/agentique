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
fn authored_redefinition_profile_matrix() {
    use agq_kerml::BaselineProfile as P;
    let source = "feature Quartz { feature segments; } feature Cobalt { feature signal; feature interval : Quartz :> Quartz::segments { feature capture redefines signal; } }";
    let mut contexts = std::collections::BTreeSet::new();
    for profile in [P::PublishedKerMl10, P::OPERATIONAL_V1, P::OPERATIONAL_V2] {
        let mut project = SourceProject::with_profile(profile).unwrap();
        let m = project
            .apply(project.current().revision(), [add("witness.kerml", source)])
            .unwrap();
        assert_eq!(project.baseline_profile(), profile);
        assert_eq!(m.queries().context().baseline_profile_id, profile.id());
        contexts.insert((
            m.queries().context().baseline_profile_id,
            m.queries().context().errata_manifest_digest,
        ));
        let r = m
            .references()
            .iter()
            .find(|r| r.kind == agq_kerml_text::syntax::ReferenceKind::Redefinition)
            .unwrap();
        if profile == P::OPERATIONAL_V2 {
            assert!(
                matches!(r.resolution.value, Resolution::Resolved(_)),
                "{:?}",
                r.resolution
            );
            assert!(
                r.resolution
                    .explanations
                    .values()
                    .flatten()
                    .any(|e| e.rule == agq_kerml_semantics::Rule::OperationalRedefinitionTargetV1)
            );
        } else {
            assert_eq!(r.resolution.value, Resolution::Unresolved);
        }
        println!("{}: {:?}", profile.id(), r.resolution.value);
    }
    assert_eq!(contexts.len(), 3);
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
        m.queries().context().baseline_profile_id,
        agq_kerml::BaselineProfile::OPERATIONAL.id()
    );
    assert!(m.queries().context().errata_manifest_digest.is_some());
    assert!(
        m.snapshot()
            .model()
            .registry()
            .property(agq_kerml::properties::A_PARTICIPANT_FEATURE_INTERACTION_PARTICIPANT_FEATURE)
            .is_err()
    );
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

#[test]
fn adding_a_document_invalidates_an_earlier_namespace_miss() {
    let mut project = SourceProject::new().unwrap();
    let old = project
        .apply(
            project.current().revision(),
            [add("use.kerml", "feature f : Missing;")],
        )
        .unwrap();
    let f = named(&project, "f");
    let miss = &old.references()[0];
    assert_eq!(miss.resolution.value, Resolution::Unresolved);
    assert!(
        miss.resolution
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers {
                namespace: old.root()
            })
    );
    assert!(
        old.semantic_diagnostics()
            .iter()
            .all(|d| d.domain == agq_kerml_text::FrontendDiagnosticDomain::Resolution)
    );
    let next = project
        .apply(
            old.revision(),
            [add("definition.kerml", "feature Missing;")],
        )
        .unwrap();
    assert_eq!(named(&project, "f"), f);
    assert_eq!(
        next.queries().direct_feature_types(f).value,
        [named(&project, "Missing")]
    );
    assert!(next.is_complete_slice());
    assert_ne!(
        miss.resolution.context,
        next.references()[0].resolution.context
    );
    assert_eq!(old.references()[0].resolution.value, Resolution::Unresolved);
}

#[test]
fn many_documents_and_specialization_chain_preserve_one_inherited_feature() {
    let mut project = SourceProject::new().unwrap();
    let mut inputs = vec![add("base.kerml", "feature Base { feature inherited; }")];
    for i in 0..48 {
        let parent = if i == 0 {
            "Base".to_string()
        } else {
            format!("T{}", i - 1)
        };
        inputs.push(add(
            &format!("{i:03}.kerml"),
            &format!("type T{i} :> {parent};"),
        ));
    }
    inputs.reverse();
    let m = project.apply(project.current().revision(), inputs).unwrap();
    assert!(m.is_complete_slice(), "{:?}", m.semantic_diagnostics());
    let base = named(&project, "Base");
    let feature = m.queries().lookup_declared_member(base, "inherited").value[0];
    let derived = m.queries().effective_features(named(&project, "T47"));
    assert_eq!(derived.completeness, Completeness::Complete);
    assert_eq!(derived.value, [feature]);
    assert_eq!(m.queries().owner(feature).value, Some(base));
    assert_eq!(
        m.snapshot()
            .model()
            .instances(agq_kerml::classes::FEATURE_MEMBERSHIP, false)
            .unwrap()
            .count(),
        1
    );
    let again = m.queries().effective_features(named(&project, "T47"));
    assert_eq!(derived, again);
}

#[test]
fn recovered_namespaces_block_guessed_denotation_without_poisoning_unrelated_scopes() {
    let mut project = SourceProject::new().unwrap();
    let m = project
        .apply(
            project.current().revision(),
            [
                add(
                    "incomplete.kerml",
                    "namespace Broken { feature X; class X; } feature f : Broken::X;",
                ),
                add(
                    "complete.kerml",
                    "namespace Good { feature Y; feature g : Y; }",
                ),
            ],
        )
        .unwrap();
    assert!(!m.is_complete_slice());
    let uncertain = m
        .references()
        .iter()
        .find(|r| r.name.segments == ["Broken", "X"])
        .unwrap();
    assert_eq!(uncertain.resolution.value, Resolution::Incomplete);
    assert!(
        m.snapshot()
            .model()
            .element(uncertain.relationship)
            .is_none()
    );
    let known = m
        .references()
        .iter()
        .find(|r| r.name.segments == ["Y"])
        .unwrap();
    assert!(matches!(known.resolution.value, Resolution::Resolved(_)));
    assert!(m.snapshot().model().element(known.relationship).is_some());
    assert_eq!(
        m.document_at("incomplete.kerml").unwrap().source(),
        "namespace Broken { feature X; class X; } feature f : Broken::X;"
    );
}

#[test]
fn recovered_type_body_keeps_effective_feature_answers_incomplete() {
    let mut project = SourceProject::new().unwrap();
    let m = project
        .apply(
            project.current().revision(),
            [add(
                "broken.kerml",
                "feature Base { feature known; class Unknown; } type Derived :> Base;",
            )],
        )
        .unwrap();
    let derived = named(&project, "Derived");
    let result = m.queries().effective_features(derived);
    assert_eq!(result.completeness, Completeness::Incomplete);
    assert_eq!(result.value.len(), 1);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_PENDING_INHERITANCE")
    );
}
