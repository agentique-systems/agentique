//! Exact Systems candidate/closure audit over a restored sealed KerML publication.
use agq_kerml_semantics::{Completeness, QualifiedName};
use agq_kerml_syntax::production::SysmlSyntaxProfile;
use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries,
    sysml::{
        prepare_systems_library_slice_with_semantic_progress,
        prepare_systems_library_with_semantic_progress,
    },
};
use agq_kernel::value::Value;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let argument = |prefix: &str| {
        std::env::args().find_map(|arg| arg.strip_prefix(prefix).map(|value| root.join(value)))
    };
    let cache =
        argument("--cache=").ok_or("--cache=<accepted KerML publication cache> is required")?;
    let output = argument("--output=").ok_or("--output=<report path> is required")?;
    std::fs::create_dir_all(output.parent().ok_or("output parent")?)?;
    let mut stages = std::fs::File::create(output.with_extension("stages.jsonl"))?;
    let started = Instant::now();
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let selected =
        std::env::args().find_map(|arg| arg.strip_prefix("--documents=").map(str::to_owned));
    let audit_only = std::env::args().any(|arg| arg == "--audit-only");
    let paths: BTreeSet<String> = if let Some(selected) = &selected {
        if !audit_only {
            return Err(
                "--documents requires --audit-only; a slice cannot accept a publication".into(),
            );
        }
        selected
            .split(',')
            .map(|name| {
                sources
                    .documents()
                    .find(|source| {
                        source.language() == agq_standard_libraries::LibraryLanguage::SysMl
                            && std::path::Path::new(source.path())
                                .file_stem()
                                .and_then(|s| s.to_str())
                                == Some(name)
                    })
                    .map(|source| source.path().to_owned())
                    .ok_or_else(|| format!("unknown Systems document: {name}"))
            })
            .collect::<Result<_, _>>()?
    } else {
        sources
            .documents()
            .filter(|source| source.language() == agq_standard_libraries::LibraryLanguage::SysMl)
            .map(|source| source.path().to_owned())
            .collect()
    };
    println!("Systems: restoring sealed KerML dependency");
    let accepted = Arc::new(CanonicalKermlStandardLibraries::restore_cache(
        std::fs::File::open(cache)?,
        &sources,
    )?);
    println!(
        "Systems: dependency restored in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    let mut last_reference_round = None;
    let mut last_producer_stage = None;
    let reference_progress = |round: &agq_kerml_text::library::ReferenceRefinementRound| {
        last_reference_round = Some(json!({
            "round":round.round,"selected":round.selected_endpoints,
            "kernel_obligations":round.structural_obligations,
            "evaluated":round.references_evaluated,"reused":round.references_reused
        }));
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
    };
    let batch_progress = |round: usize, done: usize, total: usize, planned: usize| {
        if done.is_multiple_of(256) || done == total {
            println!("Systems: producers round={round} evaluated={done}/{total} planned={planned}");
        }
    };
    let producer_progress = |stage: &agq_kerml_semantics::PublicationStage| {
        let mut diagnostic_counts = BTreeMap::<&str, usize>::new();
        for diagnostic in &stage.diagnostics {
            *diagnostic_counts.entry(diagnostic.code).or_default() += 1;
        }
        last_producer_stage = Some(json!({
            "phase":"construction",
            "stage":stage.stage,"stratum":format!("{:?}",stage.stratum),
            "added":stage.added_elements,"completeness":format!("{:?}",stage.completeness),
            "closure_counters":closure_counters(&stage.counters),
            "diagnostics":stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>()
        }));
        // Persist each completed frontier so a watchdog stop still retains
        // its diagnostics. This is observational evidence, never acceptance.
        if let Some(stage) = &last_producer_stage {
            writeln!(stages, "{stage}").expect("write producer stage evidence");
            stages.flush().expect("flush producer stage evidence");
        }
        println!(
            "Systems: producer frontier={} stratum={:?} added={} completeness={:?} diagnostics={}",
            stage.stage,
            stage.stratum,
            stage.added_elements,
            stage.completeness,
            stage.diagnostics.len()
        );
        if !diagnostic_counts.is_empty() {
            println!("Systems: producer diagnostics={diagnostic_counts:?}");
        }
    };
    // Scoped audits must exercise final predicates. A normal full publication
    // uses the standard preparation path and performs final closure under the
    // strict publication contract below, without a redundant scoped final pass.
    let preparation = if audit_only {
        prepare_systems_library_slice_with_semantic_progress(
            &sources,
            accepted.clone(),
            SysmlSyntaxProfile::OperationalV2,
            &paths,
            reference_progress,
            batch_progress,
            producer_progress,
        )
    } else {
        prepare_systems_library_with_semantic_progress(
            &sources,
            accepted.clone(),
            SysmlSyntaxProfile::OperationalV2,
            reference_progress,
            batch_progress,
            producer_progress,
        )
    };
    let candidate = match preparation {
        Ok(candidate) => candidate,
        Err(error) => {
            write_report(
                &output,
                &json!({
                    "format":"agq-sysml-systems-publication-audit/1",
                    "sysml_profile":SysmlSyntaxProfile::OperationalV2.id(),
                    "scope":paths,
                    "accepted_kerml_digest":accepted.semantic_digest(),
                    "publication_attempted":false,"publication_accepted":false,
                    "preparation_error":error.to_string(),
                    "last_reference_round":last_reference_round,"last_producer_stage":last_producer_stage,
                    "elapsed_seconds":started.elapsed().as_secs_f64()
                }),
            )?;
            return Err(error.into());
        }
    };
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
    let mut document_counts = BTreeMap::<_, BTreeMap<&str, usize>>::new();
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
            *document_counts
                .entry(reference.origin.document)
                .or_default()
                .entry(status)
                .or_default() += 1;
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
    let authority_conflicts = candidate
        .production()
        .map(|production| production.authority_conflicts())
        .unwrap_or_default();
    // Bounded explanation for actual final diagnostic subjects. This keeps
    // pending writers distinct from completed evaluations whose upstream
    // requirements remain open, without serializing the full certificate.
    let registry = agq_kerml_semantics::ProducerRegistry::new(
        agq_kerml_semantics::ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(accepted.profile()))
            .chain(agq_sysml_semantics::sysml_producer_descriptors()),
    )
    .map_err(|family| format!("duplicate producer family: {}", family.name()))?;
    let diagnostic_subjects: BTreeSet<_> = candidate
        .production()
        .and_then(|production| production.stages.last())
        .into_iter()
        .flat_map(|stage| {
            stage
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.subject)
        })
        .collect();
    let closure_explanations: Vec<_> = draft.producer_closure().into_iter().flat_map(|certificate| {
        diagnostic_subjects.iter().map(|subject| json!({
            "subject":subject,
            "metaclass":model.element(*subject).map(|record|record.metaclass()),
            "source":draft.source_map().get(&agq_kernel::provenance::FactKey::Element(*subject)),
            "declared_name":model.navigation_slot(*subject,agq_kerml::properties::ELEMENT_DECLARED_NAME)
                .and_then(|slot|slot.value().values().find_map(|value|match value {
                    Value::String(name)=>Some(name),
                    _=>None,
                })),
            "requirements":agq_kerml_semantics::SemanticClosureRequirement::ALL.into_iter().map(|requirement|json!({
                "requirement":format!("{requirement:?}"),
                "closed":certificate.is_closed(*subject,requirement),
                "source":certificate.closure_source(*subject,requirement).map(|source|format!("{source:?}")),
            })).collect::<Vec<_>>(),
            "evaluations":registry.descriptors().iter().enumerate().filter_map(|(index,descriptor)| {
                let state=certificate.evaluation(*subject,index)?;
                (state != agq_kerml_semantics::ProducerEvaluationState::Inapplicable).then(||json!({
                    "family":descriptor.id.name(),"state":format!("{state:?}"),
                    "scope":format!("{:?}",descriptor.scope),"effects":format!("{:?}",descriptor.effects),
                }))
            }).collect::<Vec<_>>(),
        }))
    }).collect();
    let documents: Vec<_> = candidate.documents().iter().map(|document| {
        let reference_counts = document_counts.get(&document.document).cloned().unwrap_or_default();
        let reference_total: usize = reference_counts.values().sum();
        json!({
            "path":document.path,"document":document.document,"sha256":document.source_sha256,
            "profile":document.profile.id(),"parsed":document.parsed,"byte_exact":document.byte_exact,
            "recovery_count":document.recovery_count,"production_count":document.production_count,
            "construction_gap":document.construction_gap,
            "reference_audit_scope":"construction",
            "mandatory_references":{"total":reference_total,"counts":reference_counts},
        })
    }).collect();
    let mut report = json!({
        "format":"agq-sysml-systems-publication-audit/1",
        "scope":paths,
        "sysml_profile":candidate.syntax_profile().id(),
        "accepted_kerml_digest":accepted.semantic_digest(),
        "accepted_kerml_profile":accepted.profile().id(),
        "kerml_producers_replayed":false,
        "systems_documents_parsed":parsed,
        "systems_documents_constructed":constructed,
        "systems_documents_byte_exact":candidate.documents().iter().filter(|d|d.byte_exact).count(),
        "construction_complete":candidate.construction_complete(),
        "producer_closure":draft.producer_closure().map(|certificate|closure_report(certificate)),
        "kernel_obligations":draft.candidate().obligations().len(),
        "kernel_obligation_details":draft.candidate().obligations().iter().map(|obligation|
            json!({"element":obligation.element,"property":obligation.property,"actual":obligation.actual})
        ).collect::<Vec<_>>(),
        "last_reference_round":last_reference_round,
        "closure_transport":candidate.production().map(|production|json!({
            "retained_evaluations":production.retained_closure_evaluations,
            "reopened_evaluations":production.reopened_closure_evaluations,
            "rebindings":production.closure_rebindings,
            "total_retained_evaluations":production.total_retained_closure_evaluations,
            "total_reopened_evaluations":production.total_reopened_closure_evaluations,
        })),
        "closure_explanations":closure_explanations,
        "closure_explanations_scope":"construction",
        "local_elements":model.elements().filter(|r|accepted.overlay().model().element(r.id()).is_none()).count(),
        "mandatory_references":{"total":draft.references().len(),"counts":counts,"failures":failures},
        "reference_audit_scope":"construction",
        "authority_targets":authority,
        "authority_conflicts":authority_conflicts.iter().map(|conflict|json!({
            "rule":conflict.rule,"subject":conflict.subject,
            "formal_target":conflict.formal_target,"original_declaration":conflict.original_declaration,
        })).collect::<Vec<_>>(),
        "documents":documents,
        "construction_producers":candidate.production().map(|production|json!({"final_predicates":production.final_predicates,"completeness":format!("{:?}",production.completeness),"converged":production.converged,"rounds":production.counters.fixed_point_rounds,"subjects_evaluated":production.counters.subjects_evaluated,"derived_elements":production.counters.new_elements_proposed,"closure_counters":closure_counters(&production.counters),"diagnostics":production.stages.last().map(|stage|stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>())})), "publication_accepted":false,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
    });
    drop(queries);
    if audit_only {
        let passed = candidate.construction_complete()
            && candidate.production().is_some_and(|production| {
                production.final_predicates
                    && production.converged
                    && production.completeness == Completeness::Complete
            })
            && failures.is_empty()
            && authority_conflicts.is_empty();
        report["scoped_preflight_passed"] = json!(passed);
        report["publication_attempted"] = json!(false);
        write_report(&output, &report)?;
        if !passed {
            std::process::exit(1);
        }
        return Ok(());
    }
    report["publication_attempted"] = json!(true);
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
            let evidence = json!({
                "phase":"publication","stage":stage.stage,
                "stratum":format!("{:?}",stage.stratum),
                "added":stage.added_elements,"completeness":format!("{:?}",stage.completeness),
                "closure_counters":closure_counters(&stage.counters),
                "diagnostics":stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>()
            });
            writeln!(stages, "{evidence}").expect("write publication stage evidence");
            stages.flush().expect("flush publication stage evidence");
            println!(
                "Systems publication: frontier={} stratum={:?} added={} completeness={:?}",
                stage.stage, stage.stratum, stage.added_elements, stage.completeness
            )
        },
    ) {
        Ok(publication) => {
            // The facade is returned only after the strict final reference and
            // closure gates pass. Retain preparation observations separately;
            // they are not the accepted publication's final counts or proof.
            report["construction_reference_audit"] = report["mandatory_references"].take();
            report["mandatory_references"] = json!({
                "total":publication.audit().mandatory_references,
                "counts":{
                    "complete":publication.audit().complete_references,
                    "incomplete":0,"unresolved":0,"ambiguous":0,
                    "invalid":0,"endpoint_mismatch":0,
                },
                "failures":[],
            });
            report["reference_audit_scope"] = json!("accepted_publication");
            report["construction_producer_closure"] = report["producer_closure"].take();
            report["producer_closure"] = closure_report(publication.producer_closure());
            report["publication_producers"] = json!({
                "converged":true,"completeness":"Complete",
            });
            report["publication_accepted"] = json!(true);
            report["publication_digest"] = json!(publication.publication_digest());
            report["semantic_digest"] = json!(publication.semantic_digest());
            report["accepted_bindings"] = json!(publication.bindings().targets().len());
            report["publication_gate"] = audit_report(publication.audit());
            report["publication_closure_counters"] = closure_counters(publication.counters());
            // Preserve the already accepted graph before this process exits.
            // Receipt files are outputs here, not trusted restoration inputs.
            let directory = output.parent().ok_or("output parent")?;
            std::fs::create_dir_all(directory)?;
            let cache_path = directory.join("canonical.publication.zip");
            let mut cache = std::fs::File::create(&cache_path)?;
            let receipt = publication.write_cache(&mut cache, &sources)?;
            cache.sync_all()?;
            write_report(&directory.join("accepted-publication.json"), &receipt)?;
            write_report(
                &directory.join("standard-bindings.json"),
                &publication.binding_manifest(&sources)?,
            )?;
            let persisted_bindings = serde_json::from_reader(std::fs::File::open(
                directory.join("standard-bindings.json"),
            )?)?;
            publication.check_binding_manifest(&sources, &persisted_bindings)?;
            report["bindings_stale_check"] = json!(true);
            report["exported_cache"] = json!(cache_path);
        }
        Err(agq_kerml_text::sysml::SystemsPublicationError::Rejected(audit)) => {
            report["publication_gate"] = audit_report(&audit);
        }
        Err(error) => {
            report["publication_error"] = json!(error.to_string());
        }
    }
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    write_report(&output, &report)?;
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

fn closure_report(
    certificate: &agq_kerml_semantics::ProducerClosureCertificate,
) -> serde_json::Value {
    json!({
        "digest":certificate.digest(),
        "semantic_closure_digest":certificate.semantic_closure_digest(),
        "model_digest":certificate.model_digest(),
        "producer_registry_digest":certificate.registry_digest(),
        "context_contract_digest":certificate.context_contract_digest(),
        "certificate_bytes":certificate.storage_bytes(),
        "revalidation_bytes":certificate.revalidation_storage_bytes(),
        "applicable_pairs":certificate.applicable_pairs(),
        "closed_pairs":certificate.closed_pairs(),
        "incomplete_pairs":certificate.incomplete_pairs(),
        "closed_requirements":certificate.closed_effects(),
    })
}

fn write_report(
    output: &std::path::Path,
    report: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output.parent().ok_or("output parent")?)?;
    let mut file = std::fs::File::create(output)?;
    file.write_all(&serde_json::to_vec_pretty(report)?)?;
    file.sync_all()?;
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

fn closure_counters(counters: &agq_kerml_semantics::PublicationCounters) -> serde_json::Value {
    json!({
        "families_registered":counters.families_registered,
        "applicable_subject_family_pairs":counters.applicable_subject_family_pairs,
        "closed_pairs":counters.closed_producer_pairs,
        "closed_requirements":counters.closed_producer_effects,
        "incomplete_pairs":counters.incomplete_producer_pairs,
        "certificate_bytes":counters.certificate_bytes,
        "certificate_build_micros":counters.certificate_build_micros,
        "negative_queries_certified":counters.negative_queries_certified,
    })
}
