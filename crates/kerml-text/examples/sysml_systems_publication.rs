//! Exact Systems candidate/closure audit over a restored sealed KerML publication.
use agq_kerml_semantics::{Completeness, PublicationFrontierSession, QualifiedName};
use agq_kerml_syntax::production::SysmlSyntaxProfile;
use agq_kerml_text::{
    library::CanonicalKermlStandardLibraries,
    sysml::{
        SYSTEMS_PUBLICATION_MAX_ROUNDS, prepare_systems_library_slice_with_semantic_progress,
        prepare_systems_library_with_frontier_checkpoints,
        prepare_systems_library_with_semantic_progress, systems_frontier_source_identity,
    },
};
use agq_kernel::value::Value;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

#[path = "support/publication_metrics.rs"]
mod publication_metrics;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let argument = |prefix: &str| {
        std::env::args().find_map(|arg| arg.strip_prefix(prefix).map(|value| root.join(value)))
    };
    let cache =
        argument("--cache=").ok_or("--cache=<accepted KerML publication cache> is required")?;
    let output = argument("--output=").ok_or("--output=<report path> is required")?;
    let checkpoint_directory = argument("--checkpoint=");
    let resume_journal = argument("--resume=");
    let resume_pin =
        std::env::args().find_map(|arg| arg.strip_prefix("--resume-sha256=").map(str::to_owned));
    let contextual_interval: usize = std::env::args()
        .find_map(|arg| {
            arg.strip_prefix("--checkpoint-interval=")
                .map(str::to_owned)
        })
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(0);
    if checkpoint_directory.is_some() && resume_journal.is_some() {
        return Err("choose --checkpoint or --resume".into());
    }
    if resume_journal.is_some() != resume_pin.is_some() {
        return Err(
            "--resume and independently retained --resume-sha256 are required together".into(),
        );
    }
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
    let source_identity = systems_frontier_source_identity(
        &sources,
        &accepted,
        SysmlSyntaxProfile::OperationalV2,
        audit_only.then_some(&paths),
    );
    let checkpoints = if let Some(directory) = checkpoint_directory {
        Some(Arc::new(PublicationFrontierSession::create(
            directory,
            source_identity,
            contextual_interval,
        )?))
    } else if let Some(journal) = resume_journal {
        Some(Arc::new(PublicationFrontierSession::resume(
            journal,
            parse_digest(resume_pin.as_deref().expect("validated resume pin"))?,
            source_identity,
            contextual_interval,
        )?))
    } else {
        None
    };
    let mut last_reference_round = None;
    let mut last_producer_stage = None;
    let reference_progress = |round: &agq_kerml_text::library::ReferenceRefinementRound| {
        last_reference_round = Some(json!({
            "elapsed_seconds":started.elapsed().as_secs_f64(),
            "round":round.round,"selected":round.selected_endpoints,
            "kernel_obligations":round.structural_obligations,
            "evaluated":round.references_evaluated,"reused":round.references_reused
        }));
        println!(
            "Systems: references round={} selected={} obligations={} evaluated={} reused={} elapsed={:.3}s",
            round.round,
            round.selected_endpoints,
            round.structural_obligations,
            round.references_evaluated,
            round.references_reused,
            started.elapsed().as_secs_f64()
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
            println!(
                "Systems: producers round={round} evaluated={done}/{total} planned={planned} elapsed={:.3}s",
                started.elapsed().as_secs_f64()
            );
        }
    };
    let producer_progress = |stage: &agq_kerml_semantics::PublicationStage| {
        let mut diagnostic_counts = BTreeMap::<&str, usize>::new();
        for diagnostic in &stage.diagnostics {
            *diagnostic_counts.entry(diagnostic.code).or_default() += 1;
        }
        last_producer_stage = Some(json!({
            "phase":"construction",
            "elapsed_seconds":started.elapsed().as_secs_f64(),
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
            "Systems: producer frontier={} stratum={:?} added={} closed_pairs={} completeness={:?} diagnostics={} elapsed={:.3}s",
            stage.stage,
            stage.stratum,
            stage.added_elements,
            stage.counters.closed_producer_pairs,
            stage.completeness,
            stage.diagnostics.len(),
            started.elapsed().as_secs_f64()
        );
        if !diagnostic_counts.is_empty() {
            println!("Systems: producer diagnostics={diagnostic_counts:?}");
        }
    };
    // Scoped audits must exercise final predicates. A normal full publication
    // uses the standard preparation path and performs final closure under the
    // strict publication contract below, without a redundant scoped final pass.
    let preparation = if let Some(session) = &checkpoints {
        prepare_systems_library_with_frontier_checkpoints(
            &sources,
            accepted.clone(),
            SysmlSyntaxProfile::OperationalV2,
            audit_only.then_some(&paths),
            session.clone(),
            reference_progress,
            batch_progress,
            producer_progress,
        )
    } else if audit_only {
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
    let mut reference_semantics = Sha256::new();
    reference_semantics.update(b"agq-systems-mandatory-reference-results/1\0");
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
            let observation = serde_json::to_vec(&json!({
                "relationship":reference.relationship,"property":reference.property,
                "name":reference.name.segments,"origin":reference.origin,
                "status":status,"targets":targets,"stored":stored,
                "diagnostics":answer.diagnostics.iter().map(|d|json!({
                    "code":d.code,"subject":d.subject,"message":d.message
                })).collect::<Vec<_>>(),
            }))?;
            reference_semantics.update((observation.len() as u64).to_le_bytes());
            reference_semantics.update(observation);
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
    // Bounded explanation for final diagnostics and incomplete evaluations. This keeps
    // pending writers distinct from completed evaluations whose upstream
    // requirements remain open, without serializing the full certificate.
    let registry = agq_kerml_semantics::ProducerRegistry::new(
        agq_kerml_semantics::ProducerFamily::ALL
            .into_iter()
            .map(|family| family.descriptor(accepted.profile()))
            .chain(agq_sysml_semantics::sysml_producer_descriptors()),
    )
    .map_err(|family| format!("duplicate producer family: {}", family.name()))?;
    let mut diagnostic_subjects: BTreeSet<_> = candidate
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
    let requested_subjects: BTreeSet<_> = std::env::var("AGQ_PRODUCER_CAUSAL_SUBJECTS")
        .unwrap_or_default()
        .split(',')
        .map(|id| id.trim().to_ascii_lowercase())
        .filter(|id| !id.is_empty())
        .take(32)
        .collect();
    diagnostic_subjects.extend(model.elements().filter_map(|record| {
        requested_subjects
            .contains(&record.id().to_string())
            .then_some(record.id())
    }));
    if let Some(certificate) = draft.producer_closure() {
        diagnostic_subjects.extend(model.elements().filter_map(|record| {
            registry
                .descriptors()
                .iter()
                .enumerate()
                .any(|(index, _)| {
                    certificate.evaluation(record.id(), index)
                        == Some(agq_kerml_semantics::ProducerEvaluationState::EvaluatedIncomplete)
                })
                .then_some(record.id())
        }));
    }
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
        "producer_closure":draft.producer_closure().map(|certificate|closure_report(certificate, model)),
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
        "construction_reference_semantic_digest":<[u8;32]>::from(reference_semantics.finalize()),
        "authority_targets":authority,
        "authority_conflicts":authority_conflicts.iter().map(|conflict|json!({
            "rule":conflict.rule,"subject":conflict.subject,
            "formal_target":conflict.formal_target,"original_declaration":conflict.original_declaration,
        })).collect::<Vec<_>>(),
        "documents":documents,
        "construction_producers":candidate.production().map(|production|json!({"final_predicates":production.final_predicates,"completeness":format!("{:?}",production.completeness),"converged":production.converged,"rounds":production.counters.fixed_point_rounds,"round_limit":SYSTEMS_PUBLICATION_MAX_ROUNDS,"round_limit_reached":!production.converged && production.counters.fixed_point_rounds >= SYSTEMS_PUBLICATION_MAX_ROUNDS,"subjects_evaluated":production.counters.subjects_evaluated,"derived_elements":production.counters.new_elements_proposed,"closure_counters":closure_counters(&production.counters),"diagnostics":production.stages.last().map(|stage|stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>())})), "publication_accepted":false,
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "checkpoint_session":checkpoint_report(checkpoints.as_deref())?,
        "incremental_certificate_reference_check":std::env::var_os("AGQ_CERTIFICATE_VERIFY_FULL_REBUILD").is_some(),
    });
    drop(queries);
    if audit_only {
        let passed = candidate.construction_complete()
            && draft
                .producer_closure()
                .is_some_and(|certificate| certificate.is_fully_closed(model))
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
        agq_kerml_semantics::PublicationClosureOptions {
            max_rounds: SYSTEMS_PUBLICATION_MAX_ROUNDS,
            frontier_checkpoints: checkpoints.clone(),
            ..Default::default()
        },
        |round, done, total, planned| {
            if done.is_multiple_of(256) || done == total {
                println!(
                    "Systems publication: round={round} evaluated={done}/{total} planned={planned} elapsed={:.3}s",
                    started.elapsed().as_secs_f64()
                );
            }
        },
        |stage| {
            let evidence = json!({
                "phase":"publication","stage":stage.stage,
                "elapsed_seconds":started.elapsed().as_secs_f64(),
                "stratum":format!("{:?}",stage.stratum),
                "added":stage.added_elements,"completeness":format!("{:?}",stage.completeness),
                "closure_counters":closure_counters(&stage.counters),
                "diagnostics":stage.diagnostics.iter().map(|d|json!({"code":d.code,"subject":d.subject,"message":d.message})).collect::<Vec<_>>()
            });
            writeln!(stages, "{evidence}").expect("write publication stage evidence");
            stages.flush().expect("flush publication stage evidence");
            println!(
                "Systems publication: frontier={} stratum={:?} added={} closed_pairs={} completeness={:?} elapsed={:.3}s",
                stage.stage,
                stage.stratum,
                stage.added_elements,
                stage.counters.closed_producer_pairs,
                stage.completeness,
                started.elapsed().as_secs_f64()
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
            report["producer_closure"] = closure_report(
                publication.producer_closure(),
                publication.overlay().model(),
            );
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
    report["checkpoint_session"] = checkpoint_report(checkpoints.as_deref())?;
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
    model: &agq_kernel::ModelView,
) -> serde_json::Value {
    json!({
        "digest":certificate.digest(),
        "semantic_closure_digest":certificate.semantic_closure_digest(),
        "revalidation_digest":certificate.revalidation_digest(),
        "model_digest":certificate.model_digest(),
        "producer_registry_digest":certificate.registry_digest(),
        "context_contract_digest":certificate.context_contract_digest(),
        "certificate_bytes":certificate.storage_bytes(),
        "revalidation_bytes":certificate.revalidation_storage_bytes(),
        "applicable_pairs":certificate.applicable_pairs(),
        "closed_pairs":certificate.closed_pairs(),
        "incomplete_pairs":certificate.incomplete_pairs(),
        "closed_requirements":certificate.closed_effects(),
        "required_requirements":model.elements().count()
            * agq_kerml_semantics::SemanticClosureRequirement::ALL.len(),
        "fully_closed":certificate.is_fully_closed(model),
    })
}

fn parse_digest(value: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("resume SHA-256 must contain exactly 64 hexadecimal digits".into());
    }
    let mut digest = [0; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(digest)
}

fn checkpoint_report(
    session: Option<&PublicationFrontierSession>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let Some(session) = session else {
        return Ok(serde_json::Value::Null);
    };
    let latest = session.latest_checkpoint()?.map(|(path, digest)| {
        json!({"journal":path,"sha256":digest.iter().map(|byte|format!("{byte:02x}")).collect::<String>()})
    });
    Ok(json!({"statistics":session.statistics(),"latest":latest,"accepted_authority":false}))
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
    let mut result = publication_metrics::counters(counters);
    result.as_object_mut().expect("counter object").extend(
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
        .as_object()
        .expect("closure counter object")
        .clone(),
    );
    result
}
