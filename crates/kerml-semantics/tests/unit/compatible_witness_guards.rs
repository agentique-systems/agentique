use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerRead, producer_reads};
use agq_kernel::{
    derived::{DerivationBuilder, DerivedOverlay, StructuralSearch},
    provenance::Explanation as KernelExplanation,
};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;

fn fixture(original: bool) -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for n in [1, 2, 3, 20, 21] {
        f.create(n, c::FEATURE);
    }
    for n in [20, 21] {
        f.value(
            n,
            p::ELEMENT_DECLARED_NAME,
            Value::String(format!("guard{n}")),
        );
    }
    if original {
        member(&mut f, 1, 2, 4, c::FEATURE_MEMBERSHIP);
    }
    f.finish()
}

fn key(n: u128) -> DerivationKey {
    DerivationKey {
        subject: id(1),
        rule: RuleId::from_u128(n),
        output: OutputKey::from_u128(n + 100),
    }
}

fn property(n: u128) -> FactKey {
    FactKey::Property {
        element: id(n),
        property: p::ELEMENT_DECLARED_NAME,
    }
}

fn append_feature(
    builder: &mut DerivationBuilder,
    derivation: DerivationKey,
    feature: u128,
    guard: u128,
) {
    builder.element(
        derivation,
        c::FEATURE_MEMBERSHIP,
        BTreeMap::from([(
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(feature))]),
        )]),
        BTreeSet::from([Dependency::Declared(property(guard))]),
    );
    builder.searches(
        FactKey::Element(derivation.element_id()),
        BTreeSet::from([StructuralSearch::Property {
            element: id(guard),
            property: p::ELEMENT_DECLARED_NAME,
        }]),
    );
    builder.extend_ordered_references(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        vec![derivation.element_id()],
        KernelExplanation {
            rule: derivation.rule,
            dependencies: BTreeSet::from([Dependency::Derived(FactKey::Element(
                derivation.element_id(),
            ))]),
        },
    );
}

fn overlay(original: bool) -> DerivedOverlay {
    let mut builder = DerivationBuilder::new(fixture(original));
    if !original {
        append_feature(&mut builder, key(1000), 2, 20);
    }
    let mut builder = DerivationBuilder::from_overlay(builder.build().unwrap());
    append_feature(&mut builder, key(2000), 3, 21);
    builder.searches(
        FactKey::Property {
            element: id(1),
            property: p::ELEMENT_OWNED_RELATIONSHIP,
        },
        BTreeSet::from([StructuralSearch::ProducerClosure {
            subject: id(21),
            requirement: SemanticClosureRequirement::EffectiveTyping
                .contract_id()
                .into(),
        }]),
    );
    builder.build().unwrap()
}

fn queries(overlay: &DerivedOverlay, production: bool) -> KerMlQueries<'_> {
    let context = SemanticContext::for_overlay(
        overlay,
        SemanticOptions {
            baseline_profile: PROFILE,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap();
    if production {
        KerMlQueries::for_production(context)
    } else {
        KerMlQueries::new(context)
    }
}

#[test]
fn nonempty_witness_retains_selected_creation_and_archive_fallback() {
    for original in [false, true] {
        let overlay = overlay(original);
        for archived in [false, true] {
            let restored;
            let current = if archived {
                let mut bytes = vec![];
                agq_kernel::archive::write_overlay(&overlay, &mut bytes).unwrap();
                restored = agq_kernel::archive::read_overlay(
                    std::io::Cursor::new(bytes),
                    Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()),
                )
                .unwrap();
                &restored
            } else {
                &overlay
            };
            for production in [false, true] {
                let q = queries(current, production);
                let witness = q.owned_feature_witness(id(1)).expect("canonical child");
                assert_eq!(witness.completeness, Completeness::Complete);
                let reads = producer_reads(&witness, current.model());
                assert!(!reads.contains(&ProducerRead::Structural(id(1))));
                assert_eq!(
                    reads.contains(&ProducerRead::Requirement(
                        id(21),
                        SemanticClosureRequirement::EffectiveTyping,
                    )),
                    !original && archived,
                    "archive fallback must retain aggregate support; native selection must exclude unrelated appends"
                );
                if original {
                    assert!(reads.contains(&ProducerRead::DeclaredProperty(
                        id(1),
                        p::ELEMENT_OWNED_RELATIONSHIP,
                    )));
                } else {
                    assert!(
                        witness
                            .canonical_dependencies
                            .contains(&Dependency::Derived(FactKey::Element(
                                key(1000).element_id()
                            ),))
                    );
                    assert!(
                        reads.contains(&ProducerRead::Property(id(20), p::ELEMENT_DECLARED_NAME))
                    );
                }
            }
        }
    }
}

#[test]
fn nonempty_witness_cannot_erase_an_independent_current_population_read() {
    for original in [false, true] {
        let overlay = overlay(original);
        for production in [false, true] {
            let q = queries(&overlay, production);
            let witness = q.owned_feature_witness(id(1)).expect("canonical child");
            let mut broad = q.result(());
            broad.merge(q.memberships(id(1)));
            broad.merge(q.canonical_fact_evidence(FactKey::Property {
                element: id(1),
                property: p::ELEMENT_OWNED_RELATIONSHIP,
            }));
            for reverse in [false, true] {
                let mut combined = q.result(());
                if reverse {
                    combined.merge(broad.clone());
                    combined.merge(witness.clone());
                } else {
                    combined.merge(witness.clone());
                    combined.merge(broad.clone());
                }
                let reads = producer_reads(&combined, overlay.model());
                assert!(reads.contains(&ProducerRead::Structural(id(1))));
                assert!(reads.contains(&ProducerRead::Property(
                    id(1),
                    p::ELEMENT_OWNED_RELATIONSHIP
                )));
                assert!(reads.contains(&ProducerRead::Requirement(
                    id(21),
                    SemanticClosureRequirement::EffectiveTyping,
                )));
            }
        }
    }
}
