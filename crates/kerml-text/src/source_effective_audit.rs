//! Strict authored audit with explicit successful-outcome reuse evidence.
use super::*;
use crate::sysml::{SystemsPublicationFamily, audit_authored_effective_subject};
use agq_kerml_semantics::{ClosedAuditContext, ClosedAuditReads, ClosedAuditSnapshot};

#[derive(Clone, Debug)]
pub(super) struct SuccessfulAudit {
    reads: ClosedAuditReads,
    checked: BTreeMap<SystemsPublicationFamily, usize>,
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
    for batch in subjects.chunks(32) {
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
                retained.insert(
                    subject,
                    SuccessfulAudit {
                        reads,
                        checked: entry.checked.clone(),
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
