//! Revision-bound semantic results with explicit evidence. No rule evaluator is built in.
use crate::association::AssociationOccurrence;
use crate::model::{DerivationInput, Slot};
use crate::provenance::{Dependency, Explanation, ExplanationPool, FactKey, Origin};
use crate::value::SlotValue;
use crate::{
    AssociationId, AssociationOccurrenceId, DerivationKey, ElementId, ElementRecord, MetaclassId,
    ModelError, ModelView, PropertyId, RevisionId, Snapshot,
};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

mod archive_restore;
mod construction;
pub use construction::{ConstructionDerivationBuilder, ConstructionOverlay};
mod proof_graph;
mod search_sets;
use proof_graph::cyclic_explanations;
pub use search_sets::{StructuralSearchPool, StructuralSearchPoolStatistics};

/// Validated, immutable derived results and a merged read-only semantic view.
///
/// The declared snapshot remains separately available. Every derived record/slot
/// carries an explanation. There is deliberately no API to rebase this overlay:
/// changing declared inputs requires rebuilding it (conservative invalidation).
#[derive(Clone, Debug)]
pub struct DerivedOverlay {
    inner: Arc<OverlayData>,
}
#[derive(Debug)]
struct OverlayData {
    declared: DerivationInput,
    model: ModelView,
    obligations: Vec<crate::ConstructionObligation>,
    explanations: BTreeMap<FactKey, Arc<Explanation>>,
    evidence_pool: ExplanationPool,
    search_pool: StructuralSearchPool,
    build_metrics: DerivationBuildMetrics,
}

/// Work performed by the latest additive overlay materialization. These counters
/// describe implementation work, never semantic completeness or acceptance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DerivationBuildMetrics {
    pub new_elements: usize,
    pub new_association_occurrences: usize,
    pub existing_facts_reused: usize,
    pub proof_sets_interned: usize,
    pub proof_sets_reused: usize,
    pub dependency_edges_considered: usize,
    /// Logical per-fact populations after this stage, including shared entries.
    pub logical_search_sets: usize,
    pub logical_search_entries: usize,
    /// Distinct allocations retained by the interning pool, including earlier
    /// union inputs. These are storage counters, not semantic work counts.
    pub retained_search_sets: usize,
    pub retained_search_entries: usize,
    pub search_sets_interned: usize,
    pub search_sets_reused: usize,
    pub full_model_validations: usize,
    /// The earlier overlay had no remaining readers, so its maps were moved.
    pub reused_owned_storage: bool,
}
impl DerivedOverlay {
    /// Original declared revision, without any inferred slots or elements.
    pub fn declared(&self) -> &Snapshot {
        self.inner.declared.strict()
    }
    /// Declared input revision. This is not an identity for the overlay: different
    /// rule sets/results can be built over the same revision.
    pub fn base_revision(&self) -> RevisionId {
        self.inner.declared.revision()
    }
    /// Declared and derived records together, with provenance retained on all facts.
    pub fn model(&self) -> &ModelView {
        &self.inner.model
    }
    /// Per-stage diagnostic counters; no global mutable accounting is used.
    pub fn build_metrics(&self) -> &DerivationBuildMetrics {
        &self.inner.build_metrics
    }
    /// Immediate rule/evidence. Follow derived dependencies to inspect the chain.
    /// `None` means this overlay has no such derived assertion, not a false fact.
    pub fn explain(&self, fact: FactKey) -> Option<&Explanation> {
        self.inner.explanations.get(&fact).map(Arc::as_ref)
    }
    /// Derived assertion keys in deterministic order.
    pub fn facts(&self) -> impl Iterator<Item = (FactKey, &Explanation)> {
        self.inner
            .explanations
            .iter()
            .map(|(key, evidence)| (*key, evidence.as_ref()))
    }
}

#[derive(Debug)]
struct ElementInput {
    key: DerivationKey,
    class: MetaclassId,
    properties: Vec<(PropertyId, SlotValue)>,
    explanation: Arc<Explanation>,
}

#[derive(Debug)]
struct OccurrenceInput {
    key: DerivationKey,
    association: AssociationId,
    ends: BTreeMap<PropertyId, ElementId>,
    positions: BTreeMap<PropertyId, usize>,
    explanation: Arc<Explanation>,
}

/// Explicit candidate construction; `build` validates the entire result atomically.
#[derive(Debug)]
pub struct DerivationBuilder<Input = Snapshot> {
    declared: DerivationInput,
    previous: Option<Arc<OverlayData>>,
    input_kind: std::marker::PhantomData<Input>,
    elements: Vec<ElementInput>,
    occurrences: Vec<OccurrenceInput>,
    properties: Vec<(ElementId, PropertyId, SlotValue, Explanation)>,
    extensions: Vec<(ElementId, PropertyId, Vec<ElementId>, Explanation)>,
    failures: BTreeMap<(ElementId, PropertyId), ComputationFailure>,
    searches: BTreeMap<FactKey, Arc<BTreeSet<StructuralSearch>>>,
    search_pool: StructuralSearchPool,
}
impl DerivationBuilder {
    /// Begin a new overlay pinned to this snapshot; no earlier results are reused.
    pub fn new(declared: Snapshot) -> Self {
        Self::with_input(DerivationInput::Strict(declared))
    }
    /// Add a later producer stage to the exact same strict declared revision.
    pub fn from_overlay(previous: DerivedOverlay) -> Self {
        let mut builder = Self::with_input(previous.inner.declared.clone());
        builder.previous = Some(previous.inner);
        builder
    }
    /// Finish a prepared batch after releasing its borrowed input queries.
    pub fn build_on_overlay(
        mut self,
        previous: DerivedOverlay,
    ) -> Result<DerivedOverlay, DerivationError> {
        self.attach_previous(previous.inner)?;
        self.build()
    }
    /// Validate every structural bound, dependency and acyclic explanation.
    pub fn build(self) -> Result<DerivedOverlay, DerivationError> {
        self.build_inner().map(|inner| DerivedOverlay { inner })
    }
}
impl<Input> DerivationBuilder<Input> {
    fn with_input(declared: DerivationInput) -> Self {
        Self {
            declared,
            previous: None,
            input_kind: std::marker::PhantomData,
            elements: Vec::new(),
            occurrences: Vec::new(),
            properties: Vec::new(),
            extensions: Vec::new(),
            failures: BTreeMap::new(),
            searches: BTreeMap::new(),
            search_pool: StructuralSearchPool::default(),
        }
    }
    fn attach_previous(&mut self, previous: Arc<OverlayData>) -> Result<(), DerivationError> {
        if self.previous.is_some()
            || !std::ptr::eq(self.declared.model(), previous.declared.model())
        {
            return Err(DerivationError::InputContextMismatch);
        }
        self.previous = Some(previous);
        Ok(())
    }
    /// Whether this batch contains any pending write, including failure or search
    /// metadata. An inherited overlay alone is not a pending write.
    ///
    /// This inspects queued operations, not semantic equivalence: redundant,
    /// empty-valued or invalid submissions still return `true` and require normal
    /// validation. It does not validate input identity or producer proposals.
    pub fn has_changes(&self) -> bool {
        !self.elements.is_empty()
            || !self.occurrences.is_empty()
            || !self.properties.is_empty()
            || !self.extensions.is_empty()
            || !self.failures.is_empty()
            || !self.searches.is_empty()
    }
    /// Add an implied element. Its ID is determined by `key`. The subject becomes
    /// an automatic dependency; other evidence must be supplied by the producer.
    /// Ordinary required properties must be present on the resulting record.
    pub fn element(
        &mut self,
        key: DerivationKey,
        class: MetaclassId,
        properties: impl IntoIterator<Item = (PropertyId, SlotValue)>,
        dependencies: BTreeSet<Dependency>,
    ) -> &mut Self {
        self.element_with_explanation(
            key,
            class,
            properties,
            Arc::new(Explanation {
                rule: key.rule,
                dependencies,
            }),
        )
    }
    /// Enqueue an element while sharing its immutable proof with other outputs.
    /// `build` checks that the proof's rule matches the identity key and adds the
    /// subject dependency without mutating the caller's proof. All ordinary
    /// collision, dependency and structural validation still applies.
    pub fn element_with_explanation(
        &mut self,
        key: DerivationKey,
        class: MetaclassId,
        properties: impl IntoIterator<Item = (PropertyId, SlotValue)>,
        explanation: Arc<Explanation>,
    ) -> &mut Self {
        self.elements.push(ElementInput {
            key,
            class,
            properties: properties.into_iter().collect(),
            explanation,
        });
        self
    }
    /// Add a canonical inferred association occurrence. The exact registry ends
    /// and explicit ordered-end positions follow the same contract as `ChangeSet::link`.
    /// Identity depends on the key, association and endpoint identities. The
    /// subject and participants become automatic declared/derived dependencies.
    /// Repeated identities, including retired declared identities, fail atomically.
    pub fn association_occurrence(
        &mut self,
        key: DerivationKey,
        association: AssociationId,
        ends: BTreeMap<PropertyId, ElementId>,
        positions: BTreeMap<PropertyId, usize>,
        dependencies: BTreeSet<Dependency>,
    ) -> &mut Self {
        self.association_occurrence_with_explanation(
            key,
            association,
            ends,
            positions,
            Arc::new(Explanation {
                rule: key.rule,
                dependencies,
            }),
        )
    }
    /// Enqueue an occurrence with shared immutable proof. Subject and endpoint
    /// dependencies are added with copy-on-write only when they are absent.
    pub fn association_occurrence_with_explanation(
        &mut self,
        key: DerivationKey,
        association: AssociationId,
        ends: BTreeMap<PropertyId, ElementId>,
        positions: BTreeMap<PropertyId, usize>,
        explanation: Arc<Explanation>,
    ) -> &mut Self {
        self.occurrences.push(OccurrenceInput {
            key,
            association,
            ends,
            positions,
            explanation,
        });
        self
    }
    /// Supply a metamodel-declared derived property on a declared or implied element.
    /// This never replaces a declared value or silently resolves conflicting results.
    /// The subject's existence is added to the dependencies automatically.
    pub fn property(
        &mut self,
        element: ElementId,
        property: PropertyId,
        value: SlotValue,
        explanation: Explanation,
    ) -> &mut Self {
        self.properties
            .push((element, property, value, explanation));
        self
    }
    /// Append inferred references to a stored ordered collection in the overlay.
    /// The declared slot remains unchanged and is automatically retained as
    /// evidence. This cannot replace, remove, reorder, or duplicate a declared
    /// value. The final merged model still passes every ordinary storage check.
    pub fn extend_ordered_references(
        &mut self,
        element: ElementId,
        property: PropertyId,
        additions: Vec<ElementId>,
        explanation: Explanation,
    ) -> &mut Self {
        self.extensions
            .push((element, property, additions, explanation));
        self
    }
    /// Supply search evidence for a computed or unsuccessful result. Absence in a
    /// search is evidence too; results remain bound to the full immutable revision.
    pub fn searches(&mut self, fact: FactKey, searches: BTreeSet<StructuralSearch>) -> &mut Self {
        self.searches_shared(fact, Arc::new(searches))
    }
    /// Supply shared immutable search evidence without copying its set for every
    /// derived fact. Repeated submissions union all keys without mutating callers.
    pub fn searches_shared(
        &mut self,
        fact: FactKey,
        searches: Arc<BTreeSet<StructuralSearch>>,
    ) -> &mut Self {
        merge_searches(&mut self.searches, &mut self.search_pool, fact, searches);
        self
    }
    /// Record an incomplete or invalid derived result with positive and search evidence.
    /// Duplicate submissions fail, including a value and failure for the same property.
    pub fn failure(
        &mut self,
        element: ElementId,
        property: PropertyId,
        failure: ComputationFailure,
    ) -> Result<&mut Self, DerivationError> {
        if self.failures.contains_key(&(element, property)) {
            return Err(DerivationError::DuplicateFact(FactKey::Property {
                element,
                property,
            }));
        }
        self.failures.insert((element, property), failure);
        Ok(self)
    }
    fn build_inner(mut self) -> Result<Arc<OverlayData>, DerivationError> {
        let registry = self.declared.model().registry.clone();
        let (parts, mut explanations, mut evidence_pool, mut search_pool, reused_owned_storage) =
            match self.previous.take() {
                Some(previous) => match Arc::try_unwrap(previous) {
                    Ok(previous) => (
                        previous.model.into_derivation_parts(),
                        previous.explanations,
                        previous.evidence_pool,
                        previous.search_pool,
                        true,
                    ),
                    Err(previous) => (
                        previous.model.derivation_parts(),
                        previous.explanations.clone(),
                        previous.evidence_pool.clone(),
                        previous.search_pool.clone(),
                        false,
                    ),
                },
                None => {
                    let (explanations, pool, searches) =
                        self.declared.immutable_dependency().map_or_else(
                            || {
                                (
                                    BTreeMap::new(),
                                    ExplanationPool::default(),
                                    StructuralSearchPool::default(),
                                )
                            },
                            |p| {
                                (
                                    p.inner.explanations.clone(),
                                    p.inner.evidence_pool.clone(),
                                    p.inner.search_pool.clone(),
                                )
                            },
                        );
                    (
                        self.declared.model().derivation_parts(),
                        explanations,
                        pool,
                        searches,
                        false,
                    )
                }
            };
        // The queued map retains its shared sets. Release this batch's temporary
        // interner before canonicalizing them into the persistent overlay pool.
        drop(self.search_pool);
        let mut records = parts.records;
        let mut links = parts.links;
        let mut derived_navigation = parts.derived_navigation;
        let mut metrics = DerivationBuildMetrics {
            new_elements: self.elements.len(),
            new_association_occurrences: self.occurrences.len(),
            existing_facts_reused: explanations.len(),
            full_model_validations: 1,
            reused_owned_storage,
            ..Default::default()
        };
        let initial_pool = evidence_pool.statistics();
        let initial_search_pool = search_pool.statistics();
        let mut changed_facts = BTreeSet::new();
        let mut changed_existing_explanation = false;
        for input in self.occurrences {
            let id = input
                .key
                .association_occurrence_id(input.association, &input.ends);
            if self.declared.has_used_occurrence(id) {
                return Err(DerivationError::AssociationIdentityCollision(id));
            }
            if links.contains_key(&id) {
                return Err(DerivationError::DuplicateFact(
                    FactKey::AssociationOccurrence(id),
                ));
            }
            let mut explanation = input.explanation;
            if explanation.rule != input.key.rule {
                return Err(DerivationError::ExplanationRuleMismatch {
                    fact: FactKey::AssociationOccurrence(id),
                    expected: input.key.rule,
                    actual: explanation.rule,
                });
            }
            for subject in std::iter::once(input.key.subject).chain(input.ends.values().copied()) {
                let fact = FactKey::Element(subject);
                let dependency = if self.declared.has_declared_fact(fact) {
                    Dependency::Declared(fact)
                } else {
                    Dependency::Derived(fact)
                };
                if !explanation.dependencies.contains(&dependency) {
                    Arc::make_mut(&mut explanation)
                        .dependencies
                        .insert(dependency);
                }
            }
            let explanation = evidence_pool.intern_shared(explanation);
            links.insert(
                id,
                AssociationOccurrence {
                    id,
                    association: input.association,
                    ends: input.ends,
                    positions: input.positions,
                    origin: Origin::Derived(explanation.clone()),
                },
            );
            explanations.insert(FactKey::AssociationOccurrence(id), explanation);
            changed_facts.insert(FactKey::AssociationOccurrence(id));
        }
        for input in self.elements {
            let id = input.key.element_id();
            if self.declared.has_used(id) || records.contains_key(&id) {
                return Err(DerivationError::IdentityCollision(id));
            }
            let mut explanation = input.explanation;
            if explanation.rule != input.key.rule {
                return Err(DerivationError::ExplanationRuleMismatch {
                    fact: FactKey::Element(id),
                    expected: input.key.rule,
                    actual: explanation.rule,
                });
            }
            let subject = FactKey::Element(input.key.subject);
            let dependency = if self.declared.has_declared_fact(subject) {
                Dependency::Declared(subject)
            } else {
                Dependency::Derived(subject)
            };
            if !explanation.dependencies.contains(&dependency) {
                Arc::make_mut(&mut explanation)
                    .dependencies
                    .insert(dependency);
            }
            let explanation = evidence_pool.intern_shared(explanation);
            let mut record = ElementRecord {
                id,
                metaclass: input.class,
                slots: BTreeMap::new(),
                origin: Origin::Derived(explanation.clone()),
            };
            explanations.insert(FactKey::Element(id), explanation);
            changed_facts.insert(FactKey::Element(id));
            for (property, mut value) in input.properties {
                value.normalize();
                let key = FactKey::Property {
                    element: id,
                    property,
                };
                let mut evidence = Explanation {
                    rule: input.key.rule,
                    dependencies: BTreeSet::from([Dependency::Derived(FactKey::Element(id))]),
                };
                add_reference_dependencies(&self.declared, &value, &mut evidence.dependencies);
                let evidence = evidence_pool.intern(evidence);
                if record
                    .slots
                    .insert(
                        property,
                        Slot {
                            value,
                            origin: Origin::Derived(evidence.clone()),
                        },
                    )
                    .is_some()
                {
                    return Err(DerivationError::DuplicateFact(key));
                }
                explanations.insert(key, evidence);
                changed_facts.insert(key);
            }
            records.insert(id, Arc::new(record));
        }
        let mut extended = BTreeSet::new();
        for (element, property, additions, mut explanation) in self.extensions {
            self.declared
                .check_dependency_write(FactKey::Property { element, property })?;
            let descriptor = registry.property(property).map_err(ModelError::from)?;
            if descriptor.derived
                || !descriptor.ordered
                || !matches!(
                    registry
                        .storage_kind(descriptor.value_kind)
                        .map_err(ModelError::from)?,
                    crate::metamodel::ValueKind::Reference(_)
                )
                || !registry
                    .supports_slot_storage(property)
                    .map_err(ModelError::from)?
            {
                return Err(DerivationError::InvalidCollectionExtension { element, property });
            }
            let key = FactKey::Property { element, property };
            if !extended.insert(key) || changed_facts.contains(&key) {
                return Err(DerivationError::DuplicateFact(key));
            }
            if let Some(previous) = explanations.get(&key) {
                changed_existing_explanation = true;
                // Keep the evidence for every previous entry. No dependency on
                // the same aggregate property is introduced into the DAG.
                explanation
                    .dependencies
                    .extend(previous.dependencies.iter().copied());
            }
            let record = records
                .get_mut(&element)
                .ok_or(ModelError::UnknownElement(element))?;
            let mut values = match record.slot(property).map(|slot| slot.value()) {
                Some(SlotValue::Ordered(values)) => {
                    if self.declared.has_declared_fact(key) {
                        explanation.dependencies.insert(Dependency::Declared(key));
                    }
                    values.clone()
                }
                None => Vec::new(),
                _ => return Err(DerivationError::InvalidCollectionExtension { element, property }),
            };
            for addition in additions {
                let value = crate::value::Value::Reference(addition);
                if values.contains(&value) {
                    return Err(DerivationError::InvalidCollectionExtension { element, property });
                }
                values.push(value);
                explanation.dependencies.insert(
                    if self.declared.has_declared_fact(FactKey::Element(addition)) {
                        Dependency::Declared(FactKey::Element(addition))
                    } else {
                        Dependency::Derived(FactKey::Element(addition))
                    },
                );
            }
            explanation.dependencies.insert(
                if self.declared.has_declared_fact(FactKey::Element(element)) {
                    Dependency::Declared(FactKey::Element(element))
                } else {
                    Dependency::Derived(FactKey::Element(element))
                },
            );
            let explanation = evidence_pool.intern(explanation);
            Arc::make_mut(record).slots.insert(
                property,
                Slot {
                    value: SlotValue::Ordered(values),
                    origin: Origin::Derived(explanation.clone()),
                },
            );
            explanations.insert(key, explanation);
            changed_facts.insert(key);
        }
        for (element, property, mut value, mut explanation) in self.properties {
            self.declared
                .check_dependency_write(FactKey::Property { element, property })?;
            let descriptor = registry.property(property).map_err(ModelError::from)?;
            if !descriptor.derived {
                return Err(DerivationError::NotDerivedProperty { element, property });
            }
            let record = records
                .get_mut(&element)
                .ok_or(ModelError::UnknownElement(element))?;
            let key = FactKey::Property { element, property };
            if record.slots.contains_key(&property) || explanations.contains_key(&key) {
                return Err(DerivationError::DuplicateFact(key));
            }
            let subject = FactKey::Element(element);
            explanation.dependencies.insert(
                if self.declared.has_declared_fact(FactKey::Element(element)) {
                    Dependency::Declared(subject)
                } else {
                    Dependency::Derived(subject)
                },
            );
            value.normalize();
            add_reference_dependencies(&self.declared, &value, &mut explanation.dependencies);
            let explanation = evidence_pool.intern(explanation);
            let slot = Slot {
                value,
                origin: Origin::Derived(explanation.clone()),
            };
            match descriptor.owner {
                crate::metamodel::PropertyOwner::Class(_) => {
                    Arc::make_mut(record).slots.insert(property, slot);
                }
                crate::metamodel::PropertyOwner::Association(_) => {
                    derived_navigation.insert((element, property), slot);
                }
            }
            explanations.insert(key, explanation);
            changed_facts.insert(key);
        }
        let (mut model, obligations) =
            self.declared
                .build_model(registry, records, links, derived_navigation)?;
        self.declared.check_dependency_ownership(&model)?;
        model.declared_source = Some(self.declared.clone());
        model.statuses = parts.statuses;
        model.searches = parts.searches;
        for ((element, property), mut failure) in self.failures {
            self.declared
                .check_dependency_write(FactKey::Property { element, property })?;
            let record = model
                .element(element)
                .ok_or(ModelError::UnknownElement(element))?;
            if !model
                .registry
                .is_applicable_navigation(record.metaclass(), property)
                .map_err(ModelError::from)?
            {
                return Err(ModelError::IllegalProperty {
                    element,
                    class: record.metaclass(),
                    property,
                }
                .into());
            }
            if !model
                .registry
                .property(property)
                .map_err(ModelError::from)?
                .derived
            {
                return Err(DerivationError::NotDerivedProperty { element, property });
            }
            let key = FactKey::Property { element, property };
            if model.navigation_slot(element, property).is_some() || explanations.contains_key(&key)
            {
                return Err(DerivationError::DuplicateFact(key));
            }
            let mut evidence = failure.explanation().clone();
            evidence.dependencies.insert(
                if self.declared.has_declared_fact(FactKey::Element(element)) {
                    Dependency::Declared(FactKey::Element(element))
                } else {
                    Dependency::Derived(FactKey::Element(element))
                },
            );
            match &mut failure {
                ComputationFailure::Incomplete {
                    explanation,
                    searches,
                    ..
                }
                | ComputationFailure::Invalid {
                    explanation,
                    searches,
                    ..
                } => {
                    *explanation = evidence.clone();
                    merge_searches(
                        &mut model.searches,
                        &mut search_pool,
                        key,
                        Arc::new(searches.clone()),
                    );
                }
            }
            explanations.insert(key, evidence_pool.intern(evidence));
            changed_facts.insert(key);
            model.statuses.insert((element, property), failure);
        }
        // Old facts were already validated and additive batches cannot remove
        // their dependencies. Check each new immutable proof once per outcome
        // class, even when thousands of outputs share a large dependency set.
        let mut checked_proofs = HashSet::new();
        for &fact in &changed_facts {
            let explanation = &explanations[&fact];
            let unsuccessful = matches!(fact, FactKey::Property { element, property }
                if model.statuses.contains_key(&(element, property)));
            if !checked_proofs.insert((Arc::as_ptr(explanation) as usize, unsuccessful)) {
                continue;
            }
            for &dependency in &explanation.dependencies {
                metrics.dependency_edges_considered += 1;
                let exists = match dependency {
                    Dependency::Declared(key) => self.declared.has_declared_fact(key),
                    Dependency::Derived(key) => {
                        if let FactKey::Property { element, property } = key
                            && model.statuses.contains_key(&(element, property))
                            && !unsuccessful
                        {
                            return Err(DerivationError::IncompleteDependency(key));
                        }
                        explanations.contains_key(&key)
                    }
                };
                if !exists {
                    return Err(DerivationError::MissingDependency { fact, dependency });
                }
            }
        }
        for (fact, searches) in self.searches {
            if !explanations.contains_key(&fact) {
                return Err(DerivationError::MissingSearchSubject(fact));
            }
            merge_searches(&mut model.searches, &mut search_pool, fact, searches);
        }
        // Every new cycle contains a changed fact. Without modification of an
        // earlier explanation, old facts cannot point to newly introduced facts,
        // so the cycle search only needs the new subgraph.
        let cycle = cyclic_explanations(
            &explanations,
            &evidence_pool,
            &changed_facts,
            changed_existing_explanation,
        );
        if !cycle.is_empty() {
            return Err(DerivationError::DependencyCycle(cycle));
        }
        let final_pool = evidence_pool.statistics();
        metrics.proof_sets_interned = final_pool.interned - initial_pool.interned;
        metrics.proof_sets_reused = final_pool.reused - initial_pool.reused;
        let final_search_pool = search_pool.statistics();
        metrics.logical_search_sets = model.searches.len();
        metrics.logical_search_entries = model.searches.values().map(|s| s.len()).sum();
        metrics.retained_search_sets = final_search_pool.interned;
        metrics.retained_search_entries = final_search_pool.entries;
        metrics.search_sets_interned = final_search_pool.interned - initial_search_pool.interned;
        metrics.search_sets_reused = final_search_pool.reused - initial_search_pool.reused;
        Ok(Arc::new(OverlayData {
            declared: self.declared,
            model,
            obligations,
            explanations,
            evidence_pool,
            search_pool,
            build_metrics: metrics,
        }))
    }
}

fn merge_searches(
    output: &mut BTreeMap<FactKey, Arc<BTreeSet<StructuralSearch>>>,
    pool: &mut StructuralSearchPool,
    fact: FactKey,
    searches: Arc<BTreeSet<StructuralSearch>>,
) {
    match output.entry(fact) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(pool.intern_shared(searches));
        }
        std::collections::btree_map::Entry::Occupied(mut entry) => {
            let union = pool.union_shared(entry.get(), searches);
            entry.insert(union);
        }
    }
}

fn add_reference_dependencies(
    snapshot: &DerivationInput,
    value: &SlotValue,
    dependencies: &mut BTreeSet<Dependency>,
) {
    for value in value.values() {
        if let crate::value::Value::Reference(target) = value {
            let fact = FactKey::Element(*target);
            dependencies.insert(if snapshot.has_declared_fact(fact) {
                Dependency::Declared(fact)
            } else {
                Dependency::Derived(fact)
            });
        }
    }
}

/// Invalid inference results, distinct from the truth or validity of a semantic rule.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DerivationError {
    #[error("explanation rule {actual} does not match identity rule {expected} for {fact:?}")]
    ExplanationRuleMismatch {
        fact: FactKey,
        expected: crate::RuleId,
        actual: crate::RuleId,
    },
    #[error("derivation input does not match the bound immutable semantic view")]
    InputContextMismatch,
    #[error("invalid monotone ordered-reference extension {element}/{property}")]
    InvalidCollectionExtension {
        element: ElementId,
        property: PropertyId,
    },
    #[error("search evidence has no computation subject {0:?}")]
    MissingSearchSubject(FactKey),
    #[error("computed fact depends on incomplete or invalid result {0:?}")]
    IncompleteDependency(FactKey),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error("derived identity {0} collides with an existing or retired identity")]
    IdentityCollision(ElementId),
    #[error("derived association identity {0} collides with an existing or retired identity")]
    AssociationIdentityCollision(AssociationOccurrenceId),
    #[error("duplicate derived fact {0:?}")]
    DuplicateFact(FactKey),
    #[error("{element}/{property} is not a derived property")]
    NotDerivedProperty {
        element: ElementId,
        property: PropertyId,
    },
    #[error("fact {fact:?} depends on missing evidence {dependency:?}")]
    MissingDependency {
        fact: FactKey,
        dependency: Dependency,
    },
    #[error("cyclic derivation dependencies involving {0:?}")]
    DependencyCycle(Vec<FactKey>),
}

/// A typed reason why a structural computation is incomplete.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IncompleteReason {
    MissingInput,
    UnsupportedRuntimeSemantics,
    IncompleteDependency,
}
/// Search dependencies include empty searches. They are evaluated against the exact
/// registry and immutable model revision, not only positive fact dependencies.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum StructuralSearch {
    /// Conservative search over the entire immutable input graph, including
    /// absent facts. Any input change invalidates the computation.
    Model,
    DescriptorGraph,
    /// Existence, metaclass or any stored property of one element, including a
    /// bounded negative lookup for an element not present in the input view.
    Element(ElementId),
    Property {
        element: ElementId,
        property: PropertyId,
    },
    Incoming(ElementId),
    Association {
        element: ElementId,
        association: crate::AssociationId,
    },
    /// Relationships of `class` (including registered subtypes) whose effective
    /// `property` source role references `source`. Property redefinitions are
    /// resolved through the immutable descriptor registry. Unlike `Incoming`,
    /// references through unrelated target roles are outside this population.
    SourceRelationships {
        source: ElementId,
        class: MetaclassId,
        property: PropertyId,
    },
}
/// Explicit unsuccessful computation; never an empty value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComputationFailure {
    Incomplete {
        reason: IncompleteReason,
        explanation: Explanation,
        searches: BTreeSet<StructuralSearch>,
    },
    Invalid {
        diagnostic: String,
        explanation: Explanation,
        searches: BTreeSet<StructuralSearch>,
    },
}
impl ComputationFailure {
    pub fn explanation(&self) -> &Explanation {
        match self {
            Self::Incomplete { explanation, .. } | Self::Invalid { explanation, .. } => explanation,
        }
    }
}
/// Borrowed structural state. Presence of an empty collection is `Computed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyState<'m> {
    Absent,
    NotComputed,
    Computed(&'m Slot),
    Incomplete(&'m ComputationFailure),
    Invalid(&'m ComputationFailure),
}
