//! Source conformance diagnostics are independent of kernel representability.
//! Reviewed dispositions never change source edges or bypass structural validation.
use crate::{Result, descriptors::Closure, ir::*};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "agentique-baseline-diagnostics/1";
pub const CONSTRAINT: &str =
    "https://www.omg.org/spec/UML/20161101/UML.xmi#Property-subsetted_property_names";
const POLICY: &str = include_str!("../../../standards/baseline-anomalies.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    format: String,
    scope: String,
    entries: Vec<ReviewedAnomaly>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedAnomaly {
    specification: String,
    version: String,
    artifact_uri: String,
    sha256: String,
    external_id: String,
    descriptor_id: String,
    target_external_id: String,
    constraint: String,
    disposition: String,
    evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BaselineDiagnostic {
    pub category: String,
    pub source: Source,
    pub external_id: String,
    pub descriptor_id: String,
    pub target: DescriptorKey,
    pub governing_constraint: String,
    pub severity: String,
    pub message: String,
    pub disposition: String,
    pub evidence: Option<String>,
}

/// Check the stated naming constraint over selected edges, including unexpected
/// nonreflexive same-name subsetting. Matching is by exact source identity, never
/// display names, source prefixes, or the mere presence of a self-edge.
pub fn diagnose(model: &Metamodel, selected: &Closure) -> Result<Vec<BaselineDiagnostic>> {
    let policy: Policy = serde_json::from_str(POLICY).map_err(|e| e.to_string())?;
    if policy.format != "agentique-baseline-anomalies/1" || policy.scope.is_empty() {
        return Err("unsupported baseline anomaly policy".into());
    }
    let mut diagnostics = Vec::new();
    for id in &selected.properties {
        let p = &model.properties[id];
        for target in &p.subsets {
            let q = model
                .properties
                .get(target)
                .ok_or("unresolved subset target")?;
            if p.entity.name != q.entity.name {
                continue;
            }
            let key = &p.entity.key;
            let source = &key.source;
            let reviewed: Vec<_> = policy
                .entries
                .iter()
                .filter(|a| {
                    key.kind == Kind::Property
                        && q.entity.key.kind == Kind::Property
                        && a.specification == source.specification
                        && a.version == source.version
                        && a.artifact_uri == source.artifact_uri
                        && a.sha256 == source.sha256
                        && a.external_id == key.external_id
                        && a.descriptor_id == key.uuid().to_string()
                        && q.entity.key.source == *source
                        && a.target_external_id == q.entity.key.external_id
                        && a.constraint == CONSTRAINT
                })
                .collect();
            if reviewed.len() > 1 {
                return Err("duplicate baseline anomaly disposition".into());
            }
            let reviewed = reviewed.first();
            if reviewed.is_some_and(|a| a.disposition != "reviewed-upstream-anomaly-preserve") {
                return Err("unsupported baseline anomaly disposition".into());
            }
            diagnostics.push(BaselineDiagnostic {
                category: "normative-source-anomaly".into(),
                source: source.clone(),
                external_id: key.external_id.clone(),
                descriptor_id: key.uuid().to_string(),
                target: q.entity.key.clone(),
                governing_constraint: CONSTRAINT.into(),
                severity: "error".into(),
                message: "Subsetted property has the same name. Exact source metadata is preserved; structural representability does not establish UML conformance.".into(),
                disposition: reviewed.map_or("unreviewed", |a| a.disposition.as_str()).into(),
                evidence: reviewed.map(|a| a.evidence.clone()),
            });
        }
    }
    Ok(diagnostics)
}
