//! SysML contributions to the shared semantic worklist. This module plans facts;
//! it never scans or mutates the accepted KerML publication.
use crate::{
    StandardSysmlBindings, StandardSysmlRole as R, SysmlBaselineProfile, SystemsLibraryIdentity,
};
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{
    Completeness, Diagnostic, FormalConstraintId, KerMlQueries, QueryResult, SearchDependency,
    StandardLibraryArtifact, StandardRole,
};
use agq_kernel::{
    DerivationKey, ElementId, LibraryId, MetaclassId, PropertyId, RuleId,
    derived::PropertyState,
    provenance::{DeclaredOrigin, FactKey, Origin},
    value::{SlotValue, Value},
};
use agq_sysml::{classes as sc, properties as sp};
use std::collections::{BTreeMap, BTreeSet};

/// A relationship proposal uses canonical subjects and targets, never names as IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysmlRelationshipProposal {
    pub key: DerivationKey,
    pub metaclass: MetaclassId,
    pub specific: ElementId,
    pub general: ElementId,
    pub source_property: PropertyId,
    pub target_property: PropertyId,
}
impl SysmlRelationshipProposal {
    /// Inputs to the shared canonical relationship materializer.
    pub fn slots(&self) -> BTreeMap<PropertyId, SlotValue> {
        BTreeMap::from([
            (
                self.source_property,
                SlotValue::Scalar(Value::Reference(self.specific)),
            ),
            (
                self.target_property,
                SlotValue::Scalar(Value::Reference(self.general)),
            ),
        ])
    }
}

/// Derived scalar facts are kept separate from monotone structural contributions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysmlPropertyProposal {
    pub subject: ElementId,
    pub property: PropertyId,
    pub value: SlotValue,
    pub rule: RuleId,
}

/// Canonical structural synthesis whose ordered ownership is part of the rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SysmlElementProposal {
    pub key: DerivationKey,
    pub metaclass: MetaclassId,
    pub slots: BTreeMap<PropertyId, SlotValue>,
    pub owner: Option<ElementId>,
}

/// One normative rule evaluation, including all positive and negative reads.
/// Incomplete evidence never authorizes any of this result's proposals.
#[derive(Clone, Debug)]
pub struct SysmlProducerResult {
    pub rule: &'static str,
    pub evidence: QueryResult<()>,
    pub relationships: Vec<SysmlRelationshipProposal>,
    pub properties: Vec<SysmlPropertyProposal>,
    pub elements: Vec<SysmlElementProposal>,
}

/// Per-subject frontier contribution. The shared scheduler owns dirty propagation.
#[derive(Clone, Debug)]
pub struct SysmlProducerPlan {
    pub subject: ElementId,
    pub results: Vec<SysmlProducerResult>,
}
impl SysmlProducerPlan {
    pub fn completeness(&self) -> Completeness {
        self.results
            .iter()
            .map(|r| r.evidence.completeness)
            .max()
            .unwrap_or(Completeness::Complete)
    }
}

/// Applicability is structural. No library declaration name selects a producer.
pub fn sysml_producers_apply(model: &agq_kernel::ModelView, class: MetaclassId) -> bool {
    [sc::DEFINITION, sc::USAGE]
        .iter()
        .any(|base| model.registry().is_subtype(class, *base).unwrap_or(false))
}

/// Finite implemented producer identities for publication provenance auditing.
/// Formal operations without a materializer are deliberately absent.
pub fn sysml_producer_rule_ids(profile: SysmlBaselineProfile) -> BTreeSet<RuleId> {
    BASE_RULES
        .iter()
        .map(|(_, rule, _)| *rule)
        .chain(COMPOSITE_RULES.iter().map(|(_, _, rule, _)| *rule))
        .chain(OWNED_RULES.iter().map(|(_, _, rule, _)| *rule))
        .chain(SUBACTION_RULES.iter().map(|(_, rule, _)| *rule))
        .chain([
            "checkAcceptActionUsageTriggerActionSpecialization",
            "checkAcceptActionUsageSpecialization",
            "checkAcceptActionUsageSubactionSpecialization",
            "checkStateUsageSubstateSpecialization",
            "checkStateUsageExclusiveStateSpecialization",
            "checkTransitionUsageStateSpecialization",
            "checkTransitionUsageActionSpecialization",
            "checkOccurrenceUsageSuboccurrenceSpecialization",
            "checkConnectionDefinitionBinarySpecialization",
            "checkConnectionUsageBinarySpecialization",
            "checkInterfaceDefinitionBinarySpecialization",
            "checkInterfaceUsageBinarySpecialization",
            "checkFlowUsageFlowSpecialization",
            "deriveUsageMayTimeVary",
            "checkTransitionUsagePayloadSpecialization",
        ])
        .map(|rule| profile.rule_id(rule))
        .collect()
}

/// Declared effects and structural applicability of every implemented family.
/// This inventory is independent of which facts a particular execution emits.
pub fn sysml_producer_descriptors() -> Vec<agq_kerml_semantics::ProducerDescriptor> {
    use agq_kerml_semantics::{
        ProducerApplicability, ProducerDescriptor, ProducerEffect, ProducerFamilyId,
        ResultStructureStratum,
    };
    let mut families = BTreeMap::<&'static str, Vec<MetaclassId>>::new();
    for &(class, rule, _) in BASE_RULES.iter().chain(SUBACTION_RULES) {
        families.entry(rule).or_default().push(class);
    }
    for &(class, _, rule, _) in COMPOSITE_RULES.iter().chain(OWNED_RULES) {
        families.entry(rule).or_default().push(class);
    }
    for (class, rules) in [
        (
            sc::ACCEPT_ACTION_USAGE,
            &[
                "checkAcceptActionUsageTriggerActionSpecialization",
                "checkAcceptActionUsageSpecialization",
                "checkAcceptActionUsageSubactionSpecialization",
            ][..],
        ),
        (
            sc::STATE_USAGE,
            &[
                "checkStateUsageSubstateSpecialization",
                "checkStateUsageExclusiveStateSpecialization",
            ][..],
        ),
        (
            sc::TRANSITION_USAGE,
            &[
                "checkTransitionUsageStateSpecialization",
                "checkTransitionUsageActionSpecialization",
                "checkTransitionUsagePayloadSpecialization",
            ][..],
        ),
        (
            sc::OCCURRENCE_USAGE,
            &["checkOccurrenceUsageSuboccurrenceSpecialization"][..],
        ),
        (
            sc::CONNECTION_DEFINITION,
            &["checkConnectionDefinitionBinarySpecialization"][..],
        ),
        (
            sc::CONNECTION_USAGE,
            &["checkConnectionUsageBinarySpecialization"][..],
        ),
        (
            sc::INTERFACE_DEFINITION,
            &["checkInterfaceDefinitionBinarySpecialization"][..],
        ),
        (
            sc::INTERFACE_USAGE,
            &["checkInterfaceUsageBinarySpecialization"][..],
        ),
        (sc::FLOW_USAGE, &["checkFlowUsageFlowSpecialization"][..]),
        (sc::USAGE, &["deriveUsageMayTimeVary"][..]),
    ] {
        for &rule in rules {
            families.entry(rule).or_default().push(class);
        }
    }
    families
        .into_iter()
        .map(|(rule, mut classes)| {
            classes.sort();
            classes.dedup();
            let effects = match rule {
                "deriveUsageMayTimeVary" => vec![
                    ProducerEffect::Scalar(kp::FEATURE_IS_VARIABLE),
                    ProducerEffect::Scalar(sp::USAGE_MAY_TIME_VARY),
                ],
                "checkTransitionUsagePayloadSpecialization" => vec![ProducerEffect::Subsetting],
                // A Usage contributes Subsetting; a Definition contributes
                // Subclassification. Both are Specialization relationships.
                _ => vec![ProducerEffect::Specialization, ProducerEffect::Subsetting],
            };
            let mut descriptor = ProducerDescriptor::new(
                ProducerFamilyId::new(rule),
                effects,
                ProducerApplicability::Subtypes(classes),
            );
            if rule == "checkTransitionUsagePayloadSpecialization" {
                // Only the second input parameter receives a new Subsetting.
                // FeatureChaining belongs to the newly created payload chain,
                // and cannot change the transition's own effective typing.
                descriptor.scope = agq_kerml_semantics::ProducerEffectScope::OwnedParameterFeatures;
                descriptor.fresh_effects = BTreeSet::from([
                    ProducerEffect::FeatureChain,
                    ProducerEffect::ResultStructure,
                ]);
                descriptor.relationship_classes =
                    Some(BTreeSet::from([kc::SUBSETTING, kc::FEATURE_CHAINING]));
            } else {
                descriptor.scope = agq_kerml_semantics::ProducerEffectScope::Subject;
            }
            if rule == "deriveUsageMayTimeVary" {
                descriptor.minimum_stratum = ResultStructureStratum::StableProperties;
            }
            descriptor
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Target {
    Sysml(R),
    KerMl(StandardRole),
    Suboccurrences,
    TimeEnclosedOccurrences,
    PublishedSubitem,
    MissingViewpoint,
    MissingViewpoints,
    MissingBinaryConnections,
}

/// Implemented positive base and composite implications. Roots are supplied by
/// the project/publication boundary; no whole-model discovery scan is performed.
pub fn plan_sysml_producers(
    queries: &KerMlQueries<'_>,
    profile: SysmlBaselineProfile,
    bindings: &StandardSysmlBindings,
    roots: &[ElementId],
    subject: ElementId,
) -> SysmlProducerPlan {
    let evaluator = Evaluator {
        queries,
        profile,
        bindings,
        roots,
    };
    let mut plan = SysmlProducerPlan {
        subject,
        results: vec![],
    };
    if !queries
        .model()
        .element(subject)
        .is_some_and(|r| sysml_producers_apply(queries.model(), r.metaclass()))
    {
        return plan;
    }
    for &(class, rule, target) in BASE_RULES {
        if evaluator.is(subject, class) {
            let result = evaluator.result(subject, rule);
            plan.results
                .push(evaluator.specialize(result, subject, target));
        }
    }
    for &(class, owner_classes, rule, target) in COMPOSITE_RULES {
        if !evaluator.is(subject, class) {
            continue;
        }
        let mut result = evaluator.result(subject, rule);
        let composite = evaluator.boolean(&mut result.evidence, subject, kp::FEATURE_IS_COMPOSITE);
        if composite == Some(true) {
            let owner = queries.owning_type(subject);
            let owner_id = owner.value;
            result
                .evidence
                .merge_evidence(owner)
                .expect("same producer context");
            if let Some(owner) = owner_id {
                result
                    .evidence
                    .merge_evidence(queries.canonical_fact_evidence(FactKey::Element(owner)))
                    .expect("same producer context");
                if owner_classes
                    .iter()
                    .any(|class| evaluator.is(owner, *class))
                {
                    let target = if matches!(target, Target::PublishedSubitem)
                        && matches!(
                            profile,
                            SysmlBaselineProfile::OPERATIONAL_V1
                                | SysmlBaselineProfile::OPERATIONAL_V2
                        ) {
                        Target::Sysml(R::Subitems)
                    } else {
                        target
                    };
                    result = evaluator.specialize(result, subject, target);
                }
            }
        }
        plan.results.push(result);
    }
    for &(class, owner_classes, rule, target) in OWNED_RULES {
        if !evaluator.is(subject, class) {
            continue;
        }
        let mut result = evaluator.result(subject, rule);
        let owner = queries.owning_type(subject);
        let owner_id = owner.value;
        result
            .evidence
            .merge_evidence(owner)
            .expect("same producer context");
        if let Some(owner) = owner_id {
            result
                .evidence
                .merge_evidence(queries.canonical_fact_evidence(FactKey::Element(owner)))
                .expect("same producer context");
            if owner_classes
                .iter()
                .any(|class| evaluator.is(owner, *class))
            {
                result = evaluator.specialize(result, subject, target);
            }
        }
        plan.results.push(result);
    }
    if evaluator.is(subject, sc::ACTION_USAGE) {
        let predicate = evaluator.subaction(subject);
        for &(class, rule, target) in SUBACTION_RULES {
            if !evaluator.is(subject, class) {
                continue;
            }
            let mut result = evaluator.result(subject, rule);
            result
                .evidence
                .merge_evidence(predicate.clone())
                .expect("same producer context");
            if predicate.value {
                result = evaluator.specialize(result, subject, target);
            }
            plan.results.push(result);
        }
    }
    if evaluator.is(subject, sc::ACCEPT_ACTION_USAGE) {
        let trigger = evaluator.trigger_action(subject);
        // Both alternatives are evaluated against the same predicate. Retaining
        // the inactive result lets the scheduler distinguish a closed family
        // from a family that has not been evaluated on this frontier.
        let mut inactive = evaluator.result(
            subject,
            if trigger.value {
                "checkAcceptActionUsageSpecialization"
            } else {
                "checkAcceptActionUsageTriggerActionSpecialization"
            },
        );
        inactive
            .evidence
            .merge_evidence(trigger.clone())
            .expect("same producer context");
        plan.results.push(inactive);
        let mut result = evaluator.result(
            subject,
            if trigger.value {
                "checkAcceptActionUsageTriggerActionSpecialization"
            } else {
                "checkAcceptActionUsageSpecialization"
            },
        );
        result
            .evidence
            .merge_evidence(trigger.clone())
            .expect("same producer context");
        result = evaluator.specialize(
            result,
            subject,
            Target::Sysml(if trigger.value {
                R::TransitionAccepter
            } else {
                R::AcceptActions
            }),
        );
        plan.results.push(result);
        let mut subaction =
            evaluator.result(subject, "checkAcceptActionUsageSubactionSpecialization");
        subaction
            .evidence
            .merge_evidence(trigger.clone())
            .expect("same producer context");
        if !trigger.value {
            let predicate = evaluator.subaction(subject);
            let applicable = predicate.value;
            subaction
                .evidence
                .merge_evidence(predicate)
                .expect("same producer context");
            if applicable {
                subaction =
                    evaluator.specialize(subaction, subject, Target::Sysml(R::AcceptSubactions));
            }
        }
        plan.results.push(subaction);
    }
    if evaluator.is(subject, sc::STATE_USAGE) {
        for (parallel, rule, target) in [
            (true, "checkStateUsageSubstateSpecialization", R::Substates),
            (
                false,
                "checkStateUsageExclusiveStateSpecialization",
                R::ExclusiveStates,
            ),
        ] {
            let mut result = evaluator.result(subject, rule);
            let predicate = evaluator.substate(subject, parallel);
            let applicable = predicate.value;
            result
                .evidence
                .merge_evidence(predicate)
                .expect("same producer context");
            if applicable {
                result = evaluator.specialize(result, subject, Target::Sysml(target));
            }
            plan.results.push(result);
        }
    }
    if evaluator.is(subject, sc::TRANSITION_USAGE) {
        let mut source = None;
        let mut premise = evaluator
            .result(subject, "transitionSpecializationAntecedent")
            .evidence;
        if evaluator.boolean(&mut premise, subject, kp::FEATURE_IS_COMPOSITE) == Some(true) {
            let source_query = evaluator.transition_source(subject);
            source = source_query.value;
            premise
                .merge_evidence(source_query)
                .expect("same producer context");
        }
        for (state, classes, rule, target) in [
            (
                true,
                [sc::STATE_DEFINITION, sc::STATE_USAGE],
                "checkTransitionUsageStateSpecialization",
                R::StateTransitions,
            ),
            (
                false,
                [sc::ACTION_DEFINITION, sc::ACTION_USAGE],
                "checkTransitionUsageActionSpecialization",
                R::DecisionTransitions,
            ),
        ] {
            let mut result = evaluator.result(subject, rule);
            result
                .evidence
                .merge_evidence(premise.clone())
                .expect("same producer context");
            if let Some(source) = source {
                result
                    .evidence
                    .merge_evidence(queries.canonical_fact_evidence(FactKey::Element(source)))
                    .expect("same producer context");
                if evaluator.is(source, sc::STATE_USAGE) == state {
                    let owner = queries.owning_type(subject);
                    let owner_id = owner.value;
                    result
                        .evidence
                        .merge_evidence(owner)
                        .expect("same producer context");
                    if let Some(owner) = owner_id {
                        result
                            .evidence
                            .merge_evidence(
                                queries.canonical_fact_evidence(FactKey::Element(owner)),
                            )
                            .expect("same producer context");
                        if classes.iter().any(|class| evaluator.is(owner, *class)) {
                            result = evaluator.specialize(result, subject, Target::Sysml(target));
                        }
                    }
                }
            }
            plan.results.push(result);
        }
    }
    if evaluator.is(subject, sc::OCCURRENCE_USAGE) {
        let mut result =
            evaluator.result(subject, "checkOccurrenceUsageSuboccurrenceSpecialization");
        if evaluator.boolean(&mut result.evidence, subject, kp::FEATURE_IS_COMPOSITE) == Some(true)
        {
            let owner = queries.owning_type(subject);
            let owner_id = owner.value;
            result
                .evidence
                .merge_evidence(owner)
                .expect("same producer context");
            if let Some(owner) = owner_id {
                result
                    .evidence
                    .merge_evidence(queries.canonical_fact_evidence(FactKey::Element(owner)))
                    .expect("same producer context");
                let mut applicable =
                    evaluator.is(owner, kc::CLASS) || evaluator.is(owner, sc::OCCURRENCE_USAGE);
                if !applicable && evaluator.is(owner, kc::FEATURE) {
                    let types = queries.feature_types(owner);
                    for &ty in &types.value {
                        result
                            .evidence
                            .merge_evidence(queries.canonical_fact_evidence(FactKey::Element(ty)))
                            .expect("same producer context");
                        applicable |= evaluator.is(ty, kc::CLASS);
                    }
                    result
                        .evidence
                        .merge_evidence(types)
                        .expect("same producer context");
                }
                if applicable {
                    result = evaluator.specialize(result, subject, Target::Suboccurrences);
                }
            }
        }
        plan.results.push(result);
    }
    for (
        definition_class,
        usage_class,
        definition_rule,
        usage_rule,
        definition_target,
        usage_target,
    ) in [
        (
            sc::CONNECTION_DEFINITION,
            sc::CONNECTION_USAGE,
            "checkConnectionDefinitionBinarySpecialization",
            "checkConnectionUsageBinarySpecialization",
            Target::MissingBinaryConnections,
            Target::Sysml(R::BinaryConnections),
        ),
        (
            sc::INTERFACE_DEFINITION,
            sc::INTERFACE_USAGE,
            "checkInterfaceDefinitionBinarySpecialization",
            "checkInterfaceUsageBinarySpecialization",
            Target::Sysml(R::BinaryInterface),
            Target::Sysml(R::BinaryInterfaces),
        ),
    ] {
        if !evaluator.is(subject, definition_class) && !evaluator.is(subject, usage_class) {
            continue;
        }
        let definition = evaluator.is(subject, definition_class);
        let mut result = evaluator.result(
            subject,
            if definition {
                definition_rule
            } else {
                usage_rule
            },
        );
        let features = queries.owned_end_features(subject);
        let ends = features.value.len();
        result
            .evidence
            .merge_evidence(features)
            .expect("same producer context");
        if ends == 2 {
            result = evaluator.specialize(
                result,
                subject,
                if definition {
                    definition_target
                } else {
                    usage_target
                },
            );
        }
        plan.results.push(result);
    }
    if evaluator.is(subject, sc::FLOW_USAGE) {
        let mut result = evaluator.result(subject, "checkFlowUsageFlowSpecialization");
        let features = queries.owned_end_features(subject);
        let has_end = !features.value.is_empty();
        result
            .evidence
            .merge_evidence(features)
            .expect("same producer context");
        if has_end {
            result = evaluator.specialize(result, subject, Target::Sysml(R::Flows));
        }
        plan.results.push(result);
    }
    if evaluator.is(subject, sc::TRANSITION_USAGE) {
        plan.results
            .push(crate::transition::plan_transition_payload(
                queries, profile, subject,
            ));
    }
    // A more specific proposed standard edge can already establish a broader
    // requirement through the target's existing ancestry. Retain that proof and
    // its reads instead of materializing a redundant second relationship.
    let mut proposals: Vec<_> = plan
        .results
        .iter()
        .filter_map(|result| {
            result
                .relationships
                .first()
                .map(|p| (p.general, result.evidence.clone()))
        })
        .collect();
    // The smallest reachable candidate is retained: any candidate capable of
    // suppressing it would be smaller and reachable by the same canonical path.
    proposals.sort_by_key(|(general, _)| *general);
    for result in &mut plan.results {
        let Some(general) = result.relationships.first().map(|p| p.general) else {
            continue;
        };
        for (other, antecedents) in &proposals {
            let other = *other;
            // A strictly decreasing target identity keeps even equivalent
            // bases acyclic without claiming that a reverse path is absent.
            if other >= general {
                continue;
            }
            if let Some(witness) = queries.canonical_specialization_witness(other, general) {
                result
                    .evidence
                    .merge_evidence(witness)
                    .expect("same producer context");
                result
                    .evidence
                    .merge_evidence(antecedents.clone())
                    .expect("same producer context");
                result.relationships.clear();
                break;
            }
        }
    }
    plan
}

const BASE_RULES: &[(MetaclassId, &str, Target)] = &[
    (
        sc::ASSIGNMENT_ACTION_USAGE,
        "checkAssignmentActionUsageSpecialization",
        Target::Sysml(R::AssignmentActions),
    ),
    (
        sc::WHILE_LOOP_ACTION_USAGE,
        "checkWhileLoopActionUsageSpecialization",
        Target::Sysml(R::WhileLoopActions),
    ),
    (
        sc::TRANSITION_USAGE,
        "checkTransitionUsageSpecialization",
        Target::Sysml(R::TransitionActions),
    ),
    (
        sc::ATTRIBUTE_USAGE,
        "checkAttributeUsageSpecialization",
        Target::KerMl(StandardRole::DataValues),
    ),
    (
        sc::OCCURRENCE_USAGE,
        "checkOccurrenceUsageSpecialization",
        Target::KerMl(StandardRole::Occurrences),
    ),
    (
        sc::ITEM_DEFINITION,
        "checkItemDefinitionSpecialization",
        Target::Sysml(R::Item),
    ),
    (
        sc::ITEM_USAGE,
        "checkItemUsageSpecialization",
        Target::Sysml(R::Items),
    ),
    (
        sc::PART_DEFINITION,
        "checkPartDefinitionSpecialization",
        Target::Sysml(R::Part),
    ),
    (
        sc::PART_USAGE,
        "checkPartUsageSpecialization",
        Target::Sysml(R::Parts),
    ),
    (
        sc::PORT_DEFINITION,
        "checkPortDefinitionSpecialization",
        Target::Sysml(R::Port),
    ),
    (
        sc::PORT_USAGE,
        "checkPortUsageSpecialization",
        Target::Sysml(R::Ports),
    ),
    (
        sc::CONNECTION_DEFINITION,
        "checkConnectionDefinitionSpecializations",
        Target::Sysml(R::Connection),
    ),
    (
        sc::CONNECTION_USAGE,
        "checkConnectionUsageSpecialization",
        Target::Sysml(R::Connections),
    ),
    (
        sc::INTERFACE_DEFINITION,
        "checkInterfaceDefinitionSpecialization",
        Target::Sysml(R::Interface),
    ),
    (
        sc::INTERFACE_USAGE,
        "checkInterfaceUsageSpecialization",
        Target::Sysml(R::Interfaces),
    ),
    (
        sc::FLOW_DEFINITION,
        "checkFlowDefinitionSpecialization",
        Target::Sysml(R::MessageAction),
    ),
    (
        sc::FLOW_USAGE,
        "checkFlowUsageSpecialization",
        Target::Sysml(R::Messages),
    ),
    (
        sc::SUCCESSION_FLOW_USAGE,
        "checkSuccessionFlowUsageSpecialization",
        Target::Sysml(R::SuccessionFlows),
    ),
    (
        sc::ACTION_DEFINITION,
        "checkActionDefinitionSpecialization",
        Target::Sysml(R::Action),
    ),
    (
        sc::ACTION_USAGE,
        "checkActionUsageSpecialization",
        Target::Sysml(R::Actions),
    ),
    (
        sc::STATE_DEFINITION,
        "checkStateDefinitionSpecialization",
        Target::Sysml(R::StateAction),
    ),
    (
        sc::STATE_USAGE,
        "checkStateUsageSpecialization",
        Target::Sysml(R::StateActions),
    ),
    (
        sc::CALCULATION_DEFINITION,
        "checkCalculationDefinitionSpecialization",
        Target::Sysml(R::Calculation),
    ),
    (
        sc::CALCULATION_USAGE,
        "checkCalculationUsageSpecialization",
        Target::Sysml(R::Calculations),
    ),
    (
        sc::CONSTRAINT_DEFINITION,
        "checkConstraintDefinitionSpecialization",
        Target::Sysml(R::ConstraintCheck),
    ),
    (
        sc::CONSTRAINT_USAGE,
        "checkConstraintUsageSpecialization",
        Target::Sysml(R::ConstraintChecks),
    ),
    (
        sc::REQUIREMENT_DEFINITION,
        "checkRequirementDefinitionSpecialization",
        Target::Sysml(R::RequirementCheck),
    ),
    (
        sc::REQUIREMENT_USAGE,
        "checkRequirementUsageSpecialization",
        Target::Sysml(R::RequirementChecks),
    ),
    (
        sc::CONCERN_DEFINITION,
        "checkConcernDefinitionSpecialization",
        Target::Sysml(R::ConcernCheck),
    ),
    (
        sc::CONCERN_USAGE,
        "checkConcernUsageSpecialization",
        Target::Sysml(R::ConcernChecks),
    ),
    (
        sc::CASE_DEFINITION,
        "checkCaseDefinitionSpecialization",
        Target::Sysml(R::Case),
    ),
    (
        sc::CASE_USAGE,
        "checkCaseUsageSpecialization",
        Target::Sysml(R::Cases),
    ),
    (
        sc::ANALYSIS_CASE_DEFINITION,
        "checkAnalysisCaseDefinitionSpecialization",
        Target::Sysml(R::AnalysisCase),
    ),
    (
        sc::ANALYSIS_CASE_USAGE,
        "checkAnalysisCaseUsageSpecialization",
        Target::Sysml(R::AnalysisCases),
    ),
    (
        sc::VERIFICATION_CASE_DEFINITION,
        "checkVerificationCaseSpecialization",
        Target::Sysml(R::VerificationCase),
    ),
    (
        sc::VERIFICATION_CASE_USAGE,
        "checkVerificationCaseUsageSpecialization",
        Target::Sysml(R::VerificationCases),
    ),
    (
        sc::USE_CASE_DEFINITION,
        "checkUseCaseDefinitionSpecialization",
        Target::Sysml(R::UseCase),
    ),
    (
        sc::USE_CASE_USAGE,
        "checkUseCaseUsageSpecialization",
        Target::Sysml(R::UseCases),
    ),
    (
        sc::ALLOCATION_DEFINITION,
        "checkAllocationDefinitionSpecialization",
        Target::Sysml(R::Allocation),
    ),
    (
        sc::ALLOCATION_USAGE,
        "checkAllocationUsageSpecialization",
        Target::Sysml(R::Allocations),
    ),
    (
        sc::METADATA_DEFINITION,
        "checkMetadataDefinitionSpecialization",
        Target::Sysml(R::MetadataItem),
    ),
    (
        sc::METADATA_USAGE,
        "checkMetadataUsageSpecialization",
        Target::Sysml(R::MetadataItems),
    ),
    (
        sc::VIEW_DEFINITION,
        "checkViewDefinitionSpecialization",
        Target::Sysml(R::View),
    ),
    (
        sc::VIEW_USAGE,
        "checkViewUsageSpecialization",
        Target::Sysml(R::Views),
    ),
    (
        sc::VIEWPOINT_DEFINITION,
        "checkViewpointDefinitionSpecialization",
        Target::MissingViewpoint,
    ),
    (
        sc::VIEWPOINT_USAGE,
        "checkViewpointUsageSpecialization",
        Target::MissingViewpoints,
    ),
    (
        sc::RENDERING_DEFINITION,
        "checkRenderingDefinitionSpecialization",
        Target::Sysml(R::Rendering),
    ),
    (
        sc::RENDERING_USAGE,
        "checkRenderingUsageSpecialization",
        Target::Sysml(R::Renderings),
    ),
];
const COMPOSITE_RULES: &[(MetaclassId, &[MetaclassId], &str, Target)] = &[
    (
        sc::STATE_USAGE,
        &[sc::PART_DEFINITION, sc::PART_USAGE],
        "checkStateUsageOwnedStateSpecialization",
        Target::Sysml(R::OwnedStates),
    ),
    (
        sc::ITEM_USAGE,
        &[sc::ITEM_DEFINITION, sc::ITEM_USAGE],
        "checkItemUsageSubitemSpecialization",
        Target::PublishedSubitem,
    ),
    (
        sc::PART_USAGE,
        &[sc::ITEM_DEFINITION, sc::ITEM_USAGE],
        "checkPartUsageSubpartSpecialization",
        Target::Sysml(R::Subparts),
    ),
    (
        sc::PORT_USAGE,
        &[sc::PORT_DEFINITION, sc::PORT_USAGE],
        "checkPortUsageSubportSpecialization",
        Target::Sysml(R::Subports),
    ),
    (
        sc::ACTION_USAGE,
        &[sc::PART_DEFINITION, sc::PART_USAGE],
        "checkActionUsageOwnedActionSpecialization",
        Target::Sysml(R::OwnedActions),
    ),
];
const OWNED_RULES: &[(MetaclassId, &[MetaclassId], &str, Target)] = &[
    (
        sc::PERFORM_ACTION_USAGE,
        &[sc::PART_DEFINITION, sc::PART_USAGE],
        "checkPerformActionUsageSpecialization",
        Target::Sysml(R::PerformedActions),
    ),
    (
        sc::EVENT_OCCURRENCE_USAGE,
        &[sc::OCCURRENCE_DEFINITION, sc::OCCURRENCE_USAGE],
        "checkEventOccurrenceUsageSpecialization",
        Target::TimeEnclosedOccurrences,
    ),
    (
        sc::PORT_USAGE,
        &[sc::PART_DEFINITION, sc::PART_USAGE],
        "checkPortUsageOwnedPortSpecialization",
        Target::Sysml(R::OwnedPorts),
    ),
    (
        sc::CONSTRAINT_USAGE,
        &[sc::ITEM_DEFINITION, sc::ITEM_USAGE],
        "checkConstraintUsageCheckedConstraintSpecialization",
        Target::Sysml(R::CheckedConstraints),
    ),
];
const SUBACTION_RULES: &[(MetaclassId, &str, Target)] = &[
    (
        sc::ACTION_USAGE,
        "checkActionUsageSubactionSpecialization",
        Target::Sysml(R::Subactions),
    ),
    (
        sc::ASSIGNMENT_ACTION_USAGE,
        "checkAssignmentActionUsageSubactionSpecialization",
        Target::Sysml(R::Assignments),
    ),
    (
        sc::WHILE_LOOP_ACTION_USAGE,
        "checkWhileLoopActionUsageSubactionSpecialization",
        Target::Sysml(R::WhileLoops),
    ),
];

struct Evaluator<'q, 'm> {
    queries: &'q KerMlQueries<'m>,
    profile: SysmlBaselineProfile,
    bindings: &'q StandardSysmlBindings,
    roots: &'q [ElementId],
}
impl Evaluator<'_, '_> {
    fn subaction(&self, subject: ElementId) -> QueryResult<bool> {
        let mut result = self.result(subject, "isSubactionUsage").evidence;
        let mut applicable = false;
        if self.boolean(&mut result, subject, kp::FEATURE_IS_COMPOSITE) == Some(true) {
            let owner = self.queries.owning_type(subject);
            let owner_id = owner.value;
            result.merge_evidence(owner).expect("same producer context");
            if let Some(owner) = owner_id {
                result
                    .merge_evidence(
                        self.queries
                            .canonical_fact_evidence(FactKey::Element(owner)),
                    )
                    .expect("same producer context");
                if self.is(owner, sc::ACTION_DEFINITION) || self.is(owner, sc::ACTION_USAGE) {
                    let membership = self.queries.owning_relationship(subject);
                    let membership_id = membership.value;
                    result
                        .merge_evidence(membership)
                        .expect("same producer context");
                    applicable = true;
                    if let Some(membership) = membership_id {
                        result
                            .merge_evidence(
                                self.queries
                                    .canonical_fact_evidence(FactKey::Element(membership)),
                            )
                            .expect("same producer context");
                        if self.is(membership, sc::STATE_SUBACTION_MEMBERSHIP) {
                            let property = sp::STATE_SUBACTION_MEMBERSHIP_KIND;
                            result
                                .merge_evidence(self.queries.canonical_fact_evidence(
                                    FactKey::Property {
                                        element: membership,
                                        property,
                                    },
                                ))
                                .expect("same producer context");
                            applicable =
                                enumeration_is(self.queries.model(), membership, property, "do");
                        }
                    }
                }
            }
        }
        result.map(|()| applicable)
    }
    fn trigger_action(&self, subject: ElementId) -> QueryResult<bool> {
        let mut result = self.result(subject, "isTriggerAction").evidence;
        let owner = self.queries.owning_type(subject);
        let owner_id = owner.value;
        result.merge_evidence(owner).expect("same producer context");
        let mut applicable = false;
        if let Some(owner) = owner_id {
            result
                .merge_evidence(
                    self.queries
                        .canonical_fact_evidence(FactKey::Element(owner)),
                )
                .expect("same producer context");
            if self.is(owner, sc::TRANSITION_USAGE) {
                // deriveTransitionUsageTriggerAction selects exactly owned
                // TransitionFeatureMemberships of kind trigger whose member is
                // an AcceptActionUsage. owning_type already proves membership.
                let membership = self.queries.owning_relationship(subject);
                let membership_id = membership.value;
                result
                    .merge_evidence(membership)
                    .expect("same producer context");
                if let Some(membership) = membership_id {
                    result
                        .merge_evidence(
                            self.queries
                                .canonical_fact_evidence(FactKey::Element(membership)),
                        )
                        .expect("same producer context");
                    if self.is(membership, sc::TRANSITION_FEATURE_MEMBERSHIP) {
                        let property = sp::TRANSITION_FEATURE_MEMBERSHIP_KIND;
                        result
                            .merge_evidence(self.queries.canonical_fact_evidence(
                                FactKey::Property {
                                    element: membership,
                                    property,
                                },
                            ))
                            .expect("same producer context");
                        applicable =
                            enumeration_is(self.queries.model(), membership, property, "trigger");
                    }
                }
            }
        }
        result.map(|()| applicable)
    }
    fn substate(&self, subject: ElementId, parallel: bool) -> QueryResult<bool> {
        let mut result = self.result(subject, "isSubstateUsage").evidence;
        let mut applicable = false;
        if self.boolean(&mut result, subject, kp::FEATURE_IS_COMPOSITE) == Some(true) {
            let owner = self.queries.owning_type(subject);
            let owner_id = owner.value;
            result.merge_evidence(owner).expect("same producer context");
            if let Some(owner) = owner_id {
                result
                    .merge_evidence(
                        self.queries
                            .canonical_fact_evidence(FactKey::Element(owner)),
                    )
                    .expect("same producer context");
                let property = if self.is(owner, sc::STATE_DEFINITION) {
                    Some(sp::STATE_DEFINITION_IS_PARALLEL)
                } else if self.is(owner, sc::STATE_USAGE) {
                    Some(sp::STATE_USAGE_IS_PARALLEL)
                } else {
                    None
                };
                if let Some(property) = property {
                    applicable = self.boolean(&mut result, owner, property) == Some(parallel);
                    let membership = self.queries.owning_relationship(subject);
                    if let Some(membership) = membership.value {
                        result
                            .merge_evidence(
                                self.queries
                                    .canonical_fact_evidence(FactKey::Element(membership)),
                            )
                            .expect("same producer context");
                        applicable &= !self.is(membership, sc::STATE_SUBACTION_MEMBERSHIP);
                    }
                    result
                        .merge_evidence(membership)
                        .expect("same producer context");
                }
            }
        }
        result.map(|()| applicable)
    }
    fn transition_source(&self, subject: ElementId) -> QueryResult<Option<ElementId>> {
        let mut result = self
            .result(subject, "sourceFeature")
            .evidence
            .map(|()| None);
        let owned = self.queries.owned_relationships_excluding(
            subject,
            kc::MEMBERSHIP,
            [kc::FEATURE_MEMBERSHIP],
        );
        for &membership in &owned.value {
            result
                .merge_evidence(
                    self.queries
                        .canonical_fact_evidence(FactKey::Element(membership)),
                )
                .expect("same producer context");
            if !self.is(membership, kc::MEMBERSHIP) || self.is(membership, kc::FEATURE_MEMBERSHIP) {
                continue;
            }
            let member = self.queries.member(membership);
            let member_id = member.value;
            result
                .merge_evidence(member)
                .expect("same producer context");
            if let Some(member) = member_id {
                result
                    .merge_evidence(
                        self.queries
                            .canonical_fact_evidence(FactKey::Element(member)),
                    )
                    .expect("same producer context");
                if self.is(member, kc::FEATURE) {
                    let target = self.queries.feature_target(member);
                    let target_id = target.value;
                    result
                        .merge_evidence(target)
                        .expect("same producer context");
                    if let Some(target) = target_id {
                        result
                            .merge_evidence(
                                self.queries
                                    .canonical_fact_evidence(FactKey::Element(target)),
                            )
                            .expect("same producer context");
                        if self.is(target, sc::ACTION_USAGE) {
                            result.value = Some(target);
                            break;
                        }
                    }
                }
            }
        }
        result.merge_evidence(owned).expect("same producer context");
        result
    }
    fn is(&self, subject: ElementId, class: MetaclassId) -> bool {
        self.queries.model().element(subject).is_some_and(|record| {
            self.queries
                .model()
                .registry()
                .is_subtype(record.metaclass(), class)
                .unwrap_or(false)
        })
    }
    fn result(&self, subject: ElementId, rule: &'static str) -> SysmlProducerResult {
        let mut evidence = self
            .queries
            .canonical_fact_evidence(FactKey::Element(subject));
        evidence
            .search_dependencies
            .insert(SearchDependency::ValidationRule(rule));
        SysmlProducerResult {
            rule,
            evidence,
            relationships: vec![],
            properties: vec![],
            elements: vec![],
        }
    }
    fn boolean(
        &self,
        evidence: &mut QueryResult<()>,
        subject: ElementId,
        property: PropertyId,
    ) -> Option<bool> {
        evidence
            .merge_evidence(self.queries.canonical_fact_evidence(FactKey::Property {
                element: subject,
                property,
            }))
            .expect("same producer context");
        match self.queries.model().property_state(subject, property) {
            Ok(PropertyState::Computed(slot)) => match slot.value() {
                SlotValue::Scalar(Value::Boolean(value)) => Some(*value),
                _ => {
                    problem(
                        evidence,
                        subject,
                        Completeness::Invalid,
                        "SQ_BOOLEAN_KIND",
                        "Expected canonical Boolean",
                    );
                    None
                }
            },
            _ => {
                problem(
                    evidence,
                    subject,
                    Completeness::Incomplete,
                    "SQ_BOOLEAN_PENDING",
                    "Required Boolean predicate has not been established",
                );
                None
            }
        }
    }
    fn specialize(
        &self,
        mut result: SysmlProducerResult,
        subject: ElementId,
        target: Target,
    ) -> SysmlProducerResult {
        let target = self.target(subject, target);
        let general = target.value;
        result
            .evidence
            .merge_evidence(target)
            .expect("same producer context");
        let reflexive = general == Some(subject);
        let witness = general.and_then(|general| {
            self.queries
                .canonical_specialization_witness(subject, general)
        });
        let already_satisfied = reflexive || witness.is_some();
        if let Some(witness) = witness {
            result
                .evidence
                .merge_evidence(witness)
                .expect("same producer context");
        }
        // Exhaustive ancestor absence is only a redundancy optimization. The
        // rule's positive antecedents and exact target already prove its edge.
        // Waiting for negative closure here would make that closure depend on
        // the very edge this family is responsible for producing.
        if !already_satisfied
            && result.evidence.completeness == Completeness::Complete
            && let Some(general) = general
        {
            let feature = self.is(subject, kc::FEATURE);
            result.relationships.push(SysmlRelationshipProposal {
                key: DerivationKey {
                    rule: self.profile.rule_id(result.rule),
                    subject,
                    output: crate::profile::output_key(
                        "standard-generalization",
                        general.as_u128(),
                    ),
                },
                metaclass: if feature {
                    kc::SUBSETTING
                } else {
                    kc::SUBCLASSIFICATION
                },
                specific: subject,
                general,
                source_property: if feature {
                    kp::SUBSETTING_SUBSETTING_FEATURE
                } else {
                    kp::SUBCLASSIFICATION_SUBCLASSIFIER
                },
                target_property: if feature {
                    kp::SUBSETTING_SUBSETTED_FEATURE
                } else {
                    kp::SUBCLASSIFICATION_SUPERCLASSIFIER
                },
            });
        }
        result
    }
    fn target(&self, subject: ElementId, target: Target) -> QueryResult<Option<ElementId>> {
        match target {
            Target::KerMl(role) => self.queries.standard_role(role),
            Target::Suboccurrences => self
                .queries
                .formal_constraint_target(FormalConstraintId::FeatureSuboccurrenceSpecialization),
            Target::TimeEnclosedOccurrences => {
                let library = self
                    .queries
                    .context()
                    .standard_bindings
                    .as_ref()
                    .map(|bindings| bindings.library());
                if let Some(library) = library {
                    self.path(
                        subject,
                        &["Occurrences", "Occurrence", "timeEnclosedOccurrences"],
                        kc::FEATURE,
                        Some(kc::CLASS),
                        library,
                    )
                } else {
                    let mut result = self.queries.standard_role(StandardRole::Occurrence);
                    result.value = None;
                    result
                }
            }
            Target::Sysml(role) => {
                let (path, class) = role.specification();
                // Candidate paths are resolved with current negative-read evidence.
                // A validated binding is an extra consistency assertion, never a
                // name lookup replacement that can hide changed input populations.
                let mut answer = self.path(
                    subject,
                    path,
                    class,
                    Some(role.prefix_class(1)),
                    SystemsLibraryIdentity::LIBRARY,
                );
                if let Some(bound) = self.bindings.get(role)
                    && answer.value != Some(bound)
                {
                    problem(
                        &mut answer,
                        subject,
                        Completeness::Invalid,
                        "SQ_BINDING_MISMATCH",
                        "Current canonical target disagrees with the validated SysML binding",
                    );
                }
                answer
            }
            Target::PublishedSubitem => self.path(
                subject,
                &["Items", "Item", "subitem"],
                sc::ITEM_USAGE,
                Some(sc::ITEM_DEFINITION),
                SystemsLibraryIdentity::LIBRARY,
            ),
            Target::MissingViewpoint => self.path(
                subject,
                if self.profile == SysmlBaselineProfile::OPERATIONAL_V2 {
                    &["Views", "ViewpointCheck"]
                } else {
                    &["Views", "Viewpoint"]
                },
                sc::VIEWPOINT_DEFINITION,
                None,
                SystemsLibraryIdentity::LIBRARY,
            ),
            Target::MissingViewpoints => self.path(
                subject,
                if self.profile == SysmlBaselineProfile::OPERATIONAL_V2 {
                    &["Views", "viewpointChecks"]
                } else {
                    &["Views", "viewpoints"]
                },
                sc::VIEWPOINT_USAGE,
                None,
                SystemsLibraryIdentity::LIBRARY,
            ),
            Target::MissingBinaryConnections => self.path(
                subject,
                if self.profile == SysmlBaselineProfile::OPERATIONAL_V2 {
                    &["Connections", "BinaryConnection"]
                } else {
                    &["Connections", "BinaryConnections"]
                },
                sc::CONNECTION_DEFINITION,
                None,
                SystemsLibraryIdentity::LIBRARY,
            ),
        }
    }
    fn path(
        &self,
        subject: ElementId,
        path: &[&str],
        class: MetaclassId,
        intermediate_class: Option<MetaclassId>,
        library: LibraryId,
    ) -> QueryResult<Option<ElementId>> {
        let q = self.queries;
        let mut answer = q
            .canonical_fact_evidence(FactKey::Element(subject))
            .map(|()| None);
        answer
            .search_dependencies
            .insert(SearchDependency::StandardLibraries);
        let mut scopes = self.roots.to_vec();
        for (index, segment) in path.iter().enumerate() {
            let mut candidates = BTreeSet::new();
            for &scope in &scopes {
                if q.context().pending_namespace_scopes.contains(&scope) {
                    problem(
                        &mut answer,
                        scope,
                        Completeness::Incomplete,
                        "SQ_TARGET_NAMESPACE_PENDING",
                        "Canonical target namespace population is pending",
                    );
                }
                let owned = q.declared_owned_relationships(scope);
                for &membership in &owned.value {
                    answer
                        .merge_evidence(q.canonical_fact_evidence(FactKey::Element(membership)))
                        .expect("same producer context");
                    if !self.is(membership, kc::OWNING_MEMBERSHIP) {
                        continue;
                    }
                    let member = q.member(membership);
                    let element = member.value;
                    answer
                        .merge_evidence(member)
                        .expect("same producer context");
                    let Some(element) = element else {
                        continue;
                    };
                    answer
                        .merge_evidence(q.canonical_fact_evidence(FactKey::Element(element)))
                        .expect("same producer context");
                    let Some(record) = q.model().element(element) else {
                        continue;
                    };
                    // Namespace roots may contain authored lookalikes; only the
                    // original library is an authority for an algorithmic anchor.
                    let origin = Origin::Declared(DeclaredOrigin::StandardLibrary { library });
                    if record.origin() != &origin {
                        continue;
                    }
                    answer
                        .merge_evidence(q.canonical_fact_evidence(FactKey::Property {
                            element,
                            property: kp::ELEMENT_DECLARED_NAME,
                        }))
                        .expect("same producer context");
                    let Ok(PropertyState::Computed(name)) =
                        q.model().property_state(element, kp::ELEMENT_DECLARED_NAME)
                    else {
                        continue;
                    };
                    if name.value() != &SlotValue::Scalar(Value::String((*segment).into())) {
                        continue;
                    }
                    answer
                        .merge_evidence(q.canonical_fact_evidence(FactKey::Property {
                            element: membership,
                            property: kp::MEMBERSHIP_VISIBILITY,
                        }))
                        .expect("same producer context");
                    if name.origin() != &origin
                        || q.model()
                            .element(membership)
                            .is_none_or(|r| r.origin() != &origin)
                        || !public_membership(q.model(), membership, &origin)
                    {
                        problem(
                            &mut answer,
                            element,
                            Completeness::Invalid,
                            "SQ_TARGET_PROVENANCE",
                            "Target path requires original public library declarations",
                        );
                    }
                    let expected_class = if index + 1 == path.len() {
                        class
                    } else if index == 0 {
                        kc::LIBRARY_PACKAGE
                    } else {
                        intermediate_class.expect("nested anchor class contract")
                    };
                    if record.metaclass() != expected_class {
                        problem(
                            &mut answer,
                            element,
                            Completeness::Invalid,
                            "SQ_TARGET_METACLASS",
                            "Target declaration has the wrong concrete metaclass",
                        );
                    }
                    candidates.insert(element);
                }
                answer.merge_evidence(owned).expect("same producer context");
            }
            if candidates.len() != 1 {
                problem(
                    &mut answer,
                    subject,
                    if candidates.len() > 1 {
                        Completeness::Invalid
                    } else {
                        Completeness::Incomplete
                    },
                    if candidates.len() > 1 {
                        "SQ_TARGET_AMBIGUOUS"
                    } else {
                        "SQ_TARGET_MISSING"
                    },
                    &format!("Exact formal target {} is unavailable", path.join("::")),
                );
                return answer;
            }
            scopes = candidates.into_iter().collect();
        }
        if answer.completeness == Completeness::Complete {
            answer.value = scopes.first().copied();
        }
        answer
    }
}

fn public_membership(
    model: &agq_kernel::ModelView,
    membership: ElementId,
    origin: &Origin,
) -> bool {
    let Ok(PropertyState::Computed(visibility)) =
        model.property_state(membership, kp::MEMBERSHIP_VISIBILITY)
    else {
        return false;
    };
    let SlotValue::Scalar(Value::Enumeration(literal)) = visibility.value() else {
        return false;
    };
    let Ok(property) = model.registry().property(kp::MEMBERSHIP_VISIBILITY) else {
        return false;
    };
    let agq_kernel::metamodel::ValueKind::Enumeration(domain) = property.value_kind else {
        return false;
    };
    visibility.origin() == origin
        && model.registry().enumeration(domain).is_ok_and(|domain| {
            domain
                .literals
                .get(literal)
                .is_some_and(|name| name == "public")
        })
}
fn enumeration_is(
    model: &agq_kernel::ModelView,
    element: ElementId,
    property: PropertyId,
    name: &str,
) -> bool {
    let Ok(PropertyState::Computed(slot)) = model.property_state(element, property) else {
        return false;
    };
    let SlotValue::Scalar(Value::Enumeration(literal)) = slot.value() else {
        return false;
    };
    let Ok(descriptor) = model.registry().property(property) else {
        return false;
    };
    let agq_kernel::metamodel::ValueKind::Enumeration(domain) = descriptor.value_kind else {
        return false;
    };
    model.registry().enumeration(domain).is_ok_and(|domain| {
        domain
            .literals
            .get(literal)
            .is_some_and(|candidate| candidate == name)
    })
}
fn problem<T>(
    answer: &mut QueryResult<T>,
    subject: ElementId,
    status: Completeness,
    code: &'static str,
    message: &str,
) {
    answer.completeness = answer.completeness.max(status);
    answer.diagnostics.insert(Diagnostic {
        code,
        subject,
        message: message.into(),
    });
}

/// Exact deriveUsageMayTimeVary over the current graph. This query does not
/// certify producer closure or replace a missing predicate with isVariable=false.
pub fn current_usage_may_time_vary(
    queries: &KerMlQueries<'_>,
    profile: SysmlBaselineProfile,
    bindings: &StandardSysmlBindings,
    roots: &[ElementId],
    subject: ElementId,
) -> QueryResult<Option<bool>> {
    usage_may_time_vary(queries, profile, bindings, roots, subject, false)
}

fn usage_may_time_vary(
    queries: &KerMlQueries<'_>,
    profile: SysmlBaselineProfile,
    bindings: &StandardSysmlBindings,
    roots: &[ElementId],
    subject: ElementId,
    require_closure: bool,
) -> QueryResult<Option<bool>> {
    let evaluator = Evaluator {
        queries,
        profile,
        bindings,
        roots,
    };
    let mut evidence = queries.canonical_fact_evidence(FactKey::Element(subject));
    evidence
        .search_dependencies
        .insert(SearchDependency::ValidationRule("deriveUsageMayTimeVary"));
    if !evaluator.is(subject, sc::USAGE) {
        problem(
            &mut evidence,
            subject,
            Completeness::Invalid,
            "SQ_ELEMENT_KIND",
            "mayTimeVary requires a Usage",
        );
        return evidence.map(|()| None);
    }
    let owner = queries.owning_type(subject);
    let owner_id = owner.value;
    evidence
        .merge_evidence(owner)
        .expect("same producer context");
    let Some(owner) = owner_id else {
        if require_closure {
            evidence
                .merge_evidence(queries.producer_closure(
                    subject,
                    agq_kerml_semantics::SemanticClosureRequirement::EffectiveOwnership,
                ))
                .expect("same producer context");
        }
        let complete = evidence.completeness == Completeness::Complete;
        return evidence.map(|()| complete.then_some(false));
    };
    let occurrence = queries.standard_role(StandardRole::Occurrence);
    let occurrence_id = occurrence.value;
    evidence
        .merge_evidence(occurrence)
        .expect("same producer context");
    let owner_witness = occurrence_id
        .and_then(|occurrence| queries.canonical_specialization_witness(owner, occurrence));
    let owner_is_occurrence = if let Some(witness) = owner_witness {
        // This is a positive existential premise. A selected canonical path
        // proves it without importing unrelated owner ancestors' pending reads.
        evidence
            .merge_evidence(witness)
            .expect("same producer context");
        true
    } else {
        // No bounded witness is not absence: virtual/chained semantic paths and
        // the negative case still use the ordinary complete ancestor query.
        let ancestors = queries.all_supertypes(owner);
        let matches = occurrence_id.is_some_and(|id| ancestors.value.contains(&id));
        evidence
            .merge_evidence(ancestors)
            .expect("same producer context");
        matches
    };
    if evidence.completeness != Completeness::Complete {
        return evidence.map(|()| None);
    }
    if !owner_is_occurrence {
        if require_closure {
            evidence
                .merge_evidence(queries.producer_closure(
                    owner,
                    agq_kerml_semantics::SemanticClosureRequirement::EffectiveTyping,
                ))
                .expect("same producer context");
        }
        let complete = evidence.completeness == Completeness::Complete;
        return evidence.map(|()| complete.then_some(false));
    }
    if evaluator.boolean(&mut evidence, subject, kp::FEATURE_IS_PORTION) == Some(true) {
        let complete = evidence.completeness == Completeness::Complete;
        return evidence.map(|()| complete.then_some(false));
    }
    let composite = evaluator.boolean(&mut evidence, subject, kp::FEATURE_IS_COMPOSITE);
    let Some(library) = queries
        .context()
        .standard_bindings
        .as_ref()
        .and_then(|bindings| {
            bindings
                .library_set()
                .artifacts
                .get(&StandardLibraryArtifact::Semantic)
        })
        .copied()
    else {
        problem(
            &mut evidence,
            subject,
            Completeness::Incomplete,
            "SQ_KERML_ANCHORS_PENDING",
            "Accepted KerML anchors are unavailable",
        );
        return evidence.map(|()| None);
    };
    let self_link = evaluator.path(
        subject,
        &["Links", "SelfLink"],
        kc::ASSOCIATION,
        None,
        library,
    );
    let happens_link = evaluator.path(
        subject,
        &["Occurrences", "HappensLink"],
        kc::ASSOCIATION,
        None,
        library,
    );
    let mut excluded: Vec<_> = self_link
        .value
        .into_iter()
        .chain(happens_link.value)
        .collect();
    evidence
        .merge_evidence(self_link)
        .expect("same producer context");
    evidence
        .merge_evidence(happens_link)
        .expect("same producer context");
    if composite == Some(true) {
        let action = evaluator.target(subject, Target::Sysml(R::Action));
        excluded.extend(action.value);
        evidence
            .merge_evidence(action)
            .expect("same producer context");
    }
    if let Some(witness) = excluded
        .iter()
        .find_map(|&excluded| queries.canonical_specialization_witness(subject, excluded))
    {
        // Any positive exclusion proves false independently of other ancestors.
        // Their derived proofs may themselves read this Usage's mayTimeVary.
        evidence
            .merge_evidence(witness)
            .expect("same producer context");
        let complete = evidence.completeness == Completeness::Complete;
        return evidence.map(|()| complete.then_some(false));
    }
    // No bounded witness is not absence: virtual/chained paths and negative
    // excluded-type conclusions retain the ordinary complete query contract.
    let types = queries.all_supertypes(subject);
    let excludes = excluded.iter().any(|id| types.value.contains(id));
    evidence
        .merge_evidence(types)
        .expect("same producer context");
    if require_closure && !excludes {
        evidence
            .merge_evidence(queries.producer_closure(
                subject,
                agq_kerml_semantics::SemanticClosureRequirement::EffectiveTyping,
            ))
            .expect("same producer context");
    }
    let complete = evidence.completeness == Completeness::Complete;
    evidence.map(|()| complete.then_some(!excludes))
}

/// Final-stratum scalar proposal. Negative type predicates require the shared
/// scheduler's closure certificate; a fixed point alone is not evidence of absence.
pub fn plan_sysml_may_time_vary(
    queries: &KerMlQueries<'_>,
    profile: SysmlBaselineProfile,
    bindings: &StandardSysmlBindings,
    roots: &[ElementId],
    subject: ElementId,
) -> SysmlProducerResult {
    let answer = usage_may_time_vary(queries, profile, bindings, roots, subject, true);
    let value = answer.value;
    let rule = "deriveUsageMayTimeVary";
    let properties = value
        .filter(|_| answer.completeness == Completeness::Complete)
        .map(|value| SysmlPropertyProposal {
            subject,
            property: sp::USAGE_MAY_TIME_VARY,
            value: SlotValue::Scalar(Value::Boolean(value)),
            rule: profile.rule_id(rule),
        })
        .into_iter()
        .collect();
    SysmlProducerResult {
        rule,
        evidence: answer.map(|_| ()),
        relationships: vec![],
        properties,
        elements: vec![],
    }
}
