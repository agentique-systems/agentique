//! Release-only complete-overlay gate. Run after focused publication fixtures.
//! Conformance output is separate and never changes publication acceptance.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::*;
use agq_kerml_text::library::{CanonicalKermlStandardLibraries, CanonicalPublicationError};
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeSet, path::Path, sync::Arc};
#[path = "support/authored_publication.rs"]
mod authored_publication;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        return Err("Use --release after passing the focused publication gate".into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output is required")?;
    let path = root.join(output);
    if path.exists() {
        return Err("Output exists; use a fresh generated evidence path".into());
    }
    std::fs::create_dir_all(path.parent().ok_or("output parent")?)?;
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let authority: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("verification/kerml-canonical-publication/authority-decisions.json"),
    )?)?;
    let blockers = authority["decisions"]
        .as_array()
        .ok_or("authority decisions")?
        .iter()
        .filter(|a| a["impact"] == "PublicationBlockingAuthorityConflict")
        .count();
    if authority["profile"] != BaselineProfile::OPERATIONAL_V8.id() || blockers != 0 {
        return Err("Publication authority gate failed".into());
    }
    let draft = agq_kerml_text::library::refine_declarations_with_profile(
        &sources,
        BaselineProfile::OPERATIONAL_V8,
        |round, refs, obligations| {
            println!("refinement {round}: {refs} endpoints, {obligations} obligations");
        },
    )?;
    let construction_obligations = draft.candidate().obligations().len();
    let mut stages = vec![];
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
                "stage {}: {} new Elements, {} occurrences, {:?}",
                stage.stage, stage.added_elements, stage.added_occurrences, stage.completeness
            );
            stages.push(json!({"stage":stage.stage,"subjects":stage.input_elements,"added_elements":stage.added_elements,"added_occurrences":stage.added_occurrences,"completeness":format!("{:?}", stage.completeness),"diagnostics":stage.diagnostics.iter().map(|d|json!({"subject":d.subject.to_string(),"code":d.code,"message":d.message})).collect::<Vec<_>>()}));
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
            std::fs::write(
                &path,
                serde_json::to_vec_pretty(
                    &json!({"format":"agq-kerml-complete-publication/1","profile":BaselineProfile::OPERATIONAL_V8.id(),"input_set":sources.content_set_id(),"overlay":"Incomplete","kernel_construction_obligations":construction_obligations,"publication_blocking_authority_conflicts":blockers,"stages":stages,"failure":format!("{error:?}"),"reference_failures": match &error { CanonicalPublicationError::References(failures) => failures.iter().map(|f| json!({"relationship":f.relationship.to_string(),"completeness":format!("{:?}",f.completeness),"candidates":f.candidates,"canonical_endpoint_valid":f.canonical_endpoint_valid})).collect::<Vec<_>>(), _ => vec![] }}),
                )?,
            )?;
            return Err(error.into());
        }
    };
    let publication = Arc::new(publication);
    let complete = publication.complete_overlay();
    let model = publication.overlay().model();
    let authored = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        authored_publication::verify(publication.clone())
    }))
    .unwrap_or_else(|_| {
        Err("Authored publication regression assertion failed; see raw output".into())
    });
    let accepted = authored.is_ok();
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "format":"agq-kerml-complete-publication/1", "profile":BaselineProfile::OPERATIONAL_V8.id(),
            "input_set":sources.content_set_id(), "overlay":"CompletePublicationOverlay", "accepted_overlay_and_references":true,
            "kernel_construction_obligations":0,"publication_blocking_authority_conflicts":blockers,
            "mandatory_references":{"total":publication.mandatory_reference_count(),"unresolved":0,"incomplete":0,"ambiguous":0,"invalid":0},
            "reference_findings":[],"canonical_facade_accepted":true,"authored_consumption_verified":accepted,"authored_failure":authored.as_ref().err().map(|e| format!("{e}")),"stages":stages,"source_elements":publication.snapshot().model().len(),"expanded_elements":model.len(),"derived_facts":complete.overlay().facts().count(),
            "capabilities":complete.checked_items().iter().map(|(family,count)|json!({"family":format!("{family:?}"),"checked_items":count,"status":"Complete"})).collect::<Vec<_>>(),
            "semantic_digest":complete.context().model_digest,
        }))?,
    )?;
    if let Some(output) =
        std::env::args().find_map(|a| a.strip_prefix("--conformance-output=").map(str::to_owned))
    {
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
            authority_conflicts: authority["decisions"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| a["impact"] == "ValidationOnlyAuthorityConflict")
                .map(|a| {
                    (
                        a["issue"].as_str().unwrap().into(),
                        AuthorityImpact::ValidationOnlyAuthorityConflict,
                    )
                })
                .collect(),
        };
        let subjects: Vec<_> = model.elements().map(|r| r.id()).collect();
        for (index, batch) in subjects.chunks(16).enumerate() {
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
        std::fs::write(
            root.join(output),
            serde_json::to_vec_pretty(
                &json!({"format":"agentique-kerml-conformance-report/1","profile":report.context.baseline_profile_id,"scope":"CompletePublicationOverlay","coverage":format!("{:?}",report.coverage.status()),"checked":report.coverage.checked,"authority_conflicts":report.authority_conflicts.iter().map(|(issue,impact)|json!({"issue":issue,"impact":format!("{impact:?}")})).collect::<Vec<_>>(),"diagnostics":report.diagnostics.iter().map(|d|json!({"subject":d.subject.to_string(),"code":d.code,"message":d.message})).collect::<Vec<_>>()}),
            )?,
        )?;
    }
    if !accepted {
        return Err(authored.unwrap_err());
    }
    println!("Complete publication overlay and mandatory reference gates passed");
    Ok(())
}
