//! Canonical occurrences for associations without a single class-slot carrier.
use crate::metamodel::{MetamodelRegistry, ValueKind};
use crate::model::{ElementRecord, Slot};
use crate::provenance::{DeclaredOrigin, Origin};
use crate::value::{SlotValue, Value};
use crate::{AssociationId, AssociationOccurrenceId, ElementId, ModelError, PropertyId};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// One association fact. Endpoint keys are descriptor identities, never positions.
/// An ordered end additionally supplies a zero-based position within its context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationOccurrence {
    pub(crate) id: AssociationOccurrenceId,
    pub(crate) association: AssociationId,
    pub(crate) ends: BTreeMap<PropertyId, ElementId>,
    pub(crate) positions: BTreeMap<PropertyId, usize>,
    pub(crate) origin: DeclaredOrigin,
}
impl AssociationOccurrence {
    pub fn id(&self) -> AssociationOccurrenceId {
        self.id
    }
    pub fn association(&self) -> AssociationId {
        self.association
    }
    pub fn ends(&self) -> &BTreeMap<PropertyId, ElementId> {
        &self.ends
    }
    pub fn positions(&self) -> &BTreeMap<PropertyId, usize> {
        &self.positions
    }
    pub fn origin(&self) -> &DeclaredOrigin {
        &self.origin
    }
}

pub(crate) type Navigation = BTreeMap<(ElementId, PropertyId), Slot>;

/// Validate incidence, both inverse bounds and order before publishing projections.
pub(crate) fn project(
    registry: &MetamodelRegistry,
    records: &BTreeMap<ElementId, Arc<ElementRecord>>,
    links: &BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
) -> Result<Navigation, ModelError> {
    type Group<'a> = Vec<(Option<usize>, ElementId, &'a AssociationOccurrence)>;
    let mut groups: BTreeMap<(ElementId, PropertyId), Group<'_>> = BTreeMap::new();
    for link in links.values() {
        let association = registry.association(link.association)?;
        if !registry.supports_occurrence_storage(link.association)? || association.is_abstract {
            return Err(ModelError::InvalidAssociationOccurrence(link.id));
        }
        if link.ends.keys().copied().collect::<BTreeSet<_>>()
            != association.member_ends.iter().copied().collect()
        {
            return Err(ModelError::InvalidAssociationOccurrence(link.id));
        }
        let mut ordered = BTreeSet::new();
        for &end in &association.member_ends {
            let p = registry.property(end)?;
            let target = link.ends[&end];
            let record = records
                .get(&target)
                .ok_or(ModelError::UnknownElement(target))?;
            let ValueKind::Reference(expected) = p.value_kind else {
                unreachable!("structural integrity")
            };
            if !registry.is_subtype(record.metaclass(), expected)? {
                return Err(ModelError::InvalidAssociationOccurrence(link.id));
            }
            if p.ordered && !p.multiplicity.scalar() {
                ordered.insert(end);
            }
            let opposite = *p.opposite_ends.first().expect("binary integrity");
            let context = link.ends[&opposite];
            if let crate::metamodel::PropertyOwner::Class(_) = p.owner
                && !registry.is_legal(records[&context].metaclass(), end)?
            {
                return Err(ModelError::InvalidAssociationOccurrence(link.id));
            }
            groups.entry((context, end)).or_default().push((
                link.positions.get(&end).copied(),
                target,
                link,
            ));
        }
        if ordered != link.positions.keys().copied().collect() {
            return Err(ModelError::InvalidAssociationOccurrence(link.id));
        }
    }
    let mut navigation = BTreeMap::new();
    for ((context, end), mut values) in groups {
        let p = registry.property(end)?;
        if !p.multiplicity.accepts(values.len()) {
            return Err(ModelError::Multiplicity {
                element: context,
                property: end,
                required: p.multiplicity,
                actual: values.len(),
            });
        }
        let unique: BTreeSet<_> = values.iter().map(|(_, target, _)| *target).collect();
        if p.unique && unique.len() != values.len() {
            return Err(ModelError::DuplicateValue {
                element: context,
                property: end,
            });
        }
        values.sort_by_key(|(position, target, link)| (*position, *target, link.id));
        if p.ordered
            && !p.multiplicity.scalar()
            && values
                .iter()
                .enumerate()
                .any(|(i, (pos, _, _))| *pos != Some(i))
        {
            return Err(ModelError::InvalidAssociationOccurrence(values[0].2.id));
        }
        if p.derived {
            continue;
        } // A submitted link never computes a derived property.
        let origin =
            Origin::AssociationOccurrences(values.iter().map(|(_, _, link)| link.id).collect());
        let entries: Vec<_> = values
            .into_iter()
            .map(|(_, target, _)| Value::Reference(target))
            .collect();
        let value = if p.multiplicity.scalar() {
            SlotValue::Scalar(entries[0].clone())
        } else if p.ordered {
            SlotValue::Ordered(entries)
        } else if p.unique {
            SlotValue::Set(entries.into_iter().collect())
        } else {
            SlotValue::Bag(entries)
        };
        navigation.insert((context, end), Slot { value, origin });
    }
    // Navigable required ends are obligations on their context instances. A
    // non-navigable inverse lower bound applies to participating occurrences;
    // it does not manufacture an authored property on every target instance.
    for p in registry.properties() {
        if p.derived
            || p.multiplicity.lower == 0
            || !registry.is_navigable(p.id)?
            || !p
                .association
                .is_some_and(|a| registry.supports_occurrence_storage(a).unwrap_or(false))
        {
            continue;
        }
        let context = registry.property_context(p)?;
        for record in records.values() {
            if registry.is_subtype(record.metaclass(), context)?
                && !navigation.contains_key(&(record.id(), p.id))
            {
                return Err(ModelError::Multiplicity {
                    element: record.id(),
                    property: p.id,
                    required: p.multiplicity,
                    actual: 0,
                });
            }
        }
    }
    Ok(navigation)
}
