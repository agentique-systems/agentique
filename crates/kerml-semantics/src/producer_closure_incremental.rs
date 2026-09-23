//! Exact delta maintenance of scheduler certificate inputs.
//!
//! This cache is not acceptance authority. Conservative causal and requirement
//! fixed points still run over the complete population. Graph-local topology and
//! unchanged producer scope rows are reused; broad read scopes invalidate broadly.
use super::*;

/// Mutable, scheduler-local acceleration. Never serialized as trusted authority.
#[derive(Debug, Default)]
pub(crate) struct ClosureCertificateBuilder {
    cache: CertificateUpdateCache,
    registry: Option<[u8; 32]>,
    contract: Option<[u8; 32]>,
    model: Option<[u8; 32]>,
}
impl ClosureCertificateBuilder {
    /// `affected` comes from the validated scheduler plan: changed subjects and
    /// both relationship endpoints, including vanished construction obligations.
    /// This is deliberately not a public caller-supplied invalidation boundary.
    pub(crate) fn issue(
        &mut self,
        model: &ModelView,
        context: &SemanticContextId,
        registry: &ProducerRegistry,
        table: &ProducerEvaluationTable,
        affected: &BTreeSet<ElementId>,
        immutable_source: impl Fn(ElementId) -> Option<ClosureSource>,
    ) -> ProducerClosureCertificate {
        let contract = context.closure_contract_digest();
        if self.registry != Some(registry.digest())
            || self.contract != Some(contract)
            || affected.is_empty() && self.model != Some(context.model_digest)
        {
            self.cache = CertificateUpdateCache::default();
        }
        self.registry = Some(registry.digest());
        self.contract = Some(contract);
        self.model = Some(context.model_digest);
        self.cache.invalidate(model, affected);
        let before = (self.cache.topology_rebuilt, self.cache.scopes_rebuilt);
        let certificate = ProducerClosureCertificate::issue_with_cache(
            model,
            context,
            registry,
            table,
            &immutable_source,
            Some(&mut self.cache),
        );
        if std::env::var_os("AGQ_CERTIFICATE_TRACE").is_some() {
            eprintln!(
                "certificate delta: topology rows rebuilt={} retained={}, scope rows rebuilt={} retained={}",
                self.cache.topology_rebuilt - before.0,
                self.cache.topology.len() - (self.cache.topology_rebuilt - before.0),
                self.cache.scopes_rebuilt - before.1,
                self.cache.scopes.len() - (self.cache.scopes_rebuilt - before.1),
            );
        }
        if std::env::var_os("AGQ_CERTIFICATE_VERIFY_FULL_REBUILD").is_some() {
            let full = ProducerClosureCertificate::issue(
                model,
                context,
                registry,
                table,
                immutable_source,
            );
            certificate.assert_exact(&full);
        }
        certificate
    }
}

impl ProducerClosureCertificate {
    /// Include transport reads: equal compact receipts alone do not prove equal
    /// negative/provider evidence for subsequent frontier revalidation.
    pub(crate) fn assert_exact(&self, full: &Self) {
        assert_eq!(
            self.model_digest, full.model_digest,
            "certificate graph binding"
        );
        assert_eq!(
            self.registry_digest, full.registry_digest,
            "certificate registry"
        );
        assert_eq!(
            self.context_contract_digest, full.context_contract_digest,
            "certificate contract"
        );
        assert!(
            self.subjects == full.subjects,
            "certificate subject population"
        );
        assert!(
            self.closed == full.closed,
            "certificate requirement/source rows"
        );
        assert!(self.states == full.states, "certificate producer pair rows");
        assert!(
            self.transport_reads == full.transport_reads,
            "certificate provider/search evidence"
        );
        assert_eq!(self.digest, full.digest, "certificate semantic digest");
        assert_eq!(self.families, full.families);
        assert_eq!(
            self.semantic_closure_digest(),
            full.semantic_closure_digest()
        );
        assert_eq!(self.applicable_pairs, full.applicable_pairs);
        assert_eq!(self.closed_pairs, full.closed_pairs);
        assert_eq!(self.incomplete_pairs, full.incomplete_pairs);
    }
}

#[derive(Debug, Default)]
pub(super) struct CertificateUpdateCache {
    topology: BTreeMap<ElementId, Arc<CertificateTopologyRow>>,
    scopes: BTreeMap<ElementId, CachedScopeRow>,
    topology_rebuilt: usize,
    scopes_rebuilt: usize,
}
impl CertificateUpdateCache {
    fn invalidate(&mut self, model: &ModelView, affected: &BTreeSet<ElementId>) {
        self.topology.retain(|subject, row| {
            model.element(*subject).is_some() && row.footprint.is_disjoint(affected)
        });
        self.scopes.retain(|subject, row| {
            model.element(*subject).is_some()
                && !affected.contains(subject)
                && (affected.is_empty() || !row.graph_sensitive)
        });
    }

    pub(super) fn topology_row(
        &mut self,
        model: &ModelView,
        context: &SemanticContextId,
        subject: ElementId,
    ) -> Arc<CertificateTopologyRow> {
        self.topology
            .entry(subject)
            .or_insert_with(|| {
                self.topology_rebuilt += 1;
                Arc::new(CertificateTopologyRow::build(model, context, subject))
            })
            .clone()
    }

    pub(super) fn scope_row(
        &mut self,
        subject: ElementId,
        states: Vec<u8>,
        context: &CertificateScopeContext<'_>,
        immutable: &impl Fn(ElementId) -> bool,
    ) -> CertificateScopeRow {
        if let Some(row) = self.scopes.get(&subject)
            && row.states == states
            && row.provider_ownership_open == context.provider_ownership_open
        {
            return row.value.clone();
        }
        self.scopes_rebuilt += 1;
        let value = context.build(subject, &states, immutable);
        let graph_sensitive =
            states
                .iter()
                .zip(context.registry.descriptors())
                .any(|(&state, descriptor)| {
                    matches!(state, 1 | 3)
                        && (descriptor.can_create_subjects()
                            || descriptor.effects.iter().any(|&effect| {
                                descriptor.effect_scope(effect) != ProducerEffectScope::Subject
                            }))
                });
        self.scopes.insert(
            subject,
            CachedScopeRow {
                states,
                provider_ownership_open: context.provider_ownership_open,
                graph_sensitive,
                value: value.clone(),
            },
        );
        value
    }
}

#[derive(Debug)]
struct CachedScopeRow {
    states: Vec<u8>,
    provider_ownership_open: bool,
    graph_sensitive: bool,
    value: CertificateScopeRow,
}

#[derive(Debug, Default, Clone)]
pub(super) struct CertificateScopeRow {
    pub global: u8,
    pub dependency_global: u8,
    pub inherited: u8,
    pub descendants: u8,
    pub owners: u8,
    pub targets: BTreeMap<ElementId, u8>,
}

pub(super) struct CertificateScopeContext<'a> {
    pub model: &'a ModelView,
    pub registry: &'a ProducerRegistry,
    pub future_effects: &'a BTreeSet<ProducerEffect>,
    pub future_transitive_requirements: u8,
    pub ownership_mutable: bool,
    pub provider_ownership_open: bool,
    pub direction_mutable: bool,
}
impl CertificateScopeContext<'_> {
    pub(super) fn build(
        &self,
        subject: ElementId,
        states: &[u8],
        immutable: &impl Fn(ElementId) -> bool,
    ) -> CertificateScopeRow {
        let Self {
            model,
            registry,
            future_effects,
            future_transitive_requirements,
            ownership_mutable,
            provider_ownership_open,
            direction_mutable,
        } = *self;
        let mut row = CertificateScopeRow::default();
        for (&state, descriptor) in states.iter().zip(registry.descriptors()) {
            if matches!(state, 1 | 3) {
                if descriptor.can_create_subjects() {
                    let future_targets = future_owner_targets(
                        model,
                        registry,
                        subject,
                        descriptor,
                        provider_ownership_open,
                        true,
                    );
                    let future_direct_targets = future_owner_targets(
                        model,
                        registry,
                        subject,
                        descriptor,
                        provider_ownership_open,
                        false,
                    );
                    for requirement in SemanticClosureRequirement::ALL {
                        if future_effects
                            .iter()
                            .any(|&effect| requirement.requires_in_model(effect, model))
                        {
                            let targets = if future_transitive_requirements & requirement.bit() != 0
                            {
                                &future_targets
                            } else {
                                &future_direct_targets
                            };
                            if let Some(targets) = targets {
                                for target in targets {
                                    if !immutable(*target) && model.element(*target).is_some() {
                                        let subjects_target = *target;
                                        *row.targets.entry(subjects_target).or_default() |=
                                            requirement.bit();
                                    }
                                }
                            } else {
                                row.global |= requirement.bit();
                            }
                            if future_cross_subject_families(registry, model).any(|family| {
                                family.effects.iter().any(|&effect| {
                                    let scope = family.effect_scope(effect);
                                    (scope == ProducerEffectScope::Model
                                        || reference_scalar(effect, model)
                                        || ownership_mutable && scope.depends_on_ownership())
                                        && requirement.requires_in_model(effect, model)
                                })
                            }) {
                                row.dependency_global |= requirement.bit();
                            }
                        }
                    }
                }
                for &effect in &descriptor.effects {
                    let scope = descriptor.effect_scope(effect);
                    let mask = SemanticClosureRequirement::ALL
                        .into_iter()
                        .filter(|r| r.requires_in_model(effect, model))
                        .fold(0, |mask, r| mask | r.bit());
                    // Other query families have additional domain/member dependencies.
                    // Until their precise footprint traversal is implemented,
                    // a relevant unfinished producer conservatively blocks them
                    // throughout the graph instead of claiming local absence.
                    row.global |= mask & !SemanticClosureRequirement::EffectiveTyping.bit();
                    if scope == ProducerEffectScope::Model
                        || reference_scalar(effect, model)
                        || ownership_mutable && scope.depends_on_ownership()
                    {
                        row.dependency_global |= mask;
                    }
                    match scope {
                        _ if reference_scalar(effect, model) => row.global |= mask,
                        scope if ownership_mutable && scope.depends_on_ownership() => {
                            row.global |= mask
                        }
                        ProducerEffectScope::Model => row.global |= mask,
                        ProducerEffectScope::SubjectAndOwned => row.inherited |= mask,
                        ProducerEffectScope::OwnedDescendants => row.descendants |= mask,
                        ProducerEffectScope::OwnedParameterFeatures
                        | ProducerEffectScope::SubjectAndOwnedResults
                        | ProducerEffectScope::SubjectAndOwnedFeatures
                        | ProducerEffectScope::SubjectAndOwningType => {
                            match scope
                                .selected_targets(model, subject, direction_mutable)
                                .expect("selected scope")
                            {
                                Ok(targets) => {
                                    for target in targets {
                                        if model.element(target).is_some() {
                                            *row.targets.entry(target).or_default() |= mask;
                                        }
                                    }
                                }
                                Err(()) => {
                                    row.global |= mask;
                                }
                            }
                        }
                        ProducerEffectScope::SubjectAndOwners => row.owners |= mask,
                        ProducerEffectScope::Subject => {
                            *row.targets.entry(subject).or_default() |= mask
                        }
                    }
                }
            }
        }
        row
    }
}

/// One graph row and the exact subject reads used to construct it. Relationship
/// endpoints and chain terminals join the footprint, including absent searches.
#[derive(Debug, Default)]
pub(super) struct CertificateTopologyRow {
    footprint: BTreeSet<ElementId>,
    pub owned: Vec<ElementId>,
    /// Target, dependent, edge mask (1 = general requirements, 2 = typing).
    pub dependencies: Vec<(ElementId, ElementId, u8)>,
    pub invalid_chain: bool,
}
impl CertificateTopologyRow {
    pub(super) fn build(
        model: &ModelView,
        context: &SemanticContextId,
        subject: ElementId,
    ) -> Self {
        use agq_kerml::{classes as c, properties as p};
        let record = model.element(subject).expect("certificate subject");
        let is = |base| {
            model
                .registry()
                .is_subtype(record.metaclass(), base)
                .unwrap_or(false)
        };
        let mut row = Self {
            footprint: BTreeSet::from([subject]),
            ..Self::default()
        };
        for property in [
            p::ELEMENT_OWNED_RELATIONSHIP,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
        ] {
            row.owned.extend(refs(model, subject, property));
        }
        let endpoints = if is(c::SPECIALIZATION) {
            Some((p::SPECIALIZATION_SPECIFIC, p::SPECIALIZATION_GENERAL))
        } else if is(c::FEATURE_CHAINING) {
            Some((
                p::FEATURE_CHAINING_FEATURE_CHAINED,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
            ))
        } else if is(c::CONJUGATION) {
            Some((p::CONJUGATION_CONJUGATED_TYPE, p::CONJUGATION_ORIGINAL_TYPE))
        } else {
            None
        };
        if let Some((source, target)) = endpoints {
            let mut sources = refs(model, subject, source);
            if sources.is_empty() && (is(c::REFERENCE_SUBSETTING) || is(c::FEATURE_CHAINING)) {
                sources.extend(
                    model
                        .incoming_for_property(subject, p::ELEMENT_OWNED_RELATIONSHIP)
                        .map(|r| r.source),
                );
            }
            for source in sources {
                for target in refs(model, subject, target) {
                    row.dependencies.push((
                        target,
                        source,
                        if is(c::FEATURE_CHAINING) { 1 } else { 3 },
                    ));
                }
            }
        }
        for property in [p::FEATURE_TYPE, p::FEATURE_CHAINING_FEATURE] {
            for target in refs(model, subject, property) {
                row.dependencies.push((
                    target,
                    subject,
                    if property == p::FEATURE_CHAINING_FEATURE {
                        1
                    } else {
                        3
                    },
                ));
            }
        }
        if is(c::FEATURE) {
            // canonical_chain_terminal reads each owned carrier and checks the
            // target's metaclass. Replacing either invalidates this cached row.
            for &owned in &row.owned {
                row.footprint.insert(owned);
                row.footprint
                    .extend(refs(model, owned, p::FEATURE_CHAINING_CHAINING_FEATURE));
            }
            match canonical_chain_terminal(model, subject) {
                Ok(Some(terminal)) => row.dependencies.push((terminal, subject, 2)),
                Ok(None) => {}
                Err(()) => row.invalid_chain = true,
            }
        }
        if let Some(bindings) = &context.standard_bindings {
            for (class, role) in crate::implicit::metaclass_library_role_specs() {
                if is(class) {
                    row.dependencies.push((bindings.get(role), subject, 3));
                }
            }
        }
        row
    }
}

fn refs(model: &ModelView, subject: ElementId, property: PropertyId) -> Vec<ElementId> {
    let property = model
        .element(subject)
        .and_then(|record| {
            model
                .registry()
                .resolve_property(record.metaclass(), property)
                .ok()
                .flatten()
        })
        .map_or(property, |descriptor| descriptor.id);
    model
        .navigation_slot(subject, property)
        .map(|slot| {
            slot.value()
                .values()
                .filter_map(|value| {
                    if let agq_kernel::value::Value::Reference(id) = value {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../tests/unit/producer_closure_incremental.rs"]
mod tests;
