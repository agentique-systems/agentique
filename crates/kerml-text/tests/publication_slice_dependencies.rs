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
use publication_dependencies::{Boundary, subjects};
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
    f.create(1, c::NAMESPACE);
    f.create(100, c::NAMESPACE);
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
fn final_reference_reads_reject_an_omitted_semantic_provider() {
    let snapshot = fixture();
    let overlay = agq_kernel::derived::DerivationBuilder::new(snapshot.clone())
        .build()
        .unwrap();
    let q = KerMlStatusQueries::new(context(&overlay).unwrap());
    let answer = q.lookup_relationship_target_with_reads(
        id(5),
        p::SUBSETTING_SUBSETTED_FEATURE,
        &QualifiedName {
            absolute: false,
            segments: vec!["Missing".into()],
        },
    );
    let mut boundary = Boundary::default();
    boundary.include_reads(overlay.model(), &BTreeSet::from([id(4)]), &answer.reads);
    assert!(!boundary.is_complete());
    assert!(boundary.missing_subjects.contains(&id(2)));
    let all_types = overlay.model().elements().map(|r| r.id()).collect();
    let mut boundary = Boundary::default();
    boundary.include_invalidation(
        overlay.model(),
        &all_types,
        &answer.reads.into_invalidation(),
    );
    assert!(boundary.is_complete());
}
