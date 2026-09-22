//! Scheduler-issued evidence for exhaustive semantic conclusions.
//!
//! Descriptors declare potential writes, independently of observed outputs.
//! Certificates are immutable, exact-frontier sidecars, never model facts.
use crate::{Completeness, ResultStructureStratum, SemanticContextId};
use agq_kernel::{ElementId, MetaclassId, ModelView, PropertyId};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Stable rule-family identity, independent of work order and implementation address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProducerFamilyId(&'static str);
impl ProducerFamilyId {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }
    pub const fn name(self) -> &'static str {
        self.0
    }
}

/// Categories of possible semantic writes, including writes not emitted this run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerEffect {
    Membership,
    Specialization,
    Subsetting,
    Redefinition,
    Typing,
    Featuring,
    Naming,
    FeatureChain,
    ResultStructure,
    ValueBinding,
    ConnectorStructure,
    Scalar(PropertyId),
}

/// Static metaclass applicability. Dynamic antecedents are evaluated, not guessed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerApplicability {
    Any,
    Never,
    Subtypes(Vec<MetaclassId>),
}
impl ProducerApplicability {
    pub fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        match self {
            Self::Any => true,
            Self::Never => false,
            Self::Subtypes(classes) => classes
                .iter()
                .any(|&base| model.registry().is_subtype(class, base).unwrap_or(false)),
        }
    }
}

/// Declared write boundary. A family using `SubjectAndOwned` may change the
/// subject and its transitive owned records (including newly created records).
/// Families with other cross-subject writes must use `Model`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProducerEffectScope {
    SubjectAndOwned,
    Model,
}

/// Immutable declaration under one semantic rule-set identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProducerDescriptor {
    pub id: ProducerFamilyId,
    pub effects: BTreeSet<ProducerEffect>,
    pub applicability: ProducerApplicability,
    pub scope: ProducerEffectScope,
    pub minimum_stratum: ResultStructureStratum,
}
impl ProducerDescriptor {
    pub fn new(
        id: ProducerFamilyId,
        effects: impl IntoIterator<Item = ProducerEffect>,
        applicability: ProducerApplicability,
    ) -> Self {
        Self {
            id,
            effects: effects.into_iter().collect(),
            applicability,
            scope: ProducerEffectScope::SubjectAndOwned,
            minimum_stratum: ResultStructureStratum::Structural,
        }
    }
}

/// Explicit effect requirements of exhaustive queries. Positive witnesses do
/// not require this boundary merely because further positive facts may appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticClosureRequirement {
    EffectiveTyping,
    EffectiveFeaturing,
    EffectiveMembership,
    EffectiveNaming,
    ValueContext,
}
impl SemanticClosureRequirement {
    pub const ALL: [Self; 5] = [
        Self::EffectiveTyping,
        Self::EffectiveFeaturing,
        Self::EffectiveMembership,
        Self::EffectiveNaming,
        Self::ValueContext,
    ];
    pub fn requires(self, effect: ProducerEffect) -> bool {
        use ProducerEffect as E;
        match self {
            Self::EffectiveTyping => matches!(
                effect,
                E::Typing | E::Subsetting | E::Redefinition | E::Specialization | E::FeatureChain
            ),
            Self::EffectiveFeaturing => matches!(
                effect,
                E::Featuring
                    | E::Membership
                    | E::Specialization
                    | E::Subsetting
                    | E::Redefinition
                    | E::FeatureChain
                    | E::Scalar(_)
            ),
            Self::EffectiveMembership => matches!(
                effect,
                E::Membership
                    | E::Specialization
                    | E::Subsetting
                    | E::Redefinition
                    | E::ResultStructure
            ),
            Self::EffectiveNaming => matches!(
                effect,
                E::Naming | E::Membership | E::Specialization | E::Subsetting | E::Redefinition
            ),
            Self::ValueContext => {
                Self::EffectiveFeaturing.requires(effect)
                    || matches!(
                        effect,
                        E::Typing | E::ValueBinding | E::ConnectorStructure | E::ResultStructure
                    )
            }
        }
    }
    fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A complete evaluation is closed only after its reads are quiescent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ProducerEvaluationState {
    Inapplicable = 0,
    Pending = 1,
    EvaluatedComplete = 2,
    EvaluatedIncomplete = 3,
}

/// Deterministic canonical registry. Duplicate identities are rejected even if
/// implementations happen to emit identical facts on one graph.
#[derive(Clone, Debug)]
pub struct ProducerRegistry {
    descriptors: Box<[ProducerDescriptor]>,
    digest: [u8; 32],
}
impl ProducerRegistry {
    pub fn new(
        descriptors: impl IntoIterator<Item = ProducerDescriptor>,
    ) -> Result<Self, ProducerFamilyId> {
        let mut descriptors: Vec<_> = descriptors.into_iter().collect();
        for descriptor in &mut descriptors {
            if let ProducerApplicability::Subtypes(classes) = &mut descriptor.applicability {
                classes.sort();
                classes.dedup();
            }
        }
        descriptors.sort_by_key(|d| d.id);
        for pair in descriptors.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(pair[0].id);
            }
        }
        let mut hash = Sha256::new();
        hash.update(b"agq-producer-registry/1");
        // Debug is deterministic for these ordered value-only declarations;
        // its encoding is versioned by the registry schema above.
        hash.update(format!("{descriptors:?}").as_bytes());
        Ok(Self {
            descriptors: descriptors.into_boxed_slice(),
            digest: hash.finalize().into(),
        })
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn descriptors(&self) -> &[ProducerDescriptor] {
        &self.descriptors
    }
    pub(crate) fn index(&self, id: ProducerFamilyId) -> Option<usize> {
        self.descriptors.binary_search_by_key(&id, |d| d.id).ok()
    }
}

/// Compressed evaluation table retained only by the scheduler. The outer index
/// is one entry per subject; there is no tree node per subject/family pair.
#[derive(Default)]
pub(crate) struct ProducerEvaluationTable {
    rows: BTreeMap<ElementId, Vec<ProducerEvaluationState>>,
}
impl ProducerEvaluationTable {
    pub(crate) fn pending(
        &mut self,
        subject: ElementId,
        model: &ModelView,
        registry: &ProducerRegistry,
    ) {
        let Some(record) = model.element(subject) else {
            return;
        };
        self.rows.insert(
            subject,
            registry
                .descriptors
                .iter()
                .map(|d| {
                    if d.applicability.applies(model, record.metaclass()) {
                        ProducerEvaluationState::Pending
                    } else {
                        ProducerEvaluationState::Inapplicable
                    }
                })
                .collect(),
        );
    }
    pub(crate) fn record(
        &mut self,
        evaluations: &[(ElementId, ProducerFamilyId, Completeness)],
        registry: &ProducerRegistry,
    ) {
        for &(subject, family, completeness) in evaluations {
            if let Some(index) = registry.index(family)
                && let Some(row) = self.rows.get_mut(&subject)
            {
                row[index] = if completeness == Completeness::Complete {
                    ProducerEvaluationState::EvaluatedComplete
                } else {
                    ProducerEvaluationState::EvaluatedIncomplete
                };
            }
        }
    }
}

/// Immutable proof that declared producers for an exact graph have no pending
/// work capable of changing the covered answer. Construction is crate-private;
/// ordinary query callers can only inspect scheduler-issued certificates.
#[derive(Clone, Debug)]
pub struct ProducerClosureCertificate {
    model_digest: [u8; 32],
    registry_digest: [u8; 32],
    context_contract_digest: [u8; 32],
    digest: [u8; 32],
    subjects: Box<[ElementId]>,
    closed: Box<[u8]>,
    states: Box<[u8]>,
    families: usize,
    applicable_pairs: usize,
    closed_pairs: usize,
    incomplete_pairs: usize,
}
impl ProducerClosureCertificate {
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn model_digest(&self) -> [u8; 32] {
        self.model_digest
    }
    pub fn registry_digest(&self) -> [u8; 32] {
        self.registry_digest
    }
    pub fn context_contract_digest(&self) -> [u8; 32] {
        self.context_contract_digest
    }
    pub fn compatible_context(&self, context: &SemanticContextId) -> bool {
        self.model_digest == context.model_digest
            && self.context_contract_digest == context.closure_contract_digest()
            && context.producer_registry_digest == Some(self.registry_digest)
    }
    pub fn is_closed(&self, subject: ElementId, requirement: SemanticClosureRequirement) -> bool {
        self.subjects
            .binary_search(&subject)
            .is_ok_and(|index| self.closed[index] & requirement.bit() != 0)
    }
    pub fn evaluation(
        &self,
        subject: ElementId,
        family_index: usize,
    ) -> Option<ProducerEvaluationState> {
        if family_index >= self.families {
            return None;
        }
        let index = self.subjects.binary_search(&subject).ok()? * self.families + family_index;
        Some(match (self.states[index / 4] >> ((index % 4) * 2)) & 3 {
            0 => ProducerEvaluationState::Inapplicable,
            1 => ProducerEvaluationState::Pending,
            2 => ProducerEvaluationState::EvaluatedComplete,
            _ => ProducerEvaluationState::EvaluatedIncomplete,
        })
    }
    pub fn storage_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of_val(self.subjects.as_ref())
            + self.closed.len()
            + self.states.len()
    }
    pub fn applicable_pairs(&self) -> usize {
        self.applicable_pairs
    }
    pub fn closed_pairs(&self) -> usize {
        self.closed_pairs
    }
    pub fn incomplete_pairs(&self) -> usize {
        self.incomplete_pairs
    }
    pub fn closed_effects(&self) -> usize {
        self.closed
            .iter()
            .map(|mask| mask.count_ones() as usize)
            .sum()
    }

    pub(crate) fn issue(
        model: &ModelView,
        context: &SemanticContextId,
        registry: &ProducerRegistry,
        table: &ProducerEvaluationTable,
        immutable: impl Fn(ElementId) -> bool,
    ) -> Self {
        use agq_kerml::{classes as c, properties as p};
        let subjects: Vec<_> = model.elements().map(|r| r.id()).collect();
        let positions: BTreeMap<_, _> = subjects
            .iter()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        let families = registry.descriptors.len();
        let mut states = vec![0; (subjects.len() * families).div_ceil(4)];
        let mut blocked = vec![0_u8; subjects.len()];
        let mut inherited_blocks = vec![0_u8; subjects.len()];
        let mut global_block = 0;
        let mut applicable_pairs = 0;
        let mut closed_pairs = 0;
        let mut incomplete_pairs = 0;
        for (i, &subject) in subjects.iter().enumerate() {
            let record = model.element(subject).expect("indexed subject");
            for (j, descriptor) in registry.descriptors.iter().enumerate() {
                let state = if immutable(subject) {
                    ProducerEvaluationState::Inapplicable
                } else {
                    table
                        .rows
                        .get(&subject)
                        .map(|row| row[j])
                        .unwrap_or_else(|| {
                            if descriptor.applicability.applies(model, record.metaclass()) {
                                ProducerEvaluationState::Pending
                            } else {
                                ProducerEvaluationState::Inapplicable
                            }
                        })
                };
                let index = i * families + j;
                states[index / 4] |= (state as u8) << ((index % 4) * 2);
                applicable_pairs += usize::from(state != ProducerEvaluationState::Inapplicable);
                closed_pairs += usize::from(state == ProducerEvaluationState::EvaluatedComplete);
                incomplete_pairs +=
                    usize::from(state == ProducerEvaluationState::EvaluatedIncomplete);
                if matches!(
                    state,
                    ProducerEvaluationState::Pending | ProducerEvaluationState::EvaluatedIncomplete
                ) {
                    let mask = SemanticClosureRequirement::ALL
                        .into_iter()
                        .filter(|r| descriptor.effects.iter().any(|&e| r.requires(e)))
                        .fold(0, |mask, r| mask | r.bit());
                    // Other query families have additional domain/member dependencies.
                    // Until their precise footprint traversal is implemented,
                    // a relevant unfinished producer conservatively blocks them
                    // throughout the graph instead of claiming local absence.
                    global_block |= mask & !SemanticClosureRequirement::EffectiveTyping.bit();
                    match descriptor.scope {
                        ProducerEffectScope::Model => global_block |= mask,
                        ProducerEffectScope::SubjectAndOwned => inherited_blocks[i] |= mask,
                    }
                }
            }
        }
        let refs = |subject, property| -> Vec<ElementId> {
            model
                .navigation_slot(subject, property)
                .map(|slot| {
                    slot.value()
                        .values()
                        .filter_map(|v| {
                            if let agq_kernel::value::Value::Reference(id) = v {
                                Some(*id)
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        // Propagate a producer's declared write scope down canonical ownership.
        let mut owned_by = vec![Vec::new(); subjects.len()];
        for (i, &subject) in subjects.iter().enumerate() {
            for property in [
                p::ELEMENT_OWNED_RELATIONSHIP,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ] {
                for child in refs(subject, property) {
                    if let Some(&j) = positions.get(&child) {
                        owned_by[i].push(j);
                    }
                }
            }
        }
        propagate(&mut inherited_blocks, &owned_by);
        for (i, mask) in inherited_blocks.into_iter().enumerate() {
            blocked[i] = mask | global_block;
        }
        // Exhaustive typing follows canonical specialization/conjugation/chains.
        // Include all specialization subtypes conservatively, including incoming
        // nonowned relationships. This catches a delayed producer on a target.
        let mut dependents = vec![Vec::new(); subjects.len()];
        for record in model.elements() {
            let class = record.metaclass();
            let is = |base| model.registry().is_subtype(class, base).unwrap_or(false);
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
                for source in refs(record.id(), source) {
                    for target in refs(record.id(), target) {
                        if let (Some(&i), Some(&j)) =
                            (positions.get(&source), positions.get(&target))
                        {
                            dependents[j].push(i);
                        }
                    }
                }
            }
            for property in [p::FEATURE_TYPE, p::FEATURE_CHAINING_FEATURE] {
                for target in refs(record.id(), property) {
                    if let Some(&j) = positions.get(&target) {
                        dependents[j].push(positions[&record.id()]);
                    }
                }
            }
        }
        propagate(&mut blocked, &dependents);
        let closed: Vec<_> = blocked.into_iter().map(|mask| !mask & 31).collect();
        let context_contract_digest = context.closure_contract_digest();
        let mut hash = Sha256::new();
        hash.update(b"agq-producer-closure-certificate/1");
        hash.update(context.model_digest);
        hash.update(registry.digest);
        hash.update(context_contract_digest);
        for subject in &subjects {
            hash.update(subject.as_u128().to_be_bytes());
        }
        hash.update(&closed);
        hash.update(&states);
        Self {
            model_digest: context.model_digest,
            registry_digest: registry.digest,
            context_contract_digest,
            digest: hash.finalize().into(),
            subjects: subjects.into_boxed_slice(),
            closed: closed.into_boxed_slice(),
            states: states.into_boxed_slice(),
            families,
            applicable_pairs,
            closed_pairs,
            incomplete_pairs,
        }
    }
}

fn propagate(masks: &mut [u8], dependents: &[Vec<usize>]) {
    let mut queue: VecDeque<_> = masks
        .iter()
        .enumerate()
        .filter_map(|(i, &mask)| (mask != 0).then_some(i))
        .collect();
    while let Some(index) = queue.pop_front() {
        for &dependent in &dependents[index] {
            let next = masks[dependent] | masks[index];
            if next != masks[dependent] {
                masks[dependent] = next;
                queue.push_back(dependent);
            }
        }
    }
}
