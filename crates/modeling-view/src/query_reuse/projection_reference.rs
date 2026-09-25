//! Unoptimized body preserved from d404a8759891acc5d621dcad655fa4a981e17d63.
//! Source blob af7e104dae64a3bdff47e564441a001a2891926e; test-only oracle.
use super::*;

pub(crate) fn project(
    revision: &ProjectRevision,
    definition: &ViewDefinition,
) -> Result<ViewProjection, ViewError> {
    project_observed(revision, definition, &mut ViewProfile::disabled())
}

pub(crate) fn current(
    revision: &ProjectRevision,
    definition: &ViewDefinition,
) -> Result<ViewProjection, ViewError> {
    super::project_observed(revision, definition, &mut ViewProfile::disabled())
}

pub(crate) fn focused_probe(
    queries: &KerMlQueries<'_>,
    focus: ElementId,
    allowed: &BTreeSet<ElementId>,
) -> (BTreeSet<ElementId>, Vec<String>) {
    let mut warnings = vec![];
    let selected = focused_interfaces(queries, focus, allowed, &mut warnings);
    (selected, warnings)
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
    // Connector endpoints are effective semantic queries, never inferred from the layout.
    if definition
        .relationship_families
        .contains(&RelationshipFamily::Connection)
    {
        let queries = profile.measure("connector_kerml_context", None, || {
            revision
                .kerml_queries()
                .map_err(|e| ViewError::Query(format!("{e:?}")))
        })?;
        let mut connector_queries = 0;
        edges.extend(profile.measure("connector_queries", None, || {
            connector_edges(&queries, revision.revision(), &local, |id, answer| {
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
                let queries =
                    profile.measure("focused_kerml_context", Some("selection_scope"), || {
                        revision
                            .kerml_queries()
                            .map_err(|e| ViewError::Query(format!("{e:?}")))
                    })?;
                selected.extend(profile.measure(
                    "focused_interface_queries",
                    Some("selection_scope"),
                    || focused_interfaces(&queries, focus, &context_allowed, &mut warnings),
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
