//! Strict authored audit with explicit successful-outcome reuse evidence.
use super::*;
use crate::sysml::{SystemsPublicationFamily, audit_authored_effective_subject};
use agq_kerml_semantics::{ClosedAuditContext, ClosedAuditReads, ClosedAuditSnapshot};

#[derive(Clone, Debug)]
pub(super) struct SuccessfulAudit {
    reads: ClosedAuditReads,
    checked: BTreeMap<SystemsPublicationFamily, usize>,
    mandatory_references: usize,
    complete_references: usize,
}

#[derive(Debug)]
pub(super) struct AuditReuse {
    snapshot: ClosedAuditSnapshot,
    subjects: BTreeMap<ElementId, SuccessfulAudit>,
}

pub(super) fn run(
    result: &SourceCompilation,
    previous: Option<&SourceCompilation>,
    control: &CompilationControl,
) -> Result<(SourceEffectiveAudit, Vec<SourceDiagnostic>, usize), LibraryLoadError> {
    run_with_batch_progress(result, previous, control, |_| {})
}

// Private deterministic observation seam for the accepted-runtime interruption
// regression. Ordinary callers use the no-op monomorphization above. The callback
// cannot supply a query answer, receipt or acceptance result.
fn run_with_batch_progress(
    result: &SourceCompilation,
    previous: Option<&SourceCompilation>,
    control: &CompilationControl,
    mut completed_batch: impl FnMut(usize),
) -> Result<(SourceEffectiveAudit, Vec<SourceDiagnostic>, usize), LibraryLoadError> {
    control.check()?;
    #[cfg(feature = "verification")]
    let mut trace = (std::env::var("AGENTIQUE_AUDIT_REUSE_TRACE").as_deref() == Ok("1"))
        .then(AuditReuseTrace::default);
    let bound = result
        .sysml_queries()
        .map_err(|error| LibraryLoadError::Interpretation(format!("{error:?}")))?;
    let context = bound.context().clone();
    let mut subjects: Vec<_> = bound
        .model()
        .elements()
        .filter(|record| {
            result
                .inputs
                .dependency
                .publication
                .overlay()
                .model()
                .element(record.id())
                .is_none()
        })
        .map(|record| record.id())
        .collect();
    subjects.sort_unstable();
    let reuse_started = Instant::now();
    control.check()?;
    let closed = ClosedAuditContext::new(bound.kerml());
    #[cfg(feature = "verification")]
    let prior_trace_reason = trace.as_ref().map(|_| {
        let Some(previous) = previous else {
            return "no_parent_compilation";
        };
        let Some(audit) = previous.effective_audit.as_ref() else {
            return "no_parent_effective_audit";
        };
        let mut current = context.clone();
        current.kerml = audit.context.kerml.clone();
        if current != audit.context {
            "sysml_static_contract_mismatch"
        } else if audit.reuse.is_none() {
            "no_parent_reuse_receipt"
        } else {
            "prior_receipt_available"
        }
    });
    let previous = previous
        .and_then(|previous| previous.effective_audit.as_ref())
        .filter(|previous| {
            let mut current = context.clone();
            current.kerml = previous.context.kerml.clone();
            current == previous.context
        })
        .and_then(|previous| previous.reuse.as_ref());
    let delta = closed
        .as_ref()
        .zip(previous)
        .and_then(|(closed, previous)| closed.delta(&previous.snapshot));
    let reuse_setup_micros = elapsed_micros(reuse_started);
    let mut retained = BTreeMap::new();
    let mut reused = 0;
    let mut reused_checks = 0;
    let mut report = SystemsPublicationAudit::default();
    let mut capabilities = Vec::new();
    for (index, batch) in subjects.chunks(32).enumerate() {
        control.check()?;
        let q = bound.fork();
        for &subject in batch {
            control.check()?;
            #[cfg(feature = "verification")]
            if let Some(trace) = trace.as_mut() {
                let entry = previous.and_then(|previous| previous.subjects.get(&subject));
                let diagnostic = match (closed.as_ref(), previous) {
                    (None, _) => serde_json::json!({"reason": "current_context_not_closed"}),
                    (_, None) => serde_json::json!({"reason": prior_trace_reason}),
                    (Some(closed), Some(previous)) => closed.verification_reuse_trace(
                        &previous.snapshot,
                        entry.map(|entry| &entry.reads),
                        delta.as_ref(),
                    ),
                };
                trace.record(
                    subject,
                    entry.map_or(0, |entry| entry.checked.values().sum()),
                    diagnostic,
                );
            }
            if let Some(entry) = previous.and_then(|previous| previous.subjects.get(&subject))
                && let Some((closed, delta)) = closed.as_ref().zip(delta.as_ref())
                && let Some(reads) = closed.transport(&entry.reads, delta)
            {
                for (&family, &count) in &entry.checked {
                    *report.checked.entry(family).or_default() += count;
                    reused_checks += count;
                }
                report.mandatory_references += entry.mandatory_references;
                report.complete_references += entry.complete_references;
                retained.insert(
                    subject,
                    SuccessfulAudit {
                        reads,
                        checked: entry.checked.clone(),
                        mandatory_references: entry.mandatory_references,
                        complete_references: entry.complete_references,
                    },
                );
                reused += 1;
                continue;
            }
            let mut one = SystemsPublicationAudit::default();
            let mut reads = ClosedAuditReads::default();
            reads.subject(subject);
            audit_authored_effective_subject(
                &q,
                subject,
                &mut one,
                |subject, answer| {
                    if answer.completeness() != Completeness::Complete {
                        capabilities.push(SourceDiagnostic::Capability {
                            subject,
                            origin: result.source_map().get(&FactKey::Element(subject)).cloned(),
                            answer: Box::new(answer.clone()),
                        });
                    }
                },
                closed.as_ref(),
                &mut reads,
            );
            if closed.is_some() && one.findings.is_empty() {
                retained.insert(
                    subject,
                    SuccessfulAudit {
                        reads,
                        checked: one.checked.clone(),
                        mandatory_references: one.mandatory_references,
                        complete_references: one.complete_references,
                    },
                );
            }
            for (family, count) in one.checked {
                *report.checked.entry(family).or_default() += count;
            }
            report.mandatory_references += one.mandatory_references;
            report.complete_references += one.complete_references;
            report.findings.extend(one.findings);
        }
        completed_batch(index + 1);
    }
    for finding in &report.findings {
        let origin = match finding {
            SystemsPublicationFinding::Capability { diagnostic, .. } => result
                .source_map()
                .get(&FactKey::Element(diagnostic.subject))
                .cloned(),
            _ => None,
        };
        capabilities.push(SourceDiagnostic::EffectiveAudit {
            origin,
            finding: Box::new(finding.clone()),
        });
    }
    let reuse = closed.map(|closed| AuditReuse {
        snapshot: closed.into_snapshot(),
        subjects: retained,
    });
    #[cfg(feature = "verification")]
    if let Some(trace) = trace {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr().lock(),
            "SOURCE_AUDIT_REUSE_TRACE {}",
            serde_json::json!({
                "format": "agentique-audit-reuse-trace/1",
                "context": format!("{context:?}"),
                "subjects": subjects.len(),
                "reused_subjects": reused,
                "reused_checks": reused_checks,
                "first_rejection_counts": trace.counts,
                "prior_checked_subject_samples": trace.samples,
                "subject_sample_limit": 16,
                "changed_read_sample_limit": 8,
                "contract": "Diagnostic only; first-rejection counts are exhaustive, samples are bounded. Existing reuse predicates are unchanged.",
            })
        );
    }
    control.check()?;
    Ok((
        SourceEffectiveAudit {
            context,
            subjects,
            report,
            reuse,
            reused_checks,
            reuse_setup_micros,
        },
        capabilities,
        reused,
    ))
}

#[cfg(feature = "verification")]
#[derive(Default)]
struct AuditReuseTrace {
    counts: BTreeMap<String, usize>,
    samples: Vec<serde_json::Value>,
}
#[cfg(feature = "verification")]
impl AuditReuseTrace {
    fn record(&mut self, subject: ElementId, checks: usize, diagnostic: serde_json::Value) {
        let reason = diagnostic["reason"].as_str().expect("trace has a reason");
        *self.counts.entry(reason.to_owned()).or_default() += 1;
        if reason != "preserved" && checks > 0 && self.samples.len() < 16 {
            self.samples.push(serde_json::json!({
                "subject": subject.to_string(), "prior_checks": checks, "diagnostic": diagnostic,
            }));
        }
    }
}

#[cfg(all(test, feature = "verification"))]
#[test]
fn reuse_trace_counts_all_subjects_but_bounds_failed_checked_samples() {
    let mut trace = AuditReuseTrace::default();
    for n in 0..100 {
        trace.record(
            ElementId::from_u128(n),
            2,
            serde_json::json!({"reason": "bounded_read_changed"}),
        );
    }
    trace.record(
        ElementId::from_u128(101),
        0,
        serde_json::json!({"reason": "no_successful_prior_subject"}),
    );
    trace.record(
        ElementId::from_u128(102),
        2,
        serde_json::json!({"reason": "preserved"}),
    );
    assert_eq!(trace.counts["bounded_read_changed"], 100);
    assert_eq!(trace.counts["no_successful_prior_subject"], 1);
    assert_eq!(trace.counts["preserved"], 1);
    assert_eq!(trace.samples.len(), 16);
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::{fs::File, path::PathBuf};

    fn subject_receipt_digest(audit: &SourceEffectiveAudit) -> [u8; 32] {
        use sha2::Digest;
        // Exact per-subject positive/negative/provider/writer reuse evidence,
        // excluding setup timings and the shared immutable model signature.
        sha2::Sha256::digest(format!("{:?}", audit.reuse.as_ref().unwrap().subjects).as_bytes())
            .into()
    }

    #[test]
    #[ignore = "requires exact accepted runtime caches; never rebuilds standards"]
    fn cancellation_after_second_effective_audit_batch_returns_no_partial_receipt() {
        let root = std::env::var_os("AGENTIQUE_SOURCE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        let open = |name| {
            File::open(
                std::env::var_os(name)
                    .unwrap_or_else(|| panic!("{name} is required for this requested gate")),
            )
            .unwrap()
        };
        let kerml_file = open("AGENTIQUE_KERML_CACHE");
        let systems_file = open("AGENTIQUE_SYSTEMS_CACHE");
        let libraries =
            agq_standard_libraries::VerifiedLibrarySet::load_from_directory(&root).unwrap();
        let kerml = Arc::new(
            crate::library::CanonicalKermlStandardLibraries::restore_cache(kerml_file, &libraries)
                .unwrap(),
        );
        let publication = Arc::new(
            CanonicalSysmlSystemsLibrary::restore_cache(systems_file, &libraries, kerml).unwrap(),
        );
        let members: String = (0..80)
            .map(|index| format!("part member{index}; "))
            .collect();
        let inputs = SourceInputs::with_accepted_sysml(publication)
            .unwrap()
            .apply([ProjectChange::Add {
                path: "Cancellation.sysml".into(),
                language: SourceLanguage::SysMl,
                source: format!(
                    "package AuditCancellation {{ part def Assembly {{ {members} }} }}"
                ),
            }])
            .unwrap();
        let parent = Arc::new(inputs).compile(None).unwrap();
        let parent_audit = parent.effective_audit().unwrap();
        assert!(
            parent_audit.subjects().len() > 64,
            "test must cross a later real audit batch"
        );
        assert!(
            parent_audit.report().findings.is_empty(),
            "{:?}",
            parent_audit.report()
        );
        let before_identity = parent.identity_checkpoint();
        let before_context = parent.sysml_queries().unwrap().context().clone();
        let before_receipts = subject_receipt_digest(parent_audit);
        let before_certificate = parent.producer_closure().unwrap().clone();
        assert!(before_certificate.is_fully_closed(parent.semantic_model().unwrap()));

        let control = CompilationControl::new();
        control
            .enter(CompilationStage::EffectiveValidation)
            .unwrap();
        let mut completed = 0;
        let interrupted = run_with_batch_progress(&parent, None, &control, |batches| {
            completed = batches;
            if batches == 2 {
                control.cancel();
            }
        });
        assert!(
            matches!(interrupted, Err(LibraryLoadError::Cancelled(_))),
            "an interrupted audit must not return any completed report or reuse receipt"
        );
        assert_eq!(
            completed, 2,
            "the third batch must not start after cancellation"
        );
        assert_eq!(control.stage(), Some(CompilationStage::EffectiveValidation));
        assert_eq!(parent.identity_checkpoint(), before_identity);
        assert_eq!(parent.sysml_queries().unwrap().context(), &before_context);
        assert!(Arc::ptr_eq(
            parent.producer_closure().unwrap(),
            &before_certificate
        ));
        assert_eq!(
            subject_receipt_digest(parent.effective_audit().unwrap()),
            before_receipts
        );

        let retry = CompilationControl::new();
        retry.enter(CompilationStage::EffectiveValidation).unwrap();
        let (audit, diagnostics, reused) = run(&parent, None, &retry).unwrap();
        assert_eq!(
            reused, 0,
            "retry must perform the full audit rather than reuse interrupted work"
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(audit.context(), parent_audit.context());
        assert_eq!(audit.subjects(), parent_audit.subjects());
        assert_eq!(audit.report().checked, parent_audit.report().checked);
        assert_eq!(
            audit.report().mandatory_references,
            parent_audit.report().mandatory_references
        );
        assert_eq!(
            audit.report().complete_references,
            parent_audit.report().complete_references
        );
        assert!(audit.report().findings.is_empty());
        assert_eq!(
            subject_receipt_digest(&audit),
            before_receipts,
            "fresh full audit retains exact read/writer proof"
        );
        eprintln!(
            "AUDIT_CANCELLATION {}",
            serde_json::json!({
                "completed_batches_before_stop": completed,
                "subjects": audit.subjects().len(),
                "retry_reused_subjects": reused,
                "exact_subject_read_proof": true,
                "parent_identity_context_certificate_preserved": true,
            })
        );
    }
}
