use crate::{projection::*, *};
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::QueryResult;
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
    let relationships = graph_edges(model, revision.revision(), &local)
        .into_iter()
        .filter(|e| e.source == id || e.target == id || e.relationship_id == Some(id))
        .collect();
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
