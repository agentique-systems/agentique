use crate::metamodel::{
    MetamodelError, MetamodelRegistry, Multiplicity, PropertyDescriptor, ValueKind,
};
use crate::provenance::{DeclaredOrigin, Origin};
use crate::value::{SlotValue, Value};
use crate::{ElementId, MetaclassId, PropertyId, RevisionId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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

/// A reconstructible occurrence of a semantic reference, not a relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReferenceOccurrence {
    /// The record containing the reference slot (possibly a relationship record).
    pub source: ElementId,
    /// The property that contains this occurrence.
    pub property: PropertyId,
    /// Zero-based position in `SlotValue::values()`. For unordered properties
    /// this is a deterministic enumeration position, not semantic ordering.
    pub position: usize,
    /// The semantic element being referenced.
    pub target: ElementId,
}

#[derive(Clone, Debug, Default)]
struct Indexes {
    exact_class: BTreeMap<MetaclassId, BTreeSet<ElementId>>,
    by_supertype: BTreeMap<MetaclassId, BTreeSet<ElementId>>,
    incoming: BTreeMap<ElementId, Vec<ReferenceOccurrence>>,
    outgoing: BTreeMap<ElementId, Vec<ReferenceOccurrence>>,
}

/// Read-only semantic queries shared by declared snapshots and derived views.
///
/// Iteration is deterministic. Maps and storage handles are not public contracts.
#[derive(Clone, Debug)]
pub struct ModelView {
    pub(crate) registry: Arc<MetamodelRegistry>,
    pub(crate) records: BTreeMap<ElementId, Arc<ElementRecord>>,
    indexes: Indexes,
}
impl ModelView {
    pub(crate) fn build(
        registry: Arc<MetamodelRegistry>,
        records: BTreeMap<ElementId, Arc<ElementRecord>>,
    ) -> Result<Self, ModelError> {
        validate(&registry, &records)?;
        let mut indexes = Indexes::default();
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
                        let occurrence = ReferenceOccurrence {
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
        Ok(Self {
            registry,
            records,
            indexes,
        })
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
    /// References sorted by property, then value position.
    pub fn outgoing(&self, id: ElementId) -> impl Iterator<Item = &ReferenceOccurrence> {
        self.indexes.outgoing.get(&id).into_iter().flatten()
    }
}

#[derive(Debug)]
struct SnapshotData {
    revision: RevisionId,
    model: ModelView,
    used_ids: BTreeSet<ElementId>,
}

/// An immutable declared revision. Cloning is cheap and preserves exact base identity.
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
                    registry,
                    records: BTreeMap::new(),
                    indexes: Indexes::default(),
                },
                used_ids: BTreeSet::new(),
            }),
        }
    }
    /// Identity of this immutable declared state, independent of element identity.
    pub fn revision(&self) -> RevisionId {
        self.inner.revision
    }
    /// Read-only queries over declared elements and slots.
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
        if !Arc::ptr_eq(&self.inner, &changes.base.inner) {
            return Err(ModelError::StaleChangeSet {
                expected: self.revision(),
                actual: changes.base.revision(),
            });
        }
        let mut records = self.model().records.clone();
        let mut used_ids = self.inner.used_ids.clone();
        for change in &changes.changes {
            match change {
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
                    Arc::make_mut(record).slots.remove(property);
                }
            }
        }
        let model = ModelView::build(self.model().registry.clone(), records)?;
        Ok(Self {
            inner: Arc::new(SnapshotData {
                revision: changes.revision,
                model,
                used_ids,
            }),
        })
    }
    pub(crate) fn has_used(&self, id: ElementId) -> bool {
        self.inner.used_ids.contains(&id)
    }
}

#[derive(Debug)]
enum Change {
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

fn validate(
    registry: &MetamodelRegistry,
    records: &BTreeMap<ElementId, Arc<ElementRecord>>,
) -> Result<(), ModelError> {
    let mut containers = BTreeMap::new();
    let mut containment: BTreeMap<ElementId, BTreeSet<ElementId>> = BTreeMap::new();
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
                match (descriptor.value_kind, value) {
                    (ValueKind::Boolean, Value::Boolean(_))
                    | (ValueKind::Integer, Value::Integer(_))
                    | (ValueKind::String, Value::String(_)) => {}
                    (ValueKind::Reference(expected), Value::Reference(target)) => {
                        let target_record =
                            records.get(target).ok_or(ModelError::DanglingReference {
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
                        if descriptor.composite {
                            let owner = (record.id, property);
                            if let Some(first) = containers.insert(*target, owner)
                                && first != owner
                            {
                                return Err(ModelError::MultipleContainers {
                                    target: *target,
                                    first,
                                    second: owner,
                                });
                            }
                            containment.entry(record.id).or_default().insert(*target);
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
        }
        for property in registry.effective_properties(record.metaclass)? {
            let slot = record.slots.get(&property.id);
            // Derived slots need not have been computed. If present, validate fully.
            if property.derived && slot.is_none() {
                continue;
            }
            let count = slot.map_or(0, |s| s.value.values().count());
            if !property.multiplicity.accepts(count) {
                return Err(ModelError::Multiplicity {
                    element: record.id,
                    property: property.id,
                    required: property.multiplicity,
                    actual: count,
                });
            }
        }
    }
    let cycle = cyclic_nodes(&containment);
    if !cycle.is_empty() {
        return Err(ModelError::ContainmentCycle(cycle));
    }
    Ok(())
}

/// Exact cyclic SCC members in sorted order, using iterative Kosaraju traversal.
/// Both passes use heap-backed stacks, with no recursion proportional to input size.
pub(crate) fn cyclic_nodes<K: Copy + Ord>(edges: &BTreeMap<K, BTreeSet<K>>) -> Vec<K> {
    let mut reverse: BTreeMap<K, BTreeSet<K>> = BTreeMap::new();
    for (&source, targets) in edges {
        reverse.entry(source).or_default();
        for &target in targets {
            reverse.entry(target).or_default().insert(source);
        }
    }

    // Keep each DFS frame until all its edges are visited to record finish order.
    // The reverse map includes isolated sources and vertices appearing only as targets.
    let mut visited = BTreeSet::new();
    let mut finished = Vec::with_capacity(reverse.len());
    for &root in reverse.keys() {
        if !visited.insert(root) {
            continue;
        }
        let mut stack = vec![(root, edges.get(&root).into_iter().flatten())];
        while let Some((node, targets)) = stack.last_mut() {
            if let Some(&target) = targets.next() {
                if visited.insert(target) {
                    stack.push((target, edges.get(&target).into_iter().flatten()));
                }
            } else {
                finished.push(*node);
                stack.pop();
            }
        }
    }

    // Reverse edges and reverse finish order isolate each strongly connected component.
    visited.clear();
    let mut cyclic = BTreeSet::new();
    for root in finished.into_iter().rev() {
        if !visited.insert(root) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            component.push(node);
            for &source in &reverse[&node] {
                if visited.insert(source) {
                    stack.push(source);
                }
            }
        }
        if component.len() > 1 || reverse[&root].contains(&root) {
            cyclic.extend(component);
        }
    }
    cyclic.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
