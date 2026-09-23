//! Exact semantic comparison shared by monolithic and compositional fixtures.
//! Scheduler counters, allocation addresses and certificate tree shape are not semantics.
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use agq_kernel::derived::{
    ComputationFailure, DerivationBuilder, DerivedOverlay, IncompleteReason, StructuralSearch,
};

pub(crate) fn graph_difference(
    expected: &DerivedOverlay,
    actual: &DerivedOverlay,
) -> Option<&'static str> {
    let a = expected.model();
    let b = actual.model();
    if !expected.facts().eq(actual.facts()) {
        return Some("derived identities and proofs");
    }
    if !a.elements().eq(b.elements()) {
        return Some("canonical elements, stored slots, provenance or ownership order");
    }
    // Query the logical original slots rather than comparing physical snapshot
    // boundaries: a composition may retain earlier derived records in a layer.
    for record in a.elements() {
        for (property, _) in record.slots() {
            if a.declared_slot(record.id(), property) != b.declared_slot(record.id(), property) {
                return Some("original declared slots");
            }
        }
    }
    if !a.association_occurrences().eq(b.association_occurrences()) {
        return Some("association occurrences");
    }
    if !a
        .derived_navigation_results()
        .eq(b.derived_navigation_results())
    {
        return Some("derived navigation slots");
    }
    if !a.computation_failures().eq(b.computation_failures()) {
        return Some("unsuccessful computations");
    }
    if !a.computation_searches().eq(b.computation_searches()) {
        return Some("positive and negative search evidence");
    }
    // Although this is optional kernel cache data, a producer may use its precise
    // proof. Compare its availability and contents before comparing query answers.
    if !a
        .ordered_reference_contributions()
        .eq(b.ordered_reference_contributions())
    {
        return Some("selected ordered contribution evidence");
    }
    let aggregate = crate::context_digest::model_digest(a);
    let other = crate::context_digest::model_digest(b);
    if aggregate != other
        || crate::context_digest::producer_model_digest(a, aggregate)
            != crate::context_digest::producer_model_digest(b, other)
    {
        return Some("semantic graph digest");
    }
    None
}

pub(crate) fn certificate_difference(
    expected: &ProducerClosureCertificate,
    actual: &ProducerClosureCertificate,
    model: &ModelView,
) -> Option<&'static str> {
    if expected.model_digest() != actual.model_digest()
        || expected.registry_digest() != actual.registry_digest()
        || expected.context_contract_digest() != actual.context_contract_digest()
    {
        return Some("certificate interpretation binding");
    }
    for subject in model.elements().map(|record| record.id()) {
        for requirement in SemanticClosureRequirement::ALL {
            if expected.is_closed(subject, requirement) != actual.is_closed(subject, requirement) {
                return Some("closed semantic requirements");
            }
        }
        let mut family = 0;
        loop {
            let a = expected.evaluation(subject, family);
            let b = actual.evaluation(subject, family);
            if a != b {
                return Some("applicable producer evaluation states");
            }
            if a.is_none() {
                break;
            }
            family += 1;
        }
    }
    None
}

pub(crate) fn assert_exact_closure(expected: &PublicationClosure, actual: &PublicationClosure) {
    assert_eq!(expected.completeness, actual.completeness);
    assert_eq!(expected.converged, actual.converged);
    assert_eq!(graph_difference(&expected.overlay, &actual.overlay), None);
    match (&expected.certificate, &actual.certificate) {
        (Some(a), Some(b)) => {
            assert_eq!(certificate_difference(a, b, expected.overlay.model()), None);
        }
        (None, None) => {}
        _ => panic!("closure certificate availability differs"),
    }
}

pub(crate) fn assert_exact_queries(expected: &KerMlQueries<'_>, actual: &KerMlQueries<'_>) {
    for record in expected.model().elements() {
        let subject = record.id();
        assert_eq!(
            expected.effective_names(subject),
            actual.effective_names(subject)
        );
        assert_eq!(expected.owner(subject), actual.owner(subject));
        assert_eq!(
            expected.owned_relationships(subject),
            actual.owned_relationships(subject)
        );
        assert_eq!(
            expected.canonical_fact_evidence(FactKey::Element(subject)),
            actual.canonical_fact_evidence(FactKey::Element(subject))
        );
        for (property, _) in record.slots() {
            let fact = FactKey::Property {
                element: subject,
                property,
            };
            assert_eq!(
                expected.canonical_fact_evidence(fact),
                actual.canonical_fact_evidence(fact)
            );
        }
        if expected.is(subject, c::NAMESPACE) {
            assert_eq!(expected.memberships(subject), actual.memberships(subject));
        }
        if expected.is(subject, c::TYPE) {
            assert_eq!(expected.supertypes(subject), actual.supertypes(subject));
            assert_eq!(
                expected.effective_features(subject),
                actual.effective_features(subject)
            );
            assert_eq!(
                expected.all_specializations(subject),
                actual.all_specializations(subject)
            );
        }
        if expected.is(subject, c::FEATURE) {
            assert_eq!(
                expected.subsetted_features(subject),
                actual.subsetted_features(subject)
            );
            assert_eq!(
                expected.redefined_features(subject),
                actual.redefined_features(subject)
            );
            assert_eq!(
                expected.chaining_features(subject),
                actual.chaining_features(subject)
            );
        }
    }
    for (&(element, property), _) in expected.model().derived_navigation_results() {
        let fact = FactKey::Property { element, property };
        assert_eq!(
            expected.canonical_fact_evidence(fact),
            actual.canonical_fact_evidence(fact)
        );
    }
    for occurrence in expected.model().association_occurrences() {
        let fact = FactKey::AssociationOccurrence(occurrence.id());
        assert_eq!(
            expected.canonical_fact_evidence(fact),
            actual.canonical_fact_evidence(fact)
        );
    }
}

fn fixture() -> Snapshot {
    let mut f = Fixture::new();
    for subject in [1, 2, 3] {
        f.create(subject, c::TYPE);
    }
    f.finish()
}

fn computed(base: &Snapshot, premise: ElementId, search: StructuralSearch) -> DerivedOverlay {
    let mut builder = DerivationBuilder::new(base.clone());
    builder.property(
        id(1),
        p::TYPE_IS_CONJUGATED,
        SlotValue::Scalar(Value::Boolean(false)),
        agq_kernel::provenance::Explanation {
            rule: RuleId::from_u128(99271),
            dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(premise))]),
        },
    );
    builder.searches(
        FactKey::Property {
            element: id(1),
            property: p::TYPE_IS_CONJUGATED,
        },
        BTreeSet::from([search]),
    );
    builder.build().unwrap()
}

#[test]
fn exact_equivalence_rejects_equal_count_different_proofs() {
    let base = fixture();
    let a = computed(&base, id(2), StructuralSearch::Incoming(id(2)));
    let b = computed(&base, id(3), StructuralSearch::Incoming(id(2)));
    assert_eq!(a.model().len(), b.model().len());
    assert_eq!(a.facts().count(), b.facts().count());
    assert_eq!(
        graph_difference(&a, &b),
        Some("derived identities and proofs")
    );
}

#[test]
fn exact_equivalence_rejects_equal_count_different_negative_searches() {
    let base = fixture();
    let a = computed(&base, id(2), StructuralSearch::Incoming(id(2)));
    let b = computed(&base, id(2), StructuralSearch::Incoming(id(3)));
    assert!(a.model().elements().eq(b.model().elements()));
    assert!(a.facts().eq(b.facts()));
    assert_eq!(
        a.model().computation_searches().count(),
        b.model().computation_searches().count()
    );
    assert_eq!(
        graph_difference(&a, &b),
        Some("positive and negative search evidence")
    );
}

#[test]
fn exact_equivalence_compares_unsuccessful_computation_details() {
    let base = fixture();
    let build = |reason| {
        let mut builder = DerivationBuilder::new(base.clone());
        builder
            .failure(
                id(1),
                p::TYPE_IS_CONJUGATED,
                ComputationFailure::Incomplete {
                    reason,
                    explanation: agq_kernel::provenance::Explanation {
                        rule: RuleId::from_u128(99272),
                        dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(
                            1,
                        )))]),
                    },
                    searches: BTreeSet::new(),
                },
            )
            .unwrap();
        builder.build().unwrap()
    };
    let a = build(IncompleteReason::MissingInput);
    let b = build(IncompleteReason::UnsupportedRuntimeSemantics);
    assert!(a.model().elements().eq(b.model().elements()));
    assert_eq!(graph_difference(&a, &b), Some("unsuccessful computations"));
}

#[test]
fn exact_equivalence_ignores_allocation_identity() {
    let base = fixture();
    let a = computed(&base, id(2), StructuralSearch::Incoming(id(2)));
    let b = computed(&base, id(2), StructuralSearch::Incoming(id(2)));
    assert!(!std::ptr::eq(a.model(), b.model()));
    assert_eq!(graph_difference(&a, &b), None);
}

#[test]
fn exact_equivalence_rejects_same_graph_with_different_closure_requirements() {
    use crate::producer_closure::ProducerEvaluationTable;
    let base = fixture();
    let family = ProducerFamilyId::new("Fixture.EquivalenceTyping");
    let registry = ProducerRegistry::new([ProducerDescriptor::new(
        family,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::TYPE]),
    )])
    .unwrap();
    let context = SemanticContext::for_snapshot(&base, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let mut table = ProducerEvaluationTable::default();
    for subject in base.model().elements().map(|record| record.id()) {
        table.pending(subject, base.model(), &registry);
    }
    let pending =
        ProducerClosureCertificate::issue(base.model(), context.id(), &registry, &table, |_| None);
    for subject in base.model().elements().map(|record| record.id()) {
        table
            .record(&[(subject, family, Completeness::Complete)], &registry)
            .unwrap();
    }
    let closed =
        ProducerClosureCertificate::issue(base.model(), context.id(), &registry, &table, |_| None);
    assert_eq!(pending.model_digest(), closed.model_digest());
    assert_eq!(
        certificate_difference(&pending, &closed, base.model()),
        Some("closed semantic requirements")
    );
}
