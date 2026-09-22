//! Ordered association and connector projections over canonical relationships.
use crate::*;
use agq_kerml::{classes as c, properties as p, views};
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
    /// Published deriveConnectorRelatedFeature: collect referenced Features in
    /// connector-end order, filtering ends with no owned ReferenceSubsetting.
    /// A known absence is distinct from an unresolved or ambiguous relationship.
    /// Concrete cardinality and strict binding endpoints are separate contracts.
    pub fn connector_related_features(&self, connector: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self
            .checked::<views::Connector, _>(&mut out, connector)
            .is_none()
        {
            return out;
        }
        let ends = self.structural_end_features(connector);
        for &end in &ends.value {
            if self.query_projection_failure(&mut out, end, p::FEATURE_OWNED_REFERENCE_SUBSETTING) {
                continue;
            }
            let owned = self.owned_relationships(end);
            let references: Vec<_> = owned
                .value
                .iter()
                .copied()
                .filter(|r| self.is(*r, c::REFERENCE_SUBSETTING))
                .collect();
            out.merge(owned);
            if self.context().pending_specialization_scopes.contains(&end)
                || self.context().pending_namespace_scopes.contains(&end)
            {
                out.problem(Completeness::Incomplete, "KQ_CONNECTOR_REFERENCE_POPULATION", end,
                    "Pending end relationships do not establish the owned ReferenceSubsetting population");
            }
            match references.as_slice() {
                [] => {}
                [reference] => {
                    if let Some(view) =
                        self.checked::<views::ReferenceSubsetting, _>(&mut out, *reference)
                    {
                        self.property(
                            &mut out,
                            *reference,
                            p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
                        );
                        if let Some(feature) =
                            self.accept(&mut out, *reference, view.referenced_feature())
                        {
                            out.value.push(feature);
                        } else {
                            out.problem(Completeness::Incomplete, "KQ_CONNECTOR_ENDPOINT", *reference,
                                "An owned ReferenceSubsetting has no established referenced Feature");
                        }
                    }
                }
                _ => out.problem(
                    Completeness::Incomplete,
                    "KQ_CONNECTOR_ENDPOINT",
                    end,
                    "Multiple owned ReferenceSubsettings do not establish a unique related Feature",
                ),
            }
        }
        out.merge(ends);
        out
    }

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
        self.project_connector_structure(connector, false)
    }

    /// Published canonical Connector projections. Unlike the historical strict
    /// endpoint contract, a determinate missing end reference contributes no
    /// related Feature; abstract library Flows therefore have a complete graph.
    pub fn connector_related_structure(
        &self,
        connector: ElementId,
    ) -> QueryResult<ConnectorStructure> {
        self.project_connector_structure(connector, true)
    }

    fn project_connector_structure(
        &self,
        connector: ElementId,
        filter_absent: bool,
    ) -> QueryResult<ConnectorStructure> {
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
        let related = if filter_absent {
            self.connector_related_features(connector)
        } else {
            self.connector_endpoints(connector)
        };
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
