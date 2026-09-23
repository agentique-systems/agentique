//! Canonical occurrences for associations without a single class-slot carrier.
use crate::metamodel::{MetamodelRegistry, ValueKind};
use crate::model::{ElementRecord, Slot};
use crate::provenance::{DeclaredOrigin, Origin};
use crate::shared_map::SharedMap;
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
    pub(crate) origin: Origin,
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
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
    /// Submitted source evidence, absent for a semantically derived occurrence.
    pub fn declared_origin(&self) -> Option<&DeclaredOrigin> {
        match &self.origin {
            Origin::Declared(origin) => Some(origin),
            _ => None,
        }
    }
}

pub(crate) type Navigation = SharedMap<(ElementId, PropertyId), Slot>;

/// Validate incidence, both inverse bounds and order before publishing projections.
pub(crate) fn project(
    registry: &MetamodelRegistry,
    records: &SharedMap<ElementId, Arc<ElementRecord>>,
    links: &SharedMap<AssociationOccurrenceId, AssociationOccurrence>,
    validation: &mut crate::model::Validation,
    inherited: Option<&Navigation>,
) -> Result<Navigation, ModelError> {
    type Group<'a> = Vec<(Option<usize>, ElementId, &'a AssociationOccurrence)>;
    let mut groups: BTreeMap<(ElementId, PropertyId), Group<'_>> = BTreeMap::new();
    for link in links.local_values() {
        let association = registry.association(link.association)?;
        let supported = if matches!(link.origin, Origin::Derived(_)) {
            registry.supports_derived_occurrence_storage(link.association)?
        } else {
            registry.supports_occurrence_storage(link.association)?
        };
        if !supported || association.is_abstract {
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
    // Reproject only groups touched by a local carrier. Inherited groups retain
    // their shared slots; extending one group retains exact prior occurrences.
    for (&(context, end), values) in &mut groups {
        if let Some(slot) = inherited.and_then(|base| base.get(&(context, end))) {
            if let Origin::AssociationOccurrences(ids) = &slot.origin {
                for id in ids {
                    let link = &links[id];
                    values.push((link.positions.get(&end).copied(), link.ends[&end], link));
                }
            } else {
                return Err(ModelError::UnsupportedAssociationStorage(end));
            }
        }
    }
    let mut navigation = inherited.map_or_else(SharedMap::new, SharedMap::fork);
    for ((context, end), mut values) in groups {
        let p = registry.property(end)?;
        if records[&context].slot(end).is_some() {
            return Err(ModelError::UnsupportedAssociationStorage(end));
        }
        validation.multiplicity(context, end, p.multiplicity, values.len())?;
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
        if p.derived
            && !values
                .iter()
                .any(|(_, _, link)| matches!(link.origin, Origin::Derived(_)))
        {
            continue;
        } // A declared link alone never asserts a derived computation.
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
    validate_required_navigation(registry, records, &navigation, validation)?;
    Ok(navigation)
}

pub(crate) fn validate_required_navigation(
    registry: &MetamodelRegistry,
    records: &SharedMap<ElementId, Arc<ElementRecord>>,
    navigation: &Navigation,
    validation: &mut crate::model::Validation,
) -> Result<(), ModelError> {
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
                validation.multiplicity(record.id(), p.id, p.multiplicity, 0)?;
            }
        }
    }
    Ok(())
}
