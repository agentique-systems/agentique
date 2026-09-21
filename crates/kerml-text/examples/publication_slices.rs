//! Independent real-corpus preflights. Scoped closure never seals a publication.
use agq_kerml_semantics::*;
use agq_kernel::{provenance::Origin, value::Value};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeSet, io::Write, path::Path};
#[path = "support/publication_input.rs"]
mod publication_input;
use publication_input::{PublicationInput, PublicationScopeBoundary};
#[path = "support/publication_metrics.rs"]
mod publication_metrics;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let slice = std::env::args()
        .find_map(|a| a.strip_prefix("--slice=").map(str::to_owned))
        .ok_or("--slice=A|B|C|D|E|all (or a comma-separated selection) is required")?;
    let slices: Vec<_> = if slice == "all" {
        vec!["A", "B", "C", "D", "E"]
    } else {
        slice.split(',').collect()
    };
    if slices
        .iter()
        .any(|s| !["A", "B", "C", "D", "E"].contains(s))
        || slices.iter().copied().collect::<BTreeSet<_>>().len() != slices.len()
    {
        return Err("Select distinct slice labels A through E".into());
    }
    let fail_fast = std::env::args().any(|a| a == "--fail-fast");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let input = PublicationInput::load(&sources)?;
    let mut failed = false;
    for slice in slices {
        if let Err(error) = run_slice(&root, &sources, &input, slice) {
            eprintln!("slice {slice}: {error}");
            failed = true;
            if fail_fast {
                break;
            }
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
    let mut completed_stages = Vec::new();
    // Preserve producer diagnostics even when the workflow watchdog terminates
    // a later frontier or audit. These observations are generated, not accepted
    // publication evidence.
    let mut stage_log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.with_extension("stages.jsonl"))?;
    let mut stage_log_error = None;
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
                "slice {slice} round {} ({:?}): {} new Elements, {} occurrences; {:?}",
                stage.stage,
                stage.stratum,
                stage.added_elements,
                stage.added_occurrences,
                stage.completeness
            );
            let observation = json!({
                "round":stage.stage, "stratum":format!("{:?}",stage.stratum), "added_elements":stage.added_elements,
                "added_occurrences":stage.added_occurrences,
                "completeness":format!("{:?}",stage.completeness),
                "counters":publication_metrics::counters(&stage.counters),
                "diagnostics":stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject.to_string(),"message":d.message})).collect::<Vec<_>>(),
            });
            if let Err(error) =
                writeln!(stage_log, "{observation}").and_then(|()| stage_log.sync_data())
            {
                eprintln!("slice {slice}: cannot persist stage diagnostics: {error}");
                stage_log_error = Some(error);
            }
            completed_stages.push(observation);
        },
    );
    if let Some(error) = stage_log_error {
        return Err(error.into());
    }
    let closure = match closure {
        Ok(closure) => closure,
        Err(error) => {
            std::fs::write(
                &path,
                serde_json::to_vec_pretty(&json!({
                    "format":"agentique-publication-slice/1", "slice":slice,
                    "seed_documents":documents, "documents":document_counts,
                    "subjects":subjects.len(), "source_content_set":sources.content_set_id(),
                    "reference_refinement":input.refinement, "passed":false,
                    "failure_phase":"producer_closure", "failure":error.to_string(),
                    "stages":completed_stages,
                }))?,
            )?;
            return Err(error.into());
        }
    };
    if !closure.converged || closure.completeness != Completeness::Complete {
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "format":"agentique-publication-slice/1", "slice":slice,
                "seed_documents":documents, "documents":document_counts,
                "subjects":subjects.len(), "source_content_set":sources.content_set_id(),
                "reference_refinement":input.refinement, "passed":false,
                "failure_phase":"producer_closure", "converged":closure.converged,
                "producer_completeness":format!("{:?}",closure.completeness),
                "counters":publication_metrics::counters(&closure.counters),
                "capability_audit":"not_run", "mandatory_reference_audit":"not_run",
                "stages":completed_stages,
            }))?,
        )?;
        return Err(
            format!("slice {slice} producer closure is incomplete; audits deferred").into(),
        );
    }
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
    let model = closure.overlay.model();
    let mut scope_boundary = PublicationScopeBoundary::from_graph(model, &audit_subjects)?;
    scope_boundary.include_invalidation(model, &audit_subjects, &closure.producer_reads);
    for (index, batch) in audit_subjects
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .chunks(32)
        .enumerate()
    {
        let audit =
            KerMlQueries::new(context.fork()).audit_publication_capabilities(batch.iter().copied());
        scope_boundary.include_invalidation(model, &audit_subjects, &audit.read_dependencies);
        for (family, count) in audit.checked_items {
            *counts.entry(family).or_default() += count;
        }
        for (family, diagnostics) in audit.failures {
            failures.extend(diagnostics.into_iter().map(|d| json!({"family":format!("{family:?}"), "subject":d.subject.to_string(), "code":d.code, "message":d.message})));
        }
        let done = ((index + 1) * 32).min(audit_subjects.len());
        if done % 512 == 0 || done == audit_subjects.len() {
            println!(
                "slice {slice} capabilities: {done}/{}, {} findings",
                audit_subjects.len(),
                failures.len()
            );
        }
    }
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
            let answer = q.lookup_relationship_target_with_reads(
                reference.relationship,
                reference.property,
                &reference.name,
            );
            scope_boundary.include_reads(model, &audit_subjects, &answer.reads);
            let answer = answer.outcome;
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
        && scope_boundary.is_complete()
        && failures.is_empty()
        && reference_failures.is_empty();
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "format":"agentique-publication-slice/1", "slice":slice, "seed_documents":documents,
            "reference_refinement":input.refinement,
            "scope":"Scoped structural and semantic-owner closure; graph, producer, capability and reference read boundaries audited",
            "scope_boundary":{"complete":scope_boundary.is_complete(),
                "missing_subject_count":scope_boundary.missing_subjects.len(),
                "missing_subjects":scope_boundary.missing_subjects.iter().take(32).map(ToString::to_string).collect::<Vec<_>>(),
                "unbounded_reads":scope_boundary.unbounded_reads},
            "documents":document_counts, "subjects":subjects.len(), "audited_subjects":audit_subjects.len(),
            "source_content_set":sources.content_set_id(), "converged":closure.converged,
            "producer_completeness":format!("{:?}",closure.completeness), "passed":success,
            "counters":publication_metrics::counters(&closure.counters), "semantic_digest":context.id().model_digest,
            "capabilities":counts.into_iter().map(|(family, count)|json!({"family":format!("{family:?}"), "checked_items":count})).collect::<Vec<_>>(),
            "capability_failures":failures, "mandatory_references_checked":references.len(), "reference_failures":reference_failures,
            "validated_standard_roles":input.identity.standard_bindings.as_ref().unwrap().iter().count(),
            "stages":completed_stages,
        }))?,
    )?;
    if !success {
        return Err(format!("slice {slice} has incomplete publication obligations").into());
    }
    Ok(())
}
