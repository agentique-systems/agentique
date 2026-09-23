use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use crate::producer_closure::{ProducerEvaluationTable, producer_reads};

const PROFILE: agq_kerml::BaselineProfile = agq_kerml::BaselineProfile::OPERATIONAL_V9;
const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.OwningTypeWriter");
const CREATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.OwningTypeCreator");
const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.OwningTypeReader");
const MUTATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.OwningTypeMutator");

fn fixture(carrier: MetaclassId, detached: bool) -> Snapshot {
    let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(PROFILE).unwrap()));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    for (element, class) in [
        (0, c::CLASSIFIER),
        (1, c::FEATURE),
        (2, c::FEATURE_REFERENCE_EXPRESSION),
        (10, c::CLASSIFIER),
    ] {
        f.create(element, class);
    }
    member(&mut f, 0, 1, 11, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 2, 12, carrier);
    if detached {
        f.owned.remove(&id(1));
    }
    f.finish()
}

fn options() -> SemanticOptions {
    SemanticOptions {
        baseline_profile: PROFILE,
        ..Default::default()
    }
}
fn context<'a>(snapshot: &'a Snapshot, registry: &ProducerRegistry) -> SemanticContext<'a> {
    SemanticContext::for_snapshot(snapshot, options(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
}
fn writer(scope: ProducerEffectScope) -> ProducerDescriptor {
    let mut descriptor = ProducerDescriptor::new(
        WRITER,
        [ProducerEffect::Typing],
        ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
    );
    descriptor.scope = scope;
    descriptor.scoped_fresh_ownership = true;
    descriptor
}

#[test]
fn immediate_owning_type_scope_excludes_containers_and_tracks_reparenting() {
    let snapshot = fixture(c::FEATURE_MEMBERSHIP, false);
    let registry =
        ProducerRegistry::new([writer(ProducerEffectScope::SubjectAndOwningType)]).unwrap();
    let before = context(&snapshot, &registry);
    assert_eq!(
        KerMlQueries::new(before.fork()).owning_type(id(2)).value,
        Some(id(1))
    );
    let certificate = ProducerClosureCertificate::initial(&before, &registry).unwrap();
    for target in [0, 1, 2, 10] {
        assert_eq!(
            certificate.is_closed(id(target), SemanticClosureRequirement::EffectiveTyping),
            !matches!(target, 1 | 2)
        );
    }
    let mut edit = snapshot.change_set();
    edit.clear(id(1), p::ELEMENT_OWNED_RELATIONSHIP);
    edit.set(
        id(10),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(12))]),
        origin(),
    );
    let changed = snapshot.apply(&edit).unwrap();
    let after = context(&changed, &registry);
    let rebound = certificate.rebind(&before, &after, &registry).unwrap();
    for target in [0, 1, 2, 10] {
        assert_eq!(
            rebound
                .certificate
                .is_closed(id(target), SemanticClosureRequirement::EffectiveTyping),
            !matches!(target, 2 | 10)
        );
    }
    for carrier in [c::OWNING_MEMBERSHIP, c::FEATURE_MEMBERSHIP] {
        let snapshot = fixture(carrier, true);
        let context = context(&snapshot, &registry);
        assert_eq!(
            KerMlQueries::new(context.fork()).owning_type(id(2)).value,
            None
        );
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert!(!certificate.is_closed(id(2), SemanticClosureRequirement::EffectiveTyping));
        assert!(certificate.is_closed(id(0), SemanticClosureRequirement::EffectiveTyping));
        assert!(certificate.is_closed(id(10), SemanticClosureRequirement::EffectiveTyping));
    }
}

#[test]
fn immediate_owner_scope_keeps_unknown_providers_and_owner_mutators_conservative() {
    for case in ["ownership", "reference_scalar", "pending_provider"] {
        let snapshot = fixture(c::FEATURE_MEMBERSHIP, case == "pending_provider");
        let mut descriptors = vec![writer(ProducerEffectScope::SubjectAndOwningType)];
        if case != "pending_provider" {
            let mut mutator = ProducerDescriptor::new(
                MUTATOR,
                [if case == "ownership" {
                    ProducerEffect::Ownership
                } else {
                    ProducerEffect::Scalar(p::FEATURE_FEATURE_TARGET)
                }],
                ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
            );
            mutator.scope = ProducerEffectScope::Subject;
            mutator.scoped_fresh_ownership = true;
            descriptors.push(mutator);
        }
        let registry = ProducerRegistry::new(descriptors).unwrap();
        let context = SemanticContext::for_project_snapshot(
            &snapshot,
            options(),
            BTreeSet::new(),
            BTreeSet::new(),
            if case == "pending_provider" {
                BTreeSet::from([id(10)])
            } else {
                BTreeSet::new()
            },
        )
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert!(
            !certificate.is_closed(id(10), SemanticClosureRequirement::EffectiveTyping),
            "{case}"
        );
    }
}

#[test]
fn future_immediate_owner_scope_keeps_transitive_owner_contracts_distinct() {
    let snapshot = fixture(c::FEATURE_MEMBERSHIP, false);
    for transitive in [false, true] {
        let mut creator = ProducerDescriptor::new(
            CREATOR,
            [],
            ProducerApplicability::Subtypes(vec![c::FEATURE_REFERENCE_EXPRESSION]),
        );
        creator.scope = ProducerEffectScope::Subject;
        creator.fresh_effects.insert(ProducerEffect::Membership);
        creator.scoped_fresh_ownership = true;
        let mut future = ProducerDescriptor::new(
            WRITER,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::BEHAVIOR]),
        );
        future.scope = if transitive {
            ProducerEffectScope::SubjectAndOwners
        } else {
            ProducerEffectScope::SubjectAndOwningType
        };
        future.scoped_fresh_ownership = true;
        let reader =
            ProducerDescriptor::new(READER, [], ProducerApplicability::Subtypes(vec![c::TYPE]));
        let registry = ProducerRegistry::new([creator, future, reader]).unwrap();
        let context = context(&snapshot, &registry);
        let q = KerMlQueries::for_production(context.fork());
        let mut table = ProducerEvaluationTable::default();
        for record in snapshot.model().elements() {
            table.pending(record.id(), snapshot.model(), &registry);
            for descriptor in registry.descriptors() {
                if !descriptor
                    .applicability
                    .applies(snapshot.model(), record.metaclass())
                {
                    continue;
                }
                table
                    .record(
                        &[(
                            record.id(),
                            descriptor.id,
                            if descriptor.id == CREATOR {
                                Completeness::Incomplete
                            } else {
                                Completeness::Complete
                            },
                        )],
                        &registry,
                    )
                    .unwrap();
                if descriptor.id == READER {
                    let answer = q.owned_relationships_of_type(record.id(), c::FEATURE_TYPING);
                    table.record_reads(
                        &[(
                            record.id(),
                            READER,
                            producer_reads(&answer, snapshot.model()),
                        )],
                        &registry,
                    );
                }
            }
        }
        let certificate = ProducerClosureCertificate::issue(
            snapshot.model(),
            context.id(),
            &registry,
            &table,
            |_| None,
        );
        for target in [0, 1, 2, 10] {
            let blocked = target == 2 || transitive && matches!(target, 0 | 1);
            assert_eq!(
                certificate.evaluation(id(target), registry.index(READER).unwrap()),
                Some(if blocked {
                    ProducerEvaluationState::Pending
                } else {
                    ProducerEvaluationState::EvaluatedComplete
                }),
                "target={target}, transitive={transitive}"
            );
            assert_eq!(
                certificate.is_closed(id(target), SemanticClosureRequirement::EffectiveTyping),
                !blocked
            );
        }
    }
}

#[test]
fn immediate_owner_audit_rejects_ancestor_writes_and_fresh_attachments() {
    let snapshot = fixture(c::FEATURE_MEMBERSHIP, false);
    let mut descriptor = writer(ProducerEffectScope::SubjectAndOwningType);
    descriptor.effects.insert(ProducerEffect::Membership);
    let registry = ProducerRegistry::new([descriptor]).unwrap();
    let q = KerMlQueries::new(context(&snapshot, &registry));
    for target in [0, 1, 2, 10] {
        let mut plan = q.plan_result_structure([]);
        let key = DerivationKey {
            subject: id(2),
            rule: RuleId::from_u128(950001),
            output: OutputKey::from_u128(target),
        };
        let output = plan
            .add_derived_element(
                key,
                c::FEATURE_TYPING,
                BTreeMap::from([
                    (
                        p::FEATURE_TYPING_TYPED_FEATURE,
                        SlotValue::Scalar(Value::Reference(id(target))),
                    ),
                    (
                        p::FEATURE_TYPING_TYPE,
                        SlotValue::Scalar(Value::Reference(id(10))),
                    ),
                ]),
                Some(id(target)),
                &q.canonical_fact_evidence(FactKey::Element(id(2))),
            )
            .unwrap();
        plan.attribute_producer_outputs(id(2), WRITER, output);
        assert_eq!(
            plan.validate_declared_effects(&[id(2)], &registry).is_ok(),
            matches!(target, 1 | 2),
            "target={target}"
        );
    }
}

#[test]
fn immediate_owner_scope_distinguishes_absent_canonical_inverse_from_unknown_alias() {
    use agq_kernel::derived::{
        ComputationFailure, DerivationBuilder, IncompleteReason, PropertyState,
    };
    use agq_kernel::metamodel::{MetamodelRegistry, PropertyOwner};
    let snapshot = fixture(c::FEATURE_MEMBERSHIP, false);
    assert!(
        matches!(snapshot.model().property_state(id(2),p::ELEMENT_OWNING_RELATIONSHIP).unwrap(),
        PropertyState::Computed(slot) if slot.value().values().any(|v| *v==Value::Reference(id(12))))
    );
    let mut edit = snapshot.change_set();
    edit.clear(id(1), p::ELEMENT_OWNED_RELATIONSHIP);
    edit.remove(id(12));
    let unowned = snapshot.apply(&edit).unwrap();
    assert!(matches!(
        unowned
            .model()
            .property_state(id(2), p::ELEMENT_OWNING_RELATIONSHIP)
            .unwrap(),
        PropertyState::Absent
    ));
    let registry =
        ProducerRegistry::new([writer(ProducerEffectScope::SubjectAndOwningType)]).unwrap();
    let certificate =
        ProducerClosureCertificate::initial(&context(&unowned, &registry), &registry).unwrap();
    assert!(certificate.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));

    let custom_class = MetaclassId::from_u128(0xfea301);
    let alias = PropertyId::from_u128(0xfea302);
    let mut descriptors = agq_kerml::descriptors_for_profile(PROFILE).unwrap();
    let mut class = descriptors
        .classes
        .iter()
        .find(|class| class.id == c::FEATURE_REFERENCE_EXPRESSION)
        .unwrap()
        .clone();
    class.id = custom_class;
    class.name = "FixtureComputedOwner".into();
    class.direct_supertypes = BTreeSet::from([c::FEATURE_REFERENCE_EXPRESSION]);
    descriptors.classes.push(class);
    let mut property = descriptors
        .properties
        .iter()
        .find(|property| property.id == p::ELEMENT_OWNING_RELATIONSHIP)
        .unwrap()
        .clone();
    property.id = alias;
    property.name = "fixtureComputedOwner".into();
    property.owner = PropertyOwner::Class(custom_class);
    property.derived = true;
    property.redefines = BTreeSet::from([p::ELEMENT_OWNING_RELATIONSHIP]);
    property.association = None;
    property.opposite_ends.clear();
    descriptors.properties.push(property);
    let base = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    let mut f = Fixture {
        changes: base.change_set(),
        base,
        owned: BTreeMap::new(),
    };
    f.create(1, c::CLASSIFIER);
    f.create(2, custom_class);
    f.create(10, c::CLASSIFIER);
    member(&mut f, 1, 2, 12, c::FEATURE_MEMBERSHIP);
    let snapshot = f.finish();
    assert!(matches!(
        snapshot
            .model()
            .property_state(id(2), p::ELEMENT_OWNING_RELATIONSHIP)
            .unwrap(),
        PropertyState::NotComputed
    ));
    for state in [0, 1, 2] {
        let mut builder = DerivationBuilder::new(snapshot.clone());
        let explanation = agq_kernel::provenance::Explanation {
            rule: RuleId::from_u128(0xfea303),
            dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(id(2)))]),
        };
        if state != 0 {
            builder
                .failure(
                    id(2),
                    alias,
                    if state == 1 {
                        ComputationFailure::Incomplete {
                            reason: IncompleteReason::MissingInput,
                            explanation,
                            searches: BTreeSet::new(),
                        }
                    } else {
                        ComputationFailure::Invalid {
                            diagnostic: "fixture invalid inverse".into(),
                            explanation,
                            searches: BTreeSet::new(),
                        }
                    },
                )
                .unwrap();
        }
        let overlay = builder.build().unwrap();
        let context = SemanticContext::for_overlay(&overlay, options(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert!(
            !certificate.is_closed(id(10), SemanticClosureRequirement::EffectiveTyping),
            "unknown inverse state={state}"
        );
    }
}
