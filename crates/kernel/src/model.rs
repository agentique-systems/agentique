use crate::association::AssociationOccurrence;
use crate::metamodel::{
    MetamodelError, MetamodelRegistry, Multiplicity, PropertyDescriptor, ValueKind,
};
use crate::provenance::{DeclaredOrigin, FactKey, Origin};
use crate::value::{SlotValue, Value};
use crate::{
    AssociationId, AssociationOccurrenceId, ElementId, MetaclassId, PropertyId, RevisionId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type StagedRecords = (
    BTreeMap<ElementId, Arc<ElementRecord>>,
    BTreeSet<ElementId>,
    BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
    BTreeSet<AssociationOccurrenceId>,
);

/// A lower bound that must be satisfied before a construction can be published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionObligation {
    pub element: ElementId,
    pub property: PropertyId,
    pub required: Multiplicity,
    pub actual: usize,
}

/// An immutable, unpublished candidate. This type cannot be used as a Snapshot.
#[derive(Clone, Debug)]
pub struct ConstructionView {
    revision: RevisionId,
    model: ModelView,
    obligations: Vec<ConstructionObligation>,
}
impl ConstructionView {
    /// Transaction revision, distinct from the base and from subsequent edits.
    pub fn revision(&self) -> RevisionId {
        self.revision
    }
    /// Canonical candidate records and indexes, including pending declarations.
    pub fn model(&self) -> &ModelView {
        &self.model
    }
    /// Missing required values, sorted by element and property identity.
    pub fn obligations(&self) -> &[ConstructionObligation] {
        &self.obligations
    }
}

pub(crate) struct Validation {
    deficits: Option<BTreeMap<(ElementId, PropertyId), ConstructionObligation>>,
}
impl Validation {
    fn strict() -> Self {
        Self { deficits: None }
    }
    pub(crate) fn multiplicity(
        &mut self,
        element: ElementId,
        property: PropertyId,
        required: Multiplicity,
        actual: usize,
    ) -> Result<(), ModelError> {
        if required.accepts(actual) {
            return Ok(());
        }
        if actual < required.lower
            && let Some(deficits) = &mut self.deficits
        {
            deficits.insert(
                (element, property),
                ConstructionObligation {
                    element,
                    property,
                    required,
                    actual,
                },
            );
            return Ok(());
        }
        Err(ModelError::Multiplicity {
            element,
            property,
            required,
            actual,
        })
    }
}

/// One present property value, carrying evidence separately from the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    pub(crate) value: SlotValue,
    pub(crate) origin: Origin,
}
impl Slot {
    /// Validated value; collections retain their descriptor-defined shape.
    pub fn value(&self) -> &SlotValue {
        &self.value
    }
    /// Evidence for this particular slot, which can differ from the element's origin.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
}

/// An ordinary semantic element, including instances of relationship metaclasses.
///
/// Fields are private. Only validated construction/change APIs can publish records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementRecord {
    pub(crate) id: ElementId,
    pub(crate) metaclass: MetaclassId,
    pub(crate) slots: BTreeMap<PropertyId, Slot>,
    pub(crate) origin: Origin,
}
impl ElementRecord {
    /// Stable semantic identity, including for relationship instances.
    pub fn id(&self) -> ElementId {
        self.id
    }
    /// Exact metaclass; use the registry for inherited membership.
    pub fn metaclass(&self) -> MetaclassId {
        self.metaclass
    }
    /// Evidence for the element's existence, separate from slot evidence.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
    /// `None` means absent, distinct from a present empty collection.
    pub fn slot(&self, property: PropertyId) -> Option<&Slot> {
        self.slots.get(&property)
    }
    /// Present slots in ascending property-ID order.
    pub fn slots(&self) -> impl Iterator<Item = (PropertyId, &Slot)> {
        self.slots.iter().map(|(id, slot)| (*id, slot))
    }
}

/// Required slot shape, derived from multiplicity, ordering and uniqueness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotShape {
    Scalar,
    Ordered,
    Set,
    Bag,
}
impl SlotShape {
    fn required(property: &PropertyDescriptor) -> Self {
        if property.multiplicity.scalar() {
            Self::Scalar
        } else if property.ordered {
            Self::Ordered
        } else if property.unique {
            Self::Set
        } else {
            Self::Bag
        }
    }
    fn actual(value: &SlotValue) -> Self {
        match value {
            SlotValue::Scalar(_) => Self::Scalar,
            SlotValue::Ordered(_) => Self::Ordered,
            SlotValue::Set(_) => Self::Set,
            SlotValue::Bag(_) => Self::Bag,
        }
    }
}

/// Canonical source of a reference; an inverse projection is never a second fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceCarrier {
    Slot,
    AssociationOccurrence(AssociationOccurrenceId),
    DerivedNavigation,
}

/// A reconstructible occurrence of a semantic reference, not a relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceOccurrence {
    pub carrier: ReferenceCarrier,
    /// The record containing the reference slot (possibly a relationship record).
    pub source: ElementId,
    /// The property that contains this occurrence.
    pub property: PropertyId,
    /// Zero-based slot/derived value position or per-end link position. Unordered
    /// links use deterministic occurrence-ID enumeration. For unordered properties
    /// this is a deterministic enumeration position, not semantic ordering.
    pub position: usize,
    /// The semantic element being referenced.
    pub target: ElementId,
}

#[derive(Clone, Debug, Default)]
struct Indexes {
    inverse_slots: BTreeMap<(ElementId, PropertyId), Slot>,
    incidence: BTreeMap<ElementId, BTreeSet<AssociationOccurrenceId>>,
    exact_class: BTreeMap<MetaclassId, BTreeSet<ElementId>>,
    by_supertype: BTreeMap<MetaclassId, BTreeSet<ElementId>>,
    incoming: BTreeMap<ElementId, Vec<ReferenceOccurrence>>,
    // Offsets preserve the canonical incoming order without copying occurrences.
    incoming_by_property: BTreeMap<(ElementId, PropertyId), Vec<usize>>,
    outgoing: BTreeMap<ElementId, Vec<ReferenceOccurrence>>,
}

/// Read-only semantic queries shared by declared snapshots and derived views.
///
/// Iteration is deterministic. Maps and storage handles are not public contracts.
#[derive(Clone, Debug)]
pub struct ModelView {
    pub(crate) declared_source: Option<Snapshot>,
    pub(crate) registry: Arc<MetamodelRegistry>,
    pub(crate) records: BTreeMap<ElementId, Arc<ElementRecord>>,
    indexes: Indexes,
    pub(crate) links: BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
    derived_navigation: crate::association::Navigation,
    pub(crate) statuses: BTreeMap<(ElementId, PropertyId), crate::derived::ComputationFailure>,
    pub(crate) searches:
        BTreeMap<crate::provenance::FactKey, Arc<BTreeSet<crate::derived::StructuralSearch>>>,
}

/// Mutable ownership transferred only inside an unpublished derivation batch.
/// Reconstructible indexes are intentionally absent: callers never clone them
/// just to discard them during validation of the next semantic frontier.
pub(crate) struct DerivationModelParts {
    pub records: BTreeMap<ElementId, Arc<ElementRecord>>,
    pub links: BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
    pub derived_navigation: crate::association::Navigation,
    pub statuses: BTreeMap<(ElementId, PropertyId), crate::derived::ComputationFailure>,
    pub searches: BTreeMap<FactKey, Arc<BTreeSet<crate::derived::StructuralSearch>>>,
}

impl ModelView {
    pub(crate) fn derivation_parts(&self) -> DerivationModelParts {
        DerivationModelParts {
            records: self.records.clone(),
            links: self.links.clone(),
            derived_navigation: self.derived_navigation.clone(),
            statuses: self.statuses.clone(),
            searches: self.searches.clone(),
        }
    }

    pub(crate) fn into_derivation_parts(self) -> DerivationModelParts {
        DerivationModelParts {
            records: self.records,
            links: self.links,
            derived_navigation: self.derived_navigation,
            statuses: self.statuses,
            searches: self.searches,
        }
    }

    pub(crate) fn build(
        registry: Arc<MetamodelRegistry>,
        records: BTreeMap<ElementId, Arc<ElementRecord>>,
        links: BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
        derived_navigation: crate::association::Navigation,
    ) -> Result<Self, ModelError> {
        Self::build_with_validation(
            registry,
            records,
            links,
            derived_navigation,
            &mut Validation::strict(),
        )
    }
    fn build_with_validation(
        registry: Arc<MetamodelRegistry>,
        records: BTreeMap<ElementId, Arc<ElementRecord>>,
        links: BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
        derived_navigation: crate::association::Navigation,
        validation: &mut Validation,
    ) -> Result<Self, ModelError> {
        let mut projected = crate::association::project(&registry, &records, &links, validation)?;
        for (&(element, property), slot) in &derived_navigation {
            let p = registry.property(property)?;
            if !p.derived || !matches!(p.owner, crate::metamodel::PropertyOwner::Association(_)) {
                return Err(ModelError::DerivedWrite { element, property });
            }
            if projected
                .insert((element, property), slot.clone())
                .is_some()
            {
                return Err(ModelError::UnsupportedAssociationStorage(property));
            }
        }
        validate(&registry, &records, &projected, validation)?;
        let mut indexes = Indexes {
            inverse_slots: projected,
            ..Indexes::default()
        };
        for link in links.values() {
            for &end in link.ends.values() {
                indexes.incidence.entry(end).or_default().insert(link.id);
            }
        }
        for record in records.values() {
            indexes
                .exact_class
                .entry(record.metaclass)
                .or_default()
                .insert(record.id);
            for class in registry.supertypes(record.metaclass) {
                indexes
                    .by_supertype
                    .entry(class)
                    .or_default()
                    .insert(record.id);
            }
            for (&property, slot) in &record.slots {
                for (position, value) in slot.value.values().enumerate() {
                    if let Value::Reference(target) = value {
                        if let Some(inverse) = registry.scalar_inverse(property)? {
                            let slot = Slot {
                                value: SlotValue::Scalar(Value::Reference(record.id)),
                                origin: slot.origin.clone(),
                            };
                            if indexes
                                .inverse_slots
                                .insert((*target, inverse), slot)
                                .is_some()
                            {
                                return Err(ModelError::InverseMultiplicity {
                                    element: *target,
                                    property: inverse,
                                });
                            }
                        }
                        let occurrence = ReferenceOccurrence {
                            carrier: ReferenceCarrier::Slot,
                            source: record.id,
                            property,
                            position,
                            target: *target,
                        };
                        indexes
                            .outgoing
                            .entry(record.id)
                            .or_default()
                            .push(occurrence);
                        indexes
                            .incoming
                            .entry(*target)
                            .or_default()
                            .push(occurrence);
                    }
                }
            }
        }
        let mut positions = BTreeMap::<(ElementId, PropertyId), usize>::new();
        for link in links.values() {
            for (&property, &target) in &link.ends {
                let p = registry.property(property)?;
                let source = link.ends[p.opposite_ends.first().expect("binary integrity")];
                let count = positions.entry((source, property)).or_default();
                let position = link.positions.get(&property).copied().unwrap_or(*count);
                *count += 1;
                let occurrence = ReferenceOccurrence {
                    carrier: ReferenceCarrier::AssociationOccurrence(link.id),
                    source,
                    property,
                    position,
                    target,
                };
                indexes.outgoing.entry(source).or_default().push(occurrence);
                indexes.incoming.entry(target).or_default().push(occurrence);
            }
        }
        for (&(source, property), slot) in &derived_navigation {
            for (position, value) in slot.value.values().enumerate() {
                if let Value::Reference(target) = value {
                    let occurrence = ReferenceOccurrence {
                        carrier: ReferenceCarrier::DerivedNavigation,
                        source,
                        property,
                        position,
                        target: *target,
                    };
                    indexes.outgoing.entry(source).or_default().push(occurrence);
                    indexes
                        .incoming
                        .entry(*target)
                        .or_default()
                        .push(occurrence);
                }
            }
        }
        for entries in indexes
            .incoming
            .values_mut()
            .chain(indexes.outgoing.values_mut())
        {
            entries.sort_by_key(|r| (r.source, r.property, r.position, r.target, r.carrier));
        }
        for (&target, entries) in &indexes.incoming {
            for (offset, occurrence) in entries.iter().enumerate() {
                indexes
                    .incoming_by_property
                    .entry((target, occurrence.property))
                    .or_default()
                    .push(offset);
            }
        }
        Ok(Self {
            declared_source: None,
            registry,
            records,
            indexes,
            links,
            derived_navigation,
            statuses: BTreeMap::new(),
            searches: BTreeMap::new(),
        })
    }
    /// Explicit property/navigation state, resolving only unambiguous class aliases.
    pub fn property_state(
        &self,
        element: ElementId,
        property: PropertyId,
    ) -> Result<crate::derived::PropertyState<'_>, ModelError> {
        use crate::derived::{ComputationFailure, PropertyState};
        let record = self
            .element(element)
            .ok_or(ModelError::UnknownElement(element))?;
        let declared = self.registry.property(property)?;
        let p = match declared.owner {
            crate::metamodel::PropertyOwner::Class(_) => self
                .registry
                .resolve_property(record.metaclass(), property)?,
            crate::metamodel::PropertyOwner::Association(_) => self
                .registry
                .is_applicable_navigation(record.metaclass(), property)?
                .then_some(declared),
        }
        .ok_or(ModelError::IllegalProperty {
            element,
            class: record.metaclass(),
            property,
        })?;
        Ok(match self.statuses.get(&(element, p.id)) {
            Some(f @ ComputationFailure::Incomplete { .. }) => PropertyState::Incomplete(f),
            Some(f @ ComputationFailure::Invalid { .. }) => PropertyState::Invalid(f),
            None => match self.navigation_slot(element, p.id) {
                Some(slot) => PropertyState::Computed(slot),
                None if p.derived => PropertyState::NotComputed,
                None => PropertyState::Absent,
            },
        })
    }
    /// Derived association-owned results, separate from canonical element slots.
    pub fn derived_navigation_results(
        &self,
    ) -> impl Iterator<Item = (&(ElementId, PropertyId), &Slot)> {
        self.derived_navigation.iter()
    }
    /// Complete search evidence, including negative searches used by computed values.
    pub fn computation_searches(
        &self,
    ) -> impl Iterator<
        Item = (
            &crate::provenance::FactKey,
            &BTreeSet<crate::derived::StructuralSearch>,
        ),
    > {
        self.searches
            .iter()
            .map(|(fact, searches)| (fact, searches.as_ref()))
    }
    /// Recorded search evidence for one fact, including negative searches.
    /// An empty iterator does not assert that the fact exists or is complete.
    pub fn computation_searches_for(
        &self,
        fact: crate::provenance::FactKey,
    ) -> impl Iterator<Item = &crate::derived::StructuralSearch> {
        self.searches.get(&fact).into_iter().flat_map(|s| s.iter())
    }
    /// Share immutable search evidence without expanding its entries. Equal
    /// sets may share storage across facts; allocation identity is not semantic.
    /// Absence, like an empty iterator, makes no completeness assertion.
    pub fn computation_searches_shared(
        &self,
        fact: crate::provenance::FactKey,
    ) -> Option<&Arc<BTreeSet<crate::derived::StructuralSearch>>> {
        self.searches.get(&fact)
    }
    pub fn computation_failures(
        &self,
    ) -> impl Iterator<
        Item = (
            &(ElementId, PropertyId),
            &crate::derived::ComputationFailure,
        ),
    > {
        self.statuses.iter()
    }
    /// Canonical association facts, deterministically ordered by occurrence identity.
    pub fn association_occurrences(&self) -> impl Iterator<Item = &AssociationOccurrence> {
        self.links.values()
    }
    pub fn association_occurrence(
        &self,
        id: AssociationOccurrenceId,
    ) -> Option<&AssociationOccurrence> {
        self.links.get(&id)
    }
    /// Incidence index semantics: includes either endpoint and preserves link identity.
    pub fn incident_associations(
        &self,
        element: ElementId,
    ) -> impl Iterator<Item = &AssociationOccurrence> {
        self.indexes
            .incidence
            .get(&element)
            .into_iter()
            .flatten()
            .map(|id| &self.links[id])
    }
    /// Occurrences classified under this association or its descendants. End identities
    /// remain those of the actual association; no implicit end alignment is invented.
    pub fn association_instances(
        &self,
        association: AssociationId,
        include_descendants: bool,
    ) -> Result<impl Iterator<Item = &AssociationOccurrence>, MetamodelError> {
        self.registry.association(association)?;
        Ok(self.links.values().filter(move |link| {
            link.association == association
                || include_descendants
                    && self
                        .registry
                        .association_ancestry(link.association)
                        .expect("integrity")
                        .contains(&association)
        }))
    }
    /// Immutable descriptors used to validate this view.
    pub fn registry(&self) -> &MetamodelRegistry {
        &self.registry
    }
    /// Number of semantic elements, counting relationships too.
    pub fn len(&self) -> usize {
        self.records.len()
    }
    /// Whether this view contains no semantic elements.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    /// Lookup by semantic identity; removed/unknown IDs return `None`.
    pub fn element(&self, id: ElementId) -> Option<&ElementRecord> {
        self.records.get(&id).map(Arc::as_ref)
    }
    /// Elements in ascending semantic-ID order.
    pub fn elements(&self) -> impl Iterator<Item = &ElementRecord> {
        self.records.values().map(Arc::as_ref)
    }
    /// Exact or subtype-inclusive metaclass membership, ordered by element ID.
    pub fn instances(
        &self,
        class: MetaclassId,
        include_subtypes: bool,
    ) -> Result<impl Iterator<Item = &ElementRecord>, MetamodelError> {
        self.registry.class(class)?;
        let index = if include_subtypes {
            &self.indexes.by_supertype
        } else {
            &self.indexes.exact_class
        };
        Ok(index
            .get(&class)
            .into_iter()
            .flatten()
            .map(|id| self.records[id].as_ref()))
    }
    /// References sorted by source, property, then value position.
    pub fn incoming(&self, id: ElementId) -> impl Iterator<Item = &ReferenceOccurrence> {
        self.indexes.incoming.get(&id).into_iter().flatten()
    }
    /// References to `target` through exactly `property`, in the same order as
    /// [`Self::incoming`]. Includes declared slots, association occurrences and
    /// derived navigation. This index does not resolve inherited property aliases.
    ///
    /// Lookup visits only matching references, independent of incoming references
    /// through other properties. Returned values borrow the ordinary incoming index.
    pub fn incoming_for_property(
        &self,
        target: ElementId,
        property: PropertyId,
    ) -> impl Iterator<Item = &ReferenceOccurrence> {
        let incoming = self
            .indexes
            .incoming
            .get(&target)
            .map_or(&[][..], Vec::as_slice);
        self.indexes
            .incoming_by_property
            .get(&(target, property))
            .into_iter()
            .flatten()
            .map(move |&offset| &incoming[offset])
    }
    /// References sorted by property, then value position.
    pub fn outgoing(&self, id: ElementId) -> impl Iterator<Item = &ReferenceOccurrence> {
        self.indexes.outgoing.get(&id).into_iter().flatten()
    }
    /// Stored slot or reconstructible scalar association navigation. Inverse
    /// projections are never independently writable and are not record slots.
    pub fn navigation_slot(&self, element: ElementId, property: PropertyId) -> Option<&Slot> {
        self.element(element)?
            .slot(property)
            .or_else(|| self.indexes.inverse_slots.get(&(element, property)))
    }
    /// Original submitted evidence, even when an overlay extends the same slot.
    /// Association projections are not independent declared facts; inspect their
    /// canonical occurrences instead. No inferred origin is reported as declared.
    pub fn declared_fact_origin(&self, fact: FactKey) -> Option<&DeclaredOrigin> {
        let origin = match fact {
            FactKey::Element(id) => self.element(id).map(|r| r.origin()),
            FactKey::Property { element, property } => self
                .element(element)
                .and_then(|r| r.slot(property))
                .map(|s| s.origin()),
            FactKey::AssociationOccurrence(id) => {
                self.association_occurrence(id).map(|r| r.origin())
            }
        };
        match origin {
            Some(Origin::Declared(origin)) => Some(origin),
            _ => self
                .declared_source
                .as_ref()
                .and_then(|source| source.model().declared_fact_origin(fact)),
        }
    }
}

#[derive(Debug)]
struct SnapshotData {
    revision: RevisionId,
    model: ModelView,
    used_ids: BTreeSet<ElementId>,
    used_links: BTreeSet<AssociationOccurrenceId>,
    dependency: Option<Arc<crate::derived::DerivedOverlay>>,
}

/// An immutable authored revision, optionally reading an immutable dependency.
/// Cloning is cheap and preserves exact base identity. Dependency origins and
/// explanations retain their declared/derived distinction.
///
/// `Snapshot` and its queries are Send + Sync; no interior mutation is used.
#[derive(Clone, Debug)]
pub struct Snapshot {
    inner: Arc<SnapshotData>,
}
impl Snapshot {
    /// Start a new empty declared history with a validated, pinned registry.
    pub fn new(registry: Arc<MetamodelRegistry>) -> Self {
        Self {
            inner: Arc::new(SnapshotData {
                revision: RevisionId::new(),
                model: ModelView {
                    declared_source: None,
                    registry,
                    records: BTreeMap::new(),
                    indexes: Indexes::default(),
                    links: BTreeMap::new(),
                    derived_navigation: BTreeMap::new(),
                    statuses: BTreeMap::new(),
                    searches: BTreeMap::new(),
                },
                used_ids: BTreeSet::new(),
                used_links: BTreeSet::new(),
                dependency: None,
            }),
        }
    }
    /// Start an independent authored history that can reference this immutable
    /// dependency. Canonical Element records remain shared. Changes cannot edit,
    /// remove, reorder or transfer ownership of dependency facts. This generic
    /// operation confers no language-specific publication acceptance.
    pub fn with_immutable_dependency(dependency: Arc<crate::derived::DerivedOverlay>) -> Self {
        let mut used_ids = dependency.declared().inner.used_ids.clone();
        used_ids.extend(dependency.model().elements().map(|r| r.id()));
        let mut used_links = dependency.declared().inner.used_links.clone();
        used_links.extend(dependency.model().association_occurrences().map(|r| r.id()));
        Self {
            inner: Arc::new(SnapshotData {
                revision: RevisionId::new(),
                model: dependency.model().clone(),
                used_ids,
                used_links,
                dependency: Some(dependency),
            }),
        }
    }
    /// The exact shared dependency supplied when this authored history began.
    pub fn immutable_dependency(&self) -> Option<&Arc<crate::derived::DerivedOverlay>> {
        self.inner.dependency.as_ref()
    }
    /// Whether this identity belongs to the immutable dependency population.
    pub fn is_dependency_element(&self, element: ElementId) -> bool {
        self.immutable_dependency()
            .is_some_and(|d| d.model().element(element).is_some())
    }
    /// Identity of this immutable declared state, independent of element identity.
    pub fn revision(&self) -> RevisionId {
        self.inner.revision
    }
    /// Read-only authored and dependency facts, retaining their original provenance.
    pub fn model(&self) -> &ModelView {
        &self.inner.model
    }
    /// Reserve a revision for an explicit ordered transaction on this exact base.
    pub fn change_set(&self) -> ChangeSet {
        ChangeSet {
            base: self.clone(),
            revision: RevisionId::new(),
            changes: Vec::new(),
        }
    }
    /// Apply all operations or return a typed error without mutating this snapshot.
    ///
    /// References and multiplicities are checked on the final candidate. Creation
    /// must precede edits to that record, but reference targets may be created later.
    pub fn apply(&self, changes: &ChangeSet) -> Result<Self, ModelError> {
        let (records, used_ids, links, used_links) = self.stage(changes)?;
        let mut model = ModelView::build(
            self.model().registry.clone(),
            records,
            links,
            self.model().derived_navigation.clone(),
        )?;
        self.check_dependency_ownership(&model)?;
        model.statuses = self.model().statuses.clone();
        model.declared_source = self.model().declared_source.clone();
        model.searches = self.model().searches.clone();
        Ok(Self {
            inner: Arc::new(SnapshotData {
                revision: changes.revision,
                model,
                used_ids,
                used_links,
                dependency: self.inner.dependency.clone(),
            }),
        })
    }
    /// Inspect an unpublished transaction with explicit lower-bound obligations.
    ///
    /// All present values, references, upper bounds, ownership and association
    /// occurrences are validated exactly as for publication. Missing required
    /// values are reported as obligations instead of inventing placeholder facts.
    /// This does not publish a snapshot, reserve identities, or mutate the base.
    /// Publication still requires `apply` to pass every structural invariant.
    pub fn preview(&self, changes: &ChangeSet) -> Result<ConstructionView, ModelError> {
        let (records, _, links, _) = self.stage(changes)?;
        let mut validation = Validation {
            deficits: Some(BTreeMap::new()),
        };
        let mut model = ModelView::build_with_validation(
            self.model().registry.clone(),
            records,
            links,
            self.model().derived_navigation.clone(),
            &mut validation,
        )?;
        self.check_dependency_ownership(&model)?;
        model.statuses = self.model().statuses.clone();
        model.declared_source = self.model().declared_source.clone();
        model.searches = self.model().searches.clone();
        Ok(ConstructionView {
            revision: changes.revision,
            model,
            obligations: validation
                .deficits
                .expect("construction validation")
                .into_values()
                .collect(),
        })
    }
    fn stage(&self, changes: &ChangeSet) -> Result<StagedRecords, ModelError> {
        if !Arc::ptr_eq(&self.inner, &changes.base.inner) {
            return Err(ModelError::StaleChangeSet {
                expected: self.revision(),
                actual: changes.base.revision(),
            });
        }
        let mut records = self.model().records.clone();
        let mut used_ids = self.inner.used_ids.clone();
        let mut links = self.model().links.clone();
        let mut used_links = self.inner.used_links.clone();
        for change in &changes.changes {
            let target = match change {
                Change::Create { id, .. }
                | Change::Remove(id)
                | Change::SetOrigin { id, .. }
                | Change::MoveInverse { element: id, .. } => FactKey::Element(*id),
                Change::Set { id, property, .. } | Change::Clear { id, property } => {
                    FactKey::Property {
                        element: *id,
                        property: *property,
                    }
                }
                Change::Link(link) => FactKey::AssociationOccurrence(link.id),
                Change::Unlink(id) | Change::Reorder { id, .. } => {
                    FactKey::AssociationOccurrence(*id)
                }
            };
            self.check_dependency_write(target)?;
            match change {
                Change::MoveInverse {
                    element,
                    inverse,
                    owner,
                    origin,
                } => {
                    let registry = &self.model().registry;
                    let storage = registry
                        .inverse_storage(*inverse)?
                        .ok_or(ModelError::UnsupportedAssociationStorage(*inverse))?;
                    let record = records
                        .get(element)
                        .ok_or(ModelError::UnknownElement(*element))?;
                    if !registry.is_legal(record.metaclass(), *inverse)? {
                        return Err(ModelError::IllegalProperty {
                            element: *element,
                            class: record.metaclass(),
                            property: *inverse,
                        });
                    }
                    for record in records.values_mut() {
                        if record.slot(storage).is_some_and(|slot| {
                            slot.value()
                                .values()
                                .any(|v| v == &Value::Reference(*element))
                        }) {
                            self.check_dependency_write(FactKey::Property {
                                element: record.id(),
                                property: storage,
                            })?;
                            let slot = Arc::make_mut(record)
                                .slots
                                .get_mut(&storage)
                                .expect("checked slot");
                            let SlotValue::Ordered(values) = &mut slot.value else {
                                return Err(ModelError::UnsupportedAssociationStorage(storage));
                            };
                            values.retain(|v| *v != Value::Reference(*element));
                            slot.origin = Origin::Declared(origin.clone());
                        }
                    }
                    if let Some((owner, position)) = owner {
                        self.check_dependency_write(FactKey::Property {
                            element: *owner,
                            property: storage,
                        })?;
                        let record = records
                            .get_mut(owner)
                            .ok_or(ModelError::UnknownElement(*owner))?;
                        let slot =
                            Arc::make_mut(record)
                                .slots
                                .entry(storage)
                                .or_insert_with(|| Slot {
                                    value: SlotValue::Ordered(vec![]),
                                    origin: Origin::Declared(origin.clone()),
                                });
                        let SlotValue::Ordered(values) = &mut slot.value else {
                            return Err(ModelError::UnsupportedAssociationStorage(storage));
                        };
                        if *position > values.len() {
                            return Err(ModelError::InvalidAssociationPosition {
                                property: storage,
                                position: *position,
                            });
                        }
                        values.insert(*position, Value::Reference(*element));
                        slot.origin = Origin::Declared(origin.clone());
                    }
                }
                Change::Link(link) => {
                    if !used_links.insert(link.id) {
                        return Err(ModelError::InvalidAssociationOccurrence(link.id));
                    }
                    links.insert(link.id, link.clone());
                }
                Change::Unlink(id) => {
                    if links.remove(id).is_none() {
                        return Err(ModelError::InvalidAssociationOccurrence(*id));
                    }
                }
                Change::Reorder {
                    id,
                    positions,
                    origin,
                } => {
                    let link = links
                        .get_mut(id)
                        .ok_or(ModelError::InvalidAssociationOccurrence(*id))?;
                    link.positions = positions.clone();
                    link.origin = Origin::Declared(origin.clone());
                }

                Change::Create {
                    id,
                    metaclass,
                    origin,
                } => {
                    if !used_ids.insert(*id) {
                        return Err(ModelError::ReusedIdentity(*id));
                    }
                    records.insert(
                        *id,
                        Arc::new(ElementRecord {
                            id: *id,
                            metaclass: *metaclass,
                            slots: BTreeMap::new(),
                            origin: Origin::Declared(origin.clone()),
                        }),
                    );
                }
                Change::Remove(id) => {
                    if records.remove(id).is_none() {
                        return Err(ModelError::UnknownElement(*id));
                    }
                }
                Change::Set {
                    id,
                    property,
                    value,
                    origin,
                } => {
                    let record = records.get_mut(id).ok_or(ModelError::UnknownElement(*id))?;
                    if self.model().registry.property(*property)?.derived {
                        return Err(ModelError::DerivedWrite {
                            element: *id,
                            property: *property,
                        });
                    }
                    if !self.model().registry.supports_slot_storage(*property)? {
                        return Err(ModelError::UnsupportedAssociationStorage(*property));
                    }
                    let mut value = value.clone();
                    value.normalize();
                    Arc::make_mut(record).slots.insert(
                        *property,
                        Slot {
                            value,
                            origin: Origin::Declared(origin.clone()),
                        },
                    );
                }
                Change::Clear { id, property } => {
                    let record = records.get_mut(id).ok_or(ModelError::UnknownElement(*id))?;
                    if !self
                        .model()
                        .registry
                        .is_legal(record.metaclass, *property)?
                    {
                        return Err(ModelError::IllegalProperty {
                            element: *id,
                            class: record.metaclass,
                            property: *property,
                        });
                    }
                    if self.model().registry.property(*property)?.derived {
                        return Err(ModelError::DerivedWrite {
                            element: *id,
                            property: *property,
                        });
                    }
                    if !self.model().registry.supports_slot_storage(*property)? {
                        return Err(ModelError::UnsupportedAssociationStorage(*property));
                    }
                    Arc::make_mut(record).slots.remove(property);
                }
                Change::SetOrigin { id, origin } => {
                    let record = records.get_mut(id).ok_or(ModelError::UnknownElement(*id))?;
                    Arc::make_mut(record).origin = Origin::Declared(origin.clone());
                }
            }
        }
        Ok((records, used_ids, links, used_links))
    }
    pub(crate) fn has_used(&self, id: ElementId) -> bool {
        self.inner.used_ids.contains(&id)
    }
    pub(crate) fn has_used_occurrence(&self, id: AssociationOccurrenceId) -> bool {
        self.inner.used_links.contains(&id)
    }
    pub(crate) fn check_dependency_write(&self, fact: FactKey) -> Result<(), ModelError> {
        let protected = match fact {
            FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                self.is_dependency_element(id)
            }
            FactKey::AssociationOccurrence(id) => self
                .immutable_dependency()
                .is_some_and(|d| d.model().association_occurrence(id).is_some()),
        };
        if protected {
            Err(ModelError::ImmutableDependency(fact))
        } else {
            Ok(())
        }
    }
    /// New local carriers can change a dependency's ownership without writing
    /// its records directly. Check the final navigation, including occurrence
    /// projections and derived results, against the protected ownership graph.
    pub(crate) fn check_dependency_ownership(
        &self,
        candidate: &ModelView,
    ) -> Result<(), ModelError> {
        let Some(dependency) = self.immutable_dependency() else {
            return Ok(());
        };
        let protected = dependency.model();
        let slots = candidate
            .records
            .values()
            .flat_map(|record| record.slots().map(move |(p, slot)| (record.id(), p, slot)))
            .chain(
                candidate
                    .indexes
                    .inverse_slots
                    .iter()
                    .map(|(&(element, property), slot)| (element, property, slot)),
            );
        for (element, property, slot) in slots {
            if !candidate.registry.property(property)?.composite {
                continue;
            }
            if protected.element(element).is_some() {
                if protected
                    .navigation_slot(element, property)
                    .map(Slot::value)
                    != Some(slot.value())
                {
                    return Err(ModelError::ImmutableDependency(FactKey::Property {
                        element,
                        property,
                    }));
                }
            } else {
                for value in slot.value().values() {
                    if let Value::Reference(target) = value
                        && protected.element(*target).is_some()
                    {
                        return Err(ModelError::ImmutableDependency(FactKey::Element(*target)));
                    }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn has_declared_fact(&self, fact: FactKey) -> bool {
        let origin = match fact {
            FactKey::Element(id) => self.model().element(id).map(|r| r.origin()),
            FactKey::Property { element, property } => self
                .model()
                .element(element)
                .and_then(|r| r.slot(property))
                .map(|s| s.origin()),
            FactKey::AssociationOccurrence(id) => {
                self.model().association_occurrence(id).map(|r| r.origin())
            }
        };
        matches!(origin, Some(Origin::Declared(_)))
            || self
                .immutable_dependency()
                .is_some_and(|d| d.declared().has_declared_fact(fact))
    }
}

#[derive(Debug)]
enum Change {
    MoveInverse {
        element: ElementId,
        inverse: PropertyId,
        owner: Option<(ElementId, usize)>,
        origin: DeclaredOrigin,
    },
    Link(AssociationOccurrence),
    Unlink(AssociationOccurrenceId),
    Reorder {
        id: AssociationOccurrenceId,
        positions: BTreeMap<PropertyId, usize>,
        origin: DeclaredOrigin,
    },
    SetOrigin {
        id: ElementId,
        origin: DeclaredOrigin,
    },
    Create {
        id: ElementId,
        metaclass: MetaclassId,
        origin: DeclaredOrigin,
    },
    Remove(ElementId),
    Set {
        id: ElementId,
        property: PropertyId,
        value: SlotValue,
        origin: DeclaredOrigin,
    },
    Clear {
        id: ElementId,
        property: PropertyId,
    },
}

/// A mutable transaction builder. Published snapshots and records stay immutable.
///
/// Every edit reserves a new candidate revision ID, so extending an already-applied
/// builder cannot publish different content under the earlier revision identity.
#[derive(Debug)]
pub struct ChangeSet {
    base: Snapshot,
    revision: RevisionId,
    changes: Vec<Change>,
}
impl ChangeSet {
    /// Edit a supported scalar inverse by modifying its sole ordered canonical slot.
    /// The caller supplies insertion order explicitly. No inverse fact is stored.
    pub fn move_inverse(
        &mut self,
        element: ElementId,
        inverse: PropertyId,
        owner: Option<(ElementId, usize)>,
        origin: DeclaredOrigin,
    ) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::MoveInverse {
            element,
            inverse,
            owner,
            origin,
        });
        self
    }
    /// Insert one fact with both endpoint values. Use this API only for associations
    /// classified for occurrence storage; slot-backed associations reject it.
    pub fn link(
        &mut self,
        id: AssociationOccurrenceId,
        association: AssociationId,
        ends: BTreeMap<PropertyId, ElementId>,
        positions: BTreeMap<PropertyId, usize>,
        origin: DeclaredOrigin,
    ) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Link(AssociationOccurrence {
            id,
            association,
            ends,
            positions,
            origin: Origin::Declared(origin),
        }));
        self
    }
    /// Remove explicitly; deleting an endpoint never silently cascades.
    pub fn unlink(&mut self, id: AssociationOccurrenceId) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Unlink(id));
        self
    }
    /// Change per-end ordering atomically alongside all other occurrence edits.
    pub fn reorder_link(
        &mut self,
        id: AssociationOccurrenceId,
        positions: BTreeMap<PropertyId, usize>,
        origin: DeclaredOrigin,
    ) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Reorder {
            id,
            positions,
            origin,
        });
        self
    }
    /// Update evidence for an existing declaration without changing its identity.
    /// Older snapshots keep their original evidence.
    pub fn set_origin(&mut self, id: ElementId, origin: DeclaredOrigin) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::SetOrigin { id, origin });
        self
    }
    /// Revision this transaction was created from; application also checks exact base identity.
    pub fn base_revision(&self) -> RevisionId {
        self.base.revision()
    }
    /// Candidate revision identity, stable until another operation is appended.
    pub fn revision(&self) -> RevisionId {
        self.revision
    }
    /// Create an element. IDs previously used in this declared history are refused.
    pub fn create(
        &mut self,
        id: ElementId,
        metaclass: MetaclassId,
        origin: DeclaredOrigin,
    ) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Create {
            id,
            metaclass,
            origin,
        });
        self
    }
    /// Delete explicitly, with no cascade. Remaining references must be repaired
    /// or removed in this same transaction.
    pub fn remove(&mut self, id: ElementId) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Remove(id));
        self
    }
    /// Replace a complete slot; provenance describes this newly submitted fact.
    pub fn set(
        &mut self,
        id: ElementId,
        property: PropertyId,
        value: SlotValue,
        origin: DeclaredOrigin,
    ) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Set {
            id,
            property,
            value,
            origin,
        });
        self
    }
    /// Remove a legal, non-derived slot. Clearing an absent optional slot is valid.
    pub fn clear(&mut self, id: ElementId, property: PropertyId) -> &mut Self {
        self.revision = RevisionId::new();
        self.changes.push(Change::Clear { id, property });
        self
    }
}

/// Structural validation failure, preserving semantic identifiers and value shape.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ModelError {
    #[error("immutable dependency fact cannot be changed: {0:?}")]
    ImmutableDependency(FactKey),
    #[error("invalid insertion position {position} for association property {property}")]
    InvalidAssociationPosition {
        property: PropertyId,
        position: usize,
    },
    #[error("invalid, unsupported, unknown or reused association occurrence {0}")]
    InvalidAssociationOccurrence(AssociationOccurrenceId),
    #[error("association inverse {element}/{property} has more than one source")]
    InverseMultiplicity {
        element: ElementId,
        property: PropertyId,
    },
    #[error(
        "property {0} requires canonical link storage beyond the supported single-slot association shape"
    )]
    UnsupportedAssociationStorage(PropertyId),
    #[error(transparent)]
    Metamodel(#[from] MetamodelError),
    #[error("unknown element {0}")]
    UnknownElement(ElementId),
    #[error("element identity {0} was already used in this history")]
    ReusedIdentity(ElementId),
    #[error("element {element} cannot instantiate abstract class {class}")]
    AbstractClass {
        element: ElementId,
        class: MetaclassId,
    },
    #[error("property {property} is illegal on {element} of class {class}")]
    IllegalProperty {
        element: ElementId,
        class: MetaclassId,
        property: PropertyId,
    },
    #[error("derived property {property} cannot be authored on {element}")]
    DerivedWrite {
        element: ElementId,
        property: PropertyId,
    },
    #[error("slot {element}/{property} has {actual} values, requires {required:?}")]
    Multiplicity {
        element: ElementId,
        property: PropertyId,
        required: Multiplicity,
        actual: usize,
    },
    #[error("slot {element}/{property} has shape {actual:?}, requires {expected:?}")]
    Shape {
        element: ElementId,
        property: PropertyId,
        expected: SlotShape,
        actual: SlotShape,
    },
    #[error("slot {element}/{property} requires {expected:?}")]
    ValueKind {
        element: ElementId,
        property: PropertyId,
        expected: ValueKind,
    },
    #[error("slot {element}/{property} contains duplicate values")]
    DuplicateValue {
        element: ElementId,
        property: PropertyId,
    },
    #[error("slot {element}/{property} references absent element {target}")]
    DanglingReference {
        element: ElementId,
        property: PropertyId,
        target: ElementId,
    },
    #[error("slot {element}/{property} target {target} must have class {expected}")]
    ReferenceType {
        element: ElementId,
        property: PropertyId,
        target: ElementId,
        expected: MetaclassId,
    },
    #[error("composite target {target} is owned by both {first:?} and {second:?}")]
    MultipleContainers {
        target: ElementId,
        first: (ElementId, PropertyId),
        second: (ElementId, PropertyId),
    },
    #[error("containment cycle involving {0:?}")]
    ContainmentCycle(Vec<ElementId>),
    #[error("change set base {actual} is not this snapshot {expected}")]
    StaleChangeSet {
        expected: RevisionId,
        actual: RevisionId,
    },
}

#[derive(Default)]
struct Containment {
    containers: BTreeMap<ElementId, (ElementId, PropertyId)>,
    edges: BTreeMap<ElementId, BTreeSet<ElementId>>,
}

fn validate(
    registry: &MetamodelRegistry,
    records: &BTreeMap<ElementId, Arc<ElementRecord>>,
    projected: &crate::association::Navigation,
    validation: &mut Validation,
) -> Result<(), ModelError> {
    let mut containment = Containment::default();
    for record in records.values() {
        if registry.class(record.metaclass)?.is_abstract {
            return Err(ModelError::AbstractClass {
                element: record.id,
                class: record.metaclass,
            });
        }
        for (&property, slot) in &record.slots {
            if !registry.is_legal(record.metaclass, property)? {
                return Err(ModelError::IllegalProperty {
                    element: record.id,
                    class: record.metaclass,
                    property,
                });
            }
            // Refuse independently writable inverse ends and inverse ordering/
            // bounds. Also guards implied elements against bypassing write policy.
            if !registry.supports_slot_storage(property)? {
                return Err(ModelError::UnsupportedAssociationStorage(property));
            }
            validate_slot(
                registry,
                records,
                record,
                property,
                slot,
                &mut containment,
                validation,
            )?;
        }
        for property in registry.effective_properties(record.metaclass)? {
            let slot = record
                .slots
                .get(&property.id)
                .or_else(|| projected.get(&(record.id, property.id)));
            // Derived slots need not have been computed. If present, validate fully.
            if property.derived && slot.is_none() {
                continue;
            }
            let count = slot.map_or(0, |s| s.value.values().count());
            validation.multiplicity(record.id, property.id, property.multiplicity, count)?;
        }
    }
    for (&(element, property), slot) in projected {
        let record = records
            .get(&element)
            .ok_or(ModelError::UnknownElement(element))?;
        if !registry.is_applicable_navigation(record.metaclass(), property)? {
            return Err(ModelError::IllegalProperty {
                element,
                class: record.metaclass(),
                property,
            });
        }
        validate_slot(
            registry,
            records,
            record,
            property,
            slot,
            &mut containment,
            validation,
        )?;
    }
    let cycle = cyclic_nodes(&containment.edges);
    if !cycle.is_empty() {
        return Err(ModelError::ContainmentCycle(cycle));
    }
    Ok(())
}

fn validate_slot(
    registry: &MetamodelRegistry,
    records: &BTreeMap<ElementId, Arc<ElementRecord>>,
    record: &ElementRecord,
    property: PropertyId,
    slot: &Slot,
    containment: &mut Containment,
    validation: &mut Validation,
) -> Result<(), ModelError> {
    let descriptor = registry.property(property)?;
    let expected = SlotShape::required(descriptor);
    let actual = SlotShape::actual(&slot.value);
    if expected != actual {
        return Err(ModelError::Shape {
            element: record.id,
            property,
            expected,
            actual,
        });
    }
    let mut unique = BTreeSet::new();
    for value in slot.value.values() {
        if descriptor.unique && !unique.insert(value) {
            return Err(ModelError::DuplicateValue {
                element: record.id,
                property,
            });
        }
        match (registry.storage_kind(descriptor.value_kind)?, value) {
            (ValueKind::Boolean, Value::Boolean(_))
            | (ValueKind::Integer, Value::Integer(_))
            | (ValueKind::Real, Value::Real(_))
            | (ValueKind::String, Value::String(_)) => {}
            (ValueKind::Enumeration(domain), Value::Enumeration(literal))
                if registry.enumeration(domain)?.literals.contains_key(literal) => {}
            (ValueKind::Reference(expected), Value::Reference(target)) => {
                let target_record = records.get(target).ok_or(ModelError::DanglingReference {
                    element: record.id,
                    property,
                    target: *target,
                })?;
                if !registry.is_subtype(target_record.metaclass, expected)? {
                    return Err(ModelError::ReferenceType {
                        element: record.id,
                        property,
                        target: *target,
                        expected,
                    });
                }
                for opposite in &descriptor.opposite_ends {
                    if let ValueKind::Reference(context) = registry.property(*opposite)?.value_kind
                        && !registry.is_subtype(record.metaclass, context)?
                    {
                        return Err(ModelError::ReferenceType {
                            element: *target,
                            property: *opposite,
                            target: record.id,
                            expected: context,
                        });
                    }
                }
                if descriptor.composite {
                    let owner = (record.id, property);
                    if let Some(first) = containment.containers.insert(*target, owner)
                        && first != owner
                    {
                        return Err(ModelError::MultipleContainers {
                            target: *target,
                            first,
                            second: owner,
                        });
                    }
                    containment
                        .edges
                        .entry(record.id)
                        .or_default()
                        .insert(*target);
                }
            }
            _ => {
                return Err(ModelError::ValueKind {
                    element: record.id,
                    property,
                    expected: descriptor.value_kind,
                });
            }
        }
    }

    let actual = slot.value.values().count();
    validation.multiplicity(record.id, property, descriptor.multiplicity, actual)?;
    Ok(())
}

/// Exact cyclic SCC members, borrowing adjacency without duplicating its edges.
pub(crate) fn cyclic_nodes<K: Copy + Ord>(edges: &BTreeMap<K, BTreeSet<K>>) -> Vec<K> {
    cyclic_nodes_by(edges.keys().copied(), |node| {
        edges.get(node).into_iter().flatten().copied()
    })
}

/// Iterative Tarjan traversal. Auxiliary storage is proportional to vertices,
/// even when many nodes share one large immutable adjacency set. DFS frames
/// retain borrowed iterators rather than allocating per-node edge collections.
pub(crate) fn cyclic_nodes_by<K: Copy + Ord, I: Iterator<Item = K>>(
    nodes: impl IntoIterator<Item = K>,
    neighbors: impl Fn(&K) -> I,
) -> Vec<K> {
    let mut indices = BTreeMap::new();
    let mut lowlinks = BTreeMap::new();
    let mut active = BTreeSet::new();
    let mut component_stack = Vec::new();
    let mut cyclic = BTreeSet::new();
    for root in nodes {
        if indices.contains_key(&root) {
            continue;
        }
        let index = indices.len();
        indices.insert(root, index);
        lowlinks.insert(root, index);
        active.insert(root);
        component_stack.push(root);
        let mut frames = vec![(root, neighbors(&root), false)];
        while let Some((node, targets, self_loop)) = frames.last_mut() {
            if let Some(target) = targets.next() {
                *self_loop |= target == *node;
                if let Some(&index) = indices.get(&target) {
                    if active.contains(&target) {
                        let low = lowlinks.get_mut(node).expect("visited node");
                        *low = (*low).min(index);
                    }
                } else {
                    let index = indices.len();
                    indices.insert(target, index);
                    lowlinks.insert(target, index);
                    active.insert(target);
                    component_stack.push(target);
                    frames.push((target, neighbors(&target), false));
                }
            } else {
                let (node, _, self_loop) = frames.pop().expect("current frame");
                let low = lowlinks[&node];
                if low == indices[&node] {
                    let mut component = Vec::new();
                    loop {
                        let member = component_stack.pop().expect("active SCC member");
                        active.remove(&member);
                        component.push(member);
                        if member == node {
                            break;
                        }
                    }
                    if component.len() > 1 || self_loop {
                        cyclic.extend(component);
                    }
                }
                if let Some((parent, _, _)) = frames.last() {
                    let parent_low = lowlinks.get_mut(parent).expect("visited parent");
                    *parent_low = (*parent_low).min(low);
                }
            }
        }
    }
    cyclic.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_four_vertex_graph_matches_reachability_cycle_oracle() {
        for mask in 0..(1u32 << 16) {
            let mut reachable = [[false; 4]; 4];
            let mut edges = BTreeMap::new();
            for (source, row) in reachable.iter_mut().enumerate() {
                for (target, present) in row.iter_mut().enumerate() {
                    *present = mask & (1 << (source * 4 + target)) != 0;
                    if *present {
                        edges
                            .entry(source)
                            .or_insert_with(BTreeSet::new)
                            .insert(target);
                    }
                }
            }
            for via in 0..4 {
                for source in 0..4 {
                    for target in 0..4 {
                        reachable[source][target] |=
                            reachable[source][via] && reachable[via][target];
                    }
                }
            }
            let expected: Vec<_> = (0..4).filter(|&node| reachable[node][node]).collect();
            assert_eq!(cyclic_nodes(&edges), expected, "edge mask {mask}");
        }
    }

    #[test]
    fn cycle_members_exclude_downstream_vertices() {
        let edges = BTreeMap::from([
            ('A', BTreeSet::from(['B'])),
            ('B', BTreeSet::from(['A', 'C'])),
        ]);
        assert_eq!(cyclic_nodes(&edges), vec!['A', 'B']);
    }

    #[test]
    fn acyclic_chain_has_no_cycle_members() {
        let edges = BTreeMap::from([
            ('A', BTreeSet::from(['B'])),
            ('B', BTreeSet::from(['C'])),
            ('C', BTreeSet::from(['D'])),
        ]);
        assert!(cyclic_nodes(&edges).is_empty());
    }

    #[test]
    fn self_loop_is_a_cycle() {
        let edges = BTreeMap::from([('A', BTreeSet::from(['A']))]);
        assert_eq!(cyclic_nodes(&edges), vec!['A']);
    }

    #[test]
    fn three_vertex_cycle_reports_every_member() {
        let edges = BTreeMap::from([
            ('A', BTreeSet::from(['B'])),
            ('B', BTreeSet::from(['C'])),
            ('C', BTreeSet::from(['A'])),
        ]);
        assert_eq!(cyclic_nodes(&edges), vec!['A', 'B', 'C']);
    }

    #[test]
    fn connected_cycles_report_members_in_sorted_order() {
        let entries = [
            ('D', BTreeSet::from(['C'])),
            ('B', BTreeSet::from(['C', 'A'])),
            ('C', BTreeSet::from(['D'])),
            ('A', BTreeSet::from(['B'])),
        ];
        for edges in [
            entries.clone().into_iter().collect(),
            entries.into_iter().rev().collect(),
        ] {
            assert_eq!(cyclic_nodes(&edges), vec!['A', 'B', 'C', 'D']);
        }
    }

    #[test]
    fn cycle_members_exclude_upstream_downstream_and_isolated_vertices() {
        let edges = BTreeMap::from([
            ('A', BTreeSet::from(['B'])),
            ('B', BTreeSet::from(['C'])),
            ('C', BTreeSet::from(['B', 'D'])),
            ('E', BTreeSet::new()),
        ]);
        assert_eq!(cyclic_nodes(&edges), vec!['B', 'C']);
        assert!(cyclic_nodes::<char>(&BTreeMap::new()).is_empty());
    }

    #[test]
    fn converging_acyclic_paths_have_no_cycle_members() {
        let edges = BTreeMap::from([
            ('A', BTreeSet::from(['B', 'C'])),
            ('B', BTreeSet::from(['C', 'D'])),
            ('C', BTreeSet::from(['D'])),
        ]);
        assert!(cyclic_nodes(&edges).is_empty());
    }

    #[test]
    fn deep_containment_and_evidence_graphs_do_not_require_recursion() {
        let mut edges: BTreeMap<_, _> = (0..20_000).map(|n| (n, BTreeSet::from([n + 1]))).collect();
        assert!(cyclic_nodes(&edges).is_empty());
        edges.insert(20_000, BTreeSet::from([19_999]));
        assert_eq!(cyclic_nodes(&edges), vec![19_999, 20_000]);
        edges.insert(20_000, BTreeSet::from([0]));
        assert_eq!(cyclic_nodes(&edges), (0..=20_000).collect::<Vec<_>>());
    }
}
