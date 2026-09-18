//! Revision-bound semantic results with explicit evidence. No rule evaluator is built in.
use crate::model::{Slot, cyclic_nodes};
use crate::provenance::{Dependency, Explanation, FactKey, Origin};
use crate::value::SlotValue;
use crate::{
    DerivationKey, ElementId, ElementRecord, MetaclassId, ModelError, ModelView, PropertyId,
    RevisionId, Snapshot,
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
    declared: Snapshot,
    model: ModelView,
    explanations: BTreeMap<FactKey, Explanation>,
}
impl DerivedOverlay {
    /// Original declared revision, without any inferred slots or elements.
    pub fn declared(&self) -> &Snapshot {
        &self.declared
    }
    /// Declared input revision. This is not an identity for the overlay: different
    /// rule sets/results can be built over the same revision.
    pub fn base_revision(&self) -> RevisionId {
        self.declared.revision()
    }
    /// Declared and derived records together, with provenance retained on all facts.
    pub fn model(&self) -> &ModelView {
        &self.model
    }
    /// Immediate rule/evidence. Follow derived dependencies to inspect the chain.
    /// `None` means this overlay has no such derived assertion, not a false fact.
    pub fn explain(&self, fact: FactKey) -> Option<&Explanation> {
        self.explanations.get(&fact)
    }
    /// Derived assertion keys in deterministic order.
    pub fn facts(&self) -> impl Iterator<Item = (FactKey, &Explanation)> {
        self.explanations
            .iter()
            .map(|(key, evidence)| (*key, evidence))
    }
}

#[derive(Debug)]
struct ElementInput {
    key: DerivationKey,
    class: MetaclassId,
    properties: Vec<(PropertyId, SlotValue)>,
    dependencies: BTreeSet<Dependency>,
}

/// Explicit candidate construction; `build` validates the entire result atomically.
#[derive(Debug)]
pub struct DerivationBuilder {
    declared: Snapshot,
    elements: Vec<ElementInput>,
    properties: Vec<(ElementId, PropertyId, SlotValue, Explanation)>,
    failures: BTreeMap<(ElementId, PropertyId), ComputationFailure>,
    searches: BTreeMap<FactKey, BTreeSet<StructuralSearch>>,
}
impl DerivationBuilder {
    /// Begin a new overlay pinned to this snapshot; no earlier results are reused.
    pub fn new(declared: Snapshot) -> Self {
        Self {
            declared,
            elements: Vec::new(),
            properties: Vec::new(),
            failures: BTreeMap::new(),
            searches: BTreeMap::new(),
        }
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
        self.elements.push(ElementInput {
            key,
            class,
            properties: properties.into_iter().collect(),
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
        let mut records = self.declared.model().records.clone();
        let mut explanations = BTreeMap::new();
        for input in self.elements {
            let id = input.key.element_id();
            if self.declared.has_used(id) || records.contains_key(&id) {
                return Err(DerivationError::IdentityCollision(id));
            }
            let mut dependencies = input.dependencies;
            let subject = FactKey::Element(input.key.subject);
            dependencies.insert(
                if self.declared.model().element(input.key.subject).is_some() {
                    Dependency::Declared(subject)
                } else {
                    Dependency::Derived(subject)
                },
            );
            let explanation = Explanation {
                rule: input.key.rule,
                dependencies,
            };
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
                let evidence = Explanation {
                    rule: input.key.rule,
                    dependencies: BTreeSet::from([Dependency::Derived(FactKey::Element(id))]),
                };
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
        let mut derived_navigation = BTreeMap::new();
        for (element, property, mut value, mut explanation) in self.properties {
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
            explanation
                .dependencies
                .insert(if self.declared.model().element(element).is_some() {
                    Dependency::Declared(subject)
                } else {
                    Dependency::Derived(subject)
                });
            value.normalize();
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
        let mut model = ModelView::build(
            registry,
            records,
            self.declared.model().links.clone(),
            derived_navigation,
        )?;
        for ((element, property), mut failure) in self.failures {
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
            if record.slot(property).is_some() || explanations.contains_key(&key) {
                return Err(DerivationError::DuplicateFact(key));
            }
            let mut evidence = failure.explanation().clone();
            evidence
                .dependencies
                .insert(if self.declared.model().element(element).is_some() {
                    Dependency::Declared(FactKey::Element(element))
                } else {
                    Dependency::Derived(FactKey::Element(element))
                });
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
            explanations.insert(key, evidence);
            model.statuses.insert((element, property), failure);
        }
        let mut edges: BTreeMap<FactKey, BTreeSet<FactKey>> = BTreeMap::new();
        for (&fact, explanation) in &explanations {
            for &dependency in &explanation.dependencies {
                let exists = match dependency {
                    Dependency::Declared(FactKey::AssociationOccurrence(id)) => {
                        self.declared.model().association_occurrence(id).is_some()
                    }
                    Dependency::Declared(FactKey::Element(id)) => {
                        self.declared.model().element(id).is_some()
                    }
                    Dependency::Declared(FactKey::Property { element, property }) => self
                        .declared
                        .model()
                        .element(element)
                        .and_then(|e| e.slot(property))
                        .is_some(),
                    Dependency::Derived(key) => {
                        if let FactKey::Property { element, property } = key
                            && model.statuses.contains_key(&(element, property))
                            && !matches!(fact, FactKey::Property { element,property } if model.statuses.contains_key(&(element,property)))
                        {
                            return Err(DerivationError::IncompleteDependency(key));
                        }
                        edges.entry(fact).or_default().insert(key);
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
        let cycle = cyclic_nodes(&edges);
        if !cycle.is_empty() {
            return Err(DerivationError::DependencyCycle(cycle));
        }
        Ok(DerivedOverlay {
            declared: self.declared,
            model,
            explanations,
        })
    }
}

/// Invalid inference results, distinct from the truth or validity of a semantic rule.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DerivationError {
    #[error("search evidence has no computation subject {0:?}")]
    MissingSearchSubject(FactKey),
    #[error("computed fact depends on incomplete or invalid result {0:?}")]
    IncompleteDependency(FactKey),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error("derived identity {0} collides with an existing or retired identity")]
    IdentityCollision(ElementId),
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
