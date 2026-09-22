//! Checked proof transport; the graph delta is computed here, never trusted from a caller.
use super::*;
use crate::{ContextError, QueryReadSet, SemanticContext};
use agq_kernel::{
    provenance::{Dependency, FactKey, Origin},
    value::Value,
};

/// Outcome of transporting producer evaluations to a reconstructed semantic graph.
#[derive(Clone, Debug)]
pub struct ReboundClosure {
    pub certificate: Arc<ProducerClosureCertificate>,
    pub retained_evaluations: usize,
    pub reopened_evaluations: usize,
    /// Includes changed records, both old/new endpoints and changed pending inputs.
    pub affected_subjects: BTreeSet<ElementId>,
}

impl ProducerClosureCertificate {
    /// Certify only requirements with no possible open writers. No producer has
    /// run: applicable families (including future activation) remain Pending.
    /// A merely immutable, unaccepted overlay is not an accepted dependency.
    pub fn initial(
        context: &SemanticContext<'_>,
        registry: &ProducerRegistry,
    ) -> Result<Self, ContextError> {
        if context.id().producer_registry_digest != Some(registry.digest()) {
            return Err(ContextError::ProducerRegistryIdentityMismatch);
        }
        Ok(Self::issue(
            context.model,
            context.id(),
            registry,
            &ProducerEvaluationTable::default(),
            |id| context.dependency_closure_source(id),
        ))
    }

    /// Revalidate exact old evidence against a new graph and unchanged registry.
    /// Unrelated additions retain evaluations; changes to read subjects, negative
    /// searches, source obligations or upstream producer opportunities reopen them.
    /// This operation never copies an old closed mask into a new graph.
    pub fn rebind(
        &self,
        old: &SemanticContext<'_>,
        new: &SemanticContext<'_>,
        registry: &ProducerRegistry,
    ) -> Result<ReboundClosure, ContextError> {
        self.checkpoint(old)?.rebind(new, registry)
    }

    /// Capture bounded semantic fingerprints before dropping a reconstruction
    /// frontier. Retains no graph records, overlay, indexes or parser state.
    pub fn checkpoint(
        &self,
        old: &SemanticContext<'_>,
    ) -> Result<ProducerClosureCheckpoint, ContextError> {
        if !self.compatible_context(old.id()) {
            return Err(ContextError::ProducerClosureMismatch);
        }
        Ok(ProducerClosureCheckpoint {
            certificate: self.clone(),
            context: old.id().clone(),
            signatures: subject_signatures(old.model),
        })
    }

    /// Identity of semantic closure state, separately from exact graph binding.
    /// Equal values do not establish compatibility: `rebind` checks all reads
    /// and recomputes causal closure before this identity can be compared.
    pub fn semantic_closure_digest(&self) -> [u8; 32] {
        certificate_digest(
            [0; 32],
            self.registry_digest,
            self.context_contract_digest,
            &self.subjects,
            &self.closed,
            &self.states,
        )
    }

    /// Retained scheduler read metadata, outside the compact receipt payload.
    /// This metadata is optional on trusted restoration; it is not acceptance authority.
    pub fn revalidation_storage_bytes(&self) -> usize {
        self.transport_reads
            .values()
            .map(|row| {
                std::mem::size_of_val(row.as_slice())
                    + row
                        .iter()
                        .map(|(_, reads)| std::mem::size_of_val(reads.as_ref()))
                        .sum::<usize>()
            })
            .sum()
    }
}

pub(super) fn read_changed(read: &ProducerRead, affected: &BTreeSet<ElementId>) -> bool {
    match read {
        ProducerRead::Global | ProducerRead::Inverse => !affected.is_empty(),
        ProducerRead::Property(id, _)
        | ProducerRead::DeclaredProperty(id, _)
        | ProducerRead::Structural(id)
        | ProducerRead::Source(id, _, _)
        | ProducerRead::Owned(id, _)
        | ProducerRead::OwnedExcluding(id, _, _)
        | ProducerRead::FeaturePopulation(id, _)
        | ProducerRead::Any(id)
        | ProducerRead::Requirement(id, _) => affected.contains(id),
    }
}

/// Compact proof transport across graph reconstruction, independent of old indexes.
#[derive(Clone, Debug)]
pub struct ProducerClosureCheckpoint {
    certificate: ProducerClosureCertificate,
    context: SemanticContextId,
    signatures: BTreeMap<ElementId, [u8; 32]>,
}
impl ProducerClosureCheckpoint {
    pub fn rebind(
        &self,
        new: &SemanticContext<'_>,
        registry: &ProducerRegistry,
    ) -> Result<ReboundClosure, ContextError> {
        if self.certificate.registry_digest != registry.digest()
            || new.id().producer_registry_digest != Some(registry.digest())
        {
            return Err(ContextError::ProducerClosureMismatch);
        }
        let mut current = new.id().clone();
        current.producer_closure_digest = self.context.producer_closure_digest;
        current.derivation_phase = self.context.derivation_phase;
        if !QueryReadSet::context_compatible(&self.context, &current) {
            return Err(ContextError::ProducerClosureMismatch);
        }
        let signatures = subject_signatures(new.model);
        let mut affected: BTreeSet<_> = self
            .signatures
            .keys()
            .chain(signatures.keys())
            .copied()
            .filter(|subject| self.signatures.get(subject) != signatures.get(subject))
            .collect();
        for (old, new) in [
            (
                &self.context.pending_specialization_scopes,
                &new.id().pending_specialization_scopes,
            ),
            (
                &self.context.pending_namespace_scopes,
                &new.id().pending_namespace_scopes,
            ),
        ] {
            affected.extend(old.symmetric_difference(new).copied());
        }
        affected.extend(
            self.context
                .construction_obligations
                .symmetric_difference(&new.id().construction_obligations)
                .map(|(id, _)| *id),
        );
        let mut table = ProducerEvaluationTable::default();
        let mut retained = 0;
        let mut reopened = 0;
        for record in new.model.elements() {
            let subject = record.id();
            table.pending(subject, new.model, registry);
            for family in 0..registry.descriptors.len() {
                let Some(state) = self.certificate.evaluation(subject, family) else {
                    continue;
                };
                if !matches!(
                    state,
                    ProducerEvaluationState::EvaluatedComplete
                        | ProducerEvaluationState::EvaluatedIncomplete
                ) {
                    continue;
                }
                let reads = self
                    .certificate
                    .transport_reads
                    .get(&subject)
                    .and_then(|row| row.iter().find(|(index, _)| *index == family))
                    .map(|(_, reads)| reads);
                let valid = !affected.contains(&subject)
                    && reads.is_some_and(|reads| {
                        !reads.iter().any(|read| read_changed(read, &affected))
                    });
                if valid {
                    table.rows.get_mut(&subject).expect("initialized subject")[family] = state;
                    table
                        .reads
                        .entry(subject)
                        .or_default()
                        .push((family, reads.expect("validated reads").clone()));
                    retained += 1;
                } else {
                    reopened += 1;
                }
            }
        }
        let certificate =
            ProducerClosureCertificate::issue(new.model, new.id(), registry, &table, |id| {
                new.dependency_closure_source(id)
            });
        Ok(ReboundClosure {
            certificate: Arc::new(certificate),
            retained_evaluations: retained,
            reopened_evaluations: reopened,
            affected_subjects: affected,
        })
    }
    /// Fingerprint payload; shared certificate/read metadata is reported separately.
    pub fn fingerprint_storage_bytes(&self) -> usize {
        self.signatures.len() * (std::mem::size_of::<ElementId>() + 32)
    }
}

fn subject_signatures(model: &ModelView) -> BTreeMap<ElementId, [u8; 32]> {
    // Hash each record once, then include source identities/content at both
    // endpoints. New inverse carriers therefore invalidate an earlier empty search.
    let records: BTreeMap<_, [u8; 32]> = model
        .elements()
        .map(|record| (record.id(), hash_debug(record)))
        .collect();
    let mut hashes: BTreeMap<_, _> = records
        .iter()
        .map(|(&id, digest)| {
            let mut hash = Sha256::new();
            hash.update(digest);
            (id, hash)
        })
        .collect();
    for record in model.elements() {
        output_support(&mut hashes, model, record.origin(), records[&record.id()]);
        for (property, slot) in record.slots() {
            output_support(
                &mut hashes,
                model,
                slot.origin(),
                hash_debug(&(record.id(), property, slot)),
            );
            for (index, value) in slot.value().values().enumerate() {
                if let Value::Reference(target) = value
                    && let Some(hash) = hashes.get_mut(target)
                {
                    hash.update(records[&record.id()]);
                    hash.update(property.as_u128().to_be_bytes());
                    hash.update(index.to_be_bytes());
                }
            }
        }
    }
    for occurrence in model.association_occurrences() {
        let digest = hash_debug(occurrence);
        output_support(&mut hashes, model, occurrence.origin(), digest);
        for target in occurrence.ends().values() {
            if let Some(hash) = hashes.get_mut(target) {
                hash.update(digest);
            }
        }
    }
    for (fact, searches) in model.computation_searches() {
        let digest = hash_debug(&(fact, searches));
        match fact {
            FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                if let Some(hash) = hashes.get_mut(id) {
                    hash.update(digest);
                }
            }
            FactKey::AssociationOccurrence(id) => {
                if let Some(occurrence) = model.association_occurrence(*id) {
                    for target in occurrence.ends().values() {
                        if let Some(hash) = hashes.get_mut(target) {
                            hash.update(digest);
                        }
                    }
                }
            }
        }
    }
    for (&(id, property), value) in model.derived_navigation_results() {
        output_support(
            &mut hashes,
            model,
            value.origin(),
            hash_debug(&(id, property, value)),
        );
        if let Some(hash) = hashes.get_mut(&id) {
            hash.update(hash_debug(&(property, value)));
        }
    }
    for (&(id, property), value) in model.computation_failures() {
        if let Some(hash) = hashes.get_mut(&id) {
            hash.update(hash_debug(&(property, value)));
        }
    }
    hashes
        .into_iter()
        .map(|(id, hash)| (id, hash.finalize().into()))
        .collect()
}
// A lost detached output must reopen its producer even if every input value
// remains equal. Kernel derivations retain the producer subject as support;
// associate each output fingerprint with every immediate canonical input.
fn output_support(
    hashes: &mut BTreeMap<ElementId, Sha256>,
    model: &ModelView,
    origin: &Origin,
    digest: [u8; 32],
) {
    if let Origin::Derived(explanation) = origin {
        for dependency in &explanation.dependencies {
            let (Dependency::Declared(fact) | Dependency::Derived(fact)) = dependency;
            match fact {
                FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                    if let Some(hash) = hashes.get_mut(id) {
                        hash.update(digest);
                    }
                }
                FactKey::AssociationOccurrence(id) => {
                    if let Some(occurrence) = model.association_occurrence(*id) {
                        for target in occurrence.ends().values() {
                            if let Some(hash) = hashes.get_mut(target) {
                                hash.update(digest);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn hash_debug(value: &impl std::fmt::Debug) -> [u8; 32] {
    // A streaming formatter avoids allocating expanded provenance strings.
    struct Writer(Sha256);
    impl std::fmt::Write for Writer {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            self.0.update(text.as_bytes());
            Ok(())
        }
    }
    let mut writer = Writer(Sha256::new());
    std::fmt::write(&mut writer, format_args!("{value:?}")).expect("hash writer cannot fail");
    writer.0.finalize().into()
}
