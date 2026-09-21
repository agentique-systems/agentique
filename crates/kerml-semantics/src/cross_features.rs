//! Separately explainable cross-feature operations over ordered canonical ownership.
use crate::*;
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, provenance::FactKey};

impl KerMlQueries<'_> {
    /// Feature::ownedCrossFeature, with the exact KERML11-1 profile boundary.
    ///
    /// Selects only directly owned members, in normative ownedMembership order.
    /// Incomplete ordering/population evidence never establishes a selected ID.
    /// Exclusions use metaclass conformance, independently of record origin.
    pub fn owned_cross_feature(&self, feature: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let Some(view) = self.checked::<views::Feature, _>(&mut out, feature) else {
            return out;
        };
        self.property(&mut out, feature, p::FEATURE_IS_END);
        if self.accept(&mut out, feature, view.is_end()) != Some(true) {
            return out;
        }
        let owner = self.owning_type(feature);
        let has_owner = owner.value.is_some();
        out.merge(owner);
        if !has_owner {
            return out;
        }
        let corrected = self
            .context()
            .options
            .baseline_profile
            .corrects_owned_cross_feature();
        out.search_dependencies
            .insert(SearchDependency::ValidationRule(if corrected {
                "agentique-kerml10-owned-cross-feature/1"
            } else {
                "KerML-1.0-Feature-ownedCrossFeature"
            }));
        if self.context().pending_namespace_scopes.contains(&feature) {
            out.problem(
                Completeness::Incomplete,
                "KQ_CROSS_MEMBER_POPULATION",
                feature,
                "Pending owned memberships do not establish the first eligible cross Feature",
            );
        }
        let memberships = self.memberships(feature);
        // Do not use an arbitrary candidate from an incomplete ordered projection.
        let ordered = memberships.value.clone();
        out.merge(memberships);
        if out.completeness != Completeness::Complete {
            return out;
        }
        for membership in ordered {
            self.fact(&mut out, FactKey::Element(membership));
            if !self.is(membership, c::OWNING_MEMBERSHIP)
                || self.is(membership, c::FEATURE_MEMBERSHIP)
                || (corrected && self.is(membership, c::FEATURE_VALUE))
            {
                continue;
            }
            let member = self.member(membership);
            let value = member.value;
            out.merge(member);
            if out.completeness != Completeness::Complete {
                return out;
            }
            let Some(value) = value else { continue };
            if !self.is(value, c::FEATURE)
                || self.is(value, c::MULTIPLICITY)
                || self.is(value, c::METADATA_FEATURE)
                || (corrected && self.is(value, c::BINDING_CONNECTOR))
                // The published body tests the member, not its membership.
                || (!corrected && self.is(value, c::FEATURE_VALUE))
            {
                continue;
            }
            out.value = Some(value);
            let premises = cross_evidence(&out);
            out.prove(
                QueryKind::OwnedCrossFeature,
                feature,
                value,
                if corrected {
                    Rule::OperationalOwnedCrossFeatureV1
                } else {
                    Rule::PublishedOwnedCrossFeature
                },
                premises,
            );
            break;
        }
        out
    }

    /// Feature::isOwnedCrossFeature. Ownership and selection remain distinct queries.
    pub fn is_owned_cross_feature(&self, feature: ElementId) -> QueryResult<bool> {
        let mut out = self.result(false);
        if self
            .checked::<views::Feature, _>(&mut out, feature)
            .is_none()
        {
            return out;
        }
        let relationship = self.owning_relationship(feature);
        let membership = relationship
            .value
            .filter(|&id| self.is(id, c::OWNING_MEMBERSHIP));
        out.merge(relationship);
        if let Some(membership) = membership {
            let owner = self.owning_related_element(membership);
            if let Some(owner) = owner.value.filter(|&id| self.is(id, c::FEATURE)) {
                let selected = self.owned_cross_feature(owner);
                out.value = selected.value == Some(feature);
                out.merge(selected);
            }
            out.merge(owner);
        }
        out
    }

    /// deriveFeatureOwnedCrossSubsetting: first owned CrossSubsetting in semantic order.
    /// The separate cardinality constraint rejects multiple owned cross-subsettings.
    pub fn owned_cross_subsetting(&self, feature: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        if self
            .checked::<views::Feature, _>(&mut out, feature)
            .is_none()
        {
            return out;
        }
        let owned = self.owned_relationships(feature);
        let selected = owned
            .value
            .iter()
            .copied()
            .find(|&r| self.is(r, c::CROSS_SUBSETTING));
        out.merge(owned);
        if self
            .context()
            .pending_specialization_scopes
            .contains(&feature)
        {
            out.problem(
                Completeness::Incomplete,
                "KQ_CROSS_SUBSETTING_POPULATION",
                feature,
                "Pending specializations may affect the owned cross-subsetting projection",
            );
        }
        if out.completeness == Completeness::Complete {
            out.value = selected;
            if let Some(value) = selected {
                self.fact(&mut out, FactKey::Element(value));
                let premises = cross_evidence(&out);
                out.prove(
                    QueryKind::OwnedCrossSubsetting,
                    feature,
                    value,
                    Rule::OwnedCrossSubsetting,
                    premises,
                );
            }
        }
        out
    }

    /// deriveFeatureCrossFeature: second chainingFeature of the crossed Feature.
    /// Does not substitute ownedCrossFeature for a missing canonical crossing graph.
    pub fn cross_feature(&self, feature: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let cross = self.owned_cross_subsetting(feature);
        if let Some(cross) = cross.value {
            if let Some(target) =
                self.read_reference(&mut out, cross, p::CROSS_SUBSETTING_CROSSED_FEATURE)
            {
                let chain = self.chaining_features(target);
                out.value = chain.value.get(1).copied();
                out.merge(chain);
            } else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_CROSSED_FEATURE",
                    cross,
                    "Missing crossed Feature",
                );
            }
        }
        out.merge(cross);
        if out.completeness != Completeness::Complete {
            out.value = None;
        }
        if let Some(value) = out.value {
            let premises = cross_evidence(&out);
            out.prove(
                QueryKind::CrossFeature,
                feature,
                value,
                Rule::CrossFeature,
                premises,
            );
        }
        out
    }
}

fn cross_evidence<T>(out: &QueryResult<T>) -> Vec<Evidence> {
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
