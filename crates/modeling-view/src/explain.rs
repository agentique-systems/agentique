use crate::{projection::*, *};
use agq_kernel::{
    ModelView,
    provenance::{Dependency, FactKey, Origin},
};
use agq_modeling_workspace::ProjectRevision;
use std::collections::BTreeSet;

/// Proof graph nodes are explicitly presentation objects; facts retain canonical IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplanationNodeKind {
    Element,
    Rule,
    Fact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplanationNode {
    pub id: String,
    pub element_id: Option<ElementId>,
    pub label: String,
    pub kind: ExplanationNodeKind,
}

/// Explanation arrows express proof support, not modeled system relationships.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplanationEdge {
    pub source: String,
    pub target: String,
    pub label: String,
    pub presentation_only: bool,
}

/// Bounded immediate canonical provenance graph. Expand a fact separately for depth.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplanationProjection {
    pub revision_id: ProjectRevisionId,
    pub subject_id: ElementId,
    pub origin: ViewOrigin,
    pub rule_id: Option<RuleId>,
    pub rule_name: Option<String>,
    pub profile: String,
    pub nodes: Vec<ExplanationNode>,
    pub edges: Vec<ExplanationEdge>,
    pub evidence_count: usize,
    pub truncated: bool,
}

/// Explain a canonical relationship, or the first derived fact on a declared element.
/// Never invent a producer or conflate display arrows with canonical relationships.
pub fn explain(
    revision: &ProjectRevision,
    id: ElementId,
) -> Result<ExplanationProjection, ViewError> {
    let model = revision
        .semantic_model()
        .ok_or(ViewError::Unavailable(revision.revision()))?;
    let record = model.element(id).ok_or(ViewError::MissingElement(id))?;
    let (fact, origin) = if matches!(record.origin(), Origin::Derived(_)) {
        (FactKey::Element(id), record.origin())
    } else if let Some((property, slot)) = record
        .slots()
        .find(|(_, s)| matches!(s.origin(), Origin::Derived(_)))
    {
        (
            FactKey::Property {
                element: id,
                property,
            },
            slot.origin(),
        )
    } else {
        (FactKey::Element(id), record.origin())
    };
    let profile = revision
        .sysml_queries()
        .map_err(|e| ViewError::Query(format!("{e:?}")))?
        .context()
        .dependencies
        .sysml_profile;
    let rule_name = if let Origin::Derived(origin) = origin {
        agq_sysml_semantics::sysml_producer_descriptors()
            .iter()
            .find(|d| profile.rule_id(d.id.name()) == origin.rule)
            .map(|d| d.id.name().to_owned())
    } else {
        None
    };
    Ok(explanation_graph(
        model,
        revision.revision(),
        id,
        fact,
        origin,
        profile.id(),
        rule_name,
    ))
}

fn explanation_graph(
    model: &ModelView,
    revision_id: ProjectRevisionId,
    subject_id: ElementId,
    fact: FactKey,
    origin: &Origin,
    profile: &str,
    rule_name: Option<String>,
) -> ExplanationProjection {
    let subject = fact_node(model, fact);
    let subject_key = subject.id.clone();
    let mut nodes = vec![subject];
    let mut edges = vec![];
    let mut evidence_count = 0;
    let mut truncated = false;
    let rule_id = if let Origin::Derived(explanation) = origin {
        let rule_key = format!("rule:{}", explanation.rule);
        nodes.push(ExplanationNode {
            id: rule_key.clone(),
            element_id: None,
            label: rule_name
                .clone()
                .unwrap_or_else(|| format!("Semantic producer {}", explanation.rule)),
            kind: ExplanationNodeKind::Rule,
        });
        edges.push(ExplanationEdge {
            source: rule_key.clone(),
            target: subject_key,
            label: "implies".into(),
            presentation_only: true,
        });
        evidence_count = explanation.dependencies.len();
        let mut seen = BTreeSet::new();
        // Prefer actual subject/endpoint and authored facts in the compact lens.
        // This reorders presentation only: every displayed arrow still cites an
        // immediate dependency in the canonical proof, with omissions explicit.
        let endpoints: BTreeSet<_> = model
            .outgoing(subject_id)
            .map(|reference| reference.target)
            .collect();
        let mut dependencies: Vec<_> = explanation.dependencies.iter().collect();
        dependencies.sort_by_key(|dependency| {
            let fact = match dependency {
                Dependency::Declared(fact) | Dependency::Derived(fact) => fact,
            };
            let element = match fact {
                FactKey::Element(id) | FactKey::Property { element: id, .. } => Some(*id),
                _ => None,
            };
            let endpoint = element.is_some_and(|id| id == subject_id || endpoints.contains(&id));
            let authored = element
                .and_then(|id| model.element(id))
                .is_some_and(|record| provenance(record.origin()) == ViewOrigin::Authored);
            (
                std::cmp::Reverse(endpoint),
                std::cmp::Reverse(authored),
                **dependency,
            )
        });
        const DISPLAY_EVIDENCE_LIMIT: usize = 8;
        for dependency in dependencies.into_iter().take(DISPLAY_EVIDENCE_LIMIT) {
            let (fact, label) = match dependency {
                Dependency::Declared(fact) => (*fact, "declared evidence"),
                Dependency::Derived(fact) => (*fact, "derived evidence"),
            };
            let projected = fact_node(model, fact);
            if seen.insert(projected.id.clone()) {
                edges.push(ExplanationEdge {
                    source: projected.id.clone(),
                    target: rule_key.clone(),
                    label: label.into(),
                    presentation_only: true,
                });
                if !nodes.iter().any(|node| node.id == projected.id) {
                    nodes.push(projected);
                }
            }
        }
        truncated = evidence_count > DISPLAY_EVIDENCE_LIMIT;
        Some(explanation.rule)
    } else {
        None
    };
    ExplanationProjection {
        revision_id,
        subject_id,
        origin: provenance(origin),
        rule_id,
        rule_name,
        profile: profile.into(),
        nodes,
        edges,
        evidence_count,
        truncated,
    }
}

fn fact_node(model: &ModelView, fact: FactKey) -> ExplanationNode {
    match fact {
        FactKey::Element(id) => ExplanationNode {
            id: format!("element:{id}"),
            element_id: Some(id),
            label: format!("{} : {}", name(model, id), kind(model, id)),
            kind: ExplanationNodeKind::Element,
        },
        FactKey::Property { element, property } => ExplanationNode {
            id: format!("property:{element}:{property}"),
            element_id: Some(element),
            label: format!(
                "{} · {}",
                name(model, element),
                model
                    .registry()
                    .property(property)
                    .map_or_else(|_| property.to_string(), |p| p.name.clone())
            ),
            kind: ExplanationNodeKind::Fact,
        },
        FactKey::AssociationOccurrence(id) => ExplanationNode {
            id: format!("occurrence:{id}"),
            element_id: None,
            label: format!("Canonical association {id}"),
            kind: ExplanationNodeKind::Fact,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_kernel::{Snapshot, provenance::Explanation};
    use std::sync::Arc;
    #[test]
    fn explanation_retains_rule_revision_and_explicitly_marks_display_links() {
        let snapshot = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let id = ElementId::from_u128(1);
        let rule = RuleId::from_u128(2);
        let revision = ProjectRevisionId::from_u128(3);
        let dependencies = (4..40)
            .map(|id| Dependency::Declared(FactKey::Element(ElementId::from_u128(id))))
            .collect();
        let origin = Origin::Derived(Arc::new(Explanation { rule, dependencies }));
        let graph = explanation_graph(
            snapshot.model(),
            revision,
            id,
            FactKey::Element(id),
            &origin,
            "test-profile",
            Some("fixtureRule".into()),
        );
        assert_eq!(graph.revision_id, revision);
        assert_eq!(graph.rule_id, Some(rule));
        assert_eq!(graph.evidence_count, 36);
        assert!(graph.truncated);
        assert!(graph.edges.iter().all(|edge| edge.presentation_only));
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| node.kind == ExplanationNodeKind::Rule)
                .count(),
            1
        );
    }
}
