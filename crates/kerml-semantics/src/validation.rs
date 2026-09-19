//! Explicit structural constraint checks, separate from executable evaluation.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, value::Value};
use std::collections::BTreeMap;

impl KerMlQueries<'_> {
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
                    let left_class = self
                        .model()
                        .element(left.element)
                        .expect("membership endpoint")
                        .metaclass();
                    let right_class = self
                        .model()
                        .element(right.element)
                        .expect("membership endpoint")
                        .metaclass();
                    if self
                        .model()
                        .registry()
                        .is_subtype(left_class, right_class)
                        .unwrap_or(false)
                        || self
                            .model()
                            .registry()
                            .is_subtype(right_class, left_class)
                            .unwrap_or(false)
                    {
                        out.problem(Completeness::Invalid,"validateNamespaceDistinguishibility",namespace,
                            format!("Name {name:?} identifies comparable members {} and {} through memberships {} and {}",
                                left.element,right.element,left.membership,right.membership));
                    }
                }
            }
        }
        out
    }

    /// Local structural constraints whose evidence is independent of value
    /// evaluation. The returned names identify exactly which checks ran.
    /// This is one validation scope, not a claim of complete KerML validation.
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
        let owned_relationships = self.owned_relationships(element);
        let mut implied = false;
        for &relationship in &owned_relationships.value {
            implied |= matches!(
                self.read_value(&mut out, relationship, p::RELATIONSHIP_IS_IMPLIED),
                Some(Value::Boolean(true))
            );
        }
        let included = matches!(
            self.read_value(&mut out, element, p::ELEMENT_IS_IMPLIED_INCLUDED),
            Some(Value::Boolean(true))
        );
        check(
            &mut out,
            "validateElementIsImpliedIncluded",
            !implied || included,
        );
        out.merge(owned_relationships);
        if self.is(element, c::REDEFINITION) {
            let redefining =
                self.read_reference(&mut out, element, p::REDEFINITION_REDEFINING_FEATURE);
            let redefined =
                self.read_reference(&mut out, element, p::REDEFINITION_REDEFINED_FEATURE);
            if let (Some(redefining), Some(redefined)) = (redefining, redefined) {
                let target_end = matches!(
                    self.read_value(&mut out, redefined, p::FEATURE_IS_END),
                    Some(Value::Boolean(true))
                );
                let source_end = matches!(
                    self.read_value(&mut out, redefining, p::FEATURE_IS_END),
                    Some(Value::Boolean(true))
                );
                check(
                    &mut out,
                    "validateRedefinitionEndConformance",
                    !target_end || source_end,
                );
            }
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
}
