//! Independent real-corpus preflights. Scoped closure never seals a publication.
use agq_kerml::properties as p;
use agq_kerml_semantics::*;
use agq_kerml_text::library::LibraryDraft;
use agq_kernel::{
    ElementId, ModelView,
    provenance::{FactKey, Origin},
    value::Value,
};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeSet, io::Write, path::Path, time::Instant};
#[path = "support/publication_input.rs"]
mod publication_input;
use publication_input::{
    PublicationInput, PublicationScopeBoundary, SLICES, slice_documents, validate_all_documents,
};
#[path = "support/multiplicity_inventory.rs"]
mod multiplicity_inventory;
#[path = "support/publication_metrics.rs"]
mod publication_metrics;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let slice = std::env::args()
        .find_map(|a| a.strip_prefix("--slice=").map(str::to_owned))
        .ok_or("--slice=A|B|C|D|E|all (or a comma-separated selection) is required")?;
    let slices: Vec<_> = if slice == "all" {
        SLICES.to_vec()
    } else {
        slice.split(',').collect()
    };
    if slices.iter().any(|s| !SLICES.contains(s))
        || slices.iter().copied().collect::<BTreeSet<_>>().len() != slices.len()
    {
        return Err("Select distinct slice labels A through E".into());
    }
    let preparation_start = Instant::now();
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    validate_all_documents(&sources)?;
    let input = PublicationInput::load(&sources)?;
    let preparation_seconds = preparation_start.elapsed().as_secs_f64();
    println!("Shared publication preparation: {preparation_seconds:.3}s");
    for slice in slices {
        if let Err(error) = run_slice(&root, &sources, &input, slice, preparation_seconds) {
            eprintln!("slice {slice}: {error}");
            return Err(error);
        }
    }
    Ok(())
}

fn run_slice(
    root: &Path,
    sources: &VerifiedLibrarySet,
    input: &PublicationInput,
    slice: &str,
    preparation_seconds: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let slice_start = Instant::now();
    let documents = slice_documents(slice).ok_or("unknown slice")?;
    let mut subjects = input.subjects(sources, documents)?;
    if slice == "C" {
        let array = input.fixture_subject(sources, &["CollectionFunctions", "array#"])?;
        if !subjects.contains(&array) {
            subjects = input.expanded_subjects(&subjects, &BTreeSet::from([array]), usize::MAX)?;
        }
    }
    let maximum_subjects = subjects.len().saturating_mul(2);
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
            format!("verification/generated/kerml-v9-publication/slice-{slice}.json")
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
    // Preserve producer diagnostics even when the workflow watchdog terminates
    // a later frontier or audit. These observations are generated, not accepted
    // publication evidence.
    let mut stage_log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.with_extension("stages.jsonl"))?;
    let mut scope_log = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.with_extension("scopes.jsonl"))?;
    let mut scope_attempt = 0usize;
    loop {
        scope_attempt += 1;
        let document_counts = input.document_counts(&subjects, sources);
        println!(
            "slice {slice} scope {scope_attempt}: {} declared subjects",
            subjects.len()
        );
        let mut completed_stages = Vec::new();
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
                    "scope_attempt":scope_attempt, "round":stage.stage, "stratum":format!("{:?}",stage.stratum), "added_elements":stage.added_elements,
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
                        "format":"agentique-publication-slice/2", "slice":slice,
                        "preparation_seconds":preparation_seconds, "slice_seconds":slice_start.elapsed().as_secs_f64(), "scope_attempts":scope_attempt,
                        "profile":input.identity.baseline_profile_id,
                        "input_identity":{"profile":input.identity.baseline_profile_id, "rule_set":input.identity.rule_set_version,
                            "source_content_set":sources.content_set_id(), "descriptor_digest":input.identity.descriptor_digest,
                            "declared_graph_digest":input.identity.model_digest},
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
        let graph_missing = scope_boundary.missing_subjects.len();
        scope_boundary.include_invalidation(model, &audit_subjects, &closure.producer_reads);
        let producer_missing = scope_boundary.missing_subjects.len();
        if !scope_boundary.is_complete() {
            subjects = expand_scope(
                input,
                sources,
                &subjects,
                &scope_boundary,
                maximum_subjects,
                json!({"scope_attempt":scope_attempt,"phase":"graph_and_producers","graph_missing":graph_missing,"producer_missing":producer_missing,"elapsed_seconds":slice_start.elapsed().as_secs_f64()}),
                &mut scope_log,
            )?;
            continue;
        }
        if !closure.converged || closure.completeness != Completeness::Complete {
            let context = input.context(&closure.overlay)?;
            std::fs::write(
                &path,
                serde_json::to_vec_pretty(&json!({
                    "format":"agentique-publication-slice/2", "slice":slice,
                        "preparation_seconds":preparation_seconds, "slice_seconds":slice_start.elapsed().as_secs_f64(), "scope_attempts":scope_attempt,
                        "profile":input.identity.baseline_profile_id,
                        "input_identity":{"profile":input.identity.baseline_profile_id, "rule_set":input.identity.rule_set_version,
                            "source_content_set":sources.content_set_id(), "descriptor_digest":input.identity.descriptor_digest,
                            "declared_graph_digest":input.identity.model_digest},
                    "seed_documents":documents, "documents":document_counts,
                    "subjects":subjects.len(), "source_content_set":sources.content_set_id(),
                    "reference_refinement":input.refinement, "passed":false,
                    "failure_phase":"producer_closure", "converged":closure.converged,
                    "producer_completeness":format!("{:?}",closure.completeness),
                    "semantic_digest":context.id().model_digest,
                    "counters":publication_metrics::counters(&closure.counters),
                    "capability_audit":"not_run", "mandatory_reference_audit":"not_run",
                    "stages":completed_stages,
                }))?,
            )?;
            return Err(
                format!("slice {slice} producer closure is incomplete; audits deferred").into(),
            );
        }

        let mut capability_coarse_reads = 0;
        let mut capability_provider_reads = 0;
        for (index, batch) in audit_subjects
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .chunks(32)
            .enumerate()
        {
            let audit = KerMlQueries::new(context.fork())
                .audit_publication_capabilities(batch.iter().copied());
            capability_coarse_reads += audit.read_dependencies.bounded_elements().len();
            capability_provider_reads += audit.provider_reads.bounded_elements().len();
            scope_boundary.include_provider_reads(model, &audit_subjects, &audit.provider_reads);
            for (family, count) in audit.checked_items {
                *counts.entry(family).or_default() += count;
            }
            for (family, diagnostics) in audit.failures {
                for diagnostic in diagnostics {
                    let finding = json!({"family":format!("{family:?}"), "subject":diagnostic.subject.to_string(),
                    "code":diagnostic.code, "message":diagnostic.message,
                    "subject_detail":subject_detail(model, &input.draft, sources, diagnostic.subject)});
                    if failures.len() < 10 {
                        println!("slice {slice} capability finding: {finding}");
                    }
                    failures.push(finding);
                }
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
        let capability_missing = scope_boundary.missing_subjects.len();
        let references: Vec<_> = input
            .draft
            .references()
            .iter()
            .filter(|r| subjects.contains(&r.relationship))
            .collect();
        let mut reference_failures = Vec::new();
        let mut reference_coarse_reads = 0;
        let mut reference_provider_reads = 0;
        for batch in references.chunks(32) {
            let q = KerMlStatusQueries::new(context.fork());
            for reference in batch {
                let answer = q.lookup_relationship_target_with_reads(
                    reference.relationship,
                    reference.property,
                    &reference.name,
                );
                let providers = answer.reads.publication_provider_reads(model);
                reference_provider_reads += providers.bounded_elements().len();
                scope_boundary.include_provider_reads(model, &audit_subjects, &providers);
                reference_coarse_reads += answer.reads.into_invalidation().bounded_elements().len();
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
                if answer.completeness != Completeness::Complete
                    || answer.value.len() != 1
                    || !valid
                {
                    let finding = json!({"relationship":reference.relationship.to_string(), "name":reference.name.segments,
                    "completeness":format!("{:?}",answer.completeness), "candidates":answer.value.len(), "stored_endpoint_valid":valid,
                    "subject_detail":subject_detail(model, &input.draft, sources, reference.relationship)});
                    if reference_failures.len() < 10 {
                        println!("slice {slice} reference finding: {finding}");
                    }
                    reference_failures.push(finding);
                }
            }
        }
        let reference_missing = scope_boundary.missing_subjects.len();
        let provider_read_counts = json!({
            "counting":"Distinct read subjects per capability batch or reference query, summed across observations",
            "capabilities":{"coarse_subject_occurrences":capability_coarse_reads,"provider_subject_occurrences":capability_provider_reads},
            "references":{"coarse_subject_occurrences":reference_coarse_reads,"provider_subject_occurrences":reference_provider_reads},
        });
        if !scope_boundary.is_complete() {
            subjects = expand_scope(
                input,
                sources,
                &subjects,
                &scope_boundary,
                maximum_subjects,
                json!({"scope_attempt":scope_attempt,"phase":"capabilities_and_references","graph_missing":graph_missing,"producer_missing":producer_missing,
                "capability_missing":capability_missing,"reference_missing":reference_missing,"elapsed_seconds":slice_start.elapsed().as_secs_f64(),
                "capability_findings":failures,"reference_findings":reference_failures,"provider_read_counts":provider_read_counts}),
                &mut scope_log,
            )?;
            continue;
        }
        let mut bounds = multiplicity_inventory::collect(
            model,
            &KerMlQueries::new(context.fork()),
            input.draft.source_map(),
            sources,
            Some(&subjects),
            true,
        )?;
        bounds["historical_population"] =
            multiplicity_inventory::historical_population(&bounds, root)?;
        let success = bounds["complete"] == true
            && closure.converged
            && closure.completeness == Completeness::Complete
            && scope_boundary.is_complete()
            && failures.is_empty()
            && reference_failures.is_empty();
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&json!({
                "format":"agentique-publication-slice/2", "slice":slice,
                        "preparation_seconds":preparation_seconds, "slice_seconds":slice_start.elapsed().as_secs_f64(), "scope_attempts":scope_attempt,
                        "profile":input.identity.baseline_profile_id,
                        "input_identity":{"profile":input.identity.baseline_profile_id, "rule_set":input.identity.rule_set_version,
                            "source_content_set":sources.content_set_id(), "descriptor_digest":input.identity.descriptor_digest,
                            "declared_graph_digest":input.identity.model_digest}, "seed_documents":documents,
                "reference_refinement":input.refinement,
                "scope":"Scoped structural and semantic-owner closure; graph, producer, capability and reference read boundaries audited",
                "scope_boundary":{"complete":scope_boundary.is_complete(),
                    "missing_subject_count":scope_boundary.missing_subjects.len(),
                    "missing_subjects":scope_boundary.missing_subjects.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "missing_subject_details":scope_boundary.missing_subjects.iter().take(32).map(|&subject|subject_detail(model,&input.draft,sources,subject)).collect::<Vec<_>>(),
                    "unbounded_reads":scope_boundary.unbounded_reads,"provider_read_counts":provider_read_counts},
                "documents":document_counts, "subjects":subjects.len(), "audited_subjects":audit_subjects.len(),
                "source_content_set":sources.content_set_id(), "converged":closure.converged,
                "producer_completeness":format!("{:?}",closure.completeness), "passed":success,
                "counters":publication_metrics::counters(&closure.counters), "semantic_digest":context.id().model_digest,
                "capabilities":counts.into_iter().map(|(family, count)|json!({"family":format!("{family:?}"), "checked_items":count})).collect::<Vec<_>>(),
                "capability_failures":failures, "mandatory_references_checked":references.len(), "reference_failures":reference_failures,
                "multiplicity_bounds":bounds,
                "validated_standard_roles":input.identity.standard_bindings.as_ref().unwrap().iter().count(),
                "resource_stop":false,
                "stages":completed_stages,
            }))?,
        )?;
        println!(
            "slice {slice}: passed={success}, elapsed={:.3}s (shared preparation {preparation_seconds:.3}s)",
            slice_start.elapsed().as_secs_f64()
        );
        if !success {
            return Err(format!("slice {slice} has incomplete publication obligations").into());
        }
        return Ok(());
    }
}

fn expand_scope(
    input: &PublicationInput,
    sources: &VerifiedLibrarySet,
    subjects: &BTreeSet<ElementId>,
    boundary: &PublicationScopeBoundary,
    maximum: usize,
    mut observation: serde_json::Value,
    log: &mut std::fs::File,
) -> Result<BTreeSet<ElementId>, Box<dyn std::error::Error>> {
    let expanded = input.expanded_subjects(subjects, &boundary.missing_subjects, maximum);
    observation["previous_subject_count"] = json!(subjects.len());
    observation["missing_subjects"] = json!(
        boundary
            .missing_subjects
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    let details: Vec<_> = boundary
        .missing_subjects
        .iter()
        .take(32)
        .map(|&subject| subject_detail(input.snapshot.model(), &input.draft, sources, subject))
        .collect();
    for detail in details.iter().take(10) {
        println!("scope boundary: {detail}");
    }
    observation["missing_subject_details"] = json!(details);
    observation["missing_subject_count"] = json!(boundary.missing_subjects.len());
    observation["next_subject_count"] = json!(expanded.as_ref().ok().map(BTreeSet::len));
    observation["development_limit"] = json!(maximum);
    observation["failure"] = json!(expanded.as_ref().err());
    writeln!(log, "{observation}")?;
    log.sync_data()?;
    println!(
        "scope expansion: {} missing providers, {} -> {:?} subjects (development limit {maximum})",
        boundary.missing_subjects.len(),
        subjects.len(),
        expanded.as_ref().ok().map(BTreeSet::len)
    );
    Ok(expanded?)
}

/// Bounded source/provenance details for workflow diagnosis; never a semantic decision.
fn subject_detail(
    model: &ModelView,
    draft: &LibraryDraft,
    sources: &VerifiedLibrarySet,
    subject: ElementId,
) -> serde_json::Value {
    let Some(record) = model.element(subject) else {
        return json!({"subject":subject.to_string(),"missing_element":true});
    };
    let name = model
        .navigation_slot(subject, p::ELEMENT_DECLARED_NAME)
        .and_then(|slot| {
            slot.value().values().find_map(|value| match value {
                Value::String(name) => Some(name.as_str()),
                _ => None,
            })
        });
    let source = draft.source_map().get(&FactKey::Element(subject)).map(|origin| {
        let document = sources.documents().find(|document|document.document()==origin.document);
        json!({"document":document.as_ref().map(|document|document.path()),
            "range":[origin.range.start(),origin.range.end()],
            "excerpt":document.map(|document|document.source()[origin.range.start() as usize..origin.range.end() as usize].chars().take(240).collect::<String>())})
    });
    json!({"subject":subject.to_string(),"metaclass":model.registry().class(record.metaclass()).ok().map(|class|class.name.as_str()),
    "declared_name":name,"source":source,"derived_rule":match record.origin() {
        Origin::Derived(origin) => Some(origin.rule.to_string()), _ => None,
    }})
}
