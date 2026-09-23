//! Versioned, tagged content encoding for a provisional dependency plan.
use super::*;

struct Encoding(Sha256);
impl Encoding {
    fn tag(&mut self, value: u8) {
        self.0.update([value]);
    }
    fn count(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }
    fn id(&mut self, value: ElementId) {
        self.0.update(value.as_u128().to_le_bytes());
    }
    fn text(&mut self, value: &str) {
        self.count(value.len());
        self.0.update(value.as_bytes());
    }
    fn subjects(&mut self, subjects: &BTreeSet<ElementId>) {
        self.count(subjects.len());
        for &subject in subjects {
            self.id(subject);
        }
    }
    fn effect(&mut self, effect: ProducerEffect) {
        use ProducerEffect as E;
        self.tag(match effect {
            E::Membership => 0,
            E::Ownership => 1,
            E::Specialization => 2,
            E::Conjugation => 3,
            E::Subsetting => 4,
            E::Redefinition => 5,
            E::Typing => 6,
            E::Featuring => 7,
            E::Naming => 8,
            E::FeatureChain => 9,
            E::ResultStructure => 10,
            E::ValueBinding => 11,
            E::ConnectorStructure => 12,
            E::Scalar(_) => 13,
        });
        if let E::Scalar(property) = effect {
            self.0.update(property.as_u128().to_le_bytes());
        }
    }
    fn reason(&mut self, reason: &PublicationDependencyReason) {
        use PublicationDependencyReason as R;
        match reason {
            R::CanonicalReference(property) => {
                self.tag(0);
                self.0.update(property.as_u128().to_le_bytes());
            }
            R::SourceImport => self.tag(1),
            R::MandatoryReference => self.tag(2),
            R::StandardTarget => self.tag(3),
            R::ProviderRead => self.tag(4),
            R::Writer {
                family,
                effect,
                future_subject,
            } => {
                self.tag(5);
                self.text(family.name());
                self.effect(*effect);
                self.tag(u8::from(*future_subject));
            }
        }
    }
    fn diagnostic(&mut self, diagnostic: &PublicationPlanDiagnostic) {
        use PublicationPlanDiagnostic as D;
        match diagnostic {
            D::MissingSubject(subject) => {
                self.tag(0);
                self.id(*subject);
            }
            D::MissingCandidateSubject(subject) => {
                self.tag(1);
                self.id(*subject);
            }
            D::ProtectedCandidateSubject(subject) => {
                self.tag(2);
                self.id(*subject);
            }
            D::UnknownProvider { consumer, provider } => {
                self.tag(3);
                self.id(*consumer);
                self.id(*provider);
            }
            D::MissingProviderEvidence(subject) => {
                self.tag(4);
                self.id(*subject);
            }
            D::GlobalProviderRead(subject) => {
                self.tag(5);
                self.id(*subject);
            }
            D::UnboundedWriter { subject, family } => {
                self.tag(6);
                self.id(*subject);
                self.text(family.name());
            }
            D::ProducerRegistryMismatch => self.tag(7),
            D::OpenProviderRequirement {
                subject,
                requirement,
            } => {
                self.tag(8);
                self.id(*subject);
                self.text(requirement.contract_id());
            }
        }
    }
}

pub(super) fn digest(plan: &PublicationDependencyPlan) -> [u8; 32] {
    let mut encoding = Encoding(Sha256::new());
    encoding.text("agq-publication-dependency-plan/1");
    encoding.0.update(plan.model_digest);
    encoding.0.update(plan.context_contract_digest);
    encoding.0.update(plan.producer_registry_digest);
    encoding.count(plan.applicable_pairs);
    encoding.count(plan.components.len());
    for component in &plan.components {
        encoding.subjects(&component.subjects);
        encoding.count(component.dependencies.len());
        for &dependency in &component.dependencies {
            encoding.count(dependency);
        }
    }
    encoding.count(plan.dependencies.len());
    for dependency in &plan.dependencies {
        encoding.id(dependency.consumer);
        encoding.id(dependency.provider);
        encoding.reason(&dependency.reason);
    }
    encoding.count(plan.writers.len());
    for writer in &plan.writers {
        encoding.id(writer.subject);
        encoding.text(writer.family.name());
        encoding.effect(writer.effect);
        encoding.tag(u8::from(writer.future_subject));
        match &writer.targets {
            Some(subjects) => {
                encoding.tag(1);
                encoding.subjects(subjects);
            }
            None => encoding.tag(0),
        }
        encoding.count(writer.global_requirements.len());
        for requirement in &writer.global_requirements {
            encoding.text(requirement.contract_id());
        }
    }
    encoding.count(plan.diagnostics.len());
    for diagnostic in &plan.diagnostics {
        encoding.diagnostic(diagnostic);
    }
    encoding.0.finalize().into()
}
