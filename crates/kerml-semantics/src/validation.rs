//! Explicit structural constraint checks, separate from executable evaluation.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, value::Value};
use std::collections::BTreeMap;

/// A rule disposition with the precise inputs retained for publication evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedefinitionEndConformance {
    pub redefining: ElementId,
    pub redefined: ElementId,
    pub redefining_is_end: bool,
    pub redefined_is_end: bool,
    pub owning_type: Option<ElementId>,
    pub owner_metaclass: Option<agq_kernel::MetaclassId>,
    pub rule: Rule,
    pub valid: bool,
}

impl KerMlQueries<'_> {
    /// Feature::owningType follows its owning FeatureMembership. Lexical owners,
    /// featuring types and inheriting namespaces do not change this identity.
    pub fn owning_type(&self, feature: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self
            .checked::<agq_kerml::views::Feature, _>(&mut out, feature)
            .is_none()
        {
            return out;
        }
        let membership = self.owning_relationship(feature);
        if let Some(membership) = membership
            .value
            .filter(|m| self.is(*m, c::FEATURE_MEMBERSHIP))
        {
            self.fact(
                &mut out,
                agq_kernel::provenance::FactKey::Element(membership),
            );
            let owner = self.owning_related_element(membership);
            if let Some(ty) = owner.value
                && self
                    .checked::<agq_kerml::views::Type, _>(&mut out, ty)
                    .is_some()
            {
                out.value = Some(ty);
                out.prove(
                    QueryKind::OwningType,
                    feature,
                    ty,
                    Rule::FeatureOwningType,
                    [
                        crate::contract::claim(QueryKind::OwningRelationship, feature, membership),
                        crate::contract::claim(QueryKind::OwningRelatedElement, membership, ty),
                    ],
                );
            }
            out.merge(owner);
        }
        out.merge(membership);
        out
    }

    /// Published 8.3.3.3.8, or exactly the reviewed KERML11-68 owner restriction
    /// under operational v4. This reads facts; it never changes end flags.
    pub fn validate_redefinition_end_conformance(
        &self,
        relationship: ElementId,
    ) -> QueryResult<Option<RedefinitionEndConformance>> {
        let mut out = self.result(None);
        if self
            .checked::<agq_kerml::views::Redefinition, _>(&mut out, relationship)
            .is_none()
        {
            return out;
        }
        let source =
            self.read_reference(&mut out, relationship, p::REDEFINITION_REDEFINING_FEATURE);
        let target = self.read_reference(&mut out, relationship, p::REDEFINITION_REDEFINED_FEATURE);
        let (Some(source), Some(target)) = (source, target) else {
            out.problem(
                Completeness::Incomplete,
                "KQ_REDEFINITION_ENDPOINT",
                relationship,
                "End conformance requires both canonical endpoints",
            );
            return out;
        };
        let source_end = self
            .read_value(&mut out, source, p::FEATURE_IS_END)
            .cloned();
        let target_end = self
            .read_value(&mut out, target, p::FEATURE_IS_END)
            .cloned();
        let (Some(Value::Boolean(source_end)), Some(Value::Boolean(target_end))) =
            (source_end, target_end)
        else {
            out.problem(
                Completeness::Incomplete,
                "KQ_REDEFINITION_END_FLAGS",
                relationship,
                "End conformance requires both end flags",
            );
            return out;
        };
        let owner = self.owning_type(source);
        let owning_type = owner.value;
        let owner_metaclass =
            owning_type.and_then(|id| self.model().element(id).map(|r| r.metaclass()));
        out.merge(owner);
        let operational = self
            .context()
            .options
            .baseline_profile
            .corrects_redefinition_end_conformance();
        let restricted = !operational
            || owning_type
                .is_some_and(|ty| self.is(ty, c::ASSOCIATION) || self.is(ty, c::CONNECTOR));
        let rule = if operational {
            Rule::OperationalRedefinitionEndConformanceV1
        } else {
            Rule::PublishedRedefinitionEndConformance
        };
        out.search_dependencies
            .insert(SearchDependency::ValidationRule(if operational {
                "agentique-kerml10-redefinition-end-conformance/1"
            } else {
                "KerML/1.0/validateRedefinitionEndConformance"
            }));
        let valid = !target_end || source_end || !restricted;
        out.value = Some(RedefinitionEndConformance {
            redefining: source,
            redefined: target,
            redefining_is_end: source_end,
            redefined_is_end: target_end,
            owning_type,
            owner_metaclass,
            rule,
            valid,
        });
        let premises: Vec<_> = out
            .positive_dependencies
            .iter()
            .copied()
            .map(Evidence::Fact)
            .chain(
                out.search_dependencies
                    .iter()
                    .cloned()
                    .map(Evidence::Search),
            )
            .collect();
        out.prove(
            QueryKind::RedefinitionEndConformance,
            relationship,
            relationship,
            rule,
            premises,
        );
        if !valid {
            out.problem(Completeness::Invalid, "validateRedefinitionEndConformance", relationship,
                format!("Non-end {source} redefines end {target}; owning type {owning_type:?}, metaclass {owner_metaclass:?}; rule {rule:?}"));
        }
        out
    }
    /// KerML 1.0 validateNamespaceDistinguishibility and
    /// Membership::isDistinguishableFrom. Different comparable element kinds
    /// cannot expose overlapping effective names through distinct memberships.
    pub fn validate_namespace_distinguishability(
        &self,
        namespace: ElementId,
    ) -> QueryResult<Vec<&'static str>> {
        let mut out = self.result(vec!["validateNamespaceDistinguishibility"]);
        let population = self.namespace_members(namespace, MemberAccess::All);
        let mut names = BTreeMap::<String, Vec<MemberMatch>>::new();
        for &member in &population.value {
            for name in self.names(&mut out, member.membership, Some(member.element)) {
                names.entry(name).or_default().push(member);
            }
        }
        out.merge(population);
        for (name, members) in names {
            for (index, left) in members.iter().enumerate() {
                for right in &members[index + 1..] {
                    if self.comparable_member_kinds(&mut out, left.element, right.element) {
                        out.problem(Completeness::Invalid,"validateNamespaceDistinguishibility",namespace,
                            format!("Name {name:?} identifies comparable members {} and {} through memberships {} and {}",
                                left.element,right.element,left.membership,right.membership));
                    }
                }
            }
        }
        out
    }

    /// Strict check of the model's implied-inclusion assertion, including when
    /// explicitly requested on a partial overlay. This does not establish closure.
    pub fn validate_implied_inclusion(&self, element: ElementId) -> QueryResult<()> {
        let mut out = self.result(());
        let owned = self.owned_relationships(element);
        let mut implied = false;
        for &relationship in &owned.value {
            implied |= matches!(
                self.read_value(&mut out, relationship, p::RELATIONSHIP_IS_IMPLIED),
                Some(Value::Boolean(true))
            );
        }
        let included = matches!(
            self.read_value(&mut out, element, p::ELEMENT_IS_IMPLIED_INCLUDED),
            Some(Value::Boolean(true))
        );
        if implied && !included {
            out.problem(
                Completeness::Invalid,
                "validateElementIsImpliedIncluded",
                element,
                "Implied relationships exist but the model does not assert implied inclusion",
            );
        }
        out.merge(owned);
        out
    }

    /// Checks deferred on this input phase, separately from executed rules.
    pub fn validation_deferred_by_phase(&self) -> Vec<&'static str> {
        if self.context().derivation_phase == DerivationPhase::PartialDerivationOverlay {
            vec!["validateElementIsImpliedIncluded"]
        } else {
            vec![]
        }
    }

    /// Local structural constraints whose evidence is independent of value
    /// evaluation. The returned names identify exactly which checks ran.
    /// This is one validation scope, not a claim of complete KerML validation.
    /// See `validation_deferred_by_phase` for checks requiring a different phase.
    pub fn validate_local_structure(&self, element: ElementId) -> QueryResult<Vec<&'static str>> {
        let mut out = self.result(vec![]);
        if self
            .checked::<agq_kerml::views::Element, _>(&mut out, element)
            .is_none()
        {
            return out;
        }
        let check = |out: &mut QueryResult<Vec<&'static str>>, rule: &'static str, valid: bool| {
            out.value.push(rule);
            if !valid {
                out.problem(
                    Completeness::Invalid,
                    rule,
                    element,
                    "KerML 1.0 structural constraint is not satisfied",
                );
            }
        };
        if self.context().derivation_phase != DerivationPhase::PartialDerivationOverlay {
            out.value.push("validateElementIsImpliedIncluded");
            out.merge(self.validate_implied_inclusion(element));
        }
        if self.is(element, c::REDEFINITION) {
            out.value.push("validateRedefinitionEndConformance");
            out.merge(self.validate_redefinition_end_conformance(element));
        }
        if self.is(element, c::SUBSETTING) {
            let source = if self.is(element, c::REFERENCE_SUBSETTING)
                || self.is(element, c::CROSS_SUBSETTING)
            {
                let owner = self.owning_related_element(element);
                let value = owner.value;
                out.merge(owner);
                value
            } else {
                self.read_reference(&mut out, element, p::SUBSETTING_SUBSETTING_FEATURE)
            };
            let target = self.read_reference(&mut out, element, p::SUBSETTING_SUBSETTED_FEATURE);
            if let (Some(source), Some(target)) = (source, target) {
                let source_unique = matches!(
                    self.read_value(&mut out, source, p::FEATURE_IS_UNIQUE),
                    Some(Value::Boolean(true))
                );
                let target_unique = matches!(
                    self.read_value(&mut out, target, p::FEATURE_IS_UNIQUE),
                    Some(Value::Boolean(true))
                );
                let source_constant = matches!(
                    self.read_value(&mut out, source, p::FEATURE_IS_CONSTANT),
                    Some(Value::Boolean(true))
                );
                let source_variable = matches!(
                    self.read_value(&mut out, source, p::FEATURE_IS_VARIABLE),
                    Some(Value::Boolean(true))
                );
                let target_constant = matches!(
                    self.read_value(&mut out, target, p::FEATURE_IS_CONSTANT),
                    Some(Value::Boolean(true))
                );
                check(
                    &mut out,
                    "validateSubsettingUniquenessConformance",
                    !target_unique || source_unique,
                );
                check(
                    &mut out,
                    "validateSubsettingConstantConformance",
                    !target_constant || !source_variable || source_constant,
                );
            }
        }
        if self.is(element, c::CLASSIFIER) {
            let generals = self.owned_specialization_targets(element);
            for (class, rule, forbidden) in [
                (
                    c::DATA_TYPE,
                    "validateDataTypeSpecialization",
                    vec![c::CLASS, c::ASSOCIATION],
                ),
                (
                    c::BEHAVIOR,
                    "validateBehaviorSpecialization",
                    vec![c::STRUCTURE],
                ),
                (
                    c::STRUCTURE,
                    "validateStructureSpecialization",
                    vec![c::BEHAVIOR],
                ),
                (
                    c::CLASS,
                    "validateClassSpecialization",
                    if self.is(element, c::ASSOCIATION) {
                        vec![c::DATA_TYPE]
                    } else {
                        vec![c::DATA_TYPE, c::ASSOCIATION]
                    },
                ),
            ] {
                if self.is(element, class) {
                    check(
                        &mut out,
                        rule,
                        !generals
                            .value
                            .iter()
                            .any(|g| forbidden.iter().any(|c| self.is(*g, *c))),
                    );
                }
            }
            out.merge(generals);
        }
        if self.is(element, c::ASSOCIATION) {
            check(
                &mut out,
                "validateAssociationStructureIntersection",
                self.is(element, c::STRUCTURE) == self.is(element, c::ASSOCIATION_STRUCTURE),
            );
        }
        if self.is(element, c::FEATURE) {
            let mut flags = BTreeMap::new();
            for property in [
                p::FEATURE_IS_END,
                p::FEATURE_IS_CONSTANT,
                p::FEATURE_IS_VARIABLE,
                p::FEATURE_IS_PORTION,
                p::FEATURE_IS_COMPOSITE,
                p::FEATURE_IS_DERIVED,
                p::TYPE_IS_ABSTRACT,
            ] {
                flags.insert(
                    property,
                    matches!(
                        self.read_value(&mut out, element, property),
                        Some(Value::Boolean(true))
                    ),
                );
            }
            let end = flags[&p::FEATURE_IS_END];
            let variable = flags[&p::FEATURE_IS_VARIABLE];
            let constant = flags[&p::FEATURE_IS_CONSTANT];
            let direction = self
                .read_value(&mut out, element, p::FEATURE_DIRECTION)
                .is_some();
            check(
                &mut out,
                "validateFeatureEndNoDirection",
                !end || !direction,
            );
            check(
                &mut out,
                "validateFeatureEndIsConstant",
                !end || !variable || constant,
            );
            check(
                &mut out,
                "validateFeatureConstantIsVariable",
                !constant || variable,
            );
            check(
                &mut out,
                "validateFeaturePortionNotVariable",
                !flags[&p::FEATURE_IS_PORTION] || !variable,
            );
            check(
                &mut out,
                "validateFeatureEndNotDerivedAbstractCompositeOrPortion",
                !end || ![
                    p::FEATURE_IS_DERIVED,
                    p::TYPE_IS_ABSTRACT,
                    p::FEATURE_IS_COMPOSITE,
                    p::FEATURE_IS_PORTION,
                ]
                .iter()
                .any(|p| flags[p]),
            );
            let owned = self.owned_relationships(element);
            let mut chains = vec![];
            for &relationship in &owned.value {
                if self.is(relationship, c::FEATURE_CHAINING)
                    && let Some(target) = self.read_reference(
                        &mut out,
                        relationship,
                        p::FEATURE_CHAINING_CHAINING_FEATURE,
                    )
                {
                    chains.push(target);
                }
            }
            let chain_count = owned
                .value
                .iter()
                .filter(|r| self.is(**r, c::FEATURE_CHAINING))
                .count();
            check(
                &mut out,
                "validateFeatureChainingFeatureNotOne",
                chain_count != 1,
            );
            check(
                &mut out,
                "validateFeatureChainingFeaturesNotSelf",
                !chains.contains(&element),
            );
            check(
                &mut out,
                "validateFeatureOwnedCrossSubsetting",
                owned
                    .value
                    .iter()
                    .filter(|r| self.is(**r, c::CROSS_SUBSETTING))
                    .count()
                    <= 1,
            );
            check(
                &mut out,
                "validateFeatureOwnedReferenceSubsetting",
                owned
                    .value
                    .iter()
                    .filter(|r| self.is(**r, c::REFERENCE_SUBSETTING))
                    .count()
                    <= 1,
            );
            out.merge(owned);
        }
        if self.is(element, c::TYPE) {
            let owned = self.owned_relationships(element);
            check(
                &mut out,
                "validateTypeAtMostOneConjugator",
                owned
                    .value
                    .iter()
                    .filter(|r| self.is(**r, c::CONJUGATION))
                    .count()
                    <= 1,
            );
            for (class, property, arity, not_self) in [
                (
                    c::DIFFERENCING,
                    p::DIFFERENCING_DIFFERENCING_TYPE,
                    "validateTypeOwnedDifferencingNotOne",
                    "validateTypeDifferencingTypesNotSelf",
                ),
                (
                    c::UNIONING,
                    p::UNIONING_UNIONING_TYPE,
                    "validateTypeOwnedUnioningNotOne",
                    "validateTypeUnioningTypesNotSelf",
                ),
                (
                    c::INTERSECTING,
                    p::INTERSECTING_INTERSECTING_TYPE,
                    "validateTypeOwnedIntersectingNotOne",
                    "validateTypeIntersectingTypesNotSelf",
                ),
            ] {
                let relationships: Vec<_> = owned
                    .value
                    .iter()
                    .copied()
                    .filter(|r| self.is(*r, class))
                    .collect();
                check(&mut out, arity, relationships.len() != 1);
                let mut targets = vec![];
                for relationship in relationships {
                    targets.extend(self.read_reference(&mut out, relationship, property));
                }
                check(&mut out, not_self, !targets.contains(&element));
            }
            let memberships = self.memberships(element);
            let mut multiplicities = 0;
            for &membership in &memberships.value {
                if self.is(membership, c::OWNING_MEMBERSHIP) {
                    let member = self.member(membership);
                    multiplicities +=
                        usize::from(member.value.is_some_and(|e| self.is(e, c::MULTIPLICITY)));
                    out.merge(member);
                }
            }
            check(
                &mut out,
                "validateTypeOwnedMultiplicity",
                multiplicities <= 1,
            );
            out.merge(memberships);
            out.merge(owned);
        }
        if self.is(element, c::EXPRESSION) || self.is(element, c::FUNCTION) {
            let expression = self.is(element, c::EXPRESSION);
            let features = self.effective_features(element);
            let mut results = 0;
            for &feature in &features.value {
                let owner = self.owning_relationship(feature);
                results += usize::from(
                    owner
                        .value
                        .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP)),
                );
                out.merge(owner);
            }
            check(
                &mut out,
                if expression {
                    "validateExpressionResultParameterMembership"
                } else {
                    "validateFunctionResultParameterMembership"
                },
                results == 1,
            );
            out.merge(features);
            let members = self.namespace_members(element, MemberAccess::All);
            let result_expressions = members
                .value
                .iter()
                .filter(|m| self.is(m.membership, c::RESULT_EXPRESSION_MEMBERSHIP))
                .count();
            check(
                &mut out,
                if expression {
                    "validateExpressionResultExpressionMembership"
                } else {
                    "validateFunctionResultExpressionMembership"
                },
                result_expressions <= 1,
            );
            out.merge(members);
        }
        if self.is(element, c::RETURN_PARAMETER_MEMBERSHIP)
            || self.is(element, c::RESULT_EXPRESSION_MEMBERSHIP)
        {
            let owner = self.owning_related_element(element);
            let valid = owner
                .value
                .is_some_and(|t| self.is(t, c::FUNCTION) || self.is(t, c::EXPRESSION));
            check(
                &mut out,
                if self.is(element, c::RETURN_PARAMETER_MEMBERSHIP) {
                    "validateReturnParameterMembershipOwningType"
                } else {
                    "validateResultExpressionMembershipOwningType"
                },
                valid,
            );
            out.merge(owner);
        }
        if self.is(element, c::PARAMETER_MEMBERSHIP) {
            let owner = self.owning_related_element(element);
            let mut constructor_parameter = false;
            if let Some(ty) = owner.value {
                let membership = self.owning_relationship(ty);
                if membership
                    .value
                    .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP))
                {
                    let constructor = self.owning_type(ty);
                    constructor_parameter = constructor
                        .value
                        .is_some_and(|o| self.is(o, c::CONSTRUCTOR_EXPRESSION));
                    out.merge(constructor);
                }
                out.merge(membership);
            }
            check(
                &mut out,
                "validateParameterMembershipOwningType",
                owner
                    .value
                    .is_some_and(|o| self.is(o, c::BEHAVIOR) || self.is(o, c::STEP))
                    || constructor_parameter,
            );
            out.merge(owner);
            let member = self.member(element);
            let expected = if self.is(element, c::RETURN_PARAMETER_MEMBERSHIP) {
                "out"
            } else {
                "in"
            };
            let correct = member
                .value
                .is_some_and(|f| self.direction_name(&mut out, f) == Some(expected));
            check(
                &mut out,
                "validateParameterMembershipParameterDirection",
                correct,
            );
            out.merge(member);
        }
        if self.is(element, c::END_FEATURE_MEMBERSHIP) {
            let member = self.member(element);
            let correct = member.value.is_some_and(|f| {
                matches!(
                    self.read_value(&mut out, f, p::FEATURE_IS_END),
                    Some(Value::Boolean(true))
                )
            });
            check(&mut out, "validateEndFeatureMembershipIsEnd", correct);
            out.merge(member);
        }
        if self.is(element, c::FEATURE_REFERENCE_EXPRESSION) {
            let members = self.memberships(element);
            let referent = members
                .value
                .iter()
                .copied()
                .find(|m| !self.is(*m, c::PARAMETER_MEMBERSHIP));
            let mut valid = false;
            if let Some(membership) = referent {
                let member = self.member(membership);
                valid = member.value.is_some_and(|e| self.is(e, c::FEATURE));
                out.merge(member);
            }
            check(
                &mut out,
                "validateFeatureReferenceExpressionReferentIsFeature",
                valid,
            );
            let result = self.expression_result(&mut out, element);
            let owner = result.map(|r| self.owner(r));
            check(
                &mut out,
                "validateFeatureReferenceExpressionResult",
                owner.as_ref().is_some_and(|r| r.value == Some(element)),
            );
            if let Some(owner) = owner {
                out.merge(owner);
            }
            out.merge(members);
        }
        out
    }

    pub(crate) fn direction_name<T>(
        &self,
        out: &mut QueryResult<T>,
        feature: ElementId,
    ) -> Option<&str> {
        let Some(Value::Enumeration(literal)) = self.read_value(out, feature, p::FEATURE_DIRECTION)
        else {
            return None;
        };
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = self
            .model()
            .registry()
            .property(p::FEATURE_DIRECTION)
            .expect("direction property")
            .value_kind
        else {
            return None;
        };
        self.model()
            .registry()
            .enumeration(domain)
            .expect("direction domain")
            .literals
            .get(literal)
            .map(String::as_str)
    }
}
