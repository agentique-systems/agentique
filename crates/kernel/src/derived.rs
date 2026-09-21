//! Revision-bound semantic results with explicit evidence. No rule evaluator is built in.
use crate::association::AssociationOccurrence;
use crate::model::{Slot, cyclic_nodes_by};
use crate::provenance::{Dependency, Explanation, ExplanationPool, FactKey, Origin};
use crate::value::SlotValue;
use crate::{
    AssociationId, AssociationOccurrenceId, DerivationKey, ElementId, ElementRecord, MetaclassId,
    ModelError, ModelView, PropertyId, RevisionId, Snapshot,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
    declared: Snapshot,
    model: ModelView,
    explanations: BTreeMap<FactKey, Arc<Explanation>>,
}
impl DerivedOverlay {
    /// Original declared revision, without any inferred slots or elements.
    pub fn declared(&self) -> &Snapshot {
        &self.inner.declared
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
    dependencies: BTreeSet<Dependency>,
}

/// Explicit candidate construction; `build` validates the entire result atomically.
#[derive(Debug)]
pub struct DerivationBuilder {
    declared: Snapshot,
    previous: Option<DerivedOverlay>,
    elements: Vec<ElementInput>,
    occurrences: Vec<OccurrenceInput>,
    properties: Vec<(ElementId, PropertyId, SlotValue, Explanation)>,
    extensions: Vec<(ElementId, PropertyId, Vec<ElementId>, Explanation)>,
    failures: BTreeMap<(ElementId, PropertyId), ComputationFailure>,
    searches: BTreeMap<FactKey, BTreeSet<StructuralSearch>>,
}
impl DerivationBuilder {
    /// Begin a new overlay pinned to this snapshot; no earlier results are reused.
    pub fn new(declared: Snapshot) -> Self {
        Self {
            declared,
            previous: None,
            elements: Vec::new(),
            occurrences: Vec::new(),
            properties: Vec::new(),
            extensions: Vec::new(),
            failures: BTreeMap::new(),
            searches: BTreeMap::new(),
        }
    }
    /// Add a later producer stage to the same immutable declared revision.
    /// Existing facts and evidence are retained; this is never a rebase. The
    /// original overlay remains unchanged if the combined candidate is rejected.
    pub fn from_overlay(previous: DerivedOverlay) -> Self {
        let mut builder = Self::new(previous.declared().clone());
        builder.previous = Some(previous);
        builder
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
        self.occurrences.push(OccurrenceInput {
            key,
            association,
            ends,
            positions,
            dependencies,
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
        self.searches.entry(fact).or_default().extend(searches);
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
    /// Validate structural constraints, dependencies and acyclic explanations.
    pub fn build(self) -> Result<DerivedOverlay, DerivationError> {
        let registry = self.declared.model().registry.clone();
        let input = self
            .previous
            .as_ref()
            .map_or(self.declared.model(), |p| p.model());
        let mut records = input.records.clone();
        let mut explanations = self.previous.as_ref().map_or_else(
            || {
                self.declared
                    .immutable_dependency()
                    .map_or_else(BTreeMap::new, |p| p.inner.explanations.clone())
            },
            |p| p.inner.explanations.clone(),
        );
        let previous_facts: BTreeSet<_> = explanations.keys().copied().collect();
        // Exact content equality, never allocation or hash iteration order,
        // determines sharing. This pool has no effect on semantic identity.
        let mut evidence_pool = ExplanationPool::default();
        for explanation in explanations.values() {
            evidence_pool.intern_shared(explanation.clone());
        }
        let mut links = input.links.clone();
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
            let mut dependencies = input.dependencies;
            for subject in std::iter::once(input.key.subject).chain(input.ends.values().copied()) {
                let fact = FactKey::Element(subject);
                dependencies.insert(if self.declared.has_declared_fact(fact) {
                    Dependency::Declared(fact)
                } else {
                    Dependency::Derived(fact)
                });
            }
            let explanation = evidence_pool.intern(Explanation {
                rule: input.key.rule,
                dependencies,
            });
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
            if !extended.insert(key)
                || (explanations.contains_key(&key) && !previous_facts.contains(&key))
            {
                return Err(DerivationError::DuplicateFact(key));
            }
            if let Some(previous) = explanations.get(&key) {
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
        }
        let mut derived_navigation = input
            .derived_navigation_results()
            .map(|(key, slot)| (*key, slot.clone()))
            .collect::<BTreeMap<_, _>>();
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
        }
        let mut model = ModelView::build(registry, records, links, derived_navigation)?;
        model.declared_source = Some(self.declared.clone());
        model.statuses = input.statuses.clone();
        model.searches = input.searches.clone();
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
                    model
                        .searches
                        .entry(key)
                        .or_default()
                        .extend(searches.iter().cloned());
                }
            }
            explanations.insert(key, evidence_pool.intern(evidence));
            model.statuses.insert((element, property), failure);
        }
        for (&fact, explanation) in &explanations {
            for &dependency in &explanation.dependencies {
                let exists = match dependency {
                    Dependency::Declared(key) => self.declared.has_declared_fact(key),
                    Dependency::Derived(key) => {
                        if let FactKey::Property { element, property } = key
                            && model.statuses.contains_key(&(element, property))
                            && !matches!(fact, FactKey::Property { element,property } if model.statuses.contains_key(&(element,property)))
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
            model.searches.entry(fact).or_default().extend(searches);
        }
        let cycle = cyclic_nodes_by(explanations.keys().copied(), |fact| {
            explanations[fact]
                .dependencies
                .iter()
                .filter_map(|dependency| match dependency {
                    Dependency::Derived(key) => Some(*key),
                    Dependency::Declared(_) => None,
                })
        });
        if !cycle.is_empty() {
            return Err(DerivationError::DependencyCycle(cycle));
        }
        Ok(DerivedOverlay {
            inner: Arc::new(OverlayData {
                declared: self.declared,
                model,
                explanations,
            }),
        })
    }
}

fn add_reference_dependencies(
    snapshot: &Snapshot,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IncompleteReason {
    MissingInput,
    UnsupportedRuntimeSemantics,
    IncompleteDependency,
}
/// Search dependencies include empty searches. They are evaluated against the exact
/// registry and immutable model revision, not only positive fact dependencies.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructuralSearch {
    /// Conservative search over the entire immutable input graph, including
    /// absent facts. Any input change invalidates the computation.
    Model,
    DescriptorGraph,
    Property {
        element: ElementId,
        property: PropertyId,
    },
    Incoming(ElementId),
    Association {
        element: ElementId,
        association: crate::AssociationId,
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
