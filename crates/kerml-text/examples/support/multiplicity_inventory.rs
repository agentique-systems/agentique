//! Exhaustive source-bound inventory. Bounds are structural Expressions, never values.
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{Completeness, KerMlQueries};
use agq_kerml_text::library::LibrarySourceMap;
use agq_kernel::{ElementId, MetaclassId, ModelView, provenance::FactKey, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn is(model: &ModelView, element: ElementId, class: MetaclassId) -> bool {
    model.element(element).is_some_and(|r| {
        model
            .registry()
            .is_subtype(r.metaclass(), class)
            .unwrap_or(false)
    })
}

fn source(element: ElementId, map: &LibrarySourceMap, sources: &VerifiedLibrarySet) -> Json {
    let Some(origin) = map.get(&FactKey::Element(element)) else {
        return Json::Null;
    };
    let document = sources
        .documents()
        .find(|d| d.document() == origin.document)
        .expect("verified source document");
    json!({"document":document.path(), "sha256":document.sha256(),
        "range":[origin.range.start(),origin.range.end()],
        "text":&document.source()[origin.range.start() as usize..origin.range.end() as usize]})
}

fn path(
    model: &ModelView,
    queries: &KerMlQueries<'_>,
    element: ElementId,
    display_owners: &mut BTreeMap<ElementId, Option<ElementId>>,
) -> String {
    let mut names = Vec::new();
    let mut next = Some(element);
    let mut seen = BTreeSet::new();
    while let Some(element) = next {
        if !seen.insert(element) {
            break;
        }
        if let Some(slot) = model.navigation_slot(element, p::ELEMENT_DECLARED_NAME) {
            for value in slot.value().values() {
                if let Value::String(name) = value
                    && !name.is_empty()
                {
                    names.push(name.clone());
                }
            }
        }
        // Presentation-only cache over this exact immutable input. Semantic
        // ownership/completeness checks below still run for every range.
        next = *display_owners
            .entry(element)
            .or_insert_with(|| queries.owner(element).value);
    }
    names.reverse();
    names.join("::")
}

/// Traverse only canonical ownership, so a reference to another Expression never
/// imports that referent's nested bound population into this bound.
fn reference_expressions(model: &ModelView, root: ElementId) -> BTreeSet<ElementId> {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    let mut references = BTreeSet::new();
    while let Some(element) = pending.pop() {
        if !seen.insert(element) {
            continue;
        }
        if is(model, element, c::FEATURE_REFERENCE_EXPRESSION) {
            references.insert(element);
        }
        pending.extend(
            model
                .outgoing(element)
                .filter(|edge| {
                    matches!(
                        edge.property,
                        p::ELEMENT_OWNED_RELATIONSHIP | p::RELATIONSHIP_OWNED_RELATED_ELEMENT
                    )
                })
                .map(|edge| edge.target),
        );
    }
    references
}

/// `audit` is enabled only after producer closure. Before closure the same report
/// inventories exact source without claiming that any implied result exists.
pub fn collect(
    model: &ModelView,
    queries: &KerMlQueries<'_>,
    map: &LibrarySourceMap,
    sources: &VerifiedLibrarySet,
    selected: Option<&BTreeSet<ElementId>>,
    audit: bool,
) -> Result<Json, Box<dyn std::error::Error>> {
    let mut counts = BTreeMap::from([
        ("ordinary_multiplicity_reference_bounds", 0usize),
        ("cross_feature_multiplicity_reference_bounds", 0),
        ("literal_bounds", 0),
        ("unbounded_bounds", 0),
        ("other_expression_bounds", 0),
    ]);
    let mut rows = Vec::new();
    let mut incomplete = Vec::new();
    let mut reference_ids = BTreeSet::new();
    let mut ranges = 0;
    let mut display_owners = BTreeMap::new();
    for record in model.instances(c::MULTIPLICITY_RANGE, true)? {
        let multiplicity = record.id();
        if !map.contains_key(&FactKey::Element(multiplicity))
            || selected.is_some_and(|set| !set.contains(&multiplicity))
        {
            continue;
        }
        ranges += 1;
        if audit && ranges % 128 == 0 {
            println!(
                "Multiplicity inventory: {ranges} ranges; {} findings",
                incomplete.len()
            );
        }
        // Keep ordinary proof caches bounded to one MultiplicityRange.
        let q = queries.fork();
        let owner = q.owner(multiplicity);
        display_owners.insert(multiplicity, owner.value);
        let qualified_path = path(model, &q, multiplicity, &mut display_owners);
        let bounds = q.multiplicity_bounds(multiplicity);
        if owner.completeness != Completeness::Complete
            || bounds.completeness != Completeness::Complete
        {
            incomplete.push(
                json!({"multiplicity":multiplicity.to_string(), "phase":"ownership_or_bounds"}),
            );
        }
        let cross = owner
            .value
            .filter(|&id| is(model, id, c::FEATURE))
            .map(|id| q.is_owned_cross_feature(id));
        if cross
            .as_ref()
            .is_some_and(|q| q.completeness != Completeness::Complete)
        {
            incomplete.push(
                json!({"multiplicity":multiplicity.to_string(), "phase":"cross_feature_selection"}),
            );
        }
        let cross = cross.is_some_and(|q| q.value);
        for bound in bounds.value.bound {
            let references = reference_expressions(model, bound);
            let category = if !references.is_empty() {
                if cross {
                    "cross_feature_multiplicity_reference_bounds"
                } else {
                    "ordinary_multiplicity_reference_bounds"
                }
            } else if is(model, bound, c::LITERAL_INFINITY) {
                "unbounded_bounds"
            } else if is(model, bound, c::LITERAL_EXPRESSION) {
                "literal_bounds"
            } else {
                "other_expression_bounds"
            };
            *counts.get_mut(category).expect("inventory category") += 1;
            let mut findings = Vec::new();
            if audit && !references.is_empty() {
                let multiplicity_context = q.featuring_types(multiplicity);
                let bound_context = q.featuring_types(bound);
                if multiplicity_context.completeness != Completeness::Complete
                    || bound_context.completeness != Completeness::Complete
                    || multiplicity_context.value != bound_context.value
                {
                    findings.push(json!({"phase":"bound_featuring", "multiplicity_status":format!("{:?}",multiplicity_context.completeness),
                        "bound_status":format!("{:?}",bound_context.completeness),
                        "multiplicity_context":multiplicity_context.value.iter().map(ToString::to_string).collect::<Vec<_>>(),
                        "bound_context":bound_context.value.iter().map(ToString::to_string).collect::<Vec<_>>()}));
                }
            }
            let mut reference_rows = Vec::new();
            for expression in references {
                reference_ids.insert(expression.to_string());
                let mut row = json!({"expression":expression.to_string(), "source":source(expression,map,sources)});
                if audit {
                    let referent = q.reference_referent(expression);
                    let result = q.structural_result(expression);
                    let binding = referent
                        .value
                        .zip(result.value)
                        .map(|(target, raw)| q.reference_binding_context(expression, target, raw));
                    let complete = referent.completeness == Completeness::Complete
                        && result.completeness == Completeness::Complete
                        && binding
                            .as_ref()
                            .is_some_and(|b| b.completeness == Completeness::Complete);
                    row["complete"] = json!(complete);
                    row["referent"] = json!(referent.value.map(|id| id.to_string()));
                    row["result"] = json!(result.value.map(|id| id.to_string()));
                    row["binding_context"] = json!(
                        binding
                            .as_ref()
                            .and_then(|b| b.value)
                            .map(|id| id.to_string())
                    );
                    if !complete {
                        findings.push(json!({"phase":"reference_binding_context", "expression":expression.to_string(),
                            "referent_status":format!("{:?}",referent.completeness), "result_status":format!("{:?}",result.completeness),
                            "binding_status":binding.as_ref().map(|b|format!("{:?}",b.completeness)),
                            "diagnostics":binding.as_ref().map(|b|b.diagnostics.iter().map(|d|json!({"code":d.code,"message":d.message})).collect::<Vec<_>>())}));
                    }
                }
                reference_rows.push(row);
            }
            if !findings.is_empty() {
                incomplete.push(json!({"bound":bound.to_string(), "findings":findings}));
            }
            rows.push(json!({"multiplicity":multiplicity.to_string(), "bound":bound.to_string(),
                "owner":owner.value.map(|id|id.to_string()), "qualified_path":qualified_path,
                "category":category, "source":source(bound,map,sources), "references":reference_rows}));
        }
    }
    Ok(
        json!({"format":"agentique-multiplicity-bound-inventory/1", "profile":queries.context().baseline_profile_id,
        "source_content_set":sources.content_set_id(), "multiplicity_ranges":ranges, "counts":counts,
        "reference_expression_count":reference_ids.len(), "reference_expression_ids":reference_ids,
        "semantic_audit_executed":audit, "semantic_audit_scope":"Every direct or nested FeatureReferenceExpression bound and its bound-expression featuring context", "complete":incomplete.is_empty(), "findings":incomplete, "bounds":rows}),
    )
}

/// Preserve the complete 9 ordinary + 6 cross reference-binding population from
/// the earlier investigation, independently of the newly authorized interpretation.
pub fn historical_population(
    report: &Json,
    root: &Path,
) -> Result<Json, Box<dyn std::error::Error>> {
    let history: Json = serde_json::from_slice(&std::fs::read(root.join(
        "verification/kerml-semantic-closure-v10/remaining-structural-investigations.json",
    ))?)?;
    let actual: BTreeSet<_> = report["reference_expression_ids"]
        .as_array()
        .ok_or("reference inventory")?
        .iter()
        .filter_map(Json::as_str)
        .collect();
    let previous = history["reference_binding_contexts"]
        .as_array()
        .ok_or("historical reference population")?;
    let missing: Vec<_> = previous.iter().filter_map(|row| {
        let id = row["expression"]["id"].as_str()?;
        (!actual.contains(id)).then(||json!({"expression":id, "path":row["expression"]["path"], "source":row["expression"]["source"]["text"]}))
    }).collect();
    Ok(
        json!({"retained_source":"verification/kerml-semantic-closure-v10/remaining-structural-investigations.json",
        "previous_count":previous.len(), "previous_groups":history["context_groups"], "missing":missing, "complete":missing.is_empty()}),
    )
}
