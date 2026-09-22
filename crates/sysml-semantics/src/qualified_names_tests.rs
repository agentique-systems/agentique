//! The root boundary and owned-name populations come from pinned KerML 1.0
//! Root-Elements-Element-deriveElementQualifiedName, not lexical path guessing.
use crate::*;
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, SemanticContext, SemanticOptions};
use agq_kernel::{
    ElementId, Snapshot,
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use agq_sysml::classes as sc;
use std::{collections::BTreeSet, sync::Arc};

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn fixture(root_name: Option<&str>, duplicate: bool, short_name: bool) -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::Authored { source: None };
    for (n, class) in [
        (1, kc::NAMESPACE),
        (2, sc::PART_DEFINITION),
        (3, sc::PART_USAGE),
        (4, sc::PART_USAGE),
        (12, kc::OWNING_MEMBERSHIP),
        (23, kc::FEATURE_MEMBERSHIP),
        (24, kc::FEATURE_MEMBERSHIP),
    ] {
        changes.create(id(n), class, origin.clone());
        for property in base.model().registry().effective_properties(class).unwrap() {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match base
                .model()
                .registry()
                .storage_kind(property.value_kind)
                .unwrap()
            {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(n.to_string()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *base
                        .model()
                        .registry()
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                other => panic!("fixture {other:?}"),
            };
            changes.set(id(n), property.id, SlotValue::Scalar(value), origin.clone());
        }
    }
    for (n, name) in [
        (1, root_name),
        (2, Some("Vehicle")),
        (3, Some("engine")),
        (4, Some(if duplicate { "engine" } else { "wheel" })),
    ] {
        if let Some(name) = name {
            changes.set(
                id(n),
                kp::ELEMENT_DECLARED_NAME,
                SlotValue::Scalar(Value::String(name.into())),
                origin.clone(),
            );
        }
    }
    if short_name {
        changes.set(
            id(3),
            kp::ELEMENT_DECLARED_SHORT_NAME,
            SlotValue::Scalar(Value::String("e".into())),
            origin.clone(),
        );
    }
    for (owner, relationships) in [(1, vec![12]), (2, vec![23, 24])] {
        changes.set(
            id(owner),
            kp::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(
                relationships
                    .into_iter()
                    .map(|n| Value::Reference(id(n)))
                    .collect(),
            ),
            origin.clone(),
        );
    }
    for (membership, member) in [(12, 2), (23, 3), (24, 4)] {
        changes.set(
            id(membership),
            kp::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(member))]),
            origin.clone(),
        );
    }
    base.apply(&changes).unwrap()
}
fn queries(snapshot: &Snapshot, pending: BTreeSet<ElementId>) -> SysmlQueries<'_> {
    let mut context = crate::context::fixture_context(snapshot, BTreeSet::new());
    context.kerml = SemanticContext::for_project_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            exclude_implied: true,
        },
        BTreeSet::new(),
        BTreeSet::new(),
        pending,
    )
    .unwrap();
    context.id.kerml = context.kerml.id().clone();
    SysmlQueries::new(context)
}

#[test]
fn qualified_name_excludes_named_and_unnamed_root_namespace() {
    for root_name in [None, Some("NeverAPrefix")] {
        let snapshot = fixture(root_name, false, false);
        let q = queries(&snapshot, BTreeSet::new());
        let answer = q.effective_qualified_name(id(3));
        assert_eq!(answer.completeness(), Completeness::Complete, "{answer:?}");
        assert_eq!(
            answer.value(),
            &Some(QualifiedNamePath {
                segments: vec![
                    BTreeSet::from(["Vehicle".into()]),
                    BTreeSet::from(["engine".into()])
                ]
            })
        );
        let root = q.effective_qualified_name(id(1));
        assert_eq!(root.completeness(), Completeness::Complete);
        assert_eq!(root.value(), &None);
        assert_eq!(
            q.effective_qualified_name(id(2)).value(),
            &Some(QualifiedNamePath {
                segments: vec![BTreeSet::from(["Vehicle".into()])]
            })
        );
    }
}

#[test]
fn qualified_name_does_not_choose_duplicate_siblings_or_incomplete_population() {
    let duplicate = fixture(None, true, false);
    let q = queries(&duplicate, BTreeSet::new());
    for id in [id(3), id(4)] {
        let answer = q.effective_qualified_name(id);
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        assert_eq!(answer.value(), &None);
        assert!(
            answer
                .diagnostics
                .iter()
                .any(|d| d.code == "SQ_QUALIFIED_NAME_COLLISION")
        );
    }
    let ordinary = fixture(None, false, false);
    for scope in [id(1), id(2)] {
        let q = queries(&ordinary, BTreeSet::from([scope]));
        let answer = q.effective_qualified_name(id(3));
        assert_eq!(answer.completeness(), Completeness::Incomplete);
        assert_eq!(answer.value(), &None);
        assert!(
            answer
                .diagnostics
                .iter()
                .any(|d| d.code == "SQ_PENDING_QUALIFIED_NAME_POPULATION")
        );
    }
}

#[test]
fn qualified_name_uses_full_name_without_promoting_short_name() {
    let snapshot = fixture(None, false, true);
    let q = queries(&snapshot, BTreeSet::new());
    let answer = q.effective_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Complete);
    assert_eq!(
        answer.value().as_ref().unwrap().segments[1],
        BTreeSet::from(["engine".into()])
    );
    let mut changes = snapshot.change_set();
    changes.clear(id(3), kp::ELEMENT_DECLARED_NAME);
    let short_only = snapshot.apply(&changes).unwrap();
    let answer = queries(&short_only, BTreeSet::new()).effective_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert_eq!(answer.value(), &None);
}
