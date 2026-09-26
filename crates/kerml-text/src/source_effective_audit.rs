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
) -> Result<(SourceEffectiveAudit, Vec<SourceDiagnostic>, usize), LibraryLoadError> {
    #[cfg(feature = "verification")]
    let experiment_started = Instant::now();
    // Verification experiment only. Shipping builds retain the same bounded
    // population; the experiment changes evaluator lifetime, never context.
    #[cfg(feature = "verification")]
    let batch_size = verification_batch_size()?;
    #[cfg(not(feature = "verification"))]
    let batch_size = 32;
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
    let closed = ClosedAuditContext::new(bound.kerml());
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
    for batch in subjects.chunks(batch_size) {
        let q = bound.fork();
        for &subject in batch {
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
    if std::env::var_os("AGENTIQUE_AUDIT_BATCH_SIZE").is_some() {
        use std::io::Write;
        let _ = writeln!(
            std::io::stderr().lock(),
            "SOURCE_AUDIT_BATCH_EXPERIMENT {}",
            serde_json::json!({
                "format": "agentique-audit-batch-experiment/1",
                "batch_size": batch_size,
                "subjects": subjects.len(),
                "batches": subjects.len().div_ceil(batch_size),
                "subjects_evaluated": subjects.len() - reused,
                "subjects_reused": reused,
                "checks_reused": reused_checks,
                "elapsed_micros": experiment_started.elapsed().as_micros(),
            })
        );
    }
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
fn verification_batch_size() -> Result<usize, LibraryLoadError> {
    match std::env::var("AGENTIQUE_AUDIT_BATCH_SIZE") {
        Err(std::env::VarError::NotPresent) => Ok(32),
        Ok(value) if value == "32" => Ok(32),
        Ok(value) if value == "128" => Ok(128),
        Ok(value) if value == "256" => Ok(256),
        _ => Err(LibraryLoadError::Interpretation(
            "verification audit batch size must be exactly 32, 128 or 256".into(),
        )),
    }
}
