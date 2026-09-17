use agq_kerml::{classes as c, properties as p, views};
use agq_kerml_semantics::{Completeness, QualifiedName, Resolution, SearchDependency};
use agq_kerml_text::{Document, WorkingModel, syntax::*};
use agq_kernel::{provenance::*, *};
use std::collections::BTreeSet;

const VERTICAL: &str = include_str!("fixtures/vertical.kerml");
fn member(model: &WorkingModel, owner: ElementId, name: &str) -> ElementId {
    let result = model.queries().lookup_declared_member(owner, name);
    assert_eq!(result.completeness, Completeness::Complete);
    assert_eq!(result.value.len(), 1, "{name}: {result:?}");
    result.value[0]
}
fn named(model: &WorkingModel, name: &str) -> ElementId {
    member(model, model.root(), name)
}
fn edit_name(document: &mut Document, old: &str, new: &str) {
    let start = document.current().syntax().source().find(old).unwrap();
    document
        .edit(TextEdit {
            range: ByteRange::new(start as u64, (start + old.len()) as u64).unwrap(),
            replacement: new.into(),
        })
        .unwrap();
}
#[test]
fn vertical_slice_uses_normative_relationships_and_queries() {
    let doc = Document::new(VERTICAL).unwrap();
    let model = doc.current();
    assert_eq!(
        model.syntax().status(),
        SyntaxStatus::Success,
        "{:?}",
        model.syntax().diagnostics()
    );
    assert!(model.validate_slice().is_ok(), "{:?}", model.diagnostics());
    let demo = named(model, "Demo");
    let base = member(model, demo, "Base");
    let derived = member(model, demo, "Derived");
    let scalar = member(model, demo, "Scalar");
    let base_count = member(model, base, "count");
    let count = member(model, derived, "count");
    let extra = member(model, derived, "extra");
    let q = model.queries();
    assert_eq!(q.direct_specializations(derived).value, vec![base]);
    assert_eq!(q.direct_feature_types(count).value, vec![scalar]);
    assert_eq!(q.redefined_features(count).value, vec![base_count]);
    assert_eq!(q.subsetted_features(extra).value, vec![base_count]);
    assert_eq!(
        q.effective_features(derived)
            .value
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([count, extra])
    );
    assert!(
        views::Feature::try_new(extra, model.snapshot().model())
            .unwrap()
            .is_unique()
            .unwrap()
    );
    assert_eq!(
        model
            .snapshot()
            .model()
            .instances(c::FEATURE_MEMBERSHIP, false)
            .unwrap()
            .count(),
        3
    );
    assert_eq!(model.references().len(), 6);
    for r in model.references() {
        assert!(model.snapshot().model().element(r.relationship).is_some());
    }
}
#[test]
fn source_tokens_trivia_utf8_and_old_revisions_are_lossless() {
    let source = include_str!("fixtures/trivia.kerml").replace('\n', "\r\n");
    let mut doc = Document::new(&source).unwrap();
    let old = doc.current().clone();
    assert!(
        old.validate_slice().is_ok(),
        "{:?} {:?}",
        old.syntax().diagnostics(),
        old.diagnostics()
    );
    let restored: String = old
        .syntax()
        .tokens()
        .iter()
        .map(|t| old.syntax().token_text(t))
        .collect();
    assert_eq!(restored, source);
    let mut end = 0;
    for token in old.syntax().tokens() {
        assert_eq!(token.range.start(), end);
        end = token.range.end();
        assert!(source.is_char_boundary(end as usize));
    }
    assert_eq!(end as usize, source.len());
    let namespace = named(&old, "Mätning");
    let temperature = member(&old, namespace, "温度");
    let record = old.snapshot().model().element(temperature).unwrap();
    let Origin::Declared(DeclaredOrigin::Authored {
        source: Some(origin),
    }) = record.slot(p::ELEMENT_DECLARED_NAME).unwrap().origin()
    else {
        panic!("source provenance")
    };
    assert_eq!(old.syntax().text(origin.range), Some("'温度'"));
    assert_eq!(origin.revision, old.syntax().revision());
    edit_name(&mut doc, "'温度'", "'温度計'");
    assert_eq!(member(doc.current(), namespace, "温度計"), temperature);
    assert_eq!(
        doc.source(old.syntax().revision()).unwrap().source(),
        source
    );
    assert_eq!(old.syntax().text(origin.range), Some("'温度'"));
    for element in doc.current().snapshot().model().elements() {
        for provenance in
            std::iter::once(element.origin()).chain(element.slots().map(|(_, s)| s.origin()))
        {
            let Origin::Declared(DeclaredOrigin::Authored {
                source: Some(origin),
            }) = provenance
            else {
                panic!("every canonical fact has source provenance")
            };
            assert_eq!(origin.revision, doc.current().syntax().revision());
            assert!(
                doc.source(origin.revision)
                    .unwrap()
                    .text(origin.range)
                    .is_some()
            );
        }
    }
}
#[test]
fn malformed_and_incomplete_text_never_invents_semantics() {
    let editing = Document::new(include_str!("fixtures/editing.kerml")).unwrap();
    assert_eq!(editing.current().snapshot().model().len(), 1);
    assert!(editing.current().validate_slice().is_err());
    let doc = Document::new(include_str!("fixtures/recovery.kerml")).unwrap();
    let model = doc.current();
    assert_eq!(model.syntax().status(), SyntaxStatus::Recovered);
    assert!(model.validate_slice().is_err());
    let draft = named(model, "Draft");
    for name in ["good", "after", "unfinished"] {
        member(model, draft, name);
    }
    for name in ["missing", "NotACompleteType", "hidden", "value"] {
        assert!(
            model
                .queries()
                .lookup_declared_member(draft, name)
                .value
                .is_empty(),
            "{name}"
        );
    }
    for source in [
        "part def Engi",
        "feature",
        "feature x typed ;",
        "feature x : Missing::;",
        "namespace N { feature x;",
    ] {
        let doc = Document::new(source).unwrap();
        assert_eq!(doc.current().syntax().source(), source);
        assert_eq!(doc.current().syntax().status(), SyntaxStatus::Recovered);
        assert!(doc.current().validate_slice().is_err());
    }
    assert_eq!(
        Document::new("part def Engi")
            .unwrap()
            .current()
            .snapshot()
            .model()
            .len(),
        1
    );
    assert_eq!(
        Document::new("feature x typed ;")
            .unwrap()
            .current()
            .snapshot()
            .model()
            .len(),
        1
    );
}
#[test]
fn unresolved_ambiguous_wrong_kind_are_explicit_and_never_kernel_targets() {
    let doc = Document::new(include_str!("fixtures/references.kerml")).unwrap();
    let model = doc.current();
    assert_eq!(model.syntax().status(), SyntaxStatus::Success);
    assert!(model.validate_slice().is_err());
    assert!(
        model
            .references()
            .iter()
            .any(|r| matches!(r.resolution.value, Resolution::Unresolved))
    );
    assert!(
        model
            .references()
            .iter()
            .any(|r| matches!(&r.resolution.value, Resolution::Ambiguous(ids) if ids.len() == 2))
    );
    assert!(
        model
            .references()
            .iter()
            .any(|r| matches!(r.resolution.value, Resolution::WrongKind(_)))
    );
    for code in [
        "KQ_UNRESOLVED",
        "KQ_AMBIGUOUS",
        "KQ_REFERENCE_KIND",
        "KT_DUPLICATE_NAME",
    ] {
        assert!(model.diagnostics().iter().any(|d| d.code == code));
    }
    assert_eq!(
        model
            .snapshot()
            .model()
            .instances(c::FEATURE_TYPING, false)
            .unwrap()
            .count(),
        0
    );
    assert!(
        model
            .references()
            .iter()
            .all(|r| !r.resolution.search_dependencies.is_empty())
    );
}
#[test]
fn rename_and_formatting_preserve_ids_but_replacement_and_recreation_do_not() {
    let mut doc = Document::new("namespace N { feature engine; feature wheel; }").unwrap();
    let namespace = named(doc.current(), "N");
    let engine = member(doc.current(), namespace, "engine");
    let wheel = member(doc.current(), namespace, "wheel");
    edit_name(&mut doc, "engine", "motor");
    assert_eq!(member(doc.current(), namespace, "motor"), engine);
    let formatted = "// note\nnamespace N\n{\n feature motor ;\n\t feature wheel;\n}\n";
    let end = doc.current().syntax().source().len();
    doc.edit(TextEdit {
        range: ByteRange::new(0, end as u64).unwrap(),
        replacement: formatted.into(),
    })
    .unwrap();
    assert_eq!(member(doc.current(), namespace, "motor"), engine);
    assert_eq!(member(doc.current(), namespace, "wheel"), wheel);
    edit_name(&mut doc, "feature motor ;", "");
    edit_name(&mut doc, "feature wheel;", "feature motor; feature wheel;");
    assert_ne!(member(doc.current(), namespace, "motor"), engine);
    let old_ids: BTreeSet<_> = doc
        .current()
        .snapshot()
        .model()
        .elements()
        .map(|e| e.id())
        .collect();
    let source = doc.current().syntax().source().to_owned();
    doc.replace(&source).unwrap();
    assert!(
        doc.current()
            .snapshot()
            .model()
            .elements()
            .all(|e| !old_ids.contains(&e.id()))
    );
}
#[test]
fn structural_replacements_and_ambiguous_duplicates_are_not_reconciled() {
    let mut doc = Document::new("feature one;").unwrap();
    let one = named(doc.current(), "one");
    edit_name(&mut doc, "feature one;", "feature two;");
    assert_ne!(named(doc.current(), "two"), one);
    let mut duplicates = Document::new("feature x; feature x;").unwrap();
    let ids: BTreeSet<_> = duplicates
        .current()
        .queries()
        .lookup_declared_member(duplicates.current().root(), "x")
        .value
        .into_iter()
        .collect();
    let len = duplicates.current().syntax().source().len();
    duplicates
        .edit(TextEdit {
            range: ByteRange::new(0, len as u64).unwrap(),
            replacement: "feature x;\nfeature x;".into(),
        })
        .unwrap();
    assert!(
        duplicates
            .current()
            .queries()
            .lookup_declared_member(duplicates.current().root(), "x")
            .value
            .iter()
            .all(|id| !ids.contains(id))
    );
}
#[test]
fn declared_resolution_has_lexical_shadowing_and_reports_inheritance_gaps() {
    let doc = Document::new("feature X; namespace N { feature X; feature f : X; }").unwrap();
    let model = doc.current();
    assert!(model.validate_slice().is_ok());
    let n = named(model, "N");
    let f = member(model, n, "f");
    let x = member(model, n, "X");
    assert_eq!(model.queries().direct_feature_types(f).value, vec![x]);
    let r = model.queries().resolve_reference(
        f,
        &QualifiedName {
            absolute: false,
            segments: vec!["Unknown".into()],
        },
        c::TYPE,
    );
    assert!(
        r.search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: n })
    );
    assert!(
        r.search_dependencies
            .contains(&SearchDependency::NamespaceMembers {
                namespace: model.root()
            })
    );
    let doc =
        Document::new("feature Base { feature X; } feature X; type T :> Base { feature f : X; }")
            .unwrap();
    assert!(
        doc.current()
            .references()
            .iter()
            .any(|r| r.resolution.value == Resolution::Incomplete)
    );
    let t = named(doc.current(), "T");
    let f = member(doc.current(), t, "f");
    assert!(
        doc.current()
            .queries()
            .direct_feature_types(f)
            .value
            .is_empty()
    );
    assert!(doc.current().validate_slice().is_err());
}
#[test]
fn regular_comments_are_preserved_but_not_mistaken_for_notes() {
    let source = "/* a modeled comment */ feature x;";
    let doc = Document::new(source).unwrap();
    assert_eq!(doc.current().syntax().tokens()[0].kind, TokenKind::Comment);
    assert_eq!(doc.current().syntax().source(), source);
    named(doc.current(), "x");
    assert!(doc.current().validate_slice().is_err());
}

#[test]
fn opaque_unsupported_constructs_cannot_inject_declarations_during_recovery() {
    for source in [
        "feature bad = \"; feature injected;\"; feature good;",
        "part def Fake { feature injected; } feature good;",
        "private feature injected; feature good;",
        "import N { feature injected; } feature good;",
    ] {
        let doc = Document::new(source).unwrap();
        assert!(doc.current().validate_slice().is_err());
        assert!(
            doc.current()
                .queries()
                .lookup_declared_member(doc.current().root(), "injected")
                .value
                .is_empty()
        );
        named(doc.current(), "good");
    }
}

#[test]
fn unresolved_specialization_obligations_block_inherited_name_lookup() {
    let doc = Document::new("feature X; type T :> Missing { feature f : X; }").unwrap();
    let model = doc.current();
    let t = named(model, "T");
    let f = member(model, t, "f");
    assert!(model.queries().direct_feature_types(f).value.is_empty());
    assert_eq!(
        model.references()[1].resolution.value,
        Resolution::Incomplete
    );
    for assertion in model.references() {
        assert_eq!(
            assertion.resolution.context.revision,
            model.snapshot().revision()
        );
        assert!(
            assertion
                .resolution
                .context
                .pending_specialization_scopes
                .contains(&t)
        );
    }
}

#[test]
fn membership_and_relationship_identity_survive_rename_and_formatting() {
    let mut doc = Document::new("feature Base; feature x : Base;").unwrap();
    let x = named(doc.current(), "x");
    let membership = doc
        .current()
        .queries()
        .owning_relationship(x)
        .value
        .unwrap();
    let typing = doc.current().references()[0].relationship;
    edit_name(&mut doc, "x", "renamed");
    assert_eq!(named(doc.current(), "renamed"), x);
    assert_eq!(
        doc.current().queries().owning_relationship(x).value,
        Some(membership)
    );
    assert_eq!(doc.current().references()[0].relationship, typing);
    let before = doc.current().snapshot().clone();
    let len = doc.current().syntax().source().len();
    doc.edit(TextEdit {
        range: ByteRange::new(0, len as u64).unwrap(),
        replacement: "feature Base;\n// a note\nfeature renamed : Base ;".into(),
    })
    .unwrap();
    assert_eq!(doc.current().references()[0].relationship, typing);
    assert_ne!(
        before.model().element(x).unwrap().origin(),
        doc.current()
            .snapshot()
            .model()
            .element(x)
            .unwrap()
            .origin()
    );
    // Retargeting changes the declaration header and has no continuity claim.
    edit_name(&mut doc, ": Base", ": Unknown");
    assert_ne!(doc.current().references()[0].relationship, typing);
    assert!(doc.current().snapshot().model().element(typing).is_none());
    assert!(before.model().element(typing).is_some());
}
#[test]
fn limits_invalid_utf8_edits_and_atomic_history() {
    assert!(
        Document::with_limits(
            "feature x;",
            ParseLimits {
                max_bytes: 2,
                ..ParseLimits::default()
            }
        )
        .is_err()
    );
    assert!(
        Document::with_limits(
            "feature x;",
            ParseLimits {
                max_tokens: 1,
                ..ParseLimits::default()
            }
        )
        .is_err()
    );
    let doc = Document::with_limits(
        "namespace A { namespace B { feature x; } }",
        ParseLimits {
            max_depth: 1,
            ..ParseLimits::default()
        },
    )
    .unwrap();
    assert!(
        doc.current()
            .syntax()
            .diagnostics()
            .iter()
            .any(|d| d.code == "KS_DEPTH")
    );
    let mut doc = Document::new("feature 'λ';").unwrap();
    let revision = doc.current().syntax().revision();
    let inside = doc.current().syntax().source().find('λ').unwrap() + 1;
    assert!(
        doc.edit(TextEdit {
            range: ByteRange::new(inside as u64, inside as u64).unwrap(),
            replacement: "x".into()
        })
        .is_err()
    );
    assert_eq!(doc.current().syntax().revision(), revision);
}
