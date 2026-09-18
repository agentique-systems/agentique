//! Structural consequences of KerML 1.0 checks, without executable evaluation.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, value::Value};

#[derive(Clone, Copy)]
enum Position {
    Parameter,
    Result,
    End,
}
impl KerMlQueries<'_> {
    pub(crate) fn expression_result<T>(
        &self,
        out: &mut QueryResult<T>,
        expression: ElementId,
    ) -> Option<ElementId> {
        let memberships = self.memberships(expression);
        let mut result = None;
        for &membership in &memberships.value {
            if self.is(membership, c::RETURN_PARAMETER_MEMBERSHIP) {
                let member = self.member(membership);
                if result.is_some() {
                    out.problem(
                        Completeness::Invalid,
                        "KQ_RESULT_ARITY",
                        expression,
                        "Expression has multiple owned results",
                    );
                }
                result = member.value;
                out.merge(member);
            }
        }
        out.merge(memberships);
        result
    }
    pub(crate) fn argument_expression<T>(
        &self,
        out: &mut QueryResult<T>,
        expression: ElementId,
    ) -> Option<ElementId> {
        let members = self.memberships(expression);
        let parameter = members.value.iter().copied().find(|m| {
            self.is(*m, c::PARAMETER_MEMBERSHIP) && !self.is(*m, c::RETURN_PARAMETER_MEMBERSHIP)
        });
        out.merge(members);
        let parameter = parameter?;
        let member = self.member(parameter);
        let feature = member.value;
        out.merge(member);
        let owned = self.owned_relationships(feature?);
        let value = owned
            .value
            .iter()
            .copied()
            .find(|r| self.is(*r, c::FEATURE_VALUE));
        out.merge(owned);
        self.read_reference(out, value?, p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
    }
    pub(crate) fn reference_expression_result(
        &self,
        feature: ElementId,
    ) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let membership = self.owning_relationship(feature);
        let is_result = membership
            .value
            .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP));
        out.merge(membership);
        if !is_result {
            return out;
        }
        let owner = self.owner(feature);
        let expression = owner.value;
        out.merge(owner);
        let Some(expression) = expression.filter(|e| self.is(*e, c::FEATURE_REFERENCE_EXPRESSION))
        else {
            return out;
        };
        let members = self.memberships(expression);
        let referent = members
            .value
            .iter()
            .copied()
            .find(|m| !self.is(*m, c::PARAMETER_MEMBERSHIP));
        out.merge(members);
        if let Some(membership) = referent {
            let member = self.member(membership);
            out.value.extend(member.value);
            out.merge(member);
            for target in out.value.clone() {
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
                    QueryKind::DirectSpecializations,
                    feature,
                    target,
                    Rule::ExpressionResult,
                    premises,
                );
            }
        }
        out
    }
    pub(crate) fn library_specializations(&self, source: ElementId) -> QueryResult<Vec<ElementId>> {
        use StandardRole as R;
        let mut out = self.result(vec![]);
        let Some(bindings) = self.context().standard_bindings.as_ref() else {
            return out;
        };
        out.search_dependencies
            .insert(SearchDependency::StandardLibraries);
        self.fact(&mut out, agq_kernel::provenance::FactKey::Element(source));
        let mut roles = vec![];
        for (class, role) in [
            (c::TYPE, R::Anything),
            (c::DATA_TYPE, R::DataValue),
            (c::CLASS, R::Occurrence),
            (c::STRUCTURE, R::Object),
            (c::ASSOCIATION, R::Link),
            (c::METACLASS, R::Metaobject),
            (c::BEHAVIOR, R::Performance),
            (c::FUNCTION, R::Evaluation),
            (c::PREDICATE, R::BooleanEvaluation),
            (c::FEATURE, R::Things),
            (c::STEP, R::Performances),
            (c::EXPRESSION, R::Evaluations),
            (c::BOOLEAN_EXPRESSION, R::BooleanEvaluations),
            (c::MULTIPLICITY, R::Naturals),
            (c::METADATA_FEATURE, R::Metaobjects),
            (c::CONNECTOR, R::Links),
            (c::BINDING_CONNECTOR, R::SelfLinks),
        ] {
            if self.is(source, class) {
                roles.push(role);
            }
        }
        if self.is(source, c::INVARIANT) {
            roles.push(
                if matches!(
                    self.read_value(&mut out, source, p::INVARIANT_IS_NEGATED),
                    Some(Value::Boolean(true))
                ) {
                    R::FalseEvaluations
                } else {
                    R::TrueEvaluations
                },
            );
        }
        if self.is(source, c::FEATURE) {
            let typing = self.direct_feature_types(source);
            for target in &typing.value {
                for (class, role) in [
                    (c::STRUCTURE, R::Objects),
                    (c::CLASS, R::Occurrences),
                    (c::DATA_TYPE, R::DataValues),
                ] {
                    if self.is(*target, class) {
                        roles.push(role);
                    }
                }
            }
            out.merge(typing);
        }
        for role in roles {
            let target = bindings.get(role);
            if target == source {
                continue;
            }
            self.fact(&mut out, agq_kernel::provenance::FactKey::Element(target));
            out.value.push(target);
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
                QueryKind::DirectSpecializations,
                source,
                target,
                Rule::LibrarySpecialization,
                premises,
            );
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    fn positioned_features<T>(
        &self,
        out: &mut QueryResult<T>,
        owner: ElementId,
        position: Position,
    ) -> Vec<ElementId> {
        let members = self.memberships(owner);
        let mut features = vec![];
        for &membership in &members.value {
            if !self.is(membership, c::FEATURE_MEMBERSHIP) {
                continue;
            }
            let member = self.member(membership);
            if let Some(feature) = member.value {
                let result = self.is(membership, c::RETURN_PARAMETER_MEMBERSHIP);
                let selected = match position {
                    Position::Result => result,
                    Position::End => matches!(
                        self.read_value(out, feature, p::FEATURE_IS_END),
                        Some(Value::Boolean(true))
                    ),
                    Position::Parameter => {
                        !result
                            && self
                                .read_value(out, feature, p::FEATURE_DIRECTION)
                                .is_some()
                    }
                };
                if selected {
                    features.push(feature);
                }
            }
            out.merge(member);
        }
        out.merge(members);
        features
    }
    pub(crate) fn implied_redefinitions(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let owner = self.owner(feature);
        let Some(ty) = owner.value.filter(|t| self.is(*t, c::TYPE)) else {
            out.merge(owner);
            return out;
        };
        out.merge(owner);
        let membership = self.owning_relationship(feature);
        let result = membership
            .value
            .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP));
        out.merge(membership);
        let directed = self
            .read_value(&mut out, feature, p::FEATURE_DIRECTION)
            .is_some();
        let end = matches!(
            self.read_value(&mut out, feature, p::FEATURE_IS_END),
            Some(Value::Boolean(true))
        );
        let behavioral = self.is(ty, c::BEHAVIOR) || self.is(ty, c::STEP);
        let mut positions = vec![];
        if result && (self.is(ty, c::FUNCTION) || self.is(ty, c::EXPRESSION)) {
            positions.push((Position::Result, Rule::ResultRedefinition));
        }
        if directed && !result && behavioral {
            positions.push((Position::Parameter, Rule::ParameterRedefinition));
        }
        if end {
            positions.push((Position::End, Rule::EndRedefinition));
        }
        if positions.is_empty() {
            return out;
        }
        // checkFeatureParameterRedefinition excludes explicit argument redefinitions
        // on an InvocationExpression. Other positional checks remain independent.
        if self.is(ty, c::INVOCATION_EXPRESSION) {
            let explicit = self.targets(feature, QueryKind::RedefinedFeatures);
            if !explicit.value.is_empty() {
                positions.retain(|(p, _)| !matches!(p, Position::Parameter));
            }
            out.merge(explicit);
        }
        let mut supers = self.targets(ty, QueryKind::DirectSpecializations);
        let library = self.library_specializations(ty);
        supers.value.extend(library.value.iter().copied());
        supers.merge(library);
        for (position, rule) in positions {
            let own = self.positioned_features(&mut out, ty, position);
            let Some(index) = own.iter().position(|f| *f == feature) else {
                continue;
            };
            for &general in &supers.value {
                let inherited = self.positioned_features(&mut out, general, position);
                if let Some(&target) = inherited.get(index).filter(|&&target| target != feature) {
                    out.value.push(target);
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
                        QueryKind::RedefinedFeatures,
                        feature,
                        target,
                        rule,
                        premises,
                    );
                }
            }
        }
        out.merge(supers);
        out.value.sort();
        out.value.dedup();
        out
    }
}
