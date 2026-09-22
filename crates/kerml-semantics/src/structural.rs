//! Small derived operations with their own completeness and evidence boundaries.
use crate::*;
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, PropertyId};

/// Structural expressions, with no claim to have evaluated their values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiplicityBounds {
    pub lower: Option<ElementId>,
    pub upper: Option<ElementId>,
    pub bound: Vec<ElementId>,
}

impl KerMlQueries<'_> {
    /// Structural featuring context of a Multiplicity, with no bound evaluation.
    /// Operational v9 follows owningNamespace and, for an owned cross Feature,
    /// its owning end Feature. Historical profiles retain their existing domain.
    pub fn multiplicity_featuring_context(
        &self,
        multiplicity: ElementId,
    ) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self
            .checked::<views::Multiplicity, _>(&mut out, multiplicity)
            .is_none()
        {
            return out;
        }
        let domains = self.featuring_types(multiplicity);
        out.value = domains.value.clone();
        out.merge(domains);
        for domain in out.value.clone() {
            out.prove(
                QueryKind::MultiplicityFeaturingContext,
                multiplicity,
                domain,
                Rule::MultiplicityFeaturingContext,
                evidence(&out),
            );
        }
        out
    }

    // One canonical v9 context-source operation, used by the iterative featuring
    // traversal. It selects ownership structurally, never by a lexical namespace
    // or by ordering candidate ElementIds. Traversal itself supplies the domain.
    pub(crate) fn multiplicity_context_feature(
        &self,
        multiplicity: ElementId,
    ) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        out.search_dependencies
            .insert(SearchDependency::ValidationRule(
                "agentique-kerml10-multiplicity-context/1",
            ));
        let namespace = self.multiplicity_owning_namespace(multiplicity);
        let owner = namespace.value.filter(|owner| self.is(*owner, c::FEATURE));
        out.merge(namespace);
        let Some(owner) = owner else {
            return out;
        };
        // Selection of an owned cross Feature also depends on its owning
        // namespace. An explicit unsuccessful projection cannot become a
        // definitive ordinary-Feature case merely because raw links exist.
        let containing_namespace = self.multiplicity_owning_namespace(owner);
        let end = containing_namespace.value;
        out.merge(containing_namespace);
        if out.completeness != Completeness::Complete {
            return out;
        }
        let cross = self.is_owned_cross_feature(owner);
        if cross.completeness == Completeness::Complete {
            if cross.value {
                out.search_dependencies
                    .insert(SearchDependency::ValidationRule(
                        "agentique-kerml10-cross-multiplicity-context/1",
                    ));
                if let Some(end) = end.filter(|end| self.is(*end, c::FEATURE)) {
                    out.value = Some(end);
                } else {
                    out.problem(
                        Completeness::Invalid,
                        "KQ_MULTIPLICITY_CROSS_OWNER",
                        owner,
                        "An owned cross Feature must have an owning end Feature",
                    );
                }
            } else {
                out.value = Some(owner);
            }
        }
        out.merge(cross);
        out
    }

    // Derived owningNamespace may be computed structurally from its canonical
    // membership, but an explicit failed computation remains authoritative input.
    fn multiplicity_owning_namespace(&self, element: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self.query_projection_failure(&mut out, element, p::ELEMENT_OWNING_NAMESPACE) {
            return out;
        }
        let relationship = self.owning_relationship(element);
        let membership = relationship
            .value
            .filter(|r| self.is(*r, c::OWNING_MEMBERSHIP));
        out.merge(relationship);
        let Some(membership) = membership else {
            return out;
        };
        if self.query_projection_failure(
            &mut out,
            membership,
            p::MEMBERSHIP_MEMBERSHIP_OWNING_NAMESPACE,
        ) {
            return out;
        }
        let owner = self.owner(element);
        if owner.completeness == Completeness::Complete {
            out.value = owner.value;
        }
        out.merge(owner);
        out
    }

    pub(crate) fn query_projection_failure<T>(
        &self,
        out: &mut QueryResult<T>,
        element: ElementId,
        property: PropertyId,
    ) -> bool {
        use agq_kernel::derived::PropertyState;
        self.property(out, element, property);
        if let Ok(PropertyState::Incomplete(failure) | PropertyState::Invalid(failure)) =
            self.model().property_state(element, property)
        {
            self.accept::<_, ()>(
                out,
                element,
                Err(agq_kerml::ViewError::ComputationFailure {
                    element,
                    property,
                    failure: Box::new(failure.clone()),
                }),
            );
            true
        } else {
            false
        }
    }

    /// KerML deriveMultiplicityRangeLowerBound/UpperBound/Bound in owned-member order.
    pub fn multiplicity_bounds(&self, multiplicity: ElementId) -> QueryResult<MultiplicityBounds> {
        let mut out = self.result(MultiplicityBounds {
            lower: None,
            upper: None,
            bound: vec![],
        });
        if self
            .checked::<views::MultiplicityRange, _>(&mut out, multiplicity)
            .is_none()
        {
            return out;
        }
        let memberships = self.memberships(multiplicity);
        let mut expressions = vec![];
        for &membership in &memberships.value {
            if !self.is(membership, c::OWNING_MEMBERSHIP) {
                continue;
            }
            let member = self.member(membership);
            expressions.extend(member.value.filter(|&e| self.is(e, c::EXPRESSION)));
            out.merge(member);
        }
        out.merge(memberships);
        if self
            .context()
            .pending_namespace_scopes
            .contains(&multiplicity)
        {
            out.problem(
                Completeness::Incomplete,
                "KQ_MULTIPLICITY_BOUNDS",
                multiplicity,
                "Pending members do not establish ordered multiplicity bounds",
            );
        }
        if out.completeness == Completeness::Complete {
            out.value.upper = expressions.get(usize::from(expressions.len() > 1)).copied();
            if expressions.len() > 1 {
                out.value.lower = expressions.first().copied();
            }
            out.value.bound.extend(out.value.lower);
            out.value.bound.extend(out.value.upper);
            for value in out.value.bound.clone() {
                out.prove(
                    QueryKind::MultiplicityBound,
                    multiplicity,
                    value,
                    Rule::MultiplicityBound,
                    evidence(&out),
                );
            }
        }
        out
    }
    /// deriveFeatureFeatureTarget: self or the last canonical chainingFeature.
    /// An incomplete chain never supplies a definitive terminal.
    pub fn feature_target(&self, feature: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self
            .checked::<views::Feature, _>(&mut out, feature)
            .is_none()
        {
            return out;
        }
        let chain = self.chaining_features(feature);
        let terminal = chain.value.last().copied().unwrap_or(feature);
        out.merge(chain);
        if out.completeness == Completeness::Complete {
            out.value = Some(terminal);
            let evidence = evidence(&out);
            out.prove(
                QueryKind::FeatureTarget,
                feature,
                terminal,
                Rule::FeatureTarget,
                evidence,
            );
        }
        out
    }

    /// FeatureValue::featureWithValue is the canonical owning Feature.
    /// This operation does not infer ownership from lexical source positions.
    pub fn feature_with_value(&self, value: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self
            .checked::<views::FeatureValue, _>(&mut out, value)
            .is_none()
        {
            return out;
        }
        let owner = self.owning_related_element(value);
        if let Some(feature) = owner.value {
            if self.is(feature, c::FEATURE) {
                out.value = Some(feature);
            } else {
                out.problem(
                    Completeness::Invalid,
                    "KQ_VALUE_OWNER",
                    value,
                    "FeatureValue must be owned by a Feature",
                );
            }
        } else {
            out.problem(
                Completeness::Incomplete,
                "KQ_VALUE_OWNER",
                value,
                "FeatureValue has no established owning Feature",
            );
        }
        out.merge(owner);
        if let Some(feature) = out.value {
            let evidence = evidence(&out);
            out.prove(
                QueryKind::FeatureWithValue,
                value,
                feature,
                Rule::FeatureWithValue,
                evidence,
            );
        }
        out
    }

    /// The unique structural result of an Expression or Function. A model with
    /// multiple result memberships is invalid, so no arbitrary first ID is used.
    pub fn structural_result(&self, expression: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if !self.is(expression, c::EXPRESSION) && !self.is(expression, c::FUNCTION) {
            out.problem(
                Completeness::Invalid,
                "KQ_RESULT_OWNER",
                expression,
                "A structural result requires an Expression or Function",
            );
            return out;
        }
        let results = self.result_parameters(expression);
        if results.value.len() > 1 {
            out.problem(
                Completeness::Invalid,
                "KQ_RESULT_ARITY",
                expression,
                "Multiple result memberships",
            );
        } else if results.completeness == Completeness::Complete {
            out.value = results.value.first().copied();
        }
        out.merge(results);
        out
    }
}

fn evidence<T>(out: &QueryResult<T>) -> Vec<Evidence> {
    out.positive_dependencies
        .iter()
        .copied()
        .map(Evidence::Fact)
        .chain(
            out.search_dependencies
                .iter()
                .cloned()
                .map(Evidence::Search),
        )
        .collect()
}
