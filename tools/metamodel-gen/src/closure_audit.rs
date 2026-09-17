//! Reproducible runtime-readiness audit; a blocked report is not a passing gate.
use crate::{Result, canonical_json, descriptors, graph, ir::*, pipeline::Bundle};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

pub const PATH: &str = "standards/generated/sysml-2.0/structural-audit.json";
pub const SEEDS: &[&str] = &["Systems-Parts-PartDefinition", "Systems-Parts-PartUsage"];

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
    let graph_bundle = Bundle {
        format: bundle.format.clone(),
        metamodel: graph::combine(&bundle.metamodel, &bundle.dependencies)?,
        primitive_types: bundle.primitive_types.clone(),
        cross_check: bundle.cross_check.clone(),
        dependencies: BTreeMap::new(),
        representation_differences: vec![],
    };
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
    let mut self_subsets = Vec::new();
    for id in &selected.properties {
        let p = &model.properties[id];
        if p.subsets.contains(id) {
            let mut path = vec![id.clone()];
            while let Some(Some(previous)) = predecessors.get(path.last().unwrap()) {
                path.push(previous.clone());
            }
            path.reverse();
            self_subsets.push(json!({
                "source_qualified_id": id, "descriptor_id": p.entity.key.uuid().to_string(),
                "source": p.entity.key.source, "source_range": p.entity.source_range,
                "owner": p.owner, "subsets": p.subsets, "required_by": path,
            }));
        }
    }
    let registration = descriptors::descriptor_set(&graph_bundle, &selected);
    let result = if registration.is_ok() {
        "representable"
    } else {
        "blocked"
    };
    let error = registration.err();
    // Verify the optional structural seeds cannot remove the minimum-slice blocker.
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
        "generator": "agq-structural-closure-audit/1", "result": result,
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
