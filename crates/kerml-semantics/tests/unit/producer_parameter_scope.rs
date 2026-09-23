use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

use crate::producer_closure::{ProducerEvaluationTable, ProducerRead};

const WRITER: ProducerFamilyId = ProducerFamilyId::new("Fixture.DirectParameterWriter");
const READER: ProducerFamilyId = ProducerFamilyId::new("Fixture.DirectParameterReader");
const MUTATOR: ProducerFamilyId = ProducerFamilyId::new("Fixture.ParameterMutator");

fn input_value(model: &ModelView) -> Value {
    let ValueKind::Enumeration(domain) = model
        .registry()
        .property(p::FEATURE_DIRECTION)
        .unwrap()
        .value_kind
    else {
        panic!("direction enumeration");
    };
    Value::Enumeration(
        *model
            .registry()
            .enumeration(domain)
            .unwrap()
            .literals
            .iter()
            .find(|(_, name)| name.as_str() == "in")
            .unwrap()
            .0,
    )
}

fn scope_fixture() -> Snapshot {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    for feature in 2..=6 {
        f.create(feature, c::FEATURE);
    }
    member(&mut f, 1, 2, 12, c::FEATURE_MEMBERSHIP);
    member(&mut f, 1, 3, 13, c::FEATURE_MEMBERSHIP);
    member(&mut f, 3, 4, 34, c::PARAMETER_MEMBERSHIP);
    member(&mut f, 1, 5, 15, c::RETURN_PARAMETER_MEMBERSHIP);
    for feature in [2, 4, 5] {
        let direction = input_value(f.base.model());
        f.value(feature, p::FEATURE_DIRECTION, direction);
    }
    f.finish()
}

fn writer() -> ProducerDescriptor {
    let mut writer = ProducerDescriptor::new(
        WRITER,
        [ProducerEffect::Subsetting],
        ProducerApplicability::Subtypes(vec![c::CLASSIFIER]),
    );
    writer.scope = ProducerEffectScope::OwnedParameterFeatures;
    writer
}

fn context<'a>(snapshot: &'a Snapshot, registry: &ProducerRegistry) -> SemanticContext<'a> {
    SemanticContext::for_snapshot(snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
}

#[test]
fn direct_parameter_scope_closes_unrelated_readers_and_audits_actual_write_targets() {
    let snapshot = scope_fixture();
    let registry = ProducerRegistry::new([
        writer(),
        ProducerDescriptor::new(
            READER,
            [ProducerEffect::Typing],
            ProducerApplicability::Subtypes(vec![c::FEATURE]),
        ),
    ])
    .unwrap();
    let context = context(&snapshot, &registry);
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), &registry);
    }
    for feature in 2..=6 {
        table
            .record(&[(id(feature), READER, Completeness::Complete)], &registry)
            .unwrap();
        table.record_reads(
            &[(
                id(feature),
                READER,
                vec![ProducerRead::Owned(id(feature), c::SUBSETTING)].into(),
            )],
            &registry,
        );
    }
    let certificate = ProducerClosureCertificate::issue(
        snapshot.model(),
        context.id(),
        &registry,
        &table,
        |_| None,
    );
    for feature in 2..=6 {
        assert_eq!(
            certificate.is_closed(id(feature), SemanticClosureRequirement::EffectiveTyping),
            feature != 2
        );
        assert_eq!(
            certificate.evaluation(id(feature), registry.index(READER).unwrap()),
            Some(if feature == 2 {
                ProducerEvaluationState::Pending
            } else {
                ProducerEvaluationState::EvaluatedComplete
            })
        );
    }
    let q = KerMlQueries::new(context);
    assert_eq!(q.owned_parameter_features(id(1)).value, [id(2)]);
    for target in 1..=5 {
        let mut plan = q.plan_result_structure([]);
        plan.add_derived_element(
            DerivationKey {
                subject: id(1),
                rule: RuleId::from_u128(1000),
                output: OutputKey::from_u128(target),
            },
            c::SUBSETTING,
            BTreeMap::from([
                (
                    p::SUBSETTING_SUBSETTING_FEATURE,
                    SlotValue::Scalar(Value::Reference(id(target))),
                ),
                (
                    p::SUBSETTING_SUBSETTED_FEATURE,
                    SlotValue::Scalar(Value::Reference(id(6))),
                ),
            ]),
            Some(id(target)),
            &q.canonical_fact_evidence(FactKey::Element(id(target))),
        )
        .unwrap();
        assert_eq!(
            plan.validate_declared_effects(&[id(1)], &ProducerRegistry::new([writer()]).unwrap())
                .is_ok(),
            target == 2
        );
    }
}

#[test]
fn primitive_direction_writers_widen_only_existing_direct_members() {
    let snapshot = scope_fixture();
    for (existing, applicable) in [(true, true), (false, true), (true, false)] {
        let mut mutator = ProducerDescriptor::new(
            MUTATOR,
            [],
            if applicable {
                ProducerApplicability::Any
            } else {
                ProducerApplicability::Never
            },
        );
        if existing {
            mutator
                .effects
                .insert(ProducerEffect::Scalar(p::FEATURE_DIRECTION));
        } else {
            mutator
                .fresh_effects
                .insert(ProducerEffect::Scalar(p::FEATURE_DIRECTION));
        }
        let registry = ProducerRegistry::new([writer(), mutator]).unwrap();
        let context = context(&snapshot, &registry);
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert_eq!(
            certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping),
            !(existing && applicable)
        );
        assert!(
            certificate.is_closed(id(4), SemanticClosureRequirement::EffectiveTyping),
            "direction cannot reown a nested parameter"
        );
    }
}

#[test]
fn future_ownership_and_reference_writers_prevent_parameter_scope_exclusion() {
    let snapshot = scope_fixture();
    for effect in [
        ProducerEffect::Ownership,
        ProducerEffect::Scalar(p::RELATIONSHIP_OWNED_RELATED_ELEMENT),
    ] {
        let mut mutator = ProducerDescriptor::new(MUTATOR, [effect], ProducerApplicability::Any);
        mutator.scope = ProducerEffectScope::Model;
        let registry = ProducerRegistry::new([writer(), mutator]).unwrap();
        let context = context(&snapshot, &registry);
        let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
        assert!(!certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
        assert!(!certificate.is_closed(id(4), SemanticClosureRequirement::EffectiveTyping));
    }
}

#[test]
fn missing_member_endpoint_cannot_close_an_unidentified_parameter() {
    let snapshot = scope_fixture();
    let mut edit = snapshot.change_set();
    edit.clear(id(12), p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
    let construction = snapshot.preview(&edit).unwrap();
    let registry = ProducerRegistry::new([writer()]).unwrap();
    let context =
        SemanticContext::for_construction(&construction, Default::default(), BTreeSet::new())
            .unwrap()
            .with_producer_registry_digest(registry.digest())
            .unwrap();
    let certificate = ProducerClosureCertificate::initial(&context, &registry).unwrap();
    assert!(!certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
    assert!(
        crate::producer_closure::owned_parameter_scope(construction.model(), id(1), false).is_err()
    );
    let protected = ProducerClosureCertificate::issue(
        construction.model(),
        context.id(),
        &registry,
        &ProducerEvaluationTable::default(),
        |subject| (subject == id(6)).then_some(ClosureSource::AcceptedDependency),
    );
    assert!(protected.is_closed(id(6), SemanticClosureRequirement::EffectiveTyping));
    assert!(!protected.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
}

#[test]
fn reconstructed_direction_and_ownership_recompute_pending_parameter_write_scope() {
    let snapshot = scope_fixture();
    let registry = ProducerRegistry::new([writer()]).unwrap();
    let before = context(&snapshot, &registry);
    let certificate = ProducerClosureCertificate::initial(&before, &registry).unwrap();
    assert!(certificate.is_closed(id(3), SemanticClosureRequirement::EffectiveTyping));
    assert!(certificate.is_closed(id(4), SemanticClosureRequirement::EffectiveTyping));
    let mut direction = snapshot.change_set();
    direction.set(
        id(3),
        p::FEATURE_DIRECTION,
        SlotValue::Scalar(input_value(snapshot.model())),
        origin(),
    );
    let directed = snapshot.apply(&direction).unwrap();
    let after = context(&directed, &registry);
    let rebound = certificate.rebind(&before, &after, &registry).unwrap();
    assert!(
        !rebound
            .certificate
            .is_closed(id(3), SemanticClosureRequirement::EffectiveTyping)
    );
    assert!(
        rebound
            .certificate
            .is_closed(id(4), SemanticClosureRequirement::EffectiveTyping)
    );
    let mut ownership = snapshot.change_set();
    ownership.set(
        id(1),
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered([12, 13, 15, 34].map(|n| Value::Reference(id(n))).into()),
        origin(),
    );
    ownership.clear(id(3), p::ELEMENT_OWNED_RELATIONSHIP);
    let adopted = snapshot.apply(&ownership).unwrap();
    let after = context(&adopted, &registry);
    let rebound = certificate.rebind(&before, &after, &registry).unwrap();
    assert!(
        !rebound
            .certificate
            .is_closed(id(4), SemanticClosureRequirement::EffectiveTyping)
    );
}

#[test]
fn adding_parameter_scope_preserves_existing_scope_discriminants() {
    assert_eq!(ProducerEffectScope::Subject as u8, 0);
    assert_eq!(ProducerEffectScope::SubjectAndOwned as u8, 1);
    assert_eq!(ProducerEffectScope::OwnedDescendants as u8, 2);
    assert_eq!(ProducerEffectScope::SubjectAndOwners as u8, 3);
    assert_eq!(ProducerEffectScope::Model as u8, 4);
}
