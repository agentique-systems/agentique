//! Exact Systems candidate/closure audit over a restored sealed KerML publication.
use agq_kerml_semantics::{Completeness, QualifiedName};
use agq_kerml_syntax::production::SysmlSyntaxProfile;
use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries, sysml::prepare_systems_library_with_semantic_progress,
};
use agq_kernel::value::Value;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{collections::BTreeMap, io::Write, path::PathBuf, sync::Arc, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let argument = |prefix: &str| {
        std::env::args().find_map(|arg| arg.strip_prefix(prefix).map(|value| root.join(value)))
    };
    let cache =
        argument("--cache=").ok_or("--cache=<accepted KerML publication cache> is required")?;
    let output = argument("--output=").ok_or("--output=<report path> is required")?;
    let started = Instant::now();
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    println!("Systems: restoring sealed KerML dependency");
    let accepted = Arc::new(CanonicalKermlStandardLibraries::restore_cache(
        std::fs::File::open(cache)?,
        &sources,
    )?);
    println!(
        "Systems: dependency restored in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    let candidate = prepare_systems_library_with_semantic_progress(
        &sources,
        accepted.clone(),
        SysmlSyntaxProfile::OperationalV1,
        |round| {
            println!(
                "Systems: references round={} selected={} obligations={} evaluated={} reused={}",
                round.round,
                round.selected_endpoints,
                round.structural_obligations,
                round.references_evaluated,
                round.references_reused
            );
            for (reference, candidates) in &round.withdrawn_endpoints {
                println!("Systems: withdrawn reference={reference:?} candidates={candidates:?}");
                if let Some(diagnostics) = round.withdrawal_diagnostics.get(reference) {
                    for diagnostic in diagnostics {
                        println!("Systems: withdrawal diagnostic={diagnostic:?}");
                    }
                }
            }
        },
        |round, done, total, planned| {
            if done.is_multiple_of(256) || done == total {
                println!(
                    "Systems: producers round={round} evaluated={done}/{total} planned={planned}"
                );
            }
        },
        |stage| {
            println!(
                "Systems: producer frontier={} stratum={:?} added={} completeness={:?} diagnostics={}",
                stage.stage,
                stage.stratum,
                stage.added_elements,
                stage.completeness,
                stage.diagnostics.len()
            )
        },
    )?;
    let draft = candidate.draft();
    let queries = candidate.queries()?;
    let model = queries.model();
    let mut counts = BTreeMap::<&str, usize>::from([
        ("complete", 0),
        ("unresolved", 0),
        ("incomplete", 0),
        ("ambiguous", 0),
        ("invalid", 0),
        ("endpoint_mismatch", 0),
    ]);
    let mut failures = vec![];
    for (index, batch) in draft.references().chunks(32).enumerate() {
        let q = queries.fork().status_queries();
        for reference in batch {
            let answer = q.lookup_relationship_target(
                reference.relationship,
                reference.property,
                &reference.name,
            );
            let targets: Vec<_> = answer
                .value
                .iter()
                .map(|member| {
                    if reference.membership_target {
                        member.membership
                    } else {
                        member.element
                    }
                })
                .collect();
            let stored: Vec<_> = model
                .navigation_slot(reference.relationship, reference.property)
                .into_iter()
                .flat_map(|slot| slot.value().values())
                .filter_map(|value| {
                    if let Value::Reference(id) = value {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();
            let status = match answer.completeness {
                Completeness::Invalid => "invalid",
                Completeness::Incomplete => "incomplete",
                Completeness::Complete if targets.is_empty() => "unresolved",
                Completeness::Complete if targets.len() > 1 => "ambiguous",
                Completeness::Complete if targets != stored => "endpoint_mismatch",
                Completeness::Complete
                    if !model.registry().is_subtype(
                        model
                            .element(targets[0])
                            .expect("lookup target")
                            .metaclass(),
                        reference.expected,
                    )? =>
                {
                    "invalid"
                }
                Completeness::Complete => "complete",
            };
            *counts.get_mut(status).expect("counter") += 1;
            if status != "complete" {
                failures.push(json!({"name":reference.name.segments, "relationship":reference.relationship,"property":reference.property,"document":reference.origin.document,"range":reference.origin.range,"status":status,"targets":targets,"stored":stored,"diagnostics":answer.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>() }));
            }
        }
        println!(
            "Systems: mandatory references audited={} failures={}",
            ((index + 1) * 32).min(draft.references().len()),
            failures.len()
        );
    }
    let authority_paths = [
        "Views::Viewpoint",
        "Views::ViewpointCheck",
        "Views::viewpoints",
        "Views::viewpointChecks",
        "Connections::BinaryConnections",
        "Connections::BinaryConnection",
        "Items::Item::subitem",
        "Items::Item::subitems",
        "Base::dataValues",
        "Attributes::attributes",
    ];
    let authority: Vec<_> = authority_paths.into_iter().map(|path| {
        let answer = queries.lookup_path(draft.roots()[0], &QualifiedName { absolute: true, segments:path.split("::").map(str::to_owned).collect() });
        json!({"path":path,"completeness":format!("{:?}",answer.completeness),"targets":answer.value.iter().map(|m|m.element).collect::<Vec<_>>()})
    }).collect();
    let parsed = candidate.documents().iter().filter(|d| d.parsed).count();
    let constructed = candidate
        .documents()
        .iter()
        .filter(|d| d.construction_gap.is_none())
        .count();
    let obligations = draft.candidate().obligations().len();
    let mut report = json!({
        "format":"agq-sysml-systems-publication-audit/1",
        "sysml_profile":candidate.syntax_profile().id(),
        "accepted_kerml_digest":accepted.semantic_digest(),
        "accepted_kerml_profile":accepted.profile().id(),
        "kerml_producers_replayed":false,
        "systems_documents_parsed":parsed,
        "systems_documents_constructed":constructed,
        "systems_documents_byte_exact":candidate.documents().iter().filter(|d|d.byte_exact).count(),
        "construction_complete":candidate.construction_complete(),
        "kernel_obligations":draft.candidate().obligations().len(),
        "local_elements":model.elements().filter(|r|accepted.overlay().model().element(r.id()).is_none()).count(),
        "mandatory_references":{"total":draft.references().len(),"counts":counts,"failures":failures},
        "authority_targets":authority,
        "documents":candidate.documents().iter().map(|d|json!({"path":d.path,"document":d.document,"sha256":d.source_sha256,"profile":d.profile.id(),"parsed":d.parsed,"byte_exact":d.byte_exact,"recovery_count":d.recovery_count,"production_count":d.production_count,"construction_gap":d.construction_gap})).collect::<Vec<_>>(),
        "construction_producers":candidate.production().map(|production|json!({"final_predicates":production.final_predicates,"completeness":format!("{:?}",production.completeness),"converged":production.converged,"rounds":production.counters.fixed_point_rounds,"subjects_evaluated":production.counters.subjects_evaluated,"derived_elements":production.counters.new_elements_proposed,"diagnostics":production.stages.last().map(|stage|stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>())})), "publication_accepted":false,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
    });
    drop(queries);
    println!("Systems: evaluating immutable publication acceptance");
    match agq_kerml_text::sysml::CanonicalSysmlSystemsLibrary::publish(
        candidate,
        &sources,
        Default::default(),
        |round, done, total, planned| {
            if done.is_multiple_of(256) || done == total {
                println!(
                    "Systems publication: round={round} evaluated={done}/{total} planned={planned}"
                );
            }
        },
        |stage| {
            println!(
                "Systems publication: frontier={} stratum={:?} added={} completeness={:?}",
                stage.stage, stage.stratum, stage.added_elements, stage.completeness
            )
        },
    ) {
        Ok(publication) => {
            report["publication_accepted"] = json!(true);
            report["publication_digest"] = json!(publication.publication_digest());
            report["semantic_digest"] = json!(publication.semantic_digest());
            report["accepted_bindings"] = json!(publication.bindings().targets().len());
            report["publication_gate"] = audit_report(publication.audit());
        }
        Err(agq_kerml_text::sysml::SystemsPublicationError::Rejected(audit)) => {
            report["publication_gate"] = audit_report(&audit);
        }
        Err(error) => {
            report["publication_error"] = json!(error.to_string());
        }
    }
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    std::fs::create_dir_all(output.parent().ok_or("output parent")?)?;
    let mut file = std::fs::File::create(&output)?;
    file.write_all(&serde_json::to_vec_pretty(&report)?)?;
    file.sync_all()?;
    println!(
        "Systems: {parsed}/21 parsed; {constructed}/21 constructed; {} kernel obligations; {:.3}s",
        obligations,
        started.elapsed().as_secs_f64()
    );
    if report["publication_accepted"] != true {
        std::process::exit(1);
    }
    Ok(())
}

fn audit_report(audit: &agq_kerml_text::sysml::SystemsPublicationAudit) -> serde_json::Value {
    json!({
        "checked":audit.checked.iter().map(|(family,count)|(format!("{family:?}"),*count)).collect::<BTreeMap<_,_>>(),
        "mandatory_references":audit.mandatory_references,
        "complete_references":audit.complete_references,
        "findings":audit.findings.iter().map(|finding|format!("{finding:?}")).collect::<Vec<_>>()
    })
}
