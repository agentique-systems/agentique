use agq_kerml_semantics::{Completeness, QualifiedName};
use agq_kerml_text::{
    Document,
    syntax::{ByteRange, TextEdit},
};

#[test]
fn authored_namespace_import_alias_visibility_and_shadowing_use_canonical_queries() {
    let source = "namespace Library { feature Base; private feature Hidden; } namespace Project { public import Library::*; alias Renamed for Base; feature Value : Renamed; feature Base; feature Local : Base; }";
    let mut document = Document::new(source).unwrap();
    let first = document.current().clone();
    assert!(
        first.validate_slice().is_ok(),
        "{:?}; {:?}",
        first.syntax().diagnostics(),
        first.diagnostics()
    );
    let q = first.queries();
    let project = q.lookup_declared_member(first.root(), "Project").value[0];
    let local = q.lookup_declared_member(project, "Base").value[0];
    let renamed = q.lookup_path(
        project,
        &QualifiedName {
            absolute: false,
            segments: vec!["Renamed".into()],
        },
    );
    assert_eq!(renamed.completeness, Completeness::Complete);
    assert_eq!(
        renamed.value[0].element, local,
        "local declaration shadows imported Base even before its textual position"
    );
    let hidden = q.lookup_path(
        project,
        &QualifiedName {
            absolute: false,
            segments: vec!["Hidden".into()],
        },
    );
    assert!(hidden.value.is_empty());
    let old_links: Vec<_> = first
        .snapshot()
        .model()
        .association_occurrences()
        .map(|l| l.id())
        .collect();
    assert_eq!(q.member(renamed.value[0].membership).value, Some(local));
    let next = document
        .edit(TextEdit {
            range: ByteRange::new(0, 0).unwrap(),
            replacement: "// revision\n".into(),
        })
        .unwrap();
    assert!(next.validate_slice().is_ok(), "{:?}", next.diagnostics());
    assert_eq!(
        old_links,
        next.snapshot()
            .model()
            .association_occurrences()
            .map(|l| l.id())
            .collect::<Vec<_>>()
    );
    assert_ne!(first.snapshot().revision(), next.snapshot().revision());
}

#[test]
fn aliases_refer_to_imported_declarations_without_copying_them() {
    let document = Document::new("namespace Library { feature Base; } namespace Project { private import Library::*; alias Renamed for Base; feature Value : Renamed; }").unwrap();
    let model = document.current();
    assert!(model.validate_slice().is_ok(), "{:?}", model.diagnostics());
    let q = model.queries();
    let library = q.lookup_declared_member(model.root(), "Library").value[0];
    let base = q.lookup_declared_member(library, "Base").value[0];
    let project = q.lookup_declared_member(model.root(), "Project").value[0];
    let value = q.lookup_declared_member(project, "Value").value[0];
    assert!(q.feature_types(value).value.contains(&base));
    assert_eq!(q.owner(base).value, Some(library));
}

#[test]
fn unsupported_import_forms_keep_the_namespace_incomplete() {
    for source in [
        "import Library::*;",
        "public import all Library::*;",
        "public import Library::*::**;",
    ] {
        let document = Document::new(source).unwrap();
        assert!(document.current().validate_slice().is_err());
        assert!(document.current().references().is_empty());
    }
}

#[test]
fn unresolved_import_prevents_closed_namespace_answers() {
    let document =
        Document::new("namespace Project { private import Missing::*; feature Local; }").unwrap();
    let model = document.current();
    assert!(model.validate_slice().is_err());
    let q = model.queries();
    let project = q.lookup_declared_member(model.root(), "Project").value[0];
    assert_eq!(
        q.namespace_members(project, agq_kerml_semantics::MemberAccess::All)
            .completeness,
        Completeness::Incomplete
    );
}

#[test]
fn resolved_imports_recheck_earlier_lexical_candidates_before_canonical_publication() {
    let document = Document::new("feature X; namespace Library { feature X; } namespace Project { private import Library::*; feature value : X; }").unwrap();
    let model = document.current();
    assert!(model.validate_slice().is_ok(), "{:?}", model.diagnostics());
    let q = model.queries();
    let library = q.lookup_declared_member(model.root(), "Library").value[0];
    let expected = q.lookup_declared_member(library, "X").value[0];
    let project = q.lookup_declared_member(model.root(), "Project").value[0];
    let value = q.lookup_declared_member(project, "value").value[0];
    assert_eq!(q.direct_feature_types(value).value, vec![expected]);
}
