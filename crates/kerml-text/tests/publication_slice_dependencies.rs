#[path = "../examples/support/publication_dependencies.rs"]
mod publication_dependencies;

use agq_kerml::{BaselineProfile, classes as c, properties as p};
use agq_kerml_semantics::*;
use agq_kernel::{
    ChangeSet, ElementId, MetaclassId, Snapshot,
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
};
use publication_dependencies::{
    Boundary, expanded_subjects, subjects, subjects_with_context_anchors,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn origin() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
struct Fixture {
    base: Snapshot,
    changes: ChangeSet,
    owned: BTreeMap<ElementId, Vec<Value>>,
}
impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V8).unwrap(),
        ));
        Self {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
        }
    }
    fn create(&mut self, n: u128, class: MetaclassId) {
        self.changes.create(id(n), class, origin());
        let registry = self.base.model().registry();
        for property in registry.effective_properties(class).unwrap() {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match registry.storage_kind(property.value_kind).unwrap() {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(n.to_string()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *registry
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                other => panic!("unexpected fixture primitive {other:?}"),
            };
            self.changes
                .set(id(n), property.id, SlotValue::Scalar(value), origin());
        }
    }
    fn reference(&mut self, from: u128, property: agq_kernel::PropertyId, to: u128) {
        self.changes.set(
            id(from),
            property,
            SlotValue::Scalar(Value::Reference(id(to))),
            origin(),
        );
    }
    fn name(&mut self, element: u128, name: &str) {
        self.changes.set(
            id(element),
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String(name.into())),
            origin(),
        );
    }
    fn own(&mut self, owner: u128, relationship: u128) {
        self.owned
            .entry(id(owner))
            .or_default()
            .push(Value::Reference(id(relationship)));
    }
    fn member(&mut self, owner: u128, relationship: u128, target: u128, kind: MetaclassId) {
        self.create(relationship, kind);
        self.own(owner, relationship);
        self.changes.set(
            id(relationship),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(target))]),
            origin(),
        );
    }
    fn finish(mut self) -> Snapshot {
        for (owner, values) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(values),
                origin(),
            );
        }
        self.base.apply(&self.changes).unwrap()
    }
}

// A consumer refers directly to ExternalFunction::result. Only the Function
// producer creates the required result binding to its nested body expression.
fn fixture() -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::LIBRARY_PACKAGE);
    f.create(100, c::PACKAGE);
    f.create(2, c::FUNCTION);
    f.member(1, 3, 2, c::OWNING_MEMBERSHIP);
    f.create(20, c::FEATURE);
    f.member(2, 21, 20, c::RETURN_PARAMETER_MEMBERSHIP);
    f.create(30, c::EXPRESSION);
    f.member(2, 31, 30, c::RESULT_EXPRESSION_MEMBERSHIP);
    f.create(40, c::FEATURE);
    f.member(30, 41, 40, c::RETURN_PARAMETER_MEMBERSHIP);
    f.create(4, c::FEATURE);
    f.member(100, 6, 4, c::OWNING_MEMBERSHIP);
    f.create(5, c::SUBSETTING);
    f.own(4, 5);
    f.reference(5, p::SUBSETTING_SUBSETTING_FEATURE, 4);
    f.reference(5, p::SUBSETTING_SUBSETTED_FEATURE, 20);
    f.create(60, c::FUNCTION);
    f.member(1, 61, 60, c::OWNING_MEMBERSHIP);
    f.create(62, c::FEATURE);
    f.member(60, 63, 62, c::RETURN_PARAMETER_MEMBERSHIP);
    f.create(110, c::NAMESPACE_IMPORT);
    f.own(100, 110);
    f.reference(110, p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE, 1);
    f.finish()
}

// The imported Functions have result structure, but the local specialization
// only reads their declared names until a qualified lookup enters a Function.
fn lookup_fixture(unnamed_feature: bool) -> Snapshot {
    let mut f = Fixture::new();
    for (element, class, name) in [
        (1, c::LIBRARY_PACKAGE, "Orchard"),
        (2, c::FUNCTION, "Copper"),
        (20, c::FEATURE, "answer"),
        (30, c::EXPRESSION, "tide"),
        (40, c::FEATURE, "foam"),
        (60, c::FUNCTION, "Indigo"),
        (62, c::FEATURE, "berry"),
        (100, c::PACKAGE, "Workshop"),
        (4, c::CLASSIFIER, "Needle"),
        (90, c::DATA_TYPE, "Thread"),
    ] {
        f.create(element, class);
        f.name(element, name);
    }
    f.member(1, 3, 2, c::OWNING_MEMBERSHIP);
    f.member(2, 21, 20, c::RETURN_PARAMETER_MEMBERSHIP);
    f.member(2, 31, 30, c::RESULT_EXPRESSION_MEMBERSHIP);
    f.member(30, 41, 40, c::RETURN_PARAMETER_MEMBERSHIP);
    f.member(1, 61, 60, c::OWNING_MEMBERSHIP);
    f.member(60, 63, 62, c::RETURN_PARAMETER_MEMBERSHIP);
    f.member(100, 6, 4, c::OWNING_MEMBERSHIP);
    f.member(100, 91, 90, c::OWNING_MEMBERSHIP);
    f.create(5, c::SPECIALIZATION);
    f.own(4, 5);
    f.reference(5, p::SPECIALIZATION_SPECIFIC, 4);
    f.reference(5, p::SPECIALIZATION_GENERAL, 90);
    f.create(110, c::NAMESPACE_IMPORT);
    f.own(100, 110);
    f.reference(110, p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE, 1);
    if unnamed_feature {
        f.create(70, c::FEATURE);
        f.member(1, 71, 70, c::OWNING_MEMBERSHIP);
    }
    f.finish()
}

fn context(
    overlay: &agq_kernel::derived::DerivedOverlay,
) -> Result<SemanticContext<'_>, PublicationOverlayError> {
    SemanticContext::for_overlay(
        overlay,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V8,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .map_err(PublicationOverlayError::Context)
}

#[test]
fn an_external_return_feature_schedules_its_function_without_importing_package_siblings() {
    let snapshot = fixture();
    let selected = subjects(snapshot.model(), [id(4), id(110)]).unwrap();
    for subject in [2, 3, 4, 5, 6, 20, 21, 30, 31, 40, 41, 110] {
        assert!(selected.contains(&id(subject)), "missing {subject}");
    }
    for unrelated in [1, 60, 61, 62, 63, 100] {
        assert!(
            !selected.contains(&id(unrelated)),
            "unrelated package fan-out {unrelated}"
        );
    }
    let run = |initial_subjects| {
        close_result_structure(
            &snapshot,
            PublicationClosureOptions {
                initial_subjects: Some(initial_subjects),
                ..Default::default()
            },
            context,
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap()
    };
    let old_scope = BTreeSet::from([id(4), id(5), id(20)]);
    let old = run(old_scope.clone());
    assert!(old.converged);
    assert_eq!(old.completeness, Completeness::Complete);
    assert!(
        KerMlQueries::new(context(&old.overlay).unwrap())
            .audit_publication_capabilities(old_scope.iter().copied())
            .failures
            .is_empty()
    );
    // Stable selected queries alone cannot establish dependency completeness.
    let boundary = Boundary::from_graph(old.overlay.model(), &old_scope).unwrap();
    assert!(!boundary.is_complete());
    assert!(boundary.missing_subjects.contains(&id(2)));
    assert_eq!(
        old.overlay
            .model()
            .instances(c::BINDING_CONNECTOR, true)
            .unwrap()
            .count(),
        0
    );
    let new = run(selected.clone());
    assert!(new.converged);
    assert_eq!(new.completeness, Completeness::Complete);
    assert!(
        new.overlay
            .model()
            .instances(c::BINDING_CONNECTOR, true)
            .unwrap()
            .count()
            > 0
    );
    let population = selected
        .into_iter()
        .chain(
            new.overlay
                .model()
                .elements()
                .filter(|r| snapshot.model().element(r.id()).is_none())
                .map(|r| r.id()),
        )
        .collect();
    let boundary = Boundary::from_graph(new.overlay.model(), &population).unwrap();
    assert!(
        boundary.is_complete(),
        "missing {:?}",
        boundary.missing_subjects
    );
}

#[test]
fn a_library_package_context_anchor_does_not_schedule_unrelated_functions() {
    let snapshot = fixture();
    let model = snapshot.model();
    let selected = subjects_with_context_anchors(model, [id(4), id(110)], [id(1)]).unwrap();
    assert_eq!(selected, subjects(model, [id(4), id(110)]).unwrap());
    // The concrete referenced return still brings its Function and body.
    for subject in [2, 20, 21, 30, 31, 40, 41] {
        assert!(selected.contains(&id(subject)), "missing {subject}");
    }
    for unrelated in [1, 60, 61, 62, 63] {
        assert!(!selected.contains(&id(unrelated)), "unrelated {unrelated}");
    }
    assert!(
        Boundary::from_graph(model, &selected)
            .unwrap()
            .is_complete()
    );

    // Excluding a concrete provider is still rejected by the unchanged boundary.
    let mut omitted_provider = selected;
    omitted_provider.remove(&id(2));
    let boundary = Boundary::from_graph(model, &omitted_provider).unwrap();
    assert!(boundary.missing_subjects.contains(&id(2)));
    assert!(!boundary.is_complete());
}

#[test]
fn a_package_selected_as_source_still_schedules_all_its_members() {
    let snapshot = fixture();
    let selected = subjects_with_context_anchors(snapshot.model(), [id(1)], [id(1)]).unwrap();
    assert_eq!(selected, subjects(snapshot.model(), [id(1)]).unwrap());
    for subject in [1, 2, 20, 21, 30, 31, 40, 41, 60, 61, 62, 63] {
        assert!(selected.contains(&id(subject)), "missing {subject}");
    }
}

#[test]
fn concrete_context_anchors_keep_their_semantic_owner_dependencies() {
    let snapshot = fixture();
    let selected =
        subjects_with_context_anchors(snapshot.model(), [id(4)], [id(1), id(62)]).unwrap();
    for subject in [60, 61, 62, 63] {
        assert!(selected.contains(&id(subject)), "missing {subject}");
    }
    assert!(!selected.contains(&id(1)));
    assert!(subjects_with_context_anchors(snapshot.model(), [id(4)], [id(999)]).is_err());
}

#[test]
fn declared_imported_function_names_do_not_require_result_production() {
    let snapshot = lookup_fixture(false);
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let population = subjects(overlay.model(), [id(4), id(110)]).unwrap();
    let q = KerMlStatusQueries::new(context(&overlay).unwrap());
    for (name, expected) in [("Copper", vec![id(2)]), ("Absent", vec![])] {
        let answer = q.lookup_relationship_target_with_reads(
            id(5),
            p::SPECIALIZATION_GENERAL,
            &QualifiedName {
                absolute: false,
                segments: vec![name.into()],
            },
        );
        assert_eq!(answer.outcome.completeness, Completeness::Complete);
        assert_eq!(
            answer
                .outcome
                .value
                .iter()
                .map(|m| m.element)
                .collect::<Vec<_>>(),
            expected
        );
        let providers = answer.reads.publication_provider_reads(overlay.model());
        let mut boundary = Boundary::from_graph(overlay.model(), &population).unwrap();
        boundary.include_provider_reads(overlay.model(), &population, &providers);
        assert!(
            boundary.is_complete(),
            "{name}: {:?}",
            boundary.missing_subjects
        );
        for unrelated in [2, 20, 30, 40, 60, 62] {
            assert!(!population.contains(&id(unrelated)));
            assert!(!providers.bounded_elements().contains(&id(unrelated)));
        }
        // Those identity/name reads still invalidate queries across revisions.
        assert!(answer.reads.affected_by(&BTreeSet::from([id(2)]), false));
        assert!(answer.reads.affected_by(&BTreeSet::from([id(60)]), false));
        let mut coarse = Boundary::default();
        coarse.include_reads(overlay.model(), &population, &answer.reads);
        assert!(coarse.missing_subjects.contains(&id(2)));
        assert!(coarse.missing_subjects.contains(&id(60)));
        let all_elements = overlay.model().elements().map(|r| r.id()).collect();
        let mut complete_coarse = Boundary::default();
        complete_coarse.include_invalidation(
            overlay.model(),
            &all_elements,
            &answer.reads.into_invalidation(),
        );
        assert!(complete_coarse.is_complete());
    }
}

#[test]
fn a_qualified_result_lookup_still_requires_its_external_function() {
    let snapshot = lookup_fixture(false);
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let population = subjects(overlay.model(), [id(4), id(110)]).unwrap();
    let q = KerMlStatusQueries::new(context(&overlay).unwrap());
    let answer = q.lookup_relationship_target_with_reads(
        id(5),
        p::SPECIALIZATION_GENERAL,
        &QualifiedName {
            absolute: false,
            segments: vec!["Copper".into(), "answer".into()],
        },
    );
    assert_eq!(answer.outcome.completeness, Completeness::Complete);
    assert_eq!(answer.outcome.value.len(), 1);
    assert_eq!(answer.outcome.value[0].element, id(20));
    let providers = answer.reads.publication_provider_reads(overlay.model());
    let mut boundary = Boundary::default();
    boundary.include_provider_reads(overlay.model(), &population, &providers);
    assert!(!boundary.is_complete());
    assert!(boundary.missing_subjects.contains(&id(2)));
    assert!(!boundary.missing_subjects.contains(&id(60)));
    let expanded =
        expanded_subjects(overlay.model(), &population, &boundary.missing_subjects, 32).unwrap();
    for required in [2, 20, 21, 30, 31, 40, 41] {
        assert!(expanded.contains(&id(required)), "missing {required}");
    }
    assert!(!expanded.contains(&id(60)));
}

#[test]
fn missing_elements_remain_publication_provider_obligations() {
    let snapshot = lookup_fixture(false);
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot)
        .build()
        .unwrap();
    let answer = KerMlStatusQueries::new(context(&overlay).unwrap())
        .lookup_relationship_target_with_reads(
            id(999),
            p::SPECIALIZATION_GENERAL,
            &QualifiedName {
                absolute: false,
                segments: vec!["Copper".into()],
            },
        );
    assert_ne!(answer.outcome.completeness, Completeness::Complete);
    let providers = answer.reads.publication_provider_reads(overlay.model());
    let population = overlay.model().elements().map(|r| r.id()).collect();
    let mut boundary = Boundary::default();
    boundary.include_provider_reads(overlay.model(), &population, &providers);
    assert_eq!(boundary.missing_subjects, BTreeSet::from([id(999)]));
}

#[test]
fn unnamed_features_keep_their_mutable_name_resolution_dependencies() {
    let snapshot = lookup_fixture(true);
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let population = subjects(overlay.model(), [id(4), id(110)]).unwrap();
    let answer = KerMlStatusQueries::new(context(&overlay).unwrap())
        .lookup_relationship_target_with_reads(
            id(5),
            p::SPECIALIZATION_GENERAL,
            &QualifiedName {
                absolute: false,
                segments: vec!["Absent".into()],
            },
        );
    let providers = answer.reads.publication_provider_reads(overlay.model());
    let mut boundary = Boundary::default();
    boundary.include_provider_reads(overlay.model(), &population, &providers);
    assert!(!boundary.is_complete());
    assert!(boundary.missing_subjects.contains(&id(70)));
    assert!(!boundary.missing_subjects.contains(&id(2)));
    assert!(!boundary.missing_subjects.contains(&id(60)));
}

#[test]
fn a_discovered_provider_expands_the_scope_before_reclosing_producers() {
    let snapshot = fixture();
    let run = |population: BTreeSet<ElementId>| {
        close_result_structure(
            &snapshot,
            PublicationClosureOptions {
                initial_subjects: Some(population),
                ..Default::default()
            },
            context,
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap()
    };
    let initial = BTreeSet::from([id(4), id(5), id(20)]);
    let first = run(initial.clone());
    let missing = Boundary::from_graph(first.overlay.model(), &initial).unwrap();
    assert!(!missing.is_complete());
    assert!(missing.missing_subjects.contains(&id(2)));
    // A too-small workflow limit stops; it never turns a missing provider into a pass.
    assert!(
        expanded_subjects(
            snapshot.model(),
            &initial,
            &missing.missing_subjects,
            initial.len()
        )
        .is_err()
    );
    let expanded =
        expanded_subjects(snapshot.model(), &initial, &missing.missing_subjects, 32).unwrap();
    let second = run(expanded.clone());
    assert_eq!(second.completeness, Completeness::Complete);
    assert!(
        second
            .overlay
            .model()
            .instances(c::BINDING_CONNECTOR, true)
            .unwrap()
            .count()
            > 0
    );
    let final_population = expanded
        .into_iter()
        .chain(
            second
                .overlay
                .model()
                .elements()
                .filter(|record| snapshot.model().element(record.id()).is_none())
                .map(|record| record.id()),
        )
        .collect();
    assert!(
        Boundary::from_graph(second.overlay.model(), &final_population)
            .unwrap()
            .is_complete()
    );
    // Expansion remains local: the unrelated Function in the imported Package is excluded.
    assert!(!final_population.contains(&id(60)));
}
