use agq_workspace::*;
use std::collections::BTreeMap;
fn revision() -> Revision {
    Revision::import(
        BTreeMap::from([(
            "p.sysml".into(),
            "package P { part def Device; part a : Device; package Q {} part b : Device; }".into(),
        )]),
        &BTreeMap::new(),
    )
    .unwrap()
}
#[test]
fn standard_text_identity_sidecar_and_implicit_ids_roundtrip() {
    let r = Revision::import(
        BTreeMap::from([(
            "models/lifecycle.sysml".into(),
            include_str!("../../../models/AgentiqueBehaviour.sysml").into(),
        )]),
        &BTreeMap::new(),
    )
    .unwrap();
    let files = export_text(&r).unwrap();
    let restored = import_text(files.clone()).unwrap();
    assert_eq!(r.model.source_digest(), restored.model.source_digest());
    assert_eq!(
        r.model.elements.keys().collect::<Vec<_>>(),
        restored.model.elements.keys().collect::<Vec<_>>()
    );
    let mut changed = files;
    changed
        .get_mut("models/lifecycle.sysml")
        .unwrap()
        .push_str("\n// external edit");
    assert_eq!(
        import_text(changed).unwrap_err().code,
        "identity_reconciliation_required"
    );
    let candidate = propose(
        &r,
        vec![Edit::AddSource {
            file: "a.sysml".into(),
            source: "package Before { part x; }".into(),
        }],
    )
    .unwrap();
    for e in r.model.elements.values().filter(|e| !e.library) {
        assert!(
            candidate.model.elements.contains_key(&e.id),
            "Identity changed for {}",
            e.qualified_name
        );
    }
}
#[test]
fn at_mod01_rename_move_identity() {
    let r = revision();
    let id = r.model.by_path("P::Device").unwrap().id.clone();
    let a = r.model.by_path("P::a").unwrap().id.clone();
    let q = r.model.by_path("P::Q").unwrap().id.clone();
    let c = propose(
        &r,
        vec![
            Edit::Rename {
                element_id: id.clone(),
                name: "Motor".into(),
            },
            Edit::Move {
                element_id: id.clone(),
                new_owner_id: q,
            },
        ],
    )
    .unwrap();
    assert!(c.model.accepted(), "{:?}", c.model.diagnostics);
    let r2 = commit(&r, &c).unwrap();
    assert_eq!(r2.model.by_path("P::Q::Motor").unwrap().id, id);
    assert_eq!(r2.model.elements[&a].target("type"), Some(id.as_str()));
    let bytes = export_kpar(&r2).unwrap();
    let reloaded = import_kpar(&bytes).unwrap();
    assert_eq!(reloaded.model.by_path("P::Q::Motor").unwrap().id, id);
    assert_eq!(
        reloaded.model.elements[&a].target("type"),
        Some(id.as_str())
    );
}
#[test]
fn at_mod02_atomic_candidate_and_conflict() {
    let r = revision();
    let id = r.model.by_path("P::a").unwrap().id.clone();
    let c = propose(
        &r,
        vec![Edit::Rename {
            element_id: id.clone(),
            name: "b".into(),
        }],
    )
    .unwrap();
    assert!(!c.model.accepted());
    assert!(commit(&r, &c).is_err());
    assert_eq!(r.model.elements[&id].name.as_deref(), Some("a"));
    let good = propose(
        &r,
        vec![Edit::Rename {
            element_id: id,
            name: "newA".into(),
        }],
    )
    .unwrap();
    let r2 = commit(&r, &good).unwrap();
    assert_eq!(commit(&r2, &good).unwrap_err().code, "revision_conflict");
}
#[test]
fn at_mod03_text_and_kpar() {
    let r = Revision::import(
        BTreeMap::from([
            (
                "architecture.sysml".into(),
                include_str!("../../../models/AgentiqueArchitecture.sysml").into(),
            ),
            (
                "behaviour.sysml".into(),
                include_str!("../../../models/AgentiqueBehaviour.sysml").into(),
            ),
        ]),
        &BTreeMap::new(),
    )
    .unwrap();
    let imported = import_kpar(&export_kpar(&r).unwrap()).unwrap();
    assert_eq!(r.model.sources, imported.model.sources);
    assert_eq!(
        agq_semantics::identity_map(&r.model),
        agq_semantics::identity_map(&imported.model)
    );
}
#[test]
fn kerml_text_roundtrip() {
    let r = Revision::import(
        BTreeMap::from([(
            "Types.kerml".into(),
            include_str!("../../../tests/fixtures/Types.kerml").into(),
        )]),
        &BTreeMap::new(),
    )
    .unwrap();
    let imported = import_kpar(&export_kpar(&r).unwrap()).unwrap();
    assert_eq!(r.model.source_digest(), imported.model.source_digest());
}
#[test]
fn no_lossy_unsupported_overwrite() {
    let source = "package P { state def S parallel {state a;state b;} part def A; }";
    let r = Revision::import(
        BTreeMap::from([("p.sysml".into(), source.into())]),
        &BTreeMap::new(),
    )
    .unwrap();
    let c = propose(
        &r,
        vec![Edit::Rename {
            element_id: r.model.by_path("P::A").unwrap().id.clone(),
            name: "B".into(),
        }],
    )
    .unwrap();
    assert!(c.sources["p.sysml"].contains("state def S parallel {state a;state b;}"));
}
#[test]
fn ambiguous_external_edit_needs_reconciliation() {
    let r = revision();
    assert_eq!(
        propose(
            &r,
            vec![Edit::ReplaceSource {
                file: "p.sysml".into(),
                source: "package P {}".into(),
                identity_mapping: BTreeMap::new()
            }]
        )
        .unwrap_err()
        .code,
        "identity_reconciliation_required"
    );
}
#[test]
fn cycle_and_traversal_rejected() {
    let r = revision();
    let id = r.model.by_path("P").unwrap().id.clone();
    let q = r.model.by_path("P::Q").unwrap().id.clone();
    assert!(
        propose(
            &r,
            vec![Edit::Move {
                element_id: id,
                new_owner_id: q
            }]
        )
        .is_err()
    );
    assert!(!safe_source_name("../p.sysml"));
    assert!(!safe_source_name("C:/p.sysml"));
}
