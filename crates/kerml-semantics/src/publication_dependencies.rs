//! Conservative semantic component planning. A plan is not closure evidence.
//!
//! Edges point from a consumer to a provider. Canonical relationship carriers
//! keep their own identity: referring to another subject is a read dependency,
//! not permission to write that subject. Potential cross-subject writers join
//! their source and target populations. Existing exhaustive-query global guards
//! remain explicit; observed read sets cannot narrow those guards.
use super::*;
use crate::read_dependencies::InvalidationKey;
use crate::{QueryReadSet, SemanticContext};

#[path = "publication_dependency_digest.rs"]
mod encoding;

/// Why one semantic subject depends on another. Source dependencies supplied by
/// a frontend supplement, rather than replace, canonical and producer edges.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationDependencyReason {
    CanonicalReference(PropertyId),
    SourceImport,
    MandatoryReference,
    StandardTarget,
    ProviderRead,
    Writer {
        family: ProducerFamilyId,
        effect: ProducerEffect,
        future_subject: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicationDependency {
    pub consumer: ElementId,
    pub provider: ElementId,
    pub reason: PublicationDependencyReason,
}

/// Potential writes derived from the existing producer descriptor contract.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublicationWriter {
    pub subject: ElementId,
    pub family: ProducerFamilyId,
    pub effect: ProducerEffect,
    pub future_subject: bool,
    /// `None` means an unbounded write population, never an empty population.
    pub targets: Option<BTreeSet<ElementId>>,
    /// The current certificate contract conservatively requires these global
    /// requirements to await this producer. This is separate from record writes.
    pub global_requirements: BTreeSet<SemanticClosureRequirement>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationPlanDiagnostic {
    MissingSubject(ElementId),
    MissingCandidateSubject(ElementId),
    ProtectedCandidateSubject(ElementId),
    UnknownProvider {
        consumer: ElementId,
        provider: ElementId,
    },
    /// Generic query read sets cannot establish complete producer-family read
    /// coverage. Only scheduler-issued evaluation evidence can discharge this.
    MissingProviderEvidence(ElementId),
    GlobalProviderRead(ElementId),
    OpenProviderRequirement {
        subject: ElementId,
        requirement: SemanticClosureRequirement,
    },
    UnboundedWriter {
        subject: ElementId,
        family: ProducerFamilyId,
    },
    ProducerRegistryMismatch,
}

/// A deterministic SCC. Its index is stable under input iteration permutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationComponent {
    subjects: BTreeSet<ElementId>,
    dependencies: BTreeSet<usize>,
}
impl PublicationComponent {
    pub fn subjects(&self) -> &BTreeSet<ElementId> {
        &self.subjects
    }
    /// Component indices, always smaller than this component's index.
    pub fn dependencies(&self) -> &BTreeSet<usize> {
        &self.dependencies
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicationPartitionError {
    SubjectPopulation,
    SplitSemanticCycle { component: usize },
    ProviderAfterConsumer { consumer: usize, provider: usize },
}

/// Provisional dependency analysis, deliberately unable to issue certificates.
/// The source/reference inventory and observed provider reads remain inputs for
/// inspection. A scheduler must independently authenticate all local evidence,
/// future writer exclusion and unchanged context before sealing any component.
#[derive(Clone, Debug)]
pub struct PublicationDependencyPlan {
    components: Vec<PublicationComponent>,
    positions: BTreeMap<ElementId, usize>,
    dependencies: BTreeSet<PublicationDependency>,
    writers: Vec<PublicationWriter>,
    diagnostics: BTreeSet<PublicationPlanDiagnostic>,
    applicable_pairs: usize,
    model_digest: [u8; 32],
    context_contract_digest: [u8; 32],
    producer_registry_digest: [u8; 32],
    digest: [u8; 32],
}
impl PublicationDependencyPlan {
    pub fn build(
        context: &SemanticContext<'_>,
        registry: &ProducerRegistry,
        subjects: BTreeSet<ElementId>,
        dependencies: impl IntoIterator<Item = PublicationDependency>,
        provider_reads: impl IntoIterator<Item = (ElementId, QueryReadSet)>,
    ) -> Self {
        let model = context.model();
        let ids: Vec<_> = subjects.iter().copied().collect();
        let positions: BTreeMap<_, _> = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        // Two virtual hubs encode all-subject reads and global requirement
        // providers in O(subjects + writers) edges, avoiding a quadratic graph.
        let global_reads = ids.len();
        let global_writers = ids.len() + 1;
        let mut adjacency = vec![BTreeSet::new(); ids.len() + 2];
        let mut diagnostics = BTreeSet::new();
        let mut dependencies: BTreeSet<_> = dependencies.into_iter().collect();
        let mut writers = Vec::new();
        let mut applicable_pairs = 0;
        // Physical immutability alone does not establish a closed semantic
        // provider. Reuse the certificate's authenticated dependency boundary.
        let protected = |id| context.dependency_closure_source(id).is_some();
        if context.id.producer_registry_digest != Some(registry.digest()) {
            diagnostics.insert(PublicationPlanDiagnostic::ProducerRegistryMismatch);
        }
        for record in model.elements() {
            if !subjects.contains(&record.id()) && !protected(record.id()) {
                diagnostics.insert(PublicationPlanDiagnostic::MissingCandidateSubject(
                    record.id(),
                ));
            }
        }
        for &subject in &ids {
            if protected(subject) {
                diagnostics.insert(PublicationPlanDiagnostic::ProtectedCandidateSubject(
                    subject,
                ));
            }
            if model.element(subject).is_none() {
                diagnostics.insert(PublicationPlanDiagnostic::MissingSubject(subject));
            }
            for reference in model.outgoing(subject) {
                dependencies.insert(PublicationDependency {
                    consumer: subject,
                    provider: reference.target,
                    reason: PublicationDependencyReason::CanonicalReference(reference.property),
                });
            }
            adjacency[global_reads].insert(positions[&subject]);
        }
        for (consumer, reads) in provider_reads {
            for key in &reads.keys {
                match key {
                    InvalidationKey::Element(provider) => {
                        dependencies.insert(PublicationDependency {
                            consumer,
                            provider: *provider,
                            reason: PublicationDependencyReason::ProviderRead,
                        });
                    }
                    // An inverse search can observe local carriers referring to
                    // an immutable standard. The referenced ID being immutable
                    // does not make that population immutable.
                    InvalidationKey::Incoming(_) | InvalidationKey::Global => {
                        diagnostics.insert(PublicationPlanDiagnostic::GlobalProviderRead(consumer));
                        if let Some(&i) = positions.get(&consumer) {
                            adjacency[i].insert(global_reads);
                        }
                    }
                }
            }
        }
        let ownership_mutable = registry.descriptors.iter().any(|descriptor| {
            descriptor.applicability != ProducerApplicability::Never
                && (descriptor.effects.contains(&ProducerEffect::Ownership)
                    || has_reference_scalar(descriptor, model))
        });
        let direction_mutable = registry.descriptors.iter().any(|descriptor| {
            descriptor.applicability != ProducerApplicability::Never
                && descriptor.effects.iter().any(|effect| matches!(effect, ProducerEffect::Scalar(property)
                    if scalar_changes_feature_population(*property, crate::FeaturePopulationKind::Parameter, model)))
        });
        let provider_masks =
            pending_provider_masks(model, &context.id, &ids, &positions, &protected);
        let provider_ownership_open = provider_masks
            .iter()
            .any(|mask| mask & SemanticClosureRequirement::EffectiveOwnership.bit() != 0);
        for (i, &mask) in provider_masks.iter().enumerate() {
            for requirement in SemanticClosureRequirement::ALL {
                if mask & requirement.bit() != 0 {
                    diagnostics.insert(PublicationPlanDiagnostic::OpenProviderRequirement {
                        subject: ids[i],
                        requirement,
                    });
                    adjacency[i].insert(global_reads);
                }
            }
        }
        let mut has_global_writers = false;
        for &subject in &ids {
            let Some(record) = model.element(subject) else {
                continue;
            };
            for descriptor in registry.descriptors() {
                if !descriptor.applicability.applies(model, record.metaclass()) {
                    continue;
                }
                applicable_pairs += 1;
                // Source-reference query reads are useful dependencies, but
                // cannot substitute for this family's unobserved evaluations.
                diagnostics.insert(PublicationPlanDiagnostic::MissingProviderEvidence(subject));
                adjacency[positions[&subject]].insert(global_reads);
                for &effect in &descriptor.effects {
                    let targets = existing_targets(
                        model,
                        descriptor,
                        subject,
                        effect,
                        ownership_mutable,
                        direction_mutable,
                    );
                    let requirements = global_requirements(effect, model);
                    if !requirements.is_empty() {
                        has_global_writers = true;
                        adjacency[global_writers].insert(positions[&subject]);
                    }
                    writers.push(PublicationWriter {
                        subject,
                        family: descriptor.id,
                        effect,
                        future_subject: false,
                        targets,
                        global_requirements: requirements,
                    });
                }
                if descriptor.can_create_subjects() {
                    for future in future_cross_subject_families(registry, model) {
                        for &effect in &future.effects {
                            if !effect_reaches_future_existing_subjects(
                                future,
                                effect,
                                ownership_mutable,
                                model,
                            ) {
                                continue;
                            }
                            let targets = future_owner_targets(
                                model,
                                registry,
                                subject,
                                descriptor,
                                provider_ownership_open,
                                future.effect_scope(effect)
                                    != ProducerEffectScope::SubjectAndOwningType,
                            );
                            let requirements = global_requirements(effect, model);
                            if !requirements.is_empty() {
                                has_global_writers = true;
                                adjacency[global_writers].insert(positions[&subject]);
                            }
                            writers.push(PublicationWriter {
                                subject,
                                family: future.id,
                                effect,
                                future_subject: true,
                                targets,
                                global_requirements: requirements,
                            });
                        }
                    }
                }
            }
        }
        writers.sort();
        writers.dedup();
        if has_global_writers {
            for neighbors in adjacency.iter_mut().take(ids.len()) {
                neighbors.insert(global_writers);
            }
        }
        for writer in &writers {
            let targets = match &writer.targets {
                Some(targets) => targets,
                None => {
                    diagnostics.insert(PublicationPlanDiagnostic::UnboundedWriter {
                        subject: writer.subject,
                        family: writer.family,
                    });
                    // Unbounded writes require joint closure, not just an order.
                    adjacency[positions[&writer.subject]].insert(global_reads);
                    has_global_writers = true;
                    adjacency[global_writers].insert(positions[&writer.subject]);
                    continue;
                }
            };
            for &target in targets {
                if target == writer.subject || !subjects.contains(&target) {
                    continue;
                }
                let reason = PublicationDependencyReason::Writer {
                    family: writer.family,
                    effect: writer.effect,
                    future_subject: writer.future_subject,
                };
                for (consumer, provider) in [(target, writer.subject), (writer.subject, target)] {
                    dependencies.insert(PublicationDependency {
                        consumer,
                        provider,
                        reason: reason.clone(),
                    });
                }
            }
        }
        if has_global_writers {
            for neighbors in adjacency.iter_mut().take(ids.len()) {
                neighbors.insert(global_writers);
            }
        }
        for edge in &dependencies {
            match (positions.get(&edge.consumer), positions.get(&edge.provider)) {
                (Some(&from), Some(&to)) => {
                    adjacency[from].insert(to);
                }
                (Some(_), None) if protected(edge.provider) => {}
                (Some(&from), None) => {
                    diagnostics.insert(PublicationPlanDiagnostic::UnknownProvider {
                        consumer: edge.consumer,
                        provider: edge.provider,
                    });
                    adjacency[from].insert(global_reads);
                }
                (None, _) => {
                    diagnostics.insert(PublicationPlanDiagnostic::MissingSubject(edge.consumer));
                }
            }
        }
        let components = components(&ids, &adjacency);
        let positions = components
            .iter()
            .enumerate()
            .flat_map(|(i, component)| component.subjects.iter().map(move |&id| (id, i)))
            .collect();
        let model_digest = context.id.model_digest;
        let context_contract_digest = context.id.closure_contract_digest();
        let producer_registry_digest = registry.digest();
        let mut plan = Self {
            components,
            positions,
            dependencies,
            writers,
            diagnostics,
            applicable_pairs,
            model_digest,
            context_contract_digest,
            producer_registry_digest,
            digest: [0; 32],
        };
        plan.digest = encoding::digest(&plan);
        plan
    }
    pub fn components(&self) -> &[PublicationComponent] {
        &self.components
    }
    pub fn component_of(&self, subject: ElementId) -> Option<usize> {
        self.positions.get(&subject).copied()
    }
    pub fn dependencies(&self) -> &BTreeSet<PublicationDependency> {
        &self.dependencies
    }
    pub fn writers(&self) -> &[PublicationWriter] {
        &self.writers
    }
    pub fn diagnostics(&self) -> &BTreeSet<PublicationPlanDiagnostic> {
        &self.diagnostics
    }
    pub fn applicable_pairs(&self) -> usize {
        self.applicable_pairs
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn model_digest(&self) -> [u8; 32] {
        self.model_digest
    }
    pub fn context_contract_digest(&self) -> [u8; 32] {
        self.context_contract_digest
    }
    pub fn producer_registry_digest(&self) -> [u8; 32] {
        self.producer_registry_digest
    }

    /// Reject a proposed publication order that splits a known semantic cycle
    /// or schedules a provider after its consumer. Success establishes only
    /// consistency with this provisional plan, never semantic completeness.
    pub fn validate_partition(
        &self,
        strata: &[BTreeSet<ElementId>],
    ) -> Result<(), PublicationPartitionError> {
        let mut positions = BTreeMap::new();
        for (i, subjects) in strata.iter().enumerate() {
            if subjects.is_empty() {
                return Err(PublicationPartitionError::SubjectPopulation);
            }
            for &subject in subjects {
                if positions.insert(subject, i).is_some() {
                    return Err(PublicationPartitionError::SubjectPopulation);
                }
            }
        }
        if positions.keys().ne(self.positions.keys()) {
            return Err(PublicationPartitionError::SubjectPopulation);
        }
        let mut component_positions = Vec::new();
        for (i, component) in self.components.iter().enumerate() {
            let position = positions[component.subjects.first().expect("nonempty component")];
            if component
                .subjects
                .iter()
                .any(|subject| positions[subject] != position)
            {
                return Err(PublicationPartitionError::SplitSemanticCycle { component: i });
            }
            component_positions.push(position);
            for &dependency in &component.dependencies {
                if component_positions[dependency] > position {
                    return Err(PublicationPartitionError::ProviderAfterConsumer {
                        consumer: i,
                        provider: dependency,
                    });
                }
            }
        }
        Ok(())
    }
}

fn global_requirements(
    effect: ProducerEffect,
    model: &ModelView,
) -> BTreeSet<SemanticClosureRequirement> {
    SemanticClosureRequirement::ALL
        .into_iter()
        .filter(|requirement| {
            *requirement != SemanticClosureRequirement::EffectiveTyping
                && requirement.requires_in_model(effect, model)
        })
        .collect()
}

fn existing_targets(
    model: &ModelView,
    descriptor: &ProducerDescriptor,
    subject: ElementId,
    effect: ProducerEffect,
    ownership_mutable: bool,
    direction_mutable: bool,
) -> Option<BTreeSet<ElementId>> {
    let scope = descriptor.effect_scope(effect);
    if scope == ProducerEffectScope::Model
        || reference_scalar(effect, model)
        || ownership_mutable && scope.depends_on_ownership()
    {
        return None;
    }
    let targets = if let Some(targets) = scope.selected_targets(model, subject, direction_mutable) {
        targets.ok()?
    } else {
        let mut targets = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut pending = vec![subject];
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            if scope != ProducerEffectScope::OwnedDescendants || current != subject {
                targets.insert(current);
            }
            for property in [
                agq_kerml::properties::ELEMENT_OWNED_RELATIONSHIP,
                agq_kerml::properties::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ] {
                if matches!(
                    scope,
                    ProducerEffectScope::SubjectAndOwned | ProducerEffectScope::OwnedDescendants
                ) {
                    match model.property_state(current, property) {
                        Ok(agq_kernel::derived::PropertyState::Computed(slot)) => {
                            for value in slot.value().values() {
                                let agq_kernel::value::Value::Reference(child) = value else {
                                    return None;
                                };
                                pending.push(*child);
                            }
                        }
                        Ok(agq_kernel::derived::PropertyState::Absent) => {}
                        Err(_)
                            if model.element(current).is_some_and(|record| {
                                model
                                    .registry()
                                    .resolve_property(record.metaclass(), property)
                                    .is_ok_and(|value| value.is_none())
                            }) => {}
                        _ => return None,
                    }
                } else if scope == ProducerEffectScope::SubjectAndOwners {
                    pending.extend(
                        model
                            .incoming_for_property(current, property)
                            .map(|incoming| incoming.source),
                    );
                }
            }
        }
        targets
    };
    Some(
        targets
            .into_iter()
            .filter(|&target| descriptor.affects_subject(model, target))
            .collect(),
    )
}

/// Iterative Kosaraju avoids stack exhaustion on deeply nested source models.
fn components(ids: &[ElementId], adjacency: &[BTreeSet<usize>]) -> Vec<PublicationComponent> {
    let mut reverse = vec![Vec::new(); adjacency.len()];
    for (from, targets) in adjacency.iter().enumerate() {
        for &to in targets {
            reverse[to].push(from);
        }
    }
    let mut seen = vec![false; adjacency.len()];
    let mut finished = Vec::new();
    for root in 0..adjacency.len() {
        let mut pending = vec![(root, false)];
        while let Some((node, exiting)) = pending.pop() {
            if exiting {
                finished.push(node);
                continue;
            }
            if std::mem::replace(&mut seen[node], true) {
                continue;
            }
            pending.push((node, true));
            pending.extend(adjacency[node].iter().rev().map(|&next| (next, false)));
        }
    }
    let mut assignment = vec![usize::MAX; adjacency.len()];
    let mut populations = Vec::<BTreeSet<ElementId>>::new();
    for &root in finished.iter().rev() {
        if assignment[root] != usize::MAX {
            continue;
        }
        let component = populations.len();
        let mut subjects = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if assignment[node] != usize::MAX {
                continue;
            }
            assignment[node] = component;
            if let Some(&id) = ids.get(node) {
                subjects.insert(id);
            }
            pending.extend(reverse[node].iter().copied());
        }
        populations.push(subjects);
    }
    let mut dependencies = vec![BTreeSet::new(); populations.len()];
    for (from, targets) in adjacency.iter().enumerate() {
        for &to in targets {
            if assignment[from] != assignment[to] {
                dependencies[assignment[from]].insert(assignment[to]);
            }
        }
    }
    // Empty hub components still forward their dependencies; they are not
    // semantic subjects and must never appear in a certificate subject set.
    for i in 0..populations.len() {
        let mut pending: Vec<_> = dependencies[i].iter().copied().collect();
        let mut visited = BTreeSet::new();
        while let Some(dependency) = pending.pop() {
            if !visited.insert(dependency) {
                continue;
            }
            if populations[dependency].is_empty() {
                pending.extend(dependencies[dependency].iter().copied());
            } else {
                dependencies[i].insert(dependency);
            }
        }
        dependencies[i].retain(|&dependency| !populations[dependency].is_empty());
    }
    let mut outstanding: Vec<_> = dependencies.iter().map(BTreeSet::len).collect();
    let mut consumers = vec![Vec::new(); populations.len()];
    for (consumer, providers) in dependencies.iter().enumerate() {
        if populations[consumer].is_empty() {
            continue;
        }
        for &provider in providers {
            consumers[provider].push(consumer);
        }
    }
    let mut ready: BTreeSet<_> = populations
        .iter()
        .enumerate()
        .filter(|(i, subjects)| !subjects.is_empty() && outstanding[*i] == 0)
        .map(|(i, subjects)| (*subjects.first().unwrap(), i))
        .collect();
    let mut order = BTreeMap::new();
    let mut result = Vec::new();
    while let Some((_, next)) = ready.pop_first() {
        order.insert(next, result.len());
        result.push(PublicationComponent {
            subjects: std::mem::take(&mut populations[next]),
            dependencies: dependencies[next]
                .iter()
                .map(|dependency| order[dependency])
                .collect(),
        });
        for &consumer in &consumers[next] {
            outstanding[consumer] -= 1;
            if outstanding[consumer] == 0 {
                ready.insert((*populations[consumer].first().unwrap(), consumer));
            }
        }
    }
    result
}

#[cfg(test)]
#[path = "../tests/unit/publication_dependencies.rs"]
mod tests;
