use crate::{projection::*, *};
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries, QueryResult};
use agq_kernel::{DocumentId, SourceRevisionId, provenance::FactKey};
use agq_modeling_workspace::ProjectRevision;
use std::collections::BTreeSet;

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

/// Contextual semantic inspection. Raw metamodel slot tables are intentionally absent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementInspector {
    pub revision_id: ProjectRevisionId,
    pub element: ViewNode,
    pub owner: Option<FeatureSummary>,
    pub effective_types: Vec<FeatureSummary>,
    pub owned_features: Vec<FeatureSummary>,
    pub effective_features: Vec<FeatureSummary>,
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
    let element = node(revision, id)?;
    let model = revision
        .semantic_model()
        .ok_or(ViewError::Unavailable(revision.revision()))?;
    let q = revision
        .kerml_queries()
        .map_err(|e| ViewError::Query(format!("{e:?}")))?;
    let mut queries = vec![];
    let owner_query = q.owner(id);
    let owner = owner_query.value.map(|id| summary(model, id));
    queries.push(QuerySummary::of("Owner", &owner_query));
    let mut effective_types = vec![];
    let mut owned_features = vec![];
    let mut effective_features = vec![];
    let mut specializations = vec![];
    let mut subsettings = vec![];
    let mut redefinitions = vec![];
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
    let (relationships, connection_queries) =
        inspect_relationships(&q, revision.revision(), &local, id);
    queries.extend(connection_queries);
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
    let profile = revision
        .sysml_queries()
        .map(|q| q.context().dependencies.sysml_profile.id().to_owned())
        .unwrap_or_else(|_| q.context().baseline_profile_id.to_owned());
    Ok(ElementInspector {
        revision_id: revision.revision(),
        element,
        owner,
        effective_types,
        owned_features,
        effective_features,
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

fn inspect_relationships(
    q: &KerMlQueries<'_>,
    revision: ProjectRevisionId,
    local: &BTreeSet<ElementId>,
    id: ElementId,
) -> (Vec<ViewEdge>, Vec<QuerySummary>) {
    let mut queries = vec![];
    let mut relationships = graph_edges(q.model(), revision, local);
    relationships.extend(connector_edges(q, revision, local, |connector, answer| {
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
            let (relationships, summaries) = inspect_relationships(&q, revision, &local, selected);
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
        let (relationships, queries) =
            inspect_relationships(&q, ProjectRevisionId::from_u128(94), &local, id(1));
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
