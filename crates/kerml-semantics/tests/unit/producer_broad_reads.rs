use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};
use agq_kernel::derived::{DerivationBuilder, StructuralSearch};

const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.UnrelatedBinding");
const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.PositionalReader");

#[test]
fn ordinary_positional_parameter_reads_preserve_bounded_searches() {
    use crate::producer_closure::{ProducerRead, producer_reads};
    let mut f = Fixture::new();
    f.create(1, c::BEHAVIOR);
    f.create(2, c::STEP);
    f.create(3, c::FEATURE);
    f.create(7, c::FEATURE);
    for feature in [3, 7] {
        f.enumeration(feature, p::FEATURE_DIRECTION, "in");
    }
    member(&mut f, 1, 3, 103, c::PARAMETER_MEMBERSHIP);
    member(&mut f, 2, 7, 107, c::PARAMETER_MEMBERSHIP);
    relation(&mut f, 2, 1, 201, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
    f.value(
        201,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(2)),
    );
    let snapshot = f.finish();
    for production in [false, true] {
        let context =
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap();
        let q = if production {
            KerMlQueries::for_production(context)
        } else {
            KerMlQueries::new(context)
        };
        let answer = q.implied_redefinitions(id(7));
        assert_eq!(
            answer.completeness,
            Completeness::Complete,
            "{:?}",
            answer.diagnostics
        );
        assert_eq!(answer.value, [id(3)]);
        let reads = producer_reads(&answer, snapshot.model());
        assert!(!reads.contains(&ProducerRead::Global), "{reads:?}");
        assert!(!reads.contains(&ProducerRead::Inverse), "{reads:?}");
    }
}

#[test]
fn sealed_dependency_proof_searches_do_not_become_project_producer_reads() {
    let registry = ProducerRegistry::new([
        ProducerDescriptor::new(
            WRITER,
            [ProducerEffect::Membership],
            ProducerApplicability::Subtypes(vec![c::EXPRESSION]),
        ),
        ProducerDescriptor::new(
            READER,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::STEP]),
        ),
    ])
    .unwrap();
    for (contribution_search, historical_search, archive) in [
        (false, StructuralSearch::Model, false),
        (false, StructuralSearch::Incoming(id(1)), false),
        (true, StructuralSearch::Incoming(id(1)), false),
        (true, StructuralSearch::Incoming(id(1)), true),
    ] {
        let mut f = Fixture::new();
        f.create(1, c::BEHAVIOR);
        f.create(3, c::FEATURE);
        f.enumeration(3, p::FEATURE_DIRECTION, "in");
        f.create(10, c::PARAMETER_MEMBERSHIP);
        let declared = f.finish();
        let mut slots: BTreeMap<_, _> = declared
            .model()
            .element(id(10))
            .unwrap()
            .slots()
            .map(|(property, slot)| (property, slot.value().clone()))
            .collect();
        slots.insert(
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(3))]),
        );
        let key = DerivationKey {
            rule: RuleId::from_u128(9100),
            subject: id(1),
            output: OutputKey::from_u128(9101),
        };
        let mut builder = DerivationBuilder::new(declared);
        builder.element(
            key,
            c::PARAMETER_MEMBERSHIP,
            slots,
            BTreeSet::from([Dependency::Declared(FactKey::Element(id(1)))]),
        );
        builder.extend_ordered_references(
            id(1),
            p::ELEMENT_OWNED_RELATIONSHIP,
            vec![key.element_id()],
            agq_kernel::provenance::Explanation {
                rule: key.rule,
                dependencies: BTreeSet::from([Dependency::Derived(FactKey::Element(
                    key.element_id(),
                ))]),
            },
        );
        let proof_fact = if contribution_search {
            FactKey::Property {
                element: id(1),
                property: p::ELEMENT_OWNED_RELATIONSHIP,
            }
        } else {
            FactKey::Element(key.element_id())
        };
        builder.searches(proof_fact, BTreeSet::from([historical_search.clone()]));
        let mut overlay = builder.build().unwrap();
        if archive {
            let mut bytes = vec![];
            agq_kernel::archive::write_overlay(&overlay, &mut bytes).unwrap();
            overlay = agq_kernel::archive::read_overlay(
                std::io::Cursor::new(bytes),
                Arc::new(agq_kerml::registry().unwrap()),
            )
            .unwrap();
            assert!(
                overlay
                    .model()
                    .ordered_reference_contribution(
                        id(1),
                        p::ELEMENT_OWNED_RELATIONSHIP,
                        key.element_id()
                    )
                    .is_none()
            );
        }
        let overlay = Arc::new(overlay);
        let sealed_context =
            SemanticContext::for_overlay(&overlay, Default::default(), BTreeSet::new())
                .unwrap()
                .with_producer_registry_digest(registry.digest())
                .unwrap();
        let sealed =
            Arc::new(ProducerClosureCertificate::initial(&sealed_context, &registry).unwrap());
        assert!(sealed.is_fully_closed(overlay.model()));
        let sealed_context = sealed_context.with_producer_closure(sealed).unwrap();
        let dependency =
            ProducerClosedDependency::new(overlay.clone(), &sealed_context, &registry).unwrap();
        let base = dependency.project_snapshot();
        let mut f = Fixture {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
        };
        f.create(2, c::STEP);
        f.create(7, c::FEATURE);
        f.enumeration(7, p::FEATURE_DIRECTION, "in");
        member(&mut f, 2, 7, 107, c::PARAMETER_MEMBERSHIP);
        relation(&mut f, 2, 1, 201, c::FEATURE_TYPING, p::FEATURE_TYPING_TYPE);
        f.value(
            201,
            p::FEATURE_TYPING_TYPED_FEATURE,
            Value::Reference(id(2)),
        );
        f.create(99, c::EXPRESSION);
        let project = f.finish();
        let context = dependency
            .project_context(&project, &[], BTreeSet::new(), BTreeSet::new())
            .unwrap();
        let initial = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
        let context = context.with_producer_closure(initial).unwrap();
        assert!(context.sealed_dependency_fact(proof_fact));
        let unauthenticated =
            SemanticContext::for_snapshot(&project, Default::default(), BTreeSet::new()).unwrap();
        assert!(
            !unauthenticated.sealed_dependency_fact(proof_fact),
            "a plain immutable mount is not authority"
        );
        let untrusted = KerMlQueries::new(unauthenticated).canonical_fact_evidence(proof_fact);
        assert!(
            untrusted
                .search_dependencies
                .contains(&SearchDependency::Kernel(historical_search.clone()))
        );
        for production in [false, true] {
            let q = if production {
                KerMlQueries::for_production(context.fork())
            } else {
                KerMlQueries::new(context.fork())
            };
            let answer = q.implied_redefinitions(id(7));
            assert_eq!(
                answer.completeness,
                Completeness::Complete,
                "{:?}",
                answer.diagnostics
            );
            assert_eq!(answer.value, [id(3)]);
            let sealed_proof = q.canonical_fact_evidence(proof_fact);
            assert!(
                sealed_proof
                    .canonical_dependencies
                    .contains(&Dependency::Derived(proof_fact))
            );
            if !production {
                assert!(matches!(
                    sealed_proof.fact_origins[&proof_fact].as_ref(),
                    Origin::Derived(_)
                ));
                let declared = q.canonical_fact_evidence(FactKey::Element(id(1)));
                assert!(
                    declared
                        .declared_fact_origins
                        .contains_key(&FactKey::Element(id(1)))
                );
            }
            let mut table = ProducerEvaluationTable::default();
            for record in project.model().elements() {
                table.pending(record.id(), project.model(), &registry);
            }
            table
                .record(&[(id(2), READER, Completeness::Complete)], &registry)
                .unwrap();
            table.record_reads(
                &[(id(2), READER, producer_reads(&answer, project.model()))],
                &registry,
            );
            let certificate = ProducerClosureCertificate::issue(
                project.model(),
                context.id(),
                &registry,
                &table,
                |subject| context.dependency_closure_source(subject),
            );
            assert_eq!(
                certificate.evaluation(id(2), registry.index(READER).unwrap()),
                Some(ProducerEvaluationState::EvaluatedComplete),
                "sealed proof's search was reinterpreted against the project; production={production}, contribution={contribution_search}"
            );
            for live_first in [false, true] {
                let mut live = q.result(Vec::<ElementId>::new());
                live.search_dependencies
                    .insert(SearchDependency::Kernel(historical_search.clone()));
                let mut mixed = if live_first {
                    live.merge(answer.clone());
                    live
                } else {
                    let mut mixed = answer.clone();
                    mixed.merge(live);
                    mixed
                };
                mixed.expand_search_dependencies();
                assert!(
                    mixed
                        .search_dependencies
                        .contains(&SearchDependency::Kernel(historical_search.clone()))
                );
                table.record_reads(
                    &[(id(2), READER, producer_reads(&mixed, project.model()))],
                    &registry,
                );
                let mixed_certificate = ProducerClosureCertificate::issue(
                    project.model(),
                    context.id(),
                    &registry,
                    &table,
                    |subject| context.dependency_closure_source(subject),
                );
                assert_eq!(
                    mixed_certificate.evaluation(id(2), registry.index(READER).unwrap()),
                    Some(ProducerEvaluationState::Pending),
                    "a live identical search must survive either merge order"
                );
            }
        }
        // The accepted relationship's record remains identical while a new
        // local noncomposite owner changes its current inverse navigation.
        let inverse = FactKey::Property {
            element: id(10),
            property: p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
        };
        assert!(
            !context.sealed_dependency_fact(inverse),
            "absent slots are not sealed facts"
        );
        let mut edit = project.change_set();
        edit.set(
            id(2),
            p::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![Value::Reference(id(107)), Value::Reference(id(10))]),
            origin(),
        );
        let changed = project.apply(&edit).unwrap();
        let changed_context = dependency
            .project_context(&changed, &[], BTreeSet::new(), BTreeSet::new())
            .unwrap();
        assert!(changed_context.sealed_dependency_fact(FactKey::Element(id(10))));
        assert!(!changed_context.sealed_dependency_fact(inverse));
        let current = KerMlQueries::new(changed_context).canonical_fact_evidence(inverse);
        assert!(
            current
                .search_dependencies
                .contains(&SearchDependency::SourceRelationships {
                    source: id(10),
                    class: c::ELEMENT,
                    property: p::ELEMENT_OWNED_RELATIONSHIP,
                })
        );
    }
}
