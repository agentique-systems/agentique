//! The root boundary and owned-name populations come from pinned KerML 1.0
//! Root-Elements-Element-deriveElementQualifiedName, not lexical path guessing.
use crate::*;
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{
    Completeness, EffectiveNames, SearchDependency, SemanticClosureRequirement, SemanticContext,
    SemanticOptions,
};
use agq_kernel::{
    ChangeSet, ElementId, MetaclassId, Snapshot,
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use agq_sysml::classes as sc;
use std::{collections::BTreeSet, sync::Arc};

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn create(base: &Snapshot, changes: &mut ChangeSet, n: u128, class: MetaclassId) {
    let origin = DeclaredOrigin::Authored { source: None };
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
fn fixture(root_name: Option<&str>, duplicate: bool, short_name: bool) -> Snapshot {
    fixture_with_owner(root_name, duplicate, short_name, sc::PART_DEFINITION)
}
fn fixture_with_owner(
    root_name: Option<&str>,
    duplicate: bool,
    short_name: bool,
    owner_class: MetaclassId,
) -> Snapshot {
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::Authored { source: None };
    for (n, class) in [
        (1, kc::NAMESPACE),
        (2, owner_class),
        (3, sc::PART_USAGE),
        (4, sc::PART_USAGE),
        (12, kc::OWNING_MEMBERSHIP),
        (23, kc::FEATURE_MEMBERSHIP),
        (24, kc::FEATURE_MEMBERSHIP),
    ] {
        create(&base, &mut changes, n, class);
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
    queries_with_sources(snapshot, pending, BTreeSet::new())
}
fn queries_with_sources(
    snapshot: &Snapshot,
    pending: BTreeSet<ElementId>,
    pending_types: BTreeSet<ElementId>,
) -> SysmlQueries<'_> {
    let mut context = crate::context::fixture_context(snapshot, BTreeSet::new());
    context.kerml = SemanticContext::for_project_snapshot(
        snapshot,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            exclude_implied: true,
        },
        BTreeSet::new(),
        pending_types,
        pending,
    )
    .unwrap();
    context.kerml = context
        .kerml
        .with_naming_extension(
            SYSML_SEMANTIC_CONTEXT_DOMAIN,
            context.id.dependencies.context_identity_digest(),
            Arc::new(SysmlNamingExtension),
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
        let answer = q.current_qualified_name(id(3));
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
        let root = q.current_qualified_name(id(1));
        assert_eq!(root.completeness(), Completeness::Complete);
        assert_eq!(root.value(), &None);
        assert_eq!(
            q.current_qualified_name(id(2)).value(),
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
        let answer = q.current_qualified_name(id);
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
        let answer = q.current_qualified_name(id(3));
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
    let answer = q.current_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Complete);
    assert_eq!(
        answer.value().as_ref().unwrap().segments[1],
        BTreeSet::from(["engine".into()])
    );
    let mut changes = snapshot.change_set();
    changes.clear(id(3), kp::ELEMENT_DECLARED_NAME);
    let short_only = snapshot.apply(&changes).unwrap();
    let answer = queries(&short_only, BTreeSet::new()).current_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert_eq!(answer.value(), &None);
}

fn with_inherited_sibling(base: &Snapshot, naming_target: u128, short: Option<&str>) -> Snapshot {
    with_inherited_sibling_class(base, naming_target, short, kc::FEATURE)
}
fn with_inherited_sibling_class(
    base: &Snapshot,
    naming_target: u128,
    short: Option<&str>,
    sibling_class: MetaclassId,
) -> Snapshot {
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::Authored { source: None };
    // The generated snapshot sibling that exposed this defect is a plain KerML
    // Feature. It must not be cast to a SysML Definition or Usage by the proof.
    for (n, class) in [
        (5, sibling_class),
        (25, kc::FEATURE_MEMBERSHIP),
        (54, kc::REDEFINITION),
    ] {
        create(base, &mut changes, n, class);
    }
    for (n, property, value) in [
        (
            2,
            kp::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![
                Value::Reference(id(23)),
                Value::Reference(id(24)),
                Value::Reference(id(25)),
            ]),
        ),
        (
            25,
            kp::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(5))]),
        ),
        (
            5,
            kp::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![Value::Reference(id(54))]),
        ),
        (
            54,
            kp::REDEFINITION_REDEFINING_FEATURE,
            SlotValue::Scalar(Value::Reference(id(5))),
        ),
        (
            54,
            kp::REDEFINITION_REDEFINED_FEATURE,
            SlotValue::Scalar(Value::Reference(id(naming_target))),
        ),
    ] {
        changes.set(id(n), property, value, origin.clone());
    }
    if let Some(short) = short {
        changes.set(
            id(5),
            kp::ELEMENT_DECLARED_SHORT_NAME,
            SlotValue::Scalar(Value::String(short.into())),
            origin,
        );
    }
    base.apply(&changes).unwrap()
}

#[test]
fn unrelated_inherited_sibling_names_have_a_bounded_negative_proof() {
    let base = fixture(None, false, false);
    let snapshot = with_inherited_sibling(&base, 4, None);
    let q = queries(&snapshot, BTreeSet::new());
    let answer = q.current_qualified_name(id(3));
    assert_eq!(
        answer.completeness(),
        Completeness::Complete,
        "{:?}",
        answer.diagnostics
    );
    assert_eq!(
        answer.value(),
        queries(&base, BTreeSet::new())
            .current_qualified_name(id(3))
            .value()
    );
    let names = answer
        .supporting_names
        .iter()
        .find(|result| {
            result.value == EffectiveNames::Determinate(BTreeSet::from(["wheel".into()]))
        })
        .expect("retain the actual sibling naming evidence");
    assert!(
        names
            .positive_dependencies
            .contains(&agq_kernel::provenance::FactKey::Property {
                element: id(54),
                property: kp::REDEFINITION_REDEFINED_FEATURE,
            })
    );
    assert!(
        names
            .search_dependencies
            .iter()
            .any(|search| matches!(search,
        SearchDependency::PropertySet { element, property }
            if *element == id(5) && *property == kp::ELEMENT_OWNED_RELATIONSHIP))
    );

    // The same current answer cannot manufacture producer closure. The sibling,
    // naming source and its owning search population remain explicit obligations.
    let effective = q.effective_qualified_name(id(3));
    assert_eq!(effective.completeness(), Completeness::Incomplete);
    for subject in [id(5), id(4)] {
        assert!(effective.supporting_queries.iter().any(|result|
            result.search_dependencies.iter().any(|search| matches!(search,
                SearchDependency::ProducerClosure { subject: observed, requirement: SemanticClosureRequirement::EffectiveNaming, certificate_digest: None, .. }
                    if *observed == subject))));
    }
    assert!(
        !effective
            .diagnostics
            .iter()
            .any(|d| d.code == "SQ_QUALIFIED_NAME_SELECTION")
    );
}

#[test]
fn overlapping_or_incomplete_sibling_names_do_not_prove_exclusion() {
    let base = fixture(None, false, false);
    let snapshot = with_inherited_sibling(&base, 3, None);
    let answer = queries(&snapshot, BTreeSet::new()).current_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert!(
        answer
            .diagnostics
            .iter()
            .any(|d| d.code == "SQ_QUALIFIED_NAME_SELECTION")
    );

    let mut changes = snapshot.change_set();
    changes.set(
        id(54),
        kp::REDEFINITION_REDEFINED_FEATURE,
        SlotValue::Scalar(Value::Reference(id(5))),
        DeclaredOrigin::Authored { source: None },
    );
    let incomplete = snapshot.apply(&changes).unwrap();
    let answer = queries(&incomplete, BTreeSet::new()).current_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert!(
        answer
            .supporting_names
            .iter()
            .any(|names| names.completeness == Completeness::Incomplete)
    );
}

#[test]
fn a_short_only_sibling_suppresses_its_inherited_full_name() {
    // Pinned Feature::effectiveName returns declaredName (null here) whenever
    // either declared name is present. The short name is not inherited engine.
    let snapshot = with_inherited_sibling(&fixture(None, false, false), 3, Some("w"));
    let q = queries(&snapshot, BTreeSet::new());
    let sibling = q.kerml().effective_names(id(5));
    assert_eq!(sibling.completeness, Completeness::Complete);
    assert_eq!(
        sibling.value,
        EffectiveNames::Determinate(BTreeSet::from(["w".into()]))
    );
    let answer = q.current_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Complete);
    // Asking for that sibling's own full qualified name remains unsupported.
    assert_eq!(
        q.current_qualified_name(id(5)).completeness(),
        Completeness::Incomplete
    );
}

#[test]
fn effective_sibling_exclusion_closes_type_ancestors_and_naming_search_owners() {
    let base = with_inherited_sibling_class(
        &fixture_with_owner(None, false, false, sc::TRANSITION_USAGE),
        4,
        None,
        sc::REFERENCE_USAGE,
    );
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::Authored { source: None };
    create(&base, &mut changes, 7, kc::CLASS);
    create(&base, &mut changes, 57, kc::FEATURE_TYPING);
    for (n, property, value) in [
        (
            5,
            kp::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![Value::Reference(id(54)), Value::Reference(id(57))]),
        ),
        (
            57,
            kp::FEATURE_TYPING_TYPED_FEATURE,
            SlotValue::Scalar(Value::Reference(id(5))),
        ),
        (
            57,
            kp::FEATURE_TYPING_TYPE,
            SlotValue::Scalar(Value::Reference(id(7))),
        ),
    ] {
        changes.set(id(n), property, value, origin.clone());
    }
    let snapshot = base.apply(&changes).unwrap();
    let q = queries_with_sources(&snapshot, BTreeSet::new(), BTreeSet::from([id(7)]));
    assert_eq!(
        q.current_qualified_name(id(3)).completeness(),
        Completeness::Complete
    );
    let answer = q.effective_qualified_name(id(3));
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    // 7 is a typed ancestor, not the selected naming source (4). The owning
    // transition's parameter population (2) is searched by the ReferenceUsage
    // naming override before it falls back to redefinition naming.
    for subject in [id(7), id(2)] {
        for requirement in SemanticClosureRequirement::ALL {
            assert!(answer.supporting_queries.iter().any(|result|
                result.search_dependencies.iter().any(|search| matches!(search,
                    SearchDependency::ProducerClosure { subject: observed, requirement: checked, certificate_digest: None, .. }
                        if *observed == subject && *checked == requirement))),
                "missing {subject:?} {requirement:?}");
        }
    }
    assert!(
        answer
            .supporting_names
            .iter()
            .any(|names| names
                .search_dependencies
                .contains(&SearchDependency::ValidationRule(
                    "ReferenceUsage::namingFeature"
                )))
    );
}
