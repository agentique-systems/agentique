//! Small derived operations with their own completeness and evidence boundaries.
use crate::*;
use agq_kerml::{classes as c, views};
use agq_kernel::ElementId;

impl KerMlQueries<'_> {
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
