use crate::{projection::*, *};
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries, QueryResult};
use agq_kernel::{DocumentId, SourceRevisionId, provenance::FactKey};
use agq_modeling_workspace::ProjectRevision;
use std::collections::{BTreeMap, BTreeSet};

/// Exact half-open UTF-8 source range for the selected revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub document_id: DocumentId,
    pub path: Option<String>,
    pub source_revision_id: SourceRevisionId,
    pub start: u64,
    pub end: u64,
}

/// Display summary of a real query contract, not a reusable semantic cache proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuerySummary {
    pub name: String,
    pub completeness: String,
    pub diagnostics: Vec<String>,
    pub positive_dependency_count: usize,
    pub search_dependency_count: usize,
}
impl QuerySummary {
    fn of<T>(name: &str, query: &QueryResult<T>) -> Self {
        Self {
            name: name.into(),
            completeness: format!("{:?}", query.completeness),
            diagnostics: query
                .diagnostics
                .iter()
                .map(|d| format!("{}: {}", d.code, d.message))
                .collect(),
            positive_dependency_count: query.positive_dependencies.len(),
            search_dependency_count: query.search_dependencies.len(),
        }
    }
}

/// Revision-bound provenance for an original canonical feature, independent of
/// whether the current spatial projection includes that feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureProvenance {
    /// Exact accepted-library membership takes precedence over record origin.
    pub origin: ViewOrigin,
    pub source_available: bool,
    /// A declared name or short name exists; a metaclass display fallback is not a name.
    pub has_declared_name: bool,
}

/// Contextual semantic inspection. Raw metamodel slot tables are intentionally absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementInspector {
    pub revision_id: ProjectRevisionId,
    pub element: ViewNode,
    pub owner: Option<FeatureSummary>,
    pub effective_types: Vec<FeatureSummary>,
    pub owned_features: Vec<FeatureSummary>,
    pub effective_features: Vec<FeatureSummary>,
    /// Provenance of owned/effective features in this same revision. Missing
    /// entries mean unknown provenance, including older serialized inspectors.
    #[serde(default)]
    pub feature_provenance: BTreeMap<ElementId, FeatureProvenance>,
    pub specializations: Vec<FeatureSummary>,
    pub subsettings: Vec<FeatureSummary>,
    pub redefinitions: Vec<FeatureSummary>,
    pub relationships: Vec<ViewEdge>,
    /// Multiplicity expression identities; no evaluation semantics are invented.
    pub multiplicity: Option<FeatureSummary>,
    pub source: Option<SourceLocation>,
    pub queries: Vec<QuerySummary>,
    pub profile: String,
}

/// Execute effective queries only for the selected canonical identity.
pub fn inspect(revision: &ProjectRevision, id: ElementId) -> Result<ElementInspector, ViewError> {
    let mut timing = ViewProfile::new("inspect", revision, Some(id), None);
    let result = inspect_observed(revision, id, &mut timing);
    if let Ok(inspector) = &result {
        timing.size("relationships", inspector.relationships.len());
        timing.size("query_summaries", inspector.queries.len());
        timing.size("owned_features", inspector.owned_features.len());
        timing.size("effective_features", inspector.effective_features.len());
        timing.size("effective_types", inspector.effective_types.len());
    }
    timing.finish(result.is_ok());
    result
}

fn inspect_observed(
    revision: &ProjectRevision,
    id: ElementId,
    timing: &mut ViewProfile,
) -> Result<ElementInspector, ViewError> {
    let element = timing.measure("node_mapping", None, || node(revision, id))?;
    let model = revision
        .semantic_model()
        .ok_or(ViewError::Unavailable(revision.revision()))?;
    timing.size("canonical_elements", model.len());
    timing.size(
        "standard_elements",
        revision.accepted_sysml().overlay().model().len(),
    );
    let q = timing.measure("kerml_context", None, || {
        revision
            .kerml_queries()
            .map_err(|e| ViewError::Query(format!("{e:?}")))
    })?;
    let mut queries = vec![];
    let owner_started = timing.start();
    let owner_query = q.owner(id);
    let owner = owner_query.value.map(|id| summary(model, id));
    queries.push(QuerySummary::of("Owner", &owner_query));
    timing.end("owner_query", None, owner_started);
    let mut effective_types = vec![];
    let mut owned_features = vec![];
    let mut effective_features = vec![];
    let mut specializations = vec![];
    let mut subsettings = vec![];
    let mut redefinitions = vec![];
    let feature_started = timing.start();
    if is(model, id, c::FEATURE) {
        let types = q.feature_types(id);
        effective_types = types.value.iter().map(|id| summary(model, *id)).collect();
        queries.push(QuerySummary::of("Effective types", &types));
        let subsets = q.subsetted_features(id);
        subsettings = subsets.value.iter().map(|id| summary(model, *id)).collect();
        queries.push(QuerySummary::of("Subsettings", &subsets));
        let redefines = q.redefined_features(id);
        redefinitions = redefines
            .value
            .iter()
            .map(|id| summary(model, *id))
            .collect();
        queries.push(QuerySummary::of("Redefinitions", &redefines));
    }
    timing.end("feature_queries", None, feature_started);
    let type_started = timing.start();
    if is(model, id, c::TYPE) {
        let owned = q.direct_features(id);
        owned_features = owned.value.iter().map(|id| summary(model, *id)).collect();
        queries.push(QuerySummary::of("Owned features", &owned));
        let effective = q.effective_features(id);
        effective_features = effective
            .value
            .iter()
            .map(|id| summary(model, *id))
            .collect();
        queries.push(QuerySummary::of("Effective features", &effective));
        let general = q.supertypes(id);
        specializations = general.value.iter().map(|id| summary(model, *id)).collect();
        queries.push(QuerySummary::of("Specializations", &general));
    }
    timing.end("type_queries", None, type_started);
    let provenance_started = timing.start();
    let feature_provenance = owned_features
        .iter()
        .chain(&effective_features)
        .filter_map(|feature| {
            feature_provenance(
                model,
                revision.accepted_sysml().overlay().model(),
                feature.id,
                revision
                    .source_for_fact(FactKey::Element(feature.id))
                    .is_some(),
            )
            .map(|provenance| (feature.id, provenance))
        })
        .collect();
    timing.end("feature_provenance", None, provenance_started);
    let local_started = timing.start();
    let local: BTreeSet<_> = model
        .elements()
        .filter(|r| {
            revision
                .accepted_sysml()
                .overlay()
                .model()
                .element(r.id())
                .is_none()
        })
        .map(|r| r.id())
        .collect();
    timing.end("local_population", None, local_started);
    timing.size("local_elements", local.len());
    let relationships_started = timing.start();
    let (relationships, connection_queries) =
        inspect_relationships(&q, revision.revision(), &local, id, timing);
    queries.extend(connection_queries);
    timing.end("relationship_queries", None, relationships_started);
    let source_started = timing.start();
    let source = revision
        .source_for_fact(FactKey::Element(id))
        .map(|source| SourceLocation {
            document_id: source.document,
            source_revision_id: source.revision,
            start: source.range.start(),
            end: source.range.end(),
            path: revision
                .documents()
                .find(|(_, d)| d.id() == source.document)
                .map(|(path, _)| path.to_owned()),
        });
    timing.end("source_mapping", None, source_started);
    let profile = timing.measure("sysml_profile_context", None, || {
        revision
            .sysml_queries()
            .map(|q| q.context().dependencies.sysml_profile.id().to_owned())
            .unwrap_or_else(|_| q.context().baseline_profile_id.to_owned())
    });
    Ok(ElementInspector {
        revision_id: revision.revision(),
        element,
        owner,
        effective_types,
        owned_features,
        effective_features,
        feature_provenance,
        specializations,
        subsettings,
        redefinitions,
        relationships,
        multiplicity: refs(model, id, p::TYPE_MULTIPLICITY)
            .first()
            .map(|id| summary(model, *id)),
        source,
        queries,
        profile,
    })
}

fn feature_provenance(
    model: &agq_kernel::ModelView,
    accepted_standard: &agq_kernel::ModelView,
    id: ElementId,
    source_available: bool,
) -> Option<FeatureProvenance> {
    let record = model.element(id)?;
    Some(FeatureProvenance {
        origin: if accepted_standard.element(id).is_some() {
            ViewOrigin::Standard
        } else {
            provenance(record.origin())
        },
        source_available,
        has_declared_name: has_declared_name(model, id),
    })
}

fn inspect_relationships(
    q: &KerMlQueries<'_>,
    revision: ProjectRevisionId,
    local: &BTreeSet<ElementId>,
    id: ElementId,
    timing: &mut ViewProfile,
) -> (Vec<ViewEdge>, Vec<QuerySummary>) {
    let mut queries = vec![];
    let mut relationships =
        timing.measure("graph_extraction", Some("relationship_queries"), || {
            graph_edges(q.model(), revision, local)
        });
    timing.size("extracted_graph_edges", relationships.len());
    let connector_started = timing.start();
    let mut connector_queries = 0;
    relationships.extend(connector_edges(q, revision, local, |connector, answer| {
        connector_queries += 1;
        // An unresolved connector might touch this selection. Retain its actual
        // incomplete contract rather than implying the incidence search is closed.
        if connector == id
            || answer.value.contains(&id)
            || answer.completeness != Completeness::Complete
        {
            queries.push(QuerySummary::of(
                &format!(
                    "Connection endpoints: {} [{connector}]",
                    name(q.model(), connector)
                ),
                answer,
            ));
        }
    }));
    timing.end(
        "connector_queries",
        Some("relationship_queries"),
        connector_started,
    );
    timing.size("connector_queries", connector_queries);
    relationships
        .retain(|edge| edge.source == id || edge.target == id || edge.relationship_id == Some(id));
    relationships.sort_by(|a, b| a.id.cmp(&b.id));
    relationships.dedup_by(|a, b| a.id == b.id);
    (relationships, queries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::tests::{fixture_queries, references, semantic_fixture};
    use agq_kernel::{ElementRecord, value::Value};
    use agq_sysml::classes as sc;

    #[test]
    fn feature_metadata_uses_exact_standard_identity_and_canonical_declared_names() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::PORT_USAGE),
                (2, sc::PORT_USAGE),
                (3, sc::INTERFACE_USAGE),
                (4, sc::PORT_USAGE),
            ],
            &[
                (
                    1,
                    p::ELEMENT_DECLARED_NAME,
                    vec![Value::String("repositoryRevisions".into())],
                ),
                (
                    3,
                    p::ELEMENT_DECLARED_NAME,
                    vec![Value::String("Connector".into())],
                ),
                (
                    4,
                    p::ELEMENT_DECLARED_SHORT_NAME,
                    vec![Value::String("\u{0394}rev".into())],
                ),
            ],
        );
        // Imported membership is represented by canonical IDs, independent of
        // display labels and of how the accepted overlay recorded its origin.
        let standard = semantic_fixture(&[(1, sc::PORT_USAGE)], &[]);
        let id = ElementId::from_u128;
        let metadata = |element, source| {
            feature_provenance(snapshot.model(), standard.model(), id(element), source).unwrap()
        };
        assert_eq!(
            metadata(1, true),
            FeatureProvenance {
                origin: ViewOrigin::Standard,
                source_available: true,
                has_declared_name: true,
            }
        );
        assert_eq!(metadata(2, false).origin, ViewOrigin::Authored);
        assert!(!metadata(2, false).has_declared_name);
        assert_eq!(name(snapshot.model(), id(2)), "PortUsage");
        assert_eq!(metadata(3, false).origin, ViewOrigin::Authored);
        assert!(metadata(3, false).has_declared_name);
        assert!(!metadata(3, false).source_available);
        assert!(metadata(4, true).has_declared_name);
        assert!(feature_provenance(snapshot.model(), standard.model(), id(99), false).is_none());
    }

    #[test]
    fn inspector_and_scene_share_canonical_connection_and_query_completeness() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::PORT_USAGE),
                (2, sc::PORT_USAGE),
                (3, sc::INTERFACE_USAGE),
                (4, c::FEATURE),
                (5, c::FEATURE),
                (11, c::END_FEATURE_MEMBERSHIP),
                (12, c::END_FEATURE_MEMBERSHIP),
                (13, c::REFERENCE_SUBSETTING),
                (14, c::REFERENCE_SUBSETTING),
            ],
            &[
                (
                    3,
                    p::ELEMENT_DECLARED_NAME,
                    vec![Value::String("queryConnection".into())],
                ),
                (3, p::ELEMENT_OWNED_RELATIONSHIP, references(&[11, 12])),
                (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[4])),
                (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[5])),
                (4, p::FEATURE_IS_END, vec![Value::Boolean(true)]),
                (5, p::FEATURE_IS_END, vec![Value::Boolean(true)]),
                (4, p::ELEMENT_OWNED_RELATIONSHIP, references(&[13])),
                (5, p::ELEMENT_OWNED_RELATIONSHIP, references(&[14])),
                (13, p::SUBSETTING_SUBSETTED_FEATURE, references(&[1])),
                (14, p::SUBSETTING_SUBSETTED_FEATURE, references(&[2])),
            ],
        );
        let id = ElementId::from_u128;
        let q = fixture_queries(&snapshot);
        let local = snapshot.model().elements().map(ElementRecord::id).collect();
        let revision = ProjectRevisionId::from_u128(93);
        let endpoint_answer = q.connector_endpoints(id(3));
        assert_eq!(endpoint_answer.value, vec![id(1), id(2)]);
        let scene = connector_edges(&q, revision, &local, |_, _| {});
        assert_eq!(scene.len(), 1);
        assert_eq!((scene[0].source, scene[0].target), (id(1), id(2)));
        assert_eq!(scene[0].relationship_id, Some(id(3)));
        assert_eq!(scene[0].semantic_kind, "InterfaceUsage");
        assert!(!scene[0].directed);
        for selected in [id(1), id(2), id(3)] {
            let (relationships, summaries) =
                inspect_relationships(&q, revision, &local, selected, &mut ViewProfile::disabled());
            assert!(relationships.contains(&scene[0]));
            let summary = summaries
                .iter()
                .find(|summary| {
                    summary.name == format!("Connection endpoints: queryConnection [{}]", id(3))
                })
                .unwrap();
            assert_eq!(*summary, QuerySummary::of(&summary.name, &endpoint_answer));
        }
    }

    #[test]
    fn unresolved_connector_keeps_actual_incomplete_query_even_without_an_incident_edge() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::PORT_USAGE),
                (3, sc::INTERFACE_USAGE),
                (4, c::FEATURE),
                (11, c::END_FEATURE_MEMBERSHIP),
            ],
            &[
                (3, p::ELEMENT_OWNED_RELATIONSHIP, references(&[11])),
                (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[4])),
                (4, p::FEATURE_IS_END, vec![Value::Boolean(true)]),
            ],
        );
        let id = ElementId::from_u128;
        let q = fixture_queries(&snapshot);
        let local = snapshot.model().elements().map(ElementRecord::id).collect();
        let actual = q.connector_endpoints(id(3));
        assert_ne!(actual.completeness, Completeness::Complete);
        let (relationships, queries) = inspect_relationships(
            &q,
            ProjectRevisionId::from_u128(94),
            &local,
            id(1),
            &mut ViewProfile::disabled(),
        );
        assert!(
            !relationships
                .iter()
                .any(|edge| edge.family == RelationshipFamily::Connection)
        );
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0], QuerySummary::of(&queries[0].name, &actual));
        assert!(
            queries[0]
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("KQ_CONNECTOR_ENDPOINT"))
        );
    }
}
