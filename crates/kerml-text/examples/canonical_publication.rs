//! Release-only complete-overlay gate. Run after focused publication fixtures.
//! Conformance output is separate and never changes publication acceptance.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::*;
use agq_kerml_text::library::{CanonicalKermlStandardLibraries, CanonicalPublicationError};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeSet, io::Write, path::Path, sync::Arc};
#[path = "support/authored_publication.rs"]
mod authored_publication;
#[path = "support/multiplicity_inventory.rs"]
mod multiplicity_inventory;
#[path = "support/publication_authority.rs"]
mod publication_authority;
#[path = "support/publication_metrics.rs"]
mod publication_metrics;
#[path = "support/publication_output.rs"]
mod publication_output;
use publication_output::ReservedFile;
#[path = "support/publication_preflight.rs"]
mod publication_preflight;
#[path = "support/publication_refinement.rs"]
mod publication_refinement;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        return Err("Use --release after passing the focused publication gate".into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output is required")?;
    let path = root.join(output);
    let cache_path = std::env::args()
        .find_map(|a| a.strip_prefix("--cache-output=").map(|p| root.join(p)))
        .unwrap_or_else(|| path.with_extension("publication.zip"));
    let generated_receipt = cache_path.with_extension("receipt.json");
    let manifest_path = root.join("standards/kerml-standard-bindings.json");
    let receipt_path = root.join("standards/kerml-accepted-publication.json");
    let write_bindings = std::env::args().any(|a| a == "--write-bindings");
    let check_bindings = write_bindings || std::env::args().any(|a| a == "--check-bindings");
    let slice_evidence = std::env::args()
        .find_map(|a| a.strip_prefix("--slice-evidence=").map(str::to_owned))
        .ok_or(
            "--slice-evidence=<directory containing slice-A.json through slice-E.json> is required",
        )?;
    // Reserve every destination while failures are still cheap. Empty reservations
    // clean themselves up; a stopped/failed populated artifact remains inspectable.
    let report_output = ReservedFile::new(&path)?;
    let stage_output = ReservedFile::new(path.with_extension("stages.jsonl"))?;
    let cache_output = ReservedFile::new(&cache_path)?;
    let receipt_output = ReservedFile::new(&generated_receipt)?;
    let staged_bindings = if write_bindings {
        Some((
            ReservedFile::replacement(&manifest_path)?,
            ReservedFile::replacement(&receipt_path)?,
        ))
    } else {
        None
    };
    let conformance_output = std::env::args()
        .find_map(|a| {
            a.strip_prefix("--conformance-output=")
                .map(|p| root.join(p))
        })
        .map(ReservedFile::new)
        .transpose()?;
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let authority = publication_authority::conflicts(&root)?;
    let blockers = authority
        .values()
        .filter(|&&impact| impact == AuthorityImpact::PublicationBlockingAuthorityConflict)
        .count();
    if blockers != 0 {
        return Err("Publication authority gate failed".into());
    }
    let (draft, refinement) = publication_refinement::prepare(&sources)?;
    let preflight_references = publication_preflight::check(
        &root.join(slice_evidence),
        &publication_preflight::identity(
            draft.queries(&sources)?.context(),
            sources.content_set_id(),
        ),
    )?;
    let preflight_population = multiplicity_inventory::historical_population(
        &json!({"reference_expression_ids":preflight_references}),
        &root,
    )?;
    if preflight_population["complete"] != true {
        return Err(format!(
            "Slices A-E did not complete every retained symbolic multiplicity reference: {}",
            preflight_population["missing"]
        )
        .into());
    }
    let construction_obligations = draft.candidate().obligations().len();
    let mut stages = vec![];
    let mut stage_log = stage_output.file();
    let mut stage_log_error = None;
    let closure = CanonicalKermlStandardLibraries::publish(
        draft,
        &sources,
        16,
        |stage, completed, total, records| {
            if completed % 512 == 0 || completed == total {
                println!(
                    "producer stage {stage}: {completed}/{total} subjects, {records} proposed records"
                );
            }
        },
        |stage| {
            println!(
                "stage {} ({:?}): {} new Elements, {} occurrences, {:?}",
                stage.stage,
                stage.stratum,
                stage.added_elements,
                stage.added_occurrences,
                stage.completeness
            );
            stages.push(json!({"stage":stage.stage,"stratum":format!("{:?}",stage.stratum),"subjects":stage.input_elements,"added_elements":stage.added_elements,"added_occurrences":stage.added_occurrences,"completeness":format!("{:?}", stage.completeness),"counters":publication_metrics::counters(&stage.counters),"diagnostics":stage.diagnostics.iter().map(|d|json!({"subject":d.subject.to_string(),"code":d.code,"message":d.message})).collect::<Vec<_>>()}));
            if let Err(error) = writeln!(stage_log, "{}", stages.last().expect("current stage"))
                .and_then(|()| stage_log.sync_data())
            {
                eprintln!("cannot persist publication stage diagnostics: {error}");
                stage_log_error = Some(error);
            }
        },
        |done, failures| {
            if done % 512 == 0 {
                println!("mandatory references: {done} checked, {failures} failures");
            }
        },
        |done, total, failures| {
            if done % 512 == 0 || done == total {
                println!(
                    "publication capability audit: {done}/{total} subjects, {failures} findings"
                );
            }
        },
    );
    let publication = match closure {
        Ok(complete) => complete,
        Err(error) => {
            report_output.write_json(
                    &json!({"format":"agq-kerml-complete-publication/1","profile":BaselineProfile::OPERATIONAL_V9.id(),"input_set":sources.content_set_id(),"overlay":"Incomplete","stage_log_error":stage_log_error.as_ref().map(ToString::to_string),"kernel_construction_obligations":construction_obligations,"publication_blocking_authority_conflicts":blockers,"stages":stages,"failure":format!("{error:?}"),"reference_failures": match &error { CanonicalPublicationError::References(failures) => failures.iter().map(|f| json!({"relationship":f.relationship.to_string(),"completeness":format!("{:?}",f.completeness),"candidates":f.candidates,"canonical_endpoint_valid":f.canonical_endpoint_valid})).collect::<Vec<_>>(), _ => vec![] }}),
            )?;
            return Err(error.into());
        }
    };
    let publication = Arc::new(publication);
    // Preserve the accepted core result before binding I/O or authored regressions.
    // A later tool failure must remain distinguishable from a failed publication.
    let initial_report_error = report_output.write_json(&json!({
            "format":"agq-kerml-complete-publication/1", "profile":publication.profile().id(),
            "input_set":sources.content_set_id(), "overlay":"CompletePublicationOverlay",
            "accepted_overlay_and_references":true, "canonical_facade_accepted":true,
            "publication_blocking_authority_conflicts":blockers,
            "mandatory_references":{"total":publication.mandatory_reference_count(),"unresolved":0,"incomplete":0,"ambiguous":0,"invalid":0,"stored_endpoint_mismatch":0},
            "semantic_digest":publication.semantic_digest(), "stages":stages,
            "capabilities":publication.complete_overlay().checked_items().iter().map(|(family,count)|json!({"family":format!("{family:?}"),"checked_items":count,"status":"Complete"})).collect::<Vec<_>>(),
            "accepted_binding_manifest":{"status":"pending"}, "authored_consumption_verified":false,
            "post_publication_checks":"pending",
            "stage_log_error":stage_log_error.as_ref().map(ToString::to_string)
        })).err().map(|error| error.to_string());
    if let Some(error) = &initial_report_error {
        eprintln!(
            "cannot persist initial accepted report; still attempting durable cache: {error}"
        );
    }
    // Reuse the accepted in-memory publication; never launch a second corpus
    // closure merely to regenerate or stale-check its binding manifest.
    let accepted_manifest = publication.binding_manifest(&sources)?;
    // Persist the already accepted facade once. ZIP deflation and the kernel
    // codec stream graph/proof/search data without buffering a second corpus.
    let cache_file = cache_output.file();
    println!(
        "accepted publication cache: streaming {}",
        cache_path.display()
    );
    let cache_receipt = publication.write_cache(cache_file, &sources)?;
    cache_file.sync_all()?;
    println!(
        "accepted publication cache: {} compressed bytes",
        cache_file.metadata()?.len()
    );
    // The archive is finalized and durable before either acceptance file changes.
    // A receipt beside the generated cache is evidence only; restoration uses
    // the separately checked-in pair compiled into the next build.
    receipt_output.write_json(&cache_receipt)?;
    if let Some((staged_manifest, staged_receipt)) = staged_bindings {
        staged_manifest.write_json(&accepted_manifest)?;
        staged_receipt.write_json(&cache_receipt)?;
        // A crash between replacements leaves a mismatched pair, rejected closed.
        staged_manifest.replace(&manifest_path)?;
        staged_receipt.replace(&receipt_path)?;
    }
    if check_bindings {
        println!("Accepted binding manifest: checking all standard roles");
        publication.check_binding_manifest(
            &sources,
            &serde_json::from_slice(&std::fs::read(&manifest_path)?)?,
        )?;
    }
    let complete = publication.complete_overlay();
    let model = publication.overlay().model();
    println!("Multiplicity inventory: auditing the accepted publication");
    let mut bounds = multiplicity_inventory::collect(
        model,
        &publication.queries(),
        publication.source_map(),
        &sources,
        None,
        true,
    )?;
    bounds["historical_population"] =
        multiplicity_inventory::historical_population(&bounds, &root)?;
    let bound_population_complete =
        bounds["complete"] == true && bounds["historical_population"]["complete"] == true;
    println!("Multiplicity inventory: complete={bound_population_complete}");
    let authored = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        authored_publication::verify(publication.clone())
    }))
    .unwrap_or_else(|_| {
        Err("Authored publication regression assertion failed; see raw output".into())
    });

    report_output.write_json(&json!({
            "format":"agq-kerml-complete-publication/1", "profile":BaselineProfile::OPERATIONAL_V9.id(),
            "input_set":sources.content_set_id(), "overlay":"CompletePublicationOverlay", "accepted_overlay_and_references":true,
            "reference_refinement":refinement,
            "post_publication_checks":"finished",
            "stage_log_error":stage_log_error.as_ref().map(ToString::to_string),
            "initial_report_error":initial_report_error,
            "stage_evidence_preserved_in_report":true,
            "preflight_symbolic_population":preflight_population,
            "accepted_binding_manifest":{"format":accepted_manifest["format"],"roles":accepted_manifest["entries"].as_array().map(Vec::len),
                "written":write_bindings,"stale_check_passed":check_bindings,"path":"standards/kerml-standard-bindings.json"},
            "publication_cache":{"path":cache_path,"receipt":receipt_path,"trusted_receipt_written":write_bindings,
                "archive_bytes":std::fs::metadata(&cache_path)?.len(),"graph_bytes":cache_receipt["complete_overlay"]["graph_bytes"]},
            "multiplicity_bounds":bounds,
            "kernel_construction_obligations":0,"publication_blocking_authority_conflicts":blockers,
            "mandatory_references":{"total":publication.mandatory_reference_count(),"unresolved":0,"incomplete":0,"ambiguous":0,"invalid":0,"stored_endpoint_mismatch":0},
            "reference_findings":[],"canonical_facade_accepted":true,"authored_consumption_verified":authored.is_ok(),"symbolic_multiplicity_audit_passed":bound_population_complete,"authored_failure":authored.as_ref().err().map(|e| format!("{e}")),"stages":stages,"source_elements":publication.snapshot().model().len(),"expanded_elements":model.len(),"derived_facts":complete.overlay().facts().count(),
            "capabilities":complete.checked_items().iter().map(|(family,count)|json!({"family":format!("{family:?}"),"checked_items":count,"status":"Complete"})).collect::<Vec<_>>(),
            "semantic_digest":complete.context().model_digest,
            "counters":publication_metrics::counters(complete.counters()),
        }))?;
    if let Some(output) = conformance_output {
        let coverage: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(
            "verification/kerml-publication-convergence/publication-critical-coverage.json",
        ))?)?;
        let mut report = KerMlConformanceReport {
            context: complete.context().clone(),
            diagnostics: BTreeSet::new(),
            coverage: ConstraintCoverage {
                inventory: coverage["constraints"]
                    .as_array()
                    .ok_or("constraint inventory")?
                    .iter()
                    .map(|r| r["rule"].as_str().unwrap().to_owned())
                    .collect(),
                checked: BTreeSet::new(),
                deferred_by_phase: BTreeSet::new(),
            },
            authority_conflicts: authority,
        };
        let subjects: Vec<_> = model.elements().map(|r| r.id()).collect();
        for (index, batch) in subjects
            .chunks(16)
            .enumerate()
            .filter(|_| std::env::args().any(|a| a == "--validate-conformance"))
        {
            let q = complete.queries();
            for &subject in batch {
                let class = model.element(subject).unwrap().metaclass();
                let mut checks = vec![q.validate_local_structure(subject)];
                if model.registry().is_subtype(class, c::NAMESPACE)? {
                    checks.push(q.validate_namespace_distinguishability(subject));
                }
                if model.registry().is_subtype(class, c::FEATURE)? {
                    checks.push(q.validate_formal_target_constraints(subject));
                }
                for check in checks {
                    report
                        .coverage
                        .checked
                        .extend(check.value.into_iter().map(str::to_owned));
                    report.diagnostics.extend(check.diagnostics);
                }
            }
            if index % 32 == 0 {
                println!("separate conformance batch {index}");
            }
        }
        output.write_json(
                &json!({"format":"agentique-kerml-conformance-report/1","profile":report.context.baseline_profile_id,"scope":"CompletePublicationOverlay","validation_executed":std::env::args().any(|a|a == "--validate-conformance"),"coverage":format!("{:?}",report.coverage.status()),"checked":report.coverage.checked,"authority_conflicts":report.authority_conflicts.iter().map(|(issue,impact)|json!({"issue":issue,"impact":format!("{impact:?}")})).collect::<Vec<_>>(),"diagnostics":report.diagnostics.iter().map(|d|json!({"subject":d.subject.to_string(),"code":d.code,"message":d.message})).collect::<Vec<_>>()}),
        )?;
    }
    authored?;
    if !bound_population_complete {
        return Err(
            "Multiplicity reference population did not pass its complete structural audit".into(),
        );
    }
    println!("KERML CANONICAL LIBRARY PUBLICATION COMPLETE");
    Ok(())
}
