//! Private, versioned content encoding. Shared proofs are hashed once per graph.
//!
//! Pointer identity is only an acceleration key for borrowed immutable evidence;
//! every emitted byte depends on semantic content. Equal proofs in distinct
//! allocations therefore produce the same graph digest.
use agq_kernel::{
    ElementRecord, ModelView, Slot,
    association::AssociationOccurrence,
    derived::{ComputationFailure, IncompleteReason, StructuralSearch},
    provenance::{DeclaredOrigin, Dependency, Explanation, FactKey, Origin},
    value::{SlotValue, Value},
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, marker::PhantomData};

const MODEL_SCHEMA: &[u8] = b"agq-semantic-model-graph/2";
const LIBRARY_SCHEMA: &[u8] = b"agq-canonical-library-graph/2";
const PROOF_SCHEMA: &[u8] = b"agq-semantic-proof/1";

pub(super) fn model_digest(model: &ModelView) -> [u8; 32] {
    let mut graph = GraphEncoder::new(MODEL_SCHEMA);
    graph.model(model);
    graph.finish()
}

pub(super) fn library_graph_digest(model: &ModelView) -> [u8; 32] {
    let mut graph = GraphEncoder::new(LIBRARY_SCHEMA);
    for record in model.elements().filter(|r| is_library(r.origin())) {
        graph.record(record);
    }
    for link in model
        .association_occurrences()
        .filter(|r| is_library(r.origin()))
    {
        graph.occurrence(link);
    }
    graph.finish()
}

fn is_library(origin: &Origin) -> bool {
    matches!(
        origin,
        Origin::Declared(
            DeclaredOrigin::StandardLibrary { .. } | DeclaredOrigin::ReviewedCorrection { .. }
        )
    )
}

struct Encoder(Sha256);
impl Encoder {
    fn new(schema: &[u8]) -> Self {
        let mut result = Self(Sha256::new());
        result.bytes(schema);
        result
    }
    fn tag(&mut self, value: u8) {
        self.0.update([value]);
    }
    fn count(&mut self, count: usize) {
        self.0.update((count as u64).to_be_bytes());
    }
    fn id(&mut self, id: u128) {
        self.0.update(id.to_be_bytes());
    }
    fn bytes(&mut self, value: &[u8]) {
        self.count(value.len());
        self.0.update(value);
    }
    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
    fn fact(&mut self, fact: FactKey) {
        match fact {
            FactKey::Element(id) => {
                self.tag(0);
                self.id(id.as_u128());
            }
            FactKey::Property { element, property } => {
                self.tag(1);
                self.id(element.as_u128());
                self.id(property.as_u128());
            }
            FactKey::AssociationOccurrence(id) => {
                self.tag(2);
                self.id(id.as_u128());
            }
        }
    }
    fn explanation(&mut self, explanation: &Explanation) {
        self.id(explanation.rule.as_u128());
        self.count(explanation.dependencies.len());
        for dependency in &explanation.dependencies {
            match dependency {
                Dependency::Declared(fact) => {
                    self.tag(0);
                    self.fact(*fact);
                }
                Dependency::Derived(fact) => {
                    self.tag(1);
                    self.fact(*fact);
                }
            }
        }
    }
    fn searches(&mut self, searches: &std::collections::BTreeSet<StructuralSearch>) {
        self.count(searches.len());
        for search in searches {
            match search {
                StructuralSearch::Model => self.tag(0),
                StructuralSearch::DescriptorGraph => self.tag(1),
                StructuralSearch::Element(element) => {
                    self.tag(2);
                    self.id(element.as_u128());
                }
                StructuralSearch::Property { element, property } => {
                    self.tag(3);
                    self.id(element.as_u128());
                    self.id(property.as_u128());
                }
                StructuralSearch::Incoming(element) => {
                    self.tag(4);
                    self.id(element.as_u128());
                }
                StructuralSearch::Association {
                    element,
                    association,
                } => {
                    self.tag(5);
                    self.id(element.as_u128());
                    self.id(association.as_u128());
                }
                StructuralSearch::SourceRelationships {
                    source,
                    class,
                    property,
                } => {
                    self.tag(6);
                    self.id(source.as_u128());
                    self.id(class.as_u128());
                    self.id(property.as_u128());
                }
                StructuralSearch::ElementIdentity(element) => {
                    self.tag(7);
                    self.id(element.as_u128());
                }
                StructuralSearch::ProducerClosure {
                    subject,
                    requirement,
                } => {
                    self.tag(8);
                    self.id(subject.as_u128());
                    self.text(requirement);
                }
                StructuralSearch::OwnedRelationships { owner, class } => {
                    self.tag(9);
                    self.id(owner.as_u128());
                    self.id(class.as_u128());
                }
                StructuralSearch::OwnedMemberProjection { owner, contract } => {
                    self.tag(10);
                    self.id(owner.as_u128());
                    self.text(contract);
                }
                StructuralSearch::OwnedRelationshipsExcluding {
                    owner,
                    class,
                    excluded,
                } => {
                    self.tag(11);
                    self.id(owner.as_u128());
                    self.id(class.as_u128());
                    self.count(excluded.len());
                    for class in excluded {
                        self.id(class.as_u128());
                    }
                }
            }
        }
    }
    fn value(&mut self, value: &Value) {
        match value {
            Value::Boolean(value) => {
                self.tag(0);
                self.tag(u8::from(*value));
            }
            Value::Integer(value) => {
                self.tag(1);
                self.text(&value.to_string());
            }
            Value::Real(value) => {
                self.tag(2);
                self.text(&value.to_string());
            }
            Value::String(value) => {
                self.tag(3);
                self.text(value);
            }
            Value::Enumeration(value) => {
                self.tag(4);
                self.id(value.as_u128());
            }
            Value::Reference(value) => {
                self.tag(5);
                self.id(value.as_u128());
            }
        }
    }
    fn slot_value(&mut self, value: &SlotValue) {
        self.tag(match value {
            SlotValue::Scalar(_) => 0,
            SlotValue::Ordered(_) => 1,
            SlotValue::Set(_) => 2,
            SlotValue::Bag(_) => 3,
        });
        self.count(value.values().count());
        for value in value.values() {
            self.value(value);
        }
    }
    fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DigestWork {
    proofs_encoded: usize,
    proof_reuses: usize,
    dependency_edges_encoded: usize,
}

struct GraphEncoder<'m> {
    encoder: Encoder,
    proofs: HashMap<usize, [u8; 32]>,
    // Enforces that memoized addresses stay alive throughout the encoding pass.
    borrowed: PhantomData<&'m Explanation>,
    #[cfg(test)]
    work: DigestWork,
}
impl<'m> GraphEncoder<'m> {
    fn new(schema: &[u8]) -> Self {
        Self {
            encoder: Encoder::new(schema),
            proofs: HashMap::new(),
            borrowed: PhantomData,
            #[cfg(test)]
            work: DigestWork::default(),
        }
    }
    fn proof(&mut self, explanation: &'m Explanation) {
        let address = std::ptr::from_ref(explanation) as usize;
        let digest = if let Some(digest) = self.proofs.get(&address) {
            #[cfg(test)]
            {
                self.work.proof_reuses += 1;
            }
            *digest
        } else {
            let mut encoder = Encoder::new(PROOF_SCHEMA);
            encoder.explanation(explanation);
            let digest = encoder.finish();
            self.proofs.insert(address, digest);
            #[cfg(test)]
            {
                self.work.proofs_encoded += 1;
                self.work.dependency_edges_encoded += explanation.dependencies.len();
            }
            digest
        };
        self.encoder.0.update(digest);
    }
    fn origin(&mut self, origin: &'m Origin) {
        match origin {
            Origin::AssociationOccurrences(ids) => {
                self.encoder.tag(0);
                self.encoder.count(ids.len());
                for id in ids {
                    self.encoder.id(id.as_u128());
                }
            }
            Origin::Derived(explanation) => {
                self.encoder.tag(1);
                self.proof(explanation);
            }
            Origin::Declared(origin) => {
                self.encoder.tag(2);
                match origin {
                    DeclaredOrigin::Authored { source } => {
                        self.encoder.tag(0);
                        self.encoder.tag(u8::from(source.is_some()));
                        if let Some(source) = source {
                            self.encoder.id(source.document.as_u128());
                            self.encoder.id(source.revision.as_u128());
                            self.encoder.0.update(source.range.start().to_be_bytes());
                            self.encoder.0.update(source.range.end().to_be_bytes());
                            self.encoder.tag(u8::from(source.syntax_node.is_some()));
                            if let Some(node) = source.syntax_node {
                                self.encoder.id(node.as_u128());
                            }
                        }
                    }
                    DeclaredOrigin::StandardLibrary { library } => {
                        self.encoder.tag(1);
                        self.encoder.id(library.as_u128());
                    }
                    DeclaredOrigin::ReviewedCorrection {
                        profile,
                        entry,
                        authority,
                        library,
                        source_key,
                        output_key,
                    } => {
                        self.encoder.tag(2);
                        self.encoder.text(profile);
                        self.encoder.text(entry);
                        self.encoder.count(authority.len());
                        for source in authority {
                            self.encoder.text(source);
                        }
                        self.encoder.id(library.as_u128());
                        self.encoder.text(source_key);
                        self.encoder.text(output_key);
                    }
                    DeclaredOrigin::Transformation {
                        transformation,
                        inputs,
                    } => {
                        self.encoder.tag(3);
                        self.encoder.id(transformation.as_u128());
                        self.encoder.count(inputs.len());
                        for input in inputs {
                            self.encoder.id(input.as_u128());
                        }
                    }
                    DeclaredOrigin::Generated { generator } => {
                        self.encoder.tag(4);
                        self.encoder.id(generator.as_u128());
                    }
                }
            }
        }
    }
    fn slot(&mut self, slot: &'m Slot) {
        self.encoder.slot_value(slot.value());
        self.origin(slot.origin());
    }
    fn record(&mut self, record: &'m ElementRecord) {
        self.encoder.tag(0);
        self.encoder.id(record.id().as_u128());
        self.encoder.id(record.metaclass().as_u128());
        self.origin(record.origin());
        self.encoder.count(record.slots().count());
        for (property, slot) in record.slots() {
            self.encoder.id(property.as_u128());
            self.slot(slot);
        }
    }
    fn occurrence(&mut self, occurrence: &'m AssociationOccurrence) {
        self.encoder.tag(1);
        self.encoder.id(occurrence.id().as_u128());
        self.encoder.id(occurrence.association().as_u128());
        self.encoder.count(occurrence.ends().len());
        for (end, participant) in occurrence.ends() {
            self.encoder.id(end.as_u128());
            self.encoder.id(participant.as_u128());
        }
        self.encoder.count(occurrence.positions().len());
        for (end, position) in occurrence.positions() {
            self.encoder.id(end.as_u128());
            self.encoder.count(*position);
        }
        self.origin(occurrence.origin());
    }
    fn failure(&mut self, failure: &'m ComputationFailure) {
        match failure {
            ComputationFailure::Incomplete {
                reason,
                explanation,
                searches,
            } => {
                self.encoder.tag(0);
                self.encoder.tag(match reason {
                    IncompleteReason::MissingInput => 0,
                    IncompleteReason::UnsupportedRuntimeSemantics => 1,
                    IncompleteReason::IncompleteDependency => 2,
                });
                self.proof(explanation);
                self.encoder.searches(searches);
            }
            ComputationFailure::Invalid {
                diagnostic,
                explanation,
                searches,
            } => {
                self.encoder.tag(1);
                self.encoder.text(diagnostic);
                self.proof(explanation);
                self.encoder.searches(searches);
            }
        }
    }
    fn model(&mut self, model: &'m ModelView) {
        for record in model.elements() {
            self.record(record);
        }
        for occurrence in model.association_occurrences() {
            self.occurrence(occurrence);
        }
        for ((element, property), slot) in model.derived_navigation_results() {
            self.encoder.tag(2);
            self.encoder.id(element.as_u128());
            self.encoder.id(property.as_u128());
            self.slot(slot);
        }
        for ((element, property), failure) in model.computation_failures() {
            self.encoder.tag(3);
            self.encoder.id(element.as_u128());
            self.encoder.id(property.as_u128());
            self.failure(failure);
        }
        for (fact, searches) in model.computation_searches() {
            self.encoder.tag(4);
            self.encoder.fact(*fact);
            self.encoder.searches(searches);
        }
    }
    fn finish(mut self) -> [u8; 32] {
        self.encoder.tag(255);
        self.encoder.finish()
    }
}

#[cfg(test)]
#[path = "../tests/unit/context_digest.rs"]
mod tests;
