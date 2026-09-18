//! Complete source inventory and an attempted atomic runtime registration.
//! Diagnostic output is never a validated runtime descriptor set.
use crate::{
    Result, baseline_diagnostics, canonical_json, closure_audit, descriptors, ir::*,
    pipeline::Bundle,
};
use agq_kernel::metamodel::MetamodelRegistry;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Select every classifier and property, including dependency metamodels.
/// Do not use a language feature's dependency closure as a coverage surrogate.
pub fn selection(model: &Metamodel) -> descriptors::Closure {
    descriptors::Closure {
        classifiers: model.classifiers.keys().cloned().collect(),
        properties: model.properties.keys().cloned().collect(),
        external_types: model
            .properties
            .values()
            .filter_map(|p| match &p.type_ref {
                TypeRef::External(id) => Some(id.clone()),
                TypeRef::Local(_) => None,
            })
            .collect(),
    }
}

fn ancestors(model: &Metamodel, id: &str) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending = model.classifiers[id].generalizations.clone();
    while let Some(next) = pending.pop() {
        if seen.insert(next.clone()) {
            pending.extend(model.classifiers[&next].generalizations.clone());
        }
    }
    seen
}

fn contexts(model: &Metamodel, p: &Property) -> BTreeSet<String> {
    if p.association.is_some() {
        p.opposite_ends
            .iter()
            .map(|id| match &model.properties[id].type_ref {
                TypeRef::Local(id) | TypeRef::External(id) => id.clone(),
            })
            .collect()
    } else {
        BTreeSet::from([p.owner.clone()])
    }
}

fn finding(category: &str, code: &str, entity: &Entity, detail: Value) -> Value {
    json!({
        "category": category, "code": code, "source": entity.key.source,
        "source_qualified_id": format!("{}#{}", entity.key.source.artifact_uri, entity.key.external_id),
        "descriptor_id": entity.key.uuid().to_string(), "source_range": entity.source_range,
        "external_id": entity.key.external_id, "detail": detail,
    })
}

/// Inventory all shapes and translation gaps without skipping unsupported facts.
/// Candidate context mismatches are review obligations, not UML verdicts.
pub fn report(bundle: &Bundle) -> Result<Value> {
    let input = closure_audit::runtime_bundle(bundle)?;
    let model = &input.metamodel;
    let selected = selection(model);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut arities = BTreeMap::<usize, usize>::new();
    let mut associations = BTreeMap::new();
    let mut primitives = BTreeMap::<String, Vec<String>>::new();
    let mut findings = Vec::new();
    let mut unions = BTreeMap::new();
    for (id, c) in &model.classifiers {
        *counts
            .entry(format!(
                "{}::{:?}",
                c.entity.key.source.specification, c.entity.key.kind
            ))
            .or_default() += 1;
        if c.entity.key.kind != Kind::Association {
            continue;
        }
        *arities.entry(c.member_ends.len()).or_default() += 1;
        associations.insert(id, json!({
            "descriptor_id": c.entity.key.uuid().to_string(),
            "source": c.entity.key.source, "source_range": c.entity.source_range,
            "direct_supertypes": c.generalizations, "ancestors": ancestors(model, id),
            "member_ends": c.member_ends.iter().map(|end| {
                let p = &model.properties[end];
                json!({"id": end, "owner": p.owner, "type": p.type_ref,
                    "class_owned": model.classifiers[&p.owner].entity.key.kind == Kind::Class,
                    "navigable": model.classifiers[&p.owner].entity.key.kind == Kind::Class || c.navigable_owned_ends.contains(end),
                    "lower": p.lower, "upper": p.upper, "ordered": p.is_ordered,
                    "unique": p.is_unique, "derived": p.is_derived,
                    "read_only": p.is_read_only, "opposites": p.opposite_ends})
            }).collect::<Vec<_>>(),
            "storage_validation": "not-established-by-descriptor-inventory",
        }));
        if c.is_abstract || !c.generalizations.is_empty() || c.member_ends.len() != 2 {
            findings.push(finding("D", "unsupported-association-descriptor-shape", &c.entity,
                json!({"abstract": c.is_abstract, "generalizations": c.generalizations,
                    "arity": c.member_ends.len(), "reason": "Current translator/registry supports concrete binary associations without inheritance."})));
        }
    }
    for (id, p) in &model.properties {
        *counts
            .entry(format!("{}::Property", p.entity.key.source.specification))
            .or_default() += 1;
        if let TypeRef::External(target) = &p.type_ref {
            primitives
                .entry(target.clone())
                .or_default()
                .push(id.clone());
            let primitive = input
                .primitive_types
                .classifiers
                .values()
                .find(|c| {
                    format!(
                        "{}#{}",
                        c.entity.key.source.artifact_uri, c.entity.key.external_id
                    ) == *target
                })
                .ok_or_else(|| format!("unresolved complete-audit domain {target}"))?;
            if !matches!(
                primitive.entity.key.external_id.as_str(),
                "Boolean" | "String"
            ) {
                findings.push(finding("C", "unsupported-primitive-domain", &p.entity,
                    json!({"domain": primitive.entity, "reason": "No normative runtime representation; never map unbounded Integer to i64 or Real to f64 implicitly."})));
            }
        }
        if p.is_read_only || p.aggregation == Aggregation::Shared {
            findings.push(finding(
                "D",
                "unsupported-property-metadata",
                &p.entity,
                json!({"read_only": p.is_read_only, "aggregation": p.aggregation}),
            ));
        }
        for base in &p.redefines {
            let b = &model.properties[base];
            let pc = contexts(model, p);
            let bc = contexts(model, b);
            // This probes the existing binary-context interpretation. Do not
            // promote it to the unresolved Property-specific normative rule.
            if !pc
                .iter()
                .all(|c| bc.iter().any(|a| ancestors(model, c).contains(a)))
            {
                findings.push(finding("F", "property-redefinition-context-authority", &p.entity,
                    json!({"base": b.entity, "base_descriptor_id": b.entity.key.uuid().to_string(),
                        "property_contexts": pc, "base_contexts": bc,
                        "property_context_ancestors": pc.iter().map(|c| (c, ancestors(model, c))).collect::<BTreeMap<_,_>>(),
                        "owner": p.owner, "base_owner": b.owner,
                        "association": p.association, "base_association": b.association,
                        "reason": "Context probe fails. ADR 0009 records the final-publication/resolution authority gap; no metadata is waived."})));
            }
        }
        if p.is_derived_union {
            let direct: BTreeSet<_> = model
                .properties
                .iter()
                .filter(|(_, p)| p.subsets.contains(id))
                .map(|(id, _)| id.clone())
                .collect();
            let mut pending: Vec<_> = direct.iter().cloned().collect();
            let mut transitive = BTreeSet::new();
            while let Some(next) = pending.pop() {
                if transitive.insert(next.clone()) {
                    pending.extend(
                        model
                            .properties
                            .iter()
                            .filter(|(_, p)| p.subsets.contains(&next))
                            .map(|(id, _)| id.clone()),
                    );
                }
            }
            unions.insert(id, json!({"direct_contributors": direct, "transitive_contributors": transitive,
                "reaches_self": transitive.contains(id), "ordered": p.is_ordered, "unique": p.is_unique,
                "value_state": "not-computed", "evaluation": "metadata-reachability-only"}));
        }
    }
    let diagnostics = baseline_diagnostics::diagnose(model, &selected)?;
    for d in &diagnostics {
        findings.push(json!({"category": "E", "code": "subsetted-property-name", "diagnostic": d}));
    }
    let (translation_error, registration_attempted, registration_error) =
        match descriptors::translate_descriptors(&input, &selected) {
            Err(e) => (Some(e), false, None),
            Ok(set) => (
                None,
                true,
                MetamodelRegistry::from_descriptors(set)
                    .err()
                    .map(|e| e.to_string()),
            ),
        };
    let result = if translation_error.is_none()
        && registration_error.is_none()
        && findings.iter().all(|f| {
            f["category"] == "E"
                && f["diagnostic"]["disposition"] == "reviewed-upstream-anomaly-preserve"
        }) {
        "representable"
    } else {
        "blocked"
    };
    Ok(json!({
        "format": "agentique-complete-structural-audit/1", "result": result,
        "scope": "Every abstract-syntax classifier/property in the selected language and all dependency metamodels. No runtime publication or semantic-rule execution.",
        "counts": counts, "association_arities": arities, "associations": associations,
        "primitive_property_uses": primitives, "derived_unions": unions,
        "findings": findings, "translation_attempted": true,
        "translation_error": translation_error, "registration_attempted": registration_attempted,
        "registration_error": registration_error,
        "limitations": ["Not a complete UML constraint checker", "Association storage not certified", "Retained rules not executed", "Context probe is not an accepted replacement rule"],
    }))
}

/// Complete raw manifest, even when runtime translation fails. Identity v1 is unchanged.
pub fn manifest(bundle: &Bundle) -> Result<Value> {
    let input = closure_audit::runtime_bundle(bundle)?;
    let mut result = descriptors::golden(&input, &selection(&input.metamodel));
    result["format"] = json!("agentique-complete-structural-manifest/1");
    result["scope"] = json!(
        "Complete raw structural inventory; effective-property projections are tooling calculations, not validated runtime claims."
    );
    result["seeds"] = json!("all-classifiers-and-properties");
    result["inputs"] = json!(
        std::iter::once(&bundle.metamodel.source)
            .chain(bundle.dependencies.values().map(|m| &m.source))
            .chain(std::iter::once(&bundle.primitive_types.source))
            .collect::<Vec<_>>()
    );
    Ok(result)
}

/// Deterministic reports; historical closure audit and runtime bytes stay separate.
pub fn artifacts(bundle: &Bundle, baseline: &str) -> Result<Vec<(String, Vec<u8>)>> {
    let prefix = format!("standards/generated/{baseline}");
    Ok(vec![
        (
            format!("{prefix}/full-audit.json"),
            canonical_json(&report(bundle)?)?,
        ),
        (
            format!("{prefix}/full.golden.json"),
            canonical_json(&manifest(bundle)?)?,
        ),
    ])
}
