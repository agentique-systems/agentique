//! Unoptimized body preserved from d404a8759891acc5d621dcad655fa4a981e17d63.
//! Source blob df8ae3617e355e6db9497d6d94a69e2eda768a10; test-only oracle.
use super::*;

pub(crate) fn inspect(
    revision: &ProjectRevision,
    id: ElementId,
) -> Result<ElementInspector, ViewError> {
    inspect_observed(revision, id, &mut ViewProfile::disabled())
}

pub(crate) fn current(
    revision: &ProjectRevision,
    id: ElementId,
) -> Result<ElementInspector, ViewError> {
    super::inspect_observed(revision, id, &mut ViewProfile::disabled())
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
