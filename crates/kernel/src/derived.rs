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
}
impl DerivationBuilder {
    /// Begin a new overlay pinned to this snapshot; no earlier results are reused.
    pub fn new(declared: Snapshot) -> Self {
        Self {
            declared,
            elements: Vec::new(),
            properties: Vec::new(),
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
        for (element, property, mut value, mut explanation) in self.properties {
            let descriptor = registry.property(property).map_err(ModelError::from)?;
            if !descriptor.derived {
                return Err(DerivationError::NotDerivedProperty { element, property });
            }
            let record = records
                .get_mut(&element)
                .ok_or(ModelError::UnknownElement(element))?;
            let key = FactKey::Property { element, property };
            if record.slots.contains_key(&property) {
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
            Arc::make_mut(record).slots.insert(
                property,
                Slot {
                    value,
                    origin: Origin::Derived(explanation.clone()),
                },
            );
            explanations.insert(key, explanation);
        }
        let model = ModelView::build(registry, records)?;
        let mut edges: BTreeMap<FactKey, BTreeSet<FactKey>> = BTreeMap::new();
        for (&fact, explanation) in &explanations {
            for &dependency in &explanation.dependencies {
                let exists = match dependency {
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
                        edges.entry(fact).or_default().insert(key);
                        explanations.contains_key(&key)
                    }
                };
                if !exists {
                    return Err(DerivationError::MissingDependency { fact, dependency });
                }
            }
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
