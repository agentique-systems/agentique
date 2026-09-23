//! Structural expression targets. No expression execution occurs here.
use crate::*;
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, value::Value};

#[cfg(test)]
#[path = "../tests/unit/instantiation_population.rs"]
mod population_tests;

impl KerMlQueries<'_> {
    /// The declared target or standard operator Function of an instantiation.
    /// Standard package identities are bound once; operator symbols remain
    /// ordinary member names within those exact public namespaces.
    pub fn instantiated_type(&self, expression: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self
            .checked::<views::InstantiationExpression, _>(&mut out, expression)
            .is_none()
        {
            return out;
        }
        if self.is(expression, c::OPERATOR_EXPRESSION) {
            let symbol =
                match self.read_value(&mut out, expression, p::OPERATOR_EXPRESSION_OPERATOR) {
                    Some(Value::String(symbol)) => Some(symbol.clone()),
                    _ => None,
                };
            if let Some(symbol) = symbol {
                for role in [
                    StandardRole::BaseFunctions,
                    StandardRole::DataFunctions,
                    StandardRole::ControlFunctions,
                ] {
                    let namespace = self.standard_role(role);
                    if let Some(namespace_id) = namespace.value {
                        let members =
                            self.lookup_member(namespace_id, &symbol, MemberAccess::Public);
                        match members.value.as_slice() {
                            [member] if self.is(member.element, c::FUNCTION) => {
                                out.value = Some(member.element)
                            }
                            [] => {}
                            _ => out.problem(
                                Completeness::Invalid,
                                "KQ_OPERATOR_TARGET",
                                expression,
                                "The standard operator name must identify one Function",
                            ),
                        }
                        out.merge(members);
                    }
                    out.merge(namespace);
                    if out.value.is_some() || out.completeness != Completeness::Complete {
                        break;
                    }
                }
            }
        } else {
            let memberships = self.owned_relationships_excluding(
                expression,
                c::MEMBERSHIP,
                [c::FEATURE_MEMBERSHIP],
            );
            if let Some(&membership) = memberships.value.first() {
                let member = self.member(membership);
                out.value = member.value.filter(|&e| self.is(e, c::TYPE));
                out.merge(member);
            }
            out.merge(memberships);
        }
        if out.value.is_none() {
            out.problem(
                Completeness::Incomplete,
                "KQ_INSTANTIATED_TYPE",
                expression,
                "The instantiation requires its canonical target Type",
            );
        }
        if out.completeness != Completeness::Complete {
            out.value = None;
        }
        out
    }

    /// First owned input Feature, in semantic membership order.
    pub(crate) fn first_input(&self, expression: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let features = self.owned_directed_features(expression);
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = self
            .model()
            .registry()
            .property(p::FEATURE_DIRECTION)
            .expect("direction")
            .value_kind
        else {
            unreachable!()
        };
        for &feature in &features.value {
            if let Some(Value::Enumeration(literal)) =
                self.read_value(&mut out, feature, p::FEATURE_DIRECTION)
                && self
                    .model()
                    .registry()
                    .enumeration(domain)
                    .expect("direction domain")
                    .literals
                    .get(literal)
                    .is_some_and(|name| name == "in")
            {
                out.value = Some(feature);
                break;
            }
        }
        out.merge(features);
        out
    }

    /// The first owned Feature of a chain expression's first owned input.
    pub fn source_target_feature(&self, expression: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self
            .checked::<views::FeatureChainExpression, _>(&mut out, expression)
            .is_none()
        {
            return out;
        }
        let input = self.first_input(expression);
        if let Some(input) = input.value {
            let features = self.direct_features(input);
            out.value = features.value.first().copied();
            out.merge(features);
        }
        out.merge(input);
        out
    }
}
