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

/// One normative rule evaluation, including all positive and negative reads.
/// Incomplete evidence never authorizes any of this result's proposals.
#[derive(Clone, Debug)]
pub struct SysmlProducerResult {
    pub rule: &'static str,
    pub evidence: QueryResult<()>,
    pub relationships: Vec<SysmlRelationshipProposal>,
    pub properties: Vec<SysmlPropertyProposal>,
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

#[derive(Clone, Copy)]
enum Target {
    Sysml(R),
    KerMl(StandardRole),
    Suboccurrences,
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
                        && profile == SysmlBaselineProfile::OPERATIONAL_V1
                    {
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
        let mut result = evaluator.result(subject, "checkActionUsageSubactionSpecialization");
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
                if evaluator.is(owner, sc::ACTION_DEFINITION)
                    || evaluator.is(owner, sc::ACTION_USAGE)
                {
                    let membership = queries.owning_relationship(subject);
                    let membership_id = membership.value;
                    result
                        .evidence
                        .merge_evidence(membership)
                        .expect("same producer context");
                    let mut applicable = true;
                    if let Some(membership) = membership_id {
                        result
                            .evidence
                            .merge_evidence(
                                queries.canonical_fact_evidence(FactKey::Element(membership)),
                            )
                            .expect("same producer context");
                        if evaluator.is(membership, sc::STATE_SUBACTION_MEMBERSHIP) {
                            let property = sp::STATE_SUBACTION_MEMBERSHIP_KIND;
                            result
                                .evidence
                                .merge_evidence(queries.canonical_fact_evidence(
                                    FactKey::Property {
                                        element: membership,
                                        property,
                                    },
                                ))
                                .expect("same producer context");
                            applicable =
                                enumeration_is(queries.model(), membership, property, "do");
                        }
                    }
                    if applicable {
                        result =
                            evaluator.specialize(result, subject, Target::Sysml(R::Subactions));
                    }
                }
            }
        }
        plan.results.push(result);
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
    if evaluator.is(subject, sc::CONNECTION_DEFINITION)
        || evaluator.is(subject, sc::CONNECTION_USAGE)
    {
        let definition = evaluator.is(subject, sc::CONNECTION_DEFINITION);
        let rule = if definition {
            "checkConnectionDefinitionBinarySpecialization"
        } else {
            "checkConnectionUsageBinarySpecialization"
        };
        let mut result = evaluator.result(subject, rule);
        let features = queries.direct_features(subject);
        let mut ends = 0;
        for &feature in &features.value {
            if evaluator.boolean(&mut result.evidence, feature, kp::FEATURE_IS_END) == Some(true) {
                ends += 1;
            }
        }
        result
            .evidence
            .merge_evidence(features)
            .expect("same producer context");
        if ends == 2 {
            result = evaluator.specialize(
                result,
                subject,
                if definition {
                    Target::MissingBinaryConnections
                } else {
                    Target::Sysml(R::BinaryConnections)
                },
            );
        }
        plan.results.push(result);
    }
    if evaluator.is(subject, sc::FLOW_USAGE) {
        let mut result = evaluator.result(subject, "checkFlowUsageFlowSpecialization");
        let features = queries.direct_features(subject);
        let mut has_end = false;
        for &feature in &features.value {
            has_end |=
                evaluator.boolean(&mut result.evidence, feature, kp::FEATURE_IS_END) == Some(true);
        }
        result
            .evidence
            .merge_evidence(features)
            .expect("same producer context");
        if has_end {
            result = evaluator.specialize(result, subject, Target::Sysml(R::Flows));
        }
        plan.results.push(result);
    }
    // A more specific proposed standard edge can already establish a broader
    // requirement through the target's existing ancestry. Retain that proof and
    // its reads instead of materializing a redundant second relationship.
    let proposals: Vec<_> = plan
        .results
        .iter()
        .filter_map(|result| result.relationships.first().map(|p| p.general))
        .collect();
    for result in &mut plan.results {
        let Some(general) = result.relationships.first().map(|p| p.general) else {
            continue;
        };
        for &other in &proposals {
            if general == other {
                continue;
            }
            let ancestors = queries.all_supertypes(other);
            if ancestors.completeness == Completeness::Complete
                && ancestors.value.contains(&general)
            {
                let reverse = queries.all_supertypes(general);
                // Equivalent bases use the lowest stable canonical ID so a
                // cycle never suppresses both obligations.
                let preferred = !reverse.value.contains(&other) || other < general;
                if preferred && reverse.completeness == Completeness::Complete {
                    result
                        .evidence
                        .merge_evidence(ancestors)
                        .expect("same producer context");
                    result
                        .evidence
                        .merge_evidence(reverse)
                        .expect("same producer context");
                    result.relationships.clear();
                    break;
                }
            }
        }
    }
    plan
}

const BASE_RULES: &[(MetaclassId, &str, Target)] = &[
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

struct Evaluator<'q, 'm> {
    queries: &'q KerMlQueries<'m>,
    profile: SysmlBaselineProfile,
    bindings: &'q StandardSysmlBindings,
    roots: &'q [ElementId],
}
impl Evaluator<'_, '_> {
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
        let ancestors = self.queries.all_supertypes(subject);
        let already_satisfied = general.is_some_and(|id| ancestors.value.contains(&id));
        result
            .evidence
            .merge_evidence(ancestors)
            .expect("same producer context");
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
                &["Views", "Viewpoint"],
                sc::VIEWPOINT_DEFINITION,
                None,
                SystemsLibraryIdentity::LIBRARY,
            ),
            Target::MissingViewpoints => self.path(
                subject,
                &["Views", "viewpoints"],
                sc::VIEWPOINT_USAGE,
                None,
                SystemsLibraryIdentity::LIBRARY,
            ),
            Target::MissingBinaryConnections => self.path(
                subject,
                &["Connections", "BinaryConnections"],
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
                answer
                    .search_dependencies
                    .insert(SearchDependency::NamespaceMembers { namespace: scope });
                if q.context().pending_namespace_scopes.contains(&scope) {
                    problem(
                        &mut answer,
                        scope,
                        Completeness::Incomplete,
                        "SQ_TARGET_NAMESPACE_PENDING",
                        "Canonical target namespace population is pending",
                    );
                }
                let owned = q.owned_relationships(scope);
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
        let complete = evidence.completeness == Completeness::Complete;
        return evidence.map(|()| complete.then_some(false));
    };
    let occurrence = queries.standard_role(StandardRole::Occurrence);
    let occurrence_id = occurrence.value;
    evidence
        .merge_evidence(occurrence)
        .expect("same producer context");
    let ancestors = queries.all_supertypes(owner);
    let owner_is_occurrence = occurrence_id.is_some_and(|id| ancestors.value.contains(&id));
    evidence
        .merge_evidence(ancestors)
        .expect("same producer context");
    if evidence.completeness != Completeness::Complete {
        return evidence.map(|()| None);
    }
    if !owner_is_occurrence {
        return evidence.map(|()| Some(false));
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
    let excluded: Vec<_> = self_link
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
    let types = queries.all_supertypes(subject);
    let mut excludes = excluded.iter().any(|id| types.value.contains(id));
    if composite == Some(true) {
        let action = evaluator.target(subject, Target::Sysml(R::Action));
        excludes |= action.value.is_some_and(|id| types.value.contains(&id));
        evidence
            .merge_evidence(action)
            .expect("same producer context");
    }
    evidence
        .merge_evidence(types)
        .expect("same producer context");
    let complete = evidence.completeness == Completeness::Complete;
    evidence.map(|()| complete.then_some(!excludes))
}

/// Final-stratum scalar proposal. Call only after the shared structural worklist
/// reaches fixed point; any subsequent structural change invalidates this proof.
pub fn plan_sysml_may_time_vary(
    queries: &KerMlQueries<'_>,
    profile: SysmlBaselineProfile,
    bindings: &StandardSysmlBindings,
    roots: &[ElementId],
    subject: ElementId,
) -> SysmlProducerResult {
    let answer = current_usage_may_time_vary(queries, profile, bindings, roots, subject);
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
    }
}
