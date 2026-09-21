//! Independent real-corpus preflights. Scoped closure never seals a publication.
use agq_kerml_semantics::*;
use agq_kernel::{provenance::Origin, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeSet, path::Path};
#[path = "support/publication_input.rs"]
mod publication_input;
use publication_input::PublicationInput;
#[path = "support/publication_metrics.rs"]
mod publication_metrics;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let slice = std::env::args()
        .find_map(|a| a.strip_prefix("--slice=").map(str::to_owned))
        .ok_or("--slice=A|B|C|D|E|all is required")?;
    let slices: Vec<_> = if slice == "all" {
        vec!["A", "B", "C", "D", "E"]
    } else {
        vec![slice.as_str()]
    };
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let input = PublicationInput::load(&sources)?;
    let mut failed = false;
    for slice in slices {
        if let Err(error) = run_slice(&root, &sources, &input, slice) {
            eprintln!("slice {slice}: {error}");
            failed = true;
        }
    }
    if failed {
        return Err("one or more independent slices failed".into());
    }
    Ok(())
}

fn run_slice(
    root: &Path,
    sources: &VerifiedLibrarySet,
    input: &PublicationInput,
    slice: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let documents: &[&str] = match slice {
        "A" => &["Base.kerml", "Objects.kerml", "Links.kerml"],
        "B" => &["Occurrences.kerml", "Transfers.kerml"],
        "C" => &[
            "Performances.kerml",
            "BaseFunctions.kerml",
            "DataFunctions.kerml",
            "ControlFunctions.kerml",
        ],
        "D" => &["Observation.kerml", "FeatureReferencingPerformances.kerml"],
        "E" => &[
            "Triggers.kerml",
            "StatePerformances.kerml",
            "ControlPerformances.kerml",
            "TransitionPerformances.kerml",
        ],
        _ => return Err("unknown slice".into()),
    };
    let subjects = input.subjects(sources, documents)?;
    let document_counts = input.document_counts(&subjects, sources);
    println!(
        "slice {slice}: {} structural dependency subjects, {} source documents",
        subjects.len(),
        document_counts.len()
    );
    let output = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--output=")
                .map(|p| p.replace("{slice}", slice))
        })
        .unwrap_or_else(|| {
            format!("verification/generated/overnight-convergence/slice-{slice}.json")
        });
    let path = root.join(output);
    if path.exists() {
        return Err("Output exists; choose a fresh generated path".into());
    }
    std::fs::create_dir_all(path.parent().ok_or("output parent")?)?;
    if std::env::args().any(|a| a == "--inventory") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(
                &json!({"slice":slice, "subjects":subjects.len(), "documents":document_counts, "reference_refinement":input.refinement}),
            )?,
        )?;
        return Ok(());
    }
    let closure = close_result_structure(
        &input.snapshot,
        PublicationClosureOptions {
            initial_subjects: Some(subjects.clone()),
            ..Default::default()
        },
        |overlay| input.context(overlay),
        |round, done, total, records| {
            if done % 256 == 0 || done == total {
                println!(
                    "slice {slice} round {round}: {done}/{total}, {records} proposed Elements"
                );
            }
        },
        |stage| {
            println!(
                "slice {slice} round {}: {} new Elements, {} occurrences; {:?}",
                stage.stage, stage.added_elements, stage.added_occurrences, stage.completeness
            )
        },
    )?;
    let context = input.context(&closure.overlay)?;
    let audit_subjects: BTreeSet<_> = subjects
        .iter()
        .copied()
        .chain(
            closure
                .overlay
                .model()
                .elements()
                .filter(|r| matches!(r.origin(), Origin::Derived(_)))
                .map(|r| r.id()),
        )
        .collect();
    let mut counts = std::collections::BTreeMap::<PublicationFamily, usize>::new();
    let mut failures = Vec::new();
    for batch in audit_subjects
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .chunks(32)
    {
        let audit =
            KerMlQueries::new(context.fork()).audit_publication_capabilities(batch.iter().copied());
        for (family, count) in audit.checked_items {
            *counts.entry(family).or_default() += count;
        }
        for (family, diagnostics) in audit.failures {
            failures.extend(diagnostics.into_iter().map(|d| json!({"family":format!("{family:?}"), "subject":d.subject.to_string(), "code":d.code, "message":d.message})));
        }
    }
    let model = closure.overlay.model();
    let references: Vec<_> = input
        .draft
        .references()
        .iter()
        .filter(|r| subjects.contains(&r.relationship))
        .collect();
    let mut reference_failures = Vec::new();
    for batch in references.chunks(32) {
        let q = KerMlStatusQueries::new(context.fork());
        for reference in batch {
            let answer = q.lookup_relationship_target(
                reference.relationship,
                reference.property,
                &reference.name,
            );
            let stored: Vec<_> = model
                .navigation_slot(reference.relationship, reference.property)
                .into_iter()
                .flat_map(|s| s.value().values())
                .filter_map(|v| match v {
                    Value::Reference(id) => Some(*id),
                    _ => None,
                })
                .collect();
            let valid = answer.value.first().is_some_and(|target| {
                let target = if reference.membership_target {
                    target.membership
                } else {
                    target.element
                };
                stored == [target]
                    && model.element(target).is_some_and(|r| {
                        model
                            .registry()
                            .is_subtype(r.metaclass(), reference.expected)
                            .unwrap_or(false)
                    })
            });
            if answer.completeness != Completeness::Complete || answer.value.len() != 1 || !valid {
                reference_failures.push(json!({"relationship":reference.relationship.to_string(), "completeness":format!("{:?}",answer.completeness), "candidates":answer.value.len(), "stored_endpoint_valid":valid}));
            }
        }
    }
    let success = closure.converged
        && closure.completeness == Completeness::Complete
        && failures.is_empty()
        && reference_failures.is_empty();
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "format":"agentique-publication-slice/1", "slice":slice, "seed_documents":documents,
            "reference_refinement":input.refinement,
            "scope":"Scoped structural dependency closure; complete declared namespace environment retained",
            "documents":document_counts, "subjects":subjects.len(), "audited_subjects":audit_subjects.len(),
            "source_content_set":sources.content_set_id(), "converged":closure.converged,
            "producer_completeness":format!("{:?}",closure.completeness), "passed":success,
            "counters":publication_metrics::counters(&closure.counters), "semantic_digest":context.id().model_digest,
            "capabilities":counts.into_iter().map(|(family, count)|json!({"family":format!("{family:?}"), "checked_items":count})).collect::<Vec<_>>(),
            "capability_failures":failures, "mandatory_references_checked":references.len(), "reference_failures":reference_failures,
            "validated_standard_roles":input.identity.standard_bindings.as_ref().unwrap().iter().count(),
            "stages":closure.stages.iter().map(|s|json!({"round":s.stage, "added_elements":s.added_elements, "added_occurrences":s.added_occurrences, "diagnostics":s.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        }))?,
    )?;
    if !success {
        return Err(format!("slice {slice} has incomplete publication obligations").into());
    }
    Ok(())
}
