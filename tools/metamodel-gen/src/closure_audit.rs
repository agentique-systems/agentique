//! Reproducible runtime-readiness audit; a blocked report is not a passing gate.
use crate::{
    Result, baseline_diagnostics, canonical_json, descriptors, graph, ir::*, pipeline::Bundle,
};
use agq_kernel::metamodel::{MetamodelError, MetamodelRegistry};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

pub const PATH: &str = "standards/generated/sysml-2.0/runtime-closure-audit.json";
pub const SEEDS: &[&str] = &["Systems-Parts-PartDefinition", "Systems-Parts-PartUsage"];

/// Exact source-qualified combined input; this does not certify runtime support.
pub fn runtime_bundle(bundle: &Bundle) -> Result<Bundle> {
    Ok(Bundle {
        format: bundle.format.clone(),
        metamodel: graph::combine(&bundle.metamodel, &bundle.dependencies)?,
        primitive_types: bundle.primitive_types.clone(),
        cross_check: bundle.cross_check.clone(),
        dependencies: BTreeMap::new(),
        representation_differences: vec![],
    })
}

fn context<'a>(model: &'a Metamodel, p: &'a Property) -> &'a str {
    if model.classifiers[&p.owner].entity.key.kind == Kind::Class {
        &p.owner
    } else {
        match &model.properties[&p.opposite_ends[0]].type_ref {
            TypeRef::Local(id) | TypeRef::External(id) => id,
        }
    }
}

fn neighbors(model: &Metamodel, id: &str) -> Vec<String> {
    if let Some(c) = model.classifiers.get(id) {
        c.generalizations
            .iter()
            .chain(&c.properties)
            .chain(&c.member_ends)
            .chain(&c.navigable_owned_ends)
            .cloned()
            .collect()
    } else if let Some(p) = model.properties.get(id) {
        let mut result: Vec<_> = std::iter::once(&p.owner)
            .chain(&p.redefines)
            .chain(&p.subsets)
            .chain(&p.opposite_ends)
            .chain(p.association.iter())
            .cloned()
            .collect();
        match &p.type_ref {
            TypeRef::Local(id) | TypeRef::External(id) => result.push(id.clone()),
        }
        result
    } else {
        vec![]
    }
}

pub fn report(bundle: &Bundle) -> Result<Value> {
    let seeds: Vec<_> = SEEDS
        .iter()
        .map(|id| graph::qualified(&bundle.metamodel, id))
        .collect();
    let graph_bundle = runtime_bundle(bundle)?;
    let model = &graph_bundle.metamodel;
    let selected =
        descriptors::closure(model, &seeds.iter().map(String::as_str).collect::<Vec<_>>())?;
    let mut counts = BTreeMap::<String, usize>::new();
    for id in &selected.classifiers {
        let key = &model.classifiers[id].entity.key;
        *counts
            .entry(format!("{}::{:?}", key.source.specification, key.kind))
            .or_default() += 1;
    }
    let mut predecessors: BTreeMap<String, Option<String>> =
        seeds.iter().map(|id| (id.clone(), None)).collect();
    let mut queue = VecDeque::from(seeds.clone());
    while let Some(id) = queue.pop_front() {
        for next in neighbors(model, &id) {
            if !predecessors.contains_key(&next) {
                predecessors.insert(next.clone(), Some(id.clone()));
                queue.push_back(next);
            }
        }
    }
    let path_to = |id: &str| {
        let mut path = vec![id.to_owned()];
        while let Some(Some(previous)) = predecessors.get(path.last().unwrap()) {
            path.push(previous.clone());
        }
        path.reverse();
        path
    };
    let mut self_subsets = Vec::new();
    for id in &selected.properties {
        let p = &model.properties[id];
        if p.subsets.contains(id) {
            self_subsets.push(json!({
                "source_qualified_id": id, "descriptor_id": p.entity.key.uuid().to_string(),
                "source": p.entity.key.source, "source_range": p.entity.source_range,
                "owner": p.owner, "subsets": p.subsets, "required_by": path_to(id),
            }));
        }
    }
    let diagnostics = baseline_diagnostics::diagnose(model, &selected)?;
    let unreviewed = diagnostics.iter().any(|d| d.disposition == "unreviewed");
    let translated = descriptors::translate_descriptors(&graph_bundle, &selected);
    let mut registration_failure = Value::Null;
    let (structural_status, error) = match translated {
        Err(error) => ("unsupported-runtime-construct", Some(error)),
        Ok(set) => match MetamodelRegistry::from_descriptors(set) {
            Ok(_) => ("structurally-representable", None),
            Err(error) => {
                if let MetamodelError::InvalidRedefinition { property, base } = &error {
                    let (id, p) = model
                        .properties
                        .iter()
                        .find(|(_, p)| p.entity.key.property_id() == Ok(*property))
                        .expect("translated property");
                    let (base_id, b) = model
                        .properties
                        .iter()
                        .find(|(_, p)| p.entity.key.property_id() == Ok(*base))
                        .expect("translated property");
                    let c = context(model, p);
                    let bc = context(model, b);
                    let ancestry = descriptors::effective(model, c).0;
                    registration_failure = json!({
                        "kind": "invalid-redefinition", "property": p.entity,
                        "source_qualified_id": id, "base": b.entity,
                        "base_source_qualified_id": base_id,
                        "property_context": c, "base_context": bc,
                        "context_is_strict_subtype": c != bc && ancestry.contains(bc),
                        "context_ancestors_including_self": ancestry,
                        "required_by": path_to(id),
                        "disposition": "rejected-no-baseline-waiver",
                    });
                }
                (
                    "invalid-generated-descriptor",
                    Some(format!("generated descriptor validation: {error}")),
                )
            }
        },
    };
    let result = if error.is_none() {
        "representable"
    } else {
        "blocked"
    };
    // Retain the historical closure comparison independently of full production selection.
    let mut extended = seeds.clone();
    for id in [
        "Systems-Attributes-AttributeDefinition",
        "Systems-Attributes-AttributeUsage",
        "Systems-Ports-PortDefinition",
        "Systems-Ports-PortUsage",
        "Systems-Connections-ConnectionDefinition",
        "Systems-Connections-ConnectionUsage",
    ] {
        extended.push(graph::qualified(&bundle.metamodel, id));
    }
    let extended_closure = descriptors::closure(
        model,
        &extended.iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    let report = json!({
        "_generated": "GENERATED FILE. DO NOT EDIT. cargo run --locked --offline -p agq-metamodel-gen",
        "generator": "agq-structural-closure-audit/2", "result": result,
        "structural_status": structural_status,
        "conformance_diagnostics_format": baseline_diagnostics::FORMAT,
        "conformance_diagnostics_scope": "Property-subsetted_property_names only; not a full UML conformance audit.",
        "baseline_diagnostics": diagnostics,
        "has_unreviewed_baseline_diagnostics": unreviewed,
        "registration_failure": registration_failure,
        "scope": "Structural runtime readiness only; does not execute retained rules or certify conformance.",
        "seeds": seeds, "classifiers": counts, "properties": selected.properties.len(),
        "external_domains": selected.external_types, "same_closure_with_attributes_ports_connections": selected == extended_closure,
        "self_subsetting_properties": self_subsets, "registration_error": error,
        "selected_class_ids": selected.classifiers.iter().filter(|id| model.classifiers[*id].entity.key.kind == Kind::Class).collect::<Vec<_>>(),
    });
    Ok(report)
}
pub fn bytes(bundle: &Bundle) -> Result<Vec<u8>> {
    canonical_json(&report(bundle)?)
}
