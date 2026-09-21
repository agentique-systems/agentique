//! Run every existing result producer over the complete v7 construction.
//! Plans remain unpublished when storage or semantic obligations are unresolved.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::Completeness;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeSet, io::Write, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let draft = agq_kerml_text::library::refine_declarations_with_profile(
        &sources,
        BaselineProfile::OPERATIONAL_V7,
        |round, refs, obligations| {
            println!("refinement {round}: {refs} endpoints, {obligations} obligations")
        },
    )?;
    let model = draft.candidate().model();
    let context = draft.queries(&sources)?.context().clone();
    let mut reference_rows = vec![];
    for batch in draft.references().chunks(128) {
        let q = draft.queries(&sources)?;
        assert_eq!(q.context(), &context);
        for r in batch {
            let answer = q.lookup_relationship_target(r.relationship, r.property, &r.name);
            reference_rows.push(json!({"relationship":r.relationship.to_string(),"name":r.name.segments,
                "candidates":answer.value.iter().map(|v|v.element.to_string()).collect::<Vec<_>>(),
                "completeness":format!("{:?}",answer.completeness),
                "diagnostics":answer.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>()}));
        }
    }
    let subjects: Vec<_> = model
        .elements()
        .filter(|r| {
            [c::FEATURE, c::FUNCTION]
                .into_iter()
                .any(|c| model.registry().is_subtype(r.metaclass(), c).unwrap())
        })
        .map(|r| r.id())
        .collect();
    let mut elements = BTreeSet::new();
    let mut diagnostics = BTreeSet::new();
    let mut rows = vec![];
    for (index, batch) in subjects.chunks(128).enumerate() {
        let q = draft.queries(&sources)?;
        assert_eq!(q.context(), &context);
        let plan = q.plan_result_structure(batch.iter().copied());
        elements.extend(plan.planned_elements());
        diagnostics.extend(plan.production.diagnostics.iter().cloned());
        rows.push(json!({"index":index,"subjects":batch.len(),"planned_elements":plan.planned_elements().count(),
            "bindings":plan.production.value.len(),"contextual_results":plan.contextual_results.len(),
            "completeness":format!("{:?}",plan.production.completeness)}));
        if index % 10 == 0 {
            println!(
                "producer batch {index}: {} unique planned elements; {} diagnostics",
                elements.len(),
                diagnostics.len()
            );
        }
    }
    let incomplete = reference_rows
        .iter()
        .filter(|r| r["completeness"] != "Complete")
        .count();
    let unresolved = reference_rows
        .iter()
        .filter(|r| r["candidates"].as_array().unwrap().is_empty())
        .count();
    let ambiguous = reference_rows
        .iter()
        .filter(|r| r["candidates"].as_array().unwrap().len() > 1)
        .count();
    let producers_complete = rows
        .iter()
        .all(|r| r["completeness"] == format!("{:?}", Completeness::Complete));
    let report = json!({"format":"agentique-v7-full-producer-audit/1","profile":draft.baseline_profile().id(),
        "library_set":sources.content_set_id(),"canonical_records":model.elements().count(),"documents":36,
        "required_references":reference_rows.len(),"unresolved":unresolved,"incomplete":incomplete,"ambiguous":ambiguous,
        "construction_obligations":draft.candidate().obligations().len(),
        "reference_answers":reference_rows,"producer_subjects":subjects.len(),"batches":rows,
        "planned_elements":elements.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "producer_diagnostics":diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>(),
        "producers_complete":producers_complete,"overlay_materialized":false,"accepted":false,
        "scope":"All existing result producer families executed over the full three-library v7 construction. Proposed records are not a validated overlay; other structural producers and strict acceptance remain separate mandatory gates."});
    let path = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output is required")?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(format!("{}\n", serde_json::to_string_pretty(&report)?).as_bytes())?;
    println!(
        "References: {unresolved} unresolved, {incomplete} incomplete, {ambiguous} ambiguous; {} planned elements; producers complete: {producers_complete}",
        elements.len()
    );
    Ok(())
}
