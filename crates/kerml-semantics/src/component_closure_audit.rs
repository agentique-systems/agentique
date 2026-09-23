//! Read-only inspection of a proposed component against existing closure proof.
//!
//! This is deliberately not a component certificate issuer. A dependency plan,
//! a closed local evaluation count, or this audit's digest cannot authenticate a
//! future-writer boundary or grant publication acceptance.
use super::*;
use crate::{ContextError, SemanticContext};

/// An existing exact-graph certificate does not establish this local obligation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentClosureFinding {
    EmptyPopulation,
    MissingSubject(ElementId),
    PendingProducer {
        subject: ElementId,
        family: ProducerFamilyId,
    },
    IncompleteProducer {
        subject: ElementId,
        family: ProducerFamilyId,
    },
    OpenRequirement {
        subject: ElementId,
        requirement: SemanticClosureRequirement,
    },
    /// Compact trusted receipts omit optional scheduler reads. They retain their
    /// acceptance authority but cannot supply a new component boundary proof.
    UnavailableReadEvidence {
        subject: ElementId,
        family: ProducerFamilyId,
    },
}

/// Deterministic diagnostic projection of scheduler-issued closure evidence.
///
/// Passing this audit is necessary, not sufficient, for sealing: the source
/// partition, graph delta, dependencies, mandatory references and exclusion of
/// future writers still need their independently checked composition contract.
/// The object has no public constructor and cannot be attached to a context.
#[derive(Clone, Debug)]
pub struct ComponentClosureAudit {
    subjects: Box<[ElementId]>,
    source_certificate_digest: [u8; 32],
    read_evidence_digest: [u8; 32],
    digest: [u8; 32],
    applicable_pairs: usize,
    closed_pairs: usize,
    closed_requirements: usize,
    findings: Box<[ComponentClosureFinding]>,
}

impl ComponentClosureAudit {
    pub fn subjects(&self) -> &[ElementId] {
        &self.subjects
    }
    pub fn source_certificate_digest(&self) -> [u8; 32] {
        self.source_certificate_digest
    }
    /// Binds exact captured provider reads, including evaluations with no output.
    /// Allocation layout, interning and row insertion order are not identities.
    pub fn read_evidence_digest(&self) -> [u8; 32] {
        self.read_evidence_digest
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn applicable_pairs(&self) -> usize {
        self.applicable_pairs
    }
    pub fn closed_pairs(&self) -> usize {
        self.closed_pairs
    }
    pub fn closed_requirements(&self) -> usize {
        self.closed_requirements
    }
    pub fn findings(&self) -> &[ComponentClosureFinding] {
        &self.findings
    }
    /// Every inspected producer/requirement is closed and its captured reads are
    /// available. This does not assert that a publication stratum may seal.
    pub fn local_obligations_closed(&self) -> bool {
        self.findings.is_empty()
    }
}

impl ProducerClosureCertificate {
    /// Inspect a proposed semantic component without evaluating any producers or
    /// changing the certificate's global writer/provider analysis. In particular,
    /// pending producers outside `subjects` are never masked or marked immutable.
    pub fn audit_component(
        &self,
        context: &SemanticContext<'_>,
        expected_registry: &ProducerRegistry,
        subjects: &BTreeSet<ElementId>,
    ) -> Result<ComponentClosureAudit, ContextError> {
        if !self.compatible_context(context.id())
            || self.registry_digest != expected_registry.digest()
        {
            return Err(ContextError::ProducerClosureMismatch);
        }
        let mut findings = Vec::new();
        let mut applicable_pairs = 0;
        let mut closed_pairs = 0;
        let mut closed_requirements = 0;
        let mut evidence = Sha256::new();
        evidence.update(b"agq-component-producer-reads/1\0");
        evidence.update((subjects.len() as u64).to_be_bytes());
        if subjects.is_empty() {
            findings.push(ComponentClosureFinding::EmptyPopulation);
        }
        for &subject in subjects {
            evidence.update(subject.as_u128().to_be_bytes());
            if context.model.element(subject).is_none()
                || self.subjects.binary_search(&subject).is_err()
            {
                evidence.update([0]);
                findings.push(ComponentClosureFinding::MissingSubject(subject));
                continue;
            }
            evidence.update([1]);
            evidence.update((expected_registry.descriptors().len() as u64).to_be_bytes());
            for (family_index, descriptor) in expected_registry.descriptors().iter().enumerate() {
                hash_bytes(&mut evidence, descriptor.id.name().as_bytes());
                let state = self
                    .evaluation(subject, family_index)
                    .expect("verified registry");
                evidence.update([state as u8]);
                applicable_pairs += usize::from(state != ProducerEvaluationState::Inapplicable);
                closed_pairs += usize::from(state == ProducerEvaluationState::EvaluatedComplete);
                match state {
                    ProducerEvaluationState::Pending => {
                        findings.push(ComponentClosureFinding::PendingProducer {
                            subject,
                            family: descriptor.id,
                        });
                    }
                    ProducerEvaluationState::EvaluatedIncomplete => {
                        findings.push(ComponentClosureFinding::IncompleteProducer {
                            subject,
                            family: descriptor.id,
                        });
                    }
                    _ => {}
                }
                let reads = self.transport_reads.get(&subject).and_then(|row| {
                    row.iter()
                        .find(|(family, _)| *family == family_index)
                        .map(|(_, reads)| reads)
                });
                if let Some(reads) = reads {
                    evidence.update([1]);
                    let ordered: BTreeSet<_> = reads.iter().collect();
                    evidence.update((ordered.len() as u64).to_be_bytes());
                    for read in ordered {
                        hash_read(&mut evidence, read);
                    }
                } else {
                    evidence.update([0]);
                    if state == ProducerEvaluationState::EvaluatedComplete {
                        findings.push(ComponentClosureFinding::UnavailableReadEvidence {
                            subject,
                            family: descriptor.id,
                        });
                    }
                }
            }
            for requirement in SemanticClosureRequirement::ALL {
                if self.is_closed(subject, requirement) {
                    closed_requirements += 1;
                } else {
                    findings.push(ComponentClosureFinding::OpenRequirement {
                        subject,
                        requirement,
                    });
                }
            }
        }
        let read_evidence_digest: [u8; 32] = evidence.finalize().into();
        let mut hash = Sha256::new();
        hash.update(b"agq-component-closure-audit/1\0");
        hash.update(self.digest);
        hash.update(read_evidence_digest);
        let digest = hash.finalize().into();
        Ok(ComponentClosureAudit {
            subjects: subjects.iter().copied().collect(),
            source_certificate_digest: self.digest,
            read_evidence_digest,
            digest,
            applicable_pairs,
            closed_pairs,
            closed_requirements,
            findings: findings.into_boxed_slice(),
        })
    }
}

fn hash_bytes(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn hash_read(hash: &mut Sha256, read: &ProducerRead) {
    let id = |hash: &mut Sha256, id: ElementId| hash.update(id.as_u128().to_be_bytes());
    match read {
        ProducerRead::Property(subject, property)
        | ProducerRead::DeclaredProperty(subject, property) => {
            hash.update([if matches!(read, ProducerRead::Property(..)) {
                0
            } else {
                1
            }]);
            id(hash, *subject);
            hash.update(property.as_u128().to_be_bytes());
        }
        ProducerRead::OrderedReferenceContribution(subject, property, target) => {
            hash.update([2]);
            id(hash, *subject);
            hash.update(property.as_u128().to_be_bytes());
            id(hash, *target);
        }
        ProducerRead::Structural(subject) => {
            hash.update([3]);
            id(hash, *subject);
        }
        ProducerRead::Source(subject, class, property) => {
            hash.update([4]);
            id(hash, *subject);
            hash.update(class.as_u128().to_be_bytes());
            hash.update(property.as_u128().to_be_bytes());
        }
        ProducerRead::Owned(subject, class) => {
            hash.update([5]);
            id(hash, *subject);
            hash.update(class.as_u128().to_be_bytes());
        }
        ProducerRead::OwnedExcluding(subject, class, excluded) => {
            hash.update([6]);
            id(hash, *subject);
            hash.update(class.as_u128().to_be_bytes());
            hash.update((excluded.len() as u64).to_be_bytes());
            for class in excluded {
                hash.update(class.as_u128().to_be_bytes());
            }
        }
        ProducerRead::FeaturePopulation(subject, kind) => {
            hash.update([7]);
            id(hash, *subject);
            hash_bytes(hash, kind.contract_id().as_bytes());
        }
        ProducerRead::Inverse => hash.update([8]),
        ProducerRead::Any(subject) => {
            hash.update([9]);
            id(hash, *subject);
        }
        ProducerRead::Global => hash.update([10]),
        ProducerRead::Requirement(subject, requirement) => {
            hash.update([11]);
            id(hash, *subject);
            hash_bytes(hash, requirement.contract_id().as_bytes());
        }
        ProducerRead::Identity(subject) => {
            hash.update([12]);
            id(hash, *subject);
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/component_closure_audit.rs"]
mod tests;
