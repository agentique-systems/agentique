//! Ordered association and connector projections over canonical relationships.
use crate::*;
use agq_kerml::views;
use agq_kernel::ElementId;
use std::collections::BTreeSet;

/// Association end order is retained even when several ends have the same Type.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AssociationStructure {
    pub ends: Vec<ElementId>,
    pub related_types: Vec<ElementId>,
    pub source_type: Option<ElementId>,
    /// An ordered set: first occurrence order, with duplicates suppressed.
    pub target_types: Vec<ElementId>,
}

/// Connector endpoint identities and their structurally determined default domain.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConnectorStructure {
    pub ends: Vec<ElementId>,
    pub related_features: Vec<ElementId>,
    pub source_feature: Option<ElementId>,
    /// An ordered set: first occurrence order, with duplicates suppressed.
    pub target_features: Vec<ElementId>,
    pub default_featuring_type: Option<ElementId>,
}

impl KerMlQueries<'_> {
    /// deriveAssociationRelatedType/SourceType/TargetType. These are query
    /// projections of the existing end and typing relationships, not new links.
    pub fn association_structure(
        &self,
        association: ElementId,
    ) -> QueryResult<AssociationStructure> {
        let mut out = self.result(AssociationStructure::default());
        if self
            .checked::<views::Association, _>(&mut out, association)
            .is_none()
        {
            return out;
        }
        let ends = self.structural_end_features(association);
        out.value.ends = ends.value.clone();
        for &end in &ends.value {
            let types = self.feature_types(end);
            out.value.related_types.extend(types.value.iter().copied());
            out.merge(types);
        }
        out.merge(ends);
        if out.completeness == Completeness::Complete {
            out.value.source_type = out.value.related_types.first().copied();
            let mut seen = BTreeSet::new();
            out.value.target_types.extend(
                out.value
                    .related_types
                    .iter()
                    .skip(1)
                    .copied()
                    .filter(|id| seen.insert(*id)),
            );
        }
        out
    }

    /// deriveConnectorRelatedFeature/SourceFeature/TargetFeature and
    /// DefaultFeaturingType. All navigation uses canonical ReferenceSubsetting
    /// and TypeFeaturing facts, including derived association occurrences.
    pub fn connector_structure(&self, connector: ElementId) -> QueryResult<ConnectorStructure> {
        let mut out = self.result(ConnectorStructure::default());
        if self
            .checked::<views::Connector, _>(&mut out, connector)
            .is_none()
        {
            return out;
        }
        let ends = self.structural_end_features(connector);
        out.value.ends = ends.value.clone();
        out.merge(ends);
        let related = self.connector_endpoints(connector);
        out.value.related_features = related.value.clone();
        out.merge(related);
        if out.completeness == Completeness::Complete {
            out.value.source_feature = out.value.related_features.first().copied();
            let mut seen = BTreeSet::new();
            out.value.target_features.extend(
                out.value
                    .related_features
                    .iter()
                    .skip(1)
                    .copied()
                    .filter(|id| seen.insert(*id)),
            );
            let domain = self.common_connector_context(&out.value.related_features);
            out.value.default_featuring_type = domain.value;
            out.merge(domain);
        }
        out
    }
}
