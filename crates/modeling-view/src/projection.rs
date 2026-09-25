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

/// Project a current canonical graph without reconstruction, validation or mutation.
pub fn project(
    revision: &ProjectRevision,
    definition: &ViewDefinition,
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
    let local: BTreeSet<_> = model
        .elements()
        .filter(|r| standard.element(r.id()).is_none())
        .map(ElementRecord::id)
        .collect();
    let mut edges = graph_edges(model, revision.revision(), &local);
    let mut warnings = vec![];
    // Connector endpoints are effective semantic queries, never inferred from the layout.
    if definition
        .relationship_families
        .contains(&RelationshipFamily::Connection)
    {
        let queries = revision
            .kerml_queries()
            .map_err(|e| ViewError::Query(format!("{e:?}")))?;
        edges.extend(connector_edges(
            &queries,
            revision.revision(),
            &local,
            |id, answer| {
                query_warning(
                    &mut warnings,
                    &format!("Connector {} endpoints", name(model, id)),
                    answer,
                );
            },
        ));
    }
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
                let queries = revision
                    .kerml_queries()
                    .map_err(|e| ViewError::Query(format!("{e:?}")))?;
                selected.extend(focused_interfaces(
                    &queries,
                    focus,
                    &context_allowed,
                    &mut warnings,
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
        selected = neighborhood(
            &BTreeSet::from([focus]),
            &selected,
            &edges,
            definition.depth.min(8),
        );
    }
    for hidden in &definition.hidden_elements {
        selected.remove(hidden);
    }
    edges.retain(|edge| selected.contains(&edge.source) && selected.contains(&edge.target));
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    edges.dedup_by(|a, b| a.id == b.id);
    let nodes = selected
        .iter()
        .map(|id| node(revision, *id))
        .collect::<Result<Vec<_>, _>>()?;
    let mut groups = BTreeMap::<ElementId, Vec<ElementId>>::new();
    for item in &nodes {
        if let Some(owner) = item.owner.filter(|id| selected.contains(id)) {
            groups.entry(owner).or_default().push(item.id);
        }
    }
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
            scope: "Current canonical graph; authored context, original identities".into(),
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
                if is(model, child, sc::PART_USAGE) {
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
