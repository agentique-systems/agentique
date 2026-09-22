//! Scheduler-issued evidence for exhaustive semantic conclusions.
//!
//! Descriptors declare potential writes, independently of observed outputs.
//! Certificates are immutable, exact-frontier sidecars, never model facts.
use crate::{Completeness, ResultStructureStratum, SemanticContextId};
use agq_kernel::{ElementId, MetaclassId, ModelView, PropertyId};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

/// Precise scalar reads coexist with conservative structural population reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ProducerRead {
    Property(ElementId, PropertyId),
    Structural(ElementId),
    Source(ElementId, MetaclassId, PropertyId),
    Owned(ElementId, MetaclassId),
    Inverse,
    Any(ElementId),
    Global,
    Requirement(ElementId, SemanticClosureRequirement),
}
pub(crate) type ProducerReads = Arc<[ProducerRead]>;
pub(crate) fn producer_reads<T>(
    answer: &crate::QueryResult<T>,
    model: &ModelView,
) -> ProducerReads {
    use crate::SearchDependency as S;
    use agq_kernel::{derived::StructuralSearch as K, provenance::FactKey};
    let mut result = BTreeSet::new();
    let mut kernel = |search: &K| match search {
        K::Property { element, property } => {
            result.insert(ProducerRead::Property(*element, *property));
        }
        K::Element(id) => {
            result.insert(ProducerRead::Any(*id));
        }
        K::ElementIdentity(id) => {
            if model.element(*id).is_none() {
                result.insert(ProducerRead::Global);
            }
        }
        K::Incoming(_) | K::Association { .. } => {
            result.insert(ProducerRead::Inverse);
        }
        K::SourceRelationships {
            source,
            class,
            property,
        } => {
            result.insert(ProducerRead::Source(*source, *class, *property));
        }
        K::OwnedRelationships { owner, class } => {
            result.insert(ProducerRead::Owned(*owner, *class));
        }
        K::ProducerClosure {
            subject,
            requirement,
            ..
        } => {
            result.insert(
                SemanticClosureRequirement::from_contract_id(requirement)
                    .map(|requirement| ProducerRead::Requirement(*subject, requirement))
                    .unwrap_or(ProducerRead::Global),
            );
        }
        K::Model => {
            result.insert(ProducerRead::Global);
        }
        K::DescriptorGraph => {}
    };
    for search in answer.shared_search_dependencies.iter() {
        kernel(search);
    }
    for search in &answer.search_dependencies {
        if let S::Kernel(search) = search {
            kernel(search);
        }
    }
    for search in &answer.search_dependencies {
        match search {
            S::PropertySet { element, property } => {
                result.insert(ProducerRead::Property(*element, *property));
            }
            S::Incoming { .. } => {
                result.insert(ProducerRead::Inverse);
            }
            S::SourceRelationships {
                source,
                class,
                property,
            } => {
                result.insert(ProducerRead::Source(*source, *class, *property));
            }
            S::OwnedRelationships { owner, class } => {
                result.insert(ProducerRead::Owned(*owner, *class));
            }
            S::NamespaceMembers { namespace } | S::ImportSet { namespace } => {
                result.insert(ProducerRead::Structural(*namespace));
            }
            S::ImportedNamespace { import, namespace } => {
                result.insert(ProducerRead::Structural(*import));
                result.insert(ProducerRead::Structural(*namespace));
            }
            S::RedefinitionScope {
                relationship,
                namespace,
                ..
            } => {
                result.insert(ProducerRead::Structural(*relationship));
                result.insert(ProducerRead::Structural(*namespace));
            }
            S::Instances { .. } => {
                result.insert(ProducerRead::Global);
            }
            S::Element(id) if model.element(*id).is_none() => {
                result.insert(ProducerRead::Global);
            }
            S::ProducerClosure {
                subject,
                requirement,
                ..
            } => {
                result.insert(ProducerRead::Requirement(*subject, *requirement));
            }
            _ => {}
        }
    }
    for fact in &answer.positive_dependencies {
        if let FactKey::Property { element, property } = fact {
            // An inverse ownership query reads one existing child's owner, not
            // arbitrary additions to the owner's backing collection. Its exact
            // canonical carrier fact remains in proof; the search defines the
            // observed projection. Explicit broad reads still take precedence.
            if !result.contains(&ProducerRead::Property(*element, *property))
                && result.iter().any(|read| match read {
                    ProducerRead::Source(child, _, backing)
                        if backing == property
                            && [
                                agq_kerml::properties::ELEMENT_OWNED_RELATIONSHIP,
                                agq_kerml::properties::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                            ]
                            .contains(backing) =>
                    {
                        model
                            .navigation_slot(*element, *property)
                            .is_some_and(|slot| {
                                slot.value().values().any(|value| {
                                    *value == agq_kernel::value::Value::Reference(*child)
                                })
                            })
                    }
                    _ => false,
                })
            {
                continue;
            }
            if *property == agq_kerml::properties::ELEMENT_OWNED_RELATIONSHIP
                && result
                    .iter()
                    .any(|read| matches!(read, ProducerRead::Owned(owner, _) if owner == element))
                && !result.contains(&ProducerRead::Property(*element, *property))
            {
                continue;
            }
            result.insert(ProducerRead::Property(*element, *property));
        }
    }
    result.into_iter().collect::<Vec<_>>().into()
}

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
    /// Changing the owner of a semantic subject already present in the graph.
    Ownership,
    Specialization,
    Conjugation,
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
    Subject,
    SubjectAndOwned,
    /// Transitive owned records, excluding the producer subject itself.
    OwnedDescendants,
    /// Subject and current transitive canonical owners. If the registry can
    /// change an existing subject's ownership, this expands to `Model`.
    SubjectAndOwners,
    Model,
}

/// Immutable declaration under one semantic rule-set identity.
///
/// This is a trusted implementation contract. Every potential existing-subject
/// write must be covered regardless of whether an observed evaluation emits it.
/// Registering an extension changes the context identity; effect audit checks
/// provide defense in depth and cannot establish this promise by observation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProducerDescriptor {
    pub id: ProducerFamilyId,
    /// Effects on semantic subjects already present in the input frontier.
    pub effects: BTreeSet<ProducerEffect>,
    /// Effects exclusively on new semantic subjects, not merely new relationship
    /// records. A fresh FeatureTyping targeting an existing Feature belongs in
    /// `effects`, even though its relationship identity is new.
    pub fresh_effects: BTreeSet<ProducerEffect>,
    /// Optional exact metaclasses of relationship records this family may
    /// create. This bounds potential output, not the records observed so far.
    /// `None` permits every relationship class covered by its effects.
    pub relationship_classes: Option<BTreeSet<MetaclassId>>,
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
            fresh_effects: BTreeSet::new(),
            relationship_classes: None,
            applicability,
            scope: ProducerEffectScope::SubjectAndOwned,
            minimum_stratum: ResultStructureStratum::Structural,
        }
    }
    fn can_create_subjects(&self) -> bool {
        !self.fresh_effects.is_empty()
            || self
                .effects
                .iter()
                .any(|effect| !matches!(effect, ProducerEffect::Scalar(_)))
    }
}

/// A pending creator can activate families with no subjects in the current
/// frontier. Their cross-subject effects cannot be omitted from the witness.
fn future_cross_subject_effects(registry: &ProducerRegistry) -> BTreeSet<ProducerEffect> {
    future_cross_subject_families(registry)
        .flat_map(|descriptor| descriptor.effects.iter().copied())
        .collect()
}
fn future_cross_subject_families(
    registry: &ProducerRegistry,
) -> impl Iterator<Item = &ProducerDescriptor> {
    registry.descriptors.iter().filter(|descriptor| {
        descriptor.applicability != ProducerApplicability::Never
            && matches!(
                descriptor.scope,
                ProducerEffectScope::Model | ProducerEffectScope::SubjectAndOwners
            )
    })
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
    EffectiveOwnership,
}
impl SemanticClosureRequirement {
    pub const fn contract_id(self) -> &'static str {
        match self {
            Self::EffectiveTyping => "agq-semantic-closure/EffectiveTyping/1",
            Self::EffectiveFeaturing => "agq-semantic-closure/EffectiveFeaturing/1",
            Self::EffectiveMembership => "agq-semantic-closure/EffectiveMembership/1",
            Self::EffectiveNaming => "agq-semantic-closure/EffectiveNaming/1",
            Self::ValueContext => "agq-semantic-closure/ValueContext/1",
            Self::EffectiveOwnership => "agq-semantic-closure/EffectiveOwnership/1",
        }
    }
    pub fn from_contract_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|requirement| requirement.contract_id() == id)
    }
    pub const ALL: [Self; 6] = [
        Self::EffectiveTyping,
        Self::EffectiveFeaturing,
        Self::EffectiveMembership,
        Self::EffectiveNaming,
        Self::ValueContext,
        Self::EffectiveOwnership,
    ];
    pub fn requires(self, effect: ProducerEffect) -> bool {
        use ProducerEffect as E;
        match self {
            Self::EffectiveOwnership => matches!(effect, E::Ownership),
            Self::EffectiveTyping => matches!(
                effect,
                E::Typing
                    | E::Subsetting
                    | E::Redefinition
                    | E::Specialization
                    | E::Conjugation
                    | E::FeatureChain
            ),
            Self::EffectiveFeaturing => matches!(
                effect,
                E::Featuring
                    | E::Membership
                    | E::Specialization
                    | E::Conjugation
                    | E::Subsetting
                    | E::Redefinition
                    | E::FeatureChain
                    | E::Scalar(_)
            ),
            Self::EffectiveMembership => matches!(
                effect,
                E::Membership
                    | E::Specialization
                    | E::Conjugation
                    | E::Subsetting
                    | E::Redefinition
                    | E::ResultStructure
            ),
            Self::EffectiveNaming => matches!(
                effect,
                E::Naming
                    | E::Membership
                    | E::Specialization
                    | E::Conjugation
                    | E::Subsetting
                    | E::Redefinition
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
        hash.update(b"agq-producer-registry/2");
        // Debug is deterministic for these ordered value-only declarations;
        // its encoding is versioned by the registry schema above.
        hash.update(format!("{descriptors:?}").as_bytes());
        // Requirement semantics are part of the registry contract, not an
        // unversioned query implementation detail. Include every declared
        // potential effect, including effects restricted to fresh subjects.
        let effects: BTreeSet<_> = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.effects.iter().chain(&descriptor.fresh_effects))
            .copied()
            .collect();
        for requirement in SemanticClosureRequirement::ALL {
            hash.update(requirement.contract_id().as_bytes());
            for &effect in &effects {
                hash.update(format!("{effect:?}").as_bytes());
                hash.update([u8::from(requirement.requires(effect))]);
            }
        }
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
    reads: BTreeMap<ElementId, Vec<(usize, ProducerReads)>>,
}
impl ProducerEvaluationTable {
    /// Least fixed point: an evaluated producer is not quiescent while an
    /// unfinished producer can change one of its actual query reads.
    fn dependency_blocked(
        &self,
        model: &ModelView,
        registry: &ProducerRegistry,
        subjects: &[ElementId],
        positions: &BTreeMap<ElementId, usize>,
        immutable: &impl Fn(ElementId) -> bool,
    ) -> BTreeSet<usize> {
        let families = registry.descriptors.len();
        let future_effects = future_cross_subject_effects(registry);
        let future_families: Vec<_> = future_cross_subject_families(registry).collect();
        let mut blocked = BTreeSet::new();
        let mut pending = VecDeque::new();
        for (i, &subject) in subjects.iter().enumerate() {
            if immutable(subject) {
                continue;
            }
            let class = model.element(subject).expect("subject").metaclass();
            for (j, descriptor) in registry.descriptors.iter().enumerate() {
                let state = self
                    .rows
                    .get(&subject)
                    .map(|row| row[j])
                    .unwrap_or_else(|| {
                        if descriptor.applicability.applies(model, class) {
                            ProducerEvaluationState::Pending
                        } else {
                            ProducerEvaluationState::Inapplicable
                        }
                    });
                if matches!(
                    state,
                    ProducerEvaluationState::Pending | ProducerEvaluationState::EvaluatedIncomplete
                ) {
                    let pair = i * families + j;
                    blocked.insert(pair);
                    pending.push_back(pair);
                }
            }
        }
        if pending.is_empty() {
            return blocked;
        }
        let mut readers: BTreeMap<ElementId, Vec<(ProducerRead, usize)>> = BTreeMap::new();
        let mut global = Vec::new();
        let mut inverse = Vec::new();
        for (&subject, row) in &self.rows {
            let i = positions[&subject];
            for (j, state) in row.iter().enumerate() {
                if *state != ProducerEvaluationState::EvaluatedComplete {
                    continue;
                }
                let pair = i * families + j;
                let reads = self
                    .reads
                    .get(&subject)
                    .and_then(|reads| reads.iter().find(|(family, _)| *family == j))
                    .map(|(_, reads)| reads.as_ref());
                if let Some(reads) = reads {
                    for read in reads {
                        // Protected dependency records and their ownership
                        // collections cannot change. Arbitrary inverse/source
                        // relationship searches remain open: local carriers
                        // may refer to a dependency without writing its record.
                        let fixed = match read {
                            ProducerRead::Any(id)
                            | ProducerRead::Structural(id)
                            | ProducerRead::Owned(id, _) => immutable(*id),
                            ProducerRead::Property(id, property) => model.element(*id).and_then(|record| record.slot(*property)).is_some_and(|slot| matches!(slot.value(), agq_kernel::value::SlotValue::Scalar(_))) || immutable(*id)
                                && (model
                                    .element(*id)
                                    .is_some_and(|record| record.slot(*property).is_some())
                                    || model
                                        .registry()
                                        .property(*property)
                                        .is_ok_and(|descriptor| descriptor.composite)
                                    || [
                                        agq_kerml::properties::ELEMENT_OWNING_RELATIONSHIP,
                                        agq_kerml::properties::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                                    ]
                                    .contains(property)),
                            ProducerRead::Source(id, _, property) => {
                                immutable(*id)
                                    && [
                                        agq_kerml::properties::ELEMENT_OWNED_RELATIONSHIP,
                                        agq_kerml::properties::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                                    ]
                                    .contains(property)
                            }
                            _ => false,
                        };
                        if fixed {
                            continue;
                        }
                        match read {
                            ProducerRead::Global => global.push(pair),
                            ProducerRead::Inverse => inverse.push(pair),
                            ProducerRead::Property(id, _)
                            | ProducerRead::Source(id, _, _)
                            | ProducerRead::Owned(id, _)
                            | ProducerRead::Structural(id)
                            | ProducerRead::Any(id)
                            | ProducerRead::Requirement(id, _) => {
                                readers.entry(*id).or_default().push((read.clone(), pair))
                            }
                        }
                    }
                } else {
                    global.push(pair);
                }
            }
        }
        use agq_kerml::properties as p;
        let mut owned: BTreeMap<ElementId, Vec<ElementId>> = BTreeMap::new();
        let mut owners: BTreeMap<ElementId, Vec<ElementId>> = BTreeMap::new();
        let ownership_mutable = registry
            .descriptors
            .iter()
            .any(|descriptor| descriptor.effects.contains(&ProducerEffect::Ownership));
        for record in model.elements() {
            for property in [
                p::ELEMENT_OWNED_RELATIONSHIP,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ] {
                if let Some(slot) = model.navigation_slot(record.id(), property) {
                    for value in slot.value().values() {
                        if let agq_kernel::value::Value::Reference(child) = value {
                            owned.entry(record.id()).or_default().push(*child);
                            owners.entry(*child).or_default().push(record.id());
                        }
                    }
                }
            }
        }
        let trace = std::env::var_os("AGQ_PRODUCER_CAUSAL_TRACE").is_some();
        let mut future_effects_applied = false;
        while let Some(pair) = pending.pop_front() {
            let subject = subjects[pair / families];
            let descriptor = &registry.descriptors[pair % families];
            if descriptor.effects.is_empty() && descriptor.fresh_effects.is_empty() {
                continue;
            }
            let mut affected = std::mem::take(&mut global);
            if descriptor.can_create_subjects()
                && !future_effects_applied
                && !future_effects.is_empty()
            {
                future_effects_applied = true;
                for reads in readers.values() {
                    for (read, reader) in reads {
                        if future_families.iter().any(|future| {
                            future
                                .effects
                                .iter()
                                .any(|&effect| descriptor_changes_read(future, effect, read, model))
                        }) {
                            if trace && !blocked.contains(reader) {
                                eprintln!(
                                    "closure future cause {subject:?}/{} -> {:?}/{} read={read:?}",
                                    descriptor.id.name(),
                                    subjects[*reader / families],
                                    registry.descriptors[*reader % families].id.name()
                                );
                            }
                            affected.push(*reader);
                        }
                    }
                }
            }
            if descriptor
                .effects
                .iter()
                .chain(&descriptor.fresh_effects)
                .any(|effect| !matches!(effect, ProducerEffect::Scalar(_)))
            {
                affected.append(&mut inverse);
            }
            let mut consume = |reads: &[(ProducerRead, usize)]| {
                for (read, reader) in reads {
                    if descriptor
                        .effects
                        .iter()
                        .any(|&effect| descriptor_changes_read(descriptor, effect, read, model))
                    {
                        if trace && !blocked.contains(reader) {
                            eprintln!(
                                "closure direct cause {subject:?}/{} -> {:?}/{} read={read:?}",
                                descriptor.id.name(),
                                subjects[*reader / families],
                                registry.descriptors[*reader % families].id.name()
                            );
                        }
                        affected.push(*reader);
                    }
                }
            };
            if descriptor.scope == ProducerEffectScope::Model
                || (ownership_mutable && descriptor.scope == ProducerEffectScope::SubjectAndOwners)
            {
                for reads in readers.values() {
                    consume(reads);
                }
            } else {
                let mut scope = vec![subject];
                let mut seen = BTreeSet::new();
                while let Some(source) = scope.pop() {
                    if !seen.insert(source) {
                        continue;
                    }
                    if (descriptor.scope != ProducerEffectScope::OwnedDescendants
                        || source != subject)
                        && let Some(reads) = readers.get(&source)
                    {
                        consume(reads);
                    }
                    if matches!(
                        descriptor.scope,
                        ProducerEffectScope::SubjectAndOwned
                            | ProducerEffectScope::OwnedDescendants
                    ) && let Some(children) = owned.get(&source)
                    {
                        scope.extend(children);
                    }
                    if descriptor.scope == ProducerEffectScope::SubjectAndOwners
                        && let Some(parents) = owners.get(&source)
                    {
                        scope.extend(parents);
                    }
                }
            }
            for reader in affected {
                if blocked.insert(reader) {
                    if trace {
                        eprintln!(
                            "closure dependency {:?}/{} -> {:?}/{}",
                            subject,
                            descriptor.id.name(),
                            subjects[reader / families],
                            registry.descriptors[reader % families].id.name()
                        );
                    }
                    pending.push_back(reader);
                }
            }
        }
        blocked
    }
    pub(crate) fn pending(
        &mut self,
        subject: ElementId,
        model: &ModelView,
        registry: &ProducerRegistry,
    ) {
        let Some(record) = model.element(subject) else {
            return;
        };
        self.reads.remove(&subject);
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
    pub(crate) fn record_reads(
        &mut self,
        reads: &[(ElementId, ProducerFamilyId, ProducerReads)],
        registry: &ProducerRegistry,
    ) {
        for (subject, family, reads) in reads {
            if let Some(index) = registry.index(*family) {
                let entries = self.reads.entry(*subject).or_default();
                if let Some((_, prior)) = entries.iter_mut().find(|(family, _)| *family == index) {
                    *prior = prior
                        .iter()
                        .chain(reads.iter())
                        .cloned()
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .into();
                } else {
                    entries.push((index, reads.clone()));
                }
            }
        }
    }
    pub(crate) fn record(
        &mut self,
        evaluations: &[(ElementId, ProducerFamilyId, Completeness)],
        registry: &ProducerRegistry,
    ) -> Result<(), agq_kernel::derived::DerivationError> {
        for &(subject, family, completeness) in evaluations {
            let Some(index) = registry.index(family) else {
                return Err(agq_kernel::derived::DerivationError::InputContextMismatch);
            };
            let Some(row) = self.rows.get_mut(&subject) else {
                return Err(agq_kernel::derived::DerivationError::InputContextMismatch);
            };
            {
                if row[index] == ProducerEvaluationState::Inapplicable {
                    return Err(agq_kernel::derived::DerivationError::InputContextMismatch);
                }
                row[index] = if completeness == Completeness::Complete
                    && row[index] != ProducerEvaluationState::EvaluatedIncomplete
                {
                    ProducerEvaluationState::EvaluatedComplete
                } else {
                    ProducerEvaluationState::EvaluatedIncomplete
                };
            }
        }
        Ok(())
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
    /// Export immutable proof bytes; acceptance and restoration require separately trusted authority.
    pub fn receipt_value(&self) -> serde_json::Value {
        serde_json::json!({
            "format": "agq-producer-closure-certificate/1",
            "model_digest": self.model_digest, "registry_digest": self.registry_digest,
            "context_contract_digest": self.context_contract_digest, "digest": self.digest,
            "subjects": self.subjects.iter().map(|id| id.as_u128().to_string()).collect::<Vec<_>>(),
            "closed": self.closed, "states": self.states, "families": self.families,
        })
    }
    /// Only the checked-in acceptance authority may call this restoration path.
    /// It validates exact graph/context/registry bindings and the compact proof
    /// bytes without replaying producers or accepting caller-provided labels.
    pub(crate) fn from_trusted_receipt(
        value: &serde_json::Value,
        context: &SemanticContextId,
        registry: &ProducerRegistry,
        model: &ModelView,
    ) -> Option<Self> {
        use serde_json::from_value;
        if value["format"] != "agq-producer-closure-certificate/1" {
            return None;
        }
        let model_digest: [u8; 32] = from_value(value["model_digest"].clone()).ok()?;
        let registry_digest: [u8; 32] = from_value(value["registry_digest"].clone()).ok()?;
        let context_contract_digest: [u8; 32] =
            from_value(value["context_contract_digest"].clone()).ok()?;
        let digest: [u8; 32] = from_value(value["digest"].clone()).ok()?;
        let subjects: Box<[_]> = from_value::<Vec<String>>(value["subjects"].clone())
            .ok()?
            .into_iter()
            .map(|id| id.parse::<u128>().ok().map(ElementId::from_u128))
            .collect::<Option<Vec<_>>>()?
            .into_boxed_slice();
        let closed: Box<[u8]> = from_value::<Vec<u8>>(value["closed"].clone())
            .ok()?
            .into_boxed_slice();
        let states: Box<[u8]> = from_value::<Vec<u8>>(value["states"].clone())
            .ok()?
            .into_boxed_slice();
        let families = usize::try_from(value["families"].as_u64()?).ok()?;
        if model_digest != context.model_digest
            || registry_digest != registry.digest()
            || context_contract_digest != context.closure_contract_digest()
            || families != registry.descriptors().len()
            || closed.len() != subjects.len()
            || states.len() != (subjects.len() * families).div_ceil(4)
            || !subjects
                .iter()
                .copied()
                .eq(model.elements().map(|r| r.id()))
            || closed.iter().any(|mask| *mask & !63 != 0)
        {
            return None;
        }
        if digest
            != certificate_digest(
                model_digest,
                registry_digest,
                context_contract_digest,
                &subjects,
                &closed,
                &states,
            )
        {
            return None;
        }
        let mut result = Self {
            model_digest,
            registry_digest,
            context_contract_digest,
            digest,
            subjects,
            closed,
            states,
            families,
            applicable_pairs: 0,
            closed_pairs: 0,
            incomplete_pairs: 0,
        };
        for &subject in &result.subjects {
            for family in 0..families {
                let state = result.evaluation(subject, family)?;
                result.applicable_pairs +=
                    usize::from(state != ProducerEvaluationState::Inapplicable);
                result.closed_pairs +=
                    usize::from(state == ProducerEvaluationState::EvaluatedComplete);
                result.incomplete_pairs +=
                    usize::from(state == ProducerEvaluationState::EvaluatedIncomplete);
            }
        }
        Some(result)
    }
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
        let future_effects = future_cross_subject_effects(registry);
        let mut states = vec![0; (subjects.len() * families).div_ceil(4)];
        let mut blocked = vec![0_u8; subjects.len()];
        let mut inherited_blocks = vec![0_u8; subjects.len()];
        let mut descendant_blocks = vec![0_u8; subjects.len()];
        let mut owner_blocks = vec![0_u8; subjects.len()];
        let ownership_mutable = registry
            .descriptors
            .iter()
            .any(|descriptor| descriptor.effects.contains(&ProducerEffect::Ownership));
        let mut global_block = 0;
        let mut applicable_pairs = 0;
        let mut closed_pairs = 0;
        let mut incomplete_pairs = 0;
        let dependency_blocked =
            table.dependency_blocked(model, registry, &subjects, &positions, &immutable);
        for (i, &subject) in subjects.iter().enumerate() {
            let record = model.element(subject).expect("indexed subject");
            for (j, descriptor) in registry.descriptors.iter().enumerate() {
                let mut state = if immutable(subject) {
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
                if state == ProducerEvaluationState::EvaluatedComplete
                    && dependency_blocked.contains(&(i * families + j))
                {
                    state = ProducerEvaluationState::Pending;
                }
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
                    if descriptor.can_create_subjects() {
                        for requirement in SemanticClosureRequirement::ALL {
                            if future_effects
                                .iter()
                                .any(|&effect| requirement.requires(effect))
                            {
                                global_block |= requirement.bit();
                            }
                        }
                    }
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
                        ProducerEffectScope::OwnedDescendants => descendant_blocks[i] |= mask,
                        ProducerEffectScope::SubjectAndOwners if ownership_mutable => {
                            global_block |= mask
                        }
                        ProducerEffectScope::SubjectAndOwners => owner_blocks[i] |= mask,
                        ProducerEffectScope::Subject => blocked[i] |= mask,
                    }
                }
            }
        }
        let refs = |subject, property| -> Vec<ElementId> {
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
        let mut owners_of = vec![Vec::new(); subjects.len()];
        for (i, &subject) in subjects.iter().enumerate() {
            for property in [
                p::ELEMENT_OWNED_RELATIONSHIP,
                p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ] {
                for child in refs(subject, property) {
                    if let Some(&j) = positions.get(&child) {
                        owned_by[i].push(j);
                        owners_of[j].push(i);
                    }
                }
            }
        }
        for (i, mask) in descendant_blocks.into_iter().enumerate() {
            for &child in &owned_by[i] {
                inherited_blocks[child] |= mask;
            }
        }
        propagate(&mut inherited_blocks, &owned_by);
        propagate(&mut owner_blocks, &owners_of);
        for (i, mask) in inherited_blocks.into_iter().enumerate() {
            blocked[i] |= mask | owner_blocks[i] | global_block;
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
                let mut sources = refs(record.id(), source);
                if sources.is_empty() && (is(c::REFERENCE_SUBSETTING) || is(c::FEATURE_CHAINING)) {
                    sources.extend(
                        model
                            .incoming_for_property(record.id(), p::ELEMENT_OWNED_RELATIONSHIP)
                            .map(|reference| reference.source),
                    );
                }
                for source in sources {
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
        // Implied metaclass bases participate even before a canonical edge is
        // materialized. Reuse the exact query role map rather than assuming
        // mutable binding targets are accepted immutable dependencies.
        if let Some(bindings) = &context.standard_bindings {
            for record in model.elements() {
                for (class, role) in crate::implicit::metaclass_library_role_specs() {
                    if model
                        .registry()
                        .is_subtype(record.metaclass(), class)
                        .unwrap_or(false)
                    {
                        let target = bindings.get(role);
                        if let Some(&j) = positions.get(&target) {
                            dependents[j].push(positions[&record.id()]);
                        }
                    }
                }
            }
        }
        propagate(&mut blocked, &dependents);
        let closed: Vec<_> = blocked.into_iter().map(|mask| !mask & 63).collect();
        let context_contract_digest = context.closure_contract_digest();
        let digest = certificate_digest(
            context.model_digest,
            registry.digest,
            context_contract_digest,
            &subjects,
            &closed,
            &states,
        );
        Self {
            model_digest: context.model_digest,
            registry_digest: registry.digest,
            context_contract_digest,
            digest,
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

fn certificate_digest(
    model: [u8; 32],
    registry: [u8; 32],
    contract: [u8; 32],
    subjects: &[ElementId],
    closed: &[u8],
    states: &[u8],
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"agq-producer-closure-certificate/1");
    hash.update(model);
    hash.update(registry);
    hash.update(contract);
    for subject in subjects {
        hash.update(subject.as_u128().to_be_bytes());
    }
    hash.update(closed);
    hash.update(states);
    hash.finalize().into()
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

pub(crate) fn descriptor_changes_read(
    descriptor: &ProducerDescriptor,
    effect: ProducerEffect,
    read: &ProducerRead,
    model: &ModelView,
) -> bool {
    if !effect_changes_read(effect, read, model) {
        return false;
    }
    if !matches!(
        effect,
        ProducerEffect::Scalar(_) | ProducerEffect::Ownership
    ) && let Some(classes) = &descriptor.relationship_classes
        && let ProducerRead::Source(_, class, _) | ProducerRead::Owned(_, class) = read
    {
        return classes.iter().any(|&output| {
            model.registry().is_subtype(output, *class).unwrap_or(false)
                && effect_changes_read(
                    effect,
                    &ProducerRead::Owned(ElementId::from_u128(0), output),
                    model,
                )
        });
    }
    true
}

pub(crate) fn effect_changes_read(
    effect: ProducerEffect,
    read: &ProducerRead,
    model: &ModelView,
) -> bool {
    match read {
        ProducerRead::Global | ProducerRead::Any(_) => true,
        ProducerRead::Requirement(_, requirement) => requirement.requires(effect),
        ProducerRead::Structural(_) | ProducerRead::Inverse => {
            !matches!(effect, ProducerEffect::Scalar(_))
        }
        ProducerRead::Source(_, _, property)
            if [
                agq_kerml::properties::ELEMENT_OWNED_RELATIONSHIP,
                agq_kerml::properties::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            ]
            .contains(property) =>
        {
            effect == ProducerEffect::Ownership
                || matches!(effect, ProducerEffect::Scalar(written) if written == *property)
        }
        ProducerRead::Source(_, class, _) | ProducerRead::Owned(_, class) => {
            use agq_kerml::classes as c;
            let potential: &[MetaclassId] = match effect {
                ProducerEffect::Typing => &[c::FEATURE_TYPING],
                ProducerEffect::Subsetting => {
                    &[c::SUBSETTING, c::REFERENCE_SUBSETTING, c::CROSS_SUBSETTING]
                }
                ProducerEffect::Redefinition => &[c::REDEFINITION],
                ProducerEffect::Specialization => &[c::SPECIALIZATION, c::SUBCLASSIFICATION],
                ProducerEffect::Conjugation => &[c::CONJUGATION],
                ProducerEffect::FeatureChain => &[c::FEATURE_CHAINING],
                ProducerEffect::Featuring => &[c::TYPE_FEATURING],
                ProducerEffect::Membership => &[c::MEMBERSHIP, c::FEATURE_MEMBERSHIP],
                _ => &[],
            };
            potential.iter().any(|&potential| {
                model
                    .registry()
                    .is_subtype(potential, *class)
                    .unwrap_or(false)
                    || model
                        .registry()
                        .is_subtype(*class, potential)
                        .unwrap_or(false)
            })
        }
        ProducerRead::Property(element, property) => {
            let resolved = model
                .element(*element)
                .and_then(|record| {
                    model
                        .registry()
                        .resolve_property(record.metaclass(), *property)
                        .ok()
                        .flatten()
                })
                .map_or(*property, |descriptor| descriptor.id);
            if let ProducerEffect::Scalar(written) = effect {
                return written == *property || written == resolved;
            }
            if [
                agq_kerml::properties::ELEMENT_OWNING_RELATIONSHIP,
                agq_kerml::properties::RELATIONSHIP_OWNING_RELATED_ELEMENT,
            ]
            .contains(property)
            {
                return effect == ProducerEffect::Ownership;
            }
            if effect == ProducerEffect::Naming {
                return true;
            }
            model.registry().property(resolved).is_ok_and(|property| {
                matches!(
                    model.registry().storage_kind(property.value_kind),
                    Ok(agq_kernel::metamodel::ValueKind::Reference(_))
                )
            })
        }
    }
}
