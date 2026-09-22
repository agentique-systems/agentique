//! Feature typing closure over canonical relationships, excluding cross-subsettings.
use crate::*;
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, provenance::FactKey};
use std::collections::BTreeSet;

impl KerMlQueries<'_> {
    /// Feature::typingFeatures over the current canonical graph. CrossSubsetting
    /// never contributes to typing; a conjugator replaces other typing Features.
    pub fn typing_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self
            .checked::<views::Feature, _>(&mut out, feature)
            .is_none()
        {
            return out;
        }
        if self
            .context()
            .pending_specialization_scopes
            .contains(&feature)
        {
            out.problem(
                Completeness::Incomplete,
                "KQ_TYPING_POPULATION",
                feature,
                "Pending specializations may contribute typing Features",
            );
        }
        let owned = self.owned_relationships(feature);
        let conjugators: Vec<_> = owned
            .value
            .iter()
            .copied()
            .filter(|&r| self.is(r, c::CONJUGATION))
            .collect();
        out.merge(owned);
        if !conjugators.is_empty() {
            for r in conjugators {
                if let Some(ty) = self.read_reference(&mut out, r, p::CONJUGATION_ORIGINAL_TYPE) {
                    if self.is(ty, c::FEATURE) {
                        out.value.push(ty);
                    }
                } else {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_TYPING_CONJUGATION",
                        r,
                        "Missing original Type",
                    );
                }
            }
            return out;
        }
        let mut relationships = self.incoming_source_relationships(
            &mut out,
            feature,
            c::SUBSETTING,
            p::SUBSETTING_SUBSETTING_FEATURE,
        );
        let owned = self.owned_relationships(feature);
        relationships.extend(owned.value.iter().copied());
        out.merge(owned);
        for relationship in relationships {
            if !self.is(relationship, c::SUBSETTING) || self.is(relationship, c::CROSS_SUBSETTING) {
                continue;
            }
            let source = if self.is(relationship, c::REFERENCE_SUBSETTING) {
                let owner = self.owning_related_element(relationship);
                let source = owner.value;
                out.merge(owner);
                source
            } else {
                self.read_reference(&mut out, relationship, p::SUBSETTING_SUBSETTING_FEATURE)
            };
            if source != Some(feature) {
                continue;
            }
            self.fact(&mut out, FactKey::Element(relationship));
            if let Some(target) =
                self.read_reference(&mut out, relationship, p::SUBSETTING_SUBSETTED_FEATURE)
            {
                out.value.push(target);
            } else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_TYPING_SUBSETTING",
                    relationship,
                    "Missing subsetted Feature",
                );
            }
        }
        let chain = self.chaining_features(feature);
        if let Some(&last) = chain.value.last() {
            out.value.push(last);
        }
        out.merge(chain);
        if !self.context().options.exclude_implied {
            let bases = self.metaclass_library_bases(feature);
            out.value.extend(
                bases
                    .value
                    .iter()
                    .copied()
                    .filter(|&b| self.is(b, c::FEATURE)),
            );
            out.merge(bases);
        }
        // typing is an identity-set computation; this sort does not define ownership order.
        out.value.sort();
        out.value.dedup();
        let premises = typing_evidence(&out);
        for value in out.value.clone() {
            out.prove(
                QueryKind::TypingFeatures,
                feature,
                value,
                Rule::TypingFeatures,
                premises.clone(),
            );
        }
        out
    }

    /// deriveFeatureType on the current canonical graph. Implied relationships
    /// are included when present in the graph; this does not certify that all
    /// semantic producers have run. Closure is iterative and handles cycles.
    pub fn feature_types(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut seen = BTreeSet::new();
        let mut pending = vec![feature];
        let mut types = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            let direct = self.direct_feature_types(current);
            types.extend(direct.value.iter().copied());
            out.merge(direct);
            let features = self.typing_features(current);
            pending.extend(features.value.iter().copied());
            out.merge(features);
        }
        // Reachability over canonical specialization facts suffices to remove
        // redundant general types without recursing into type-dependent producers.
        let mut redundant = BTreeSet::new();
        for &ty in &types {
            let mut seen = BTreeSet::from([ty]);
            let mut pending = vec![ty];
            while let Some(current) = pending.pop() {
                let supers = self.targets(current, QueryKind::DirectSpecializations);
                for &general in &supers.value {
                    if general != ty && types.contains(&general) {
                        redundant.insert(general);
                    }
                    if seen.insert(general) {
                        pending.push(general);
                    }
                }
                out.merge(supers);
            }
        }
        out.value = types.difference(&redundant).copied().collect();
        let premises = typing_evidence(&out);
        for value in out.value.clone() {
            out.prove(
                QueryKind::FeatureTypes,
                feature,
                value,
                Rule::FeatureTypes,
                premises.clone(),
            );
        }
        out
    }
}

fn typing_evidence<T>(out: &QueryResult<T>) -> Vec<Evidence> {
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
