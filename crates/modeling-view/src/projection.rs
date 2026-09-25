use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries, QueryResult};
use agq_kernel::{
    ElementRecord, MetaclassId, ModelView, PropertyId,
    provenance::{DeclaredOrigin, FactKey, Origin},
    value::Value,
};
use agq_modeling_workspace::ProjectRevision;
use agq_sysml::classes as sc;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "query_reuse/projection_reference.rs"]
pub(crate) mod reference;

/// Project a current canonical graph without reconstruction, validation or mutation.
pub fn project(
    revision: &ProjectRevision,
    definition: &ViewDefinition,
) -> Result<ViewProjection, ViewError> {
    let mut profile = ViewProfile::new("project", revision, definition.focus, Some(definition));
    let result = project_observed(revision, definition, &mut profile);
    if let Ok(view) = &result {
        profile.size("projected_nodes", view.nodes.len());
        profile.size("projected_edges", view.edges.len());
        profile.size("groups", view.groups.len());
        profile.size("warnings", view.metadata.warnings.len());
    }
    profile.finish(result.is_ok());
    result
}

fn project_observed(
    revision: &ProjectRevision,
    definition: &ViewDefinition,
    profile: &mut ViewProfile,
) -> Result<ViewProjection, ViewError> {
    if definition.version != ViewDefinition::VERSION {
        return Err(ViewError::UnsupportedVersion(definition.version));
    }
    let model = revision
        .semantic_model()
        .ok_or(ViewError::Unavailable(revision.revision()))?;
    if let Some(focus) = definition.focus {
        model
            .element(focus)
            .ok_or(ViewError::MissingElement(focus))?;
    }
    let standard = revision.accepted_sysml().overlay().model();
    profile.size("canonical_elements", model.len());
    profile.size("standard_elements", standard.len());
    let local: BTreeSet<_> = profile.measure("local_population", None, || {
        model
            .elements()
            .filter(|r| standard.element(r.id()).is_none())
            .map(ElementRecord::id)
            .collect()
    });
    profile.size("local_elements", local.len());
    let mut edges = profile.measure("graph_extraction", None, || {
        graph_edges(model, revision.revision(), &local)
    });
    profile.size("extracted_graph_edges", edges.len());
    let mut warnings = vec![];
    // This evaluator never escapes the call or crosses immutable revisions.
    // Preserve lazy construction and the order of all semantic queries.
    let mut query_session = None;
    let mut context_constructions = 0;
    // Connector endpoints are effective semantic queries, never inferred from the layout.
    if definition
        .relationship_families
        .contains(&RelationshipFamily::Connection)
    {
        query_session = Some(profile.measure("connector_kerml_context", None, || {
            revision
                .kerml_queries()
                .map_err(|e| ViewError::Query(format!("{e:?}")))
        })?);
        context_constructions += 1;
        let queries = query_session
            .as_ref()
            .expect("constructed connector context");
        let mut connector_queries = 0;
        edges.extend(profile.measure("connector_queries", None, || {
            connector_edges(queries, revision.revision(), &local, |id, answer| {
                connector_queries += 1;
                query_warning(
                    &mut warnings,
                    &format!("Connector {} endpoints", name(model, id)),
                    answer,
                );
            })
        }));
        profile.size("connector_queries", connector_queries);
    }
    let selection_started = profile.start();
    let mut selected: BTreeSet<_> = local
        .iter()
        .copied()
        .filter(|id| *id != revision.root() && displayable(model, *id))
        .collect();
    let suggested_focus = definition
        .focus
        .or_else(|| architecture_root(model, &selected, &edges));
    if definition.kind == ViewKind::Architecture {
        let context_allowed = selected.clone();
        selected.retain(|id| {
            is(model, *id, sc::PART_DEFINITION)
                || is(model, *id, sc::PART_USAGE)
                || (definition.focus.is_some()
                    && (is(model, *id, sc::PORT_USAGE) || is(model, *id, c::CONNECTOR)))
        });
        if let Some(focus) = suggested_focus {
            selected.insert(focus);
            // An architecture level crosses two real relations: owns usage,
            // usage typed by definition. They stay separate visible edges.
            selected = architecture_neighborhood(
                focus,
                &selected,
                &edges,
                definition.depth.saturating_mul(2).min(8),
            );
            if definition.focus.is_some() && is(model, focus, c::TYPE) {
                profile.size(
                    "focused_context_reused",
                    usize::from(query_session.is_some()),
                );
                if query_session.is_none() {
                    query_session = Some(profile.measure(
                        "focused_kerml_context",
                        Some("selection_scope"),
                        || {
                            revision
                                .kerml_queries()
                                .map_err(|e| ViewError::Query(format!("{e:?}")))
                        },
                    )?);
                    context_constructions += 1;
                }
                let queries = query_session.as_ref().expect("constructed focused context");
                selected.extend(profile.measure(
                    "focused_interface_queries",
                    Some("selection_scope"),
                    || focused_interfaces(queries, focus, &context_allowed, &mut warnings),
                ));
            }
        }
    }
    edges.retain(|edge| definition.relationship_families.contains(&edge.family));
    let standard_endpoints: BTreeSet<_> = edges
        .iter()
        .flat_map(|e| [e.source, e.target])
        .filter(|id| standard.element(*id).is_some())
        .collect();
    if definition.include_standard_library {
        selected.extend(standard_endpoints.iter().copied());
    }
    if definition.kind == ViewKind::Requirements {
        let seeds: BTreeSet<_> = selected
            .iter()
            .copied()
            .filter(|id| {
                is(model, *id, sc::REQUIREMENT_DEFINITION)
                    || is(model, *id, sc::REQUIREMENT_USAGE)
                    || is(model, *id, sc::VERIFICATION_CASE_DEFINITION)
                    || is(model, *id, sc::VERIFICATION_CASE_USAGE)
            })
            .collect();
        selected = requirement_neighborhood(model, &seeds, &selected, &edges);
    }
    if let Some(focus) = definition
        .focus
        .filter(|_| definition.kind != ViewKind::Architecture)
    {
        // An explicitly focused standard element is deliberate expansion.
        selected.insert(focus);
        selected = if dependency_scope(definition) {
            dependency_neighborhood(
                model,
                &BTreeSet::from([focus]),
                &selected,
                &edges,
                definition.depth.min(8),
            )
        } else {
            neighborhood(
                &BTreeSet::from([focus]),
                &selected,
                &edges,
                definition.depth.min(8),
            )
        };
    }
    for hidden in &definition.hidden_elements {
        selected.remove(hidden);
    }
    edges.retain(|edge| selected.contains(&edge.source) && selected.contains(&edge.target));
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    edges.dedup_by(|a, b| a.id == b.id);
    profile.end("selection_scope", None, selection_started);
    let nodes = profile.measure("node_mapping", None, || {
        selected
            .iter()
            .map(|id| node(revision, *id))
            .collect::<Result<Vec<_>, _>>()
    })?;
    let groups_started = profile.start();
    let mut groups = BTreeMap::<ElementId, Vec<ElementId>>::new();
    for item in &nodes {
        if let Some(owner) = item.owner.filter(|id| selected.contains(id)) {
            groups.entry(owner).or_default().push(item.id);
        }
    }
    profile.end("group_mapping", None, groups_started);
    profile.size("kerml_context_constructions", context_constructions);
    Ok(ViewProjection {
        revision_id: revision.revision(),
        view: definition.clone(),
        nodes,
        edges,
        groups: groups
            .into_iter()
            .map(|(element_id, children)| ViewGroup {
                element_id,
                children,
            })
            .collect(),
        metadata: ViewMetadata {
            suggested_focus,
            scope: if dependency_scope(definition) {
                "Dependency neighborhood; reached package owners are context anchors, not sibling expansion"
            } else {
                "Current canonical graph; authored context, original identities"
            }.into(),
            producer_completeness: revision
                .producer_status()
                .map_or("Unavailable".into(), |s| format!("{:?}", s.completeness)),
            local_element_count: local.len(),
            omitted_standard_endpoints: if definition.include_standard_library {
                0
            } else {
                standard_endpoints.len()
            },
            warnings,
        },
    })
}

/// Opt-in application profiling only: no clocks or output when disabled, and no
/// timing fields are attached to canonical/query/presentation result DTOs.
pub(crate) struct ViewProfile(Option<ViewProfileState>);

struct ViewProfileState {
    identity: serde_json::Value,
    started: std::time::Instant,
    phases: Vec<serde_json::Value>,
    sizes: BTreeMap<&'static str, usize>,
}

impl ViewProfile {
    pub(crate) const fn disabled() -> Self {
        Self(None)
    }

    pub(crate) fn new(
        operation: &'static str,
        revision: &ProjectRevision,
        focus: Option<ElementId>,
        definition: Option<&ViewDefinition>,
    ) -> Self {
        if !std::env::var_os("AGENTIQUE_VIEW_PROFILE").is_some_and(|value| value == "1") {
            return Self::disabled();
        }
        let identity = serde_json::json!({
            "format": "agentique-modeling-view-profile/1",
            "operation": operation,
            "project": revision.project(),
            "revision": revision.revision(),
            "focus": focus,
            "kind": definition.map_or_else(|| "Inspector".into(), |view| format!("{:?}", view.kind)),
            "scope": definition.map_or_else(|| "SelectedElement".into(), |view| format!("{:?}", view.graph_scope)),
            "view": definition,
            "phase_contract": "Elapsed wall time. included_in names an inclusive parent phase; do not sum nested phases. total_ms is measured independently and excludes JSON emission. A missing phase was skipped or did not complete.",
        });
        Self(Some(ViewProfileState {
            identity,
            started: std::time::Instant::now(),
            phases: vec![],
            sizes: BTreeMap::new(),
        }))
    }

    pub(crate) fn start(&self) -> Option<std::time::Instant> {
        self.0.as_ref().map(|_| std::time::Instant::now())
    }

    pub(crate) fn end(
        &mut self,
        name: &'static str,
        included_in: Option<&'static str>,
        started: Option<std::time::Instant>,
    ) {
        if let (Some(profile), Some(started)) = (&mut self.0, started) {
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            profile.phases.push(serde_json::json!({
                "name": name, "included_in": included_in, "elapsed_ms": elapsed_ms,
            }));
        }
    }

    pub(crate) fn measure<T>(
        &mut self,
        name: &'static str,
        included_in: Option<&'static str>,
        work: impl FnOnce() -> T,
    ) -> T {
        let started = self.start();
        let result = work();
        self.end(name, included_in, started);
        result
    }

    pub(crate) fn size(&mut self, name: &'static str, size: usize) {
        if let Some(profile) = &mut self.0 {
            profile.sizes.insert(name, size);
        }
    }

    pub(crate) fn finish(self, success: bool) {
        use std::io::Write;
        if let Some(profile) = self.0 {
            let total_ms = profile.started.elapsed().as_secs_f64() * 1000.0;
            let mut record = profile.identity;
            record["outcome"] = if success { "ok" } else { "error" }.into();
            record["total_ms"] = total_ms.into();
            record["phases"] = profile.phases.into();
            record["sizes"] = serde_json::json!(profile.sizes);
            // A closed/full diagnostic sink cannot turn a valid query into an
            // application error. One lock keeps concurrent read records intact.
            let _ = writeln!(std::io::stderr().lock(), "{record}");
        }
    }
}

fn architecture_root(
    model: &ModelView,
    candidates: &BTreeSet<ElementId>,
    edges: &[ViewEdge],
) -> Option<ElementId> {
    let definitions: BTreeSet<_> = candidates
        .iter()
        .copied()
        .filter(|id| is(model, *id, sc::PART_DEFINITION))
        .collect();
    let typed: BTreeSet<_> = edges
        .iter()
        .filter(|edge| edge.family == RelationshipFamily::Typing)
        .map(|edge| edge.target)
        .collect();
    let mut ranked: Vec<_> = definitions
        .iter()
        .map(|id| {
            let reachable = architecture_neighborhood(*id, candidates, edges, 8).len();
            (
                std::cmp::Reverse(!typed.contains(id)),
                std::cmp::Reverse(reachable),
                *id,
            )
        })
        .collect();
    ranked.sort();
    ranked.first().map(|(_, _, id)| *id)
}

fn architecture_neighborhood(
    focus: ElementId,
    allowed: &BTreeSet<ElementId>,
    edges: &[ViewEdge],
    hops: u8,
) -> BTreeSet<ElementId> {
    let mut selected = BTreeSet::from([focus]);
    for _ in 0..hops {
        let additions: BTreeSet<_> = edges
            .iter()
            .filter(|edge| {
                selected.contains(&edge.source)
                    && matches!(
                        edge.family,
                        RelationshipFamily::Ownership | RelationshipFamily::Typing
                    )
            })
            .map(|edge| edge.target)
            .filter(|id| allowed.contains(id))
            .collect();
        let prior = selected.len();
        selected.extend(additions);
        if selected.len() == prior {
            break;
        }
    }
    selected
}

fn neighborhood(
    seeds: &BTreeSet<ElementId>,
    allowed: &BTreeSet<ElementId>,
    edges: &[ViewEdge],
    depth: u8,
) -> BTreeSet<ElementId> {
    let mut selected = seeds.clone();
    for _ in 0..depth {
        let frontier: BTreeSet<_> = edges
            .iter()
            .filter(|e| selected.contains(&e.source) || selected.contains(&e.target))
            .flat_map(|e| [e.source, e.target])
            .filter(|id| allowed.contains(id))
            .collect();
        let prior = selected.len();
        selected.extend(frontier);
        if selected.len() == prior {
            break;
        }
    }
    selected
}

fn dependency_scope(definition: &ViewDefinition) -> bool {
    definition.kind == ViewKind::SemanticGraph
        && definition.focus.is_some()
        && definition.graph_scope == GraphScope::DependencyNeighborhood
}

/// Family/standard filtering supplies the same edges and eligible identities as
/// ordinary Graph. Only outward Ownership from a reached Package is terminal;
/// Type/Feature ownership, incoming owners and other relationships still expand.
fn dependency_neighborhood(
    model: &ModelView,
    seeds: &BTreeSet<ElementId>,
    allowed: &BTreeSet<ElementId>,
    edges: &[ViewEdge],
    depth: u8,
) -> BTreeSet<ElementId> {
    let mut selected = seeds.clone();
    for _ in 0..depth {
        let mut additions = BTreeSet::new();
        for edge in edges {
            let terminal_package_owner = edge.family == RelationshipFamily::Ownership
                && !seeds.contains(&edge.source)
                && is(model, edge.source, c::PACKAGE);
            if selected.contains(&edge.source)
                && !terminal_package_owner
                && allowed.contains(&edge.target)
            {
                additions.insert(edge.target);
            }
            if selected.contains(&edge.target) && allowed.contains(&edge.source) {
                additions.insert(edge.source);
            }
        }
        let prior = selected.len();
        selected.extend(additions);
        if selected.len() == prior {
            break;
        }
    }
    selected
}

fn query_warning<T>(warnings: &mut Vec<String>, label: &str, answer: &QueryResult<T>) {
    if answer.completeness != Completeness::Complete {
        warnings.push(format!("{label} are {:?}", answer.completeness));
    }
}

/// Explicit focus can reveal inherited connection surfaces, without reparenting
/// their canonical records or synthesizing a second copy under the focused type.
fn focused_interfaces(
    queries: &KerMlQueries<'_>,
    focus: ElementId,
    allowed: &BTreeSet<ElementId>,
    warnings: &mut Vec<String>,
) -> BTreeSet<ElementId> {
    let model = queries.model();
    let effective = queries.effective_features(focus);
    query_warning(warnings, "Focused effective features", &effective);
    let mut selected = BTreeSet::new();
    for feature in effective.value.iter().copied().filter(|id| {
        allowed.contains(id)
            && (is(model, *id, sc::PORT_USAGE) || is(model, *id, sc::INTERFACE_USAGE))
    }) {
        selected.insert(feature);
        let owner = queries.owner(feature);
        query_warning(warnings, "Connection surface owners", &owner);
        selected.extend(owner.value.filter(|owner| allowed.contains(owner)));
        let types = queries.feature_types(feature);
        query_warning(warnings, "Connection surface types", &types);
        // A port definition remains an Inspector type reference. Treating it as
        // another boundary port would give it an invented presentation owner.
        selected.extend(
            types
                .value
                .iter()
                .filter(|id| allowed.contains(id) && !is(model, **id, sc::PORT_DEFINITION))
                .copied(),
        );
    }
    selected
}

/// Keep the literal subject feature and its architecture type as two identities
/// connected by their two original relationships. No requirement-to-type shortcut.
fn requirement_neighborhood(
    model: &ModelView,
    seeds: &BTreeSet<ElementId>,
    allowed: &BTreeSet<ElementId>,
    edges: &[ViewEdge],
) -> BTreeSet<ElementId> {
    let mut selected = neighborhood(seeds, allowed, edges, 1);
    let subjects: BTreeSet<_> = edges
        .iter()
        .filter(|edge| {
            seeds.contains(&edge.source)
                && edge.family == RelationshipFamily::Requirement
                && edge
                    .relationship_id
                    .is_some_and(|id| is(model, id, sc::SUBJECT_MEMBERSHIP))
        })
        .map(|edge| edge.target)
        .filter(|id| selected.contains(id))
        .collect();
    selected.extend(
        edges
            .iter()
            .filter(|edge| {
                subjects.contains(&edge.source)
                    && edge.family == RelationshipFamily::Typing
                    && allowed.contains(&edge.target)
            })
            .map(|edge| edge.target),
    );
    selected
}

/// Both scene projection and inspection use this exact semantic endpoint query.
/// The observer retains completeness and evidence summaries even for empty results.
pub(crate) fn connector_edges(
    queries: &KerMlQueries<'_>,
    revision: ProjectRevisionId,
    local: &BTreeSet<ElementId>,
    mut observe: impl FnMut(ElementId, &QueryResult<Vec<ElementId>>),
) -> Vec<ViewEdge> {
    let model = queries.model();
    let mut edges = vec![];
    for id in local
        .iter()
        .copied()
        .filter(|id| is(model, *id, c::CONNECTOR))
    {
        let answer = queries.connector_endpoints(id);
        observe(id, &answer);
        // A connector is a semantic hyperedge. Preserve its canonical endpoint
        // order and relationship identity for every spoke, without flow claims.
        if let Some(source) = answer.value.first() {
            let record = model.element(id).expect("queried canonical connector");
            for (order, target) in answer.value.iter().enumerate().skip(1) {
                edges.push(edge(
                    model,
                    revision,
                    record,
                    RelationshipFamily::Connection,
                    *source,
                    *target,
                    order,
                    name(model, id),
                ));
            }
        }
    }
    edges
}

pub(crate) fn is(model: &ModelView, id: ElementId, class: MetaclassId) -> bool {
    model.element(id).is_some_and(|r| {
        model
            .registry()
            .is_subtype(r.metaclass(), class)
            .unwrap_or(false)
    })
}
pub(crate) fn refs(model: &ModelView, id: ElementId, property: PropertyId) -> Vec<ElementId> {
    effective_slot(model, id, property)
        .into_iter()
        .flat_map(|slot| slot.value().values())
        .filter_map(|value| {
            if let Value::Reference(id) = value {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

fn effective_slot(
    model: &ModelView,
    id: ElementId,
    property: PropertyId,
) -> Option<&agq_kernel::Slot> {
    let record = model.element(id)?;
    // Kernel navigation reads exact storage identities. Inherited/redefined
    // metamodel properties must resolve before looking up a canonical carrier.
    let property = model
        .registry()
        .resolve_property(record.metaclass(), property)
        .ok()??;
    model.navigation_slot(id, property.id)
}
pub(crate) fn owner(model: &ModelView, id: ElementId) -> Option<ElementId> {
    let relationship = refs(model, id, p::ELEMENT_OWNING_RELATIONSHIP)
        .first()
        .copied()?;
    refs(model, relationship, p::RELATIONSHIP_OWNING_RELATED_ELEMENT)
        .first()
        .copied()
}
pub(crate) fn name(model: &ModelView, id: ElementId) -> String {
    for property in [
        p::ELEMENT_DECLARED_NAME,
        p::ELEMENT_NAME,
        p::ELEMENT_DECLARED_SHORT_NAME,
    ] {
        if let Some(value) =
            effective_slot(model, id, property).and_then(|s| s.value().values().next())
            && let Value::String(value) = value
            && !value.is_empty()
        {
            return value.clone();
        }
    }
    kind(model, id)
}
pub(crate) fn kind(model: &ModelView, id: ElementId) -> String {
    model
        .element(id)
        .and_then(|r| model.registry().class(r.metaclass()).ok())
        .map_or_else(|| "Missing element".into(), |c| c.name.clone())
}
fn displayable(model: &ModelView, id: ElementId) -> bool {
    if is(model, id, c::RELATIONSHIP) && !is(model, id, c::FEATURE) {
        return false;
    }
    has_declared_name(model, id)
}
pub(crate) fn has_declared_name(model: &ModelView, id: ElementId) -> bool {
    [p::ELEMENT_DECLARED_NAME, p::ELEMENT_DECLARED_SHORT_NAME]
        .iter()
        .any(|property| {
            effective_slot(model, id, *property).is_some_and(|s| {
                s.value()
                    .values()
                    .any(|v| matches!(v,Value::String(s) if !s.is_empty()))
            })
        })
}
pub(crate) fn provenance(origin: &Origin) -> ViewOrigin {
    match origin {
        Origin::Derived(_) => ViewOrigin::Derived,
        Origin::Declared(DeclaredOrigin::Authored { .. }) => ViewOrigin::Authored,
        Origin::Declared(
            DeclaredOrigin::StandardLibrary { .. } | DeclaredOrigin::ReviewedCorrection { .. },
        ) => ViewOrigin::Standard,
        _ => ViewOrigin::Generated,
    }
}
pub(crate) fn summary(model: &ModelView, id: ElementId) -> FeatureSummary {
    FeatureSummary {
        id,
        name: name(model, id),
        semantic_kind: kind(model, id),
    }
}
pub(crate) fn node(revision: &ProjectRevision, id: ElementId) -> Result<ViewNode, ViewError> {
    let model = revision
        .semantic_model()
        .ok_or(ViewError::Unavailable(revision.revision()))?;
    let record = model.element(id).ok_or(ViewError::MissingElement(id))?;
    let (counts, features) = owned_feature_summaries(model, id);
    let standard = revision
        .accepted_sysml()
        .overlay()
        .model()
        .element(id)
        .is_some();
    let origin = if standard {
        ViewOrigin::Standard
    } else {
        provenance(record.origin())
    };
    let mut badges = vec![];
    if origin == ViewOrigin::Derived {
        badges.push("Derived".into());
    }
    if standard {
        badges.push("Standard library".into());
    }
    Ok(ViewNode {
        id,
        revision_id: revision.revision(),
        semantic_kind: kind(model, id),
        name: name(model, id),
        qualified_name: qualified_name(model, id, revision.root()),
        owner: owner(model, id),
        origin,
        source_available: revision.source_for_fact(FactKey::Element(id)).is_some(),
        features,
        counts,
        badges,
    })
}

fn owned_feature_summaries(
    model: &ModelView,
    id: ElementId,
) -> (FeatureCounts, Vec<FeatureSummary>) {
    let mut counts = FeatureCounts::default();
    let mut features = vec![];
    // Navigate canonical ownership; no inherited feature is cloned into this list.
    for relationship in refs(model, id, p::ELEMENT_OWNED_RELATIONSHIP) {
        for child in refs(model, relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT) {
            if is(model, child, c::FEATURE) {
                // ConnectionUsage (including InterfaceUsage) is also a PartUsage
                // in the metamodel, but is a separate engineering object in the
                // Studio. Keep its feature/ownership; exclude it from part totals.
                if is(model, child, sc::PART_USAGE) && !is(model, child, c::CONNECTOR) {
                    counts.parts += 1;
                }
                if is(model, child, sc::PORT_USAGE) {
                    counts.ports += 1;
                }
                if is(model, child, sc::REQUIREMENT_USAGE) {
                    counts.requirements += 1;
                }
                if displayable(model, child) {
                    features.push(summary(model, child));
                }
            }
        }
    }
    (counts, features)
}
pub(crate) fn qualified_name(
    model: &ModelView,
    mut id: ElementId,
    root: ElementId,
) -> Option<String> {
    let mut visited = BTreeSet::new();
    let mut names = vec![];
    while id != root && visited.insert(id) {
        if displayable(model, id) {
            names.push(name(model, id));
        }
        let Some(next) = owner(model, id) else { break };
        id = next;
    }
    names.reverse();
    (!names.is_empty()).then(|| names.join("::"))
}

pub(crate) fn graph_edges(
    model: &ModelView,
    revision: ProjectRevisionId,
    local: &BTreeSet<ElementId>,
) -> Vec<ViewEdge> {
    let mut edges = vec![];
    for id in local {
        let Some(record) = model.element(*id) else {
            continue;
        };
        if !is(model, *id, c::RELATIONSHIP) {
            continue;
        }
        let (family, source_property, target_property, label) = if is(model, *id, c::FEATURE_TYPING)
        {
            (
                RelationshipFamily::Typing,
                p::FEATURE_TYPING_TYPED_FEATURE,
                p::FEATURE_TYPING_TYPE,
                "typed by",
            )
        } else if is(model, *id, c::REDEFINITION) {
            (
                RelationshipFamily::Redefinition,
                p::REDEFINITION_REDEFINING_FEATURE,
                p::REDEFINITION_REDEFINED_FEATURE,
                "redefines",
            )
        } else if is(model, *id, c::SUBSETTING) {
            (
                RelationshipFamily::Subsetting,
                p::SUBSETTING_SUBSETTING_FEATURE,
                p::SUBSETTING_SUBSETTED_FEATURE,
                "subsets",
            )
        } else if is(model, *id, c::SPECIALIZATION) {
            (
                RelationshipFamily::Specialization,
                p::SPECIALIZATION_SPECIFIC,
                p::SPECIALIZATION_GENERAL,
                "specializes",
            )
        } else if is(model, *id, sc::SUBJECT_MEMBERSHIP)
            && refs(model, *id, p::RELATIONSHIP_OWNING_RELATED_ELEMENT)
                .iter()
                .any(|owner| {
                    is(model, *owner, sc::REQUIREMENT_DEFINITION)
                        || is(model, *owner, sc::REQUIREMENT_USAGE)
                })
        {
            (
                RelationshipFamily::Requirement,
                p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                "subject",
            )
        } else if is(model, *id, sc::REQUIREMENT_VERIFICATION_MEMBERSHIP) {
            (
                RelationshipFamily::Verification,
                p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                "verification requirement",
            )
        } else if is(model, *id, c::OWNING_MEMBERSHIP) {
            (
                RelationshipFamily::Ownership,
                p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                "owns",
            )
        } else if is(model, *id, c::MEMBERSHIP) {
            (
                RelationshipFamily::Reference,
                p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                p::MEMBERSHIP_MEMBER_ELEMENT,
                "references",
            )
        } else {
            continue;
        };
        let sources = refs(model, *id, source_property);
        let targets = refs(model, *id, target_property);
        for source in &sources {
            for (order, target) in targets.iter().enumerate() {
                edges.push(edge(
                    model,
                    revision,
                    record,
                    family,
                    *source,
                    *target,
                    order,
                    label.into(),
                ));
            }
        }
    }
    edges
}

#[allow(clippy::too_many_arguments)]
fn edge(
    model: &ModelView,
    revision_id: ProjectRevisionId,
    relationship: &ElementRecord,
    family: RelationshipFamily,
    source: ElementId,
    target: ElementId,
    order: usize,
    label: String,
) -> ViewEdge {
    ViewEdge {
        id: format!("{}:{source}:{target}:{order}", relationship.id()),
        relationship_id: Some(relationship.id()),
        revision_id,
        family,
        semantic_kind: kind(model, relationship.id()),
        source,
        target,
        origin: provenance(relationship.origin()),
        rule_id: match relationship.origin() {
            Origin::Derived(e) => Some(e.rule),
            _ => None,
        },
        label,
        directed: family != RelationshipFamily::Connection,
        order,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use agq_kerml_semantics::{SemanticContext, SemanticOptions};
    use agq_kernel::{Snapshot, metamodel::ValueKind, value::SlotValue};
    use std::sync::Arc;

    /// Small canonical records exercise the public query contracts; no accepted
    /// runtime, generated semantic answers or frontend fixture edges are used.
    pub(crate) fn semantic_fixture(
        records: &[(u128, MetaclassId)],
        slots: &[(u128, PropertyId, Vec<Value>)],
    ) -> Snapshot {
        let registry = Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        );
        let base = Snapshot::new(registry.clone());
        let mut changes = base.change_set();
        let origin = || DeclaredOrigin::Authored { source: None };
        let records: BTreeMap<_, _> = records.iter().copied().collect();
        for (&element, &class) in &records {
            let element = ElementId::from_u128(element);
            changes.create(element, class, origin());
            for property in registry
                .effective_properties(class)
                .unwrap()
                .filter(|p| !p.derived && p.multiplicity.lower > 0)
            {
                let value = match registry.storage_kind(property.value_kind).unwrap() {
                    ValueKind::Boolean => Value::Boolean(false),
                    ValueKind::String => Value::String(element.to_string()),
                    ValueKind::Enumeration(domain) => {
                        let literals = &registry.enumeration(domain).unwrap().literals;
                        let literal = literals
                            .iter()
                            .find(|(_, name)| name.as_str() == "public")
                            .or_else(|| literals.iter().next())
                            .unwrap()
                            .0;
                        Value::Enumeration(*literal)
                    }
                    ValueKind::Reference(_) => continue,
                    other => panic!("unexpected fixture property: {other:?}"),
                };
                changes.set(element, property.id, SlotValue::Scalar(value), origin());
            }
        }
        for (element, property, values) in slots {
            let property = registry
                .resolve_property(records[element], *property)
                .unwrap()
                .unwrap();
            let value = if property.multiplicity.upper.is_some_and(|upper| upper <= 1) {
                SlotValue::Scalar(values[0].clone())
            } else if property.ordered {
                SlotValue::Ordered(values.clone())
            } else if property.unique {
                SlotValue::Set(values.iter().cloned().collect())
            } else {
                SlotValue::Bag(values.clone())
            };
            changes.set(ElementId::from_u128(*element), property.id, value, origin());
        }
        base.apply(&changes).unwrap()
    }

    pub(crate) fn fixture_queries(snapshot: &Snapshot) -> KerMlQueries<'_> {
        KerMlQueries::new(
            SemanticContext::for_snapshot(
                snapshot,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .unwrap(),
        )
    }

    pub(crate) fn references(ids: &[u128]) -> Vec<Value> {
        ids.iter()
            .map(|id| Value::Reference(ElementId::from_u128(*id)))
            .collect()
    }

    #[test]
    fn requirement_subject_reaches_architecture_through_two_canonical_relationships_only() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::REQUIREMENT_DEFINITION),
                (2, sc::REFERENCE_USAGE),
                (3, sc::PART_DEFINITION),
                (4, sc::PART_USAGE),
                (5, sc::PART_DEFINITION),
                (11, sc::SUBJECT_MEMBERSHIP),
                (12, c::FEATURE_TYPING),
                (13, c::FEATURE_MEMBERSHIP),
                (14, c::FEATURE_TYPING),
            ],
            &[
                (1, p::ELEMENT_OWNED_RELATIONSHIP, references(&[11])),
                (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[2])),
                (2, p::ELEMENT_OWNED_RELATIONSHIP, references(&[12])),
                (12, p::SPECIALIZATION_SPECIFIC, references(&[2])),
                (12, p::SPECIALIZATION_GENERAL, references(&[3])),
                (3, p::ELEMENT_OWNED_RELATIONSHIP, references(&[13])),
                (13, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[4])),
                (4, p::ELEMENT_OWNED_RELATIONSHIP, references(&[14])),
                (14, p::SPECIALIZATION_SPECIFIC, references(&[4])),
                (14, p::SPECIALIZATION_GENERAL, references(&[5])),
            ],
        );
        let id = ElementId::from_u128;
        let model = snapshot.model();
        let local = model.elements().map(ElementRecord::id).collect();
        let edges = graph_edges(model, ProjectRevisionId::from_u128(90), &local);
        let selected = requirement_neighborhood(model, &BTreeSet::from([id(1)]), &local, &edges);
        assert_eq!(selected, BTreeSet::from([id(1), id(2), id(3)]));
        let subject = edges
            .iter()
            .find(|edge| edge.relationship_id == Some(id(11)))
            .unwrap();
        assert_eq!(
            (subject.source, subject.target, subject.family),
            (id(1), id(2), RelationshipFamily::Requirement)
        );
        assert_eq!(subject.semantic_kind, "SubjectMembership");
        assert_eq!(subject.origin, ViewOrigin::Authored);
        let typing = edges
            .iter()
            .find(|edge| edge.relationship_id == Some(id(12)))
            .unwrap();
        assert_eq!((typing.source, typing.target), (id(2), id(3)));
        assert_eq!(typing.semantic_kind, "FeatureTyping");
        assert!(
            !edges
                .iter()
                .any(|edge| edge.source == id(1) && edge.target == id(3))
        );
        // Filtering Typing removes the second hop, not the literal subject.
        let filtered: Vec<_> = edges
            .into_iter()
            .filter(|edge| edge.family != RelationshipFamily::Typing)
            .collect();
        assert_eq!(
            requirement_neighborhood(model, &BTreeSet::from([id(1)]), &local, &filtered),
            BTreeSet::from([id(1), id(2)])
        );
    }

    #[test]
    fn verification_membership_is_a_modeled_link_and_case_subject_is_not_a_requirement() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::VERIFICATION_CASE_DEFINITION),
                (2, sc::REQUIREMENT_USAGE),
                (3, sc::REFERENCE_USAGE),
                (11, sc::REQUIREMENT_VERIFICATION_MEMBERSHIP),
                (12, sc::SUBJECT_MEMBERSHIP),
            ],
            &[
                (1, p::ELEMENT_OWNED_RELATIONSHIP, references(&[11, 12])),
                (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[2])),
                (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[3])),
            ],
        );
        let model = snapshot.model();
        let edges = graph_edges(
            model,
            ProjectRevisionId::from_u128(91),
            &model.elements().map(ElementRecord::id).collect(),
        );
        let verification = edges
            .iter()
            .find(|edge| edge.relationship_id == Some(ElementId::from_u128(11)))
            .unwrap();
        assert_eq!(verification.family, RelationshipFamily::Verification);
        assert_eq!(
            verification.semantic_kind,
            "RequirementVerificationMembership"
        );
        assert_eq!(verification.label, "verification requirement");
        assert_eq!(
            (verification.source, verification.target),
            (ElementId::from_u128(1), ElementId::from_u128(2))
        );
        assert!(edges.iter().any(
            |edge| edge.relationship_id == Some(ElementId::from_u128(12))
                && edge.family == RelationshipFamily::Ownership
        ));
    }

    #[test]
    fn focused_inherited_port_keeps_owner_membership_and_original_identity() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::PART_DEFINITION),
                (2, sc::PART_DEFINITION),
                (3, sc::PORT_USAGE),
                (4, sc::PORT_DEFINITION),
                (11, c::SUBCLASSIFICATION),
                (12, c::FEATURE_MEMBERSHIP),
                (13, c::FEATURE_TYPING),
            ],
            &[
                (1, p::ELEMENT_OWNED_RELATIONSHIP, references(&[11])),
                (11, p::SPECIALIZATION_SPECIFIC, references(&[1])),
                (11, p::SPECIALIZATION_GENERAL, references(&[2])),
                (2, p::ELEMENT_OWNED_RELATIONSHIP, references(&[12])),
                (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[3])),
                (3, p::ELEMENT_OWNED_RELATIONSHIP, references(&[13])),
                (13, p::SPECIALIZATION_SPECIFIC, references(&[3])),
                (13, p::SPECIALIZATION_GENERAL, references(&[4])),
            ],
        );
        let id = ElementId::from_u128;
        let q = fixture_queries(&snapshot);
        let allowed = BTreeSet::from([id(1), id(2), id(3), id(4)]);
        let mut warnings = vec![];
        assert!(q.effective_features(id(1)).value.contains(&id(3)));
        let selected = focused_interfaces(&q, id(1), &allowed, &mut warnings);
        assert_eq!(selected, BTreeSet::from([id(2), id(3)]));
        assert_eq!(q.owner(id(3)).value, Some(id(2)));
        assert_eq!(owner(snapshot.model(), id(3)), Some(id(2)));
        let edges = graph_edges(
            snapshot.model(),
            ProjectRevisionId::from_u128(92),
            &snapshot.model().elements().map(ElementRecord::id).collect(),
        );
        assert!(edges.iter().any(|edge| edge.relationship_id == Some(id(12))
            && edge.source == id(2)
            && edge.target == id(3)));
        assert!(
            !edges
                .iter()
                .any(|edge| edge.family == RelationshipFamily::Ownership
                    && edge.source == id(1)
                    && edge.target == id(3))
        );
        assert!(edges.iter().any(|edge| edge.relationship_id == Some(id(11))
            && edge.family == RelationshipFamily::Specialization));
        let effective = q.effective_features(id(1));
        assert_eq!(
            warnings
                .iter()
                .any(|warning| warning.starts_with("Focused effective features")),
            effective.completeness != Completeness::Complete
        );
    }
    fn e(source: u128, target: u128) -> ViewEdge {
        ViewEdge {
            id: format!("{source}-{target}"),
            relationship_id: Some(ElementId::from_u128(100 + source)),
            revision_id: ProjectRevisionId::from_u128(1),
            family: RelationshipFamily::Ownership,
            semantic_kind: "OwningMembership".into(),
            source: ElementId::from_u128(source),
            target: ElementId::from_u128(target),
            origin: ViewOrigin::Authored,
            rule_id: None,
            label: "owns".into(),
            directed: true,
            order: 0,
        }
    }
    #[test]
    fn neighborhood_is_bounded_stable_and_handles_cycles() {
        let ids: BTreeSet<_> = (1..=5).map(ElementId::from_u128).collect();
        let seed = BTreeSet::from([ElementId::from_u128(1)]);
        let edges = vec![e(1, 2), e(2, 3), e(3, 1), e(3, 4), e(4, 5), e(5, 99)];
        assert_eq!(neighborhood(&seed, &ids, &edges, 1).len(), 3);
        assert_eq!(neighborhood(&seed, &ids, &edges, 8), ids);
        assert_eq!(neighborhood(&seed, &ids, &edges, 0), seed);
    }
    #[test]
    fn architecture_expansion_preserves_usage_and_type_as_distinct_nodes() {
        let ids: BTreeSet<_> = (1..=5).map(ElementId::from_u128).collect();
        let mut typing = e(2, 3);
        typing.family = RelationshipFamily::Typing;
        typing.label = "typed by".into();
        let edges = vec![e(1, 2), typing, e(3, 4), e(5, 3)];
        let selected = architecture_neighborhood(ElementId::from_u128(1), &ids, &edges, 2);
        assert_eq!(selected, (1..=3).map(ElementId::from_u128).collect());
        assert_eq!(edges[0].target, ElementId::from_u128(2));
        assert_eq!(edges[1].family, RelationshipFamily::Typing);
        assert!(!selected.contains(&ElementId::from_u128(5)));
    }

    fn dependency_fixture() -> Snapshot {
        semantic_fixture(
            &[
                (1, c::PACKAGE),
                (2, sc::PART_DEFINITION),
                (3, sc::PART_DEFINITION),
                (4, sc::PORT_USAGE),
                (5, sc::PART_DEFINITION),
                (6, sc::PART_DEFINITION),
                (11, c::OWNING_MEMBERSHIP),
                (12, c::OWNING_MEMBERSHIP),
                (13, c::FEATURE_MEMBERSHIP),
                (14, c::MEMBERSHIP),
                (15, c::MEMBERSHIP),
                (16, c::MEMBERSHIP),
                (17, c::OWNING_MEMBERSHIP),
            ],
            &[
                (
                    1,
                    p::ELEMENT_OWNED_RELATIONSHIP,
                    references(&[11, 12, 16, 17]),
                ),
                (2, p::ELEMENT_OWNED_RELATIONSHIP, references(&[13, 14])),
                (5, p::ELEMENT_OWNED_RELATIONSHIP, references(&[15])),
                (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[2])),
                (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[3])),
                (13, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[4])),
                (17, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[5])),
                // A direct dependency on sibling 5, with a reference cycle.
                (14, p::MEMBERSHIP_MEMBER_ELEMENT, references(&[5])),
                (15, p::MEMBERSHIP_MEMBER_ELEMENT, references(&[2])),
                // Reached package 1 has a genuine non-Ownership dependency.
                (16, p::MEMBERSHIP_MEMBER_ELEMENT, references(&[6])),
            ],
        )
    }

    #[test]
    fn dependency_scope_stops_package_siblings_but_keeps_features_and_direct_dependencies() {
        let snapshot = dependency_fixture();
        let model = snapshot.model();
        let id = ElementId::from_u128;
        let local = model.elements().map(ElementRecord::id).collect();
        let edges = graph_edges(model, ProjectRevisionId::from_u128(97), &local);
        let original_edges = edges.clone();
        let allowed = (1..=6).map(id).collect();
        let seeds = BTreeSet::from([id(2)]);
        assert!(is(model, id(2), c::NAMESPACE));
        assert!(!is(model, id(2), c::PACKAGE));
        assert_eq!(
            dependency_neighborhood(model, &seeds, &allowed, &edges, 0),
            seeds
        );
        assert_eq!(
            dependency_neighborhood(model, &seeds, &allowed, &edges, 1),
            BTreeSet::from([id(1), id(2), id(4), id(5)])
        );
        let selected = dependency_neighborhood(model, &seeds, &allowed, &edges, 2);
        assert_eq!(
            selected,
            BTreeSet::from([id(1), id(2), id(4), id(5), id(6)])
        );
        assert_eq!(
            dependency_neighborhood(model, &seeds, &allowed, &edges, 8),
            selected
        );
        // Ordinary Graph deliberately retains its existing broad neighborhood.
        assert_eq!(neighborhood(&seeds, &allowed, &edges, 2), allowed);
        for relationship in [11, 13, 14, 15, 16, 17] {
            let original = edges
                .iter()
                .find(|edge| edge.relationship_id == Some(id(relationship)))
                .unwrap();
            assert!(selected.contains(&original.source) && selected.contains(&original.target));
        }
        assert_eq!(edges, original_edges);
        assert_eq!(owner(model, id(4)), Some(id(2)));
        assert_eq!(owner(model, id(5)), Some(id(1)));
    }

    #[test]
    fn explicit_package_seed_can_expand_members_without_changing_depth_contract() {
        let snapshot = dependency_fixture();
        let model = snapshot.model();
        let id = ElementId::from_u128;
        let local = model.elements().map(ElementRecord::id).collect();
        let edges = graph_edges(model, ProjectRevisionId::from_u128(98), &local);
        let allowed = (1..=6).map(id).collect();
        let seeds = BTreeSet::from([id(1)]);
        assert_eq!(
            dependency_neighborhood(model, &seeds, &allowed, &edges, 0),
            seeds
        );
        assert_eq!(
            dependency_neighborhood(model, &seeds, &allowed, &edges, 1),
            BTreeSet::from([id(1), id(2), id(3), id(5), id(6)])
        );
        assert_eq!(
            dependency_neighborhood(model, &seeds, &allowed, &edges, 2),
            allowed
        );
    }

    #[test]
    fn dependency_scope_respects_filtered_families_and_standard_eligibility() {
        let snapshot = dependency_fixture();
        let model = snapshot.model();
        let id = ElementId::from_u128;
        let local = model.elements().map(ElementRecord::id).collect();
        let edges = graph_edges(model, ProjectRevisionId::from_u128(99), &local);
        let seeds = BTreeSet::from([id(2)]);
        let all = (1..=6).map(id).collect();
        let ownership: Vec<_> = edges
            .iter()
            .filter(|edge| edge.family == RelationshipFamily::Ownership)
            .cloned()
            .collect();
        assert_eq!(
            dependency_neighborhood(model, &seeds, &all, &ownership, 8),
            BTreeSet::from([id(1), id(2), id(4)])
        );
        let references: Vec<_> = edges
            .iter()
            .filter(|edge| edge.family == RelationshipFamily::Reference)
            .cloned()
            .collect();
        assert_eq!(
            dependency_neighborhood(model, &seeds, &all, &references, 8),
            BTreeSet::from([id(2), id(5)])
        );
        assert_eq!(dependency_neighborhood(model, &seeds, &all, &[], 8), seeds);
        // `project` supplies the same exact-ID eligibility set after its standard
        // switch. No traversal through an excluded standard endpoint is allowed.
        let standard = semantic_fixture(&[(6, sc::PART_DEFINITION)], &[]);
        let authored: BTreeSet<_> = all
            .iter()
            .copied()
            .filter(|element| standard.model().element(*element).is_none())
            .collect();
        let selected = dependency_neighborhood(model, &seeds, &authored, &edges, 8);
        assert_eq!(selected, BTreeSet::from([id(1), id(2), id(4), id(5)]));
        let expanded = dependency_neighborhood(model, &seeds, &all, &edges, 8);
        assert_eq!(
            expanded.difference(&selected).copied().collect::<Vec<_>>(),
            vec![id(6)]
        );
        // An explicit standard focus remains selected, matching ordinary Graph.
        assert_eq!(
            dependency_neighborhood(model, &BTreeSet::from([id(6)]), &authored, &edges, 1),
            BTreeSet::from([id(1), id(6)])
        );
    }

    #[test]
    fn dependency_scope_round_trips_defaults_old_views_and_applies_only_to_focused_graph() {
        let mut view = ViewDefinition::semantic_graph();
        view.focus = Some(ElementId::from_u128(2));
        view.graph_scope = GraphScope::DependencyNeighborhood;
        view.relationship_families = vec![RelationshipFamily::Reference];
        view.include_standard_library = true;
        view.depth = 3;
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(
            serde_json::from_value::<ViewDefinition>(json.clone()).unwrap(),
            view
        );
        assert!(dependency_scope(&view));
        let mut legacy = json;
        legacy.as_object_mut().unwrap().remove("graph_scope");
        let old = serde_json::from_value::<ViewDefinition>(legacy).unwrap();
        assert_eq!(old.graph_scope, GraphScope::Neighborhood);
        assert!(!dependency_scope(&old));
        for kind in [ViewKind::Architecture, ViewKind::Requirements] {
            let mut other = view.clone();
            other.kind = kind;
            assert!(!dependency_scope(&other));
        }
        view.focus = None;
        assert!(!dependency_scope(&view));
    }
    #[test]
    fn definitions_are_selection_metadata_and_preserve_hidden_identity() {
        let mut definition = ViewDefinition::semantic_graph();
        definition.hidden_elements.push(ElementId::from_u128(9));
        let json = serde_json::to_string(&definition).unwrap();
        assert!(!json.contains("nodes"));
        assert_eq!(
            serde_json::from_str::<ViewDefinition>(&json).unwrap(),
            definition
        );
    }

    #[test]
    fn ownership_projection_reads_canonical_memberships_and_keeps_revision_identity() {
        use agq_kernel::{Snapshot, provenance::DeclaredOrigin, value::SlotValue};
        use std::sync::Arc;
        let root = ElementId::from_u128(1);
        let child = ElementId::from_u128(2);
        let membership = ElementId::from_u128(3);
        let authored = || DeclaredOrigin::Authored { source: None };
        let snapshot = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let mut changes = snapshot.change_set();
        changes.create(root, c::PACKAGE, authored());
        changes.create(child, c::CLASS, authored());
        changes.create(membership, c::OWNING_MEMBERSHIP, authored());
        changes.set(
            root,
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String("System".into())),
            authored(),
        );
        changes.set(
            child,
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String("Component".into())),
            authored(),
        );
        changes.set(
            root,
            p::ELEMENT_OWNED_RELATIONSHIP,
            SlotValue::Ordered(vec![Value::Reference(membership)]),
            authored(),
        );
        changes.set(
            membership,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(child)]),
            authored(),
        );
        let preview = snapshot.preview(&changes).unwrap();
        let revision = ProjectRevisionId::from_u128(40);
        let edges = graph_edges(
            preview.model(),
            revision,
            &BTreeSet::from([root, child, membership]),
        );
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relationship_id, Some(membership));
        assert_eq!((edges[0].source, edges[0].target), (root, child));
        assert_eq!(edges[0].revision_id, revision);
        assert_eq!(owner(preview.model(), child), Some(root));
        assert_eq!(
            qualified_name(preview.model(), child, ElementId::from_u128(99)),
            Some("System::Component".into())
        );
        // Projection and filtering never mutate canonical ownership.
        assert_eq!(
            refs(
                preview.model(),
                membership,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT
            ),
            vec![child]
        );
    }

    #[test]
    fn engineering_part_count_excludes_connectors_without_losing_owned_features() {
        let snapshot = semantic_fixture(
            &[
                (1, sc::PART_DEFINITION),
                (2, sc::PART_USAGE),
                (3, sc::INTERFACE_USAGE),
                (4, sc::CONNECTION_USAGE),
                (11, c::FEATURE_MEMBERSHIP),
                (12, c::FEATURE_MEMBERSHIP),
                (13, c::FEATURE_MEMBERSHIP),
            ],
            &[
                (1, p::ELEMENT_OWNED_RELATIONSHIP, references(&[11, 12, 13])),
                (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[2])),
                (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[3])),
                (13, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, references(&[4])),
                (
                    2,
                    p::ELEMENT_DECLARED_NAME,
                    vec![Value::String("Connector".into())],
                ),
                (
                    3,
                    p::ELEMENT_DECLARED_NAME,
                    vec![Value::String("queryConnection".into())],
                ),
                (
                    4,
                    p::ELEMENT_DECLARED_NAME,
                    vec![Value::String("module".into())],
                ),
            ],
        );
        let model = snapshot.model();
        let id = ElementId::from_u128;
        assert!(is(model, id(2), sc::PART_USAGE));
        assert!(!is(model, id(2), c::CONNECTOR));
        for child in [id(3), id(4)] {
            assert!(is(model, child, sc::PART_USAGE));
            assert!(is(model, child, c::CONNECTOR));
        }
        let (counts, features) = owned_feature_summaries(model, id(1));
        assert_eq!(
            counts,
            FeatureCounts {
                parts: 1,
                ..Default::default()
            }
        );
        assert_eq!(
            features
                .iter()
                .map(|feature| feature.id)
                .collect::<Vec<_>>(),
            vec![id(2), id(3), id(4)],
            "canonical feature identities and ownership order remain intact"
        );
        assert_eq!(features[0].name, "Connector");
        assert_eq!(features[1].semantic_kind, "InterfaceUsage");
        assert_eq!(features[2].semantic_kind, "ConnectionUsage");
        for child in [id(2), id(3), id(4)] {
            assert_eq!(owner(model, child), Some(id(1)));
        }
    }

    #[test]
    fn canonical_sysml_ownership_typing_and_feature_counts_resolve_effective_properties() {
        use agq_kernel::{
            Snapshot, metamodel::ValueKind, provenance::DeclaredOrigin, value::SlotValue,
        };
        use std::sync::Arc;
        let registry = Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        );
        let base = Snapshot::new(registry.clone());
        let mut changes = base.change_set();
        let id = ElementId::from_u128;
        let origin = || DeclaredOrigin::Authored { source: None };
        let records = BTreeMap::from([
            (id(1), c::PACKAGE),
            (id(2), sc::PART_DEFINITION),
            (id(3), sc::PART_USAGE),
            (id(4), sc::PART_DEFINITION),
            (id(5), sc::PORT_USAGE),
            (id(6), sc::REQUIREMENT_USAGE),
            (id(11), c::OWNING_MEMBERSHIP),
            (id(12), c::FEATURE_MEMBERSHIP),
            (id(13), c::FEATURE_TYPING),
            (id(14), c::FEATURE_MEMBERSHIP),
            (id(15), c::FEATURE_MEMBERSHIP),
            (id(16), c::OWNING_MEMBERSHIP),
        ]);
        for (&element, &class) in &records {
            changes.create(element, class, origin());
            for property in registry
                .effective_properties(class)
                .unwrap()
                .filter(|p| !p.derived && p.multiplicity.lower > 0)
            {
                let value = match registry.storage_kind(property.value_kind).unwrap() {
                    ValueKind::Boolean => Value::Boolean(false),
                    ValueKind::String => Value::String(element.to_string()),
                    ValueKind::Enumeration(domain) => Value::Enumeration(
                        *registry
                            .enumeration(domain)
                            .unwrap()
                            .literals
                            .keys()
                            .next()
                            .unwrap(),
                    ),
                    ValueKind::Reference(_) => continue,
                    other => panic!("unexpected required fixture property: {other:?}"),
                };
                changes.set(element, property.id, SlotValue::Scalar(value), origin());
            }
        }
        for (element, label) in [
            (1, "Project"),
            (2, "System"),
            (3, "engine"),
            (4, "Engine"),
            (5, "control"),
            (6, "safe"),
        ] {
            changes.set(
                id(element),
                p::ELEMENT_DECLARED_NAME,
                SlotValue::Scalar(Value::String(label.into())),
                origin(),
            );
        }
        for (element, property, targets) in [
            (1, p::ELEMENT_OWNED_RELATIONSHIP, vec![11, 16]),
            (2, p::ELEMENT_OWNED_RELATIONSHIP, vec![12, 14, 15]),
            (3, p::ELEMENT_OWNED_RELATIONSHIP, vec![13]),
            (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![2]),
            (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![3]),
            (14, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![5]),
            (15, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![6]),
            (16, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![4]),
            (13, p::SPECIALIZATION_SPECIFIC, vec![3]),
            (13, p::SPECIALIZATION_GENERAL, vec![4]),
        ] {
            let property = registry
                .resolve_property(records[&id(element)], property)
                .unwrap()
                .unwrap();
            let values: Vec<_> = targets
                .into_iter()
                .map(|target| Value::Reference(id(target)))
                .collect();
            let value = if property.multiplicity.upper.is_some_and(|upper| upper <= 1) {
                SlotValue::Scalar(values[0].clone())
            } else if property.ordered {
                SlotValue::Ordered(values)
            } else if property.unique {
                SlotValue::Set(values.into_iter().collect())
            } else {
                SlotValue::Bag(values)
            };
            changes.set(id(element), property.id, value, origin());
        }
        let snapshot = base.apply(&changes).unwrap();
        let model = snapshot.model();
        // The ancestor alias is not itself a legal stored slot on FeatureTyping.
        assert!(
            model
                .navigation_slot(id(13), p::SPECIALIZATION_SPECIFIC)
                .is_none()
        );
        assert_eq!(refs(model, id(13), p::SPECIALIZATION_SPECIFIC), vec![id(3)]);
        assert_eq!(refs(model, id(13), p::SPECIALIZATION_GENERAL), vec![id(4)]);
        let revision = ProjectRevisionId::from_u128(99);
        let edges = graph_edges(model, revision, &records.keys().copied().collect());
        assert!(edges.iter().any(|edge| edge.relationship_id == Some(id(11))
            && edge.family == RelationshipFamily::Ownership
            && edge.source == id(1)
            && edge.target == id(2)));
        assert!(edges.iter().any(|edge| edge.relationship_id == Some(id(12))
            && edge.family == RelationshipFamily::Ownership
            && edge.source == id(2)
            && edge.target == id(3)));
        assert!(edges.iter().any(|edge| edge.relationship_id == Some(id(13))
            && edge.family == RelationshipFamily::Typing
            && edge.source == id(3)
            && edge.target == id(4)));
        assert!(edges.iter().all(|edge| edge.revision_id == revision));
        assert_eq!(owner(model, id(3)), Some(id(2)));
        assert_eq!(owner(model, id(2)), Some(id(1)));
        let (counts, features) = owned_feature_summaries(model, id(2));
        assert_eq!(
            counts,
            FeatureCounts {
                parts: 1,
                ports: 1,
                requirements: 1
            }
        );
        assert_eq!(
            features
                .iter()
                .map(|feature| feature.id)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([id(3), id(5), id(6)])
        );
        assert_eq!(
            qualified_name(model, id(3), id(1)),
            Some("System::engine".into())
        );
    }
}
