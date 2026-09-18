//! Complete source inventory and an attempted atomic runtime registration.
//! Diagnostic output is never a validated runtime descriptor set.
use crate::{
    Result, baseline_diagnostics, canonical_json, closure_audit, descriptors, ir::*,
    pipeline::Bundle,
};
use agq_kernel::metamodel::{
    DescriptorId, DiagnosticCategory, DiagnosticDisposition, DiagnosticSeverity, MetamodelRegistry,
};
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

fn finding(category: &str, code: &str, entity: &Entity, detail: Value) -> Value {
    json!({
        "category": category, "code": code, "source": entity.key.source,
        "source_qualified_id": format!("{}#{}", entity.key.source.artifact_uri, entity.key.external_id),
        "descriptor_id": entity.key.uuid().to_string(), "source_range": entity.source_range,
        "external_id": entity.key.external_id, "detail": detail,
    })
}

fn diagnostic_identity(id: DescriptorId) -> Value {
    let (kind, id) = match id {
        DescriptorId::Metamodel(v) => ("metamodel", v.to_string()),
        DescriptorId::Class(v) => ("class", v.to_string()),
        DescriptorId::Property(v) => ("property", v.to_string()),
        DescriptorId::Association(v) => ("association", v.to_string()),
        DescriptorId::Enumeration(v) => ("enumeration", v.to_string()),
        DescriptorId::Literal(v) => ("literal", v.to_string()),
        DescriptorId::Primitive(v) => ("primitive", v.to_string()),
    };
    json!({"kind":kind,"id":id})
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
    let mut shapes = BTreeMap::<String, Value>::new();
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
                    "read_only": p.is_read_only, "opposites": p.opposite_ends, "redefines": p.redefines, "subsets": p.subsets})
            }).collect::<Vec<_>>(),
            "storage_validation": "not-established-by-descriptor-inventory",
        }));
        if c.member_ends.len() != 2 {
            findings.push(finding("D", "unsupported-association-descriptor-shape", &c.entity,
                json!({"abstract": c.is_abstract, "generalizations": c.generalizations,
                    "arity": c.member_ends.len(), "reason": "Only binary association incidence is supported."})));
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
                "Boolean" | "String" | "Integer" | "Real"
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
    let mut runtime_errors = Vec::new();
    let mut conformance = Vec::new();
    let (translation_error, registration_attempted, registration_error) =
        match descriptors::translate_descriptors(&input, &selected) {
            Err(e) => (Some(e), false, None),
            Ok(set) => {
                let classes: Vec<_> = set.classes.iter().map(|c| c.id).collect();
                let assoc: Vec<_> = set.associations.iter().map(|c| c.id).collect();
                let props = set.properties.clone();
                match MetamodelRegistry::from_descriptors(set) {
                    Err(e) => (None, true, Some(e.to_string())),
                    Ok(registry) => {
                        for (id, c) in &model.classifiers {
                            if c.entity.key.kind != Kind::Association {
                                continue;
                            }
                            let aid =
                                agq_kernel::AssociationId::from_u128(c.entity.key.uuid().as_u128());
                            let a = registry.association(aid).map_err(|e| e.to_string())?;
                            let mut authored = false;
                            let mut storage = Vec::new();
                            for &end in &a.member_ends {
                                let p = registry.property(end).map_err(|e| e.to_string())?;
                                if !p.derived
                                    && registry.is_navigable(end).map_err(|e| e.to_string())?
                                {
                                    authored = true;
                                    storage.push(
                                        if registry
                                            .supports_slot_storage(end)
                                            .map_err(|e| e.to_string())?
                                        {
                                            "canonical-class-slot"
                                        } else if registry
                                            .inverse_storage(end)
                                            .map_err(|e| e.to_string())?
                                            .is_some()
                                        {
                                            "inverse-slot-projection"
                                        } else if registry
                                            .supports_occurrence_storage(aid)
                                            .map_err(|e| e.to_string())?
                                        {
                                            "canonical-association-occurrences"
                                        } else {
                                            "unsupported"
                                        },
                                    );
                                }
                            }
                            if !authored {
                                storage.push("descriptor-only-derived");
                            }
                            let mut shape = associations[id]["member_ends"].clone();
                            for end in shape.as_array_mut().expect("ends") {
                                let raw = end.as_object_mut().expect("end");
                                raw.remove("id");
                                raw.remove("owner");
                                raw.remove("type");
                                raw.remove("opposites");
                                raw.remove("subsets");
                                let redef = !raw["redefines"]
                                    .as_array()
                                    .expect("redefinitions")
                                    .is_empty();
                                raw.insert("redefines".into(), json!(redef));
                            }
                            let shape = json!({"ends":shape,"has_super_association":!a.direct_supertypes.is_empty(),"storage":storage});
                            let key = crate::sha256(&canonical_json(&shape)?);
                            shapes.entry(key.clone()).or_insert(shape);
                            associations.get_mut(id).expect("inventory")["shape_id"] = json!(key);
                            associations.get_mut(id).expect("inventory")["storage_validation"] =
                                json!(storage);
                        }
                        for c in classes {
                            if let Err(e) = registry.effective_properties(c) {
                                runtime_errors.push(e.to_string());
                            }
                        }
                        for a in assoc {
                            if let Err(e) = registry.effective_association_ends(a) {
                                runtime_errors.push(e.to_string());
                            }
                        }
                        for p in props {
                            if !p.derived
                                && registry.is_navigable(p.id).map_err(|e| e.to_string())?
                                && !registry
                                    .supports_slot_storage(p.id)
                                    .map_err(|e| e.to_string())?
                                && registry
                                    .inverse_storage(p.id)
                                    .map_err(|e| e.to_string())?
                                    .is_none()
                                && !p.association.is_some_and(|a| {
                                    registry.supports_occurrence_storage(a).unwrap_or(false)
                                })
                            {
                                runtime_errors.push(format!(
                                    "authored association storage not supported for {}",
                                    p.id
                                ));
                            }
                        }
                        for d in registry.validate_conformance().diagnostics {
                            let disposition = match &d.disposition {
                                DiagnosticDisposition::Unreviewed => json!({"kind":"unreviewed"}),
                                DiagnosticDisposition::ReviewedBaselineAnomaly { evidence } => {
                                    json!({"kind":"reviewed-baseline-anomaly","evidence":evidence})
                                }
                            };
                            let category = match d.category {
                                DiagnosticCategory::Conformance => "conformance",
                                DiagnosticCategory::UnsupportedRuntimeSemantics => {
                                    "unsupported-runtime-semantics"
                                }
                            };
                            let severity = match d.severity {
                                DiagnosticSeverity::Error => "error",
                                DiagnosticSeverity::Warning => "warning",
                            };
                            let diagnostic = json!({"rule": format!("{:?}",d.rule),"severity":severity,"category":category,
                                "subject":diagnostic_identity(d.subject),"related_descriptors":d.related_descriptors.iter().copied().map(diagnostic_identity).collect::<Vec<_>>(),
                                "source": d.source.as_ref().map(|v| json!({"specification": v.specification,"version":v.version,"artifact_uri":v.artifact_uri,"sha256":v.sha256,"external_id":v.external_id,"byte_range":v.byte_range})),
                                "disposition":disposition});
                            findings.push(json!({"category":if d.category==DiagnosticCategory::Conformance { "E" } else { "F" },
                                "code":"metamodel-diagnostic","descriptor_id":diagnostic["subject"]["id"],
                                "source_qualified_id":d.source.as_ref().map(|source|format!("{}#{}",source.artifact_uri,source.external_id)),"diagnostic":diagnostic}));
                            conformance.push(diagnostic);
                        }
                        (None, true, None)
                    }
                }
            }
        };
    let result = if translation_error.is_none()
        && registration_error.is_none()
        && runtime_errors.is_empty()
        && findings.iter().all(|f| f["category"] == "E")
    {
        "representable"
    } else {
        "blocked"
    };
    Ok(json!({
        "format": "agentique-complete-structural-audit/2", "result": result,
        "scope": "Every abstract-syntax classifier/property in the selected language and all dependency metamodels. No runtime publication or semantic-rule execution.",
        "counts": counts, "association_arities": arities, "associations": associations,
        "association_shapes": shapes, "primitive_property_uses": primitives, "derived_unions": unions,
        "findings": findings, "translation_attempted": true,
        "translation_error": translation_error, "registration_attempted": registration_attempted,
        "registration_error": registration_error, "runtime_errors": runtime_errors,
        "conformance_result": if conformance.iter().any(|d| d["severity"] == "error") { "errors" } else { "conformant-to-implemented-rules" }, "conformance": conformance,
        "limitations": ["Not a complete UML constraint checker", "Occurrence incidence is structural; language derivations remain above the kernel", "Retained rules not executed", "Strict conformance is separate from structural readiness"],
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
