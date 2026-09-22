use super::*;
use agq_kernel::{
    derived::{DerivationBuilder, DerivedOverlay},
    metamodel::*,
    provenance::{ByteRange, SourceOrigin},
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const MM: MetamodelId = MetamodelId::from_u128(1);
const NODE: MetaclassId = MetaclassId::from_u128(2);
const NAME: PropertyId = PropertyId::from_u128(3);
const REFS: PropertyId = PropertyId::from_u128(4);
const COMPUTED: PropertyId = PropertyId::from_u128(5);
const ASSOCIATION: AssociationId = AssociationId::from_u128(6);
const LEFT: PropertyId = PropertyId::from_u128(7);
const RIGHT: PropertyId = PropertyId::from_u128(8);
const RULE: RuleId = RuleId::from_u128(9);

fn id(value: u128) -> ElementId {
    ElementId::from_u128(value)
}
fn authored() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
fn key(index: usize) -> DerivationKey {
    DerivationKey {
        rule: RULE,
        subject: id(1),
        output: OutputKey::from_u128(index as u128),
    }
}
fn property(id: PropertyId, name: &str, kind: ValueKind) -> PropertyDescriptor {
    PropertyDescriptor {
        id,
        name: name.into(),
        owner: PropertyOwner::Class(NODE),
        value_kind: kind,
        multiplicity: Multiplicity::MANY,
        ordered: false,
        unique: true,
        derived: false,
        composite: false,
        redefines: BTreeSet::new(),
        subsets: BTreeSet::new(),
        derived_union: false,
        association: None,
        opposite_ends: BTreeSet::new(),
    }
}
fn snapshot(count: usize, origin: DeclaredOrigin) -> Snapshot {
    let mut name = property(NAME, "name", ValueKind::String);
    name.multiplicity.upper = Some(1);
    let mut refs = property(REFS, "references", ValueKind::Reference(NODE));
    refs.ordered = true;
    let mut computed = property(COMPUTED, "computed", ValueKind::Boolean);
    computed.multiplicity.upper = Some(1);
    computed.derived = true;
    let mut properties = vec![name, refs, computed];
    for (id, opposite, name) in [(LEFT, RIGHT, "left"), (RIGHT, LEFT, "right")] {
        let mut end = property(id, name, ValueKind::Reference(NODE));
        end.owner = PropertyOwner::Association(ASSOCIATION);
        end.association = Some(ASSOCIATION);
        end.opposite_ends.insert(opposite);
        end.ordered = true;
        end.derived = id == LEFT;
        properties.push(end);
    }
    let registry = MetamodelRegistry::from_descriptors(DescriptorSet {
        models: vec![MetamodelDescriptor {
            id: MM,
            name: "Digest fixture".into(),
            version: Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
            uri: "urn:digest-test".into(),
        }],
        classes: vec![MetaclassDescriptor {
            id: NODE,
            name: "Node".into(),
            package: vec![],
            metamodel: MM,
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
        }],
        associations: vec![AssociationDescriptor {
            id: ASSOCIATION,
            name: "Association".into(),
            package: vec![],
            metamodel: MM,
            member_ends: vec![LEFT, RIGHT],
            navigable_owned_ends: BTreeSet::from([LEFT, RIGHT]),
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
        }],
        properties,
        ..Default::default()
    })
    .unwrap();
    let base = Snapshot::new(Arc::new(registry));
    let mut change = base.change_set();
    for index in 1..=count {
        change.create(id(index as u128), NODE, origin.clone());
    }
    base.apply(&change).unwrap()
}
fn evidence(dependencies: impl IntoIterator<Item = Dependency>) -> Explanation {
    Explanation {
        rule: RULE,
        dependencies: dependencies.into_iter().collect(),
    }
}
fn inspect(model: &ModelView) -> ([u8; 32], DigestWork) {
    let mut graph = GraphEncoder::new(MODEL_SCHEMA);
    graph.model(model);
    let work = graph.work;
    (graph.finish(), work)
}

#[test]
fn digest_encodes_sixty_thousand_premises_once_for_twenty_thousand_outputs() {
    let base = snapshot(60_000, authored());
    let proof =
        Arc::new(evidence(base.model().elements().map(|record| {
            Dependency::Declared(FactKey::Element(record.id()))
        })));
    let mut builder = DerivationBuilder::new(base);
    for index in 0..20_000 {
        builder.element_with_explanation(key(index), NODE, [], proof.clone());
    }
    let overlay = builder.build().unwrap();
    let (_, work) = inspect(overlay.model());
    assert_eq!(
        work,
        DigestWork {
            proofs_encoded: 1,
            proof_reuses: 19_999,
            dependency_edges_encoded: 60_000,
        }
    );
}

#[test]
fn digest_is_independent_of_proof_allocation_creation_order_and_batch_boundaries() {
    let base = snapshot(3, authored());
    let explanation = evidence([Dependency::Declared(FactKey::Element(id(1)))]);
    let shared = Arc::new(explanation.clone());
    let distinct = Arc::new(explanation);
    assert!(!Arc::ptr_eq(&shared, &distinct));
    let encode = |proofs: &[Arc<Explanation>]| {
        let origins: Vec<_> = proofs.iter().cloned().map(Origin::Derived).collect();
        let mut graph = GraphEncoder::new(MODEL_SCHEMA);
        for origin in &origins {
            graph.origin(origin);
        }
        let work = graph.work;
        (graph.finish(), work)
    };
    let (a, shared_work) = encode(&[shared.clone(), shared.clone()]);
    let (b, distinct_work) = encode(&[shared.clone(), distinct]);
    assert_eq!(a, b);
    assert_eq!(shared_work.proofs_encoded, 1);
    assert_eq!(distinct_work.proofs_encoded, 2);
    let mut all = DerivationBuilder::new(base.clone());
    for index in 0..8 {
        all.element_with_explanation(key(index), NODE, [], shared.clone());
    }
    let all = all.build().unwrap();
    let mut staged = DerivationBuilder::new(base).build().unwrap();
    for index in (0..8).rev() {
        let mut next = DerivationBuilder::from_overlay(staged);
        next.element_with_explanation(key(index), NODE, [], Arc::new(shared.as_ref().clone()));
        staged = next.build().unwrap();
    }
    assert_eq!(model_digest(all.model()), model_digest(staged.model()));
}

#[test]
fn slot_values_order_source_evidence_and_library_population_affect_identity() {
    let library = DeclaredOrigin::StandardLibrary {
        library: LibraryId::from_u128(101),
    };
    let base = snapshot(3, library.clone());
    let edit = |value: SlotValue, origin: DeclaredOrigin| {
        let mut change = base.change_set();
        change.set(id(1), REFS, value, origin);
        base.apply(&change).unwrap()
    };
    let first = edit(
        SlotValue::Ordered(vec![Value::Reference(id(2)), Value::Reference(id(3))]),
        library.clone(),
    );
    let reordered = edit(
        SlotValue::Ordered(vec![Value::Reference(id(3)), Value::Reference(id(2))]),
        library,
    );
    assert_ne!(model_digest(first.model()), model_digest(reordered.model()));
    assert_ne!(
        library_graph_digest(first.model()),
        library_graph_digest(reordered.model())
    );
    let empty = edit(SlotValue::Ordered(vec![]), authored());
    assert_ne!(model_digest(base.model()), model_digest(empty.model()));
    let source = |end| DeclaredOrigin::Authored {
        source: Some(SourceOrigin {
            document: DocumentId::from_u128(4),
            revision: SourceRevisionId::from_u128(5),
            range: ByteRange::new(1, end).unwrap(),
            syntax_node: Some(SyntaxNodeId::from_u128(6)),
        }),
    };
    let a = edit(SlotValue::Ordered(vec![]), source(2));
    let b = edit(SlotValue::Ordered(vec![]), source(3));
    assert_ne!(model_digest(a.model()), model_digest(b.model()));
    let mut change = first.change_set();
    change.create(id(9), NODE, authored());
    let authored_edit = first.apply(&change).unwrap();
    assert_ne!(
        model_digest(first.model()),
        model_digest(authored_edit.model())
    );
    assert_eq!(
        library_graph_digest(first.model()),
        library_graph_digest(authored_edit.model())
    );
    let mut extension = DerivationBuilder::new(first.clone());
    extension.extend_ordered_references(id(1), REFS, vec![id(1)], evidence([]));
    let extended = extension.build().unwrap();
    assert_ne!(
        library_graph_digest(first.model()),
        library_graph_digest(extended.model())
    );
}

#[test]
fn derived_proofs_searches_navigation_and_failure_details_are_included() {
    let base = snapshot(3, authored());
    let build = |extra: Option<Dependency>, search: Option<StructuralSearch>, navigation: bool| {
        let mut builder = DerivationBuilder::new(base.clone());
        builder.element(key(1), NODE, [], extra.into_iter().collect());
        if let Some(search) = search {
            builder.searches(
                FactKey::Element(key(1).element_id()),
                BTreeSet::from([search]),
            );
        }
        if navigation {
            builder.property(
                id(1),
                LEFT,
                SlotValue::Ordered(vec![Value::Reference(id(2))]),
                evidence([]),
            );
        }
        builder.build().unwrap()
    };
    let plain = build(None, None, false);
    let extra = build(
        Some(Dependency::Declared(FactKey::Element(id(2)))),
        None,
        false,
    );
    let search = build(None, Some(StructuralSearch::Incoming(id(2))), false);
    let navigation = build(None, None, true);
    let identity_search = build(None, Some(StructuralSearch::ElementIdentity(id(2))), false);
    let closure_search = |subject| {
        build(
            None,
            Some(StructuralSearch::ProducerClosure {
                subject,
                requirement: "agq-semantic-closure/EffectiveTyping/1".into(),
            }),
            false,
        )
    };
    let closure = closure_search(id(2));
    let other_closure = closure_search(id(3));
    let owned_search = build(
        None,
        Some(StructuralSearch::OwnedRelationships {
            owner: id(2),
            class: NODE,
        }),
        false,
    );
    let other_owned_search = build(
        None,
        Some(StructuralSearch::OwnedRelationships {
            owner: id(3),
            class: NODE,
        }),
        false,
    );
    let source_search = build(
        None,
        Some(StructuralSearch::SourceRelationships {
            source: id(2),
            class: NODE,
            property: REFS,
        }),
        false,
    );
    let other_source_search = build(
        None,
        Some(StructuralSearch::SourceRelationships {
            source: id(2),
            class: NODE,
            property: LEFT,
        }),
        false,
    );
    let digests: BTreeSet<_> = [
        &plain,
        &extra,
        &search,
        &navigation,
        &identity_search,
        &closure,
        &other_closure,
        &owned_search,
        &other_owned_search,
        &source_search,
        &other_source_search,
    ]
    .into_iter()
    .map(|overlay| model_digest(overlay.model()))
    .collect();
    assert_eq!(digests.len(), 11);
    let failures = [
        ComputationFailure::Incomplete {
            reason: IncompleteReason::MissingInput,
            explanation: evidence([]),
            searches: BTreeSet::new(),
        },
        ComputationFailure::Incomplete {
            reason: IncompleteReason::IncompleteDependency,
            explanation: evidence([]),
            searches: BTreeSet::new(),
        },
        ComputationFailure::Invalid {
            diagnostic: "a".into(),
            explanation: evidence([]),
            searches: BTreeSet::new(),
        },
        ComputationFailure::Invalid {
            diagnostic: "b".into(),
            explanation: evidence([]),
            searches: BTreeSet::new(),
        },
        ComputationFailure::Invalid {
            diagnostic: "a".into(),
            explanation: evidence([]),
            searches: BTreeSet::from([StructuralSearch::Element(id(3))]),
        },
    ];
    let mut digests = BTreeSet::new();
    for failure in failures {
        let mut builder = DerivationBuilder::new(base.clone());
        builder.failure(id(1), COMPUTED, failure).unwrap();
        digests.insert(model_digest(builder.build().unwrap().model()));
    }
    assert_eq!(digests.len(), 5);
}

#[test]
fn transported_closure_requirements_do_not_canonicalize_certificate_history() {
    use crate::{QueryResult, SearchDependency, SemanticClosureRequirement, SemanticContext};
    let normative = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
    let context =
        SemanticContext::for_snapshot(&normative, Default::default(), Default::default()).unwrap();
    let base = snapshot(3, authored());
    let build = |digests: &[Option<[u8; 32]>]| {
        let mut answer = QueryResult::new(context.id(), ());
        for certificate_digest in digests {
            answer
                .search_dependencies
                .insert(SearchDependency::ProducerClosure {
                    subject: id(2),
                    requirement: SemanticClosureRequirement::EffectiveTyping,
                    certificate_digest: *certificate_digest,
                });
        }
        let searches = crate::read_dependencies::structural_searches(&answer);
        assert_eq!(
            searches,
            BTreeSet::from([StructuralSearch::ProducerClosure {
                subject: id(2),
                requirement: "agq-semantic-closure/EffectiveTyping/1".into(),
            }])
        );
        let mut builder = DerivationBuilder::new(base.clone());
        builder.element(key(1), NODE, [], BTreeSet::new());
        builder.searches(FactKey::Element(key(1).element_id()), searches);
        (answer.search_dependencies, builder.build().unwrap())
    };
    let (first_evidence, first) = build(&[Some([42; 32])]);
    let (second_evidence, second) = build(&[Some([43; 32])]);
    let (_, repeated) = build(&[None, Some([42; 32]), Some([43; 32])]);
    assert_ne!(
        first_evidence, second_evidence,
        "query evidence remains exact"
    );
    for other in [&second, &repeated] {
        assert_eq!(model_digest(first.model()), model_digest(other.model()));
        assert!(
            first
                .model()
                .computation_searches()
                .eq(other.model().computation_searches())
        );
    }
}

#[test]
fn occurrence_endpoints_positions_and_provenance_are_included() {
    let base = snapshot(3, authored());
    let build = |reverse: bool, swap: bool, extra: bool| -> DerivedOverlay {
        let mut builder = DerivationBuilder::new(base.clone());
        for index in 0..2 {
            let mut ends = BTreeMap::from([(LEFT, id(1)), (RIGHT, id(index + 2))]);
            if swap {
                ends = BTreeMap::from([(RIGHT, id(1)), (LEFT, id(index + 2))]);
            }
            let position = if reverse { 1 - index } else { index } as usize;
            let positions = if swap {
                BTreeMap::from([(RIGHT, 0), (LEFT, position)])
            } else {
                BTreeMap::from([(LEFT, 0), (RIGHT, position)])
            };
            builder.association_occurrence(
                key(index as usize),
                ASSOCIATION,
                ends,
                positions,
                if extra {
                    BTreeSet::from([Dependency::Declared(FactKey::Element(id(3)))])
                } else {
                    BTreeSet::new()
                },
            );
        }
        builder.build().unwrap()
    };
    let digests: BTreeSet<_> = [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ]
    .into_iter()
    .map(|(reverse, swap, extra)| model_digest(build(reverse, swap, extra).model()))
    .collect();
    assert_eq!(digests.len(), 4);
}

#[test]
fn framed_values_and_origin_variants_do_not_alias() {
    let slot_digest = |value: &SlotValue| {
        let mut encoder = Encoder::new(MODEL_SCHEMA);
        encoder.slot_value(value);
        encoder.finish()
    };
    let values = [
        SlotValue::Scalar(Value::String("ab".into())),
        SlotValue::Ordered(vec![Value::String("a".into()), Value::String("b".into())]),
        SlotValue::Ordered(vec![Value::String("ab".into())]),
        SlotValue::Set(BTreeSet::from([Value::String("ab".into())])),
        SlotValue::Bag(vec![Value::String("ab".into())]),
        SlotValue::Scalar(Value::Boolean(true)),
        SlotValue::Scalar(Value::Integer(1.into())),
        SlotValue::Scalar(Value::Real("1.0".parse().unwrap())),
        SlotValue::Scalar(Value::Enumeration(EnumerationLiteralId::from_u128(1))),
        SlotValue::Scalar(Value::Reference(id(1))),
    ];
    assert_eq!(
        values
            .iter()
            .map(slot_digest)
            .collect::<BTreeSet<_>>()
            .len(),
        values.len()
    );
    let origins = [
        Origin::Declared(authored()),
        Origin::Declared(DeclaredOrigin::StandardLibrary {
            library: LibraryId::from_u128(1),
        }),
        Origin::Declared(DeclaredOrigin::Generated {
            generator: GeneratorId::from_u128(1),
        }),
        Origin::Declared(DeclaredOrigin::Transformation {
            transformation: TransformationId::from_u128(1),
            inputs: BTreeSet::from([id(1)]),
        }),
        Origin::Declared(DeclaredOrigin::ReviewedCorrection {
            profile: "p".into(),
            entry: "e".into(),
            authority: BTreeSet::from(["a".into()]),
            library: LibraryId::from_u128(1),
            source_key: "s".into(),
            output_key: "o".into(),
        }),
        Origin::AssociationOccurrences(BTreeSet::from([AssociationOccurrenceId::from_u128(1)])),
        Origin::Derived(Arc::new(evidence([]))),
    ];
    assert_eq!(
        origins
            .iter()
            .map(|origin| {
                let mut encoder = GraphEncoder::new(MODEL_SCHEMA);
                encoder.origin(origin);
                encoder.finish()
            })
            .collect::<BTreeSet<_>>()
            .len(),
        origins.len()
    );
}
